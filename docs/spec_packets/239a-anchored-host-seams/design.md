# Design: 239a-anchored-host-seams

## ADR Conformance

This packet is governed by `docs/adr/0059-support-families-and-anchored-entities.md`
(slug `0059-support-families-and-anchored-entities`, `Status: accepted`). Its normative clause,
quoted verbatim:

> A planar entity between model planes is anchored to the upper global layer, executes in
> ascending Z before that layer's ordinary model event, and is optimized and cooling-accounted
> independently; same-Z support joins the ordinary model event. A future atomic Z-spanning entity
> may extend outside its anchor layer's Z interval while still executing at that layer's normal
> position.

Three conformance rulings follow, and every AC, locked assumption, and step in this packet is
written to them:

1. **Planar off-grid → synthesized row: conforms.** Emitting a solo `LayerCollectionIR` row at
   the entity's declared Z, ordered *before* the upper anchor layer's ordinary `Model` row, is
   precisely "executes in ascending Z before that layer's ordinary model event". The synthesized
   row is also the unit at which the entity is optimized and cooling-accounted independently.
2. **Anchor attribution is the UPPER global layer.** A solo synthesized row adopts the
   `global_layer_index` of the **upper** global layer — the layer it is anchored to — not that of
   the nearest preceding `Model` row. The ADR says "anchored to the upper global layer"; the
   nearest-preceding rule would attribute it to the layer below and contradict the ADR. See
   §Locked Assumptions.
3. **Z-spanning entities do NOT get their own row.** The ADR requires a Z-spanning entity to
   execute "at that layer's normal position", i.e. as one atomic contiguous block inside the
   anchor layer's **ordinary** row (`CommittedLayerEvent::Model`'s `ordered_entities`), even
   though its geometry extends outside that layer's Z interval. Atomicity ("Z-spanning
   atomicity") is unchanged; only the location of the block changes. AC-4 is written to this.

Same-Z (on-grid) support continues to join the ordinary model event unchanged — the ADR's
"same-Z support joins the ordinary model event", which is the plan doc's "same-Z support in
ordinary ordering" invariant and AC-N1's guard.

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-runtime/src/pipeline.rs` — `PipelineConfig` (the struct that gains the
    anchored input field), and the two entry points that own duplicated per-layer bodies:
    `run_pipeline_with_events` (public) and `run_pipeline_core` (private `fn`). Both take
    `config: PipelineConfig` **by value** and destructure it **exhaustively, without a `..`
    rest**, so the additive field forces an edit in both patterns. `run_pipeline` and
    `run_pipeline_with_raw_config` and `run_pipeline_with_instrumentation` are thin forwarders;
    `run_pipeline_with_events` is deliberately **not** a forwarder — an inline NOTE comment in its
    body records that it emits a bare G-code body with no thumbnail/CONFIG_BLOCK wrapper, which is why it
    keeps a separate body. The switch therefore lands twice.
  - `crates/slicer-runtime/src/layer_executor.rs` — `execute_per_layer_with_committed_anchored_events`
    (the target entry point; its signature ends
    `anchored_entities: &[slicer_ir::AnchoredEntity]) -> Result<(Vec<CommittedLayerEvent>, Vec<ModuleAccessAudit>), LayerExecutionError>`),
    the `pub enum CommittedLayerEvent` with exactly two variants
    `Anchored(slicer_ir::OrderedEventCollection)` and `Model(LayerCollectionIR)`, and the
    complementary filter pair `is_same_z_entity` (private) used positively in
    `append_same_z_entities` and negatively in `execute_anchored_event_collections`.
    `CommittedLayerEvent` is **not** re-exported from `crates/slicer-runtime/src/lib.rs`; tests
    reach it as `slicer_runtime::layer_executor::CommittedLayerEvent`.
  - `crates/slicer-runtime/src/anchored_rows.rs` — **new module** holding the pure row-synthesis
    function. Declared `pub mod anchored_rows;` in `crates/slicer-runtime/src/lib.rs` so
    integration tests can drive AC-5 against the function directly without going through a
    pipeline run.
  - `crates/pnp-cli/src/visual_debug.rs` — the third non-anchored caller, invoking
    `slicer_runtime::layer_executor::execute_per_layer_with_events_and_support_tools` and then
    `slicer_runtime::execute_layer_finalization` /
    `slicer_runtime::postpass::execute_postpass_with_capture`. It has the same
    per-layer → finalization → postpass shape as the two pipeline bodies.
- Neighboring tests/fixtures:
  - `crates/slicer-runtime/tests/integration/main.rs` — the aggregator for the `integration`
    binary declared as `[[test]] name = "integration"`. It carries 69 top-level `mod`
    declarations plus a `#[path]`-mounted `common`, and 22 inline `#[test] fn` wrappers that
    call submodule functions (the `anchored_*` family uses wrappers). `pipeline_tdd` does not:
    its functions carry `#[test]` in place and it is mounted by a bare `mod pipeline_tdd;` line.
    This packet's new file follows `pipeline_tdd`'s convention, so exactly one `mod` line is
    added and no wrapper is ever needed.
  - `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` — home of `LayerCountEmitter` and
    `OrderTrackingEmitter`. Both prove the `PipelineStageRunners.emitter: Box<dyn GCodeEmitter>`
    injection seam works, but neither stores the payload: `LayerCountEmitter` records only
    `.len()`, `OrderTrackingEmitter` ignores the rows entirely. No existing mock in the
    workspace keeps the `&[LayerCollectionIR]` slice for assertion — hence the new fixture.
  - `crates/slicer-runtime/tests/common/mod.rs` — `pipeline_config_base`, one of the two FRU
    base helpers. Measured 2026-08-28 (`rg -n 'PipelineConfig \{' crates/`, then classifying each
    hit by whether its body carries a `..` rest): **33** `PipelineConfig` construction literals
    exist workspace-wide (1 production, 32 test); **27** use FRU over a base helper and **6** are
    exhaustive. The two FRU base helpers absorb 27 of the 33.
  - `crates/pnp-cli/tests/e2e_integration_tdd.rs` — the second `pipeline_config_base`, plus its
    own `LayerCountEmitter`. Its `pipeline_config_base` body is itself exhaustive (it carries an
    `// exhaustive:` waiver; the `..Default::default()` further down that file belongs to
    `make_global_layer`, not to the `PipelineConfig` literal — do not misread it as FRU).
  - `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs` — **three** fully
    exhaustive `PipelineConfig` literals, each under an `// exhaustive: boundary fixture
    preserves explicit test data` waiver. This file is in the `contract` test binary, not
    `integration`, which is why an integration-only sweep misses it. Packet 239a's original
    "exactly three edit sites" claim omitted all three; they are now in Step 1's edit list, and
    without them `cargo check --workspace --all-targets` cannot go green.
  - `crates/slicer-runtime/tests/integration/anchored_event_ordering.rs`,
    `anchored_parallel_determinism.rs`, `anchored_z_validation.rs`,
    `anchored_z_span_validation.rs`, `anchored_event_accounting.rs` — the existing anchored-route
    suite. AC-N1 is one of these (`anchored_event_ordering`), used unchanged as a regression
    guard; this packet authors none of them.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat
  delegation rules.

## Architecture Constraints

- **Executor routing is already total (F1); do not design as if it has a hole.**
  `is_same_z_entity` has exactly three references — its definition, the positive filter in
  `append_same_z_entities`, and the negated filter in `execute_anchored_event_collections`. The
  two filters are exact complements, so an off-grid `AnchoredGeometryContract::Planar { z }`
  already reaches the anchored collection today. The defect is downstream of the executor, in
  the absence of an input seam (F3) and an emission representation (F4). Any change to the
  filters is a clarity refactor only and cannot move an AC from red to green.
- **`GCodeEmitter::emit_gcode`'s signature is frozen.** Measured 2026-08-28:
  `rg -n 'impl GCodeEmitter for' crates/` → **14** impl blocks workspace-wide;
  `rg -c '\.emit_gcode\(' crates/ -g '*.rs'` summed → **52** call sites. Critically, the impls are
  **distributed across test crates, not concentrated in `crates/slicer-gcode/src/emit.rs`**: that
  file holds exactly **one**, `impl GCodeEmitter for DefaultGCodeEmitter` (the production one).
  The remaining 13 live in `crates/slicer-runtime/tests/` (`visual_debug_postpass_tap_tdd.rs`,
  `contract/dispatch_infill_output_tdd.rs`, `integration/run_pipeline_with_instrumentation_tdd.rs`,
  `integration/runtime_wiring_tdd.rs`, `integration/pipeline_tdd.rs` ×4,
  `executor/postpass_executor_tdd.rs` ×3) and `crates/pnp-cli/tests/e2e_integration_tdd.rs` ×2.
  Off-grid work must arrive as ordinary `LayerCollectionIR` rows in the existing
  `&[LayerCollectionIR]` argument, never as a new parameter or a new trait method.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it. (Included because the pipeline switch changes which executor entry point performs guest dispatches: any evidence run that touches a guest module — every integration run using real WASM handles — must pass the freshness gate before a failure is attributed to this packet's edits. This packet's own edit list is host-only, so a stale report here means a pre-existing stale artifact, not a new one; it must still be resolved before drawing conclusions.)
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`. (Concretely here: routing and merging compare a declared planar Z, which is already canonical i64 units — the existing fixtures use `z: 3000` = 0.3 mm — against `mm_to_units(row.z)` derived from `LayerCollectionIR.z: f32`, which is mm. `mm_to_units` has signature `mm_to_units(mm: f32) -> i64` in `crates/slicer-ir/src/slice_ir.rs`. Row synthesis converts once, at the boundary, and does all comparisons in i64 units; it never float-compares mm against units and never hard-codes a raw unit literal without provenance.)
- **No version constant is bumped.** Synthesized rows set
  `schema_version: CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` (`crates/slicer-ir/src/slice_ir.rs`),
  **reusing whatever the live constant is at the moment of the edit**. This packet deliberately
  does **not** pin the value: the schema version is a mutable ledger fact that another packet can
  bump between authoring and execution. (For orientation only, re-derived 2026-08-28 — the
  constant reads `major: 1, minor: 4, patch: 0` and `docs/02_ir_schemas.md`'s
  `LayerCollectionIR` entry reads "Current schema_version: 1.4.0"; an earlier draft of this
  packet asserted `1.2.0` in three places, which was already stale. Do not copy `1.4.0` forward
  either — read the constant.) Nothing in this packet freezes a literal or bumps it, and no event
  wire format is locked or changed.
  `PipelineConfig` is a host-side orchestration struct — not an IR type, not a wire type, no
  schema version, no component boundary — so its additive field carries no versioning
  obligation.
- **`cargo xtask check-literals` applies to every new test literal.** New `LayerCollectionIR`
  and `PipelineConfig` literals in test code must carry a `..` FRU rest or an
  `// exhaustive: <reason>` waiver; production `src/` literals stay exhaustive. See
  `docs/21_data_defaults_and_fixtures.md`.

## Code Change Surface

- Selected approach:
  1. **Input seam.** `PipelineConfig` gains one additive owned field
     `pub anchored_entities: Vec<slicer_ir::AnchoredEntity>`. Owned, not borrowed: `PipelineConfig`
     carries no lifetime parameter today and introducing one would ripple through every
     construction site and both destructuring patterns. Empty vec is the meaningful default and
     is what all **six** exhaustive literal sites get (enumerated below; re-derive at edit time).
  2. **Executor switch.** In `run_pipeline_core` and `run_pipeline_with_events`, replace the
     `execute_per_layer_with_instrumentation_and_support_tools` /
     `execute_per_layer_with_events_and_support_tools` call with
     `execute_per_layer_with_committed_anchored_events`, passing `&anchored_entities`. The call
     now yields `Vec<CommittedLayerEvent>` instead of `Vec<LayerCollectionIR>`.
  3. **Row synthesis.** `slicer_runtime::anchored_rows::synthesize_anchored_rows` consumes that
     `Vec<CommittedLayerEvent>` and returns an ordered `Vec<LayerCollectionIR>`. It walks the
     committed sequence with the canonical two-index discipline: object rows from
     `CommittedLayerEvent::Model`, anchored planes from `CommittedLayerEvent::Anchored`; at each
     step it takes the minimum of the two pending Z values in i64 units and un-consumes whichever
     side exceeds it by more than the merge epsilon. When `|dz| <= epsilon` the anchored
     collection's entities are appended into the object row's `ordered_entities` and one row is
     emitted; otherwise the lower Z emits a solo row and the other side retries on the next
     iteration. The merge epsilon is `COORDINATE_TOLERANCE_UNITS` (= 10 units = 10⁻³ mm), the
     same constant `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` re-exports, so AC-2's
     on-grid/off-grid partition and AC-5's merge rule are decided by one value and cannot
     disagree.
  4. **Insertion.** The synthesized `Vec<LayerCollectionIR>` becomes the `layer_irs` binding
     that flows into finalization. In `run_pipeline_core` that means synthesis happens between
     the per-layer call and `execute_layer_finalization_with_instrumentation(..., &mut layer_irs, ...)`
     — the last mutable seam before `run_postpass_with_thumbnail(..., &layer_irs, ...)`, which
     hands an immutable slice to `slicer_runtime::postpass::execute_postpass_with_capture`, and
     that function deep-copies via `layer_irs.to_vec()` before `.emit_gcode`. Rows inserted at or
     before finalization therefore survive into emission; rows inserted after it do not. The
     parallel non-instrumented sequence in `run_pipeline_with_events`
     (`execute_per_layer_with_events_and_support_tools` → `execute_layer_finalization` →
     `execute_postpass`) receives the identical treatment.
  5. **Third call site.** `crates/pnp-cli/src/visual_debug.rs` performs the same substitution in
     its own per-layer → finalization → postpass sequence.
  6. **Clarity refactor.** The complementary `is_same_z_entity` / `!is_same_z_entity` filter
     pair is replaced by one shared named helper (e.g. `route_of` returning a two-variant
     routing enum, or a single `same_z_route` predicate consumed with an explicit
     `partition`), so a future reader cannot mistake the pair for a partial cover. Behaviour is
     bit-identical by construction.
  7. **Test fixture.** A payload-capturing emitter holding `Arc<Mutex<Vec<LayerCollectionIR>>>`
     (working name `CapturedRowsEmitter`) is added to `crates/slicer-runtime/tests/common/mod.rs`
     so both the `integration` binary and any other slicer-runtime test target can use it. It
     stores a clone of every row handed to `emit_gcode` and exposes an accessor returning the
     captured sequence.
- Exact functions, traits, manifests, tests, and fixtures:
  - `PipelineConfig` and its two exhaustive destructuring patterns in `run_pipeline_with_events`
    and `run_pipeline_core` (`crates/slicer-runtime/src/pipeline.rs`).
  - **The six exhaustive `PipelineConfig` literal sites** — every one needs the new field
    initializer, because none of them carries a `..` rest (an `// exhaustive:` waiver satisfies
    `check-literals`; it does **not** make the literal FRU):
    1. `run_slice_with_collector` (`crates/slicer-runtime/src/run.rs`) — the single production
       literal.
    2. `pipeline_config_base` (`crates/slicer-runtime/tests/common/mod.rs`) — FRU base for the
       `slicer-runtime` test binaries.
    3. `pipeline_config_base` (`crates/pnp-cli/tests/e2e_integration_tdd.rs`) — FRU base for the
       `pnp-cli` test binaries.
    4.–6. Three literals in `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs`
       (in the `contract` binary, each preceded by an `// exhaustive: boundary fixture preserves
       explicit test data` waiver). These are the sites an integration-only sweep misses and the
       reason Step 1's postcondition was previously unachievable.
    The other 27 construction literals inherit the field through one of the two
    `pipeline_config_base` helpers and are not edited.
  - `execute_per_layer_with_committed_anchored_events`, `CommittedLayerEvent`,
    `is_same_z_entity`, `append_same_z_entities`, `execute_anchored_event_collections`
    (`crates/slicer-runtime/src/layer_executor.rs`).
  - New `synthesize_anchored_rows` plus its in-module `#[cfg(test)] mod tests`
    (`crates/slicer-runtime/src/anchored_rows.rs`), and the `pub mod anchored_rows;` declaration
    in `crates/slicer-runtime/src/lib.rs`.
  - New `CapturedRowsEmitter` (`crates/slicer-runtime/tests/common/mod.rs`).
  - New `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` carrying
    `offgrid_support_row_emitted_at_declared_z` (AC-1),
    `every_same_z_support_entity_routes_exactly_once` (AC-2),
    `offgrid_entity_never_merged_into_grid_layers` (AC-N2),
    `offgrid_row_merge_matches_canonical_epsilon_rule` (AC-5),
    `support_free_slice_row_sequence_unchanged` (AC-6),
    `support_disabled_pipeline_emits_no_anchored_rows` (AC-N3),
    `offgrid_row_order_identical_serial_and_parallel` (AC-3), and
    `zspanning_support_entity_emits_atomic_single_block` (AC-4); mounted by one
    `mod offgrid_rows_tdd;` line in `crates/slicer-runtime/tests/integration/main.rs`.
    **All eight are addressed by libtest as `offgrid_rows_tdd::<fn>`** — every `--exact` filter
    in this packet must use that prefixed form.
  - New `payload_capturing_emitter_records_row_sequence`
    (`crates/slicer-runtime/tests/integration/pipeline_tdd.rs`) — Step 2's fixture proof and
    AC-6's baseline capture. Addressed as
    `pipeline_tdd::payload_capturing_emitter_records_row_sequence`.
  - The per-layer call inside `crates/pnp-cli/src/visual_debug.rs`.
  - Docs: `docs/07_implementation_status.md`, `docs/specs/support-parity-gap-register.md`,
    `docs/specs/support-independent-layer-z-split-plan.md`.
  - No manifest, no WIT, no module.
- Rejected alternatives and reasons:
  - **Change `GCodeEmitter::emit_gcode`'s signature** (add an off-grid-rows parameter, or a
    second trait method). **Rejected.** Measured 2026-08-28: **14** `impl GCodeEmitter for`
    blocks and **52** `.emit_gcode(` call sites workspace-wide. `crates/slicer-gcode/src/emit.rs`
    holds exactly **one** of those impls (the production `DefaultGCodeEmitter`); the other **13
    are distributed across test crates** (`crates/slicer-runtime/tests/` and
    `crates/pnp-cli/tests/`). A signature change is a ~66-site mechanical churn across four
    crates for zero
    behavioural gain, because the receiving type `&[LayerCollectionIR]` can already express an
    off-grid row: `LayerCollectionIR` carries a free `z: f32`, and nothing constrains that `z`
    to the global layer grid. Synthesizing ordinary rows and inserting them into `layer_irs`
    reaches the identical emitter with zero API churn. `emit.rs` is out of bounds for this
    packet as a direct consequence — **not** because the impls could be surveyed there (they
    cannot; 13 of the 14 are elsewhere) but because the trait's shape is already recorded here
    and nothing in this packet changes it.
  - **A new IR row type or an additive `LayerCollectionIR` field.** Rejected — it would bump
    `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` (whatever its live value; do not pin it — see
    §Architecture Constraints), touch the five production literal sites recorded in F9
    (`crates/slicer-runtime/src/layer_executor.rs` ×2, `crates/slicer-sdk/src/traits.rs`,
    `crates/slicer-macros/src/lib.rs`, `crates/slicer-wasm-host/src/dispatch.rs`), and drag the
    guest build into a host-only change. The existing row shape suffices.
  - **Making `run_pipeline_with_events` forward to `run_pipeline_core` to avoid the duplicate
    switch.** Rejected — the duplication is deliberate and documented by an inline NOTE comment in
    `run_pipeline_with_events`'s body (it emits a bare body without the thumbnail/CONFIG_BLOCK
    serializer wrapper).
    Collapsing them would silently change that entry point's output, which AC-6 would then be
    unable to distinguish from a row-synthesis regression.
  - **Changing any `pipeline.rs` signature** (e.g. threading `&[AnchoredEntity]` as a new
    parameter instead of a `PipelineConfig` field). **Rejected** — two source-text guard tests,
    `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs` and
    `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs`, assert the entry-point and
    `run_pipeline_core` signature strings **verbatim**. Because both bodies destructure a
    by-value `PipelineConfig`, an additive field reaches them without touching any signature, so
    both guards stay green and act as tripwires instead of fallout.
  - **Fixing the routing filters as a functional change.** Rejected — F1 measured the partition
    already total. Presenting the refactor as a fix would be a fabricated defect.
  - **Renumbering `global_layer_index` so synthesized rows get fresh indices.** Rejected — it
    would shift every downstream row's index and make AC-6's element-wise equality impossible to
    satisfy on any run with anchored entities, while breaking index-keyed postpass consumers.
    See §Locked Assumptions.

## Files in Scope (read + edit)

Primary (three):

- `crates/slicer-runtime/src/pipeline.rs` — role: owns `PipelineConfig` and both duplicated
  per-layer bodies; expected change: one additive field, two destructuring patterns updated, two
  executor calls switched, two synthesis-insertion points added.
- `crates/slicer-runtime/src/anchored_rows.rs` — role: new home of the pure row-synthesis
  function; expected change: new file with `synthesize_anchored_rows` and its unit tests.
- `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` — role: new integration test
  file carrying eight of the nine AC test names; expected change: new file, appended across four
  steps.

Justified extras — each is a one-to-few-line mechanical surface, and each is assigned to exactly
one step so no step exceeds its own budget:

- `crates/slicer-runtime/src/run.rs` — the single production `PipelineConfig` literal
  (`run_slice_with_collector`); expected change: one field initializer.
- `crates/slicer-runtime/tests/common/mod.rs` — `pipeline_config_base` FRU base (one field
  initializer) and the new `CapturedRowsEmitter` fixture.
- `crates/pnp-cli/tests/e2e_integration_tdd.rs` — the second `pipeline_config_base` FRU base;
  expected change: one field initializer.
- `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs` — three exhaustive
  `PipelineConfig` literals in the `contract` binary; expected change: three field initializers,
  one per literal. Omitted from the packet's first draft; without it Step 1 cannot reach a green
  `cargo check --workspace --all-targets`.
- `crates/slicer-runtime/src/lib.rs` — expected change: one `pub mod anchored_rows;` line.
- `crates/slicer-runtime/src/layer_executor.rs` — expected change: the behaviour-neutral shared
  route-partition helper only.
- `crates/slicer-runtime/tests/integration/main.rs` — expected change: one `mod` line.
- `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` — expected change: one new test
  proving the capturing emitter and recording AC-6's baseline.
- `crates/pnp-cli/src/visual_debug.rs` — expected change: one call-site switch plus the
  synthesis insertion in its own finalization sequence.
- `docs/07_implementation_status.md`, `docs/specs/support-parity-gap-register.md`,
  `docs/specs/support-independent-layer-z-split-plan.md` — closure-step doc edits only.

## Read-Only Context

Ranges given for every file over 300 lines.

- `crates/slicer-runtime/src/layer_executor.rs` — 3886 lines. Read only two neighbourhoods:
  the anchored entry points and routing (`execute_per_layer_with_anchored_events` through
  `append_same_z_entities`, roughly lines 190–470) and the anchored-collection execution family
  (`execute_anchored_event_collections` and its `_with_accounting` / `_with_mode` /
  `_with_mode_and_feedrate` siblings, roughly lines 2470–2600). Purpose: the
  `CommittedLayerEvent` variant list, the committed entry point's exact return tuple, and the
  two complementary filter uses. Never full-read.
- `crates/slicer-runtime/src/pipeline.rs` — 626 lines. Purpose: `PipelineConfig` field list and
  both destructuring patterns; the per-layer → finalization → postpass ordering in both bodies;
  `run_postpass_with_thumbnail`'s immutable-slice argument.
- `crates/pnp-cli/src/visual_debug.rs` — 2340 lines. Read only the per-layer →
  finalization → postpass sequence around the
  `slicer_runtime::layer_executor::execute_per_layer_with_events_and_support_tools` call
  (roughly lines 1120–1200). Purpose: the third call site's shape. Never full-read.
- `crates/slicer-ir/src/slice_ir.rs` — 3141 lines. Read only: the `COORDINATE_TOLERANCE_UNITS`
  and `mm_to_units` declarations near the file head, the `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`
  constant, the `LayerCollectionIR` struct (fields: `schema_version`, `global_layer_index: u32`,
  `z: f32`, `ordered_entities`, `support_entity_identities`, `tool_changes`, `z_hops`,
  `annotations`, `retracts`, `travel_moves`, `speed_profiles`), and the
  `AnchoredGeometryContract` / `AnchoredEntity` definitions with the
  `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS` re-export. Never full-read.
- `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` — 1533 lines. Read only the
  `LayerCountEmitter` and `OrderTrackingEmitter` definitions and one representative pipeline
  test that builds a `PipelineStageRunners`. Purpose: the emitter-injection idiom the new
  fixture copies.
- `crates/slicer-runtime/tests/integration/main.rs` — 183 lines; full read is acceptable.
  Purpose: confirm `mod pipeline_tdd;` is a bare mount (no wrapper) before adding the new mod
  line.
- `crates/slicer-runtime/tests/common/mod.rs` — 604 lines. Read only `pipeline_config_base` and
  the surrounding fixture helpers.
- `crates/pnp-cli/tests/e2e_integration_tdd.rs` — 360 lines. Read only `pipeline_config_base`.
- `crates/slicer-runtime/tests/contract/dispatch_infill_output_tdd.rs` — read only the three
  `PipelineConfig` literals and their `// exhaustive:` waivers; locate them by symbol, not by a
  pinned line, then read ±20 lines around each.
- `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs` (241 lines) and
  `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs` (680 lines) — read only the
  assertions that quote `pipeline.rs` signature strings, and only if a signature change is being
  contemplated. Otherwise run them, do not read them.
- `crates/slicer-runtime/src/postpass.rs` — 518 lines. Read only
  `execute_postpass_with_capture`. Purpose: confirm the `layer_irs.to_vec()` deep copy precedes
  `.emit_gcode`, which is what makes the finalization seam sufficient.
- `docs/specs/support-independent-layer-z-split-plan.md` — 152 lines; direct full read.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies, and guest build artifacts
  under `modules/core-modules/*/wit-guest/target/` and
  `crates/slicer-wasm-host/test-guests/target/` — never load.
- `crates/slicer-gcode/src/emit.rs` — the emitter trait is frozen by design; 14 impls and 52
  call sites depend on it workspace-wide. **Do not open it to survey the impls — they are not
  there.** Only 1 of the 14 lives in this file; the other 13 are distributed across
  `crates/slicer-runtime/tests/` and `crates/pnp-cli/tests/`. The file stays closed because
  nothing in this packet changes the trait, not because it is a useful survey point.
- `crates/slicer-schema/wit/**` — no WIT change in this packet; the anchored records stay
  orphaned until `239b-anchored-wit-contract`.
- `crates/slicer-sdk/**`, `crates/slicer-macros/**`, `crates/slicer-wasm-host/**` — the F9
  `LayerCollectionIR` literal sites live here; this packet adds no IR field, so none of them is
  touched. Delegate any symbol lookup rather than browsing.
- `modules/core-modules/**` — support modules are `239c`'s surface (F8).
- Other packet directories under `docs/spec_packets/` — never modify.
- `crates/slicer-scheduler/src/execution_plan.rs` — read-only if consulted at all;
  `ExecutionPlan` stores no anchored entities and this packet does not change that.

## Expected Sub-Agent Dispatches

- Question: re-verify canonical `GCode::collect_layers_to_print`'s object/support merge
  discipline — the two-index walk, the `print_z_min` selection, the un-consume condition, and
  what EPSILON is compared against; scope: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`;
  return: `SUMMARY` ≤200 words or `SNIPPETS` ≤30 lines; purpose: Step 5, before implementing
  `synthesize_anchored_rows`.
- Question: enumerate every `PipelineConfig { ... }` struct-literal site and every
  `let PipelineConfig { ... }` destructuring pattern in the workspace, classifying each literal
  as FRU (its body contains a `..` rest) or exhaustive (it does not — an `// exhaustive:` comment
  does **not** make it FRU), and naming the base helper for each FRU site; the sweep must cover
  **all** `slicer-runtime` test binaries, `unit` / `contract` / `executor` / `integration` /
  `e2e`, not only `integration`; scope: `crates/ --include *.rs`; return: `LOCATIONS` ≤20
  entries; purpose: Step 1's blast-radius list, re-derived at edit time rather than trusted from
  this document.
- Question: confirm `crates/slicer-runtime/tests/integration/main.rs` mounts `pipeline_tdd`
  with a bare `mod` line and no inline `#[test] fn` wrapper, and report which of the ~20 wrapper
  functions exist; scope: `crates/slicer-runtime/tests/integration/main.rs`; return: `FACT` plus
  ≤20 `LOCATIONS`; purpose: Step 3, so the new file is mounted correctly on the first try.
- Question: list every `.emit_gcode(` call site reachable from production code (not tests);
  scope: `crates/ --include *.rs`; return: `FACT` (the count and the single production path);
  purpose: confirm before Step 6 that
  `slicer_runtime::postpass::execute_postpass_with_capture` is still the only production funnel,
  so the finalization seam is provably sufficient.
- Question: does any test in the workspace assert on `PipelineConfig`'s field count, field
  order, or `Debug` output; scope: `crates/ --include *.rs`; return: `FACT`; purpose: Step 1
  risk check for the additive field.

## Data and Contract Notes

- **IR/manifest contracts:** none changed. Synthesized rows are ordinary `LayerCollectionIR`
  values with `schema_version: CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` read from the live
  constant and **not** bumped. No version literal is pinned here — the current value is a mutable
  ledger fact; read `crates/slicer-ir/src/slice_ir.rs` and `docs/02_ir_schemas.md` at edit time.
  All non-anchored
  fields (`support_entity_identities`, `tool_changes`, `z_hops`, `annotations`, `retracts`,
  `travel_moves`, `speed_profiles`) take their empty/default values on a synthesized row unless
  the anchored collection supplies them; `ordered_entities` carries the collection's entities in
  the order the executor committed them.
- **WIT boundary:** untouched. The anchored records in
  `crates/slicer-schema/wit/deps/ir-types.wit` (`anchored-entity`,
  `anchored-geometry-contract`, `anchored-entity-provenance`, `anchored-event-runtime-hooks`,
  `ordered-event-collection`) remain referenced by zero interfaces, zero worlds, and zero
  function signatures (F7). Wiring them is `239b`'s work. Because no WIT text changes, guest
  artifacts stay WIT-fresh; the fingerprint gate still applies to dependency-closure changes,
  which is why the freshness constraint is listed above.
- **Determinism/scheduler constraints:** `synthesize_anchored_rows` is a pure function over an
  already-ordered `Vec<CommittedLayerEvent>`. The executor's committed ordering is the sole
  source of order; synthesis introduces no sort keyed on a hash map, no parallel iteration, and
  no floating-point tie-break — ties at exactly the epsilon boundary resolve by a fixed
  precedence (object row before anchored row) so the output is byte-stable. AC-3 checks this at
  the **executor call**, not at the pipeline: `force_parallel` is a positional `bool` parameter of
  `execute_anchored_event_collections_with_mode` (`crates/slicer-runtime/src/layer_executor.rs`),
  threaded on to `execute_anchored_event_collections_with_mode_and_feedrate`. It is **not** a
  config key, an env var, or a `PipelineConfig` field, and this packet does not create one —
  pipeline-level parallel determinism is explicitly out of scope (`packet.spec.md` §Scope
  Boundaries). AC-3 therefore mirrors
  `crates/slicer-runtime/tests/integration/anchored_parallel_determinism.rs`, which calls
  `execute_anchored_event_collections_with_mode(&plan, &entities, false, &module)` and the same
  call with `true`, and additionally lowers both returned collection sequences through
  `synthesize_anchored_rows` against identical fixed `CommittedLayerEvent::Model` rows to compare
  the resulting `(z, global_layer_index)` sequences. That existing test must also stay green
  across the switch.
- **Insertion-seam contract:** rows must be inserted at or before layer finalization.
  `run_postpass_with_thumbnail` receives `&layer_irs` immutably and
  `slicer_runtime::postpass::execute_postpass_with_capture` deep-copies with `layer_irs.to_vec()`
  before calling `.emit_gcode`, so nothing after finalization can add a row that reaches the
  emitter.

## Locked Assumptions and Invariants

- **Merge epsilon = `COORDINATE_TOLERANCE_UNITS` (10 units = 10⁻³ mm).** One constant governs
  both AC-2's on-grid/off-grid partition and AC-5's merge decision. Reversible only by changing
  the constant in `crates/slicer-ir/src/slice_ir.rs`, which is out of scope here.
- **Merge direction.** On merge, the anchored collection's entities are appended into the
  **object** row; the object row's `z` and `global_layer_index` win. This mirrors canonical
  behaviour, where the merged row keeps a single `print_z`.
- **`global_layer_index` for a solo synthesized row — the UPPER anchor layer's index.** A solo
  synthesized row adopts the `global_layer_index` of the **upper** global layer, i.e. the
  `CommittedLayerEvent::Model` row that immediately **follows** it in ascending Z. This is
  required by `docs/adr/0059-support-families-and-anchored-entities.md`: "A planar entity between
  model planes is **anchored to the upper global layer**, executes in ascending Z before that
  layer's ordinary model event." An earlier draft of this design locked the *nearest preceding*
  `Model` row's index; that attributed the row to the layer below and contradicted the ADR, and
  it is **superseded**. When the synthesized row has no upper `Model` row (it sits above every
  object layer), it adopts the index of the last `Model` row — the only anchor available.
  Consequences that make this a lock, not a preference: the emitted index sequence stays monotone
  non-decreasing (a solo row carries the index of the row that follows it, which is ≥ every index
  before it); no existing object row is renumbered; and AC-6's element-wise equality on
  `(len, global_layer_index, z)` holds trivially when `anchored_entities` is empty, because no
  synthesized row exists. A run **with** anchored entities therefore produces duplicate indices
  across adjacent rows by design — any consumer that assumes index uniqueness is out of contract
  and must be reported, not silently accommodated.
- **Z-spanning atomicity — one atomic block INSIDE the anchor layer's ordinary row.** Cited by
  phrase (the plan doc's "Z-spanning atomicity"), never by ordinal. A
  `AnchoredGeometryContract::ZSpanning` entity produces exactly one contiguous block of paths in
  the `ordered_entities` of its **anchor layer's ordinary `CommittedLayerEvent::Model` row**, at
  that layer's normal position — **not** on a separate synthesized row. This is ADR-0059's "may
  extend outside its anchor layer's Z interval while still executing at that layer's normal
  position". An earlier draft required a separate synthesized row covering the inclusive span;
  that contradicted the ADR and is **superseded**. Atomicity itself is unchanged: never
  per-object-layer fragments. AC-4 is the guard.
- **On-grid behaviour is unchanged — the plan doc's "same-Z support in ordinary ordering"
  invariant.** (Cited by phrase: §6's items 1–14 are an unnumbered prose parenthetical, and
  positional item 6 is "same-family merge preserving demand IDs", a different rule.) Entities
  inside the tolerance keep flowing through `append_same_z_entities` into their anchor layer's
  `ordered_entities` in the pre-existing order — ADR-0059's "same-Z support joins the ordinary
  model event". AC-N1 reuses the existing `anchored_event_ordering` test as the guard; this
  packet does not modify that test.
- **Support-disabled silence — the plan doc's "support-disabled emits nothing" invariant.** With
  `anchored_entities` empty, zero synthesized rows exist and no `;TYPE:Support` fragment appears.
  AC-N3 is the guard.
- **No signature in `crates/slicer-runtime/src/pipeline.rs` changes.** Enforced externally by
  two source-text guard tests. This is what keeps the packet's blast radius at six exhaustive
  literal sites plus two destructuring patterns.
- **Invariant 16 (a genuinely numbered list item in the plan doc's §6) / verification shape.**
  Every verification command names one test with `--exact`, tees to `target/test-output.log`, and
  asserts a non-zero matched count. **Every filter naming a test in
  `offgrid_rows_tdd.rs` or `pipeline_tdd.rs` must carry its module path**
  (`offgrid_rows_tdd::<fn>`, `pipeline_tdd::<fn>`), because those files are mounted by bare `mod`
  lines and libtest names their tests with the module prefix — a bare function name matches zero
  tests and reads green. The plan doc's item 16 records this exact failure (the 224 lesson).
  Only the top-level `#[test] fn` wrappers declared in
  `crates/slicer-runtime/tests/integration/main.rs` (`anchored_event_ordering`,
  `anchored_parallel_determinism`, `anchored_z_validation`, `anchored_z_span_validation`,
  `anchored_event_accounting`) carry no prefix. `cargo test --workspace` is never a step or AC
  command.

## Risks and Tradeoffs

- **Highest risk — the substrate has no production producer (F5/F6/F7).** Nothing in production
  constructs an `AnchoredEntity`: four production files mention the type
  (`crates/slicer-ir/src/lib.rs`, `crates/slicer-ir/src/slice_ir.rs`,
  `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-scheduler/src/execution_plan.rs`)
  and all **9** literal construction sites (measured 2026-08-28 via
  `rg -n 'AnchoredEntity \{' crates/`, discounting the `pub struct` definition and the
  `-> AnchoredEntity {` fn-signature lines) are tests; the module-commit path is a closed host
  loop with no guest writer; the WIT records are orphaned. Every AC here is therefore integration-level, driven by a hand-built
  `ExecutionPlan` and an explicit `PipelineConfig.anchored_entities` payload. **Mitigation:**
  state it in `packet.spec.md`, `requirements.md` §Problem Statement, and here; exclude every
  fixture-slice artifact, human-validation gate, and `tmp/` evidence file from this packet's
  closure; leave the real-slice proof to `239c-support-layer-height-producer`. The residual risk
  is that the seam ships correct-by-test and still needs adjustment when a real producer appears
  — accepted, because the alternative is shipping nothing until `239c`.
- **Duplicate-body drift.** `run_pipeline_with_events` and `run_pipeline_core` must receive
  equivalent switches in two different steps. If Step 7 diverges from Step 6's shape, the two
  entry points silently produce different row sequences. **Mitigation:** both call the same
  `synthesize_anchored_rows`; Step 7's exit condition names Step 6's shape explicitly.
- **A third body exists outside `slicer-runtime`.** `crates/pnp-cli/src/visual_debug.rs` was
  never recorded by packet 239 and is easy to miss again. **Mitigation:** it is its own step
  (Step 8) with its own exit condition.
- **`global_layer_index` duplication.** The locked index rule intentionally allows adjacent rows
  to share an index. If a downstream consumer keys on index uniqueness, it will misbehave on
  runs with off-grid rows. **Mitigation:** AC-3 pins the `(z, global_layer_index)` pair sequence
  so any change to the rule is caught; the alternative (renumbering) was rejected for a larger
  blast radius. Unmeasured: whether any consumer today assumes uniqueness — the Step 6 dispatch
  on production `.emit_gcode` funnels is the cheapest probe.
- **Struct-literal gate churn.** New `LayerCollectionIR` and `PipelineConfig` test literals must
  satisfy `cargo xtask check-literals`. **Mitigation:** the gate is in every step's verification
  set, not deferred to closure.
- **Behaviour-neutral refactor mistaken for a fix.** Step 4 is easy to mis-report as closing a
  routing hole. **Mitigation:** F1 is stated in three places and Step 4's exit condition asserts
  that no AC changed colour.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 6 — the `run_pipeline_core` switch plus the finalization-seam
  insertion, read against both the executor's committed return tuple and the postpass deep-copy
  contract)
- Highest-risk dispatch and required return format: canonical
  `GCode::collect_layers_to_print` merge discipline — `SUMMARY` ≤200 words or `SNIPPETS`
  ≤30 lines, never a file body. Runner-up: the `PipelineConfig` literal-site sweep —
  `LOCATIONS` ≤20 entries.

## Open Questions

- `[FWD]` **`global_layer_index` for solo synthesized rows.** The design locks "adopt the
  **upper** global layer's index", mandated by `docs/adr/0059-support-families-and-anchored-entities.md`
  ("anchored to the upper global layer"); see §ADR Conformance and §Locked Assumptions. This is
  no longer open — the rule itself is settled. The open part is whether any downstream consumer
  assumes index uniqueness across the emitted row slice. Implementer-resolvable via the Step 6
  production-`.emit_gcode`-funnel dispatch; if a uniqueness assumption is found, record it and
  raise it rather than switching to renumbering, which was rejected for blast radius. Not an
  activation blocker.
- `[FWD]` **Epsilon-boundary tie precedence.** When `|dz|` equals the epsilon exactly, the
  design merges (`<=`, matching canonical's `|dz| <= EPSILON`) and orders the object row first.
  Canonical's own tie behaviour at exact equality is a float comparison and may be
  unobservable in practice. Implementer may proceed with the stated rule; if the Step 5 dispatch
  returns something different, follow canonical and update AC-5's fixture rather than the rule.
- `[FWD]` **Where the shared route-partition helper lives.** Step 4 may keep it private inside
  `crates/slicer-runtime/src/layer_executor.rs` or hoist it beside `synthesize_anchored_rows`.
  Recommendation: keep it private in `layer_executor.rs`, since both call sites are there and
  hoisting would widen the public surface for no gain. Implementer-resolvable; record the
  choice in the Step 4 commit message.
- `[FWD]` **Whether `crates/pnp-cli/src/visual_debug.rs` needs its own AC.** No AC in
  `packet.spec.md` targets it; Step 8 is proven by the existing visual-debug suite plus
  `cargo check --workspace --all-targets`. If the implementer finds an existing visual-debug
  test that would meaningfully assert off-grid row presence, adding it is welcome but is not
  required for closure and must not become a new AC (ACs are `packet.spec.md`-owned).

No `[BLOCK]` questions. `packet.spec.md` records no activation blockers and no dependencies.

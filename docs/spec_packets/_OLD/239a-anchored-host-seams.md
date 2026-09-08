---
status: implemented
packet: 239a-anchored-host-seams
supersedes: 239-support-independent-layer-z
task_ids:
  - TASK-399
  - TASK-400
  - TASK-401
  - TASK-402
  - TASK-403
  - TASK-404
  - TASK-405
  - TASK-406
  - TASK-407
  - TASK-408
---

# 239a-anchored-host-seams

## Goal

Give the host an anchored-entity input seam, switch all three non-anchored
`execute_per_layer*` call sites to `execute_per_layer_with_committed_anchored_events`, and
lower `CommittedLayerEvent::Anchored` collections into ordinary `LayerCollectionIR` rows at
their declared Z — merged against object rows by the canonical `|dz| <= EPSILON` rule — so
off-grid support work survives finalization and postpass into G-code.

## Problem Statement

Packet `239-support-independent-layer-z` is superseded. Its central premise — that the
anchored-event substrate "already carries everything needed" and only two blockers stand in the
way — was measured false during its `/swarm` run and replaced by
`docs/specs/support-independent-layer-z-split-plan.md`, whose findings F1–F9 are the plan of
record. This packet is the first of that plan's three successors and inherits 239's reserved
`TASK-399..TASK-408` range.

Three of those findings define this packet's gap, and one refutes a gap 239 claimed:

- **F1 refutes 239's Blocker 1.** `is_same_z_entity` (`crates/slicer-runtime/src/layer_executor.rs`)
  has exactly three references — its definition, a positive filter in `append_same_z_entities`,
  and a negated filter in `execute_anchored_event_collections`. Those two filters are **exact
  complements**, so the executor's routing partition is **already total**. An off-grid
  `AnchoredGeometryContract::Planar { z }` does not fall through a gap; it is rejected by the
  ordinary route and therefore caught by the anchored route. 239's `requirements.md` ("matches
  nothing, so it is silently excluded") and `design.md` ("matches neither route and vanishes")
  are both wrong. Consequence for this packet: AC-2 and AC-N2 are **not** red at the executor
  level and cannot be made red there. They are genuinely red only at pipeline level, which is
  where this packet places them.
- **F2 — the real blocker.** No production call site invokes
  `execute_per_layer_with_anchored_events` or `execute_per_layer_with_committed_anchored_events`.
  Three non-anchored call sites exist and must switch: two in
  `crates/slicer-runtime/src/pipeline.rs` (`run_pipeline_with_events` and `run_pipeline_core`)
  and one in `crates/pnp-cli/src/visual_debug.rs`, which 239 never recorded.
- **F3 — no injection seam.** `PipelineConfig` (`crates/slicer-runtime/src/pipeline.rs`) has no
  anchored-entity field, so no public entry point can carry anchored work into the run.
- **F4 — no emission representation.** `LayerCollectionIR` (`crates/slicer-ir/src/slice_ir.rs`)
  carries exactly one `z: f32` and one `global_layer_index: u32` per row, so a row *is* a whole
  layer at a single Z. `CommittedLayerEvent::Anchored(OrderedEventCollection)` currently has
  nowhere to go once the executor produces it.

One coherent slice: F3 opens the input seam, F2 routes through the executor entry point that
already exists, F4 is closed by lowering anchored collections into ordinary `LayerCollectionIR`
rows at their declared Z, merged against object rows by the canonical
`GCode::collect_layers_to_print` (`GCode.cpp`) rule. All three sit inside one crate plus one
`pnp-cli` call site, and none of them is meaningful without the other two.

### Honest limitation (repeated from `packet.spec.md`, restated in `design.md` §Risks)

**Nothing in production constructs an `AnchoredEntity` today.** The type appears in exactly four
production files (`crates/slicer-ir/src/lib.rs`, `crates/slicer-ir/src/slice_ir.rs`,
`crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-scheduler/src/execution_plan.rs`)
and every one of its literal construction sites is a test (F5). Re-derived 2026-08-28 with
`rg -n 'AnchoredEntity \{' crates/`, discounting the `pub struct` definition and the
`-> AnchoredEntity {` fn-signature lines: **9** literal sites across 7 test files
(`crates/slicer-ir/tests/ir_validation_tdd.rs` ×2,
`crates/slicer-scheduler/tests/integration/capability_derived_anchor_closure.rs` ×2, and one each
in the five `crates/slicer-runtime/tests/integration/anchored_*.rs` files). Zero production
literals — the qualitative conclusion F5 draws is unchanged; only its count was wrong.
The module-commit path is a closed host
loop with no guest writer (F6), and the anchored WIT records are referenced by zero interfaces,
zero worlds, and zero function signatures (F7).

Therefore every acceptance criterion in `packet.spec.md` is an **integration-level** truth driven
by a hand-built `ExecutionPlan` plus an explicit `PipelineConfig.anchored_entities` payload. No
real slice exercises this path until `239c-support-layer-height-producer` lands a producer. This
packet must not claim otherwise. Its closure must not rest on any fixture-slice artifact, human
validation gate, or `tmp/` evidence file — those belong to `239c`, and asserting them here would
be vacuous evidence.

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

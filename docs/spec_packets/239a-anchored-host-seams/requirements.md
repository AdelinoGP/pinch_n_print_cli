# Requirements: 239a-anchored-host-seams

## Packet Metadata

- Grouped task IDs: `TASK-399`, `TASK-400`, `TASK-401`, `TASK-402`, `TASK-403`, `TASK-404`,
  `TASK-405`, `TASK-406`, `TASK-407`, `TASK-408`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented`
- Aggregate context cost: `M`

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

## In Scope

Authoritative full scope. Anything not listed here is out of scope.

The exact code surface — which functions and literal sites are edited, and in what order — is
owned by `design.md` §Code Change Surface and §Files in Scope, and is **not** duplicated here.
This section states what is in scope, not where it lands.

- **Injection seam.** One additive field on `PipelineConfig`,
  `anchored_entities: Vec<slicer_ir::AnchoredEntity>`, plus its **complete** struct-literal blast
  radius. `PipelineConfig` has no `Default` impl and is not `#[non_exhaustive]`, so every literal
  that lacks a `..` rest must name the new field, and every exhaustive destructuring pattern must
  bind or `..`-ignore it. The radius spans more than one `slicer-runtime` test binary: an
  `integration`-only sweep is incomplete, and the `contract` binary carries exhaustive literals
  too. `design.md` enumerates the sites; the implementer re-derives them at edit time.
- **Executor switch, three call sites.** The two non-anchored per-layer call sites in
  `crates/slicer-runtime/src/pipeline.rs` and the one in `crates/pnp-cli/src/visual_debug.rs` all
  move to `execute_per_layer_with_committed_anchored_events`. The two `pipeline.rs` entry points
  do not forward to one another — an inline NOTE comment in `run_pipeline_with_events`'s body
  records that it deliberately keeps a duplicated body because it emits a bare G-code body with
  no thumbnail/CONFIG_BLOCK wrapper — so the switch is made twice inside
  that file, not once.
- **Row synthesis.** A pure function lowering a `Vec<CommittedLayerEvent>` into an ordered
  `Vec<LayerCollectionIR>`, merging an anchored collection into an object row iff their Z differ
  by at most the merge epsilon and otherwise emitting the lower Z as a solo row — matching
  canonical `GCode::collect_layers_to_print` (`GCode.cpp`).
- **Insertion at the finalization seam.** Synthesized rows enter the layer-row vector at or
  before layer finalization — the last mutable seam before the postpass path deep-copies the
  slice and hands it to `.emit_gcode`. Both `pipeline.rs` bodies get the identical treatment.
- **ADR-0059 conformance.** Anchor attribution and Z-spanning placement follow
  `docs/adr/0059-support-families-and-anchored-entities.md`: a planar off-grid entity is anchored
  to the **upper** global layer and executes in ascending Z before that layer's ordinary model
  event; a Z-spanning entity executes as one atomic block at its anchor layer's **normal
  position**, inside that layer's ordinary row rather than on a row of its own; same-Z support
  joins the ordinary model event. See `design.md` §ADR Conformance.
- **Behavior-neutral route-partition refactor.** Extracting the complementary
  `is_same_z_entity` / `!is_same_z_entity` filter pair into one shared named helper, per F1.
  Clarity only — it flips no AC.
- **Test fixture.** A payload-capturing `GCodeEmitter` that stores the `&[LayerCollectionIR]`
  rows it is handed, so an AC can assert on a synthesized row's `z`. No such fixture exists
  today: the existing mocks capture only `.len()` or ignore the payload entirely.
- **Doc registration.** `docs/07_implementation_status.md` rows for `TASK-399..TASK-408`,
  `docs/specs/support-parity-gap-register.md` edits (re-point `G-02`; add a row for the
  production-dead anchored substrate, F5/F6/F7), and the split-plan queue row.

## Out of Scope

- **`GCodeEmitter::emit_gcode`'s signature.** Measured 2026-08-28: **14** `impl GCodeEmitter for`
  blocks and **52** `.emit_gcode(` call sites workspace-wide. Only **one** impl is in
  `crates/slicer-gcode/src/emit.rs` (the production `DefaultGCodeEmitter`); the other **13 are
  distributed across test crates** (`crates/slicer-runtime/tests/`, `crates/pnp-cli/tests/`) —
  they are not concentrated in `emit.rs`. The signature is frozen; `crates/slicer-gcode/src/emit.rs`
  is not edited by this packet at all. See `design.md` §Rejected alternatives.
- **Pipeline-level parallel determinism.** No `force_parallel` config key, env var, or
  `PipelineConfig` field exists; `force_parallel` is a positional `bool` parameter of
  `execute_anchored_event_collections_with_mode`
  (`crates/slicer-runtime/src/layer_executor.rs`). This packet neither creates such a knob nor
  threads a parallel-mode selector through `run_pipeline_core` / `PipelineConfig`. AC-3 is scoped
  to the executor call instead.
- **Any WIT, IR, schema, SDK, manifest, or config-key change.** `LayerCollectionIR` gains no
  field and no version bump — synthesized rows **reuse the live
  `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` constant, whatever its value is at edit time**.
  This packet pins no version literal: the schema version is a mutable ledger fact and an earlier
  draft's `1.2.0` was already stale when written. `PipelineConfig` is a host-side orchestration
  struct, not an IR or wire type; it carries no schema version and crosses no component boundary.
- **Guest-side transmission of anchored work** (F6/F7: WIT world/interface wiring, host
  lift/lower glue, SDK drain glue) → `239b-anchored-wit-contract`.
- **A production producer of `AnchoredEntity`**, support-Z decoupling from `LayerPlanView`
  (F8), the `independent_support_layer_height` key, and the measure-first `height_delta`
  verdict → `239c-support-layer-height-producer`.
- **Module edits.** `modules/core-modules/**` is untouched.
- **Fixture-slice artifacts, human-validation gates, and `tmp/` evidence.** Structurally
  vacuous here (see §Problem Statement honest limitation); they belong to `239c`.
- **Renumbering existing `global_layer_index` values.** Synthesized rows adopt an existing
  index (see `design.md` §Locked Assumptions); no object row is renumbered.

## Authoritative Docs

- `docs/specs/support-independent-layer-z-split-plan.md` — 152 lines; direct full read. The
  plan of record: findings F1–F9, the canonical `GCode::collect_layers_to_print` merge rule,
  the coordinate-discipline note, the packet queue, and the gap-register instructions.
- `docs/adr/0059-support-families-and-anchored-entities.md` (`Status: accepted`) — the governing
  ADR. Its anchored-entity paragraph is normative for anchor attribution (upper global layer),
  planar ordering (ascending Z before the anchor layer's ordinary model event), Z-spanning
  placement (at the anchor layer's normal position), and same-Z joining. Short; read directly.
- `docs/specs/support-families-anchored-entities-plan.md` — 755 lines; **delegate or read
  ranged only**, never full-read. **Cite §6 invariants by quoted phrase, never by ordinal:**
  items 1–14 are an unnumbered semicolon-separated prose parenthetical, so "invariant 6" does not
  resolve (positionally it is "same-family merge preserving demand IDs", a different rule).
  The phrases this packet depends on are "same-Z support in ordinary ordering" (on-grid entities
  append into their anchor layer in pre-existing order — AC-N1), "Z-spanning atomicity" (AC-4),
  "serial/parallel determinism" (AC-3), "support-disabled emits nothing" (AC-N3), and "planar
  anchored output on declared Z" (AC-1). Item **16** is a genuinely numbered list item and may be
  cited by number: every verification asserts a non-zero matched-test count.
- `docs/02_ir_schemas.md` — delegated `FACT` only: read the live value of
  `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` and confirm synthesized rows reuse it rather than
  bump it. Do **not** assert a specific version literal — it is a mutable ledger fact. Do not
  full-read.
- `docs/08_coordinate_system.md` — consulted only through the `coord-system` constraint bullet
  in `design.md`. Do not full-read.
- `CLAUDE.md` §Test Discipline and §Guest WASM Staleness — already in every agent's context.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `collect_layers_to_print`: the canonical
  object/support row-merge rule that this packet's row synthesis must match (AC-5). A delegated
  dispatch during the 239 swarm run returned: two independent indices,
  `print_z_min = min(object_layer->print_z, support_layer->print_z)`, un-consume whichever side
  exceeds `print_z_min` by more than EPSILON, merge into one row iff `|dz| <= EPSILON`, else the
  **lower** one emits a solo row and the other retries. Re-verify by dispatch before implementing
  Step 5; this paragraph is a summary of a prior return, not a substitute for inspection.

Citation policy (E7): canonical behaviour is cited by file + function only, never by line
number, and only what a delegated dispatch actually returned.

## Acceptance Summary

Criteria are owned by `packet.spec.md` and referenced here by ID only; never copied. The
measurable refinements below are additions absent from the Given/When/Then text, not restatements.

- Positive: `AC-1` through `AC-6`.
- Negative: `AC-N1`, `AC-N2`, `AC-N3`.

Refinements:

- `AC-1` / `AC-N2`: the off-grid plane `z: 3000` is in canonical internal units — 3000 units ×
  10⁻⁴ mm = 0.3 mm. Comparisons between a declared planar Z and a row's `z: f32` must go through
  `mm_to_units(row.z)` in i64 space, never float-compare mm against units.
- `AC-2` / `AC-N1`: "on-grid" means within `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`,
  which re-exports the module-level `COORDINATE_TOLERANCE_UNITS: i64 = 10`
  (`crates/slicer-ir/src/slice_ir.rs`) — 10 units = 10⁻³ mm. This is the same constant the merge
  epsilon uses, so AC-2's partition and AC-5's merge cannot disagree.
- `AC-N1` reuses the **existing** test name `anchored_event_ordering`
  (`crates/slicer-runtime/tests/integration/anchored_event_ordering.rs`, wrapped by an inline
  `#[test] fn` in `crates/slicer-runtime/tests/integration/main.rs`). Because it is a top-level
  wrapper, libtest names it `anchored_event_ordering` with **no** module prefix — the one AC
  whose filter is correct in bare form. It is a regression guard: it must stay green unchanged
  across every step. It is not authored by this packet.
- **Module-prefixed test names.** Every other AC test lands in
  `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs` (mounted by a bare `mod` line,
  `#[test]` in place) or in `pipeline_tdd.rs` (same convention), so libtest names them
  `offgrid_rows_tdd::<fn>` and `pipeline_tdd::<fn>`. Filters below use the prefixed form. A bare
  function name would match zero tests and exit 0 — the failure the plan doc's numbered
  invariant 16 exists to prevent.
- `AC-3` is an **executor-level** criterion, not a pipeline-level one: it drives
  `execute_anchored_event_collections_with_mode(&plan, &entities, false|true, &module)` — the
  idiom `anchored_parallel_determinism.rs` uses — and lowers both results through
  `synthesize_anchored_rows`. There is no pipeline `force_parallel` knob and this packet does not
  create one. AC-3 compares `(z, global_layer_index)` pair sequences, so it also guards the
  index-assignment rule locked in `design.md` (the **upper** anchor layer's index, per ADR-0059)
  — a nondeterministic index choice fails AC-3 even when Z ordering is stable.
- `AC-4` asserts the Z-spanning block lands **inside its anchor layer's ordinary row**, at that
  layer's normal position, not on a separate synthesized row. This is ADR-0059's requirement;
  atomicity (one contiguous block, never per-object-layer fragments) is unchanged.
- `AC-6`'s "recorded pre-change baseline" is captured in Step 2, **before** any executor switch
  lands, using the payload-capturing emitter on an existing support-free run. A baseline
  recorded after the switch would prove nothing.
- Cross-packet impact: none of `239b`/`239c` is blocked or unblocked mid-packet; `239c` gains
  its host-side prerequisite only at closure. No other active packet reads or writes
  `PipelineConfig`'s field list, `layer_executor.rs`'s anchored routes, or
  `slicer_runtime::postpass::execute_postpass_with_capture`.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands. Every
cargo-test row names one test with `--exact`, tees to `target/test-output.log`, and appends the
in-run zero-guard `test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` so a zero-match
run can never read green (invariant 16). `cargo test --workspace` appears **nowhere** in this
matrix and must not be used as a step or AC command.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_support_row_emitted_at_declared_z --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-1 off-grid row reaches emission | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::every_same_z_support_entity_routes_exactly_once --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-2 routing totality end to end | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_row_order_identical_serial_and_parallel --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-3 executor-level "serial/parallel determinism" | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::zspanning_support_entity_emits_atomic_single_block --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-4 "Z-spanning atomicity" — block inside the anchor layer's ordinary row (ADR-0059) | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_row_merge_matches_canonical_epsilon_rule --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-5 canonical merge rule | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::support_free_slice_row_sequence_unchanged --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-6 empty-collection equivalence | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- anchored_event_ordering --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N1 "same-Z support in ordinary ordering" preserved (pre-existing top-level wrapper test, no module prefix; regression guard) | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::offgrid_entity_never_merged_into_grid_layers --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N2 no grid collapse | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- offgrid_rows_tdd::support_disabled_pipeline_emits_no_anchored_rows --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | AC-N3 "support-disabled emits nothing" | FACT pass/fail |
| `cargo test -p slicer-runtime --lib -- anchored_rows::tests::merge_within_epsilon_produces_one_row --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | Step 5 pure-function proof; source-module `#[cfg(test)]` tests are reachable ONLY via `--lib` | FACT pass/fail |
| `cargo test -p slicer-runtime --lib -- anchored_rows::tests::beyond_epsilon_lower_z_emits_solo_row --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | Step 5 canonical un-consume branch | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- pipeline_tdd::payload_capturing_emitter_records_row_sequence --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | Step 2 fixture proof + AC-6 baseline capture | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- anchored_parallel_determinism --exact 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | Pre-existing executor determinism guard; must stay green across the switch | FACT pass/fail |
| `cargo test -p slicer-runtime --test visual_debug_agent_overhead_tdd 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | Source-text guard asserting `pipeline.rs` signatures verbatim; tripwire for an accidental signature change | FACT pass/fail |
| `cargo test -p pnp-cli --test visual_debug_typed_tap_capture_tdd 2>&1 \| tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` | Second source-text signature guard; same tripwire | FACT pass/fail |
| `cargo check --workspace --all-targets` | Type gate; catches the `PipelineConfig` struct-literal blast radius including the two destructuring patterns | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate; required before committing | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal churn gate; required before committing. Synthesized-row and fixture test literals of `LayerCollectionIR` and `PipelineConfig` must carry `..` FRU or an `// exhaustive:` waiver | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest freshness; **exit code only** — 0 fresh, 1 stale, 3 `wasm-tools` missing. Never grep for `STALE:` | FACT exit code |
| `rg -q '^\s*- \[[ x]\] TASK-399 ' docs/07_implementation_status.md && rg -q '^\s*- \[[ x]\] TASK-408 ' docs/07_implementation_status.md` | Doc Impact: task registration (Step 10) | FACT pass/fail |
| `rg -q '239a-anchored-host-seams' docs/specs/support-parity-gap-register.md` | Doc Impact: gap-register re-point (Step 10) | FACT pass/fail |
| `rg -q '^\| 1 \|.*\| closed \|.*docs/spec_packets/239a-anchored-host-seams' docs/specs/support-independent-layer-z-split-plan.md` | Doc Impact: split-plan queue row (Step 10) | FACT pass/fail |

Commands must have small, parseable output suitable for delegation. `crates/slicer-runtime`
declares features `default = ["report"]` and `report = []` with **no** `required-features` on any
test target, and no test file in the crate opens with `#![cfg(feature = ...)]`, so the
feature-gated-blindness hazard in `CLAUDE.md` §Test Discipline does not apply to this suite. Do
not add `--features` flags to these commands on the assumption that it does.

## Step Completion Expectations

Cross-step invariants only; per-step pre/postconditions live in `implementation-plan.md`.

- **Baseline before switch.** Step 2 must record AC-6's row-sequence baseline before Step 6 or
  Step 7 changes any executor call. Recording it afterwards makes AC-6 tautological.
- **Red before green.** Step 3's tests must be observed failing for the stated reason (the
  synthesized off-grid row is absent from the captured sequence), not failing to compile for an
  unrelated reason. Steps 6 and 7 are the only steps permitted to turn them green.
- **Step 4 flips nothing.** Per F1 the routing partition is already total, so the shared-helper
  extraction is behavior-neutral by construction. If any AC changes colour across Step 4, the
  extraction was not equivalent — revert and re-derive.
- **Signature freeze is shared state.** Two source-text guard tests assert `pipeline.rs`
  signature strings verbatim: `crates/slicer-runtime/tests/visual_debug_agent_overhead_tdd.rs`
  and `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs`. The design keeps every
  signature in `pipeline.rs` unchanged, so both stay green as tripwires. If any step finds a
  signature change unavoidable, that step must add **both** files to its own edit list and its
  own verification commands before making the change — never defer the fallout.
- **Shared scratch file.** Steps 3, 5, 7, and 9 all append tests to
  `crates/slicer-runtime/tests/integration/offgrid_rows_tdd.rs`. Step 3 creates it and adds the
  single `mod offgrid_rows_tdd;` line to `crates/slicer-runtime/tests/integration/main.rs`. Its
  test functions carry `#[test]` in place — matching `pipeline_tdd`, which is mounted by a bare
  `mod` line — so no later step touches `main.rs`. Any later step that finds itself editing
  `main.rs` has diverged from the design. **Consequence:** libtest names every test in that file
  `offgrid_rows_tdd::<fn>`. Do **not** switch to top-level wrappers to shorten the filters; that
  would force later steps to edit `main.rs` and contradict this rule. Prefix the filters instead.
- **Ordering.** Steps run 1 → 10 in order. Steps 6 and 7 both consume Step 5's synthesis
  function; Step 7 must rebase onto Step 6's shape rather than duplicate its insertion logic in
  a second form.
- **Ledger facts are re-derived at Step 10, not quoted.** The next free `G-` row, the next free
  `DEV-` id, and the `docs/07_implementation_status.md` high-water mark are mutable shared state.
  The split plan quotes `G-27`, `DEV-157`, and `TASK-507` as their values *at split time*; treat
  those as provenance, not as facts about the tree — `G-27` in particular is contested and must
  not be relied on. Re-derive each at the moment of the edit.

## Context Discipline Notes

- `crates/slicer-runtime/src/layer_executor.rs` is 3886 lines — **never full-read**. Read only
  the anchored-route neighbourhood (`execute_per_layer_with_anchored_events`,
  `CommittedLayerEvent`, `execute_per_layer_with_committed_anchored_events`,
  `is_same_z_entity`, `append_same_z_entities`) and the
  `execute_anchored_event_collections` family separately; ranges are pinned per step in
  `implementation-plan.md`.
- `crates/pnp-cli/src/visual_debug.rs` is 2340 lines and
  `crates/slicer-ir/src/slice_ir.rs` is 3141 lines — ranged reads only, never full.
- `docs/specs/support-families-anchored-entities-plan.md` is 755 lines — delegate a `SUMMARY`
  for the five named invariants rather than reading it.
- Tempting read to skip: `crates/slicer-gcode/src/emit.rs` (1359 lines). The design does not
  change `emit_gcode`, so the only fact needed from it is the trait method's existing shape,
  which is already recorded in `design.md`. **Do not open it to "check the impls" — 13 of the 14
  `impl GCodeEmitter for` blocks are not in that file at all**; they are distributed across
  `crates/slicer-runtime/tests/` and `crates/pnp-cli/tests/`. `emit.rs` holds exactly one (the
  production `DefaultGCodeEmitter`). It stays closed because nothing here changes the trait.
- Heavy-dispatch return limits: the OrcaSlicer `collect_layers_to_print` dispatch returns
  `SUMMARY` ≤200 words or `SNIPPETS` ≤30 lines. The `PipelineConfig` literal-site dispatch
  returns `LOCATIONS` ≤20 entries. Neither may return file bodies.
- Test-log discipline: `target/test-output.log` is overwritten by every run. Capture the fact
  you need from a run before launching the next one; never re-run a test to see more output.

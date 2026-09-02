# Implementation Plan: 239d-support-coarse-floating-planes

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Run `cargo xtask build-guests --check` (exit code only) before every slice-level evidence run (AC-5).

## Steps

### Step 1: Measure-first — decimation reconciliation and baselines

- Task IDs: `TASK-523`
- Objective: Record the pre-239d state so the design decisions are measured, not assumed:
  (a) the decimation reconciliation — the tree planner's layer loop has no `support_step`,
  the host `build_emit_schedule` never reaches the meshed-object planner path (both planners
  ignore `SupportGeometryView` there; the tree's only read is the mesh-less legacy contact
  fallback inside `SupportPlanner::plan_for_object` in the tree planner's `lib.rs`), and the
  traditional `support_step` decimates on-grid; (b) the coarse-direction baseline —
  `support_layer_height_mm = 0.3` over `layer_height` 0.2 emits 0 off-grid rows, and over
  0.1 emits support on 85/299 rows normal(auto) (tree(auto) exploratory run: 248 support
  rows); (c) the AC-N1 baseline — the
  disabled `;Z:` sequence with `support_layer_height_mm = 0.3` (flag false), captured before
  any planner edit.
- Precondition: `cargo xtask build-guests --check` exits `0`; the 239c suite is green
  (spot-check AC-1 and AC-N1 commands).
- Postcondition: the measurement record (the three numbers, the root cause, the baseline
  `;Z:` sequence) filed under `TASK-523` in `docs/07_implementation_status.md`; the baseline
  sequence available for Step 4's hard-coded const.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/support_geometry.rs` - lines ~51-127 (`build_emit_schedule`, `execute_support_geometry`)
  - `crates/slicer-runtime/src/builtins/support_geometry_producer.rs` - lines ~40-60
  - `modules/core-modules/tree-support-planner/src/lib.rs` - lines ~2260-2280 (layer loop head), ~1845-1855 (`_support_geometry` param), and ~2160-2190 (the mesh-less legacy read)
  - `modules/core-modules/traditional-support-planner/src/lib.rs` - lines ~170-180 (`_support_geometry`) and ~360-370 (`support_step`)
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` - the helper block only (~35-260)
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (the `TASK-523` measurement record)
  - `tmp/` (the baseline slice artifacts, e.g. `tmp/p239d-disabled-coarse.gcode`)
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/support_geometry.rs`, `crates/slicer-runtime/src/builtins/support_geometry_producer.rs` (read-only)
  - `OrcaSlicerDocumented/...` (delegate)
- Blast-radius discipline: not applicable (no struct field or schema constant changes).
- Expected sub-agent dispatches:
  - Question: confirm the tree planner's layer loop has no `support_step`-style decimation and locate every site that reads `_support_geometry` in both planners (expected: exactly one — the tree's mesh-less legacy contact fallback at tree `lib.rs` ~2173; the traditional parameter at `lib.rs` ~174 is never read); scope: the two planner `lib.rs` files; return: `FACT` (yes/no + the loop head line + read sites)
  - Question: run the two baseline slices (0.3 over 0.2, 0.3 over 0.1) and the disabled slice, and return the distinct `;Z:` counts, the off-grid row counts, and the support-row counts; scope: `crates/slicer-runtime/`; return: `FACT` (six numbers plus the disabled `;Z:` sequence length)
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-independent-layer-z-split-plan.md` - whole file (short)
  - `docs/DEVIATION_LOG.md` - rows `DEV-159`..`DEV-163` only
- OrcaSlicer refs: none (measurement step).
- Verification:
  - `rg -q 'TASK-523' docs/07_implementation_status.md` - FACT pass/fail
  - `cargo xtask build-guests --check && echo FRESH` - FACT exit code
- Exit condition: the `TASK-523` record exists with the decimation root cause, the coarse
  baseline numbers (family-labeled per the live record: normal(auto) 85/299 support rows
  over 0.1; tree(auto) exploratory run 248), and the disabled baseline sequence; the
  baseline is captured before any planner edit.

### Step 2: Tree planner coarse derivation

- Task IDs: `TASK-524`
- Objective: In `SupportPlanner::plan_for_object`
  (`modules/core-modules/tree-support-planner/src/lib.rs`), for each consecutive demanded
  bracket pair where the binding coarse predicate holds (configured nonzero pitch >=
  `local_support_gap`, the maximum positive anchor-Z difference between consecutive
  surviving support-bearing rows of that same `(object_id, region_id)` contiguous run
  covered by the bracket; these rows are already available to both planner callers; where
  the predicate fails, the bracket keeps the existing 239c
  finer derivation), bracket the demanded planes per the **binding Q1
  decision**: partition by `(object_id, region_id)` and contiguous run; with count(interface-role
  planes (`TopInterface`/`BaseInterface`/`BottomInterface`)) >= 2 the bracket set is the
  sorted/deduplicated interface planes (endpoints not added); with count < 2 that set is
  supplemented with the run's first
  and last surviving support-bearing rows, then sorted/deduplicated by `anchor_z` —
  a run with exactly one genuine interface plane keeps it as a bracket (never demoted to
  body). Generate the stack between brackets
  by the **tree-family** rule of `plan_layer_heights` (`TreeSupport.cpp`):
  `n = ceil(dist / pitch)` (**no** EPSILON bias), `step = dist / n`, planes at
  `below_z + k * step`, last aligned to `above_z`. Apply the canonical grouping of
  `generate_support_layers` (`SupportCommon.cpp`) — group candidate print-Z within
  `EPSILON`, take the midpoint; the group **minimum-height** rule is
  representation-inapplicable (`SupportPlanEntry` has no height field; effective row height
  derives from adjacent `anchor_z`) and is not reproduced. Replace only the **non-interface
  rows strictly inside each bracket pair** (genuine interface bracket entries always
  remain) with the stack planes per the **binding Q2 decision**: clone the lower bracket's geometry, rewrite
  roles to `SupportBody`, capture the source `global_layer_index` into the local duplicate
  key and clone-source provenance decision only, and assign the emitted entry's final
  `global_layer_index` from the per-plane DEV-163 synthetic identity map
  (`BTreeMap<i64, i32>`); preserve other provenance fields. The stable dedup applies to
  **synthesized candidates only** — surviving real entries are never deduplicated by
  emitted `global_layer_index`. Entries nondecreasing in
  `anchor_z` per object in original output order, distinct planes strictly increasing,
  identity key `(source global_layer_index, object_id, region_id, ordered body_ids,
  anchor_z)` deduplicated, and
  `anchor_layer_index` = true-nearest layer by absolute Z distance with lower-index tie
  break. The finer direction (bracket pairs where the binding predicate fails — configured
  nonzero pitch < `local_support_gap`, decided per bracket pair, never from the
  first/contact layer height alone) and the 0.0 sentinel are unchanged.
- Precondition: Step 1 complete; the 239c tree tests are green.
- Postcondition: the coarse direction emits free-floating `anchor_z` values; the finer
  direction and the sentinel are bit-identical; AC-2 and AC-N3 tests pass.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/tree-support-planner/src/lib.rs` - lines ~3580-3800 (the 239c derivation and caller) and ~3060-3480 (the emit pass, roles, entry push)
  - `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs` - lines ~150-200 (the `layer_plan()` fixture) and ~920-990 (the 239c independent-height test)
  - `crates/slicer-ir/src/slice_ir.rs` - the `SupportPlanEntry` definition only
- Files allowed to edit (at most 3):
  - `modules/core-modules/tree-support-planner/src/lib.rs`
  - `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/tree-support/src/lib.rs` (renderer; 239c-owned)
  - `crates/slicer-core/src/algos/support_geometry.rs` (read-only)
  - `OrcaSlicerDocumented/...` (delegate)
- Blast-radius discipline: not applicable (no struct field or schema constant changes). The
  `SupportPlanEntry` literal in the test file uses the live `SupportPlanEntry` field shape
  (body membership via `body_ids: Vec<String>`; no entry `id` field); if a new
  literal is added it must use `..` rest or an `// exhaustive:` waiver per
  `docs/21_data_defaults_and_fixtures.md`.
- Expected sub-agent dispatches:
  - Question: confirm the non-synchronized branch of `raft_and_intermediate_support_layers` brackets the sorted `extremes` and fills between consecutive ones at `step = dist / n_layers_extra`, last aligned to `extr2z`; scope: `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp`; return: `SUMMARY` (<= 200 words)
  - Question: which layers carry interface roles in the tree emit pass and how the roles are decided per node; scope: `modules/core-modules/tree-support-planner/src/lib.rs` (~3060-3480); return: `SUMMARY` (<= 200 words)
- Context cost: `M`
- Authoritative docs:
  - `docs/spec_packets/239c-support-layer-height-producer/design.md` - §Code Change Surface and §Open Questions (the `[FWD]` sentinel decision)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - `raft_and_intermediate_support_layers` (delegate; never load)
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` - `generate_support_layers` (delegate; never load)
- Verification:
  - `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- coarse_pitch_produces_free_floating_anchor_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- zero_pitch_sentinel_stays_object_grid --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- enabled_independent_height_produces_free_floating_anchor_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (finer direction unregressed)
  - `mkdir -p target && cargo test -p tree-support-planner --test tree_family_tdd -- adaptive_local_gap_stays_finer --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (finer-direction adaptive local gap regression: pitch 0.2 over covered surviving-row gaps 0.3 stays finer even when the first/base layer gap is 0.2; bracket-local coarse/finer selection by the binding predicate; NET-NEW test authored by this step, present and passing in the existing `tree_family_tdd` target)
  - `cargo xtask build-guests --check && echo FRESH` - FACT exit code
- Exit condition: AC-2 and AC-N3 pass; the 239c finer-direction test still passes; the
  finer-direction adaptive regression `adaptive_local_gap_stays_finer` (pitch 0.2 over
  covered gaps 0.3 stays finer) passes; the Q1/Q2
  decisions are implemented as bound (see `design.md` §Recorded Decisions D1/D2).

### Step 3: Traditional planner coarse derivation and `support_step` neutralization

- Task IDs: `TASK-525`
- Objective: Apply the same coarse derivation at the traditional planner's 239c caller
  (`modules/core-modules/traditional-support-planner/src/lib.rs` ~597-650) using the
  **traditional-family** rule of `raft_and_intermediate_support_layers`
  (`Support/SupportMaterial.cpp`): `n = ceil((dist - EPSILON) / pitch)` (**with** the
  EPSILON bias, unlike the tree rule), same step/plane/alignment shape, and the same
  `generate_support_layers` grouping/midpoint application (no group-height
  representation). Neutralize `support_step` per the **binding Q3 decision**: set
  `support_step = 1` exactly for bracket pairs satisfying the binding coarse predicate
  (configured nonzero pitch >= `local_support_gap`) — no global bypass of the gate at ~511.
  Bracket selection, body replacement (only non-interface rows strictly inside each
  bracket pair; genuine interface bracket entries survive), and the synthesized-candidates-only
  dedup follow the same Q1/Q2 rules as Step 2. Entries nondecreasing in
  `anchor_z` per object, distinct planes strictly increasing, identity key
  `(source global_layer_index, object_id, region_id, ordered body_ids, anchor_z)`
  deduplicated, true-nearest `anchor_layer_index`
  with lower-index tie break. The finer direction and the sentinel are unchanged.
- Precondition: Step 2 complete; the 239c traditional tests are green.
- Postcondition: the coarse direction emits free-floating `anchor_z` values with
  `support_step` neutralized; AC-3 passes; the finer direction is bit-identical.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support-planner/src/lib.rs` - lines ~560-720 (the 239c derivation and caller) and ~355-370, ~500-520 (the `support_step` decimation)
  - `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs` - lines ~320-360 (the 239c disabled test)
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support-planner/src/lib.rs`
  - `modules/core-modules/traditional-support-planner/tests/traditional_family_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/traditional-support/src/lib.rs` (renderer; 239c-owned)
  - `crates/slicer-core/src/algos/support_geometry.rs` (read-only)
  - `OrcaSlicerDocumented/...` (delegate)
- Blast-radius discipline: the `support_step` neutralization changes the traditional
  planner's decimation behaviour; the Step 3 dispatch inventories every test that hard-asserts
  the decimation (row counts or layer multiples) before editing. Never widen a tolerance to
  make a change pass.
- Expected sub-agent dispatches:
  - Question: every test in the workspace that hard-asserts the traditional planner's
    `support_step` decimation behaviour; scope: `crates/`, `modules/`; return: `LOCATIONS`
    (file + test fn, <= 15 entries)
  - Question: the traditional planner's interface-layer exemption at the decimation gate
    (`is_interface_layer`) and how it interacts with the 239c caller's `previous_supported_layer`
    bracketing; scope: `modules/core-modules/traditional-support-planner/src/lib.rs`
    (~500-660); return: `SUMMARY` (<= 150 words)
- Context cost: `M`
- Authoritative docs:
  - `docs/spec_packets/239c-support-layer-height-producer/design.md` - §Code Change Surface
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - `raft_and_intermediate_support_layers` (delegate; never load)
- Verification:
  - `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd -- coarse_pitch_produces_free_floating_anchor_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd -- disabled_independent_height_copies_object_layer_print_z_exactly --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail (disabled branch unregressed)
  - `cargo xtask build-guests --check && echo FRESH` - FACT exit code
- Exit condition: AC-3 passes; the 239c disabled test still passes; the Q3 decision
  (`support_step = 1` per coarse bracket pair, no global bypass) is implemented as bound
  (see `design.md` §Recorded Decisions D3).

### Step 4: Real-slice integration tests

- Task IDs: `TASK-526`
- Objective: Author the AC-1 check `coarse_support_pitch_emits_free_floating_extruding_rows`
  and the AC-N1 check `disabled_coarse_pitch_reproduces_baseline_z_sequence` as `pub fn`
  checks in `crates/slicer-runtime/tests/integration/support_family_closure.rs` with bare
  `#[test]` wrappers in `integration/main.rs` (the wrapper convention), plus the E-assertion
  helper (parse the `;TYPE:Support` block after an off-grid `;Z:` row, extract G1 `E` tokens,
  assert at least one `> 0`). AC-1 covers both families (the family names from the 239c
  `run_slice_for_family_with_extra` call sites). AC-N1 hard-codes the Step 1 baseline in the
  new `P239D_DISABLED_COARSE_PITCH_BASELINE_Z` const (following the 239c
  `DISABLED_INDEPENDENT_HEIGHT_BASELINE_Z` pattern).
- Precondition: Steps 2-3 complete; the Step 1 baseline sequence is available.
- Postcondition: AC-1 and AC-N1 pass; the E-assertion helper is in place.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs` - the helper block (~35-260) and the 239c tests (~280-380)
  - `crates/slicer-runtime/tests/integration/main.rs` - the wrapper declarations only
  - `crates/slicer-gcode/tests/gcode_relative_extrusion_tdd.rs` - `extract_e_values` only (~45-60)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/support_family_closure.rs`
  - `crates/slicer-runtime/tests/integration/main.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/**` (host runtime; 239a/239c-owned)
  - `OrcaSlicerDocumented/...` (delegate)
- Blast-radius discipline: not applicable. The baseline const follows the 239c pattern
  (a literal list of `";Z:<z>"` strings); the `SupportPlanEntry` literals in the planner
  tests (Steps 2-3) use `..` rest or waivers per `docs/21_data_defaults_and_fixtures.md`.
- Expected sub-agent dispatches:
  - Question: the exact family names and config-edit helper used by the 239c real-slice
    tests so AC-1 can cover both families; scope:
    `crates/slicer-runtime/tests/integration/support_family_closure.rs`; return: `FACT`
    (<= 5 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/spec_packets/239c-support-layer-height-producer/packet.spec.md` - the test-naming convention section
- OrcaSlicer refs: none (test authoring).
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- coarse_support_pitch_emits_free_floating_extruding_rows --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-runtime --test integration -- disabled_coarse_pitch_reproduces_baseline_z_sequence --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
  - `cargo xtask build-guests --check && echo FRESH` - FACT exit code
- Exit condition: AC-1 and AC-N1 pass; the wrapper convention is followed (bare names in
  `main.rs`); the E-assertion helper asserts `E > 0` on every off-grid support row; AC-1
  also checks the enabled run's distinct support `;Z:` sequence is a strict superset of the
  disabled baseline in original output order.

### Step 5: Measure-first coarse `height_delta`

- Task IDs: `TASK-527`
- Objective: Measure the three numbers for a minimal coarse off-grid case through
  `DefaultGCodeEmitter::emit_gcode` — applied height term, declared plane delta (the row's
  own Z minus the previous extrusion Z), resulting E — for a 0.3-pitch row, plus the same
  three for the immediately following object pass (observation O-1). Verdict:
  `MISSCALE_FIXED` iff the applied height term differs from the declared plane delta by more
  than `1e-6` absolute, else `CONSISTENT`. Record the verdict and the six numbers under
  `TASK-527` in `docs/07_implementation_status.md` (the TASK-519 pattern).
- Precondition: Steps 2-4 complete; the coarse stack emits off-grid rows.
- Postcondition: the verdict and the six numbers are recorded; no emitter edit is made
  (the verdict decides Step 6's branch only).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - only the two bounded ranges the dispatch returns
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` - the 239c verdict test only (~1883-1960)
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (the `TASK-527` measurement record)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (read-only; the verdict decides whether Step 6 may touch it)
  - `OrcaSlicerDocumented/...` (delegate)
- Blast-radius discipline: not applicable (measurement only).
- Expected sub-agent dispatches:
  - Question: the three measurement numbers for a minimal coarse off-grid case through
    `DefaultGCodeEmitter::emit_gcode` — applied height term, declared plane delta, resulting
    E — plus the same three for the immediately following object pass; scope:
    `crates/slicer-gcode/`; return: `FACT` (six numbers plus the verdict word).
    **Highest-risk dispatch.**
- Context cost: `S`
- Authoritative docs:
  - `docs/07_implementation_status.md` - the `TASK-519` row only (the verdict-record template)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `_extrude` (comparison target only; delegate; never load)
- Verification:
  - `rg -q 'TASK-527' docs/07_implementation_status.md` - FACT pass/fail
- Exit condition: the verdict and the six numbers are recorded under `TASK-527`; the
  verdict word is one of `CONSISTENT` / `MISSCALE_FIXED`; no emitter edit was made.

### Step 6: Verdict test

- Task IDs: `TASK-528`
- Objective: Author `coarse_pass_height_delta_matches_recorded_verdict` in
  `crates/slicer-gcode/tests/gcode_emit_tdd.rs`, mirroring the 239c verdict test (~1883):
  it asserts exactly the Step 5 recorded branch and names it in its own assertion message.
  On `CONSISTENT`: assert the current per-row formula equal within `1e-6` **on the recorded
  applied-height constants** (the height term actually applied, as recorded under
  `TASK-527`) and the recorded declared plane delta, and assert no emitter behaviour
  changed. On `MISSCALE_FIXED`: assert
  `e == distance * point.width * declared_plane_delta * point.flow_factor / filament_area`
  within `1e-6` — and in that branch only, Step 6 owns the emitter edit and its full
  test-fallout inventory in the same step.
- Precondition: Step 5 complete (the verdict recorded before the test is authored).
- Postcondition: AC-4 passes; the test names the recorded verdict in its assertion message.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs` - the 239c verdict test only (~1883-1960)
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/tests/gcode_emit_tdd.rs`
  - `crates/slicer-gcode/src/emit.rs` (only on the `MISSCALE_FIXED` branch, with the
    test-fallout inventory in the same step)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...` (delegate)
- Blast-radius discipline: on the `MISSCALE_FIXED` branch, sweep every test that hard-asserts
  an emitted E value or a `;HEIGHT:` value before editing (the 239c Step 6 pattern); never
  widen a tolerance to make a change pass.
- Expected sub-agent dispatches:
  - Question: every test in the workspace that hard-asserts an emitted E value or a
    `;HEIGHT:` value; scope: `crates/`, `modules/`; return: `LOCATIONS` (file + test fn,
    <= 25 entries); purpose: fix branch only.
- Context cost: `S`
- Authoritative docs:
  - `docs/07_implementation_status.md` - the `TASK-527` row only (the recorded verdict)
- OrcaSlicer refs: none (the verdict is measured, not borrowed).
- Verification:
  - `mkdir -p target && cargo test -p slicer-gcode --test gcode_emit_tdd -- coarse_pass_height_delta_matches_recorded_verdict --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0` - FACT pass/fail
- Exit condition: AC-4 passes; the assertion message names the recorded verdict; on
  `CONSISTENT` the emitter is unmodified (empty-diff guard).

### Step 7: Human-gate artifacts and gate document

- Task IDs: `TASK-529`
- Objective: Regenerate the packet artifacts immediately after `cargo xtask build-guests
  --check` returns exit `0`: `tmp/p239d-support-coarse-tree.gcode` and
  `tmp/p239d-support-coarse-normal.gcode` (the 0.3-pitch slices), `tmp/vd-p239d/` (the
  visual-debug bundle), and `tmp/239d-human-validation.md` (the gate document with the
  checklist, the `REFS-PRESENT`/`REFS-ABSENT-GATE-OPEN` record, and the recommended
  reference nozzle per `[FWD]` Q4).
- Precondition: Steps 1-6 complete; `cargo xtask build-guests --check` exits `0`.
- Postcondition: the artifacts exist; the gate document records `REFS-ABSENT-GATE-OPEN`
  (verified at authoring time) and the checklist.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/spec_packets/239c-support-layer-height-producer/packet.spec.md` - the Human Validation Gate section only
- Files allowed to edit (at most 3):
  - `tmp/239d-human-validation.md`
  - `tmp/` (the gcode artifacts and the visual-debug bundle)
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/visual_debug.rs` (239a-owned; read-only if unavoidable)
  - `OrcaSlicerDocumented/...` (delegate)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: the `pnp_cli visual-debug --request` invocation shape used by the 239c
    artifacts; scope: `tmp/` and `crates/pnp-cli/`; return: `FACT` (<= 5 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/19_visual_debug.md` - the visual-debug bundle guide (delegated summary)
- OrcaSlicer refs: none (the references are human-generated).
- Verification:
  - `test -f tmp/p239d-support-coarse-tree.gcode && test -f tmp/p239d-support-coarse-normal.gcode && test -d tmp/vd-p239d && echo ARTIFACTS-PRESENT` - FACT pass/fail
  - `test -f tmp/239d-human-validation.md && echo GATE-DOC-PRESENT` - FACT pass/fail
- Exit condition: the artifacts exist; the gate document records the checklist and the
  `REFS-ABSENT-GATE-OPEN` status; the packet may reach "all steps complete, sign-off
  pending".

### Step 8: Docs registration and closure

- Task IDs: `TASK-530`
- Objective: Register `TASK-523`..`TASK-530` in `docs/07_implementation_status.md` (through
  a worker dispatch, never a full backlog read), add queue row 4 to
  `docs/specs/support-independent-layer-z-split-plan.md`, add the new coarse-direction row
  to `docs/specs/support-parity-gap-register.md`, re-derive the ledger facts (task
  high-water, next free `DEV-###` via
  `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, next free `G-` row),
  then run the closure gates: `cargo check --workspace --all-targets`, `cargo clippy
  --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, every pipe-suffixed
  AC command, and the full suite through `cargo xtask test --summary --workspace
  --no-fail-fast` (the packet-close ceremony).
- Precondition: Steps 1-7 complete; the human gate is signed or explicitly pending.
- Postcondition: the docs are registered; the closure gates pass; the packet is ready for
  `status: implemented` (pending the human-gate sign-off).
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` - the support-family block only (delegated)
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md`
  - `docs/specs/support-independent-layer-z-split-plan.md`
  - `docs/specs/support-parity-gap-register.md`
- Files explicitly out of bounds:
  - `docs/DEVIATION_LOG.md` (read-only unless a deviation is genuinely needed; none is
    planned — the seam completion DEV-159..163 is inherited, not re-filed)
  - `OrcaSlicerDocumented/...` (delegate)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: the current task high-water, the next free `DEV-###`, and the next free `G-`
    row in `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, and
    `docs/specs/support-parity-gap-register.md`; scope: those three files; return: `FACT`
    (<= 5 lines)
- Context cost: `M`
- Authoritative docs:
  - `docs/07_implementation_status.md` - the support-family block only (delegated)
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'TASK-523' docs/07_implementation_status.md && rg -q 'TASK-530' docs/07_implementation_status.md && rg -q 'TASK-527' docs/07_implementation_status.md` - FACT pass/fail
  - `rg -q '239d-support-coarse-floating-planes' docs/specs/support-independent-layer-z-split-plan.md` - FACT pass/fail
  - `rg -q '239d-support-coarse-floating-planes' docs/specs/support-parity-gap-register.md` - FACT pass/fail
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
  - `cargo xtask check-literals` - FACT pass/fail
  - `cargo xtask test --summary --workspace --no-fail-fast` - FACT pass/fail (ceremony only)
- Exit condition: the docs are registered with re-derived ledger facts; the closure gates
  pass; the packet is ready for `status: implemented` pending the human-gate sign-off.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | measurement + baselines |
| Step 2 | M | tree derivation + tests |
| Step 3 | M | traditional derivation + `support_step` |
| Step 4 | M | real-slice tests + E-helper |
| Step 5 | S | measure-first verdict |
| Step 6 | S | verdict test |
| Step 7 | M | human-gate artifacts |
| Step 8 | M | registration + closure |

Aggregate `M`; no step is `L`.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none planned; 239c stays `implemented`).
- `packet.spec.md` is ready for `status: implemented` (pending the human-gate sign-off).

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (the human-gate `REFS-ABSENT-GATE-OPEN` status).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.

# Implementation Plan: 239-support-independent-layer-z

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs
  (`TASK-399`..`TASK-408`).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Every verification command satisfies invariant 16: explicit `--exact` names or an asserted
  non-zero matched count. Combined output tees to `target/test-output.log`; results are read
  from the file, never by re-running.
- No step edits more than 3 files. Guest freshness (`cargo xtask build-guests --check`)
  precedes every fixture-slice artifact run.

## Steps

### Step 1: Re-verify blockers live (read-only discovery)

- Task IDs: `TASK-399`
- Objective: confirm both G-02 blockers still hold on the working tree: (a)
  `is_same_z_entity`'s tolerance match excludes off-grid planes from both routes; (b) no
  production call site invokes `execute_per_layer_with_anchored_events` or its committed
  variant.
- Precondition: packet generated; branch `parity/support-planners-clean`.
- Postcondition: a recorded LOCATIONS inventory naming each consumer of `is_same_z_entity`
  and each `execute_per_layer*` caller, with one-line context each; if either blocker has
  moved, STOP and re-derive from the plan before proceeding.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - symbol-windowed ranges around
    `is_same_z_entity`, `append_same_z_entities`, and the committed-event exclusion filter
  - `crates/slicer-runtime/src/pipeline.rs` - the two per-layer call sites only
- Files allowed to edit (at most 3):
  - none (discovery step; findings recorded in the dispatch return)
- Files explicitly out of bounds:
  - everything in design.md §Out-of-Bounds Files; all test fixtures; `OrcaSlicerDocumented/`
- Expected sub-agent dispatches:
  - Question: "List every consumer of `is_same_z_entity` and every caller of any
    `execute_per_layer*` function"; scope: `crates/slicer-runtime/src/`; return:
    `LOCATIONS` ≤20 entries
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-parity-gap-register.md` - G-02 row only (ranged read)
- OrcaSlicer refs:
  - none this step
- Verification:
  - The dispatch return itself is the deliverable: it must show ≥2 `is_same_z_entity`
    consumers and ≥1 non-test pipeline call site of a non-anchored variant — FACT counts,
    else the step's exit condition fails.
- Exit condition: blocker inventory recorded verbatim in the step log; zero edits made.

### Step 2: Red tests for exact-Z routing totality

- Task IDs: `TASK-400`
- Objective: author the failing tests that define the target semantics: AC-2
  (`every_same_z_support_entity_routes_exactly_once`) and AC-N2
  (`offgrid_entity_never_merged_into_grid_layers`), plus AC-N1's preservation check wired to
  the existing `anchored_event_ordering` expectations.
- Precondition: Step 1 inventory recorded.
- Postcondition: new integration module mounted in
  `crates/slicer-runtime/tests/integration/main.rs`; AC-2/AC-N2 compile and FAIL for the
  right reason (off-grid entity dropped today); `anchored_event_ordering` still passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/anchored_event_ordering.rs` - full (small file;
    reuses its plan/event builders)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/offgrid_routing_tdd.rs` (new)
  - `crates/slicer-runtime/tests/integration/main.rs` (module mount + wrappers)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/**` (no implementation this step); planner/renderer modules
- Blast-radius discipline: not applicable (no struct/schema change).
- Expected sub-agent dispatches:
  - none
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §6 invariants 6/8/9/12/13
    (ranged read)
- OrcaSlicer refs:
  - none this step
- Verification:
  - `cargo test -p slicer-runtime --test integration -- every_same_z_support_entity_routes_exactly_once --exact 2>&1 | tee target/test-output.log && test "$(grep -cF 'every_same_z_support_entity_routes_exactly_once ... FAILED' target/test-output.log)" -eq 1`
  - `cargo test -p slicer-runtime --test integration -- offgrid_entity_never_merged_into_grid_layers --exact 2>&1 | tee target/test-output.log && test "$(grep -cF 'offgrid_entity_never_merged_into_grid_layers ... FAILED' target/test-output.log)" -eq 1`
  - `cargo test -p slicer-runtime --test integration -- anchored_event_ordering --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- Exit condition: each red-first command names ONE test with `--exact` and asserts exactly
  its own `... FAILED` line (libtest accepts a single filter; the tee masks cargo's nonzero
  exit so the grep guard is the verdict); substrate suite green. Falsifying exit: new tests
  pass pre-implementation (they assert nothing).

### Step 3: Total route-partition predicate in the executor

- Task IDs: `TASK-401`
- Objective: replace `is_same_z_entity`'s boolean filter role with a total partition helper:
  tolerance match ⇒ ordinary merge (unchanged), else ⇒ anchored route. Both consumers call
  the one helper.
- Precondition: Step 2 red tests in place.
- Postcondition: AC-N2 and AC-N1 green; off-grid entities now flow into the committed
  anchored stream when the executor entry point is called directly (AC-2 may remain red
  until Step 4 wires production).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` - windows around `is_same_z_entity`,
    `append_same_z_entities`, the committed-event filter
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/layer_executor.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/pipeline.rs`, `crates/slicer-gcode/**`, guest/module trees
- Blast-radius discipline: not applicable (private helper; no signature/struct change).
- Expected sub-agent dispatches:
  - Question: "Confirm no other crate references `is_same_z_entity`"; scope:
    `crates/`; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - consulted via the coord-system constraint; do not full-read
- OrcaSlicer refs:
  - none this step
- Verification:
  - `cargo xtask build-guests --check && echo FRESH` (exit 0 required before the commands below; exit 1 = rebuild guests first, exit 3 = `wasm-tools` infrastructure error — stop and report, never grep for `STALE:`)
  - `cargo test -p slicer-runtime --test integration -- offgrid_entity_never_merged_into_grid_layers --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo test -p slicer-runtime --test integration -- anchored_event_ordering --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo test -p slicer-runtime --test integration -- anchored_parallel_determinism --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo test -p slicer-runtime --test integration -- anchored_z_span_validation --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo check --workspace --all-targets`
- Exit condition: partition is total (every same-z-support entity takes exactly one route);
  substrate suites green. Falsifying exit: `anchored_event_ordering` regresses (invariant 6
  broken) or any anchored_* suite flips red.

### Step 4: Production enablement + support-only row synthesis

- Task IDs: `TASK-402`
- Objective: switch both `run_pipeline_core` per-layer call sites to
  `execute_per_layer_with_committed_anchored_events`, thread the anchored entity list, split
  committed events, and synthesize support-only print rows at their declared Z positions so
  finalization/postpass/G-code see them.
- Precondition: Step 3 landed; AC-N1/N2 green.
- Postcondition: AC-1, AC-3, AC-4 green through the real pipeline; support-free output
  unchanged (AC-N3 green); empty-collection stream byte-equivalent with the pre-change
  layer list.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/pipeline.rs` - the per-layer phase region only
  - `crates/slicer-runtime/src/blackboard.rs` - the `anchored_event_collections` slot region only
  - `crates/slicer-ir/src/slice_ir.rs` - `OrderedEventCollection`,
    `AnchoredGeometryContract` definitions only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/pipeline.rs`
  - `crates/slicer-runtime/tests/integration/offgrid_routing_tdd.rs` (extend with AC-1/3/4
    pipeline-level cases; counts as the step's second edit surface alongside main.rs mounts
    already owned from Step 2)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/**` until Step 5's verdict; planner/renderer modules; WIT trees
- Blast-radius discipline: mandatory check — if synthesis requires a new `LayerCollectionIR`
  or `PipelineOutput` field, STOP within this step and enumerate every struct-literal site
  compiling against the type via a LOCATIONS dispatch BEFORE adding it; add the field plus
  all literal-site updates in this step, never as a follow-up. Current design needs none.
- Expected sub-agent dispatches:
  - Question: "Run the AC-1/AC-3/AC-4 commands and report FACT pass/fail with failure
    snippets ≤20 lines"; scope: repo root; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §7 E4/E5/E8 (ranged read)
- OrcaSlicer refs:
  - none this step
- Verification:
  - `cargo xtask build-guests --check && echo FRESH` (exit 0 required immediately before the evidence commands below; exit 1 = rebuild guests first, exit 3 = `wasm-tools` infrastructure error — stop and report, never grep for `STALE:`)
  - `cargo test -p slicer-runtime --test integration -- offgrid_support_entity_emits_intermediate_print_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo test -p slicer-runtime --test integration -- offgrid_interleaving_identical_serial_and_parallel --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo test -p slicer-runtime --test integration -- zspanning_support_entity_emits_atomic_single_block --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo test -p slicer-runtime --test integration -- support_disabled_pipeline_emits_nothing --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Exit condition: all four pipeline-level tests green; lint gate green. Falsifying exit: any
  grid-only slice's layer count/order changes vs pre-change baseline (empty-collection
  equivalence broken).

### Step 5: Measure-first `height_delta` verdict (measurement dispatch)

- Task IDs: `TASK-403`
- Objective: measure whether the emitter mis-scales E for an off-grid pass, per design.md
  §Approach 3: construct the minimal case through the real emitter, record applied height
  term vs declared plane delta, and emit the verdict string.
- Precondition: Step 4 landed (off-grid passes reach emission).
- Postcondition: verdict `MISSCALE_FIXED` or `CONSISTENT` recorded under TASK-403 in
  `docs/07_implementation_status.md` WITH the three measured numbers (applied height term,
  declared plane delta, resulting E). No source edit in this step.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - the height_delta derivation region and the
    volumetric-E product region only
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (verdict record only)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/emit.rs` (read-only THIS step — measuring, not fixing)
- Blast-radius discipline: not applicable (no code change).
- Expected sub-agent dispatches:
  - Question: the measurement protocol verbatim; scope: `crates/slicer-gcode` +
    throwaway harness; return: `FACT` = three numbers + verdict string; purpose: gate Step 6.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §7 E1/E7 (ranged read)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `_extrude` flow product; delegate;
    never load
- Verification:
  - `rg -q 'TASK-403' docs/07_implementation_status.md && rg -q 'MISSCALE_FIXED|CONSISTENT' docs/07_implementation_status.md && echo VERDICT-RECORDED`
- Exit condition: verdict + numbers recorded; falsifying exit: any `emit.rs` edit made in
  this step, or a verdict recorded without measured numbers.

### Step 6: Conditional emitter fix OR assert-only lock (branch on TASK-403)

- Task IDs: `TASK-404`
- Objective: act on the recorded verdict. `MISSCALE_FIXED`: carry per-entity plane-Z context
  so an off-grid pass's E uses its declared plane delta; keep grid passes bit-identical.
  `CONSISTENT`: no behavior change; author the AC-5 test asserting current correct behavior
  within `1e-6` on the Step 5 constants.
- Precondition: Step 5 verdict recorded with numbers.
- Postcondition: AC-5 green naming the recorded branch; grid-only slices produce identical
  E values on regression fixtures (self-captured baseline rules, E3 — golden reblessing
  forbidden without classified drift).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-gcode/src/emit.rs` - height_delta + E product regions
- Files allowed to edit (at most 3):
  - `crates/slicer-gcode/src/emit.rs` (fix branch only)
  - the `slicer-gcode` unit-test location hosting `height_delta_verdict_matches_measured_behavior`
- Files explicitly out of bounds:
  - serializer/feedrate surfaces unrelated to the E product; runtime crates
- Blast-radius discipline: mandatory on the fix branch — enumerate via LOCATIONS dispatch
  every test that hard-asserts emitted E values against `emit.rs` behavior before editing;
  update only assertions invalidated by the corrected off-grid term, never widen tolerances.
- Expected sub-agent dispatches:
  - Question: "List tests hard-asserting emitted E/extrusion values fed by emit.rs";
    scope: `crates/slicer-gcode`, `crates/pnp-cli/tests`; return: `LOCATIONS` ≤20
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §7 E3 (ranged read)
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `_extrude` (fix branch shape check);
    delegate; never load
- Verification:
  - `cargo test -p slicer-gcode --lib -- height_delta_verdict_matches_measured_behavior --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
  - `cargo test -p slicer-runtime --test integration -- offgrid_support_entity_emits_intermediate_print_z --exact 2>&1 | tee target/test-output.log && test "$(grep -c '^test .* ok$' target/test-output.log)" -gt 0`
- Exit condition: AC-5 green on the recorded branch; AC-1 unaffected. Falsifying exit: the
  fix changes grid-pass E anywhere (regression), or the CONSISTENT branch edited `emit.rs`.

### Step 7: Freshness gate + human-gate artifact production

- Task IDs: `TASK-405`
- Objective: run `cargo xtask build-guests --check` (exit 0 required; rebuild on 1; stop and
  report infra on 3), then produce `tmp/p239-support-indep-tree.gcode` /
  `tmp/p239-support-indep-normal.gcode` and the `tmp/vd-p239/` visual-debug bundle per the
  packet.spec.md Human Validation Gate.
- Precondition: Steps 1–6 complete; matched configs present in `tmp/` (regenerate if absent).
- Postcondition: artifacts exist; freshness token FRESH captured immediately before slicing.
- Files allowed to read, with ranges when over 300 lines:
  - none beyond command output
- Files allowed to edit (at most 3):
  - none in-tree (artifacts land under gitignored `tmp/`)
- Files explicitly out of bounds:
  - all source; `OrcaSlicerDocumented/`
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: execute the slice + visual-debug commands; scope: repo root; return: `FACT`
    artifact paths + sizes-exist booleans; purpose: evidence production without loading
    outputs into context.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` - §8/§9 (ranged read)
- OrcaSlicer refs:
  - none this step (references are human-generated; existence-checked only)
- Verification:
  - `cargo xtask build-guests --check && echo FRESH`
  - `test -f tmp/p239-support-indep-tree.gcode && test -f tmp/p239-support-indep-normal.gcode && test -d tmp/vd-p239 && echo ARTIFACTS-PRESENT`
- Exit condition: FRESH then ARTIFACTS-PRESENT. Falsifying exit: freshness exit 3 (infra) or
  stale-after-rebuild loop — stop and report, do not attribute failures blind (E4/T4).

### Step 8: Reference existence gate + matched-height delta record

- Task IDs: `TASK-406`
- Objective: verify the human-generated §9 references exist
  (`tmp/p239-orca-ref-tree-independent.gcode`, `tmp/p239-orca-ref-normal-independent.gcode`);
  if absent, the human gate stays unsigned and this step records exactly that. If present:
  record measured block-count and distinct-print-Z deltas (PnP vs reference) for both
  families into the gate checklist notes — numbers only, no requoted legacy figures (T11).
- Precondition: Step 7 artifacts exist.
- Postcondition: either (a) REFS-PRESENT + measured delta table recorded, or (b) documented
  absence blocking ONLY sign-off, with packet otherwise complete.
- Files allowed to read, with ranges when over 300 lines:
  - reference G-code files - grep/count extraction only, never full-load
- Files allowed to edit (at most 3):
  - `docs/spec_packets/239-support-independent-layer-z/packet.spec.md` (checklist notes only)
- Files explicitly out of bounds:
  - plan file; other packets' directories; DEVIATION_LOG
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: extract `;TYPE:Support`/`;TYPE:Support interface` counts and distinct `;Z:`
    rows from the four G-code files; scope: `tmp/p239-*`; return: `FACT` table
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-parity-gap-register.md` - G-02 row (ranged re-read for closure wording)
- OrcaSlicer refs:
  - none directly (the references ARE the Orca evidence; generation was human-owned)
- Verification:
  - `test -f tmp/p239-orca-ref-tree-independent.gcode && test -f tmp/p239-orca-ref-normal-independent.gcode && echo REFS-PRESENT || echo REFS-ABSENT-GATE-OPEN`
- Exit condition: explicit REFS-PRESENT or REFS-ABSENT-GATE-OPEN outcome recorded; no
  fabricated numbers either way.

### Step 9: Full-suite reconciliation on touched surfaces

- Task IDs: `TASK-407`
- Objective: prove no collateral regression: the whole `slicer-runtime` integration binary,
  the family suites, `slicer-gcode`, and the workspace type/lint gates.
- Precondition: Steps 1–8 complete.
- Postcondition: all targeted suites green; gates green; any red attributed AFTER a fresh
  `--check`.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - summary/failure regions only
- Files allowed to edit (at most 3):
  - none planned; a fix landing here loops through the owning step's file set
- Files explicitly out of bounds:
  - planner/renderer modules (238b/c territory)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: run the commands below; return FACT pass/fail per line + failing-test names;
    scope: repo root
- Context cost: `M`
- Authoritative docs:
  - `.agents/doc-index.md` - only if an unfamiliar failing-suite owner must be located
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo xtask build-guests --check && echo FRESH` (exit 0 required immediately before the test commands below; exit 1 = rebuild guests first, exit 3 = `wasm-tools` infrastructure error — stop and report, never grep for `STALE:`)
  - `cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log && test "$(grep -c '^test result: ok' target/test-output.log)" -gt 0 && ! grep -q 'FAILED' target/test-output.log`
  - `cargo test -p slicer-gcode 2>&1 | tee target/test-output.log && test "$(grep -c '^test result: ok' target/test-output.log)" -gt 0 && ! grep -q 'FAILED' target/test-output.log`
  - `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo xtask check-literals`
- Exit condition: all green with non-zero ok-counts (binary-count sanity vs prior runs — a
  drop means a silently skipped suite, E6 discipline). Falsifying exit: any FAILED line after
  a fresh `--check`.

### Step 10: Packet-owned closure registration

- Task IDs: `TASK-408`
- Objective: register `TASK-399..TASK-408` in `docs/07_implementation_status.md` with
  outcomes; flip gap-register G-02 destination note to implemented-pending-human-gate state;
  leave `status: draft` until the Human Validation Gate sign-off lands.
- Precondition: Step 9 green (or documented gate-open absence from Step 8).
- Postcondition: backlog rows registered; register row updated; packet remains `draft` pending
  sign-off.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` - tail region around the newest task IDs only
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md`
  - `docs/specs/support-parity-gap-register.md` (G-02 status column only)
- Files explicitly out of bounds:
  - DEVIATION_LOG; other packets; the plan file
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: perform the registration edit; scope: the two doc files; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/07_implementation_status.md` - format conventions from adjacent rows
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'TASK-399' docs/07_implementation_status.md && rg -q 'TASK-408' docs/07_implementation_status.md && rg -q 'G-02' docs/specs/support-parity-gap-register.md && echo CLOSURE-RECORDED`
- Exit condition: CLOSURE-RECORDED; human gate remains the sole remaining blocker to
  `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | discovery, delegated |
| Step 2 | S | two red tests |
| Step 3 | M | executor predicate swap |
| Step 4 | M | largest: pipeline enablement + synthesis |
| Step 5 | M | measurement dispatch |
| Step 6 | M | conditional fix / lock |
| Step 7 | S | artifact production |
| Step 8 | S | reference gate + deltas |
| Step 9 | M | reconciliation runs |
| Step 10 | S | registration |

Aggregate: M. No step is L. Split trigger: if Step 4's blast-radius check finds a needed
struct field touching >10 literal sites, split Step 4 into transport-field and synthesis
steps before proceeding (report the split; task IDs TASK-402a/b are NOT available — reuse
Step numbering inside the same ID).

## Packet Completion Gate

- All steps and exits complete (or Step 8's documented gate-open absence recorded).
- Every pipe-suffixed AC command returns PASS with non-zero matched counts.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Gap-register G-02 reflects implementation state; human gate checklist carries measured deltas.
- `packet.spec.md` is ready for `status: implemented` ONLY after gate sign-off (date + verdict).

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (expected residue: reference-vs-PnP placement deltas
  recorded as measurements, waived or accepted in writing at the gate).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where applicable so the test, bench, and example targets compile.

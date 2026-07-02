# Implementation Plan: 134_rectilinear-raw-emit

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Survey + RED — author the 8-test suite

- Task IDs:
  - `TASK-259`
- Objective: survey the existing module tests (which pin the old wrong geometry — enumerate,
  with the bug each encodes); author the 8 AC tests
  (`square_10mm_density_20_emits_n_raw_segments`,
  `polygon_with_hole_segments_split_around_hole`,
  `two_disjoint_expolygons_independent_scan_conversion`,
  `angle_45_rotated_output_matches_unrotated_after_inverse`,
  `solid_spacing_adjusted_for_solid_role`, `bridge_angle_overrides_layer_rotation`,
  `pattern_shift_interleaves_layers`, `half_open_vertex_test_no_double_count`); confirm RED.
- Precondition: packet 133 closed; clean tree.
- Postcondition: suite compiles; new tests RED against the stub; stale-geometry test list
  recorded in the test file header comment.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — full (361 lines, one read)
  - `modules/core-modules/rectilinear-infill/tests/` — existing files
- Files allowed to edit (≤ 3):
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` (new)
- Files explicitly out-of-bounds for this step: production source (RED first).
- Expected sub-agent dispatches:
  - "Run `cargo test -p rectilinear-infill 2>&1 | tee target/test-output.log | grep -E
    '^test |^test result'`; FACT per-test" — RED confirmation
- Context cost: `M`
- Authoritative docs: spec §Phase 2 (AC semantics).
- OrcaSlicer refs: none this step.
- Verification:
  - the dispatch above — FACT (new tests RED)
- Exit condition: 8 tests RED; stale-test inventory recorded.

### Step 2: GREEN — infill_direction + per-ExPolygon scan conversion

- Task IDs:
  - `TASK-259`
- Objective: port `infill_direction` (FillBase.cpp:352-391: bridge > per-layer > base, +π/2,
  bbox-center ref) and the single-level per-ExPolygon scan conversion
  (FillRectilinear.cpp:842-1154 discipline: integer y-intersections, half-open vertex test,
  sort + pair); delete `fill_expolygon_multi` + `collect_edges`; attribution header; AC-1,
  AC-2, AC-3, AC-4, AC-6, AC-N1 green.
- Precondition: Step 1 RED state.
- Postcondition: six of eight tests green; four-role structure untouched (diff review).
- Files allowed to read (with line-range hints when > 300 lines):
  - own module only
- Files allowed to edit (≤ 3):
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs` (fixture
    tweaks only)
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - the sectioned 842-1154 SUMMARY + SNIPPETS series (design §Expected Sub-Agent Dispatches)
  - "SNIPPETS FillBase.cpp:352-391; ≤30 lines"
  - "FACT: where does the current bridge emission read its angle from (module source)?"
  - "Run `cargo test -p rectilinear-infill …`; FACT + counts"
- Context cost: `M`
- Authoritative docs: `docs/08_coordinate_system.md` (delegate; ÷100 + rotation rounding).
- OrcaSlicer refs: FillRectilinear.cpp:842-1154, FillBase.cpp:352-391 — delegate.
- Verification:
  - `cargo test -p rectilinear-infill 2>&1 | tee target/test-output.log | grep "^test result"` — FACT
- Exit condition: AC-1/2/3/4/6/N1 green.

### Step 3: GREEN — adjust_solid_spacing + pattern_shift

- Task IDs:
  - `TASK-259`
- Objective: port `adjust_solid_spacing` (FillBase.cpp:326-340) into the solid-role path;
  apply `pattern_shift` (FillRectilinear.cpp:3023-3024 semantics) to the scan-line origin x;
  AC-5, AC-7 green.
- Precondition: Step 2 exit condition.
- Postcondition: all 8 tests green.
- Files allowed to read: own module.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
  - `modules/core-modules/rectilinear-infill/tests/rectilinear_raw_emit_tdd.rs`
- Files explicitly out-of-bounds for this step: `OrcaSlicerDocumented/**` directly.
- Expected sub-agent dispatches:
  - "SNIPPETS FillBase.cpp:326-340; ≤20 lines" + the pattern_shift FACT
  - "Run `cargo test -p rectilinear-infill …`; FACT + counts"
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs: FillBase.cpp:326-340, FillRectilinear.cpp:3023-3024 — delegate.
- Verification:
  - AC-5, AC-7 pipe commands — FACT each
- Exit condition: full new suite green.

### Step 4: Stale-test reconciliation + gates

- Task IDs:
  - `TASK-259`
- Objective: rewrite the Step-1-enumerated stale-geometry tests (each rewrite comments which
  bug the old expectation encoded); rebuild guests; run module + workspace gates; append any
  newly-affected goldens to the 131 carve list (recorded deviation).
- Precondition: Step 3 exit condition.
- Postcondition: full module suite green; guests fresh; gates green; carve delta recorded.
- Files allowed to read: own module tests; carve list.
- Files allowed to edit (≤ 3):
  - the existing module test file(s)
  - `.ralph/specs/131_per-region-config-delivery/carve-list.md` (append-only, if needed)
- Files explicitly out-of-bounds for this step: everything else.
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; FACT; rebuild if STALE"
  - "Run `cargo test -p rectilinear-infill …`; FACT + counts"
  - "Run `cargo check --workspace --all-targets` + `cargo clippy -p rectilinear-infill
    --all-targets -- -D warnings`; FACT each"
  - "Run the non-carved e2e/executor golden subset; FACT; list newly red" — carve delta
- Context cost: `M`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification:
  - the §Verification gates from `packet.spec.md` — FACT each
- Exit condition: all packet ACs green; carve delta recorded.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | survey + 8 tests |
| Step 2 | M | direction + scan-conversion port |
| Step 3 | S | solid spacing + pattern shift |
| Step 4 | M | reconciliation + gates + carve delta |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-259 (via worker dispatch — never edited
  by loading the full backlog into the implementer's context).
- Reopened or superseded packet status transitions reconciled (none expected).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a
  packet-authoring lesson for future spec-packet-generator runs.

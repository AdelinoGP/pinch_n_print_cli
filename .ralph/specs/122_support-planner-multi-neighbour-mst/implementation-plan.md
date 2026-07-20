# Implementation Plan: support-planner-multi-neighbour-mst

## Execution Rules

- One atomic step at a time. Maps to `TASK-287` (renumbered from source-plan `TASK-263`).
- TDD: AC-2, AC-3, AC-N1, AC-N2 unit tests authored RED before implementation.
- Wedge invariant (AC-4) added before integration; it will fail under single-neighbour algorithm; turns GREEN when Step 3 lands.
- Honors context-discipline preamble.

## Steps

### Step 1: Confirm Orca formula + locate propagation block

- Task IDs: `TASK-287`
- Files allowed to read: `docs/specs/support-modules-orca-port.md` §C4; planner propagation block (range-read).
- Sub-agent dispatches:
  - "Summarize OrcaSlicer `TreeSupport::drop_nodes` aggregation formula; return SUMMARY ≤ 200 words."
  - "Locate the `nearest_neighbour` Vec and the `for (i, node) in active_nodes.iter().enumerate()` propagation loop in support-planner/src/lib.rs; return LOCATIONS file:line + 1-line context."
- Files allowed to edit: none.
- Context cost: `S`
- Verification: implementer can recite the formula and points at the block.
- Exit condition: discovery captured.

### Step 2: Author AC-2 / AC-3 / AC-N1 / AC-N2 as RED

- Files allowed to edit (≤ 3): `modules/core-modules/support-planner/tests/multi_neighbour_mst_tdd.rs` (new).
- Sub-agent dispatches:
  - "Run `cargo test -p support-planner --test multi_neighbour_mst_tdd`; return FACT (expected: AC-2 fails; AC-3 fails; AC-N1 may pass coincidentally; AC-N2 fails with 'function not defined' or similar)."
- Context cost: `S`
- Verification: RED state for AC-2, AC-3, AC-N2.
- Exit condition: RED.

### Step 3: Implement `aggregate_neighbour_targets` + replace single-neighbour with multi-neighbour aggregation

- Files allowed to edit (≤ 3): `modules/core-modules/support-planner/src/lib.rs`.
- Sub-agent dispatches:
  - "Run `cargo build -p support-planner`; return FACT."
  - "Run `cargo test -p support-planner --test multi_neighbour_mst_tdd`; return FACT pass/fail."
  - "Run `cargo test -p support-planner` (existing tests); return FACT."
- Context cost: `M`
- Verification: AC-1 grep PASS; AC-2, AC-3, AC-N1, AC-N2 PASS; existing tests PASS.
- Exit condition: algorithm change live.

### Step 4: Add wedge symmetry invariant (AC-4) + regenerate goldens + extend docs/specs invariant list

- Files allowed to edit (3, at the ceiling — the two goldens are mechanically rewritten by the env-var-driven test at `support_golden_regression_wedge_tdd.rs:65` and arrive paired):
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
  - `resources/golden/support_regression_wedge_branch_count.txt`
  - `resources/golden/support_regression_wedge_endpoints.txt`
  - `docs/specs/support-modules-orca-port.md` (one line)
- Sub-agent dispatches:
  - "Run `SUPPORT_WEDGE_REGEN_GOLDEN=1 cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd -- current_wedge_output_stays_within_self_capture_tolerance`; return FACT (regen happened)."
  - "Run `cargo test -p slicer-runtime --test support_invariants_wedge_tdd`; return FACT per-test (the new test passes; all 8 existing pass)."
  - "Run `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd`; return FACT pass/fail."
  - "Run `rg -q 'merge_geometry_symmetric_for_n_branches' docs/specs/support-modules-orca-port.md`; return FACT pass/fail."
  - "Run `cargo xtask build-guests --check`; return FACT."
- Context cost: `M`
- Verification: AC-4, AC-5, AC-6, AC-7 PASS; doc updated.
- Exit condition: harness gates green; doc updated.

### Step 5: Final verification + close

- Sub-agent dispatches:
  - "Run all AC commands sequentially; return FACT (PASS / FAIL list)."
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; return FACT."
- Context cost: `S`
- Verification: all ACs PASS; clippy clean.
- Exit condition: closure summary; `packet.spec.md` ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Cost |
| --- | --- |
| 1 | S |
| 2 | S |
| 3 | M |
| 4 | M |
| 5 | S |

Aggregate: `M`. No step is L.

## Packet Completion Gate

- All ACs PASS; `cargo xtask build-guests --check` clean; `docs/07` marks `TASK-287` `[x]`; docs/specs invariant list extended.

## Acceptance Ceremony

- Re-dispatch every AC command; confirm gate commands green; mark `TASK-287` `[x]`; transition.

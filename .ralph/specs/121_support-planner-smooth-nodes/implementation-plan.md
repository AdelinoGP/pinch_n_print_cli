# Implementation Plan: support-planner-smooth-nodes

## Execution Rules

- One atomic step at a time. Maps to `TASK-262`.
- TDD: AC-2 / AC-3 / AC-N1 unit tests authored RED before smoothing implementation.
- The wedge curvature invariant (AC-5) is added first (RED on current planner output), then turned GREEN by the integration in Step 4.
- Honors context-discipline preamble.

## Steps

### Step 1: Confirm Orca formula + locate planner emission tail

- Task IDs: `TASK-262`
- Objective: confirm 100-iter Laplacian; find the integration point in `plan_for_object`.
- Files allowed to read: `docs/specs/support-modules-orca-port.md` §C3 directly; planner `plan_for_object` tail (range-read).
- Files allowed to edit: none.
- Sub-agent dispatches:
  - "Summarize OrcaSlicer `TreeSupport::smooth_nodes`; return SUMMARY ≤ 200 words."
  - "Locate the emission loop in `support-planner::plan_for_object` (where `SupportPlanEntry.branch_segments.push(...)` happens); return LOCATIONS file:line + 1-line context."
- Context cost: `S`
- Authoritative docs: `docs/specs/support-modules-orca-port.md` §C3
- OrcaSlicer refs: delegate per Orca obligations.
- Verification: implementer knows the formula and the integration point.
- Exit condition: discovery captured.

### Step 2: Author AC-2 / AC-3 / AC-N1 as RED in `smooth_nodes_tdd.rs`

- Task IDs: `TASK-262`
- Files allowed to read: planner internal type defs (range).
- Files allowed to edit (≤ 3): `modules/core-modules/support-planner/tests/smooth_nodes_tdd.rs` (new).
- Files out-of-bounds: planner `lib.rs` (Step 3 owns).
- Sub-agent dispatches:
  - "Run `cargo test -p support-planner --test smooth_nodes_tdd`; return FACT (expected: AC-2 and AC-3 fail; AC-N1 may pass coincidentally)."
- Context cost: `S`
- Verification: tests compile; RED state confirmed for at least AC-2 and AC-3.
- Exit condition: RED.

### Step 3: Implement `smooth_chains` + helpers; pass AC-2 / AC-3 / AC-N1

- Task IDs: `TASK-262`
- Files allowed to read: planner internal type defs.
- Files allowed to edit (≤ 3): `modules/core-modules/support-planner/src/lib.rs`.
- Files out-of-bounds: wedge harness file (Step 4 owns); goldens (Step 5).
- Sub-agent dispatches:
  - "Run `cargo test -p support-planner --test smooth_nodes_tdd`; return FACT pass/fail; SNIPPETS ≤ 20 lines on failure."
  - "Run `cargo build -p support-planner`; return FACT pass/fail."
- Context cost: `M`
- Verification: AC-2, AC-3, AC-N1 PASS; AC-1 grep PASS.
- Exit condition: function exists and is tested in isolation.

### Step 4: Integrate `smooth_chains` into `plan_for_object`; add wedge curvature invariant (AC-5); regenerate goldens

- Task IDs: `TASK-262`
- Files allowed to read: `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` (current state).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/support-planner/src/lib.rs` (integration call only — implementation already in Step 3)
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` (add AC-5 test)
  - `resources/golden/support_regression_wedge_branch_count.txt` (regenerated)
  - `resources/golden/support_regression_wedge_endpoints.txt` (regenerated)
- Files out-of-bounds: other test files.
- Sub-agent dispatches:
  - "Run the xtask golden-regen for support; return FACT (file sizes + line counts)."
  - "Run `cargo test -p slicer-runtime --test support_invariants_wedge_tdd`; return FACT (per-test pass/fail)."
  - "Run `cargo test -p slicer-runtime --test support_golden_regression_wedge_tdd`; return FACT pass/fail."
  - "Run `cargo xtask build-guests --check`; return FACT."
- Context cost: `M`
- Verification: AC-4, AC-5, AC-6, AC-7 PASS; existing wedge invariants 1-5 still PASS.
- Exit condition: smoothing live; harness gates green.

### Step 5: Final verification + close

- Files allowed to read: none beyond prior.
- Files allowed to edit: none.
- Sub-agent dispatches:
  - "Run all packet AC commands sequentially; return FACT (PASS / FAIL list)."
  - "Run `cargo clippy -p support-planner -p slicer-runtime --all-targets -- -D warnings`; return FACT."
- Context cost: `S`
- Verification: all ACs PASS; clippy clean.
- Exit condition: closure summary; `packet.spec.md` ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Cost | Notes |
| --- | --- | --- |
| 1 | S | Discovery |
| 2 | S | RED tests |
| 3 | M | Implementation |
| 4 | M | Integration + goldens |
| 5 | S | Verification |

Aggregate: `M`.

## Packet Completion Gate

- All steps complete; all ACs PASS; `cargo xtask build-guests --check` clean; `docs/07` marks `TASK-262` `[x]`.

## Acceptance Ceremony

- Re-dispatch every AC command. Confirm gate commands green. Mark `TASK-262` `[x]`; transition to `status: implemented`.

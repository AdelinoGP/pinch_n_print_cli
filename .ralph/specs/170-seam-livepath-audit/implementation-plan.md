# Implementation Plan: 170-seam-livepath-audit

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Author the sibling-wall regression fixtures

- Task IDs: `TASK-120c`
- Objective: Create `modules/core-modules/seam-placer/tests/seam_sibling_walls_tdd.rs` with four tests — `siblings_survive_rotation` (AC-1), `multi_region_wall_counts_preserved` (AC-2), `aligned_snap_preserves_siblings` (AC-3), `tolerance_miss_emits_all_walls_pristine` (AC-N1) — plus a concentric-square multi-loop region helper (closed loops with explicit closing repeat; distinct point sets per loop; parallel `feature_flags` / `width_profile.widths` arrays).
- Precondition: packet `168-seam-aligned-modes` is `implemented` (AC-3 needs `seam_mode = "aligned"` and its snap semantics).
- Postcondition: the file compiles and all four tests run to a verdict; each test's identity-comparison covers `path.points`, `feature_flags`, `width_profile.widths`, and `path.is_closed()`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-placer/tests/seam_placer_dispatch_tdd.rs`
  - `modules/core-modules/seam-placer/src/lib.rs`
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-placer/tests/seam_sibling_walls_tdd.rs` (new)
- Files explicitly out of bounds:
  - `modules/core-modules/seam-placer/src/lib.rs` (read-only in this step), host crates, `modules/core-modules/seam-planner-default/**`
- Expected sub-agent dispatches:
  - Question: output-inspection API used by `seam_placer_dispatch_tdd.rs` to read emitted loops/regions back from `PerimeterOutputBuilder` (`begin_region` `builders.rs:266`, `push_reordered_wall_loop` `builders.rs:337`); scope: that test file + `crates/slicer-sdk/src/builders.rs`; return: `FACT` (only if not evident from the test file)
- Context cost: `S`
- Authoritative docs:
  - none (builder semantics resolved via the `crates/slicer-sdk/src/builders.rs` FACT dispatch above)
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p seam-placer --test seam_sibling_walls_tdd 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail with per-test names on failure
- Exit condition: all four tests have run to a recorded verdict (pass or fail list captured from `target/test-output.log`); RED-capability of each assertion confirmed by construction review (assertions compare full vectors, not lengths only).

### Step 2 (conditional): Fix `run_wall_postprocess` if falsified

- Task IDs: `TASK-120c`
- Objective: If any Step 1 test fails, apply the minimal fix in `run_wall_postprocess` (emission loop `lib.rs:260-275` or its `seam_target` interplay) so all four tests pass without changing seam-selection behavior pinned by the existing suites. If all Step 1 tests passed, skip this step explicitly and record "invariant verified, no fix needed" in the packet report.
- Precondition: Step 1 verdicts recorded.
- Postcondition: `cargo test -p seam-placer` fully green; guest rebuilt if `src/lib.rs` changed.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-placer/src/lib.rs`
  - `target/test-output.log` (failure detail)
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-placer/src/lib.rs`
- Files explicitly out of bounds:
  - all other crates and modules; test file assertions (never weaken a Step 1 assertion to pass)
- Expected sub-agent dispatches: none
- Context cost: `S`
- Authoritative docs:
  - none
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p seam-placer 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail
  - `cargo xtask build-guests --check` (rebuild if STALE) - FACT clean
- Exit condition: whole seam-placer suite green AND guest freshness clean; or the step is recorded as skipped with all Step 1 tests green.

### Step 3: TASK-120c disposition in docs/07

- Task IDs: `TASK-120c`
- Objective: Reconcile the existing reopened `- [~] TASK-120c` row at `docs/07_implementation_status.md:92` per the audit outcome — flip to `- [x]` (closed: name the invariant, the new test file, and packet `170-seam-livepath-audit`) or `- [ ]` (re-scoped: name the exact residual defect found), replacing the stale reopened-gap text (candidate preference is already fixed per `lib.rs:242-252`) — via a worker dispatch with the exact anchor and replacement row text.
- Precondition: Steps 1-2 resolved with a recorded outcome.
- Postcondition: AC-4 grep passes.
- Files allowed to read, with ranges when over 300 lines:
  - none directly (docs/07 handled by dispatch)
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (via dispatch only)
- Files explicitly out of bounds:
  - full read of `docs/07_implementation_status.md`
- Expected sub-agent dispatches:
  - Question: replace the reopened TASK-120c row (anchor: `- [~] TASK-120c Restore seam placement on real wall-loop seam candidates`, line 92) with the supplied reconciled row; scope: `docs/07_implementation_status.md`; return: `FACT` (grep confirmation of the updated row)
- Context cost: `S`
- Authoritative docs:
  - `docs/07_implementation_status.md` - dispatch only
- OrcaSlicer refs:
  - none
- Verification:
  - `grep -E '^- \[[x ]\] TASK-120c ' docs/07_implementation_status.md | grep -q '170-seam-livepath-audit' && ! grep -qE '^- \[~\] TASK-120c ' docs/07_implementation_status.md && echo PASS` - FACT PASS (fails until the `[~]` row is reconciled)
- Exit condition: AC-4 grep PASS and the row text states the audit finding (not a placeholder).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | fixture authoring, one optional FACT dispatch |
| Step 2 | S | conditional; skip-record if green |
| Step 3 | S | docs/07 dispatch |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete (Step 2 skip is a valid completion when recorded).
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (TASK-120c disposition is this packet's deliverable).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (residual tolerance-miss coordinate gap if TASK-120c was re-scoped rather than closed).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.

# Implementation Plan: core-strengthen

This packet covers five named test functions across four existing weak-oracle groups and a separate real §7 ledger-row edit.

## Execution Rules

- Work one atomic step at a time; map every step to `core/DUP-CORE-strengthen (excluding wider-bead)`.
- Preserve the existing test names, target registration, fixtures, and meaningful assertions before adding the independent expectation.
- Every Cargo test invocation uses `--features host-algos`, an exact filter, an anchored one-passed/zero-failed result check, and combined output through `target/test-output.log`; cargo runs are delegated.
- No step edits production implementation, creates a fixture/test target, or changes the approved queue. The final step edits only the §7 `core` progress row as a future implementation action; this packet author does not edit the plan.

## Steps

### Step 1: Strengthen bridge angle and vertical flow witnesses

- Task IDs: `core/DUP-CORE-strengthen (excluding wider-bead)`
- Objective: retain `bridging_angle_is_deterministic` and `flow_correction_stays_positive_for_vertical_input`, adding independent finite/numeric assertions for their already-approved inputs.
- Precondition: the existing bridge integration target and crate-root inline test both register the named tests; production symbols remain unchanged.
- Postcondition: the bridge test still compares two identical calls and additionally checks `90.0` degrees within `1e-6`; the flow test checks finite exact `1.0` for `(0.0, 0.0, 1.0)`; names and targets are unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/bridge_over_infill_tdd.rs` - imports, local `square`, and `bridging_angle_is_deterministic`.
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` - `determine_bridging_angle` and its degree/perpendicular return behavior.
  - `crates/slicer-core/src/lib.rs` - lines `286-330` - `flow_correction` and the inline `tests` module containing the existing vertical test.
  - `crates/slicer-core/Cargo.toml` - existing integration target registration.
  - `docs/22_test_quality.md`, `docs/08_coordinate_system.md`, and `docs/adr/0064-existing-tests-retire-if-unjustified.md` - independent-oracle and coordinate constraints.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/bridge_over_infill_tdd.rs`
  - `crates/slicer-core/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/algos/bridge_over_infill.rs` and every production implementation file
  - triangle, polygon, wider-bead, or unrelated test files
  - `docs/specs/test-quality-remediation-plan.md` (owned by Step 3 only), the queue, fixtures, manifests, and new targets
- Expected sub-agent dispatches:
  - Question: does each exact filter execute its named test and pass the one-passed/zero-failed assertion?; scope: the bridge integration target and crate library target; return: `FACT` pass/fail plus bounded failure `SNIPPETS`.
- Context cost: `S`
- Authoritative docs:
  - `docs/22_test_quality.md` - direct sections on independent production oracles and determinism exceptions.
  - `docs/08_coordinate_system.md` - direct Point2/mm conversion rule.
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` - direct earn-their-keep rule.
- OrcaSlicer refs:
  - None; no parity behavior is asserted.
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test bridge_over_infill_tdd -- --exact bridging_angle_is_deterministic 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail.
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib -- --exact tests::flow_correction_stays_positive_for_vertical_input 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail.
- Exit condition: both exact filters pass one test with zero failures, and inspection confirms the old determinism assertion and test names remain.

### Step 2: Strengthen triangle endpoints and boolean geometry witnesses

- Task IDs: `core/DUP-CORE-strengthen (excluding wider-bead)`
- Objective: retain both triangle crossing tests and the overlapping-square boolean test while adding orientation-insensitive analytic endpoints and independent area/bounds/component assertions.
- Precondition: the existing inline triangle filters and auto-discovered `polygon_ops_tdd` filter are registered; `Line` has public `start`/`end` fields and `shape_signature` already exists.
- Postcondition: both triangle tests assert their approved endpoint pairs in either order; the boolean test asserts all four operation areas, bounds, component counts, and pairwise `shape_signature` distinction without expected vertex-order sequences; existing non-empty guards remain.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs` - `Line`, `triangle_z_intersection`, helpers, and both tests.
  - `crates/slicer-ir/src/slice_ir.rs` - `Point2::from_mm` and point fields only.
  - `crates/slicer-core/tests/polygon_ops_tdd.rs` - `square`, `shape_signature`, boolean test, and nearby bounds assertion patterns.
  - `crates/slicer-core/src/polygon_ops.rs` - lines `291-350` - public operation signatures; do not copy private production area code.
  - `docs/08_coordinate_system.md`, `docs/22_test_quality.md`, and `docs/adr/0064-existing-tests-retire-if-unjustified.md` - geometry and oracle constraints.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/algos/paint_segmentation/triangle_intersect.rs`
  - `crates/slicer-core/tests/polygon_ops_tdd.rs`
- Files explicitly out of bounds:
  - all production files except the test module in the named triangle file
  - bridge, flow, wider-bead, unrelated tests, fixtures, manifests, and new targets
  - `docs/specs/test-quality-remediation-plan.md` (owned by Step 3 only) and the approved queue
- Expected sub-agent dispatches:
  - Question: do the exact triangle and boolean filters execute and pass the one-passed/zero-failed assertion?; scope: `slicer-core` library and `polygon_ops_tdd`; return: `FACT` pass/fail plus bounded failure `SNIPPETS`.
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct Point2 scale and `from_mm` boundary.
  - `docs/22_test_quality.md` - direct independent-oracle and non-source-grep requirements.
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` - direct regression-input standard.
- OrcaSlicer refs:
  - None; no parity behavior is asserted.
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --lib -- --exact algos::paint_segmentation::triangle_intersect::tests::triangle_crossing_two_edges 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log; cargo test -p slicer-core --features host-algos --lib -- --exact algos::paint_segmentation::triangle_intersect::tests::triangle_crossing_one_edge_vertex_on_plane 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail.
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-core --features host-algos --test polygon_ops_tdd -- --exact boolean_ops_produce_expected_presence_for_overlapping_squares 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail.
- Exit condition: all three exact filters pass one test with zero failures; output assertions are numeric/analytic and no expected vertex sequence is introduced.

### Step 3: Record the partial core ledger row

- Task IDs: `core/DUP-CORE-strengthen (excluding wider-bead)`
- Objective: update the existing `core` row only inside the real §7 Ledger span, preserving accumulated prior content and recording this packet’s changed symbols, surviving coverage, exact new oracles, own validation command, and remaining gap.
- Precondition: Steps 1 and 2 have passed their narrow filters; the plan contains `## 7. Ledger (progress record; rows store re-derivable facts, never frozen counts)` followed later by `## Packet Queue`; the queue remains parent-owned.
- Postcondition: exactly one six-column `core` row in that span has state `partial` or `partial (parenthesized detail)`; its changed cell contains `core-strengthen` and all five real test names, its surviving cell contains all five names and the exact new oracle tokens required by AC-5, and its Validation cell preserves accumulated prior commands while containing, anywhere, the whitespace-normalized token-bounded AC-4 representative invocation `cargo test -p slicer-core --features host-algos --test polygon_ops_tdd -- --exact boolean_ops_produce_expected_presence_for_overlapping_squares` with optional trailing flags or a closing code-span backtick allowed. Its gap cell contains `core-retire`, `core-paint`, `core-brittle`, `core-cross`, and `core-parity`. The next queue heading and all queue rows are unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - real §7 Ledger heading through the next `## Packet Queue`, including accumulated `core` row content only.
  - `docs/specs/test-quality-remediation-plan.md` - queue boundary for a no-edit comparison; do not read or rewrite unrelated plan sections.
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md` (the single §7 `core` row only)
- Files explicitly out of bounds:
  - the `## Packet Queue` table and every other plan row/section
  - all source/test files, fixtures, manifests, generated artifacts, lockfiles, and other packets
- Expected sub-agent dispatches:
  - Question: does the real heading-to-queue span contain exactly one updated `core` row satisfying the six-column/partial/oracle/token-bounded-AC-4-validation/gap grammar while preserving prior content and queue text?; scope: `docs/specs/test-quality-remediation-plan.md`; return: `FACT` with the real-plan predicate result.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - delegated ledger and queue contract.
  - `docs/22_test_quality.md` - direct report-mode quality-gate contract.
  - `docs/21_data_defaults_and_fixtures.md` - direct literal gate contract.
- OrcaSlicer refs:
  - None.
- Verification:
  - Run the exact AC-5 real-plan predicate against `docs/specs/test-quality-remediation-plan.md`; it must print one bounded `core ledger predicate: PASS` line only after the actual §7 `core` row has been updated and its Validation cell contains the exact AC-4 representative command. Do not manufacture a row or use a temporary copy for closure.
  - `set -euo pipefail; mkdir -p target; cargo xtask check-literals 2>&1 | tee target/core-strengthen-literals.log >/dev/null` - FACT pass/fail.
  - `set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report 2>&1 | tee target/core-strengthen-quality.log >/dev/null` - FACT pass/fail.
- Exit condition: the real-plan predicate passes against the implemented row; the plan diff contains only the `core` row in §7, with no queue change. Authoring-only synthetic controls must reject the current open row (exit 1), accept a valid accumulated row with the exact AC-4 command (exit 0), and reject `unrelated_target`, wrong filters, partial commands, missing `--features host-algos`, and the prior malformed variants, while accepting the backticked house-style form. Synthetic controls are diagnostics only and are not closure evidence.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Two existing test surfaces; exact bridge and flow filters. |
| Step 2 | S | Two existing test surfaces; endpoint and boolean geometry assertions. |
| Step 3 | S | One bounded ledger-row edit and real-plan predicate; authoring controls remain non-closure diagnostics. |

Aggregate context cost: `S`.

## Packet Completion Gate

- All three steps and exits are complete.
- Every pipe-suffixed acceptance command returns PASS through its delegated runtime check.
- The `core` ledger row is updated by the implementation worker through the bounded plan dispatch; AC-5 proves the actual row and the packet author does not edit the plan during generation.
- The queue remains parent-owned and is not changed by this packet’s implementation step.
- `packet.spec.md` remains `draft` until independent preflight and later acceptance; no implementation result is claimed here.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and the three packet-level gate commands.
- Record any remaining packet-local risk, especially output-order changes that still satisfy the independent geometry contract.
- Confirm the implementation worker used only the listed files and that the ledger row is `partial`, not a premature wave closure.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands use the required workspace/feature flags and tee test output as specified above.

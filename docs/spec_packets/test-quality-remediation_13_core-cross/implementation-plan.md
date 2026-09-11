# Implementation Plan: core-cross

## Execution Rules

- Work one atomic step at a time; map every step to the `core/CROSS` task ID.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- Steps 1-4 are independent edit surfaces (one file each) and may run in any order; Step 5 (ledger) runs last.
- All cargo **test** invocations tee to `target/test-output.log`; check/clippy use dedicated logs.

## Steps

### Step 1: SDK wrapper markers + new miter-limit delegate test

- Task IDs: `core/CROSS`
- Objective: add the seven KEEP-review marker comments (six on retained tests + one above the new test) and the new `offset_polygons_with_miter_limit_clamps_sharp_miter_corners` test to `crates/slicer-sdk/tests/host_wrappers_tdd.rs`; insert the new test immediately after `offset_polygons_shrinks_and_grows`'s closing brace.
- Precondition: read-only confirmation of the delegation arms (`crates/slicer-sdk/src/host.rs` lines `520-620`: `offset_polygons` → `offset_polygons_with_optional_miter_limit` → `slicer_core::polygon_ops::offset` / `offset_with_miter_limit`; core `offset` uses default miter limit `2.0`).
- Postcondition: AC-1, AC-3, AC-4 marker pins and the AC-N1 12-name roster (new test at pinned position) hold; the file compiles (sdk test target builds under `--features test`).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/host.rs` - lines `520-620` only - delegation arms + inline simplify branch
  - `crates/slicer-sdk/tests/host_wrappers_tdd.rs` - lines `85-230` only - current tests and `square()` helper
  - `crates/slicer-sdk/Cargo.toml` - lines `95-110` only - `required-features`
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/tests/host_wrappers_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-sdk/src/host.rs`, `crates/slicer-sdk/src/host_batch.rs` (Step 2), all other crates, `docs/` (Step 5), census JSON
- Blast-radius discipline: not applicable — no struct fields or schema constants change; the new test adds no struct literal of a watched type (it calls the existing `square()` helper and existing host functions).
- Expected sub-agent dispatches:
  - Question: run the AC-1 command and the AC-3/AC-4 commands; report PASS/FAIL each; scope: workspace cargo + the one test file; return: `FACT`; purpose: step validation without loading output.
- Context cost: `S`
- Authoritative docs:
  - `docs/22_test_quality.md` - §§1-3 and §5 (earn-their-keep, legitimate weak forms, gate table); direct read or delegated SUMMARY
  - `docs/adr/0064-existing-tests-retire-if-unjustified.md` - direct read (44 lines)
- OrcaSlicer refs: none.
- Verification:
  - `set -euo pipefail; mkdir -p target; cargo test -p slicer-sdk --features test --test host_wrappers_tdd -- offset_polygons_with_miter_limit_clamps_sharp_miter_corners --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log` - FACT pass/fail; if the new test fails, inspect the assertion that failed (read the tee'd log), adjust the count-strictness per design.md's documented risk, and re-run — the pinned assertions (non-empty, `assert_ne!`, strictly-more) must stay intact.
  - AC-3 and AC-4 command lines from `packet.spec.md` - FACT pass/fail.
  - `set -euo pipefail; mkdir -p target; cargo check --workspace --all-targets 2>&1 | tee target/core-cross-check.log >/dev/null` - FACT pass/fail.
- Exit condition: AC-1, AC-3, AC-4 all PASS and the 12-name roster pin from AC-N1 holds for this file.

### Step 2: Batch offset wrapper markers

- Task IDs: `core/CROSS`
- Objective: add the two KEEP-review marker comments in `crates/slicer-sdk/src/host_batch.rs` above `batch_offset_keeps_results_aligned_with_inputs` and `batch_forms_agree_with_the_singular_forms` in the `#[cfg(test)]` tests module.
- Precondition: read-only confirmation of the native batch mapping (`crates/slicer-sdk/src/host_batch.rs` lines `136-180`: `offset_polygons_batch` maps over `offset_polygons_with_miter_limit`/`offset_polygons`).
- Postcondition: AC-2 marker pins hold; both batch tests still pass; guest freshness check run and reconciled.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/host_batch.rs` - lines `136-180` and `424-496` only
- Files allowed to edit (at most 3):
  - `crates/slicer-sdk/src/host_batch.rs`
- Files explicitly out of bounds:
  - `crates/slicer-sdk/tests/*`, `crates/slicer-core/**`, `docs/` (Step 5), census JSON
- Blast-radius discipline: not applicable — comment-only edits to a src file; the guest-staleness bullet in `design.md` applies because this path is inside `crates/slicer-sdk/**`.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask build-guests --check` and report the numeric exit code only; scope: xtask; return: `FACT`; purpose: guest-fingerprint freshness after the src edit.
  - Question: run the AC-2 command; report PASS/FAIL; scope: workspace cargo + the one src file; return: `FACT`; purpose: step validation.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 CROSS row and §6 log discipline (ranged reads)
- OrcaSlicer refs: none.
- Verification:
  - `set -euo pipefail; mkdir -p target; for t in batch_offset_keeps_results_aligned_with_inputs batch_forms_agree_with_the_singular_forms; do cargo test -p slicer-sdk --features test --lib "$t" -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log || { echo "FAIL: $t"; exit 1; }; done` - FACT pass/fail.
  - `cargo xtask build-guests --check` — exit 0 = fresh (done); exit 1 = stale → run `cargo xtask build-guests` then re-run the AC-2 command; exit 3 = infrastructure error → record and escalate, never treat as fresh.
- Exit condition: AC-2 PASS; both batch unit tests green; `build-guests --check` result reconciled (fresh, or stale-then-rebuilt).

### Step 3: Core AabbTree counterpart markers

- Task IDs: `core/CROSS`
- Objective: add the three KEEP-review marker comments in `crates/slicer-core/tests/aabb_tree_tdd.rs` above `bounds_match_unit_cube_vertex_extrema`, `positive_z_raycast_from_below_hits_cube_bottom_face_first`, and `raycast_all_hits_returns_sorted_entry_and_exit_intersections`.
- Precondition: read-only confirmation of the `AabbTree` method shapes (`crates/slicer-core/src/aabb_tree.rs` lines `40-60`).
- Postcondition: AC-5 marker pins hold; all six tests still pass (3 marked + 3 unmarked roster-protected); the 6-name roster pin holds.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/aabb_tree.rs` - lines `40-60` only
  - `crates/slicer-core/tests/aabb_tree_tdd.rs` - lines `85-240` only
  - `docs/specs/test-quality-remediation-census.json` - lines `256-270` only - `aabb_tree_tdd` target entry (`required: []`)
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/aabb_tree_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/aabb_tree.rs` (read-only), `crates/slicer-sdk/**`, `docs/` (Step 5), census JSON (read-only)
- Blast-radius discipline: not applicable — comment-only edits to a test file; no struct literals added.
- Expected sub-agent dispatches:
  - Question: run the AC-5 command; report PASS/FAIL; scope: workspace cargo + the one test file; return: `FACT`; purpose: step validation.
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY (unit convention only)
- OrcaSlicer refs: none.
- Verification:
  - `set -euo pipefail; mkdir -p target; for t in bounds_match_unit_cube_vertex_extrema positive_z_raycast_from_below_hits_cube_bottom_face_first raycast_all_hits_returns_sorted_entry_and_exit_intersections; do cargo test -p slicer-core --features host-algos --test aabb_tree_tdd -- "$t" --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log || { echo "FAIL: $t"; exit 1; }; done` - FACT pass/fail.
  - `grep -Eq '^test result: ok\. 6 passed; 0 failed;' target/test-output.log` via a whole-file run (`cargo test -p slicer-core --features host-algos --test aabb_tree_tdd 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 6 passed; 0 failed;' target/test-output.log`) - FACT pass/fail: full-file green proves the unmarked roster tests still pass.
- Exit condition: AC-5 PASS; whole-file run reports 6 passed; 6-name roster pin holds.

### Step 4: Core polygon-op counterpart markers

- Task IDs: `core/CROSS`
- Objective: add the two KEEP-review marker comments in `crates/slicer-core/src/polygon_ops.rs` above `offset_round_trip_preserves_hole_nesting` and `expolygons_simplify_preserves_square` inside `mod tests`; production functions untouched.
- Precondition: read-only confirmation of `offset`/`offset_with_miter_limit`/`expolygons_simplify` shapes (`crates/slicer-core/src/polygon_ops.rs` lines `420-500` and `700-800`).
- Postcondition: AC-6 marker pins hold; the two marked tests still pass; no production line changed.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/polygon_ops.rs` - lines `420-500`, `700-800`, `900-960`, `990-1020`, `1070-1120`, `1170-1200` only - production shapes plus the tests-mod region
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/polygon_ops.rs`
- Files explicitly out of bounds:
  - the production halves of `polygon_ops.rs` (read-only), `crates/slicer-core/tests/polygon_ops_tdd.rs`, `crates/slicer-sdk/**`, `docs/` (Step 5), census JSON
- Blast-radius discipline: not applicable — comments only, inside `mod tests`.
- Expected sub-agent dispatches:
  - Question: run the AC-6 command; report PASS/FAIL; scope: workspace cargo + the one src file; return: `FACT`; purpose: step validation.
- Context cost: `S`
- Authoritative docs:
  - `docs/22_test_quality.md` - §§1-3 (earn-their-keep mapping for the two oracles); direct read or delegated SUMMARY
- OrcaSlicer refs: none.
- Verification:
  - `set -euo pipefail; mkdir -p target; for t in offset_round_trip_preserves_hole_nesting expolygons_simplify_preserves_square; do cargo test -p slicer-core --features host-algos --lib "$t" -- --nocapture 2>&1 | tee target/test-output.log >/dev/null; grep -Eq '^test result: ok\. 1 passed; 0 failed;' target/test-output.log || { echo "FAIL: $t"; exit 1; }; done` - FACT pass/fail.
  - `git diff --stat crates/slicer-core/src/polygon_ops.rs` shows only comment lines added (implementer-side check; no gate command) - FACT-like local check.
- Exit condition: AC-6 PASS; diff on the file touches only comment lines.

### Step 5: §7 ledger core-row update

- Task IDs: `core/CROSS`
- Objective: append this packet's dispositions to the single `core` row in §7 of `docs/specs/test-quality-remediation-plan.md` — state `partial` (or `partial (core-cross done)`-style parenthesized detail if already `partial`), Retired/changed symbols gains the `core-cross` token and the fourteen reviewed test names, Surviving/new coverage gains the fourteen names plus oracle tokens `KEEP-review (core-cross)`, `assert_ne`, `1.2`, Validation gains the representative command plus the four gate tokens, Remaining gap contains `core-parity` and `census-target-scope` and no longer contains `core-cross`; preserve all accumulated content from earlier packets.
- Precondition: Steps 1-4 complete; current §7 row fetched via the dispatch below.
- Postcondition: AC-7 predicate passes against the real plan file; no other plan section, queue row, or cell changed.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - §7 ledger table rows and the §6 command/log rules only; ranged reads
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md`
- Files explicitly out of bounds:
  - Packet Queue rows (parent-owned), §5.1 row text, census JSON, every other file
- Blast-radius discipline: not applicable — documentation row append.
- Expected sub-agent dispatches:
  - Question: read the current §7 `core` row and report its six cells verbatim; scope: plan §7 only; return: `FACT`; purpose: append base for this step.
  - Question: run the AC-7 command; report PASS/FAIL; scope: plan file only; return: `FACT`; purpose: step validation.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §7 ledger ownership (ranged read)
- OrcaSlicer refs: none.
- Verification:
  - AC-7 command from `packet.spec.md` - FACT pass/fail (read-only predicate against the real file).
  - `set -euo pipefail; mkdir -p target; cargo xtask check-literals; cargo xtask check-test-quality --report 2>&1 | tail -5` - FACT pass/fail, report mode must not list findings on the four touched test surfaces.
- Exit condition: AC-7 PASS; check-literals clean; check-test-quality report shows no unwaived findings on the four touched paths.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | one file; seven markers (six retained + one new-test) + one new test |
| Step 2 | S | one file; two markers; guest-staleness check |
| Step 3 | S | one file; three markers |
| Step 4 | S | one file; two markers |
| Step 5 | S | one ledger row append |

Aggregate `M` (approved), largest step `S`; no L step, no split required.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS (AC-1 through AC-7, AC-N1).
- The four gate commands pass: `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, `cargo xtask check-test-quality --report` (zero unwaived on the four touched surfaces).
- The §7 ledger append is recorded through a worker dispatch against the real plan file.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: the strict point-count assertion's Clipper2 detail (design.md Risks), and the guest-freshness follow-up if Step 2's `build-guests --check` reported stale.
- Confirm context stayed at or below 150k standard; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.

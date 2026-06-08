# Implementation Plan: 90_regression-wedge-stl-swap

## Execution Rules

- One atomic step at a time.
- Maps to `TASK-240`.
- File rename + body edits + harness `mod` declaration update happen in a single Git commit.
- Test output teed to `target/test-output.log` per `CLAUDE.md` §Test Discipline; never re-run a multi-minute suite to re-read stdout.
- The `cargo test -p slicer-runtime` wall-clock measurement is the final acceptance step, not a per-step measurement.

## Steps

### Step 1: Author `regression_wedge.stl` deterministically; verify size + feature inventory

- Task IDs:
  - `TASK-240`
- Objective: produce `resources/regression_wedge.stl` containing all six documented features (40 mm height, 45° overhang, flat top ≥ 25 × 25 mm, flat bottom ≥ 25 × 25 mm, 10 mm bridge gap, ironable top ≥ 25 × 25 mm) at ≤ 50 KB; document authoring procedure + SHA-256 in implementer's notes.
- Precondition: working tree clean. If OpenSCAD or equivalent is unavailable, the implementer must surface this open question before continuing.
- Postcondition: `resources/regression_wedge.stl` exists; size ≤ 51,200 bytes; SHA-256 recorded; closure-log scaffold contains the authoring procedure.
- Files allowed to read:
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0b" — lines 268-330 approx.
- Files allowed to edit (≤ 3):
  - `resources/regression_wedge.stl` (CREATE).
- Files explicitly out-of-bounds for this step:
  - All test sources. Resist the temptation to look ahead.
- Expected sub-agent dispatches:
  - "Run `wc -c < resources/regression_wedge.stl`; return FACT (integer)" — purpose: AC-1 / AC-N3 size check.
  - "Run `sha256sum resources/regression_wedge.stl`; return FACT (single hash)" — purpose: AC-8 record.
  - "Given the wedge STL at `resources/regression_wedge.stl`, run pnp_cli's mesh-analyze (or a small slicer-helpers Rust harness) and report: bounding box, triangle count, presence of an overhanging face at >= 30° from vertical, and the area of the largest top-facing flat region. Return SUMMARY ≤ 100 words" — purpose: AC-1 feature-inventory verification.
- Context cost: `M`.
- Authoritative docs:
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0b" — feature inventory.
- OrcaSlicer refs: none.
- Verification:
  - File exists; size ≤ 50 KB; feature-inventory SUMMARY confirms all six features.
- Exit condition: AC-1 satisfied; authoring procedure documented.

### Step 2: Inventory `benchy_end_to_end_tdd.rs` tests by classification

- Task IDs:
  - `TASK-240`
- Objective: produce an authoritative function-name × classification table for the 42 tests in `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs`. This drives the prefix sweep in Step 3.
- Precondition: Step 1 complete.
- Postcondition: a 42-entry table (test name, CLI-SHAPE / SHAPE-DEPENDENT / STRUCTURAL, target prefix `slice_*` or `wedge_*`, expected wedge feature for SHAPE-DEPENDENT tests) recorded in implementer's notes.
- Files allowed to read:
  - `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` — range-read by test (locate `#[test]` boundaries; read 10-30 lines around each).
- Files allowed to edit (≤ 3):
  - None.
- Files explicitly out-of-bounds for this step:
  - Any other test file.
- Expected sub-agent dispatches:
  - "Open `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs`. For each `#[test]` function, return LOCATIONS: function name + 1-line summary of the strongest assertion. Cap at 42 entries" — purpose: feed the classification.
  - Cross-check returned classification against `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0b" test-classification table (22 / 17 / 3).
- Context cost: `S`.
- Authoritative docs: roadmap §"P0b".
- OrcaSlicer refs: none.
- Verification: classification table matches the roadmap's counts (22 + 17 + 3 = 42) within ±2 (drift from when the roadmap was authored is acceptable; document any drift in closure log).
- Exit condition: table recorded; Step 3 can write specific assertion targets.

### Step 3: Rename `benchy_end_to_end_tdd.rs` → `slice_end_to_end_tdd.rs`; function-prefix sweep; fixture-path swap

- Task IDs:
  - `TASK-240`
- Objective: rename the test file; rewrite each `#[test] fn benchy_*` to `fn slice_*` or `fn wedge_*` per the Step 2 classification; swap every `resources/benchy.stl` literal to `resources/regression_wedge.stl`; calibrate marker-targeting assertions to the wedge's known features (e.g., the `;TYPE:Top surface` test now targets the top section's last layers).
- Precondition: Steps 1 and 2 complete.
- Postcondition: file renamed; 42 tests pass; classification preserved (no silently weakened assertion).
- Files allowed to read:
  - `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` (the file being renamed; full read acceptable since we're editing the whole file).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` (renamed in the same commit).
  - `crates/slicer-runtime/tests/e2e.rs` — update `mod` declaration.
- Files explicitly out-of-bounds for this step:
  - Non-test files; reference sites in other crates (Step 4 territory).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 | tee target/test-output.log`; return FACT pass/fail with overall test count" — purpose: validate.
  - "Run `cargo test -p slicer-runtime --test e2e slice_end_to_end 2>&1 | tee target/test-output.log` (on failure): SNIPPETS ≤ 20 lines around the first `FAILED` block" — purpose: targeted diagnosis.
- Context cost: `M`.
- Authoritative docs: roadmap §"P0b".
- OrcaSlicer refs: none.
- Verification:
  - All 42 tests pass.
  - File contains zero `benchy` substrings; renamed correctly.
- Exit condition: AC-4 and AC-5 satisfied.

### Step 4: Update 4 non-test reference sites; verify per-crate tests

- Task IDs:
  - `TASK-240`
- Objective: edit each of the 4 known reference sites and confirm the affected tests pass.
- Precondition: Step 3 green.
- Postcondition: all 4 reference sites point at `regression_wedge.stl`; respective crates' tests pass.
- Files allowed to read:
  - `crates/slicer-runtime/tests/common/slicer_cache.rs` — read lines 120-150.
  - `crates/slicer-model-io/tests/stl_roundtrip_tdd.rs` — read lines 1-50.
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` — read lines 320-345.
  - `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs` — read lines 20-50.
- Files allowed to edit (≤ 3 per sub-step):
  - All four files above (split across two sub-commits if preferred: `slicer-runtime` reference sites in one commit, `slicer-model-io` + `pnp-cli` in another).
- Files explicitly out-of-bounds for this step:
  - Any production source.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-model-io --test stl_roundtrip_tdd`; return FACT pass/fail" — purpose: validate.
  - "Run `cargo test -p slicer-runtime --test integration live_module_loading`; return FACT pass/fail".
  - "Run `cargo test -p pnp-cli --test slice_instrumentation_fork_tdd`; return FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test e2e slice_end_to_end`; return FACT pass/fail" — purpose: regression check (cache-key changes in `slicer_cache.rs` could break Step 3's tests).
- Context cost: `S`.
- Authoritative docs: roadmap §"P0b" reference site list.
- OrcaSlicer refs: none.
- Verification: all four per-crate tests pass.
- Exit condition: AC-6 satisfied.

### Step 5: Delete `resources/benchy.stl`; residual-reference sweep

- Task IDs:
  - `TASK-240`
- Objective: delete the benchy STL; confirm zero residual references.
- Precondition: Step 4 green.
- Postcondition: file deleted; AC-2 and AC-3 hold.
- Files allowed to read:
  - None.
- Files allowed to edit (≤ 3):
  - `resources/benchy.stl` (delete only).
- Files explicitly out-of-bounds for this step:
  - Any test source. If a residual is found, this step FAILS; roll back to the missing site's per-file step.
- Expected sub-agent dispatches:
  - "Run `rg -n --glob '!.ralph/specs/90_regression-wedge-stl-swap/**' 'benchy\.stl' crates/ modules/ docs/ .ralph/`; return LOCATIONS or empty" — purpose: sweep.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - File deleted (`test ! -f resources/benchy.stl`).
  - Sweep returns empty.
- Exit condition: AC-2, AC-3 satisfied.

### Step 6: Wall-clock measurement + closure-log scaffold

- Task IDs:
  - `TASK-240`
- Objective: measure cold-cache wall-clock for `cargo test -p slicer-runtime` against the pre-migration baseline (captured at packet activation) and record the after-migration time. Confirm AC-7's ≥ 60 second improvement floor.
- Precondition: Steps 1-5 green.
- Postcondition: closure log records before/after wall-clock with the delta.
- Files allowed to read: none.
- Files allowed to edit: none (the closure-log entry is in implementer's notes / commit message, not a tracked file).
- Files explicitly out-of-bounds for this step: any.
- Expected sub-agent dispatches:
  - "Run `cargo clean -p slicer-runtime && /usr/bin/time -f '%e' cargo test -p slicer-runtime 2>&1 | tee target/test-output.log | tail -5`; return FACT with elapsed time in seconds" — purpose: post-migration timing.
  - The before-migration timing must already be recorded at packet activation (Step 0 implicit baseline); if it was not, capture it via `git stash && cargo clean && time cargo test && git stash pop` before continuing.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: AC-7 — before − after ≥ 60 s.
- Exit condition: AC-7 satisfied; closure log scaffold contains authoring procedure (AC-8), SHA-256, and wall-clock measurement.

### Step 7: Final acceptance ceremony — full e2e bucket + clippy

- Task IDs:
  - `TASK-240`
- Objective: workspace gate.
- Precondition: Steps 1-6 green.
- Postcondition: clippy clean; full e2e bucket green; integration bucket green.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files explicitly out-of-bounds for this step: any.
- Expected sub-agent dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test e2e 2>&1 | tee target/test-output.log`; return FACT pass/fail with overall count".
  - "Run `cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log`; return FACT pass/fail".
- Context cost: `S` (dispatch-only).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: all three dispatches return PASS.
- Exit condition: AC-10-equivalent (clippy + e2e + integration) satisfied; packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Wedge authoring + verification. |
| Step 2 | S | Pure dispatch — test inventory. |
| Step 3 | M | 42-test file rewrite. |
| Step 4 | S | 4 reference-site swaps. |
| Step 5 | S | Delete + sweep. |
| Step 6 | S | Wall-clock measurement. |
| Step 7 | S | Workspace gate. |

Aggregate: M (no L step).

## Packet Completion Gate

- All 7 steps complete; each exit condition satisfied.
- AC-1 through AC-8 + AC-N1, AC-N2, AC-N3 verified.
- Closure log contains: wedge authoring procedure, wedge SHA-256, before/after `cargo test -p slicer-runtime` wall-clock, assertion-diff for SHAPE-DEPENDENT tests (AC-N1).
- `docs/07_implementation_status.md` updated to record `TASK-240` as implemented and link to `.ralph/specs/90_regression-wedge-stl-swap/` (delegate the edit).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`; confirm each PASS.
- Confirm `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, e2e bucket, integration bucket all green via sub-agent FACT.
- Record wall-clock delta in the closure log (AC-7).
- Record wedge SHA-256 in the closure log (AC-8).
- Confirm peak context usage stayed under 70%.

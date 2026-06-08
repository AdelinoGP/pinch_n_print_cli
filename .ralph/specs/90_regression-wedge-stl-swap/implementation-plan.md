# Implementation Plan: 90_regression-wedge-stl-swap

## Execution Rules

- One atomic step at a time.
- Maps to `TASK-240`.
- File rename + body edits + harness `mod` declaration update happen in a single Git commit.
- Test output teed to `target/test-output.log` per `CLAUDE.md` §Test Discipline; never re-run a multi-minute suite to re-read stdout.
- The `cargo test -p slicer-runtime` wall-clock measurement is the final acceptance step, not a per-step measurement.

## Steps

### Step 0: Capture pre-migration baseline metrics into closure-log

- Task IDs:
  - `TASK-240`
- Objective: capture the **pre-migration** values that AC-N1, AC-N2, and AC-7 will compare against — assertion count in `benchy_end_to_end_tdd.rs`, cold-cache wall-clock for `cargo test -p slicer-runtime`, byte SHA-256 of current `resources/benchy.stl` (for completeness). Persist into `.ralph/specs/90_regression-wedge-stl-swap/closure-log.md`.
- Precondition: working tree clean; packet `status: draft` (this step runs at activation, before any code edits).
- Postcondition: `.ralph/specs/90_regression-wedge-stl-swap/closure-log.md` exists and contains, at minimum, the lines `PRE_ASSERT_COUNT=<integer>`, `WALL_CLOCK_BEFORE_E2E=<integer-seconds>` (e2e bucket only, per AC-7's clarified scope), `BENCHY_SHA256_BEFORE=<hex>`. Each on its own line, in the form `KEY=VALUE` so subsequent ACs can `grep | cut`.
- Files allowed to read:
  - `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` (count only — dispatched).
- Files allowed to edit (≤ 3):
  - `.ralph/specs/90_regression-wedge-stl-swap/closure-log.md` (CREATE).
- Files explicitly out-of-bounds for this step: all other test sources, all production source.
- Expected sub-agent dispatches:
  - "Run `rg -c --no-filename '^\\s*assert(_eq|_ne)?!' crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs`; return FACT integer" — purpose: PRE_ASSERT_COUNT.
  - "Run `cargo clean -p slicer-runtime && START=$(date +%s) && cargo test -p slicer-runtime --test e2e 2>&1 | tee target/test-output.log | tail -5 && END=$(date +%s) && echo \"ELAPSED=$((END-START))\"`; return FACT integer (seconds) plus the final `test result` summary line" — purpose: WALL_CLOCK_BEFORE_E2E (e2e bucket only, per AC-7's clarified scope).
  - "Run `sha256sum resources/benchy.stl`; return FACT (single hex)" — purpose: BENCHY_SHA256_BEFORE (provenance).
- Context cost: `S` (dispatch + small file write).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: `grep -E '^(PRE_ASSERT_COUNT|WALL_CLOCK_BEFORE_E2E|BENCHY_SHA256_BEFORE)=' .ralph/specs/90_regression-wedge-stl-swap/closure-log.md | wc -l` returns 3.
- Exit condition: closure-log scaffold populated; AC-N1 / AC-7 baselines pinned.

### Step 1: Author `regression_wedge.stl` deterministically; verify size + feature inventory

- Task IDs:
  - `TASK-240`
- Objective: produce `resources/regression_wedge.stl` containing all six documented features (40 mm height, 45° overhang, flat top ≥ 25 × 25 mm, flat bottom ≥ 25 × 25 mm, 10 mm bridge gap, ironable top ≥ 25 × 25 mm) at ≤ 50 KB; append authoring procedure + `WEDGE_SHA256=<hex>` line to `.ralph/specs/90_regression-wedge-stl-swap/closure-log.md`.
- Precondition: Step 0 complete. Any deterministic authoring procedure is acceptable (see design.md "Open Questions"); if the chosen procedure is non-deterministic across runs, pin the canonical SHA-256 in the closure log and call out the non-determinism explicitly.
- Postcondition: `resources/regression_wedge.stl` exists; size ≤ 51,200 bytes; closure-log contains `WEDGE_SHA256=…`, a `## Authoring Procedure` section (AC-8), and a `## Feature Inventory` section with the five `KEY=VALUE` lines (AC-1b).
- Files allowed to read:
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0b" — heading at line 204 (read ~60 lines).
- Files allowed to edit (≤ 3):
  - `resources/regression_wedge.stl` (CREATE).
  - `.ralph/specs/90_regression-wedge-stl-swap/closure-log.md` (APPEND: `WEDGE_SHA256=…` + authoring procedure block).
- Files explicitly out-of-bounds for this step:
  - All test sources. Resist the temptation to look ahead.
- Expected sub-agent dispatches:
  - "Run `wc -c < resources/regression_wedge.stl`; return FACT (integer)" — purpose: AC-1 / AC-N3 size check.
  - "Run `sha256sum resources/regression_wedge.stl`; return FACT (single hash)" — purpose: AC-8 / AC-N2 record.
  - "Given the wedge STL at `resources/regression_wedge.stl`, run pnp_cli's mesh-analyze (or a small slicer-helpers Rust harness) and report: bounding box, triangle count, max overhang angle from vertical, area of the largest top-facing flat region, and bridge gap width. Return a SUMMARY ≤ 100 words AND a structured `KEY=VALUE` block for these keys: `bounding_box_height_mm`, `triangle_count`, `max_overhang_angle_deg`, `largest_flat_top_area_mm2`, `bridge_gap_width_mm`" — purpose: AC-1b feature-inventory verification. The implementer copies the `KEY=VALUE` block verbatim into `closure-log.md` under `## Feature Inventory`. **Note**: if `pnp_cli mesh-analyze` does not exist as a subcommand, fall back to a one-shot `slicer-helpers` Rust harness (the wedge format is binary STL; standard parsers suffice).
- Context cost: `M`.
- Authoritative docs:
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P0b" (line 204+) — feature inventory.
- OrcaSlicer refs: none.
- Verification:
  - File exists; size ≤ 50 KB; feature-inventory SUMMARY confirms all six features.
- Exit condition: AC-1, AC-1b, AC-8 satisfied; authoring procedure + feature inventory documented; `WEDGE_SHA256` pinned in closure log.

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
- Precondition: Steps 1 and 2 complete. Note: AC-5's pipe-suffix grep targets the **renamed** file path (`slice_end_to_end_tdd.rs`); replaying it before the rename completes is a category error, not a test failure.
- Postcondition: file renamed; 42 tests pass; classification preserved (no silently weakened assertion).
- Files allowed to read:
  - `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` (the file being renamed; full read acceptable since we're editing the whole file).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/e2e/benchy_end_to_end_tdd.rs` (renamed in the same commit to `slice_end_to_end_tdd.rs`).
  - `crates/slicer-runtime/tests/e2e/main.rs` — update `mod` declaration on line 12 (`mod benchy_end_to_end_tdd;` → `mod slice_end_to_end_tdd;`).
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

### Step 4: Update 5 non-test reference sites; verify per-crate tests

- Task IDs:
  - `TASK-240`
- Objective: edit each of the 5 known reference sites (4 in `crates/`, 1 in `modules/core-modules/support-planner/`) and confirm the affected tests pass.
- Precondition: Step 3 green.
- Postcondition: all 5 reference sites point at `regression_wedge.stl`; respective crates' tests pass.
- Files allowed to read:
  - `crates/slicer-runtime/tests/common/slicer_cache.rs` — read lines 120-150 (`benchy.stl` at :135).
  - `crates/slicer-model-io/tests/stl_roundtrip_tdd.rs` — read lines 1-50 (`benchy.stl` at :1,15-17).
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` — read lines 320-345 (`benchy.stl` at :332).
  - `crates/pnp-cli/tests/slice_instrumentation_fork_tdd.rs` — read lines 20-50 (`benchy.stl` at :32).
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` — range-read around the `benchy.stl` occurrence (located via a single Grep dispatch — line not pre-captured).
  - `modules/core-modules/support-planner/Cargo.toml` — for the cargo package name needed by the `cargo test -p` verification command.
- Files allowed to edit (≤ 3 per sub-step):
  - All five files above (split across two or three sub-commits if preferred: `slicer-runtime` reference sites in one commit, `slicer-model-io` + `pnp-cli` in another, `modules/core-modules/support-planner` in a third).
- Files explicitly out-of-bounds for this step:
  - Any production source under `crates/*/src/` or `modules/core-modules/*/src/`.
- Expected sub-agent dispatches:
  - "Run `rg -n 'benchy\\.stl' modules/core-modules/support-planner/tests/orca_parity_tdd.rs`; return LOCATIONS (line + 1-line context)" — purpose: resolve the modules-site line(s) before edit.
  - "Run `rg -nE '^name = ' modules/core-modules/support-planner/Cargo.toml | head -n1`; return FACT (the package-name line)" — purpose: resolve the `cargo test -p <name>` value.
  - "Run `cargo test -p slicer-model-io --test stl_roundtrip_tdd`; return FACT pass/fail" — purpose: validate site 2.
  - "Run `cargo test -p slicer-runtime --test integration live_module_loading`; return FACT pass/fail" — purpose: validate site 3.
  - "Run `cargo test -p pnp-cli --test slice_instrumentation_fork_tdd`; return FACT pass/fail" — purpose: validate site 4.
  - "Run `cargo test -p <support-planner-pkg-name> --test orca_parity_tdd`; return FACT pass/fail" — purpose: validate site 5.
  - "Run `cargo test -p slicer-runtime --test e2e slice_end_to_end`; return FACT pass/fail" — purpose: regression check (cache-key changes in `slicer_cache.rs` could break Step 3's tests).
- Context cost: `S`.
- Authoritative docs: roadmap §"P0b" reference-site list (line 204+).
- OrcaSlicer refs: none.
- Verification: all five per-crate tests pass; `! rg -q 'benchy\.stl'` on all five files.
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
  - "Run `rg -n 'benchy\\.stl' crates/ modules/`; return LOCATIONS or empty" — purpose: live-code residual sweep per AC-3.
  - "Run `rg -n 'benchy\\.stl' docs/ .ralph/` (informational only — historical mentions in roadmap and prior packets are allowed); return LOCATIONS or empty" — purpose: confirm no surprise consumer in docs.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - File deleted (`test ! -f resources/benchy.stl`).
  - Live-code sweep returns empty.
- Exit condition: AC-2, AC-3 satisfied.

### Step 6: Post-migration e2e wall-clock + closure-log finalization

- Task IDs:
  - `TASK-240`
- Objective: measure cold-cache wall-clock for `cargo test -p slicer-runtime --test e2e` (the only bucket where the swap moves wall-clock — see closure-log Step 0 Notes for AC-7 scope rationale) after the swap; compare to `WALL_CLOCK_BEFORE_E2E`; confirm AC-7's ≥ 60-second improvement floor. Append `WALL_CLOCK_AFTER_E2E=…` and the per-test assertion-diff (AC-N1 manual audit) to the closure log.
- Precondition: Steps 0-5 green. `WALL_CLOCK_BEFORE_E2E` and `PRE_ASSERT_COUNT` already pinned in `closure-log.md` by Step 0.
- Postcondition: `closure-log.md` contains `WALL_CLOCK_AFTER_E2E=<integer-seconds>`, the delta, and the assertion-diff section.
- Files allowed to read:
  - `.ralph/specs/90_regression-wedge-stl-swap/closure-log.md` (read to retrieve `WALL_CLOCK_BEFORE_E2E`).
- Files allowed to edit (≤ 3):
  - `.ralph/specs/90_regression-wedge-stl-swap/closure-log.md` (APPEND).
- Files explicitly out-of-bounds for this step: any.
- Expected sub-agent dispatches:
  - "Run `cargo clean -p slicer-runtime && START=$(date +%s) && cargo test -p slicer-runtime --test e2e 2>&1 | tee target/test-output.log | tail -5 && END=$(date +%s) && echo \"ELAPSED=$((END-START))\"`; return FACT with integer seconds plus the `test result` summary line" — purpose: WALL_CLOCK_AFTER_E2E.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: `BEFORE=$(grep -E '^WALL_CLOCK_BEFORE_E2E=' .ralph/specs/90_regression-wedge-stl-swap/closure-log.md | cut -d= -f2) && AFTER=$(grep -E '^WALL_CLOCK_AFTER_E2E=' .ralph/specs/90_regression-wedge-stl-swap/closure-log.md | cut -d= -f2) && [ $((BEFORE - AFTER)) -ge 60 ]`.
- Exit condition: AC-7 satisfied; closure log contains authoring procedure (AC-8), SHA-256 (AC-N2), assertion-diff (AC-N1), and wall-clock numbers (AC-7).

### Step 7: Final acceptance ceremony + `docs/07_implementation_status.md` backfill

- Task IDs:
  - `TASK-240`
- Objective: workspace gate + ledger reconciliation. Run the full pipe-suffixed acceptance command set; if all pass, backfill a `TASK-240` row into `docs/07_implementation_status.md` reflecting the closed packet.
- Precondition: Steps 0-6 green.
- Postcondition: `cargo check --workspace --all-targets` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; e2e bucket green; integration bucket green; `docs/07_implementation_status.md` contains a row referencing `TASK-240` with status `implemented` and a link to `.ralph/specs/90_regression-wedge-stl-swap/`.
- Files allowed to read:
  - `docs/07_implementation_status.md` (range-read the relevant section the new row joins — dispatched).
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` (one row APPEND, delegated to a sub-agent per `CLAUDE.md`).
- Files explicitly out-of-bounds for this step: any code source.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test e2e 2>&1 | tee target/test-output.log`; return FACT pass/fail with overall count".
  - "Run `cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test executor 2>&1 | tee target/test-output.log`; return FACT with the failing-test names list. **Executor RED carve-out**: pass if and only if the failing test set is exactly the 12 baseline `cube_4color_paint_tdd::*` and `cube_fuzzy_painted_tdd::*` tests captured in closure-log Step 0 Notes. Any new failure or any of the 12 unexpectedly turning green is a fail (the latter means a paint-pipeline packet landed concurrently and this packet's baseline assumption is stale)."
  - "Open `docs/07_implementation_status.md`. Identify the section where `TASK-23x`/`TASK-24x` rows live (paint-pipeline roadmap section). Append a `TASK-240` row with status `implemented`, link `.ralph/specs/90_regression-wedge-stl-swap/`, one-line description matching the packet goal. Return DONE + the inserted line. If the maintainer prefers to omit the row (no ledger row was present for the paint-pipeline roadmap tasks at refinement time), return `WAIVED` plus the rationale and the user can resolve at closure-log review."
- Context cost: `S` (dispatch-only).
- Authoritative docs: `docs/07_implementation_status.md`.
- OrcaSlicer refs: none.
- Verification: all four cargo dispatches return PASS; the ledger dispatch returns DONE (or an explicit WAIVED with rationale captured in the closure log).
- Exit condition: AC-1 through AC-8 and AC-N1, AC-N2, AC-N3 all satisfied; `cargo check`, `cargo clippy`, e2e bucket, integration bucket green; ledger reconciled; packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Baseline capture (PRE_ASSERT_COUNT, WALL_CLOCK_BEFORE, SHA). |
| Step 1 | M | Wedge authoring + verification. |
| Step 2 | S | Pure dispatch — test inventory. |
| Step 3 | M | 42-test file rewrite. |
| Step 4 | S | 5 reference-site swaps (incl. modules/ site). |
| Step 5 | S | Delete + sweep. |
| Step 6 | S | Wall-clock measurement + closure-log finalization. |
| Step 7 | S | Workspace gate + docs/07 backfill. |

Aggregate: M (no L step).

## Packet Completion Gate

- All 8 steps (Step 0 through Step 7) complete; each exit condition satisfied.
- AC-1, AC-1b, AC-2 through AC-8 + AC-N1, AC-N2, AC-N3 verified.
- `.ralph/specs/90_regression-wedge-stl-swap/closure-log.md` contains all of: `PRE_ASSERT_COUNT`, `WEDGE_SHA256`, `WALL_CLOCK_BEFORE`, `WALL_CLOCK_AFTER`, `BENCHY_SHA256_BEFORE`, wedge authoring procedure, per-test assertion-diff (AC-N1 manual audit).
- `docs/07_implementation_status.md` either contains a `TASK-240` row marked `implemented` (with link to this packet), or the closure log records an explicit WAIVED rationale per CLAUDE.md §Completion Rules.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`; confirm each PASS.
- Confirm `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, e2e bucket, integration bucket all green via sub-agent FACT.
- Record wall-clock delta in the closure log (AC-7).
- Record wedge SHA-256 in the closure log (AC-8).
- Confirm peak context usage stayed under 70%.

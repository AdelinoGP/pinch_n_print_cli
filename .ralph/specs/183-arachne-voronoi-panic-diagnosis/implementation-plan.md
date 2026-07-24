# Implementation Plan: 183-arachne-voronoi-panic-diagnosis

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Capture the pre-change baseline

- Task IDs: `TASK-296`
- Objective: Record the `perimeter_parity` workload's pass/fail status and its count of raw `is_finite()` assertion panic lines on the **unmodified** tree. This is the only moment this data can be obtained — the Step 2 guard converts those panics into errors.
- Precondition: working tree clean of this packet's changes; `crates/slicer-core/src/voronoi.rs` still has no `catch_unwind`.
- Postcondition: baseline pass/fail status and raw-panic count are written into `FINDINGS.md` under a `## Baseline` heading. No source edited.
- Files allowed to read, with ranges when over 300 lines:
  - None directly — the baseline comes from the delegated run below.
- Files allowed to edit (at most 3):
  - `.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md` (created here, `## Baseline` section only)
- Files explicitly out of bounds:
  - All production source — this step must not perturb the tree it is measuring.
- Blast-radius discipline: not applicable — no struct field or constant change.
- Expected sub-agent dispatches:
  - Question: run the workload and report only the final `test result:` line and the count of lines matching `is_finite`; scope: `cargo test -p slicer-runtime --test integration -- perimeter_parity`; return: `FACT` (<=5 lines)
- Context cost: `S`
- Authoritative docs:
  - None required for this step.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'rg -q "## Baseline" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && echo PASS || echo FAIL'` — FACT PASS/FAIL.
- Exit condition: `FINDINGS.md` `## Baseline` records both the suite status and an explicit raw-panic count (including `0` if none reproduce).

### Step 2: Add the catch_unwind guard and the distinct error variant

- Task IDs: `TASK-296`
- Objective: Wrap the boostvoronoi `Builder::build()` call in `voronoi_from_segments` in `std::panic::catch_unwind(AssertUnwindSafe(...))`, copying the guard shape already used by `medial_axis.rs` and `algos/paint_segmentation/voronoi_graph.rs`, and map a caught panic to a new distinct `VoronoiError` variant. On the catch branch only, capture the segment count, coordinate bounds (internal units), and duplicate/zero-length/near-collinear classification.
- Precondition: Step 1's baseline is recorded.
- Postcondition: AC-1 passes; a builder panic surfaces as `Err(VoronoiError::<variant>)` instead of unwinding; the success path is unchanged and pays no new cost.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/voronoi.rs` — the `voronoi_from_segments` body and the `VoronoiError` enum, located by `rg`.
  - `crates/slicer-core/src/medial_axis.rs` — the `catch_unwind` block only, via the dispatch below.
  - `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` (1715 lines) — the single `voronoi_from_segments(&segments)?` call site inside `from_polygons` only — confirm the new `Err` propagates cleanly.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/src/voronoi.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/arachne/preprocess.rs` (successor packet's surface — no pre-snapping here)
  - `crates/slicer-core/src/medial_axis.rs`, `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` (pattern references, already correct)
- Blast-radius discipline (mandatory — this step adds an enum variant): adding a `VoronoiError` variant ripples to every exhaustive `match` on `VoronoiError`. Dispatch a `LOCATIONS` worker for those match sites before editing and add any non-exhaustive ones to this step's edit list; do not let a follow-up `cargo check` discover them.
- Expected sub-agent dispatches:
  - Question: quote the `catch_unwind` guard block from the two already-guarded call sites; scope: `crates/slicer-core/src/medial_axis.rs`, `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`; return: `SNIPPETS` (<=2 x 30 lines)
  - Question: list every site that `match`es on `VoronoiError`; scope: `crates/**`; return: `LOCATIONS` (<=20 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0023-arachne-port-strategy.md` — the caller-pre-snaps contract this guard must not silently assume.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'rg -q "catch_unwind" crates/slicer-core/src/voronoi.rs && rg -q "AssertUnwindSafe" crates/slicer-core/src/voronoi.rs && echo PASS || echo FAIL'` — FACT PASS/FAIL (AC-1).
  - `cargo check --workspace --all-targets` — FACT pass/fail; catches any non-exhaustive `VoronoiError` match.
- Exit condition: AC-1 passes, the workspace compiles, and no empty-graph-on-catch shortcut was introduced.

### Step 3: Degenerate-input regression test

- Task IDs: `TASK-296`
- Objective: Add `voronoi_from_segments_degenerate_input_returns_result_not_panic` to the existing `voronoi_stress` binary, passing a segment set containing duplicate, zero-length, and near-collinear segments (modeled on `crates/slicer-core/tests/medial_axis_degenerate_input_tdd.rs`) and asserting the call returns `Ok` or `Err(VoronoiError::…)` without unwinding the test thread.
- Precondition: Step 2's guard is in place.
- Postcondition: AC-N1 passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/medial_axis_degenerate_input_tdd.rs` — full; reuse its degenerate-input construction.
  - `crates/slicer-core/tests/voronoi_stress.rs` — full; match its existing harness style.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/voronoi_stress.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/voronoi.rs` (frozen after Step 2 — do not relax the guard to make the test pass)
  - `crates/slicer-core/Cargo.toml` — `voronoi_stress` already declares `required-features = ["host-algos"]`; no manifest change is needed and none may be made.
- Blast-radius discipline: not applicable — test-only, adds no struct field or constant.
- Expected sub-agent dispatches:
  - Question: run the new test and report only the assertion outcome; scope: `cargo test -p slicer-core --features host-algos --test voronoi_stress -- voronoi_from_segments_degenerate_input_returns_result_not_panic --exact`; return: `FACT` (<=5 lines)
- Context cost: `S`
- Authoritative docs:
  - None required for this step.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test voronoi_stress -- voronoi_from_segments_degenerate_input_returns_result_not_panic --exact 2>&1 | tail -20` — FACT pass/fail (AC-N1).
  - `cargo test -p slicer-core --features host-algos --test voronoi_stress 2>&1 | tail -15` — FACT pass/fail; no stress regression.
- Exit condition: AC-N1 passes and the whole `voronoi_stress` binary is green.

### Step 4: Measure the workload and the geometry delta

- Task IDs: `TASK-296`
- Objective: Re-run the `perimeter_parity` workload with the guard in place; record how many builder panics are now caught, the characterization of each offending segment set, and the owning layer/region ids. Compare wall-loop output on affected layers/regions against the Step 1 baseline to answer whether the panicking computation was feeding live geometry.
- Precondition: Steps 1-3 complete; the Step 1 baseline exists.
- Postcondition: AC-2 passes and the raw data for AC-3's `## Caught panic count`, `## Input characterization`, and geometry-delta findings is captured.
- Files allowed to read, with ranges when over 300 lines:
  - `target/183-parity.log` — via delegated grep only; never load the full log.
- Files allowed to edit (at most 3):
  - `.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md`
- Files explicitly out of bounds:
  - All production source — this is a measurement step.
- Blast-radius discipline: not applicable — no source change.
- Expected sub-agent dispatches:
  - Question: run the workload, then report the final `test result:` line, the count of caught-panic diagnostics, and up to 10 captured segment characterizations; scope: `cargo test -p slicer-runtime --test integration -- perimeter_parity`; return: `SUMMARY` (<=200 words)
- Context cost: `M`
- Authoritative docs:
  - None required for this step.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `mkdir -p target && cargo xtask build-guests --check && cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 | tee target/183-parity.log | rg "^test result"; rg -c 'fpv_?\.is_finite|assertion failed.*is_finite' target/183-parity.log || echo "0 raw panics"` — FACT: suite status plus raw-panic count, which must be 0 (AC-2). The `build-guests --check` prefix is mandatory — `--test integration` loads core-module WASMs, so a stale guest would fail this workload and be misattributed to the new guard.
  - `bash -c 'rg -q "## Caught panic count" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && rg -q "## Input characterization" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && echo PASS || echo FAIL'` — FACT PASS/FAIL.
- Exit condition: zero raw `is_finite()` panic lines reach stderr, the suite's pass/fail status matches the Step 1 baseline, and the caught-panic count plus input characterization are recorded — including an explicit `0` if the panics do not reproduce on this tree.

### Step 5: Write the verdict and update the deviation row

- Task IDs: `TASK-296`
- Objective: Complete `FINDINGS.md` with an explicit `## Verdict` sentence answering "does the panicking computation feed live geometry or is it discarded", then update the D-167 row in `docs/DEVIATION_LOG.md` to match — either `Closed` with the evidence summary, or `Open — narrowed` naming a successor deviation that owns the `preprocess_input_outline` hardening.
- Precondition: Step 4's measurements are recorded.
- Postcondition: AC-3 and AC-4 pass; `FINDINGS.md` and the D-167 row state the same verdict and, if applicable, the same successor id.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` — the D-167 row only, via the dispatch below.
- Files allowed to edit (at most 3):
  - `.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md`
  - `docs/DEVIATION_LOG.md` (the D-167 row only, plus a new successor row if the verdict requires one)
- Files explicitly out of bounds:
  - Any other DEVIATION_LOG row — edit only D-167 and, if needed, append one successor row.
  - `crates/**` — no source change in this step.
- Blast-radius discipline: not applicable — documentation only.
- Expected sub-agent dispatches:
  - Question: report the D-167 row's current Status cell verbatim and the highest `D-`/`DEV-` id currently present, so a successor id can be allocated without colliding; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (<=5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` — the D-167 row; the file's own rule is that a row is open unless its Status begins with `Closed`.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'rg -q "## Verdict" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && echo PASS || echo FAIL'` — FACT PASS/FAIL (AC-3).
  - `rg -q '^\|\s*D-167-BOOSTVORONOI-ROBUST-FPT-PANICS\b.*\|\s*\*{0,2}(Closed|Open — narrowed)[^|]*\|?\s*$' docs/DEVIATION_LOG.md && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-4). Copy verbatim: the alternation pipe must be bare `|` (rg's `\|` is a *literal* pipe, which makes the check unpassable), and the `[^|]*\|?\s*$` tail is what pins the match to the Status cell instead of matching "Closed" anywhere in the row. See AC-4 in `packet.spec.md` for the full rationale.
- Exit condition: the verdict is explicit, the D-167 row matches it, and any successor id was re-derived from the log at the moment of writing rather than assumed from this packet's text.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Delegated baseline run; artifact section only. |
| Step 2 | S | One function plus one enum variant; two bounded dispatches. |
| Step 3 | S | One test file; reuses an existing degenerate-input precedent. |
| Step 4 | M | Workload run plus geometry-delta comparison; summary-capped dispatch. |
| Step 5 | S | Two documentation edits. |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` are clean.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read: register `TASK-296` complete and reconcile the D-167 line.
- If the verdict is "geometry is lost", the successor deviation row exists and a follow-up packet for `preprocess_input_outline` hardening is appended to `docs/specs/deviation-backlog-remediation-plan.md`'s Packet Queue.
- No reopened/superseded packet transitions apply.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk: this packet makes a previously-unwinding failure observable; it does **not** harden the degenerate inputs. If the verdict is that geometry was being lost, that defect remains open under the successor id and must not be reported as closed by this packet.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.

# Implementation Plan: 183-arachne-voronoi-panic-diagnosis

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 0: Attribute the observed panics to a call site BEFORE scoping the guard

- Task IDs: `TASK-296`
- Objective: Establish **which boostvoronoi entry point** produces each `is_finite()` panic line in the baseline `perimeter_parity` output. The rest of this packet assumes the answer is `voronoi_from_segments`; that assumption is **unverified** and must be tested before a single line of guard is written.
- Why this cannot be skipped: the same `robust_fpt` `fpv.is_finite()` assertion fires from **three** call sites in `crates/slicer-core/`, and the printed panic line is identical in all three — `std::panic::catch_unwind` does **not** suppress the default panic hook, so an already-*caught* panic prints exactly the same text as an unguarded one. Attribution by reading the log is therefore impossible. Worse, one of the two "already guarded" sites hides its own occurrences: `slicer_core::medial_axis::medial_axis`'s catch arm returns `Err(())`, which its caller converts to `return Ok(vec![])` — a **silent empty result**, no error, no diagnostic. That site is reached from `classic-perimeters` (two `slicer_sdk::host::medial_axis` calls in `ClassicPerimeters::run_perimeters`, `modules/core-modules/classic-perimeters/src/lib.rs`, bridged by the `medial_axis` host-service impl in `crates/slicer-wasm-host/src/host.rs`); `voronoi_from_segments` is reached from arachne via `SkeletalTrapezoidationGraph::from_polygons`; `algos/paint_segmentation/voronoi_graph.rs` is the third. `perimeter_parity` exercises classic and arachne in the same run, so all three are live in the baseline.
- Precondition: working tree clean of this packet's changes.
- Postcondition: `FINDINGS.md` `## Panic attribution` names the producing call site(s) for the observed panics and the mechanism used to establish it. If the answer is **not** `voronoi_from_segments`, stop and re-scope: Step 2's guard is aimed at the wrong site and D-167's premise needs revisiting before proceeding.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/src/medial_axis.rs` — the `catch_unwind` block and its `Err(())` catch arm only, via `rg 'catch_unwind'` — to confirm the silent-degrade behaviour first-hand.
- Files allowed to edit (at most 3):
  - `.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md` (`## Panic attribution` section)
  - a **temporary, reverted-before-Step-2** per-site marker, if that is the mechanism chosen — record it in `FINDINGS.md` and confirm the tree is clean again before Step 1's baseline run.
- Files explicitly out of bounds: `crates/slicer-core/src/voronoi.rs` (Step 2's surface — no guard yet).
- Blast-radius discipline: not applicable — measurement only; any temporary instrumentation must be reverted before Step 1.
- Expected sub-agent dispatches:
  - Question: run `perimeter_parity` with `RUST_BACKTRACE=1` and report, for up to 5 `is_finite` panic lines, the innermost `slicer_core::` frame in each backtrace; scope: `cargo test -p slicer-runtime --test integration -- perimeter_parity`; return: `FACT` (<=10 lines)
- Context cost: `S`
- Authoritative docs: none required for this step.
- OrcaSlicer refs: none — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'rg -q "^## Panic attribution" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && echo PASS || echo FAIL'` — FACT PASS/FAIL (AC-0).
  - `bash -c 'git status --porcelain crates/ | rg -q . && echo "FAIL: temporary instrumentation not reverted" || echo PASS'` — FACT PASS/FAIL; the tree must be clean before Step 1 measures it.
- Exit condition: the producing call site is named with evidence, any temporary instrumentation is reverted, and the packet's scope is either confirmed or explicitly re-opened.

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
- Objective: Wrap the boostvoronoi `Builder::build()` call in `voronoi_from_segments` in `std::panic::catch_unwind(AssertUnwindSafe(...))`, copying **only** the guard shape used by `MmuGraphError::PredicatePanic` in `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` (`match { Ok(Ok(d)) => d, Ok(Err(e)) => return Err(…), Err(_) => return Err(<panic variant>) }`), and map a caught panic to a new distinct `VoronoiError` variant. **Do not copy `medial_axis.rs`'s catch arm** — it returns `Err(())` which its caller converts to `Ok(vec![])`, the silent-empty outcome this packet's Architecture Constraints explicitly bar. `medial_axis.rs` may be read for its `AssertUnwindSafe` justification comment only. On the catch branch only, capture the segment count, coordinate bounds (internal units), and duplicate/zero-length/near-collinear classification.
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
  - Question: quote the `catch_unwind` guard block and its `MmuGraphError::PredicatePanic` catch arm; scope: `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs`; return: `SNIPPETS` (<=1 x 30 lines). Do **not** also fetch `medial_axis.rs`'s catch arm as a pattern — it is the anti-pattern here.
  - Question: list every site that `match`es on `VoronoiError`; scope: `crates/**`; return: `LOCATIONS` (<=20 entries)
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0023-arachne-port-strategy.md` — the caller-pre-snaps contract this guard must not silently assume.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'rg -q "catch_unwind" crates/slicer-core/src/voronoi.rs && rg -q "AssertUnwindSafe" crates/slicer-core/src/voronoi.rs && rg -q "PredicatePanic" crates/slicer-core/src/voronoi.rs && echo PASS || echo FAIL'` — FACT PASS/FAIL (AC-1).
  - `cargo check --workspace --all-targets` — FACT pass/fail; catches any non-exhaustive `VoronoiError` match.
- Exit condition: AC-1 passes, the workspace compiles, and no empty-graph-on-catch shortcut was introduced.

### Step 3: Degenerate-input regression test

- Task IDs: `TASK-296`
- Objective: Add `voronoi_from_segments_degenerate_input_returns_result_not_panic` to the existing `voronoi_stress` binary, passing a segment set containing duplicate, zero-length, and near-collinear segments (modeled on `crates/slicer-core/tests/medial_axis_degenerate_input_tdd.rs`) and asserting the call returns `Ok` or `Err(VoronoiError::…)` without unwinding the test thread.
- Precondition: Step 2's guard is in place.
- Postcondition: AC-N1 passes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/medial_axis_degenerate_input_tdd.rs` — full; reuse its degenerate-input construction.
  - `crates/slicer-core/tests/voronoi_stress.rs` — full; match its existing harness style for the ordinary host-algos tests.
  - `crates/slicer-core/tests/voronoi_panic_regression.rs` — new explicit-feature target for the synthetic panic assertion.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/voronoi_stress.rs`
  - `crates/slicer-core/tests/voronoi_panic_regression.rs`
  - `crates/slicer-core/Cargo.toml`
- Files explicitly out of bounds:
  - `crates/slicer-core/src/voronoi.rs` (frozen after Step 2 — do not relax the guard to make the test pass)
- Blast-radius discipline: not applicable — test-only, adds no struct field or constant.
- Expected sub-agent dispatches:
  - Question: run the new test and report only the assertion outcome; scope: `bash -c 'L=target/183-voronoi-n1.log; mkdir -p target || exit $?; cargo test -p slicer-core --features host-algos --test voronoi_stress -- voronoi_from_segments_degenerate_input_returns_result_not_panic --exact 2>&1 | tee "$L" >/dev/null; cargo_status=${PIPESTATUS[0]}; if [ "$cargo_status" -eq 0 ] && rg -q "^test result: ok\." "$L"; then echo PASS; else echo FAIL; exit 1; fi'`; return: `FACT` (<=5 lines)
  - Question: run the explicit synthetic panic target with `voronoi-panic-regression` enabled, then repeat after temporarily removing the production catch arm; return: `FACT` (<=5 lines per run)
- Context cost: `S`
- Authoritative docs:
  - None required for this step.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'L=target/183-voronoi-n1.log; mkdir -p target || exit $?; cargo test -p slicer-core --features host-algos --test voronoi_stress -- voronoi_from_segments_degenerate_input_returns_result_not_panic --exact 2>&1 | tee "$L" >/dev/null; cargo_status=${PIPESTATUS[0]}; if [ "$cargo_status" -eq 0 ] && rg -q "^test result: ok\." "$L"; then echo PASS; else echo FAIL; exit 1; fi'` — FACT pass/fail (AC-N1).
  - `bash -c 'L=target/183-voronoi-stress.log; mkdir -p target || exit $?; cargo test -p slicer-core --features host-algos --test voronoi_stress 2>&1 | tee "$L" >/dev/null; cargo_status=${PIPESTATUS[0]}; if [ "$cargo_status" -eq 0 ] && rg -q "^test result: ok\." "$L"; then echo PASS; else echo FAIL; exit 1; fi'` — FACT pass/fail; no stress regression.
  - `bash -c 'L=target/183-voronoi-panic-regression.log; mkdir -p target || exit $?; cargo test -p slicer-core --features host-algos,voronoi-panic-regression --test voronoi_panic_regression 2>&1 | tee "$L" >/dev/null; cargo_status=${PIPESTATUS[0]}; if [ "$cargo_status" -eq 0 ] && rg -q "^test result: ok\." "$L"; then echo PASS; else echo FAIL; exit 1; fi'` — FACT pass/fail; explicit production-assert coverage.
- Exit condition: AC-N1 passes and the whole `voronoi_stress` binary is green.

### Step 4: Measure the workload and the geometry delta

- Task IDs: `TASK-296`
- Objective: Re-run the `perimeter_parity` workload with the guard in place; record how many builder panics are now caught, the characterization of each offending segment set, and the owning layer/region ids. If the caught count is greater than zero, compare wall-loop output on affected layers/regions against the Step 1 baseline; if it is zero, explicitly record that geometry comparison is not applicable because no affected computation was observed.
- Precondition: Steps 1-3 complete; the Step 1 baseline exists.
- Postcondition: AC-2 passes and the raw data for AC-3's `## Caught panic count`, `## Input characterization`, and geometry-delta findings is captured. A zero caught count produces an explicit no-affected-computation statement, not a fabricated geometry delta.
- Files allowed to read, with ranges when over 300 lines:
  - `target/183-parity.log` — via delegated grep only; never load the full log.
- Files allowed to edit (at most 3):
  - `.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md`
- Files explicitly out of bounds:
  - All production source — this is a measurement step.
- Blast-radius discipline: not applicable — no source change.
- Expected sub-agent dispatches:
  - Question: run the workload, then report the final `test result:` line, the count of caught-panic diagnostics, and up to 10 captured segment characterizations; when the count is zero, report that no affected computation was observed and geometry comparison is not applicable; scope: `cargo test -p slicer-runtime --test integration -- perimeter_parity`; return: `SUMMARY` (<=200 words)
- Context cost: `M`
- Authoritative docs:
  - None required for this step.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'F=.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md; L=target/183-parity.log; mkdir -p target || exit $?; cargo xtask build-guests --check || exit $?; cargo test -p slicer-runtime --test integration -- perimeter_parity 2>&1 | tee "$L" >/dev/null; cargo_status=${PIPESTATUS[0]}; if [ "$cargo_status" -eq 0 ] && rg -q "^test result: ok\." "$L" && rg -q "^## Baseline" "$F" && rg -q "^## Caught panic count" "$F" && rg -q "^## Suite status vs baseline" "$F" && rg -q "Debug-profile comparison: unchanged.*3/3.*3/3" "$F" && rg -q "Geometry comparison: not applicable.*caught panic count is 0.*no affected computation was observed" "$F"; then echo PASS; else echo "FAIL: cargo/test-result/FINDINGS checks failed"; exit 1; fi'` — FACT: the debug-profile suite status must match the Step-1 baseline, and the required `FINDINGS.md` headings plus explicit unchanged-status and no-affected-geometry evidence must exist (AC-2). The `build-guests --check` prefix is mandatory — `--test integration` loads core-module WASMs, so a stale guest would fail this workload and be misattributed to the new guard. **Do not re-add a "raw panic count must be 0" clause:** `catch_unwind` does not suppress the default panic hook, `rg 'set_hook|take_hook' crates/slicer-core/src/` returns nothing, and the process-global-hook alternative was rejected as racy under the `par_iter` in `crates/slicer-runtime/src/layer_executor.rs`. See AC-2 in `packet.spec.md`.
  - `bash -c 'rg -c "fpv_?\.is_finite|assertion failed.*is_finite" target/183-parity.log || echo "0 raw panic lines"'` — FACT: **recorded, not asserted.** Raw panic lines are expected to persist (the guard converts the unwind, not the printing); the count is a datum for `FINDINGS.md`, not a pass/fail gate.
  - `bash -c 'rg -q "## Caught panic count" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && rg -q "## Input characterization" .ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md && echo PASS || echo FAIL'` — FACT PASS/FAIL.
- Exit condition: the debug-profile suite's pass/fail status matches the Step 1 baseline; `FINDINGS.md` carries `## Baseline`, `## Caught panic count`, `## Input characterization`, and `## Suite status vs baseline` — including an explicit `0` if the panics do not reproduce on this tree and an explicit statement that geometry comparison is not applicable when no affected computation was observed. Raw stderr panic lines are recorded as a datum, not required to be zero.

### Step 5: Write the verdict and update the deviation row

- Task IDs: `TASK-296`
- Objective: Complete `FINDINGS.md` with an explicit `## Verdict` sentence answering "does the panicking computation feed live geometry or is it discarded", then update the D-167 row in `docs/DEVIATION_LOG.md` to match — either `Closed` with the evidence summary, or `Open — narrowed` naming a successor deviation that owns the `preprocess_input_outline` hardening. **Also file the `medial_axis` inconsistency row** (see below).
- **Second required row: the `medial_axis` degrade-to-empty inconsistency.** This packet asserts "a caught panic must never become a silently-successful empty result" **only** for the boostvoronoi entry points that return a typed error (`voronoi_from_segments` and `MmuGraphError::PredicatePanic`). Shipped `medial_axis.rs` contradicts it — its catch arm returns `Err(())` and its caller converts that to `return Ok(vec![])` — and this packet does not change that, because doing so would turn quiet degenerate regions into hard errors in `classic-perimeters`' gap-fill and thin-wall paths, unmeasured here. **No ADR is authored for this**; file a `DEV-###` row instead, severity Low, status Open, recording the scope of the invariant, why `medial_axis` is exempt, and the cost of a fix; owner: whichever packet next touches `medial_axis`. **Re-derive the id at the moment of filing — never carry one forward from this packet:** `rg -o '^\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1`, then take the next free number. Use the log's existing eight-column format.
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
  - Question: report the boostvoronoi panic observation row's current Status cell verbatim and the highest `D-`/`DEV-` id currently present, so a successor id can be allocated without colliding; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (<=5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` — the boostvoronoi panic observation row; the file's own rule is that a row is open unless its Status begins with `Closed`.
- OrcaSlicer refs:
  - None — this packet ports no canonical behavior.
- Verification:
  - `bash -c 'F=.ralph/specs/183-arachne-voronoi-panic-diagnosis/FINDINGS.md; if rg -q "^## Caught panic count" "$F" && rg -q "^## Input characterization" "$F" && rg -q "^## Verdict" "$F" && rg -q "Owning layer/region IDs:" "$F" && rg -q "panicking computation.*(live geometry|discarded)" "$F"; then echo PASS; else echo FAIL; exit 1; fi'` — FACT PASS/FAIL (AC-3).
  - `rg -q '^\|\s*boostvoronoi panic observation\b.*\|\s*\*{0,2}(Closed|Open — narrowed)[^|]*\|?\s*$' docs/DEVIATION_LOG.md && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-4). Copy verbatim: the alternation pipe must be bare `|` (rg's `\|` is a *literal* pipe, which makes the check unpassable), and the `[^|]*\|?\s*$` tail is what pins the match to the Status cell instead of matching "Closed" anywhere in the row. See AC-4 in `packet.spec.md` for the full rationale.
- Exit condition: the verdict is explicit, the boostvoronoi panic observation row matches it, and any successor id was re-derived from the log at the moment of writing rather than assumed from this packet's text.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Delegated backtrace/marker attribution run; artifact section only. Gates the guard's scope. |
| Step 1 | S | Delegated baseline run; artifact section only. |
| Step 2 | S | One function plus one enum variant; two bounded dispatches. |
| Step 3 | S | One test file; reuses an existing degenerate-input precedent. |
| Step 4 | M | Workload run plus geometry-delta comparison; summary-capped dispatch. |
| Step 5 | S | Two documentation edits. |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo xtask test --summary --workspace` are clean. The workspace test gate must run through `cargo xtask test` so the guest freshness check fires first; its `VERDICT` and process exit must agree.
- The now-passing `pnp_cli_rebuild_abort_is_nonzero_with_named_failure_detail` test in `xtask/src/test.rs` is recorded in `FINDINGS.md`; it proves nonzero status plus named failure detail for the synthetic controlled runner, not a real `pnp_cli` kill.
- The `support-surface-ironing --test ironing_tdd` failures remain a separately tracked external fixture blocker under `modules/core-modules/support-surface-ironing/**`; they are out of scope and must not be conflated with packet-local completion or the passing named abort-path evidence.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read: register `TASK-296` complete and reconcile the boostvoronoi panic observation line.
- If the verdict is "geometry is lost", the successor deviation row exists and a follow-up packet for `preprocess_input_outline` hardening is appended to `docs/specs/deviation-backlog-remediation-plan.md`'s Packet Queue.
- The `medial_axis` degrade-to-empty inconsistency row is filed (Low / Open, id re-derived at filing time), and no ADR was authored for it. Verify: `bash -c 'rg -q "^\|.*DEV-[0-9]{3}.*(\bLow\b.*\bOpen\b.*medial_axis|\bLow\b.*medial_axis.*\bOpen\b|\bOpen\b.*\bLow\b.*medial_axis|\bOpen\b.*medial_axis.*\bLow\b|medial_axis.*\bLow\b.*\bOpen\b|medial_axis.*\bOpen\b.*\bLow\b).*\|$" docs/DEVIATION_LOG.md && echo PASS || echo "FAIL: dedicated Low/Open medial_axis deviation row not filed"'`.
- Step 0's attribution stands: `FINDINGS.md` `## Panic attribution` names the producing call site, and the guard was scoped to it — not assumed.
- No reopened/superseded packet transitions apply.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Re-dispatch the reopened workspace Voronoi-debug allocation issue checks: the ordinary workspace test build
  does not enable `boostvoronoi/console_debug`; the explicit
  `voronoi_panic_regression` target does enable it and passes; removing the
  production catch arm makes that target fail; and
  `cargo xtask test --summary --workspace` reaches the executor without a
  process-abort failure. The current ceremony run remains red on the
  independently reproducible unrelated unit assertion recorded in
  `FINDINGS.md`; packet 183 stays active until that workspace gate is cleared
  or explicitly accepted by the maintainer.
- The now-passing named abort-path test is
  `pnp_cli_rebuild_abort_is_nonzero_with_named_failure_detail` in
  `xtask/src/test.rs` (`cargo test -p xtask`, 41/41 green). It proves the
  synthetic controlled runner returns nonzero and reports named failure
  detail; it is not a real `pnp_cli` kill.
- The `support-surface-ironing --test ironing_tdd` failures are the independent
  external fixture blocker recorded in `FINDINGS.md` and are explicitly out of
  scope under `modules/core-modules/support-surface-ironing/**`. They remain a
  separate workspace-gate concern and must not be conflated with packet-local
  completion or with the passing named abort-path evidence.
- Exercise the `pnp_cli` rebuild failure path and require `cargo xtask test` to
  return a nonzero exit code rather than treating the aborted run as a pass.
- Record remaining packet-local risk: this packet makes a previously-unwinding failure observable; it does **not** harden the degenerate inputs. If the verdict is that geometry was being lost, that defect remains open under the successor id and must not be reported as closed by this packet.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

Workspace `cargo check` and `cargo clippy` gates use `--all-targets`; targeted `cargo test --test ...` verification commands intentionally select their named test binary. Do not combine `--all-targets` with an explicit `--test` target: the workspace check/clippy gates compile all targets, while targeted test commands verify the requested binary.

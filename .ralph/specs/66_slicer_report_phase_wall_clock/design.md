# Design: 66_slicer_report_phase_wall_clock

## Controlling Code Paths

- Primary code path: `render.rs::render_phase_summary` → reads phase durations from `SliceMeta.phase_times` and per-layer sum from existing `r.layers.iter().map(|l| l.duration_ns()).sum()`.
- Timing recording path: `collector.rs::on_phase_start` → stores `now_ns()`; `collector.rs::on_phase_end` → computes elapsed, dispatches to appropriate phase field.
- Wire path: `collector.rs::finalize()` → populates `SliceMeta.phase_times` from stored per-phase wall-clock.
- Neighboring tests: `crates/slicer-host/tests/slicer_report_html_tdd.rs` — exercises the collector → render pipeline end-to-end.

## Architecture Constraints

- Packet-specific: All phase `on_phase_start`/`on_phase_end` calls are single-threaded (main thread only, per `pipeline.rs:339,357,365,368,398`). The phase wall-clock fields may use `Mutex<u64>` for consistency with the collector's existing locking pattern, even though contention is zero.
- Packet-specific: The collector already uses `now_ns()` (lines 140-141) which returns `base_instant.elapsed().as_nanos()`. The same monotonic clock is used for phase wall-clock, so all timestamps are relative to the same base.
- Packet-specific: Parallelism does not affect phase wall-clock accuracy — phase brackets encompass all rayon parallel work (`.par_iter().collect()` blocks the main thread until all workers finish).

## Code Change Surface

- Selected approach: Add `PhaseWallTimes` to the model, record phase timestamps in the collector, and render a two-column Phase Totals table.

### Exact changes

**`crates/slicer-host/src/report/model.rs`**
- Add `PhaseWallTimes` struct with `prepass_ns: u64`, `perlayer_ns: u64`, `postpass_ns: u64` (derive `Debug, Clone, Default`)
- Add `pub phase_times: PhaseWallTimes` field to `SliceMeta`

**`crates/slicer-host/src/report/collector.rs`**
- Add four fields: `prepass_wall_ns: Mutex<u64>`, `perlayer_wall_ns: Mutex<u64>`, `postpass_wall_ns: Mutex<u64>`, `phase_start_ns: Mutex<Option<u64>>`
- In `new()` / `new_with_verbose()`: initialize all to `Mutex::new(0)` / `Mutex::new(None)`
- In `on_phase_start`: `*self.phase_start_ns.lock().unwrap() = Some(self.now_ns())`
- In `on_phase_end`: compute `wall = self.now_ns() - start.take()`, store into the appropriate `*_wall_ns` field (match on `phase`, removing the underscore prefix from `_phase`)
- In `finalize()`: construct `PhaseWallTimes { prepass_ns, perlayer_ns, postpass_ns }` from the stored fields, include in `SliceMeta`

**`crates/slicer-host/src/report/render.rs`**
- In `render_phase_summary` (lines 166-195): rewrite to query both `r.slice_meta.phase_times.*_ns` (wall-clock) and the existing sum-of-durations (worker total). Update table header to `Phase | Wall (ms) | Worker total (ms) | Count`.
- PrePass row: wall = `phase_times.prepass_ns`, worker total = `r.prepass.iter().map(|s| s.duration_ns()).sum()`
- PerLayer row: wall = `phase_times.perlayer_ns`, worker total = `r.layers.iter().map(|l| l.duration_ns()).sum()` (existing)
- PostPass row: wall = `phase_times.postpass_ns`, worker total = `r.postpass.iter().map(|s| s.duration_ns()).sum()`
- Add `<div class="note">` below the table: "PerLayer is parallel (rayon). Wall time = elapsed clock; worker total = sum of per-layer durations across all threads. For sequential phases (PrePass, PostPass), the two are identical."

**`docs/16_slicer_report.md`**
- §"What the report shows" bullet "Phase Totals" — change "PerLayer (sum of per-layer wall-clock)" to describe the two-column layout

- Rejected alternative: Using `max(end_ns) - min(start_ns)` across layers as a derived PerLayer wall-clock. Rejected because it excludes setup/teardown time between the first layer start and last layer end. The explicit phase bracket captures the full wall-clock.
- Rejected alternative: Removing the sum-of-durations column and showing only wall-clock. Rejected because the aggregate thread time is useful for profiling — it shows total CPU effort, which helps diagnose load imbalance.

## Files in Scope (read + edit)

- `crates/slicer-host/src/report/model.rs` — edit: add `PhaseWallTimes` struct, add `phase_times` field to `SliceMeta`
- `crates/slicer-host/src/report/collector.rs` — edit: add phase wall-clock fields, record in start/end callbacks, populate in `finalize()`
- `crates/slicer-host/src/report/render.rs` — edit: two-column Phase Totals table, explanatory note
- `docs/16_slicer_report.md` — edit: update Phase Totals bullet
- `crates/slicer-host/tests/slicer_report_html_tdd.rs` — read-only: verify existing tests pass; optionally add assertion for new column

## Read-Only Context

- `crates/slicer-host/src/report/collector.rs` — lines 1-72 (struct definition and imports), 113-142 (`new()` and `now_ns()`), 200-229 (`finalize()`), 310-321 (`on_phase_start/end` callbacks)
- `crates/slicer-host/src/report/render.rs` — lines 37-57 (`render_html` orchestrator), 166-195 (`render_phase_summary`), 14-35 (CSS `STYLE` constant, for note class)
- `crates/slicer-host/tests/slicer_report_html_tdd.rs` — full file (177 lines, read directly — verify existing assertions still pass)
- `docs/16_slicer_report.md` — lines 36-51 (Phase Totals bullet and surrounding context)
- `crates/slicer-host/src/pipeline.rs` — lines 339, 357, 365, 368, 398 (confirm phase bracket call order — read-only, no edits)

## Out-of-Bounds Files

- `target/`, `Cargo.lock` — never load
- `OrcaSlicerDocumented/` — no OrcaSlicer parity; never load
- `crates/slicer-host/src/instrumentation.rs` — trait definition; delegate any fact-check, do not browse
- `crates/slicer-host/src/main.rs` — CLI wiring; no changes needed, do not load
- Any WASM guest source or `.wit` files — host-only change

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-host --test slicer_report_html_tdd`; return FACT (pass/fail)" — purpose: validate existing tests pass before changes (Step 0 baseline)
- "Run `cargo test -p slicer-host --test slicer_report_html_tdd` after each step; return FACT (pass/fail)" — purpose: validate no regression after each edit
- "Run `cargo check --workspace`; return FACT (pass/fail + first error if any)" — purpose: compile gate after all edits
- "Run `cargo clippy --workspace -- -D warnings`; return FACT (pass/fail + first warning if any)" — purpose: lint gate

## Data and Contract Notes

- IR or manifest contracts touched: None.
- WIT boundary considerations: None — host-only change.
- Determinism or scheduler constraints: None. Phase wall-clock is inherently non-deterministic (system load), but the recording mechanism is deterministic — same input always produces the same wall-clock measurement logic.

## Locked Assumptions and Invariants

- Phase bracket ordering is fixed: PrePass → PerLayer → PostPass, always in that order, never nested. The `on_phase_start`/`on_phase_end` calls are properly paired. The collector's existing `current_phase` atomic already enforces this; the new phase timing fields assume the same ordering.
- `now_ns()` returns monotonic values relative to `base_instant`. `Instant::elapsed()` is guaranteed monotonic on all platforms the host targets.
- The existing test (`collector_full_run_produces_well_formed_html`) does not explicitly assert on Phase Totals content. This is an observation, not a gap — the packet adds assertions or keeps the test green.

## Risks and Tradeoffs

- Risk: Phase wall-clock fields are zero when a phase is skipped (e.g., no prepass stages). The renderer displays zero — this is correct and self-documenting.
- Tradeoff: Adding two `Mutex` fields adds ~40 bytes to the collector struct and two uncontended lock operations per phase. Overhead is negligible compared to ray tracing, mesh processing, and WASM calls in the pipeline.
- Tradeoff: The PerLayer "Worker total" column is redundant with "Wall" for single-threaded runs — but this is rare in production (rayon defaults to num_cpus threads).

## Context Cost Estimate

- Aggregate (sum across all steps): `M` (4 code edits + 1 doc edit, all small files except collector.rs at 504 lines)
- Largest single step: `M` (Step 2: collector changes span 4 locations in a 504-line file)
- Highest-risk dispatch: `cargo test -p slicer-host --test slicer_report_html_tdd` — could fail if the two-column output breaks existing string-match assertions. Mitigation: the existing test does not assert on Phase Totals table content, only on section headings and data presence, so two-column change should be transparent.

## Open Questions

None.

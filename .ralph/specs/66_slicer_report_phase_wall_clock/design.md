# Design: 66_slicer_report_phase_wall_clock

## Controlling Code Paths

- Primary code path: `render.rs::render_phase_summary` → reads phase durations from `SliceMeta.phase_times` and per-layer sum from existing `r.layers.iter().map(|l| l.duration_ns()).sum()`.
- JSON data path: `render.rs::render_llm_data` → constructs a `LlmReport` struct from the `Report`, serializes with `serde_json::to_string_pretty`, wraps in `<script type="application/json" id="slicer-report-data">`.
- Timing recording path: `collector.rs::on_phase_start` → stores `now_ns()`; `collector.rs::on_phase_end` → computes elapsed, dispatches to appropriate phase field.
- Wire path: `collector.rs::finalize()` → populates `SliceMeta.phase_times` from stored per-phase wall-clock; `render_html()` → calls `render_llm_data()` after all visible sections.
- Neighboring tests: `crates/slicer-host/tests/slicer_report_html_tdd.rs` — exercises the collector → render pipeline end-to-end. Test assertions must be extended to grep for the JSON script tag and validate parseable JSON.

## Architecture Constraints

- Packet-specific: All phase `on_phase_start`/`on_phase_end` calls are single-threaded (main thread only, per `pipeline.rs:339,357,365,368,398`). The phase wall-clock fields may use `Mutex<u64>` for consistency with the collector's existing locking pattern, even though contention is zero.
- Packet-specific: The collector already uses `now_ns()` (lines 140-141) which returns `base_instant.elapsed().as_nanos()`. The same monotonic clock is used for phase wall-clock, so all timestamps are relative to the same base.
- Packet-specific: Parallelism does not affect phase wall-clock accuracy — phase brackets encompass all rayon parallel work (`.par_iter().collect()` blocks the main thread until all workers finish).
- Packet-specific: `serde` with `derive` feature and `serde_json` are already in `slicer-host`'s `Cargo.toml` dependencies. Adding `#[derive(Serialize)]` to model structs requires no new crate dependencies.
- Packet-specific: The JSON summary struct (`LlmReport`) is a render-private type defined in `render.rs`, not in `model.rs`. It curates a subset of the `Report` fields; it does not duplicate the full model.

## Code Change Surface

- Selected approach: Add `PhaseWallTimes` to the model, record phase timestamps in the collector, and render a two-column Phase Totals table.

### Exact changes

**`crates/slicer-host/src/report/model.rs`**
- Add `PhaseWallTimes` struct with `prepass_ns: u64`, `perlayer_ns: u64`, `postpass_ns: u64` (derive `Debug, Clone, Default`)
- Add `pub phase_times: PhaseWallTimes` field to `SliceMeta`
- Add `#[derive(serde::Serialize)]` to `SliceMeta`, `LayerRecord`, `ModuleRecord`, `ParallelismRecord`, and `PhaseWallTimes` (each struct that contributes to the JSON summary)

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
- Add `LlmReport` struct (derive `Serialize`) with curated summary fields: `total_wallclock_ms`, `peak_host_memory_bytes`, `layer_count`, `module_count`, `threads_observed`, `max_layers_concurrent`, `phases` (nested), `module_aggregates` (array), `per_layer_summary` (array)
- Add `render_llm_data(out: &mut String, r: &Report)` function: constructs `LlmReport` from `Report`, serializes with `serde_json::to_string_pretty`, wraps in `<script type="application/json" id="slicer-report-data">…</script>`
- In `render_html()`: call `render_llm_data(&mut out, r)` after `render_serial_edges` and before `</body></html>`
- Add `use serde::Serialize;` to imports

**`docs/16_slicer_report.md`**
- §"What the report shows" bullet "Phase Totals" — change "PerLayer (sum of per-layer wall-clock)" to describe the two-column layout
- Add a new paragraph after the "What the report shows" list documenting the JSON data block (`<script type="application/json" id="slicer-report-data">`): its purpose, top-level keys, and that it is invisible in visual rendering

- Rejected alternative: Using `max(end_ns) - min(start_ns)` across layers as a derived PerLayer wall-clock. Rejected because it excludes setup/teardown time between the first layer start and last layer end. The explicit phase bracket captures the full wall-clock.
- Rejected alternative: Removing the sum-of-durations column and showing only wall-clock. Rejected because the aggregate thread time is useful for profiling — it shows total CPU effort, which helps diagnose load imbalance.

## Files in Scope (read + edit)

- `crates/slicer-host/src/report/model.rs` — edit: add `PhaseWallTimes` struct, add `phase_times` field to `SliceMeta`, add `#[derive(Serialize)]` to structs consumed by JSON summary
- `crates/slicer-host/src/report/collector.rs` — edit: add phase wall-clock fields, record in start/end callbacks, populate in `finalize()`
- `crates/slicer-host/src/report/render.rs` — edit: two-column Phase Totals table, explanatory note, add `LlmReport` struct + `render_llm_data()` + call from `render_html()`
- `docs/16_slicer_report.md` — edit: update Phase Totals bullet, add JSON block documentation paragraph
- `crates/slicer-host/tests/slicer_report_html_tdd.rs` — edit: add assertion for `<script type="application/json" id="slicer-report-data">` tag presence and valid parseable JSON with required keys

## Read-Only Context

- `crates/slicer-host/src/report/collector.rs` — lines 1-72 (struct definition and imports), 113-142 (`new()` and `now_ns()`), 200-229 (`finalize()`), 310-321 (`on_phase_start/end` callbacks)
- `crates/slicer-host/src/report/render.rs` — lines 37-57 (`render_html` orchestrator), 166-195 (`render_phase_summary`), 14-35 (CSS `STYLE` constant, for note class), 390-457 (render_serial_edges — to place `render_llm_data` call after it)
- `crates/slicer-host/tests/slicer_report_html_tdd.rs` — full file (177 lines, read directly — verify existing assertions still pass; add JSON assertions)
- `docs/16_slicer_report.md` — lines 36-51 (Phase Totals bullet and surrounding context), full file (147 lines — identify insertion point for JSON documentation paragraph)
- `crates/slicer-host/src/pipeline.rs` — lines 339, 357, 365, 368, 398 (confirm phase bracket call order — read-only, no edits)
- `crates/slicer-host/Cargo.toml` — lines 18-19 (confirm serde + serde_json already in deps — read-only, no edits)

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
- "Run `cargo test -p slicer-host --test slicer_report_html_tdd -- --nocapture` and extract the JSON block between `<script type=\"application/json\" id=\"slicer-report-data\">` and `</script>`; parse with serde_json to verify key presence; return FACT (keys found / missing X)" — purpose: validate AC-7 JSON content

## Data and Contract Notes

- IR or manifest contracts touched: None.
- WIT boundary considerations: None — host-only change.
- Determinism or scheduler constraints: None. Phase wall-clock is inherently non-deterministic (system load), but the recording mechanism is deterministic — same input always produces the same wall-clock measurement logic.
- JSON schema: No external schema file. The `LlmReport` struct's `Serialize` derive defines the implicit schema. Field names are snake_case (Rust default). All time values are in milliseconds (f64, 3 decimal places from `fmt_ms`). All byte values are in raw bytes (u64/i64).
- Serde contract: `serde` and `serde_json` are already workspace dependencies. Adding `#[derive(Serialize)]` to model structs in `model.rs` is additive and does not affect deserialization or other serde traits.

## Locked Assumptions and Invariants

- Phase bracket ordering is fixed: PrePass → PerLayer → PostPass, always in that order, never nested. The `on_phase_start`/`on_phase_end` calls are properly paired. The collector's existing `current_phase` atomic already enforces this; the new phase timing fields assume the same ordering.
- `now_ns()` returns monotonic values relative to `base_instant`. `Instant::elapsed()` is guaranteed monotonic on all platforms the host targets.
- The existing test (`collector_full_run_produces_well_formed_html`) does not explicitly assert on Phase Totals content. This is an observation, not a gap — the packet adds assertions or keeps the test green.

## Risks and Tradeoffs

- Risk: Phase wall-clock fields are zero when a phase is skipped (e.g., no prepass stages). The renderer displays zero — this is correct and self-documenting.
- Tradeoff: Adding two `Mutex` fields adds ~40 bytes to the collector struct and two uncontended lock operations per phase. Overhead is negligible compared to ray tracing, mesh processing, and WASM calls in the pipeline.
- Tradeoff: The PerLayer "Worker total" column is redundant with "Wall" for single-threaded runs — but this is rare in production (rayon defaults to num_cpus threads).

## Context Cost Estimate

- Aggregate (sum across all steps): `M` (4 code edits + 1 doc edit + 1 JSON render step, all small files except collector.rs at 504 lines; JSON step adds ~60 lines of new code)
- Largest single step: `M` (Step 2: collector changes span 4 locations in a 504-line file; Step 3.5: render.rs JSON changes + model.rs derives + test assertions)
- Highest-risk dispatch: `cargo test -p slicer-host --test slicer_report_html_tdd` — could fail if the two-column output or JSON block breaks existing string-match assertions. Mitigation: the existing test does not assert on Phase Totals table content, only on section headings and data presence, so two-column change and JSON block addition should be transparent to existing assertions. The JSON test assertions are additive.

## Open Questions

None.

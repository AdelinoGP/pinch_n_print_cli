---
status: implemented
packet: 66_slicer_report_phase_wall_clock
task_ids: []
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 66_slicer_report_phase_wall_clock

## Goal

Fix the slicer report Phase Totals table so PerLayer displays actual phase wall-clock elapsed time alongside aggregate thread time, rather than conflating the two under a single "wall-clock, sum" label. Additionally, embed a machine-readable JSON summary block (`<script type="application/json" id="slicer-report-data">`) so LLMs can consume phase timing, per-module aggregates, per-layer summaries, memory, and thread usage without parsing the visual table markup.

## Scope Boundaries

This packet corrects the slicer report HTML renderer and underlying collector so the Phase Totals table shows two distinct columns: actual phase wall-clock (`Wall (ms)`) and aggregate across threads (`Worker total (ms)`). The collector gains per-phase start/end timestamp recording; `SliceMeta` gains a `PhaseWallTimes` struct; the renderer displays both values. Additionally, the renderer embeds a `<script type="application/json" id="slicer-report-data">` block containing a curated JSON summary of phase timing, per-module aggregates, per-layer summaries, memory, and thread usage — invisible to humans, consumable by LLMs. Both changes are host-only — no WASM, IR schema, WIT boundary, scheduler rules, or OrcaSlicer parity.

## Prerequisites and Blockers

- Depends on: None.
- Unblocks: None.
- Activation blockers: None.

## Acceptance Criteria

- **AC-1. Given** the report model at `crates/slicer-host/src/report/model.rs`, **when** inspected, **then** a `PhaseWallTimes` struct exists with `pub prepass_ns: u64`, `pub perlayer_ns: u64`, `pub postpass_ns: u64` fields, and `SliceMeta` contains `pub phase_times: PhaseWallTimes`. | `rg -q 'struct PhaseWallTimes' crates/slicer-host/src/report/model.rs && rg -q 'prepass_ns.*u64' crates/slicer-host/src/report/model.rs && rg -q 'perlayer_ns.*u64' crates/slicer-host/src/report/model.rs && rg -q 'postpass_ns.*u64' crates/slicer-host/src/report/model.rs && rg -q 'phase_times.*PhaseWallTimes' crates/slicer-host/src/report/model.rs && echo PASS || echo FAIL`
- **AC-2. Given** the collector at `collector.rs`, **when** `finalize()` constructs `SliceMeta`, **then** `phase_times` is populated with the wall-clock elapsed for each phase (PrePass, PerLayer, PostPass) measured from `on_phase_start` to `on_phase_end`. | `rg -q 'phase_times' crates/slicer-host/src/report/collector.rs && rg -q 'PhaseWallTimes' crates/slicer-host/src/report/collector.rs && echo PASS || echo FAIL`
- **AC-3. Given** a slicer report HTML rendered by `render_phase_summary`, **when** inspecting the Phase Totals `<table>`, **then** the header row contains `<th>Wall (ms)</th>` and `<th>Worker total (ms)</th>` as distinct columns. | `rg -q 'Wall \(ms\)' crates/slicer-host/src/report/render.rs && rg -q 'Worker total \(ms\)' crates/slicer-host/src/report/render.rs && echo PASS || echo FAIL`
- **AC-4. Given** the Phase Totals table, **when** inspecting the PerLayer row, **then** the Wall column shows `r.slice_meta.phase_times.perlayer_ns` and the Worker total column shows the sum of per-layer durations (existing `perlayer_ns` computation). Both values are non-zero for a non-empty run. | `cargo test -p slicer-host --test slicer_report_html_tdd -- collector_full_run_produces_well_formed_html && echo PASS || echo FAIL`
- **AC-5. Given** `docs/16_slicer_report.md`, **when** inspecting the §"What the report shows" Phase Totals bullet, **then** it describes two columns ("wall-clock" and "aggregate thread time") for the PerLayer row and the note below advises "Worker total exceeds wall for PerLayer when layers run in parallel." | `rg -q 'Worker total' docs/16_slicer_report.md && rg -q 'aggregate thread' docs/16_slicer_report.md && echo PASS || echo FAIL`
- **AC-6. Given** a slicer report HTML rendered by `render_html`, **when** inspecting the output string, **then** it contains `<script type="application/json" id="slicer-report-data">` followed by valid JSON (parseable by `serde_json::from_str`) and a closing `</script>` tag. | `cargo test -p slicer-host --test slicer_report_html_tdd -- collector_full_run_produces_well_formed_html --nocapture 2>&1 | Select-String -Pattern 'script type=.application.json. id=.slicer-report-data.' && echo PASS || echo FAIL`
- **AC-7. Given** the JSON block content, **when** parsed, **then** the root object contains keys `total_wallclock_ms` (number), `peak_host_memory_bytes` (number), `layer_count` (number), `module_count` (number), `threads_observed` (number), `max_layers_concurrent` (number), `phases` (object with `prepass`, `perlayer`, `postpass` each having `wall_ms` and `worker_total_ms`), `module_aggregates` (array of objects with `module_id`, `calls`, `total_ms`, `mean_ms`, `p95_ms`, `peak_host_delta_bytes`, `wasm_peak_bytes`), and `per_layer_summary` (array of objects with `layer_index`, `z_mm`, `duration_ms`, `worker`, `stages`, `modules`, `host_delta_bytes`, `host_peak_bytes`). | `cargo test -p slicer-host --test slicer_report_html_tdd -- collector_full_run_produces_well_formed_html && echo PASS || echo FAIL`
- **AC-8. Given** a slicer report with zero layers (empty run via `collector_no_phases_produces_empty_but_valid_html`), **when** inspecting the HTML output, **then** the `<script type="application/json" id="slicer-report-data">` block still exists with `layer_count: 0`, `threads_observed: []`, empty `per_layer_summary` array, and empty `module_aggregates` array. | `cargo test -p slicer-host --test slicer_report_html_tdd -- collector_no_phases_produces_empty_but_valid_html --nocapture 2>&1 | Select-String -Pattern 'slicer-report-data' && echo PASS || echo FAIL`

## Negative Test Cases

- **AC-N2. Given** the rendered HTML, **when** scanning all visible `<div>`, `<table>`, `<h2>`, `<p>`, `<span>`, `<td>`, and `<th>` elements (excluding `<script>`), **then** no JSON key names (`total_wallclock_ms`, `module_aggregates`, `per_layer_summary`, `phases`) appear as visible text content. The JSON is confined to the `<script>` tag. | `cargo test -p slicer-host --test slicer_report_html_tdd -- collector_full_run_produces_well_formed_html && echo PASS || echo FAIL`

## Verification

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-host --test slicer_report_html_tdd`

## Authoritative Docs

- `docs/16_slicer_report.md` (147 lines — read directly)
- `crates/slicer-host/src/report/model.rs` (161 lines — read directly)
- `crates/slicer-host/src/report/collector.rs` (504 lines — read lines 1-72 for struct fields, 113-142 for `new()`, 200-229 for `finalize()`, 310-321 for phase callbacks, 140-141 for `now_ns()`)
- `crates/slicer-host/src/report/render.rs` (457 lines — read lines 166-195 for `render_phase_summary`, 37-57 for `render_html` orchestrator)
- `crates/slicer-host/tests/slicer_report_html_tdd.rs` (177 lines — read directly)

## Doc Impact Statement

- `docs/16_slicer_report.md` §"What the report shows" bullet "Phase Totals" — update description from "PerLayer (sum of per-layer wall-clock)" to describe two columns: wall-clock and worker total. Add a `.note` in the render explaining the distinction. | `rg -q 'Worker total' docs/16_slicer_report.md`
- `docs/16_slicer_report.md` — add a new paragraph documenting the `<script type="application/json" id="slicer-report-data">` block: its purpose (LLM-readable structured data), the keys it contains, and that it is invisible in visual rendering. | `rg -q 'slicer-report-data' docs/16_slicer_report.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- [design.md §model.rs Serialize] — Specified: `#[derive(serde::Serialize)]` on `SliceMeta`, `LayerRecord`, `ModuleRecord`, `ParallelismRecord`, `PhaseWallTimes` | Implemented: only `PhaseWallTimes` derives Serialize; all other structs use a separate `LlmReport` hierarchy in `render.rs` | Reason: avoids serde coupling on the data model and avoids requiring Serialize on transitive types (`TierKind`, `SerialEdge`). The `LlmReport` approach was already described in design.md and is architecturally superior.

- [AC-7 `threads_observed`] — Specified: `threads_observed` is `(number)` in AC-7 but `[]` (empty array) in AC-8 | Implemented: array of thread name strings matching `ParallelismRecord.threads_observed: Vec<String>` | Reason: spec is internally inconsistent. Implemented per AC-8 array form; thread names are more useful for analysis than a bare count.

- [AC-N2 `phases` exclusion] — Specified: `phases` must not appear in visible HTML | Implemented: `phases` excluded from the leak check | Reason: the word "phases" appears in the `.note` div's prose ("For sequential phases…"), unavoidable false positive. Spec should not list terms that occur in the report's natural language.

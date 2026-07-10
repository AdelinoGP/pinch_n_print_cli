---
status: implemented
packet: 66_slicer_report_phase_wall_clock
task_ids: []
---

# 66_slicer_report_phase_wall_clock

## Goal

Fix the slicer report Phase Totals table so PerLayer displays actual phase wall-clock elapsed time alongside aggregate thread time, rather than conflating the two under a single "wall-clock, sum" label. Additionally, embed a machine-readable JSON summary block (`<script type="application/json" id="slicer-report-data">`) so LLMs can consume phase timing, per-module aggregates, per-layer summaries, memory, and thread usage without parsing the visual table markup.

## Problem Statement

The slicer report's Phase Totals table displays a single "Total (ms)" column. For PrePass and PostPass (sequential, single-threaded), this column shows wall-clock time. For PerLayer (parallel via rayon), the column sums per-layer wall-clock durations — which is **aggregate thread time across all cores**, not wall-clock. The label "PerLayer (wall-clock, sum)" acknowledges the sum but calls it "wall-clock," an oxymoron when layers overlap in time.

A user reading the report sees the header `total: 5000 ms` (wall-clock) next to `PerLayer: 40000 ms` (thread time) with no clear distinction between the two quantities. This misleads performance analysis: a naive reader assumes PerLayer wall-clock is 40 s when it is actually 5 s.

The fix records actual phase start/end wall-clock timestamps in the collector, passes them through the model, and renders a two-column Phase Totals table: wall-clock vs. aggregate worker total.

Separately, the HTML report currently has no machine-readable data channel. LLMs and automated analysis tools parsing the report must scrape visual table markup, which is fragile and loses structure (column meanings, units, relationships). The fix embeds a structured JSON summary block (`<script type="application/json" id="slicer-report-data">`) that carries phase timing, per-module aggregates, per-layer summaries, memory, and thread counts in a parseable format invisible to human viewers.

## Architecture Constraints

- Packet-specific: All phase `on_phase_start`/`on_phase_end` calls are single-threaded (main thread only, per `pipeline.rs:339,357,365,368,398`). The phase wall-clock fields may use `Mutex<u64>` for consistency with the collector's existing locking pattern, even though contention is zero.
- Packet-specific: The collector already uses `now_ns()` (lines 140-141) which returns `base_instant.elapsed().as_nanos()`. The same monotonic clock is used for phase wall-clock, so all timestamps are relative to the same base.
- Packet-specific: Parallelism does not affect phase wall-clock accuracy — phase brackets encompass all rayon parallel work (`.par_iter().collect()` blocks the main thread until all workers finish).
- Packet-specific: `serde` with `derive` feature and `serde_json` are already in `slicer-host`'s `Cargo.toml` dependencies. Adding `#[derive(Serialize)]` to model structs requires no new crate dependencies.
- Packet-specific: The JSON summary struct (`LlmReport`) is a render-private type defined in `render.rs`, not in `model.rs`. It curates a subset of the `Report` fields; it does not duplicate the full model.

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

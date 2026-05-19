# Requirements: 66_slicer_report_phase_wall_clock

## Packet Metadata

- Grouped task IDs: none (correction to existing slicer report; no backlog task)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The slicer report's Phase Totals table displays a single "Total (ms)" column. For PrePass and PostPass (sequential, single-threaded), this column shows wall-clock time. For PerLayer (parallel via rayon), the column sums per-layer wall-clock durations — which is **aggregate thread time across all cores**, not wall-clock. The label "PerLayer (wall-clock, sum)" acknowledges the sum but calls it "wall-clock," an oxymoron when layers overlap in time.

A user reading the report sees the header `total: 5000 ms` (wall-clock) next to `PerLayer: 40000 ms` (thread time) with no clear distinction between the two quantities. This misleads performance analysis: a naive reader assumes PerLayer wall-clock is 40 s when it is actually 5 s.

The fix records actual phase start/end wall-clock timestamps in the collector, passes them through the model, and renders a two-column Phase Totals table: wall-clock vs. aggregate worker total.

## In Scope

- Add `PhaseWallTimes` struct (`prepass_ns`, `perlayer_ns`, `postpass_ns`) to `model.rs` and a `phase_times` field on `SliceMeta`
- In `collector.rs`: record `now_ns()` timestamp in `on_phase_start`, compute elapsed in `on_phase_end`, store per-phase wall-clock
- In `collector.rs::finalize()`: populate `SliceMeta.phase_times` from recorded phase timings
- In `render.rs::render_phase_summary`: add "Wall (ms)" and "Worker total (ms)" columns to the Phase Totals table; show phase wall-clock and aggregate-thread sums
- In `render.rs`: add a `.note` div below the Phase Totals table explaining the distinction for PerLayer
- Update `docs/16_slicer_report.md` §"What the report shows" Phase Totals bullet
- Verify existing `slicer_report_html_tdd.rs` tests pass without regression
- The "Per-Module Aggregate" and "Per-Stage Aggregate" tables retain their existing `Total (ms)` columns (sum of durations) — these are explicitly aggregates, not wall-clock

## Out of Scope

- Adding phase wall-clock to the Per-Module Aggregate or Per-Stage Aggregate tables (those are explicitly per-call aggregates already)
- Adding CPU-time / thread-count metrics to the header
- Modifying the parallelism Gantt chart
- Changing the `LayerRecord` or `StageRecord` timing model (already correct — individual brackets capture wall-clock relative to `base_instant`)
- Adding per-phase memory-peak tracking
- Any WASM guest changes, WIT boundary changes, IR schema changes, or config manifest changes
- OrcaSlicer parity (no OrcaSlicer equivalent for this debugging report)
- Adding dedicated benchmark targets for the report collector
- Changing the phase bracket granularity in `pipeline.rs` (already correct — phase brackets exist at `pipeline.rs:339,357-365,368-398`)

## Authoritative Docs

- `docs/16_slicer_report.md` (147 lines — read directly)
- `crates/slicer-host/src/report/model.rs` (161 lines — read directly)
- `crates/slicer-host/src/report/collector.rs` (504 lines — range-read; see `design.md` §Read-Only Context for line hints)
- `crates/slicer-host/src/report/render.rs` (457 lines — range-read)
- `crates/slicer-host/tests/slicer_report_html_tdd.rs` (177 lines — read directly)

## Acceptance Summary

- Positive cases: `AC-1` through `AC-5` from `packet.spec.md`. `AC-1` confirms the `PhaseWallTimes` struct shape. `AC-2` confirms collector wiring. `AC-3` and `AC-4` confirm renderer output structure and correctness. `AC-5` confirms doc update.
- Negative cases: None. This packet does not change validation, enforcement, contract boundaries, or error-handling behavior — it adds a new display column to existing report infrastructure.
- Cross-packet impact: None.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace` | Compile gate — catches type errors across crate boundaries | FACT pass/fail |
| `cargo clippy --workspace -- -D warnings` | Lint gate | FACT pass/fail |
| `cargo test -p slicer-host --test slicer_report_html_tdd` | Existing report TDD tests pass without regression | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `rg -q 'struct PhaseWallTimes' crates/slicer-host/src/report/model.rs` | AC-1: struct exists | FACT pass/fail |
| `rg -q 'phase_times' crates/slicer-host/src/report/collector.rs` | AC-2: collector populates field | FACT pass/fail |
| `rg -q 'Wall \(ms\)' crates/slicer-host/src/report/render.rs` | AC-3: column header in renderer | FACT pass/fail |
| `cargo test -p slicer-host --test slicer_report_html_tdd -- collector_full_run_produces_well_formed_html` | AC-4: full report renders correctly | FACT pass/fail |
| `rg -q 'Worker total' docs/16_slicer_report.md` | AC-5: doc updated | FACT pass/fail |

## Step Completion Expectations

- Cross-step invariant: No step may regress existing `slicer_report_html_tdd` tests. After each step, run `cargo test -p slicer-host --test slicer_report_html_tdd` to confirm.
- Step ordering rationale: Model changes (Step 1) must precede collector changes (Step 2) because the collector references `PhaseWallTimes`. Collector changes must precede renderer changes (Step 3) because the renderer reads `SliceMeta.phase_times`. Doc changes (Step 4) are independent but are best done last to reflect the final output.
- No shared scratch state across steps.

## Context Discipline Notes

- `collector.rs` is 504 lines — the implementer must range-read, not load in full. See `design.md` §Read-Only Context for line-range hints.
- `render.rs` is 457 lines — range-read only, focusing on `render_phase_summary` (lines 166-195).
- The implementer may be tempted to read `pipeline.rs` to understand phase bracket timing. Resist — the phase brackets are already verified correct by the existing pipeline; this packet only consumes them.
- Temptation to read `docs/01_system_architecture.md` or `docs/04_host_scheduler.md` — unnecessary; the report infra is self-contained under `src/report/`.

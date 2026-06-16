# Implementation Plan: 66_slicer_report_phase_wall_clock

## Execution Rules

- One atomic step at a time.
- Each step honors the context-discipline preamble. The fields below are the budget contract for this step.
- TDD first: run existing tests before each edit to establish baseline; run after each edit to confirm no regression.

## Steps

### Step 0: Baseline

- Task IDs: none
- Objective: Confirm existing `slicer_report_html_tdd` tests pass before any changes.
- Precondition: Working tree clean (no uncommitted changes to report files).
- Postcondition: All existing tests pass; any pre-existing failures are noted.
- Files allowed to read: none (pure dispatch)
- Files allowed to edit: none
- Files explicitly out-of-bounds for this step: all
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test slicer_report_html_tdd`; return FACT (pass/fail + failing test name if any)"
- Context cost: `S`
- Verification:
  - `cargo test -p slicer-host --test slicer_report_html_tdd` — dispatch as FACT
- Exit condition: Existing tests pass (or pre-existing failures are documented and unrelated).

### Step 1: Add PhaseWallTimes to model.rs

- Task IDs: none
- Objective: Add `PhaseWallTimes` struct and `phase_times` field to `SliceMeta`.
- Precondition: Step 0 baseline passes.
- Postcondition: `PhaseWallTimes` struct compiles; `SliceMeta` has `phase_times` field with `Default` value.
- Files allowed to read:
  - `crates/slicer-host/src/report/model.rs` — full file (161 lines)
- Files allowed to edit:
  - `crates/slicer-host/src/report/model.rs`
- Files explicitly out-of-bounds: none for this step
- Expected sub-agent dispatches:
  - "Run `cargo check -p slicer-host`; return FACT (pass/fail + first error if any)" — purpose: verify struct compiles
- Context cost: `S`
- Authoritative docs: none
- Verification:
  - `cargo check -p slicer-host` — dispatch as FACT
  - `rg -q 'struct PhaseWallTimes' crates/slicer-host/src/report/model.rs && rg -q 'prepass_ns.*u64' crates/slicer-host/src/report/model.rs && rg -q 'phase_times.*PhaseWallTimes' crates/slicer-host/src/report/model.rs && echo PASS || echo FAIL`
- Exit condition: `PhaseWallTimes` struct with three `u64` fields exists and compiles.

### Step 2: Record phase wall-clock in collector.rs

- Task IDs: none
- Objective: Record `now_ns()` at `on_phase_start`, compute elapsed at `on_phase_end`, store per-phase wall-clock, and populate `SliceMeta.phase_times` in `finalize()`.
- Precondition: Step 1 complete (`PhaseWallTimes` exists).
- Postcondition: `on_phase_start` stores a timestamp; `on_phase_end` computes elapsed; `finalize()` populates `phase_times` with non-zero values for each phase bracket that fired.
- Files allowed to read:
  - `crates/slicer-host/src/report/collector.rs` — lines 1-72 (struct + types), 113-142 (`new` + `now_ns`), 200-229 (`finalize`), 310-321 (phase callbacks)
- Files allowed to edit:
  - `crates/slicer-host/src/report/collector.rs`
- Files explicitly out-of-bounds: none for this step
- Expected sub-agent dispatches:
  - "Run `cargo check -p slicer-host`; return FACT (pass/fail + first error if any)" — purpose: verify collector compiles
  - "Run `cargo test -p slicer-host --test slicer_report_html_tdd`; return FACT (pass/fail)" — purpose: verify no regression
- Context cost: `M` (504-line file, but only range-read 4 sections)
- Authoritative docs: none (existing collector code is self-documenting)
- Verification:
  - `cargo check -p slicer-host`
  - `cargo test -p slicer-host --test slicer_report_html_tdd`
  - `rg -q 'phase_times' crates/slicer-host/src/report/collector.rs && rg -q 'PhaseWallTimes' crates/slicer-host/src/report/collector.rs && echo PASS || echo FAIL`
- Exit condition: Collector records phase wall-clock; `finalize()` populates `SliceMeta.phase_times`; existing tests pass.

### Step 3: Render two-column Phase Totals table

- Task IDs: none
- Objective: Update `render_phase_summary` to display "Wall (ms)" and "Worker total (ms)" columns, reading wall values from `r.slice_meta.phase_times.*_ns` and worker totals from existing sum-of-durations. Add explanatory `.note` div.
- Precondition: Step 2 complete (slice_meta.phase_times is populated).
- Postcondition: Phase Totals HTML table has two time columns; PerLayer row shows distinct wall and worker-total values; note explains the distinction.
- Files allowed to read:
  - `crates/slicer-host/src/report/render.rs` — lines 37-57 (render_html), 166-195 (render_phase_summary), 14-35 (STYLE for note class)
- Files allowed to edit:
  - `crates/slicer-host/src/report/render.rs`
- Files explicitly out-of-bounds: none for this step
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test slicer_report_html_tdd -- collector_full_run_produces_well_formed_html --nocapture`; capture stdout; search for 'Wall (ms)' and 'Worker total (ms)' in stdout; return FACT (both found / missing X)" — purpose: verify columns appear in rendered HTML
  - "Run `cargo test -p slicer-host --test slicer_report_html_tdd`; return FACT (pass/fail)" — purpose: verify no regression
- Context cost: `M` (renderer is 457 lines; range-read 3 sections)
- Authoritative docs: none
- Verification:
  - `rg -q 'Wall \(ms\)' crates/slicer-host/src/report/render.rs && rg -q 'Worker total \(ms\)' crates/slicer-host/src/report/render.rs && echo PASS || echo FAIL`
  - `cargo test -p slicer-host --test slicer_report_html_tdd`
- Exit condition: Two-column table renders correctly; existing tests pass; rendered HTML contains both column headers.

### Step 3.5: Embed LLM-readable JSON block

- Task IDs: none
- Objective: Add `#[derive(Serialize)]` to model structs consumed by the JSON summary. Add `LlmReport` struct and `render_llm_data()` in `render.rs`. Embed `<script type="application/json" id="slicer-report-data">` in `render_html()`. Add JSON assertions to the TDD test.
- Precondition: Step 3 complete (two-column Phase Totals renders correctly).
- Postcondition: `render_html()` output contains a `<script type="application/json" id="slicer-report-data">` block with valid JSON. The JSON block exists for both populated and empty reports. Test assertions confirm tag presence and parseable JSON with required keys.
- Files allowed to read:
  - `crates/slicer-host/src/report/model.rs` — full file (161 lines) to identify structs needing `#[derive(Serialize)]`: `SliceMeta`, `LayerRecord`, `ModuleRecord`, `ParallelismRecord`, `PhaseWallTimes`
  - `crates/slicer-host/src/report/render.rs` — lines 37-57 (render_html), 390-457 (render_serial_edges, to know where to place `render_llm_data` call), 1-7 (imports area)
  - `crates/slicer-host/Cargo.toml` — lines 18-19 (confirm serde/serde_json deps — read-only)
  - `crates/slicer-host/tests/slicer_report_html_tdd.rs` — full file (177 lines, for assertion placement)
- Files allowed to edit:
  - `crates/slicer-host/src/report/model.rs` — add `#[derive(Serialize)]` to `SliceMeta`, `LayerRecord`, `ModuleRecord`, `ParallelismRecord`, `MemDelta`, and `PhaseWallTimes`; add `use serde::Serialize;`
  - `crates/slicer-host/src/report/render.rs` — add `LlmReport` struct (derive `Serialize`) and `render_llm_data()` function; add `use serde::Serialize;`; call from `render_html()`
  - `crates/slicer-host/tests/slicer_report_html_tdd.rs` — add assertion: `<script type="application/json" id="slicer-report-data">` tag exists; extract and parse JSON; verify required keys (`total_wallclock_ms`, `peak_host_memory_bytes`, `layer_count`, `module_count`, `threads_observed`, `phases`, `module_aggregates`, `per_layer_summary`)
- Files explicitly out-of-bounds for this step: none
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test slicer_report_html_tdd`; return FACT (pass/fail + failing test name if any)" — purpose: verify JSON assertions pass
  - "Run `cargo test -p slicer-host --test slicer_report_html_tdd -- --nocapture`; extract the `<script type=\"application/json\" id=\"slicer-report-data\">...</script>` block; search for keys `total_wallclock_ms`, `phases`, `module_aggregates`, `per_layer_summary`; return FACT (all keys found / missing X)" — purpose: AC-7 key validation
- Context cost: `M` (adds derives to model.rs + ~60 lines of new code in render.rs + test assertions + serde import; spans 3 files)
- Authoritative docs: none (self-documenting code)
- Verification:
  - `cargo check -p slicer-host`
  - `cargo test -p slicer-host --test slicer_report_html_tdd`
  - `rg -q 'slicer-report-data' crates/slicer-host/src/report/render.rs && rg -q 'render_llm_data' crates/slicer-host/src/report/render.rs && echo PASS || echo FAIL`
  - `rg -q 'Serialize' crates/slicer-host/src/report/model.rs && echo PASS || echo FAIL`
- Exit condition: JSON block exists in rendered HTML; test assertions pass (tag found, JSON parseable, all required keys present); empty-report test also shows JSON block.

### Step 4: Update docs/16_slicer_report.md

- Task IDs: none
- Objective: Update the Phase Totals bullet in `docs/16_slicer_report.md` to describe the two-column layout. Add a new paragraph documenting the JSON data block.
- Precondition: Step 3 complete (rendered output has two columns). Step 3.5 complete (JSON block renders correctly).
- Postcondition: Doc describes "wall-clock" and "worker total (aggregate thread time)" columns for Phase Totals. A new paragraph documents the `<script type="application/json" id="slicer-report-data">` block, its purpose, and key structure.
- Files allowed to read:
  - `docs/16_slicer_report.md` — lines 36-51 (§"What the report shows" Phase Totals bullet), full file (147 lines — identify insertion point for JSON paragraph after the "What the report shows" list)
- Files allowed to edit:
  - `docs/16_slicer_report.md`
- Files explicitly out-of-bounds: none for this step
- Expected sub-agent dispatches: none (direct edit)
- Context cost: `S`
- Authoritative docs: same file being edited
- Verification:
  - `rg -q 'Worker total' docs/16_slicer_report.md && rg -q 'aggregate thread' docs/16_slicer_report.md && echo PASS || echo FAIL`
  - `rg -q 'slicer-report-data' docs/16_slicer_report.md && echo PASS || echo FAIL`
- Exit condition: Doc grep confirms both Phase Totals update and JSON block documentation present.

### Step 5: Final gate

- Task IDs: none
- Objective: Full verification gate — compile, lint, test.
- Precondition: Steps 1-4 complete.
- Postcondition: All gate commands pass.
- Files allowed to read: none (pure dispatch)
- Files allowed to edit: none
- Files explicitly out-of-bounds: all
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace`; return FACT (pass/fail + first error if any)"
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT (pass/fail + first warning if any)"
  - "Run `cargo test -p slicer-host --test slicer_report_html_tdd`; return FACT (pass/fail)"
- Context cost: `S`
- Verification:
  - `cargo check --workspace`
  - `cargo clippy --workspace -- -D warnings`
  - `cargo test -p slicer-host --test slicer_report_html_tdd`
- Exit condition: All three commands pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Dispatch-only baseline check |
| Step 1 | S | Add one struct + one field to 161-line file |
| Step 2 | M | Range-read 4 sections of 504-line collector, 3 edit locations |
| Step 3 | M | Range-read 3 sections of 457-line renderer, ~30 lines edited |
| Step 3.5 | M | Add serde derives to model.rs + ~60 lines render.rs + test assertions |
| Step 4 | S | Two-paragraph doc update |
| Step 5 | S | Dispatch-only final gate |
| **Aggregate** | **M** | Largest single step: M (Steps 2 and 3.5) |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- All acceptance criteria (`AC-1` through `AC-8`, `AC-N2`) dispatched and returning PASS.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm Step 5 gate commands pass (`cargo check`, `cargo clippy`, `cargo test`).
- Confirm `docs/16_slicer_report.md` greps return hits for both Phase Totals update and JSON block documentation.
- Record any remaining packet-local risk before moving to `status: implemented`.

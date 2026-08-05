---
status: implemented
packet: 169-time-estimator-slice-stats
task_ids:
  - TASK-275
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
depends_on:
  - .ralph/specs/167-config-block-viewer-keys (queue row #2; supplies the CONFIG_BLOCK viewer-key contract and the "padding never emits machine limits" invariant used by this packet's fixtures)
plan_source: docs/specs/fork-gaps-wave1-plan.md (Packet A — fork handoff items 1, 2, 12)
---

# Packet Contract: 169-time-estimator-slice-stats

## Goal

Add an acceleration-aware trapezoidal print-time estimator as a post-emit analysis pass in `crates/slicer-gcode`, emit a new `slice_stats` progress event (schema 1.2.0, amended field list) before `slice_complete`, and add an optional `layer_count` field to `phase_start(per_layer)`.

## Scope Boundaries

One walk over the emitted `GCodeIR.commands` stream computes total time (Marlin-style simplified trapezoidal model), per-extruder extruded volume (mm³), filament length, and toolchange count; the result fills `PrintMetadata.estimated_print_time_s` (currently hardcoded `0` at `crates/slicer-gcode/src/emit.rs:739`) and feeds the host-side `slice_stats` JSONL event in `crates/slicer-runtime/src/progress_events.rs`, wired at the end-of-slice path in `crates/slicer-runtime/src/run.rs` (which also gains the production `slice_complete` emission — today `ProgressEvent::slice_complete` has no production call site). No new WASM module, no WIT changes, no M73 emission (wave 2), and explicitly no cost field.

## Prerequisites and Blockers

- Depends on: packet `167-config-block-viewer-keys` (TASK-273) — cites its docs/02 "CONFIG_BLOCK viewer-key contract" anchor and its invariant that CONFIG_BLOCK padding never emits machine-limit/speed/accel/jerk keys; test fixtures supply `machine_max_*` / `filament_density` via raw_config per that contract.
- Unblocks: wave-2 M73 progress-remaining emission (handoff item 15), fork GUI exact progress bar and post-slice stats panel.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** a synthetic `GCodeIR` whose commands are three `GCodeCommand::Move` entries forming a single straight 100 mm XY segment at `f = 3000` (50 mm/s) with config `machine_max_acceleration_extruding = 500` mm/s² and jerk 9 mm/s, **when** `estimate_print(&gcode_ir, &estimator_config)` runs, **then** `PrintEstimate.total_time_s` equals the analytic trapezoid time for that segment within ±2% and is strictly greater than the constant-velocity lower bound `100/50 = 2.0` s. | `mkdir -p target && cargo test -p slicer-gcode --test estimator -- trapezoid_single_segment_analytic 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-2. Given** a synthetic `GCodeIR` containing extruding moves on tool 0 (total `e` delta 10.0 mm), one `GCodeCommand::ToolChange { from: 0, to: 1, .. }`, then extruding moves on tool 1 (`e` delta 5.0 mm), with `filament_diameter = 1.75` for both tools, **when** the estimator runs, **then** `PrintEstimate.extruded_volume_mm3` is a map with exactly keys `0` and `1`, values `10.0 * π*(1.75/2)²` and `5.0 * π*(1.75/2)²` mm³ within 1e-3, `filament_length_mm` per tool equals `[10.0, 5.0]` within 1e-6, and `toolchange_count == 1`. | `mkdir -p target && cargo test -p slicer-gcode --test estimator -- two_tool_volume_map_and_toolchange_count 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-3. Given** the estimator ran, **when** `emit_gcode` returns its `GCodeIR`, **then** `gcode_ir.metadata.estimated_print_time_s` is the estimator total rounded to whole seconds and is `> 0` for any non-empty move stream (the `// Not calculated in this implementation` literal `0` at `emit.rs:739` is gone). | `mkdir -p target && cargo test -p slicer-gcode --test estimator -- metadata_estimated_time_filled 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-4. Given** a slice run through `run.rs` with progress events enabled (note: `ProgressEvent::slice_complete` currently has **no** production call site — this packet creates that emission from the existing `SliceEventCollector`), **when** the run finishes, **then** the JSONL stream contains exactly one `"event":"slice_stats"` line emitted before exactly one `"event":"slice_complete"` line, the `slice_stats` line has `"schema_version":"1.2.0"` and fields `gcode_prediction_seconds`, `gcode_weight_grams`, `gcode_filament_length_mm`, `layer_count`, `first_layer_height_mm`, `extruded_volume_mm3` (map keyed by extruder index), `toolchange_count` — and no key named `cost` or `gcode_cost` anywhere in the event. | `mkdir -p target && cargo test -p slicer-runtime --test integration -- slice_stats_event_shape_and_ordering 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-5. Given** the per-layer phase begins, **when** `ProgressPipelineInstrumentation` handles the per-layer phase start (event constructed at `progress_instrumentation.rs:104`), **then** the emitted `phase_start` event carries `"layer_count": <N>` where `N == plan.global_layers.len()`, and `phase_start` events for other phases omit the key entirely (serde skip on `None`). | `mkdir -p target && cargo test -p slicer-runtime --test integration -- phase_start_per_layer_carries_layer_count 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-6. Given** the docs are amended, **when** grepping `docs/09_progress_events.md`, **then** the 1.2.0 row is no longer "Reserved" and lists the amended field set including `extruded_volume_mm3` and `toolchange_count`. | `rg -q 'extruded_volume_mm3' docs/09_progress_events.md && rg -q 'toolchange_count' docs/09_progress_events.md && ! rg -q 'Reserved for `pinch_n_print_studio` T-096' docs/09_progress_events.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** an estimator config where every `machine_max_acceleration_*`, `machine_max_speed_*`, and jerk key is absent, **when** the estimator runs on a non-empty move stream, **then** it uses the documented fallback defaults (accel 1500 mm/s², max speed X/Y 200 mm/s, Z 12 mm/s, E 25 mm/s, jerk X/Y 9 mm/s, Z 0.2 mm/s, E 2.5 mm/s), returns `total_time_s > 0`, and never panics or returns an error. | `mkdir -p target && cargo test -p slicer-gcode --test estimator -- fallback_defaults_when_machine_limits_absent 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-N2. Given** `filament_density` is absent from the slice config, **when** `slice_stats` is emitted, **then** the JSON object omits the `gcode_weight_grams` key entirely (it is never `0` or `null`), while all other slice_stats fields remain present. | `mkdir -p target && cargo test -p slicer-runtime --test integration -- slice_stats_omits_weight_without_density 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`
- **AC-N3. Given** a consumer parsing only 1.1.0-era events, **when** it deserializes a 1.2.0 stream via the existing `ProgressEvent` serde round-trip, **then** every pre-existing event variant (`phase_start` without `layer_count`, `slice_complete`, etc.) round-trips unchanged — additive-only bump per docs/09's "Additive fields are a minor version bump" rule. | `mkdir -p target && cargo test -p slicer-runtime --test integration -- progress_event_1_1_0_roundtrip_unchanged 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `mkdir -p target && cargo test -p slicer-runtime --test integration -- slice_stats 2>&1 | tee target/test-output.log | grep -E "^test result|FAILED"`

## Authoritative Docs

- `docs/09_progress_events.md` — direct ranged read: version table (~lines 145-160) and additive-bump rule (~line 113); the reserved 1.2.0 row at line 153 is amended by this packet.
- `docs/02_ir_schemas.md` — section "CONFIG_BLOCK viewer-key contract" (authored by packet 167) — cite only; delegated grep, no full read.
- `docs/07_implementation_status.md` — delegated; TASK-275 minted at closure via `task-map.md`.

## Doc Impact Statement (Required)

- `docs/09_progress_events.md` — amend the 1.2.0 row from "Reserved" to the shipped `slice_stats` field list (five reserved fields + `extruded_volume_mm3` + `toolchange_count`, explicitly no cost field), bump the current-version line to 1.2.0, document `layer_count` as an optional additive field on `phase_start(per_layer)` and the omit-when-no-density semantics of `gcode_weight_grams`, and resolve the instrumented-stream version divergence: the doc's `--instrument-stderr` section (lines ~162-163) claims the instrumented stream is `"1.3.0"` while `PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED` is actually `"1.1.0"`; this packet sets the instrumented const to `"1.2.0"` (same additive payload; the 1.3.0 stage/module-event row stays future) and rewrites the doc section to match - `rg -q 'slice_stats' docs/09_progress_events.md && rg -q '"1.2.0"|1\.2\.0' docs/09_progress_events.md && rg -q 'layer_count' docs/09_progress_events.md`
- `docs/07_implementation_status.md` — add the TASK-275 row at closure (owned by `task-map.md`) - `rg -q 'TASK-275' docs/07_implementation_status.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

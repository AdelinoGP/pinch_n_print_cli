# Requirements: 169-time-estimator-slice-stats

## Packet Metadata

- Grouped task IDs: `TASK-275` (new; minted into `docs/07_implementation_status.md` via `task-map.md`)
- Backlog source: `docs/07_implementation_status.md` (via approved plan `docs/specs/fork-gaps-wave1-plan.md`, Packet A)
- Packet status: `draft`
- Aggregate context cost: `M`
- Depends on: packet `167-config-block-viewer-keys` (TASK-273)

## Problem Statement

The OrcaSlicer-frontend fork shells out to `pnp_cli slice` and shows the user no gap warnings, so every missing stat fails silently. Today `PrintMetadata.estimated_print_time_s` is hardcoded `0` (`crates/slicer-gcode/src/emit.rs:739`, comment "Not calculated in this implementation"), the progress-event stream (schema 1.1.0) has no post-slice statistics event (the 1.2.0 `slice_stats` row in `docs/09_progress_events.md:153` is only reserved), and `phase_start(per_layer)` carries no total layer count so the fork's progress bar cannot be exact during the slice. This packet closes fork handoff items 1, 2, and 12 as one coherent slice: the estimator produces the numbers, `slice_stats` transports them, and `layer_count` on `phase_start` makes the in-slice progress bar exact.

## In Scope

- A Marlin-style simplified trapezoidal time estimator as a **post-emit analysis pass** in `crates/slicer-gcode` (new `estimator` module): one walk over `GCodeIR.commands` (`GCodeCommand::Move` / `Retract` / `Unretract` / `ToolChange` / `ExtrusionMode`), acceleration- and jerk-limited per-segment planning using `machine_max_acceleration_*` / `machine_max_speed_*` / jerk values from the slice config.
- Documented fallback defaults when `machine_max_*` keys are absent from config (exact values in AC-N1 / design.md): accel 1500 mm/s², max speed X/Y 200, Z 12, E 25 mm/s, jerk X/Y 9, Z 0.2, E 2.5 mm/s.
- Estimator outputs: `total_time_s`, per-extruder `extruded_volume_mm3` map (keyed by extruder index; volume = filament length × π·(filament_diameter/2)², per-tool diameter from `tool_configs`), per-extruder `filament_length_mm`, `toolchange_count`.
- Fill `PrintMetadata.estimated_print_time_s` (and reuse the estimator's per-tool lengths for `filament_used_mm` if currently divergent — verify, do not silently change semantics).
- New `slice_stats` progress event in `crates/slicer-runtime/src/progress_events.rs`, emitted exactly once, before `slice_complete`, bumping `PROGRESS_EVENT_SCHEMA_VERSION` (and its `_INSTRUMENTED` twin) from `1.1.0` to `1.2.0`. Fields: `gcode_prediction_seconds`, `gcode_weight_grams` (optional; omitted when `filament_density` absent), `gcode_filament_length_mm`, `layer_count`, `first_layer_height_mm`, `extruded_volume_mm3` (map), `toolchange_count`.
- **Create the production `slice_complete` emission**: `ProgressEvent::slice_complete` (constructor at `progress_events.rs:346`) has **no** production call site today — it is built only in tests; the emitter and `SliceEventCollector` are wired in `crates/slicer-runtime/src/run.rs:168-170` but never finalized into a `slice_complete` line. This packet emits `slice_stats` then `slice_complete` (from the collector's error counts) at the end-of-slice path in `run.rs`, making the ordering contract observable.
- `layer_count` plumbing through the instrumentation layer: the JSONL `phase_start` is constructed in `crates/slicer-runtime/src/progress_instrumentation.rs:104` inside `PipelineInstrumentation::on_phase_start`, driven from `pipeline.rs:356`. Extend the trait with a default-implemented, additive method (e.g. `on_phase_start_with_layer_count(phase, Option<u32>)` defaulting to `on_phase_start`) so other `PipelineInstrumentation` impls compile unchanged.
- Resolve the instrumented-version divergence: docs/09 (~lines 162-163) says `--instrument-stderr` bumps the version to `"1.3.0"`, but `PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED` is `"1.1.0"` in code. Decision: instrumented const becomes `"1.2.0"` (same additive payload fields; the 1.3.0 stage/module-event cadence row stays future) and the doc section is rewritten to match reality.
- Weight derivation: `gcode_weight_grams = Σ_extruder volume_cm³ × filament_density_g_cm3`, density taken from the slice config (fork-supplied per the packet-167 raw_config contract), **not** from the `serialize.rs` header default `1.24`.
- Optional additive `layer_count` field on `phase_start` when `phase = per_layer` (value: `plan.global_layers.len()` at `crates/slicer-runtime/src/pipeline.rs:352-356`), omitted for all other phases.
- Amend `docs/09_progress_events.md`: 1.2.0 row from "Reserved" to the shipped amended field list; document `layer_count` on `phase_start` and the weight-omission semantics.
- Fork-realistic test fixtures supplying `machine_max_*` / `filament_density` via raw_config per the docs/02 "CONFIG_BLOCK viewer-key contract" (packet 167).

## Out of Scope

- Any cost field in `slice_stats` — **design invariant: the fork computes cost from its own preset**; PNP never emits cost.
- M73 remaining-time G-code emission (wave 2, handoff item 15 — strictly downstream of this estimator).
- A new WASM module, WIT changes, or guest rebuilds (`crates/slicer-gcode` and `progress_events.rs` are host-side only; grounding confirmed no WIT progress surface exists).
- Changing `ORCA_CONFIG_PADDING` or CONFIG_BLOCK serialization (owned by packet 167). The estimator reads machine limits from the slice config host-side, never from CONFIG_BLOCK padding.
- Porting OrcaSlicer's `GCodeProcessor` time model — the trapezoid model is written fresh (Marlin-style), no C++ port, no attribution header needed.
- Fixing the pre-existing docs/09-vs-code discrepancy on `slice_complete.output_path` (doc claims a field the struct lacks) — note it, do not fix here.

## Authoritative Docs

- `docs/09_progress_events.md` — ~160+ lines; direct ranged reads only: version table (~145-160), additive rule (~113), event field tables as needed.
- `docs/02_ir_schemas.md` — large; delegated grep for the "CONFIG_BLOCK viewer-key contract" anchor only.
- `docs/07_implementation_status.md` — always delegated; TASK-275 confirmed absent (highest existing: TASK-271).
- `docs/specs/fork-gaps-wave1-plan.md` — approved plan; Packet A section is the scope authority.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (analytic trapezoid time), `AC-2` (two-tool volume map + toolchange count), `AC-3` (metadata filled), `AC-4` (slice_stats shape/ordering/version), `AC-5` (phase_start layer_count), `AC-6` (docs/09 amended).
- Negative: `AC-N1` (fallback defaults when machine limits absent), `AC-N2` (weight omitted without density; never 0/null), `AC-N3` (1.1.0-era events round-trip unchanged — additive-only bump).
- Cross-packet impact: fixtures cite packet 167's viewer-key contract; wave-2 M73 packet will consume `PrintEstimate`.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `mkdir -p target && cargo test -p slicer-gcode --test estimator 2>&1 \| tee target/test-output.log \| grep -E "^test result\|FAILED"` | All estimator unit/analytic tests (AC-1/2/3, AC-N1) | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- slice_stats 2>&1 \| tee target/test-output.log \| grep -E "^test result\|FAILED"` | slice_stats event shape, ordering, weight omission (AC-4, AC-N2) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- phase_start_per_layer_carries_layer_count 2>&1 \| tee target/test-output.log \| grep -E "^test result\|FAILED"` | layer_count additive field (AC-5) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test integration -- progress_event_1_1_0_roundtrip_unchanged 2>&1 \| tee target/test-output.log \| grep -E "^test result\|FAILED"` | Backward-compat negative (AC-N3) | FACT pass/fail |
| `rg -q 'extruded_volume_mm3' docs/09_progress_events.md && rg -q 'toolchange_count' docs/09_progress_events.md && ! rg -q 'Reserved for \`pinch_n_print_studio\` T-096' docs/09_progress_events.md && echo PASS` | Doc amendment (AC-6) | FACT PASS/absent |
| `cargo check --workspace --all-targets` | Compile gate incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint gate | FACT pass/fail |
| End-to-end (closure only, per plan §Verification): slice `resources/benchy.stl` with fork-realistic raw_config; assert `slice_stats` JSONL line present with all fields and `estimated_print_time_s > 0` in output | Plan-mandated G+A e2e | FACT pass/fail via delegated run |

## Step Completion Expectations

- The estimator (`slicer-gcode`) must land and be unit-proven before the runtime wiring step consumes `PrintEstimate`; the schema bump and doc amendment land in the same step as the `slice_stats` emission so version const, event, and doc never diverge across a commit boundary.
- `filament_density` plumbing: the estimator itself is density-agnostic (volume only); weight is computed at the runtime wiring layer where config access exists. Do not add density to the estimator's inputs.

## Context Discipline Notes

- `crates/slicer-gcode/src/emit.rs` and `crates/slicer-ir/src/slice_ir.rs` are both large; read only the grounded ranges (emit.rs 60-80, 130-190, 700-760; slice_ir.rs 2190-2300).
- `docs/07_implementation_status.md` must never be read in full — closure update goes through a worker dispatch.
- Packet 167's files: inspect only via SUMMARY dispatch (Packet Safety rule); its docs/02 anchor is cited by name, not re-read.
- New runtime progress-event tests go in the existing integration bucket (`crates/slicer-runtime/tests/integration/progress_events_tdd.rs` already hosts `slice_complete_event_has_required_fields` etc.) — do NOT add a new unit/main.rs module registration; all runtime AC commands target `--test integration`.

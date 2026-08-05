---
status: implemented
packet: 169-time-estimator-slice-stats
task_ids:
  - TASK-275
---

# 169-time-estimator-slice-stats

## Goal

Add an acceleration-aware trapezoidal print-time estimator as a post-emit analysis pass in `crates/slicer-gcode`, emit a new `slice_stats` progress event (schema 1.2.0, amended field list) before `slice_complete`, and add an optional `layer_count` field to `phase_start(per_layer)`.

## Problem Statement

The OrcaSlicer-frontend fork shells out to `pnp_cli slice` and shows the user no gap warnings, so every missing stat fails silently. Today `PrintMetadata.estimated_print_time_s` is hardcoded `0` (`crates/slicer-gcode/src/emit.rs:739`, comment "Not calculated in this implementation"), the progress-event stream (schema 1.1.0) has no post-slice statistics event (the 1.2.0 `slice_stats` row in `docs/09_progress_events.md:153` is only reserved), and `phase_start(per_layer)` carries no total layer count so the fork's progress bar cannot be exact during the slice. This packet closes fork handoff items 1, 2, and 12 as one coherent slice: the estimator produces the numbers, `slice_stats` transports them, and `layer_count` on `phase_start` makes the in-slice progress bar exact.

## Architecture Constraints

- Progress events are host-side JSONL only; grounding confirmed `crates/slicer-schema/wit` has zero progress/phase/event surface. If the implementer discovers any WIT change is needed, that is a `[BLOCK]` — stop and report; do not extend WIT under this packet.
- `crates/slicer-gcode` is not a guest-WASM input path (not in the CLAUDE.md staleness list) — no guest rebuilds required.
- Estimator arithmetic operates on the already-emitted G-code coordinate space (mm, from `GCodeCommand::Move { x, y, z, e, f }`), not on IR units; the 100 nm unit hazard does not apply to this pass. Do not reach back into pre-emit geometry.
- Invariant inherited from packet 167: CONFIG_BLOCK padding never emits machine-limit/speed/accel/jerk keys and fork-supplied raw_config always wins. The estimator therefore reads machine limits **only** from the slice config (`ResolvedConfig` / raw config host-side), never by parsing CONFIG_BLOCK output.

## Data and Contract Notes

- IR/manifest contracts: `PrintMetadata.estimated_print_time_s: u32` (seconds) — estimator rounds `total_time_s`. No IR shape changes beyond optional `ResolvedConfig` fields (additive, snake_case keys).
- WIT boundary: none touched (verified empty grep); any discovered need is `[BLOCK]`.
- Determinism: the estimator is a pure function of `GCodeIR` + limits; use `BTreeMap` (not HashMap) for stable JSON key order in `extruded_volume_mm3`.
- Schema contract: 1.1.0 → 1.2.0 is additive-only per docs/09 line 113 ("Additive fields are a minor version bump"); all new `ProgressEvent` fields are `Option` + `skip_serializing_if`.
- Known pre-existing discrepancy (do not fix): docs/09's 1.1.0 row mentions `slice_complete.output_path` but the struct has no such field.

## Locked Assumptions and Invariants

- **No cost field, ever**: `slice_stats` never carries cost; the fork computes cost from its own preset. Adding one later is a schema change requiring its own packet.
- **Weight omission semantics**: `gcode_weight_grams` is omitted (not `0`, not `null`) when `filament_density` is absent from config.
- **Fallback machine limits** (used only when config keys absent): accel 1500 mm/s²; max speed X/Y 200, Z 12, E 25 mm/s; jerk X/Y 9, Z 0.2, E 2.5 mm/s. Documented in docs/09 amendment.
- **Machine limits come from slice config only**, never from CONFIG_BLOCK padding (packet-167 invariant).
- `slice_stats` is emitted exactly once per successful slice, strictly before `slice_complete` (whose production emission this packet creates — grounding confirmed `ProgressEvent::slice_complete` currently has zero production call sites).
- Instrumented stream version: `"1.2.0"` after this packet; `"1.3.0"` remains reserved for the future stage/module-event schema (docs/09 instrumented section rewritten to say so).
- `PipelineInstrumentation` gains only an additive default-implemented method — no existing trait impl may require changes to compile.

## Risks and Tradeoffs

- Simplified trapezoid (single scalar accel, per-axis speed caps, jerk as junction floor) will deviate from Marlin's full planner on corner-heavy models; acceptable — the fork needs an estimate, not firmware parity. Analytic single-segment test (AC-1) pins the model's core math instead.
- Absolute/relative `e` tracking is the likeliest silent-corruption point for volumes; mitigated by the serialize.rs FACT dispatch and AC-2.
- `ResolvedConfig` additions ripple into any exhaustive-construction sites; mitigated by `cargo check --workspace --all-targets`.
- Creating the `slice_complete` emission is a small scope addition beyond the plan text, but AC-4's ordering contract (and docs/09's own event table) is unobservable without it; consumers already tolerate its absence, so adding it is strictly additive. Flagged for the coordinator as a plan deviation.
- `filament_used_mm` already exists with unknown provenance; the packet fills only `estimated_print_time_s` and leaves `filament_used_mm` untouched unless a step proves them identical.

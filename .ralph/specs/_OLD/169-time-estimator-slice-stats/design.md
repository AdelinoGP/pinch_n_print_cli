# Design: 169-time-estimator-slice-stats

## Controlling Code Paths

- Primary code path: `crates/slicer-runtime/src/postpass.rs` drives `emit_gcode(&reconciled_layers)` (line ~189) → `GCodeIR` → `serialize_gcode(&gcode_ir)` (line ~305). The estimator inserts between emit and serialize. Progress events flow through `crates/slicer-runtime/src/progress_events.rs` (`ProgressEvent` struct at 124-168, `phase_start` ctor at 174, `slice_complete` ctor at 346, `JsonLinesEmitter` at 512-544). The emitter + `SliceEventCollector` are wired in `crates/slicer-runtime/src/run.rs:168-174`; **`ProgressEvent::slice_complete` has no production call site today** (constructor + tests only), so `run.rs` is the end-of-slice anchor where this packet emits `slice_stats` and then creates the `slice_complete` emission from the collector's error counts.
- Instrumentation path for `layer_count`: `pipeline.rs:356` calls `PipelineInstrumentation::on_phase_start(Phase::PerLayer)` (immediately after `plan.global_layers` is populated at 352-354); the JSONL `ProgressEvent::phase_start` is constructed inside `ProgressPipelineInstrumentation::on_phase_start` at `crates/slicer-runtime/src/progress_instrumentation.rs:104`. Plumbing `layer_count` therefore touches the trait, `progress_instrumentation.rs`, and `pipeline.rs`.
- Neighboring tests/fixtures: existing progress-event tests live in `crates/slicer-runtime/tests/integration/progress_events_tdd.rs` (`--test integration` bucket) — new runtime tests join that file; `crates/slicer-gcode` gains a new `tests/estimator.rs`.
- OrcaSlicer comparison: none — the trapezoid model is written fresh (Marlin-style), no port, no attribution obligations.

## Architecture Constraints

- Progress events are host-side JSONL only; grounding confirmed `crates/slicer-schema/wit` has zero progress/phase/event surface. If the implementer discovers any WIT change is needed, that is a `[BLOCK]` — stop and report; do not extend WIT under this packet.
- `crates/slicer-gcode` is not a guest-WASM input path (not in the CLAUDE.md staleness list) — no guest rebuilds required.
- Estimator arithmetic operates on the already-emitted G-code coordinate space (mm, from `GCodeCommand::Move { x, y, z, e, f }`), not on IR units; the 100 nm unit hazard does not apply to this pass. Do not reach back into pre-emit geometry.
- Invariant inherited from packet 167: CONFIG_BLOCK padding never emits machine-limit/speed/accel/jerk keys and fork-supplied raw_config always wins. The estimator therefore reads machine limits **only** from the slice config (`ResolvedConfig` / raw config host-side), never by parsing CONFIG_BLOCK output.

## Code Change Surface

- Selected approach: pure-function post-emit pass in a new module `crates/slicer-gcode/src/estimator.rs`.
  - `pub struct EstimatorLimits { pub max_acceleration: f32, pub max_speed_xy: f32, pub max_speed_z: f32, pub max_speed_e: f32, pub jerk_xy: f32, pub jerk_z: f32, pub jerk_e: f32 }` with `impl Default` = the documented fallbacks (1500 mm/s²; 200 / 12 / 25 mm/s; 9 / 0.2 / 2.5 mm/s). Construction: `EstimatorLimits::from_config(&ResolvedConfig)`-style helper that falls back per-field to `Default` when the corresponding `machine_max_*` key is absent. (Grounding: **no** `machine_max_*` key is read anywhere in the tree today — grep returned zero matches — so this packet also adds those optional fields to `ResolvedConfig` via its existing cli-key macro, `crates/slicer-ir/src/resolved_config.rs`, as `Option<f32>`/vec fields with snake_case keys.)
  - `pub struct PrintEstimate { pub total_time_s: f64, pub extruded_volume_mm3: BTreeMap<u32, f64>, pub filament_length_mm: BTreeMap<u32, f64>, pub toolchange_count: u32 }`
  - `pub fn estimate_print(gcode_ir: &GCodeIR, limits: &EstimatorLimits, tool_diameters: &BTreeMap<u32, f32>) -> PrintEstimate` — one forward walk over `GCodeIR.commands`. Trapezoidal model per segment: entry/exit junction speed = min(jerk-limited junction, programmed `f`, axis max-speed caps); accelerate–cruise–decelerate with `max_acceleration`; degenerate triangle profile when the segment is too short to reach cruise. `ToolChange { from, to }` switches the active-extruder accumulator and increments `toolchange_count`. `ExtrusionMode` toggles absolute/relative `e` interpretation (the emitter emits both; the walk must track mode exactly as `serialize` does). `Retract`/`Unretract` contribute e-axis time via `max_speed_e`/`jerk_e` and are excluded from volume.
- Exact functions/edits:
  - `crates/slicer-gcode/src/emit.rs` — replace the literal `estimated_print_time_s: 0` (line 739) by running `estimate_print` before constructing `PrintMetadata` (or immediately after, mutating `metadata`); keep `filament_used_mm` semantics unchanged unless proven identical to the estimator's per-tool lengths.
  - `crates/slicer-gcode/src/lib.rs` — export the `estimator` module and `PrintEstimate` so slicer-runtime can consume it.
  - `crates/slicer-runtime/src/progress_events.rs` — add `ProgressEventType::SliceStats` (serde rename `slice_stats`), optional fields on `ProgressEvent` (`gcode_prediction_seconds: Option<u64>`, `gcode_weight_grams: Option<f64>` with `skip_serializing_if = "Option::is_none"`, `gcode_filament_length_mm: Option<f64>`, `first_layer_height_mm: Option<f32>`, `extruded_volume_mm3: Option<BTreeMap<u32, f64>>`, `toolchange_count: Option<u32>`, reusing the existing `layer_count`-style Option pattern), a `slice_stats(...)` constructor, `layer_count: Option<u32>` on `phase_start`, and bump `PROGRESS_EVENT_SCHEMA_VERSION` (line 29) to `"1.2.0"` and `PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED` (line 35) from `"1.1.0"` to `"1.2.0"` (see instrumented-version decision below).
  - `crates/slicer-runtime/src/run.rs` — end-of-slice wiring at the emitter/collector site (lines 168-174): run the estimator on the produced `GCodeIR` (or receive its `PrintEstimate` from the postpass output), compute weight from config `filament_density` (present → `Some(volume_cm3 * density)`, absent → `None`), emit `slice_stats`, then **create** the `slice_complete` emission from `SliceEventCollector`'s fatal/non-fatal counts — today no production code emits `slice_complete` at all; this packet makes the documented final event real and the AC-4 ordering observable.
  - `crates/slicer-runtime/src/postpass.rs` — after `emit_gcode`, call the estimator and write `estimated_print_time_s` into `gcode_ir.metadata`; surface the `PrintEstimate` to `run.rs` (via the postpass/pipeline output struct) so the event layer does not re-walk the IR.
  - `crates/slicer-runtime/src/progress_instrumentation.rs` — thread `layer_count` into the `ProgressEvent::phase_start` construction at line 104.
  - `PipelineInstrumentation` trait — add an **additive default-implemented** method `on_phase_start_with_layer_count(&self, phase: Phase, layer_count: Option<u32>)` whose default delegates to `on_phase_start(phase)`; `ProgressPipelineInstrumentation` overrides it. Other trait impls compile unchanged (no signature break).
  - `crates/slicer-runtime/src/pipeline.rs` — call `on_phase_start_with_layer_count(Phase::PerLayer, Some(plan.global_layers.len() as u32))` at lines 352-356.
- Instrumented-version decision (resolves a live three-way divergence): docs/09 lines 162-163 claim `--instrument-stderr` bumps the version to `"1.3.0"` ("1.1.0 and 1.2.0 are reserved for additive payload fields"), yet both consts are `"1.1.0"` in code and the 1.3.0 cadence row is marked "(future)". This packet sets the instrumented const to `"1.2.0"` — the instrumented stream carries the same additive 1.2.0 payload; the 1.3.0 designation stays reserved for the future stage/module-event schema — and Step 5 rewrites the docs/09 instrumented section to state exactly that.
  - `crates/slicer-ir/src/resolved_config.rs` — optional `machine_max_*` / jerk / `filament_density` fields (snake_case cli keys per the Config Key Naming Convention).
  - Tests: new `crates/slicer-gcode/tests/estimator.rs`; new/extended cases in `crates/slicer-runtime/tests/integration/progress_events_tdd.rs` (the existing `--test integration` bucket — no new test-binary module registration needed).
- Rejected alternatives:
  - New WASM module for estimation — rejected: plan mandates a post-emit host pass; a module would need WIT surface for the full move stream.
  - Emitting `slice_stats` from `slicer-gcode` directly — rejected: the JSONL emitter and slice_id/timestamps live in slicer-runtime; slicer-gcode stays IO-free.
  - Weight = 0 when density absent — rejected in favor of key omission: `0 g` is a plausible-looking wrong value the fork would display; an absent key lets the fork fall back to its own preset density. **Recorded design decision.**
  - Parsing final G-code text instead of walking `GCodeIR.commands` — rejected: the typed stream already carries roles, tool changes, and extrusion mode; re-parsing invites drift.

## Files in Scope (read + edit)

Six files exceed the 3-file target because the slice spans producer (gcode), transport (runtime events), and config plumbing; the packet remains atomic because the schema bump and its producer must land together (see requirements.md §Step Completion Expectations). Splitting would strand a 1.2.0 version const with no emitter or vice versa.

- `crates/slicer-gcode/src/estimator.rs` (new) — role: the pass; expected change: new module.
- `crates/slicer-gcode/src/emit.rs` — role: fill `estimated_print_time_s`; expected change: replace line-739 literal, small.
- `crates/slicer-gcode/src/lib.rs` — role: export; expected change: 1-2 lines.
- `crates/slicer-runtime/src/progress_events.rs` — role: schema 1.2.0 (both consts) + `SliceStats` + `layer_count`; expected change: moderate.
- `crates/slicer-runtime/src/run.rs` — role: emit slice_stats + create the slice_complete emission at the collector site; expected change: small.
- `crates/slicer-runtime/src/postpass.rs` — role: run estimator, fill metadata, surface `PrintEstimate`; expected change: small.
- `crates/slicer-runtime/src/progress_instrumentation.rs` — role: layer_count into the phase_start construction (line 104) + trait default-method override; expected change: small.
- `crates/slicer-runtime/src/pipeline.rs` — role: call the layer_count-aware phase-start hook; expected change: 1-3 lines.
- Plus: `crates/slicer-ir/src/resolved_config.rs` (optional config fields), `crates/slicer-gcode/tests/estimator.rs` (new tests), `crates/slicer-runtime/tests/integration/progress_events_tdd.rs` (runtime tests), `docs/09_progress_events.md` (amendment).

## Read-Only Context

- `crates/slicer-gcode/src/emit.rs` — lines 60-80 (emitter fields incl. `tool_configs`), 130-190 (`resolve_feedrate`), 360-380 (ToolChange push), 700-760 (`PrintMetadata` literal) — purpose: integration points.
- `crates/slicer-ir/src/slice_ir.rs` — lines 2190-2300 only (`GCodeCommand`, `PrintMetadata`, `GCodeIR`) — purpose: exact variant/field shapes.
- `crates/slicer-gcode/src/serialize.rs` — lines 60-140 only — purpose: how `filament_density_g_cm3` default 1.24 is header-only; estimator must not inherit it.
- `crates/slicer-runtime/src/progress_events.rs` — lines 20-200, 340-380, 500-560 — purpose: version consts, ctors, emitter, `SliceEventCollector`.
- `crates/slicer-runtime/src/run.rs` — lines 150-220 — purpose: emitter/collector wiring, end-of-slice anchor.
- `crates/slicer-runtime/src/progress_instrumentation.rs` — lines 40-130 — purpose: `on_phase_start` impl and event construction (line 104).
- `crates/slicer-runtime/src/pipeline.rs` — lines 340-370 only — purpose: per-layer phase_start call site and `PipelineInstrumentation` trait location.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — no parity work in this packet; never load.
- `.ralph/specs/167-config-block-viewer-keys/**` — never modify; inspect only via SUMMARY dispatch.
- `crates/slicer-gcode/src/serialize.rs` beyond lines 60-140 (CONFIG_BLOCK/padding is packet 167's surface).
- `target/`, `Cargo.lock`, generated code, vendored dependencies — never load.
- `docs/07_implementation_status.md` — worker dispatch only.

## Expected Sub-Agent Dispatches

- Question: exact `ResolvedConfig` cli-key macro syntax for an `Option<f32>` field and whether `first_layer_height` already exists as a field (name + type); scope: `crates/slicer-ir/src/resolved_config.rs`; return: `SNIPPETS` (≤2, ≤20 lines); purpose: Step 1 config plumbing.
- Question: at run.rs's post-pipeline return path, which bindings are live (pipeline output struct fields, config handle, collector) for wiring slice_stats + the new slice_complete emission; scope: `crates/slicer-runtime/src/run.rs`; return: `LOCATIONS`; purpose: Step 3 wiring.
- Question: full definition site of the `PipelineInstrumentation` trait (file:line, method list, existing impls) to confirm the additive default method compiles all impls unchanged; scope: `crates/slicer-runtime/src/`; return: `LOCATIONS`; purpose: Step 4 layer_count plumbing.
- Question: how does serialize.rs interpret `ExtrusionMode` + `e` (absolute vs relative) so the estimator's e-tracking matches; scope: `crates/slicer-gcode/src/serialize.rs`; return: `FACT`; purpose: Step 2 correctness.
- Question: run each verification command; scope: workspace; return: `FACT pass/fail` + ≤20-line failure SNIPPETS.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 2, estimator core)
- Highest-risk dispatch and required return format: serialize.rs extrusion-mode semantics — `FACT` (≤5 lines); a wrong answer corrupts every volume figure.

## Open Questions

- `[FWD]` Does `ResolvedConfig` already expose `first_layer_height` (needed for `first_layer_height_mm` in slice_stats)? If absent, add it as an optional field in Step 1 alongside the machine limits.
- `[FWD]` Should per-axis `machine_max_acceleration_extruding` vs `_travel` be distinguished (Orca config has both)? Default decision: single `machine_max_acceleration_extruding` for extruding moves and `machine_max_acceleration_travel` for travel moves, each optional with the same 1500 fallback; implementer may collapse to one key if the fork only supplies one — record in docs/09 amendment.
- `[FWD]` Whether `slice_stats` should also fire on degraded-but-successful slices (status Ok with non-fatal errors). Default: yes — emit whenever `slice_complete` reports a produced G-code artifact.

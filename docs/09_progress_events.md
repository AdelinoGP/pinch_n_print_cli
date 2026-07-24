# Pinch 'n Print — Progress & Error Event Contract

**What this covers:** the JSON-Lines event stream the host emits on stderr during
a slice — the event types, their required fields, ordering guarantees, and the
schema-version history.

**Who it's for:** anyone building a frontend or agent that consumes slice
progress, and anyone adding or changing an emitted event.

**Prerequisites:** none. The instrumented-stream section pairs with
`17_agent_debugging.md`.

This document is authoritative for structured runtime events emitted by the host during one `slice` command.

## Transport

- Default transport: JSON Lines (`.jsonl`) on **stderr**, emitted by every
  `pnp_cli slice` run without any flag. G-code is written to stdout, so the
  event stream is intentionally separated from G-code output to avoid
  interleaving. See `crates/pnp-cli/src/main.rs` and `JsonLinesEmitter` in
  `crates/slicer-runtime/src/progress_events.rs`.
- `--no-progress-events` suppresses the stream entirely (nothing JSONL reaches
  stderr; human-readable warnings are unaffected).
- `--instrument-stderr` emits a strict superset: the core stream plus the
  per-stage / per-module timing events (see Instrumented Stream below).
- Every event is a single JSON object on one line. Human-readable log lines
  may interleave with events on stderr; consumers filter by lines that parse
  as JSON objects with a `schema_version` field.

Buffering requirement:

- Event emission must be non-blocking to per-layer compute threads.
- Implementations should queue events to a dedicated emitter thread/process.

## Event Schema (v1)

```json
{
  "schema_version": "1.3.0",
  "event": "phase_start|phase_complete|layer_start|layer_complete|module_error|validation_error|slice_stats|slice_complete",
  "timestamp_ms": 1735843200123,
  "slice_id": "9f9075ad-2bd8-4e9a-a2f5-3b9055d2f239",
  "phase": "prepass|per_layer|postpass|validation",
  "stage": "Layer::Perimeters",
  "layer_index": 42,
  "module_id": "com.example.perimeters",
  "status": "ok|non_fatal_error|fatal_error",
  "elapsed_ms": 18,
  "degraded": false,
  "error": {
    "code": 12014,
    "message": "feature_flags length mismatch",
    "fatal": true,
    "suggestion": "Verify wall-loop feature flag cardinality",
    "reason": "numerical-edge-ambiguity"
  }
}
```

`error.reason` is optional and additive in schema 1.1.0 (kebab-case). Consumers
parsing schema 1.0.0 must ignore unknown fields and keep working.

Field semantics:

- `timestamp_ms` is Unix epoch time in milliseconds.
- `elapsed_ms` is duration relative to the local event scope (`phase`, `layer`, or module call).
- `stage` is required for `module_error` and recommended for all per-layer events.

## Required Field Matrix (Normative)

| Event              | Required fields                                                                                                 |
|--------------------|-----------------------------------------------------------------------------------------------------------------|
| `phase_start`      | `schema_version,event,timestamp_ms,slice_id,phase,status` (`layer_count` is additive OPTIONAL, present only when `phase=per_layer`; value = total planned layer count) |
| `slice_stats`      | `schema_version,event,timestamp_ms,slice_id,status,gcode_prediction_seconds,gcode_filament_length_mm,layer_count,first_layer_height_mm,extruded_volume_mm3,toolchange_count` (`gcode_weight_grams` OPTIONAL — omitted when `filament_density` absent) |
| `phase_complete`   | `schema_version,event,timestamp_ms,slice_id,phase,status,elapsed_ms`                                            |
| `layer_start`      | `schema_version,event,timestamp_ms,slice_id,phase,layer_index,status`                                           |
| `layer_complete`   | `schema_version,event,timestamp_ms,slice_id,phase,layer_index,status,elapsed_ms,degraded`                       |
| `module_error`     | `schema_version,event,timestamp_ms,slice_id,phase,stage,module_id,status,error` (`layer_index` required only when `phase=per_layer`) |
| `validation_error` | `schema_version,event,timestamp_ms,slice_id,phase,status,error`                                                 |
| `slice_complete`   | `schema_version,event,timestamp_ms,slice_id,status,degraded,elapsed_ms,fatal_error_count,non_fatal_error_count` |

Rules:

- Fields not listed for an event are optional unless otherwise stated.
- `degraded` is required on `layer_complete` and `slice_complete`.
- `error` object is required for `module_error` and `validation_error`.

## Required Events

The host must emit at minimum:

1. `phase_start` and `phase_complete` for `validation`, `prepass`, `per_layer`, `postpass`.
2. `layer_start` and `layer_complete` for every global layer.
3. `module_error` for every module-reported fatal or non-fatal error.
4. `validation_error` for startup validation failures (fatal-only: advisory
   validation diagnostics stay human-readable on stderr).
5. `slice_complete` exactly once **on success**. A fatally-failing slice ends
   its stream at the terminal `module_error`/`validation_error` with no
   `slice_complete`; the abort is signalled by that absence plus the non-zero
   process exit code.

## Determinism Rules

- Event order must be deterministic within a layer (`layer_start` before any module events for that layer, then `layer_complete`).
- For parallel layers, ordering across different `layer_index` values is not guaranteed.
- `slice_complete` must include aggregate fields:
  - `degraded=true` if any non-fatal module error occurred.
  - `fatal_error_count` and `non_fatal_error_count`.

Ordering guarantees:

- Within one `layer_index`, events are strictly ordered:
  - `layer_start`
  - zero or more module-level events
  - `layer_complete`
- `phase_complete` for `per_layer` may only be emitted after all layer-complete events are emitted.

Backpressure behavior:

- If event sink is slower than producer, host must prefer bounded queue + lossless flush-at-end behavior.
- Dropping `module_error` and `slice_complete` events is never allowed.

## Error Visibility Contract

- Non-fatal module failure must never be silent.
- A successful slice with any `non_fatal_error` is considered a degraded success.
- Frontends must surface a warning when `degraded=true`.

## Compatibility

- Additive fields are a minor version bump.
- Renames/removals/type changes are major version bumps.

## Canonical Event Sequences

Normal success (single layer excerpt):

1. `phase_start(validation)`
2. `phase_complete(validation)`
3. `phase_start(prepass)`
4. `phase_complete(prepass)`
5. `phase_start(per_layer)`
6. `layer_start(42)`
7. `layer_complete(42)`
8. `phase_complete(per_layer)`
9. `phase_start(postpass)`
10. `phase_complete(postpass)`
11. `slice_complete(status=ok,degraded=false)`

Degraded success excerpt:

1. `layer_start(42)`
2. `module_error(status=non_fatal_error,fatal=false)`
3. `layer_complete(42,status=non_fatal_error)`
4. `slice_complete(status=ok,degraded=true,non_fatal_error_count>0)`

Fatal failure excerpt:

1. `layer_start(42)`
2. `module_error(status=fatal_error,fatal=true)`
3. (stream ends; process exits non-zero — no `slice_complete` on fatal abort)

Cancellation (`--cancel-on-stdin-eof` or Ctrl+C/Ctrl+Break)
──────────────────────────────────────────────────────────
`layer_start` (Layer N)
  … layer in progress …
`cancelled` { "schema_version": "1.3.0", "event": "cancelled", "timestamp_ms": ..., "slice_id": "..." }
(stream ends; no `slice_complete`; process exits with code 130; `--output` path is absent)

Note: under `--instrument-stderr` the failing module's `module_complete`
timing event precedes its `module_error` (the runtime samples timing before
matching on the dispatch result).

## Schema Version Cadence

- `1.3.0`: New `cancelled` event (added by packet 174) — emitted at most once on the cancel path; never followed by `slice_complete`.
- A stream carries exactly one `schema_version`. Every constructor stamps `PROGRESS_EVENT_SCHEMA_VERSION` (or `PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED`); no event hard-codes a version literal. `slice_stats` did until this was corrected, which put two versions in one stream.

The `schema_version` field follows additive minor bumps:

| Version | Change | Owning packet / task |
|---|---|---|
| `1.0.0` | Baseline 7-event schema (`phase_start`, `phase_complete`, `layer_start`, `layer_complete`, `module_error`, `validation_error`, `slice_complete`). | Phase 0 (T-005) |
| `1.1.0` | `error.reason: Option<String>` kebab-case classification tag added. The `--instrument-stderr` events (`stage_start`, `stage_complete`, `module_start`, `module_complete`, plus `wasm_peak_kb`) also ship at this version — the instrumented stream shares the baseline schema. (`slice_complete.output_path` was once claimed for this row but never shipped in the runtime — the G-code file is written by the CLI after the event fires.) | `pinch_n_print_studio` packet 49 (Phase 3b preview hand-off) |
| `1.2.0` | New `slice_stats` event, emitted exactly once per successful slice (including degraded-but-successful runs that produced G-code), strictly before `slice_complete` (whose production emission now exists, built from `SliceEventCollector` counts). Fields: `gcode_prediction_seconds` (u64), `gcode_weight_grams` (f64, OPTIONAL — key omitted entirely when `filament_density` is absent from config; never `0`/`null`), `gcode_filament_length_mm` (f64), `layer_count` (u32), `first_layer_height_mm` (f32), `extruded_volume_mm3` (map keyed by extruder index, mm³), `toolchange_count` (u32). Deliberately NO cost field: the `pinch_n_print_studio` fork computes cost from its own filament preset, so the runtime never emits one. Also additive: OPTIONAL `layer_count` field on `phase_start`, present only when `phase == per_layer` (value = total planned layer count), omitted for all other phases. | Packet 169 (time-estimator-slice-stats) |

Each row is additive and backward-compatible: a 1.0.0 consumer ignores
`error.reason` and unknown event types, a 1.1.0 consumer ignores
`slice_stats`, etc. Validators must accept any same-major version line.

### Time-Estimator Machine Limits (`slice_stats` inputs)

`gcode_prediction_seconds` is computed by the G-code time estimator, which
reads machine limits from config. All keys are optional (snake_case):
`machine_max_acceleration_extruding`, `machine_max_acceleration_travel`,
`machine_max_speed_x`, `machine_max_speed_y`, `machine_max_speed_z`,
`machine_max_speed_e`, `machine_max_jerk_x`, `machine_max_jerk_y`,
`machine_max_jerk_z`, `machine_max_jerk_e`, plus `filament_density`
(gates `gcode_weight_grams`). When a `machine_max_*` key is absent the
estimator falls back to:

| Limit | Fallback |
|---|---|
| Acceleration (extruding and travel) | 1500 mm/s² |
| Max speed X / Y | 200 mm/s |
| Max speed Z | 12 mm/s |
| Max speed E | 25 mm/s |
| Jerk X / Y | 9 mm/s |
| Jerk Z | 0.2 mm/s |
| Jerk E | 2.5 mm/s |

## Instrumented Stream (`--instrument-stderr`)

Passing `--instrument-stderr` to `pnp_cli slice` additionally emits
per-stage and per-module brackets on the same stderr JSONL stream, at the
same schema version as the core stream — the instrumented stream carries
the same additive payload as the base stream, so
`PROGRESS_EVENT_SCHEMA_VERSION_INSTRUMENTED` always equals
`PROGRESS_EVENT_SCHEMA_VERSION` (currently `"1.3.0"`). New event types
(additive, backward-compatible with consumers that ignore unknown
`event` values):

| Event             | Required fields                                                                                 |
|-------------------|-------------------------------------------------------------------------------------------------|
| `stage_start`     | `schema_version,event,timestamp_ms,slice_id,phase,stage,status`                                 |
| `stage_complete`  | `schema_version,event,timestamp_ms,slice_id,phase,stage,status,elapsed_ms`                      |
| `module_start`    | `schema_version,event,timestamp_ms,slice_id,phase,stage,module_id,status`                       |
| `module_complete` | `schema_version,event,timestamp_ms,slice_id,phase,stage,module_id,status,elapsed_ms,wasm_peak_kb` |

`layer_index` is populated when the stage runs inside the per-layer tier
and omitted otherwise (prepass / postpass stages).

`wasm_peak_kb` is the ceiling of the WASM linear-memory highwater
(`wasm_peak_bytes / 1024`) sampled by the runtime around the module
dispatch. Host built-ins report `0`.

Compatibility: the flag is fully composable with `--report` so an agent
can stream live timing to a file while also producing the HTML report. For the
agent-facing workflow, see `17_agent_debugging.md`.

Example excerpt (one prepass stage + one per-layer module):

```jsonl
{"schema_version":"1.3.0","event":"stage_start","timestamp_ms":1735843200125,"slice_id":"slice-1735843200000","phase":"prepass","stage":"PrePass::MeshAnalysis","status":"ok"}
{"schema_version":"1.3.0","event":"module_start","timestamp_ms":1735843200126,"slice_id":"slice-1735843200000","phase":"prepass","stage":"PrePass::MeshAnalysis","module_id":"host:mesh_analysis","status":"ok"}
{"schema_version":"1.3.0","event":"module_complete","timestamp_ms":1735843200450,"slice_id":"slice-1735843200000","phase":"prepass","stage":"PrePass::MeshAnalysis","module_id":"host:mesh_analysis","status":"ok","elapsed_ms":324,"wasm_peak_kb":0}
{"schema_version":"1.3.0","event":"stage_complete","timestamp_ms":1735843200451,"slice_id":"slice-1735843200000","phase":"prepass","stage":"PrePass::MeshAnalysis","status":"ok","elapsed_ms":326}
```

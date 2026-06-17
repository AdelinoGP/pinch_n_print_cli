# Pinch 'n Print — Progress & Error Event Contract

This document is authoritative for structured runtime events emitted by the host during one `slice` command.

## Transport

- Default transport: JSON Lines (`.jsonl`) on **stderr**. G-code is written to
  stdout, so the event stream is intentionally separated from G-code output to
  avoid interleaving. See `crates/pnp-cli/src/main.rs` and
  `JsonLinesEmitter` in `crates/slicer-runtime/src/progress_events.rs`.
- Every event is a single JSON object on one line.
- <!-- VERIFY: an explicit `--log-events <path>` CLI flag is referenced in the
     `progress_events.rs` doc-comment but is not currently exposed by
     `crates/slicer-runtime/src/cli.rs`. Until it ships, events stream only to
     stderr. -->

Buffering requirement:

- Event emission must be non-blocking to per-layer compute threads.
- Implementations should queue events to a dedicated emitter thread/process.

## Event Schema (v1)

```json
{
  "schema_version": "1.1.0",
  "event": "phase_start|phase_complete|layer_start|layer_complete|module_error|validation_error|slice_complete",
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
| `phase_start`      | `schema_version,event,timestamp_ms,slice_id,phase,status`                                                       |
| `phase_complete`   | `schema_version,event,timestamp_ms,slice_id,phase,status,elapsed_ms`                                            |
| `layer_start`      | `schema_version,event,timestamp_ms,slice_id,phase,layer_index,status`                                           |
| `layer_complete`   | `schema_version,event,timestamp_ms,slice_id,phase,layer_index,status,elapsed_ms,degraded`                       |
| `module_error`     | `schema_version,event,timestamp_ms,slice_id,phase,stage,layer_index,module_id,status,error`                     |
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
4. `validation_error` for startup validation failures.
5. `slice_complete` exactly once.

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
3. `slice_complete(status=fatal_error,degraded=false,fatal_error_count>0)`

## Schema Version Cadence

The `schema_version` field follows additive minor bumps:

| Version | Change | Owning packet / task |
|---|---|---|
| `1.0.0` | Baseline 7-event schema (`phase_start`, `phase_complete`, `layer_start`, `layer_complete`, `module_error`, `validation_error`, `slice_complete`). | Phase 0 (T-005) |
| `1.1.0` | `slice_complete.output_path: Option<PathBuf>` added (packet 49); `error.reason: Option<String>` kebab-case classification tag added (earlier additive). Both are observable in the example schema above. | `pinch_n_print_studio` packet 49 (Phase 3b preview hand-off) |
| `1.2.0` | New `slice_stats` event emitted before `slice_complete`, carrying `gcode_prediction_seconds`, `gcode_weight_grams`, `gcode_filament_length_mm`, `layer_count`, `first_layer_height_mm`. | Reserved for `pinch_n_print_studio` T-096 (SliceStats Event Wiring) — coordinated backend PR |
| `1.3.0` | `--instrument-stderr` stream — adds `stage_start`, `stage_complete`, `module_start`, `module_complete` events on the same stderr JSONL. | (future) |

Each row is additive and backward-compatible: a 1.0.0 consumer ignores
`output_path`, a 1.1.0 consumer ignores `slice_stats`, etc. Validators
must accept any same-major version line.

## Instrumented Stream (`--instrument-stderr`)

Passing `--instrument-stderr` to `pnp_cli slice` bumps the schema version
to `"1.3.0"` (see Schema Version Cadence above; 1.1.0 and 1.2.0 are
reserved for additive payload fields) and additionally emits per-stage
and per-module brackets on the same stderr JSONL stream. New event types
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

Compatibility: events emitted without `--instrument-stderr` report the
highest baseline schema version (see Schema Version Cadence). The flag
is fully composable with `--report` so an agent can stream live timing
to a file while also producing the HTML report (see
`docs/specs/agent-cli-debugging.md` §4.2).

Example excerpt (one prepass stage + one per-layer module):

```jsonl
{"schema_version":"1.3.0","event":"stage_start","timestamp_ms":1735843200125,"slice_id":"slice-1735843200000","phase":"prepass","stage":"PrePass::MeshAnalysis","status":"ok"}
{"schema_version":"1.3.0","event":"module_start","timestamp_ms":1735843200126,"slice_id":"slice-1735843200000","phase":"prepass","stage":"PrePass::MeshAnalysis","module_id":"host::mesh_analysis","status":"ok"}
{"schema_version":"1.3.0","event":"module_complete","timestamp_ms":1735843200450,"slice_id":"slice-1735843200000","phase":"prepass","stage":"PrePass::MeshAnalysis","module_id":"host::mesh_analysis","status":"ok","elapsed_ms":324,"wasm_peak_kb":0}
{"schema_version":"1.3.0","event":"stage_complete","timestamp_ms":1735843200451,"slice_id":"slice-1735843200000","phase":"prepass","stage":"PrePass::MeshAnalysis","status":"ok","elapsed_ms":326}
```

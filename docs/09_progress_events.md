# ModularSlicer — Progress & Error Event Contract

This document is authoritative for structured runtime events emitted by the host during one `slice` command.

## Transport

- Default transport: JSON Lines (`.jsonl`) on stdout.
- Optional transport: explicit event file via `--log-events <path>`.
- Every event is a single JSON object on one line.

Buffering requirement:
- Event emission must be non-blocking to per-layer compute threads.
- Implementations should queue events to a dedicated emitter thread/process.

## Event Schema (v1)

```json
{
  "schema_version": "1.0.0",
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
    "suggestion": "Verify wall-loop feature flag cardinality"
  }
}
```

Field semantics:
- `timestamp_ms` is Unix epoch time in milliseconds.
- `elapsed_ms` is duration relative to the local event scope (`phase`, `layer`, or module call).
- `stage` is required for `module_error` and recommended for all per-layer events.

## Required Field Matrix (Normative)

| Event | Required fields |
|---|---|
| `phase_start` | `schema_version,event,timestamp_ms,slice_id,phase,status` |
| `phase_complete` | `schema_version,event,timestamp_ms,slice_id,phase,status,elapsed_ms` |
| `layer_start` | `schema_version,event,timestamp_ms,slice_id,phase,layer_index,status` |
| `layer_complete` | `schema_version,event,timestamp_ms,slice_id,phase,layer_index,status,elapsed_ms,degraded` |
| `module_error` | `schema_version,event,timestamp_ms,slice_id,phase,stage,layer_index,module_id,status,error` |
| `validation_error` | `schema_version,event,timestamp_ms,slice_id,phase,status,error` |
| `slice_complete` | `schema_version,event,timestamp_ms,slice_id,status,degraded,elapsed_ms,fatal_error_count,non_fatal_error_count` |

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

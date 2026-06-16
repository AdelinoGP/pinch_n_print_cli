# Agent CLI Debugging — stderr JSONL + DAG Introspection

Status: draft
Date: 2026-05-26
Owner: see CODEOWNERS for `docs/`

## 1. Summary

LLM agents debugging the slicer pipeline need two capabilities: live per-module/per-stage
timing during a slice, and static DAG introspection to understand module wiring. Today
the stderr JSONL stream only carries phase-level and layer-level timing; per-module and
per-stage timing is only available post-hoc in the HTML `--report`. DAG structure
(serial edges, claim assignments, IR access masks) has no query surface at all.

This spec adds both capabilities to the `slicer-host` binary as **zero-dependency CLI
extensions**: new event types on the existing stderr JSONL stream, and new `dag` /
`diagnose` subcommands that output JSON. No pipeline-side changes, no new crate
dependencies, and backward-compatible with the existing event contract.

## 2. Motivation

Agentic debugging workflows follow a stateless tool-call pattern: each invocation is a
subprocess that returns structured output, and the agent's context window holds the
correlated state. CLI subcommands map perfectly to this; persistent MCP servers do not.

| Debugging need | Current state | Proposed |
|---|---|---|
| "Which module is slow right now?" | Only HTML report, post-hoc | `ModuleComplete` on stderr with `elapsed_ms` |
| "Is WASM memory leaking?" | Only HTML report, post-hoc | `ModuleComplete` with `wasm_peak_kb` |
| "Why is stage X serialized to stage Y?" | No query surface | `slicer-host dag stage <id>` |
| "What depends on cubic_infill?" | No query surface | `slicer-host dag depends <id>` |
| "Are these manifests valid?" | Only `Run` exit code / stderr text | `slicer-host diagnose` (structured JSON) |
| "What stages exist in this module set?" | No query surface | `slicer-host dag stages` |

## 3. Non-goals

- **No MCP server.** The CLI additions are a standalone feature; if an MCP server is
  built later, it wraps these same CLI subcommands.
- **No pipeline changes.** The `PipelineInstrumentation` trait already fires at every
  boundary; this spec adds a new consumer of that trait.
- **No live pause/resume/hot-swap.** Phase 2 (future) may add control-plane operations;
  this spec is observation-only.
- **No breaking changes.** Existing stderr events (`PhaseStart`, `LayerComplete`, etc.)
  are unchanged. New events are additive; consumers that ignore unknown event types
  remain compatible.

## 4. Design

### 4.1 Architecture

```
┌──────────────────────┐
│  slicer-host run --instrument-stderr                  │
│                                 │
│  PipelineInstrumentation trait (unchanged)            │
│  ├─ Collector (--report, unchanged)                   │
│  └─ ProgressPipelineInstrumentation ◄── NEW          │
│       │                                              │
│       └► RuntimeProgressSink                         │
│            ├─ JsonLinesEmitter(stderr) ◄── new events│
│            └─ SliceEventCollector (unchanged)        │
└──────────────────────┘

┌──────────────────────┐
│  slicer-host dag stages|stage|depends|claims          │
│                                 │
│  Load manifests → build DAG → emit JSON to stdout     │
│  (No slicing, no WASM execution)                     │
└──────────────────────┘

┌──────────────────────┐
│  slicer-host diagnose                                 │
│                                 │
│  Load manifests → validate DAG → emit JSON to stdout  │
│  (Collects all LoadDiagnostic entries)               │
└──────────────────────┘
```

### 4.2 Enhanced stderr events

Four new `ProgressEventType` variants. All use the existing `ProgressEvent` struct
fields — the struct already carries `stage`, `module_id`, `layer_index`, `phase`,
and `elapsed_ms` as optional fields.

| Event type | Fields populated | Emitted when |
|---|---|---|
| `stage_start` | phase, stage, [layer_index] | `on_stage_start` |
| `stage_complete` | phase, stage, [layer_index], elapsed_ms | `on_stage_end` |
| `module_start` | phase, stage, module_id, [layer_index] | `on_module_start` |
| `module_complete` | phase, stage, module_id, [layer_index], elapsed_ms, wasm_peak_kb | `on_module_end` |

The `wasm_peak_kb: Option<u64>` field is new on `ProgressEvent`. Populated only on
`module_complete`, derived from `PipelineInstrumentation::on_module_end`'s
`wasm_peak_bytes` argument (converted to KiB, rounded up).

Events emitted under `--instrument-stderr` **replace** the old phase and layer events
for that execution, avoiding duplicates. The `ProgressPipelineInstrumentation`
adapter emits `PhaseStart`, `PhaseComplete`, `LayerStart`, `LayerComplete` events
itself, superseding the events that `run_pipeline_with_events` would have emitted.

**Schema version:** bumped to `"1.1.0"` when `--instrument-stderr` is active.
Backward-compatible: consumers that check `schema_version` can distinguish;
consumers that ignore unknown event types are unaffected.

**Example output with `--instrument-stderr`:**

```jsonl
{"schema_version":"1.1.0","event":"phase_start","timestamp_ms":1735843200123,"slice_id":"9f9...","phase":"prepass","status":"ok"}
{"schema_version":"1.1.0","event":"stage_start","timestamp_ms":1735843200125,"slice_id":"9f9...","phase":"prepass","stage":"MeshAnalysis","status":"ok"}
{"schema_version":"1.1.0","event":"module_start","timestamp_ms":1735843200126,"slice_id":"9f9...","phase":"prepass","stage":"MeshAnalysis","module_id":"host::mesh_analysis","status":"ok"}
{"schema_version":"1.1.0","event":"module_complete","timestamp_ms":1735843200450,"slice_id":"9f9...","phase":"prepass","stage":"MeshAnalysis","module_id":"host::mesh_analysis","status":"ok","elapsed_ms":324,"wasm_peak_kb":0}
{"schema_version":"1.1.0","event":"stage_complete","timestamp_ms":1735843200451,"slice_id":"9f9...","phase":"prepass","stage":"MeshAnalysis","status":"ok","elapsed_ms":326}
{"schema_version":"1.1.0","event":"stage_start","timestamp_ms":1735843200452,"slice_id":"9f9...","phase":"per_layer","layer_index":0,"stage":"Layer::Perimeters","status":"ok"}
{"schema_version":"1.1.0","event":"module_start","timestamp_ms":1735843200453,"slice_id":"9f9...","phase":"per_layer","layer_index":0,"stage":"Layer::Perimeters","module_id":"com.example.perimeters","status":"ok"}
{"schema_version":"1.1.0","event":"module_complete","timestamp_ms":1735843205400,"slice_id":"9f9...","phase":"per_layer","layer_index":0,"stage":"Layer::Perimeters","module_id":"com.example.perimeters","status":"ok","elapsed_ms":4947,"wasm_peak_kb":2048}
{"schema_version":"1.1.0","event":"stage_complete","timestamp_ms":1735843205401,"slice_id":"9f9...","phase":"per_layer","layer_index":0,"stage":"Layer::Perimeters","status":"ok","elapsed_ms":4949}
{"schema_version":"1.1.0","event":"layer_complete","timestamp_ms":1735843205800,"slice_id":"9f9...","phase":"per_layer","layer_index":0,"status":"ok","elapsed_ms":5348}
```

### 4.3 DAG introspection subcommands

All `dag` subcommands call `load_modules_from_roots()` — pure manifest TOML parsing
only. They never enter `load_live_modules_for_plan()` and never compile WASM. DAG
construction (`build_intra_stage_dag`, the new `build_global_dag` for `dag depends`)
operates entirely on `LoadedModule` metadata fields. Responses are sub-100ms
regardless of module count.

**Shared CLI args:**

```
slicer-host dag <subcommand>
    --module-dir <DIR> [--module-dir <DIR> ...] [--no-default-module-paths]
    [--model <PATH>]
```

`--model <PATH>` is optional. When provided, the model file is loaded to extract
object IDs and per-object config overrides from 3MF sidecar data. This metadata is
included in the JSON output for context (e.g., `object_ids` field) but does **not**
filter the module set — all discovered modules appear in the DAG regardless of model.
When omitted, output covers the full static DAG across all loaded manifests.

#### `dag stages`

Lists every stage with its tier, module count, and claim count.

```json
{
  "stages": [
    {"id": "MeshSegmentation", "tier": "prepass", "module_count": 1, "claim_count": 0},
    {"id": "MeshAnalysis", "tier": "prepass", "module_count": 1, "claim_count": 0},
    {"id": "Layer::Perimeters", "tier": "per_layer", "module_count": 3, "claim_count": 3},
    {"id": "Layer::Infill", "tier": "per_layer", "module_count": 5, "claim_count": 4},
    {"id": "LayerFinalization", "tier": "layer_finalization", "module_count": 1, "claim_count": 0},
    {"id": "GCodeEmit", "tier": "postpass", "module_count": 1, "claim_count": 0}
  ]
}
```

#### `dag stage <id>`

Full detail for one stage: every module with its claims, IR access masks, required
modules, and config keys, plus the serial edges between them.

```json
{
  "id": "Layer::Infill",
  "tier": "per_layer",
  "modules": [
    {
      "id": "com.example.cubic_infill",
      "claims": ["claim:sparse"],
      "ir_reads": ["SliceIR.layers[].perimeters", "LayerPlanIR.regions[].walls"],
      "ir_writes": ["InfillIR.regions[].paths"],
      "requires_modules": [],
      "config_keys": ["density", "spacing", "angle", "pattern", "bridge_angle"]
    },
    {
      "id": "com.example.gyroid_infill",
      "claims": ["claim:sparse"],
      "ir_reads": ["SliceIR.layers[].perimeters"],
      "ir_writes": ["InfillIR.regions[].paths"],
      "requires_modules": [],
      "config_keys": ["density", "spacing_min", "spacing_max"]
    },
    {
      "id": "host::paint_region_annotation",
      "claims": [],
      "ir_reads": [],
      "ir_writes": [],
      "requires_modules": [],
      "config_keys": []
    }
  ],
  "serial_edges": [
    {"from": "com.example.cubic_infill", "to": "com.example.gyroid_infill", "reason": "ir_write_read: InfillIR.regions[].paths"},
    {"from": "host::paint_region_annotation", "to": "com.example.cubic_infill", "reason": "explicit_requires"}
  ]
}
```

#### `dag depends <module-id>`

Upstream and downstream edges for a single module, **across all stages** (global DAG).
Uses a new `build_global_dag()` function that applies the same `IrWriteRead` +
`ExplicitRequires` edge rules to every module from every stage simultaneously —
identical logic to `build_intra_stage_dag` but without the stage filter. Each edge
includes `from_stage` / `to_stage` so the agent can see where stage boundaries are
crossed.

```json
{
  "module_id": "com.example.cubic_infill",
  "object_ids": ["benchy"],
  "upstream": [
    {"from": "com.example.perimeter_generator", "from_stage": "Layer::Perimeters", "to": "com.example.cubic_infill", "to_stage": "Layer::Infill", "reason": "ir_write_read: LayerIR.regions[].shells"}
  ],
  "downstream": [
    {"from": "com.example.cubic_infill", "from_stage": "Layer::Infill", "to": "com.example.gyroid_infill", "to_stage": "Layer::Infill", "reason": "ir_write_read: InfillIR.regions[].paths"},
    {"from": "com.example.cubic_infill", "from_stage": "Layer::Infill", "to": "com.example.gcode_postprocess", "to_stage": "PostPass::GCodePostProcess", "reason": "ir_write_read: InfillIR.regions[].paths"}
  ]
}
```

`object_ids` is present only when `--model` is passed.

#### `dag claims`

Every claim with its holders and requesters.

```json
{
  "claims": [
    {
      "id": "claim:sparse",
      "holders": ["com.example.cubic_infill", "com.example.gyroid_infill"],
      "requesters": [],
      "interchangeable": true
    },
    {
      "id": "perimeter-generator",
      "holders": ["com.example.perimeter_generator"],
      "requesters": ["com.example.seam_planner"],
      "interchangeable": false
    }
  ]
}
```

### 4.4 Diagnose subcommand

Wraps the existing manifest-loading and DAG-validation pipeline, collecting all
`LoadDiagnostic` entries into structured JSON output.

```
slicer-host diagnose --module-dir ./modules [--module-dir ...] [--no-default-module-paths]
```

```json
{
  "pass": false,
  "modules_loaded": 12,
  "stages": 8,
  "diagnostics": [
    {
      "level": "error",
      "file": "com.example.cubic_infill.toml",
      "field": "ir_writes[0]",
      "message": "IR path 'InfillIR.regions[].paths' conflicts with module 'com.example.gyroid_infill' in stage 'Layer::Infill'"
    },
    {
      "level": "warning",
      "file": null,
      "field": null,
      "message": "Module 'com.example.draft_perimeters' declares claim 'perimeter-generator' but 'com.example.perimeter_generator' also holds it — only one claim-holder will be selected per region"
    }
  ]
}
```

Exit code: 0 on pass, 1 on errors, 2 on unreadable files.

## 5. Type-level changes

### 5.1 `ProgressEventType` enum (additive)

```rust
pub enum ProgressEventType {
    // existing variants — unchanged
    PhaseStart,
    PhaseComplete,
    LayerStart,
    LayerComplete,
    ModuleError,
    ValidationError,
    SliceComplete,
    // new variants
    StageStart,
    StageComplete,
    ModuleStart,
    ModuleComplete,
}
```

### 5.2 `ProgressEvent` struct (additive)

Add one field:

```rust
pub struct ProgressEvent {
    // ... all existing fields unchanged ...
    /// Peak WASM linear memory during this module call, in KiB.
    /// Populated only on ModuleComplete events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_peak_kb: Option<u64>,
}
```

### 5.3 Serialize derives (additive, zero behavioral change)

| Type | File |
|---|---|
| `IrAccessMask` | `crates/slicer-host/src/execution_plan.rs` |
| `SerialEdge` | `crates/slicer-host/src/instrumentation.rs` |
| `EdgeReason` | `crates/slicer-host/src/instrumentation.rs` |

### 5.4 New accessor on `CompiledModule`

```rust
impl CompiledModule {
    /// Config keys this module declared in its manifest.
    pub fn config_keys(&self) -> Vec<String>;
}
```

## 6. File changes (complete list)

| File | Action | Est. LOC |
|---|---|---|
| `crates/slicer-host/src/progress_events.rs` | Edit: new event types, `wasm_peak_kb` field, bump schema version constant | +40 |
| `crates/slicer-host/src/progress_instrumentation.rs` | **New**: `ProgressPipelineInstrumentation` adapter | +130 |
| `crates/slicer-host/src/dag_cli.rs` | **New**: `dag` subcommand impl + `build_global_dag()` + JSON output types | +290 |
| `crates/slicer-host/src/cli.rs` | Edit: `Dag` command group with `--model`, `Diagnose` command, `--instrument-stderr` flag on `Run` | +95 |
| `crates/slicer-host/src/main.rs` | Edit: wire new subcommands, wire `ProgressPipelineInstrumentation`, model loading for `dag` | +80 |
| `crates/slicer-host/src/execution_plan.rs` | Edit: `Serialize` on `IrAccessMask`, `config_keys()` accessor on `CompiledModule` | +15 |
| `crates/slicer-host/src/instrumentation.rs` | Edit: `Serialize` on `SerialEdge`, `EdgeReason` | +5 |
| `crates/slicer-host/src/lib.rs` | Edit: re-export `dag_cli` types if needed | +5 |
| `crates/slicer-host/src/manifest.rs` | Edit: expose `LoadDiagnostic` fields for `Diagnose` subcommand, add `Serialize` to diagnostics types | +15 |
| `crates/slicer-host/tests/dag_cli_integration.rs` | **New**: integration tests for `dag` subcommands | +100 |
| `docs/09_progress_events.md` | Edit: document new event types + `wasm_peak_kb` | +40 |
| `docs/17_agent_debugging.md` | **New**: agent debugging guide | +90 |
| `.agents/skills/debug-pipeline/SKILL.md` | **New**: pipeline debugging skill | +70 |
| `.opencode/agents/debug-pipeline/config.toml` | **New**: subagent config | +25 |
| `CLAUDE.md` | Edit: reference new doc + skill | +5 |
| **Total** | | **~995** |

## 7. Implementation order

### Chunk 1 — Enhanced stderr events (~175 LOC)

1. Add `StageStart`, `StageComplete`, `ModuleStart`, `ModuleComplete` variants to
   `ProgressEventType`.
2. Add `wasm_peak_kb` field to `ProgressEvent`.
3. Bump schema version constant to `"1.1.0"`.
4. Create `progress_instrumentation.rs` with `ProgressPipelineInstrumentation` adapter.
5. Add `--instrument-stderr` flag to `Run` in `cli.rs`.
6. Wire adapter in `main.rs`: when `--instrument-stderr`, use
   `run_pipeline_with_instrumentation` with both `ProgressPipelineInstrumentation`
   and (optionally) `Collector`.

**Verification:**
```bash
cargo check -p slicer-host
cargo test -p slicer-host --lib -- progress_events
cargo test -p slicer-host --lib -- progress_instrumentation
```

### Chunk 2 — DAG introspection subcommands (~490 LOC)

1. Add `Serialize` derives to `IrAccessMask`, `SerialEdge`, `EdgeReason`.
2. Add `config_keys()` accessor to `CompiledModule`.
3. Add `build_global_dag()` function — same edge rules as `build_intra_stage_dag`
   but applied across all stages (union of all `LoadedModule`s, no stage filter).
4. Create `dag_cli.rs` — JSON output structs + `run_dag_stages()`, `run_dag_stage()`,
   `run_dag_depends()` (uses `build_global_dag`), `run_dag_claims()`.
5. Add `Dag` subcommand group to `HostCommands` in `cli.rs` with `--module-dir` and
   optional `--model` flag.
6. Wire in `main.rs`: `dag` subcommands use `load_modules_from_roots()` directly
   (no WASM compilation). When `--model` is provided, load the mesh for object IDs
   and per-object config context.

**Verification:**
```bash
cargo check -p slicer-host
cargo test -p slicer-host --tests -- dag_cli
```

### Chunk 3 — Diagnose subcommand (~100 LOC)

1. Expose `LoadDiagnostic` fields publicly (or add `Serialize` + accessors).
2. Add `Diagnose` variant to `HostCommands`.
3. Implement `run_diagnose()`: load manifests, run DAG validation, collect diagnostics,
   emit JSON.
4. Wire in `main.rs`.

**Verification:**
```bash
cargo check -p slicer-host
cargo test -p slicer-host --tests -- diagnose
```

### Chunk 4 — Documentation and agent tooling (~230 LOC)

1. Update `docs/09_progress_events.md` with new event types.
2. Create `docs/17_agent_debugging.md`.
3. Create `.agents/skills/debug-pipeline/SKILL.md`.
4. Create `.opencode/agents/debug-pipeline/config.toml`.
5. Update `CLAUDE.md`.

**Verification:** manual review.

## 8. Test strategy

### 8.1 Unit tests (`crates/slicer-host/src/progress_events.rs` tests)

- `ProgressPipelineInstrumentation` emits `StageStart` with correct `stage_id` and `phase`
  when `on_stage_start` is called.
- Emits `ModuleComplete` with `elapsed_ms > 0` and `wasm_peak_kb` populated.
- `wasm_peak_kb` rounds correctly from `wasm_peak_bytes` (ceil to KiB).
- Elapsed time is computed from monotonic clock and non-negative.
- Non-compiled host built-in produces `wasm_peak_kb: 0`.

### 8.2 Integration tests (new file: `crates/slicer-host/tests/dag_cli_integration.rs`)

Uses existing test-guest modules from `test-guests/`. Tests the actual CLI binary
via `std::process::Command`.

- `dag stages` on `test-guests/` module set returns expected stage count.
- `dag stages` on empty dir returns empty list.
- `dag stage "Layer::Infill"` returns correct `serial_edges` with reason strings.
- `dag depends "com.example.cubic_infill"` with test modules returns correct upstream/downstream counts.
- `dag claims` returns claim holders matching expected assigns from test manifests.

### 8.3 Compile gate

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
```

### 8.4 Freshness check (WASM staleness)

```bash
cargo xtask build-guests --check
```

## 9. Backward compatibility

| Concern | Resolution |
|---|---|
| Existing consumers of stderr JSONL (IDE frontends, log parsers) | New event types are additive. Consumers that match on `event` field and ignore unknown types are unaffected. Existing events are unchanged. |
| `schema_version` bump to `"1.1.0"` | Only emitted when `--instrument-stderr` is passed. Default path without the flag emits `"1.0.0"` as before. |
| `ProgressEvent` struct field addition | `wasm_peak_kb` uses `#[serde(skip_serializing_if = "Option::is_none")]` — absent from all existing event types. No existing consumer will see it. |
| `--report` path | Unchanged. `Collector` and `ProgressPipelineInstrumentation` both implement `PipelineInstrumentation` and can coexist. |

## 10. Risks

| Risk | Mitigation |
|---|---|
| `--instrument-stderr` balloons stderr volume for large prints (220 layers × 8 stages × 3 modules ≈ 10k events/lines) | Acceptable. A 220-layer Benchy is ~10K JSONL lines at ~200 bytes/line ≈ 2 MB. Agents can `tail -f` or stream-read. |
| `dag` subcommands use `load_modules_from_roots()` — confirms zero WASM compilation, sub-100ms responses | No risk. Manifests are plain TOML files. `load_modules_from_roots()` scans and parses them; it never touches `wasmtime`. Verification: the existing `ConfigSchema` subcommand already uses this path. |
| `dag` output may be large with 50+ modules | Acceptable. A 50-module stage produces ~200 lines of JSON. Agents handle this trivially. |
| `diagnose` subcommand's diagnostic messages are currently human-readable strings, not machine-readable codes | Acceptable. Agents parse natural language well. Machine-readable error codes can be added later without breaking the JSON schema. |
| Serialization of `IrAccessMask.paths` exposes internal IR field paths | These paths are already visible in module manifests (TOML) and in the `--report` HTML. No new information is exposed. |
| `--model` on `dag` subcommands adds mesh-loading latency | Acceptable. `--model` is optional; agents use it only when object-specific context is needed. A Benchy-sized STL parses in single-digit ms. |

## 11. Resolved decisions

| Question | Decision | Rationale |
|---|---|---|
| Flag name for enhanced stderr on `Run` | `--instrument-stderr` | Technically accurate — it instruments the pipeline and writes to stderr. Clearer than `--verbose-events` (generic). |
| WASM compilation for `dag` subcommands | **None.** Use `load_modules_from_roots()` directly. | Traced the code: `load_modules_from_roots()` stops at manifest TOML parsing. `load_live_modules_for_plan()` is the function that compiles WASM — `dag` subcommands never call it. `build_intra_stage_dag` and `compute_serial_edges_for_stage` operate on `LoadedModule` metadata only (no WASM types). |
| `dag depends` scope | **Cross-stage (global DAG).** New `build_global_dag()` function. | Intra-stage only misses dependencies like "PostPass::GCodePostProcess reads InfillIR written by Layer::Infill". Cross-stage is straightforward to build — same edge rules without the stage filter. |

## 12. Open questions

1. **`Diagnose` output format.** Currently proposes a simple `pass, modules_loaded,
   stages, diagnostics}` struct. Should it include the full DAG structure inline for
   correlated debugging? Recommendation: no. That's what `dag stages` / `dag stage` are
   for. `diagnose` stays focused on errors/warnings.

## 13. Agent workflow example

```
Agent: slicer-host run --instrument-stderr --model benchy.stl --module-dir ./modules
       --output /tmp/out.gcode 2> /tmp/events.jsonl &
       tail -f /tmp/events.jsonl

Agent: [observes] module_complete: cubic_infill, layer=7, elapsed_ms=18000

Agent: That's 18s on layer 7. Median across layers is 0.9s. Let me check the DAG.

Agent: slicer-host dag depends cubic_infill --module-dir ./modules
       → downstream: ["cubic_infill" → "infill_postprocess"]
       → upstream:  ["perimeter_generator" → "cubic_infill"]

Agent: No unexpected dependency chain. Let me check the config.

Agent: slicer-host dag stage Layer::Infill --module-dir ./modules
       → cubic_infill config_keys: ["density", "spacing", "angle", "pattern"]

Agent: Config looks standard. The stall is likely WASM-level — the module is
       compute-bound on layer 7's geometry. Not a DAG issue, not a memory leak
       (wasm_peak_kb is stable at 2048 across all layers).
```

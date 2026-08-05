---
status: implemented
packet: non-planar-z-envelope
task_ids:
  - TASK-127
---

# 05_non-planar-z-envelope

## Goal

Enforce the non-planar Z envelope `[layer.z, layer.z + effective_layer_height]` at per-layer output-commit boundaries (Tier 2), treating violations as fatal contract errors. Covers DEV-005.

## Problem Statement

DEV-005 documents that the non-planar Z envelope `[layer.z, layer.z + effective_layer_height]` is not enforced at output-commit boundaries. Per-layer modules that emit paths with Z outside this envelope produce physically impossible output (extrusions at invalid print heights). Violations must be caught as fatal contract errors at the WIT boundary, not silently accepted.

The envelope rule is documented in `docs/01_system_architecture.md` (Non-Planar Z Envelope Rules, lines 260-268) but no runtime check exists. The gap is that any per-layer module can push geometry with arbitrary Z and the host accepts it without error.

This is a coherent slice: it adds one validation gate at every per-layer output-commit path, using the layer metadata already available at dispatch time.

## Architecture Constraints

- Z envelope validation must happen inside the WIT boundary, not at the IR commit stage, to catch violations as early as possible and before any state is mutated
- `HostExecutionContext` is created per-call and already carries per-call state; it is the natural place to store the envelope bounds
- The envelope check must be in the `push_*` methods that accept `ExtrusionPath3d` or `Point3`, not deferred to a later commit step, because those methods are the actual commit boundary for Tier 2
- Fatal errors from `push_*` methods are already handled as `Result<Result<(), String>, wasmtime::Error>` — the inner `Err(String)` maps to the module error message; this is consistent with the existing error handling pattern
- Catch-up layer adjustment: when `is_catchup_layer = true`, the lower bound is `catchup_z_bottom` (not `layer.z`), but the upper bound remains `catchup_z_bottom + effective_layer_height` (same H, just shifted down)

## Data and Contract Notes

- IR or manifest contracts touched: None — envelope enforcement is purely a runtime check at the WIT boundary, not a change to any IR schema or manifest schema
- WIT boundary considerations: The `push_*` methods already return `Result<Result<(), String>, wasmtime::Error>`. The inner `Err(String)` is the module error message surfaced to the progress event system. The envelope violation will be expressed as `Err("Z_ENVELOPE_VIOLATION: Z {z} below layer.z floor {floor}")`. This is consistent with how other fatal contract errors (undeclared reads/writes) are already expressed in the codebase.
- Determinism or scheduler constraints: The check is pure (no side effects, deterministic given the same z value and envelope bounds). Adding it to `push_*` does not affect scheduling or ordering.

## Locked Assumptions and Invariants

- `layer.z < layer.z + effective_layer_height` always holds (envelope always has positive height)
- For catch-up layers: `catchup_z_bottom < layer.z` (catch-up layer's bottom is below the normal layer Z because it skipped a gap)
- `effective_layer_height > 0` always holds
- All Z values in an `ExtrusionPath3d` have the same Z (extrusion paths are planar at one Z height); the envelope check only needs to inspect one Z value per path, not every vertex

## Risks and Tradeoffs

- **Risk**: Adding envelope parameters to `HostExecutionContext::new` changes the call signature, requiring updates to all call sites (prepass dispatch, postpass dispatch, layer dispatch). Mitigation: grep all `HostExecutionContext::new` call sites and update each with the appropriate layer parameters.
- **Risk**: The catch-up layer condition (`is_catchup_layer`) needs to be plumbed from `GlobalLayer` through the dispatch path into `HostExecutionContext`. If `GlobalLayer` does not yet have `catchup_z_bottom` populated reliably for all layers, this needs investigation first. Mitigation: confirm in IR schema (`docs/02_ir_schemas.md` lines 276-278) that `is_catchup_layer` and `catchup_z_bottom` are populated by PrePass.
- **Tradeoff**: The envelope check adds a float comparison to every `push_*` call. This is negligible overhead compared to WASM boundary crossing.

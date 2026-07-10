---
status: implemented
packet: skirt-brim-finalization-live-path
task_ids:
  - TASK-142
---

# 16_skirt-brim-finalization-live-path

## Goal

Port `SkirtBrim` live geometry from the legacy `process(&mut Vec<LayerCollectionIR>)` path into `FinalizationModule::run_finalization()` using `LayerCollectionView` inputs and `FinalizationOutputBuilder` output, so the live host finalization path no longer depends on the legacy vector-mutation surface.

## Problem Statement

`SkirtBrim` already has real geometry logic, but the live host finalization path still calls a default `run_finalization()` that does nothing. This leaves finalization behavior split between the documented SDK authoring surface and a legacy direct vector-mutation helper. The packet fixes that gap by porting the existing geometry logic onto the canonical finalization API without mixing in wipe tower or travel work.

## Architecture Constraints

- Selected approach: port the existing bbox and loop-generation helpers unchanged where possible, and swap only the input/output surface from `&mut Vec<LayerCollectionIR>` to `LayerCollectionView` plus `FinalizationOutputBuilder`.
- The live host path must not depend on calling the legacy `process()` helper after the port lands.
- The packet must stay on `push_entity_to_layer()` for skirt and brim geometry; synthetic layers are out of scope here.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionView.layer_index()`, `z()`, and `ordered_entities()`
  - `FinalizationOutputBuilder.push_entity_to_layer()`
  - `ExtrusionRole::Skirt`
  - `RegionKey.object_id = "__skirt__"` and `"__brim__"`
- WIT boundary considerations:
  - no WIT widening is expected; the packet uses the existing world-finalization surface
- Determinism or scheduler constraints:
  - emitted skirt/brim pushes must remain deterministic for the same input layer set and config

## Locked Assumptions and Invariants

- Skirt and brim stay on existing layers, not synthetic layers.
- The geometry formulas already validated on `process()` are the source of truth for the port.

## Risks and Tradeoffs

- Risk: bbox discovery through `LayerCollectionView` can diverge from the legacy vector path. Mitigation: reuse the same geometric helpers and assert exact layer targets.
- Risk: the host may still call the legacy helper after the port. Mitigation: keep a host integration regression that proves merge-back from finalization output.
- Risk: the current host entity-push merge path (`WasmRuntimeDispatcher::run_stage`,
  `dispatch.rs:2250-2296`) appends finalization entity pushes via `ordered_entities.push()`.
  This inverts the legacy ordering where skirt and brim appear **before** model entities.
  Mitigation: Step 3 must update the merge path to batch-prepend finalization entity pushes
  (e.g., collect all pushes for a layer then `ordered_entities.splice(0..0, pushes)` or
  `Vec::extend` after reversing the collected set) so AC-4's "prepended" assertion holds.

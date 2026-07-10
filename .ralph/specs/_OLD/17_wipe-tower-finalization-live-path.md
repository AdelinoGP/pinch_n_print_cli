---
status: implemented
packet: wipe-tower-finalization-live-path
task_ids:
  - TASK-143
---

# 17_wipe-tower-finalization-live-path

## Goal

Port `WipeTower` live geometry from the legacy `process(&mut Vec<LayerCollectionIR>)` path into `FinalizationModule::run_finalization()` and retire the legacy-only finalization dependency on the live host path.

## Problem Statement

`WipeTower` mirrors the same DEV-013 gap as `SkirtBrim`: real geometry exists on a legacy vector-mutation helper while the documented `run_finalization()` path is a no-op. This packet ports the existing wipe-tower logic onto the canonical finalization API and adds host integration proof so the live path can stop depending on the legacy helper.

## Architecture Constraints

- Selected approach: port the existing purge-geometry helpers onto `run_finalization()` and preserve the geometry rules already validated on `process()`.
- The live host path must retire its dependency on the legacy helper once the port lands.
- The packet stays on `push_entity_to_layer()`; synthetic layers are unnecessary for the current wipe-tower model.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionView.tool_changes()`
  - `FinalizationOutputBuilder.push_entity_to_layer()`
  - `ExtrusionRole::WipeTower`
  - live layer targeting by `ToolChange.after_entity_index` and layer index
- WIT boundary considerations:
  - no WIT widening is expected; the packet uses the existing finalization world surface
- Determinism or scheduler constraints:
  - given identical tool-change input and purge config, finalization pushes must be deterministic

## Locked Assumptions and Invariants

- Wipe-tower entities stay on existing layers, not synthetic layers.
- The existing purge-volume behavior is the source of truth for the port.

## Risks and Tradeoffs

- Risk: layer-height inference can drift on the new surface. Mitigation: keep direct tests on purge-volume and layer targeting.
- Risk: the host may still route through the legacy helper. Mitigation: require a live finalization merge regression.

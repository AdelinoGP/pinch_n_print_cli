---
status: implemented
packet: wipe-tower-finalization-live-path
task_ids:
  - TASK-143
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: wipe-tower-finalization-live-path

## Goal

Port `WipeTower` live geometry from the legacy `process(&mut Vec<LayerCollectionIR>)` path into `FinalizationModule::run_finalization()` and retire the legacy-only finalization dependency on the live host path.

## Scope Boundaries

- In scope:
  - port wipe-tower purge geometry into `run_finalization()` using `LayerCollectionView` and `FinalizationOutputBuilder`
  - preserve existing purge-volume, tower-position, and per-tool-change behavior on the new surface
  - host integration proving finalization output merges back into live layers
  - retire the host's dependency on the legacy vector-mutation path for wipe-tower geometry
- Out of scope:
  - finalization-aware travel reconciliation with wipe geometry (packet `20`)
  - Orca-facing prime-tower text emission (packet `11`)
  - mixed-tool ordering heuristics (packet `19`)

## Prerequisites and Blockers

- Depends on:
  - the SDK finalization surface in `slicer-sdk`
  - current legacy `WipeTower::process()` logic as the port source
- Unblocks:
  - TASK-152f finalization-aware travel coordination
  - packet `19` mixed-tool sequencing when wipe geometry is present on the live path
- Activation blockers:
  - None. The packet is `draft` by default.

## Acceptance Criteria

- **Given** a finalization fixture whose layer `0` contains one `ToolChange { from: 0, to: 1 }`, **when** `WipeTower::run_finalization()` executes with `wipe_tower_enabled=true`, **then** `FinalizationOutputBuilder.entity_pushes()` contains non-empty pushes targeting layer `0` and every pushed path has `path.role = ExtrusionRole::WipeTower`. | `cargo test -p wipe-tower --test finalization_live_tdd run_finalization_pushes_wipe_tower_entities_for_tool_change_layers -- --exact --nocapture`
- **Given** two otherwise identical fixtures except `wipe_tower_purge_volume=70.0` and `140.0`, **when** `run_finalization()` executes, **then** the larger-purge fixture emits strictly more wipe-tower entity pushes than the smaller-purge fixture. | `cargo test -p wipe-tower --test finalization_live_tdd purge_volume_controls_finalization_push_count -- --exact --nocapture`
- **Given** two layers where only the second layer contains `tool_changes`, **when** `run_finalization()` executes, **then** only the second layer receives `WipeTower` entity pushes. | `cargo test -p wipe-tower --test finalization_live_tdd run_finalization_targets_only_layers_with_tool_changes -- --exact --nocapture`
- **Given** a live host finalization dispatch using the canonical `wipe-tower` module, **when** the host merges finalization output into `LayerCollectionIR`, **then** the resulting finalized layers contain appended `ExtrusionRole::WipeTower` entities without invoking the legacy `process()` path. | `cargo test -p slicer-host --test finalization_live_tdd live_finalization_dispatch_merges_wipe_tower_entity_pushes -- --exact --nocapture`

## Negative Test Cases

- **Given** `wipe_tower_enabled=false` or a layer set with no `tool_changes`, **when** `run_finalization()` executes, **then** `FinalizationOutputBuilder.entity_pushes()` is empty. | `cargo test -p wipe-tower --test finalization_live_tdd disabled_or_no_tool_changes_emit_no_finalization_pushes -- --exact --nocapture`

## Verification

- `cargo test -p wipe-tower --test finalization_live_tdd run_finalization_pushes_wipe_tower_entities_for_tool_change_layers -- --exact --nocapture`
- `cargo test -p wipe-tower --test finalization_live_tdd purge_volume_controls_finalization_push_count -- --exact --nocapture`
- `cargo test -p wipe-tower --test finalization_live_tdd run_finalization_targets_only_layers_with_tool_changes -- --exact --nocapture`
- `cargo test -p wipe-tower --test finalization_live_tdd disabled_or_no_tool_changes_emit_no_finalization_pushes -- --exact --nocapture`
- `cargo test -p slicer-host --test finalization_live_tdd live_finalization_dispatch_merges_wipe_tower_entity_pushes -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/03_wit_and_manifest.md`
- `docs/05_module_sdk.md`
- `docs/07_implementation_status.md`
- `docs/DEVIATION_LOG.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
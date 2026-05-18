# Requirements: wipe-tower-finalization-live-path

## Packet Metadata

- Grouped task IDs:
  - `TASK-143` — port `WipeTower` live geometry into `run_finalization()` and retire the legacy-only finalization path
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

`WipeTower` mirrors the same DEV-013 gap as `SkirtBrim`: real geometry exists on a legacy vector-mutation helper while the documented `run_finalization()` path is a no-op. This packet ports the existing wipe-tower logic onto the canonical finalization API and adds host integration proof so the live path can stop depending on the legacy helper.

## In Scope

- `WipeTower::run_finalization()`
- `LayerCollectionView`-based tool-change discovery and layer targeting
- `FinalizationOutputBuilder.push_entity_to_layer()` for wipe-tower entities
- host integration proving merge-back into finalized layers

## Out of Scope

- finalization-aware travel reconciliation
- tool-order planning policy
- final GCode comment emission

## Authoritative Docs

- `docs/03_wit_and_manifest.md`
- `docs/05_module_sdk.md`
- `docs/07_implementation_status.md`
- `docs/DEVIATION_LOG.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp`

## Acceptance Summary

### Positive Cases

- Layers with tool changes emit non-empty `WipeTower` finalization pushes.
- Purge volume changes finalization push count.
- Only layers with tool changes receive wipe-tower pushes.
- The live host finalization path merges those pushes into finalized layers.

### Negative Cases

- Disabled or no-tool-change inputs emit no finalization pushes.

### Measurable Outcomes

- Acceptance tests assert exact layer targeting and exact `ExtrusionRole::WipeTower` roles.

### Cross-Packet Impact

- Packet `20` depends on this packet before it can coordinate travel decisions against wipe-tower geometry.
- Packet `19` can only reason about mixed-tool sequencing honestly once live wipe-tower geometry exists.

## Verification Commands

- `cargo test -p wipe-tower --test finalization_live_tdd run_finalization_pushes_wipe_tower_entities_for_tool_change_layers -- --exact --nocapture`
- `cargo test -p wipe-tower --test finalization_live_tdd purge_volume_controls_finalization_push_count -- --exact --nocapture`
- `cargo test -p wipe-tower --test finalization_live_tdd run_finalization_targets_only_layers_with_tool_changes -- --exact --nocapture`
- `cargo test -p wipe-tower --test finalization_live_tdd disabled_or_no_tool_changes_emit_no_finalization_pushes -- --exact --nocapture`
- `cargo test -p slicer-host --test finalization_live_tdd live_finalization_dispatch_merges_wipe_tower_entity_pushes -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the finalization or host merge gap is localized to one surface
- Postcondition: one exact wipe-tower output-builder behavior is restored on the canonical SDK path
- Falsifying check: a focused test fails if the module still depends on the legacy `process()` path
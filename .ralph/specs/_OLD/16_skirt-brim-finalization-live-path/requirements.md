# Requirements: skirt-brim-finalization-live-path

## Packet Metadata

- Grouped task IDs:
  - `TASK-142` — port `SkirtBrim` live geometry into `run_finalization()`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

`SkirtBrim` already has real geometry logic, but the live host finalization path still calls a default `run_finalization()` that does nothing. This leaves finalization behavior split between the documented SDK authoring surface and a legacy direct vector-mutation helper. The packet fixes that gap by porting the existing geometry logic onto the canonical finalization API without mixing in wipe tower or travel work.

## In Scope

- `SkirtBrim::run_finalization()`
- `LayerCollectionView`-based bbox discovery and layer targeting
- `FinalizationOutputBuilder.push_entity_to_layer()` for skirt and brim geometry
- host integration proving finalization output merges back into finalized layers

## Out of Scope

- wipe tower
- finalization-aware travel coordination
- final GCode comment emission

## Authoritative Docs

- `docs/03_wit_and_manifest.md`
- `docs/05_module_sdk.md`
- `docs/07_implementation_status.md`
- `docs/DEVIATION_LOG.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Brim.hpp`
- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp`

## Acceptance Summary

### Positive Cases

- `run_finalization()` emits skirt entity pushes on the targeted layers.
- `run_finalization()` emits brim entity pushes on layer `0` only and does not misuse synthetic layers.
- The live host finalization path merges those pushes into finalized layer collections.

### Negative Cases

- Disabled or empty inputs emit no finalization pushes.

### Measurable Outcomes

- Acceptance tests assert exact layer targets, exact `RegionKey.object_id` values, and exact `ExtrusionRole::Skirt` roles.

### Cross-Packet Impact

- Packet `20` depends on this packet before it can reconcile travel decisions against brim geometry.
- Packet `11` later serializes the resulting `Skirt` entities as `;TYPE:Skirt/Brim`.

## Verification Commands

- `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_pushes_skirt_entities_to_target_layers -- --exact --nocapture`
- `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_pushes_brim_entities_on_layer_zero_only -- --exact --nocapture`
- `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_respects_skirt_height_layer_targeting -- --exact --nocapture`
- `cargo test -p skirt-brim --test finalization_live_tdd disabled_or_empty_input_emits_no_finalization_pushes -- --exact --nocapture`
- `cargo test -p slicer-host --test finalization_live_tdd live_finalization_dispatch_merges_skirt_brim_entity_pushes -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the finalization helper or host gap is isolated
- Postcondition: one exact output-builder behavior is restored on the canonical SDK path
- Falsifying check: a focused test fails if the module still depends on the legacy `process()` path
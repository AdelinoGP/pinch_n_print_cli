---
status: implemented
packet: skirt-brim-finalization-live-path
task_ids:
  - TASK-142
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: skirt-brim-finalization-live-path

## Goal

Port `SkirtBrim` live geometry from the legacy `process(&mut Vec<LayerCollectionIR>)` path into `FinalizationModule::run_finalization()` using `LayerCollectionView` inputs and `FinalizationOutputBuilder` output, so the live host finalization path no longer depends on the legacy vector-mutation surface.

## Scope Boundaries

- In scope:
  - port existing skirt and brim geometry helpers onto `run_finalization()`
  - use `LayerCollectionView` for bbox discovery and layer targeting
  - use `FinalizationOutputBuilder.push_entity_to_layer()` for skirt and brim additions
  - host integration proving the live finalization path merges those entity pushes back into `LayerCollectionIR`
- Out of scope:
  - travel reconciliation with brim geometry (packet `20`)
  - Orca-facing `;TYPE:Skirt/Brim` emission (packet `11`)
  - wipe tower finalization (packet `17`)

## Prerequisites and Blockers

- Depends on:
  - the SDK finalization surface in `slicer-sdk`
  - current legacy `SkirtBrim::process()` logic as the port source
- Unblocks:
  - TASK-152f finalization-aware travel coordination
  - TASK-135 Benchy evidence for brim/skirt-adjacent behavior once travel policy is restored
- Activation blockers:
  - None. The packet is `draft` by default.

## Acceptance Criteria

- **Given** a finalization fixture whose first layer encloses one model bbox and config `skirt_loops=2`, **when** `SkirtBrim::run_finalization()` executes, **then** `FinalizationOutputBuilder.entity_pushes()` contains exactly two pushes targeting layer `0`, each push has `path.role = ExtrusionRole::Skirt`, and each push uses `RegionKey.object_id = "__skirt__"`. | `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_pushes_skirt_entities_to_target_layers -- --exact --nocapture`
- **Given** the same fixture with `brim_width=3.0`, **when** `run_finalization()` executes, **then** layer `0` receives non-empty brim pushes whose `RegionKey.object_id = "__brim__"`, whose `path.role = ExtrusionRole::Skirt`, and whose pushes are emitted through `push_entity_to_layer()` rather than `insert_synthetic_layer()`. | `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_pushes_brim_entities_on_layer_zero_only -- --exact --nocapture`
- **Given** config `skirt_height=3` and four existing layers, **when** `run_finalization()` executes, **then** entity pushes target only layers `0`, `1`, and `2` and never layer `3`. | `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_respects_skirt_height_layer_targeting -- --exact --nocapture`
- **Given** a live host finalization dispatch using a `push_entity_to_layer`-emitting guest (verified via `sdk-finalization-guest.component.wasm`; the canonical `skirt-brim.wasm` requires a WASM rebuild via `build-core-modules.sh` to activate the ported `run_finalization()`), **when** the host merges finalization output into `LayerCollectionIR`, **then** the finalization entities are batch-prepended before the original model entities in each target layer. | `cargo test -p slicer-host --test finalization_live_tdd live_finalization_dispatch_merges_skirt_brim_entity_pushes -- --exact --nocapture`

## Negative Test Cases

- **Given** `skirt_brim_enabled=false` or a finalization input with no printable entities, **when** `run_finalization()` executes, **then** `FinalizationOutputBuilder.entity_pushes()` and `synthetic_layers()` are both empty. | `cargo test -p skirt-brim --test finalization_live_tdd disabled_or_empty_input_emits_no_finalization_pushes -- --exact --nocapture`

## Verification

- `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_pushes_skirt_entities_to_target_layers -- --exact --nocapture`
- `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_pushes_brim_entities_on_layer_zero_only -- --exact --nocapture`
- `cargo test -p skirt-brim --test finalization_live_tdd run_finalization_respects_skirt_height_layer_targeting -- --exact --nocapture`
- `cargo test -p skirt-brim --test finalization_live_tdd disabled_or_empty_input_emits_no_finalization_pushes -- --exact --nocapture`
- `cargo test -p slicer-host --test finalization_live_tdd live_finalization_dispatch_merges_skirt_brim_entity_pushes -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — finalization world contract
- `docs/05_module_sdk.md` — `FinalizationModule`, `LayerCollectionView`, and `FinalizationOutputBuilder`
- `docs/07_implementation_status.md` — TASK-142 scope
- `docs/DEVIATION_LOG.md` — DEV-013 live-path gap record

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Brim.hpp`
- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
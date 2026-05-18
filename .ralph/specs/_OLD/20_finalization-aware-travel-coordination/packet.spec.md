---
status: implemented
packet: finalization-aware-travel-coordination
task_ids:
  - TASK-152
  - TASK-152f
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: finalization-aware-travel-coordination

## Goal

Coordinate live travel decisions with finalization-generated `Skirt` and `WipeTower` geometry so brim and wipe detours stop being ignored once finalization entities are present on the completed layer set.

## Scope Boundaries

- In scope:
  - post-finalization travel reconciliation against `Skirt` and `WipeTower` entities already present in finalized layers
  - preservation of the model extrusion order while reconciling travel transitions
  - deterministic host regressions for brim-aware and wipe-aware travel transitions
- Out of scope:
  - generating SkirtBrim geometry (packet `16`)
  - generating WipeTower geometry (packet `17`)
  - deciding the base retract/no-retract policy (packet `15`)
  - final text formatting for the resulting travel moves (packet `11`)

## Prerequisites and Blockers

- Depends on:
  - packet `15` for base travel policy
  - packet `16` and packet `17` for live finalization geometry
- Unblocks:
  - packet `21` Benchy evidence when brim or wipe detours should affect travel behavior
- Activation blockers:
  - finalization geometry packets must land first; until then this packet remains `draft`.

## Acceptance Criteria

- **Given** a finalized layer whose first emitted entities are `ExtrusionRole::Skirt` paths followed by model wall entities, **when** the post-finalization travel reconciliation pass runs, **then** the first model travel transition is computed from the last skirt endpoint instead of ignoring the skirt loop entirely. | `cargo test -p slicer-host --test finalization_aware_travel_tdd brim_geometry_changes_first_model_travel_transition -- --exact --nocapture`
- **Given** a finalized layer with one `ToolChange` and one appended `ExtrusionRole::WipeTower` entity block, **when** the reconciliation pass runs, **then** the emitted travel sequence includes the wipe-tower detour between the tool change and the next model entity and preserves the matched retract/unretract pairing chosen by packet `15`. | `cargo test -p slicer-host --test finalization_aware_travel_tdd wipe_tower_geometry_is_included_in_travel_reconciliation -- --exact --nocapture`
- **Given** a finalized layer set with no `Skirt` or `WipeTower` entities, **when** the reconciliation pass runs, **then** the travel sequence is byte-identical to the unreconciled output. | `cargo test -p slicer-host --test finalization_aware_travel_tdd no_finalization_geometry_is_a_reconciliation_no_op -- --exact --nocapture`

## Negative Test Cases

- **Given** a finalized layer with finalization geometry present, **when** the reconciliation pass runs, **then** it does not reorder model extrusion entities relative to one another; it only changes travel transitions and their paired policy markers. | `cargo test -p slicer-host --test finalization_aware_travel_tdd reconciliation_preserves_model_extrusion_entity_order -- --exact --nocapture`
- **Given** a finalized layer with `ExtrusionRole::WipeTower` entities present, **when** the reconciliation pass runs, **then** the wipe tower block is included in the travel detour sequence without altering the relative order of model extrusion entities. | `cargo test -p slicer-host --test finalization_aware_travel_tdd reconciliation_preserves_model_extrusion_entity_order_with_wipe_tower -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test finalization_aware_travel_tdd brim_geometry_changes_first_model_travel_transition -- --exact --nocapture`
- `cargo test -p slicer-host --test finalization_aware_travel_tdd wipe_tower_geometry_is_included_in_travel_reconciliation -- --exact --nocapture`
- `cargo test -p slicer-host --test finalization_aware_travel_tdd no_finalization_geometry_is_a_reconciliation_no_op -- --exact --nocapture`
- `cargo test -p slicer-host --test finalization_aware_travel_tdd reconciliation_preserves_model_extrusion_entity_order -- --exact --nocapture`
- `cargo test -p slicer-host --test finalization_aware_travel_tdd reconciliation_preserves_model_extrusion_entity_order_with_wipe_tower -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — stage ownership and finalization constraints
- `docs/04_host_scheduler.md` — finalization then postpass ordering
- `docs/05_module_sdk.md` — finalization output semantics
- `docs/07_implementation_status.md` — TASK-152f scope

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/AvoidCrossingPerimeters.cpp`
- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
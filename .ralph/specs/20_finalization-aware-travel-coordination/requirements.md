# Requirements: finalization-aware-travel-coordination

## Packet Metadata

- Grouped task IDs:
  - `TASK-152` — expand path optimization beyond comment-only output for the finalization-aware coordination slice
  - `TASK-152f` — coordinate path optimization with `SkirtBrim` and `WipeTower` outputs
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

The path-optimization and travel packets can only see the pre-finalization layer graph. Once finalization appends brim or wipe geometry, the current travel behavior can still pretend those entities do not exist. This packet owns that last gap by reconciling travel transitions after finalization geometry is present, without reopening geometry generation or base retract policy.

## In Scope

- post-finalization travel reconciliation
- brim-aware and wipe-aware detour handling
- deterministic no-op and preserve-order regressions

## Out of Scope

- geometry generation for brim or wipe tower
- base retract/no-retract policy
- final GCode text formatting

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/04_host_scheduler.md`
- `docs/05_module_sdk.md`
- `docs/07_implementation_status.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/AvoidCrossingPerimeters.cpp`
- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp`

## Acceptance Summary

### Positive Cases

- Brim geometry changes the first model travel transition.
- Wipe geometry is included in the travel detour sequence after tool changes.
- Layers without finalization geometry are unchanged.

### Negative Cases

- Model extrusion entity order is preserved while travel transitions change.

### Measurable Outcomes

- Acceptance tests assert exact travel-transition behavior and exact preservation of model entity order.

### Cross-Packet Impact

- Packet `21` depends on this packet before it can make honest Benchy travel assertions when finalization geometry is present.

## Verification Commands

- `cargo test -p slicer-host --test finalization_aware_travel_tdd brim_geometry_changes_first_model_travel_transition -- --exact --nocapture`
- `cargo test -p slicer-host --test finalization_aware_travel_tdd wipe_tower_geometry_is_included_in_travel_reconciliation -- --exact --nocapture`
- `cargo test -p slicer-host --test finalization_aware_travel_tdd no_finalization_geometry_is_a_reconciliation_no_op -- --exact --nocapture`
- `cargo test -p slicer-host --test finalization_aware_travel_tdd reconciliation_preserves_model_extrusion_entity_order -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the relevant finalization geometry and travel-policy surfaces are already available
- Postcondition: one exact travel reconciliation rule is observable
- Falsifying check: a focused travel-transition assertion fails if the rule regresses
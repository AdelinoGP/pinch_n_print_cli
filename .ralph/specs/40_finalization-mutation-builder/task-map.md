# Task Map: finalization-mutation-builder

## Backlog Mapping

| Task ID | docs/07 row status | Packet step coverage |
| --- | --- | --- |
| `TASK-171` (NEW; will be inserted by Step 6) | not yet present | Steps 0, 1, 2, 3, 4, 5, 6 |

## Predecessor Reconciliation

- This packet does NOT supersede any prior packet.
- It depends on `39_stable-entity-ids` (`TASK-170`) being `implemented`. Step 0 explicitly verifies this and stops if Packet 39 is not closed.
- It closes the deferred print-order concern from Packet `38-rev1_top-surface-ironing` (`TASK-169`). Packet 38-rev1's final report flagged literal-prepend at `dispatch.rs:2877` as a print-quality concern; this packet ships the fix. The TASK-169 row at `docs/07_implementation_status.md:83` is NOT touched (closed work belongs to its own packet).

## docs/07 Edit Plan

- Add ONE new row for `TASK-171` describing: "Promote `FinalizationOutputBuilder` from push-only to a true mutation builder (push_with_priority, modify_entity, sort_layer_by, insert_synthetic_layer_after) backed by `ExtrusionRole::default_priority()` per-role table. Replace `dispatch.rs:2877` `splice(0..0, ...)` prepend with extend + ID-stamp + stable-sort by priority + apply recorded mutations. Migrate `top-surface-ironing` to land entities at `ExtrusionRole::Ironing.default_priority()` so ironing G-code emits AFTER top-fill within each top layer (closes the deferred print-order concern from Packet 38-rev1). Backwards-compatible: skirt-brim's existing call site preserved via `push_entity_to_layer → push_entity_with_priority(..., 0)` alias. Foundation for future PostPass mutation modules (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`)."
- Status flag set when Step 6 acceptance ceremony PASSES.
- DO NOT edit, modify, or annotate any existing row.

## Authoritative Docs Per Step

| Step | Primary docs | Notes |
| --- | --- | --- |
| Step 0 | none directly | Discovery dispatches only. |
| Step 1 | `docs/02_ir_schemas.md`, `docs/05_module_sdk.md` | Test fixture pattern; ExtrusionRole + builder shape. |
| Step 2 | `docs/02_ir_schemas.md` | ExtrusionRole enum. |
| Step 3 | `docs/05_module_sdk.md` | FinalizationOutputBuilder API. |
| Step 4 | `docs/04_host_scheduler.md` § 309–317 | Composable multi-writer patterns. |
| Step 5 | none | Mechanical one-line migration. |
| Step 6 | `docs/07_implementation_status.md` | Backlog row insertion. |

## OrcaSlicer Refs Per Step

None required. If parity is challenged for the role-priority defaults, delegate one SUMMARY ≤ 200 words on `OrcaSlicerDocumented/src/libslic3r/GCode/PrintExtents.cpp` (or the equivalent layer-emit ordering site). All OrcaSlicer reads MUST be delegated.

## Cross-Packet Dependencies

- **Depends on** packet `39_stable-entity-ids` (`TASK-170`). Step 0 explicitly verifies it is `implemented`.
- **Closes** the deferred print-order concern from packet `38-rev1_top-surface-ironing`'s final swarm report.
- **Unblocks** future PostPass mutation modules (each a separate future packet):
  - `SequentialPrintOrder` — uses `sort_layer_by` to group entities by `object_id`.
  - `MinLayerTimeEnforcer` — uses `modify_entity` to scale `speed_factor` on slow-printing layers; potentially `insert_synthetic_layer_after` for cooling pauses.
  - `FlushVolumeCalculator` — uses `modify_entity` on wipe-tower entities.
  - `PrimeTower` — uses `push_entity_with_priority(..., PrimeTower.default_priority())` per layer.

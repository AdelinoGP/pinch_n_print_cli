# Task Map: stable-entity-ids

## Backlog Mapping

| Task ID | docs/07 row status | Packet step coverage |
| --- | --- | --- |
| `TASK-170` (NEW; will be inserted by Step 7) | not yet present | Steps 0, 1, 2, 3, 4, 5, 6, 7 |

## Predecessor Reconciliation

- This packet does NOT supersede any prior packet. It builds atop `38-rev1_top-surface-ironing` (already `implemented`) and unblocks `40_finalization-mutation-builder`.
- No retrofit of closed task rows. The `TASK-169` row at `docs/07_implementation_status.md:83` belongs to packet `38-rev1_top-surface-ironing` and is NOT touched.

## docs/07 Edit Plan

- Add ONE new row for `TASK-170` describing: "Foundation refactor: introduce stable per-layer-monotonic `entity_id: u64` on `LayerCollectionIR.ordered_entities` entries; migrate `TravelMove` anchor from positional index to `entity_id`. Producers issue IDs at construction; `gcode_emit` resolves travel anchors via per-layer `HashMap<u64, usize>` lookup. Pure refactor — zero G-code byte change. Foundation for packet `40_finalization-mutation-builder` and future PostPass mutators (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`)."
- Status flag set when Step 7 acceptance ceremony PASSES.
- DO NOT edit, modify, or annotate any existing row.

## Authoritative Docs Per Step

| Step | Primary docs | Notes |
| --- | --- | --- |
| Step 0 | none directly | Discovery dispatches only. |
| Step 1 | `docs/02_ir_schemas.md` | Test fixture pattern; entity / TravelMove shape. |
| Step 2 | `docs/02_ir_schemas.md` | Schema versioning rule. |
| Step 3 | none | Validation helper is internal. |
| Step 4 | `docs/04_host_scheduler.md` § 309–317 | Producer concurrency model. |
| Step 5 | none | Emit-side change is mechanical. |
| Step 6 | none | Fixture sweep is mechanical. |
| Step 7 | `docs/07_implementation_status.md`; `docs/14_deviation_audit_history.md` (conditional) | Backlog row insertion + schema-bump audit if policy requires. |

## OrcaSlicer Refs Per Step

None. This packet is internal-IR-only; OrcaSlicer is not a parity reference for entity identity.

## Cross-Packet Dependencies

- **Unblocks** packet `40_finalization-mutation-builder`. Packet 40's Step 0 must verify `TASK-170` is `implemented` before its own discovery dispatches.
- **Future modules dependent on this work** (per `docs/01_system_architecture.md:328-363`): `SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`, plus any module that needs to mutate or reorder existing layer entities.

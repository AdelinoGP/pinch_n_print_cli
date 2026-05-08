# Task Map: finalization-mutation-enum-refactor

## Backlog Mapping

| Task ID | docs/07 row status | Packet step coverage |
| --- | --- | --- |
| `TASK-172` (NEW; will be inserted by Step 6) | not yet present | Steps 0, 1, 2, 3, 4, 5, 6 |

## Predecessor Reconciliation

- This packet does NOT supersede any prior packet.
- It depends on `40_finalization-mutation-builder` (`TASK-171`) being `implemented`. Step 0 explicitly verifies via FACT.
- It closes `DEV-041`, which was registered at packet 40's acceptance (2026-05-07) as an "open" deviation tracking the silent-no-op WIT gap. Step 6 appends a closure note to `docs/14_deviation_audit_history.md`. The TASK-171 row at `docs/07_implementation_status.md:85` is NOT touched (closed work belongs to its own packet).

## docs/07 Edit Plan

- Add ONE new row for `TASK-172` describing: "Refactor `FinalizationOutputBuilder`'s mutation methods (`modify_entity`, `sort_layer_by`, `insert_synthetic_layer_after`) from closure-based APIs to serializable-enum-based APIs (`EntityMutation`, `SortKey`, `SyntheticLayerData`) so they round-trip cleanly across the WIT boundary. Wire the `slicer-macros` `run_finalization` drain-back loop to forward `merge_ops` via WIT. Add a WASM-side round-trip test guest at `test-guests/finalization-mutation-roundtrip-guest/` and host-side end-to-end test at `crates/slicer-host/tests/finalization_mutation_roundtrip_tdd.rs` proving a guest's `modify_entity(layer, id, EntityMutation::SetSpeedFactor(0.5))` actually mutates the host-side IR. Closes `DEV-041`. Establishes the WIT round-trip contract that future PostPass mutation modules (`SequentialPrintOrder`, `MinLayerTimeEnforcer`, `FlushVolumeCalculator`, `PrimeTower`) will consume."
- Status flag set when Step 6 acceptance ceremony PASSES.
- DO NOT edit, modify, or annotate any existing row.

## docs/14 Edit Plan

- Append ONE new line in the chronology section: `**YYYY-MM-DD — DEV-041 closed**` (date at acceptance ceremony) with a one-paragraph closure note describing the SDK API refactor, the drain-back fix, and the WASM round-trip validation. Reference TASK-172 and packet `41_finalization-mutation-enum-refactor`.
- DO NOT modify any other DEV-XXX entry. DO NOT alter the `Outcome Summary` or `Audit Method Summary` sections.

## Authoritative Docs Per Step

| Step | Primary docs | Notes |
| --- | --- | --- |
| Step 0 | `.ralph/specs/40_finalization-mutation-builder/design.md` (narrow), `docs/14_deviation_audit_history.md` (narrow) | Discovery dispatches: Packet 40 status, future-module audit, DEV-041 entry. |
| Step 1 | `docs/02_ir_schemas.md`, `docs/05_module_sdk.md`, `docs/03_wit_and_manifest.md` | Test fixture pattern; PrintEntity/ExtrusionPath3D/LayerCollectionIR shapes; WIT conventions for the new test guest. |
| Step 2 | `docs/02_ir_schemas.md` | New SDK types use `ExtrusionPath3D` from slicer-ir. |
| Step 3 | `docs/05_module_sdk.md` | FinalizationOutputBuilder API contract. |
| Step 4 | `docs/03_wit_and_manifest.md` | WIT shape conventions. |
| Step 5 | `docs/04_host_scheduler.md` § 309–317, 680–717 | PostPass scheduler; multi-writer composition. |
| Step 6 | `docs/07_implementation_status.md`, `docs/14_deviation_audit_history.md` | Backlog row + DEV-041 closure (delegated edits). |

## OrcaSlicer Refs Per Step

None required. If parity is challenged for `EntityMutation::SetSpeedFactor` semantics, delegate one SUMMARY ≤ 200 words on `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.cpp` for context only. All OrcaSlicer reads MUST be delegated.

## Cross-Packet Dependencies

- **Depends on** packet `40_finalization-mutation-builder` (`TASK-171`, `implemented` 2026-05-07). Step 0 explicitly verifies it is `implemented`.
- **Closes** deviation `DEV-041` registered at packet 40's acceptance.
- **Unblocks** future PostPass mutation modules (each a separate future packet):
  - `SequentialPrintOrder` — uses `sort_layer_by(SortKey::ByObjectIdThenPriority)` to group entities by `object_id`.
  - `MinLayerTimeEnforcer` — uses `modify_entity(EntityMutation::SetSpeedFactor)` to slow specific extrusions on fast-printing layers; potentially `insert_synthetic_layer_after` for cooling pauses.
  - `FlushVolumeCalculator` — uses `modify_entity(EntityMutation::SetExtrusionWidthFactor)` (or similar `Set*` variant) on wipe-tower entities.
  - `PrimeTower` — uses `push_entity_with_priority(..., PrimeTower.default_priority())` per layer (already round-trips via Packet 40); may also use `modify_entity` for prime-amount adjustments.

## Migration Obligations Inherited

None. Packet 40 follow-up (2026-05-07 session) closed the DEV-039 carry-forward by migrating `skirt-brim` and `wipe-tower` to the builder API. No further module-side migration owed by this packet.

## Notes on Step Sequencing

- Steps 0 → 1 → 2 → 3 → 4 → 5 → 6 are sequential. No safe parallelism opportunities — each step's edits land in files that the next step reads.
- Step 1 has multi-file scaffold (4 files allowed); if the implementer feels scope is tight, splitting Step 1 into Step 1a (test migrations) and Step 1b (new test guest scaffold + new host test file scaffold) is acceptable. The packet does not lose correctness from the split.
- Step 4 may discover that the SDK and WIT names already match (Packet 40 Step 3b authored both); in that case Step 4's wit/world-finalization.wit and inline-WIT edits are no-ops, and only `wit_host.rs` simplification remains.

# Design: wipe-tower-finalization-live-path

## Controlling Code Paths

- Primary module path: `modules/core-modules/wipe-tower/src/lib.rs`.
- SDK surface: `crates/slicer-sdk/src/traits.rs` for `LayerCollectionView`, `FinalizationOutputBuilder`, and `FinalizationModule`.
- Host integration path: `crates/slicer-host/src/dispatch.rs` finalization dispatch and merge behavior.
- Neighboring tests or fixtures: existing `wipe_tower_tdd.rs` plus new `finalization_live_tdd.rs` in the module crate and host test tree.
- OrcaSlicer comparison surface: `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower.cpp` and `WipeTower2.cpp`.

## Architecture Constraints

- Selected approach: port the existing purge-geometry helpers onto `run_finalization()` and preserve the geometry rules already validated on `process()`.
- The live host path must retire its dependency on the legacy helper once the port lands.
- The packet stays on `push_entity_to_layer()`; synthetic layers are unnecessary for the current wipe-tower model.

## Code Change Surface

- Selected approach:
  - add focused module tests for `run_finalization()` wipe-tower pushes
  - implement `run_finalization()` from the existing tool-change driven helper logic
  - add one host integration regression proving finalization merge-back on the live path
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `modules/core-modules/wipe-tower/src/lib.rs`
  - `modules/core-modules/wipe-tower/tests/finalization_live_tdd.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/finalization_live_tdd.rs`
  - `docs/DEVIATION_LOG.md`
- Rejected alternatives that were considered and why they were not chosen:
  - keeping wipe-tower on the legacy helper while only documenting the gap: rejected because TASK-143 is an implementation backlog item, not a docs-only closure
  - bundling travel reconciliation into the same packet: rejected because packet `20` owns that next slice

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

## Open Questions

- None. The packet chooses the direct port to `run_finalization()`.
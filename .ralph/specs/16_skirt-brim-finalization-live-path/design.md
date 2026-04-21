# Design: skirt-brim-finalization-live-path

## Controlling Code Paths

- Primary module path: `modules/core-modules/skirt-brim/src/lib.rs`.
- SDK surface: `crates/slicer-sdk/src/traits.rs` for `LayerCollectionView`, `FinalizationOutputBuilder`, and `FinalizationModule`.
- Host integration path: `crates/slicer-host/src/dispatch.rs` finalization dispatch and merge behavior.
- Neighboring tests or fixtures: existing `skirt_brim_tdd.rs` plus new `finalization_live_tdd.rs` in the module crate and host test tree.
- OrcaSlicer comparison surface: `OrcaSlicerDocumented/src/libslic3r/Brim.cpp`.

## Architecture Constraints

- Selected approach: port the existing bbox and loop-generation helpers unchanged where possible, and swap only the input/output surface from `&mut Vec<LayerCollectionIR>` to `LayerCollectionView` plus `FinalizationOutputBuilder`.
- The live host path must not depend on calling the legacy `process()` helper after the port lands.
- The packet must stay on `push_entity_to_layer()` for skirt and brim geometry; synthetic layers are out of scope here.

## Code Change Surface

- Selected approach:
  - add focused module tests for `run_finalization()` pushes
  - implement `run_finalization()` using the existing geometry helpers and finalization builder
  - add one host integration regression proving merge-back into finalized layers
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `modules/core-modules/skirt-brim/src/lib.rs`
  - `modules/core-modules/skirt-brim/tests/finalization_live_tdd.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/finalization_live_tdd.rs`
  - `docs/DEVIATION_LOG.md`
- Rejected alternatives that were considered and why they were not chosen:
  - keeping the legacy `process()` path as the live host integration surface: rejected because it leaves DEV-013 unresolved
  - widening the packet to include travel coordination: rejected because packet `20` owns that separate slice

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionView.layer_index()`, `z()`, and `ordered_entities()`
  - `FinalizationOutputBuilder.push_entity_to_layer()`
  - `ExtrusionRole::Skirt`
  - `RegionKey.object_id = "__skirt__"` and `"__brim__"`
- WIT boundary considerations:
  - no WIT widening is expected; the packet uses the existing world-finalization surface
- Determinism or scheduler constraints:
  - emitted skirt/brim pushes must remain deterministic for the same input layer set and config

## Locked Assumptions and Invariants

- Skirt and brim stay on existing layers, not synthetic layers.
- The geometry formulas already validated on `process()` are the source of truth for the port.

## Risks and Tradeoffs

- Risk: bbox discovery through `LayerCollectionView` can diverge from the legacy vector path. Mitigation: reuse the same geometric helpers and assert exact layer targets.
- Risk: the host may still call the legacy helper after the port. Mitigation: keep a host integration regression that proves merge-back from finalization output.

## Open Questions

- None. The packet chooses the direct port to `run_finalization()`.
# Design: finalization-aware-travel-coordination

## Controlling Code Paths

- Primary host path: `crates/slicer-host/src/gcode_emit.rs` or a dedicated post-finalization travel helper invoked immediately before final text emission.
- Supporting stage surfaces: finalized `LayerCollectionIR` content produced after packet `16` and packet `17`, plus travel policy metadata from packet `15`.
- Neighboring tests or fixtures: new `finalization_aware_travel_tdd.rs` and existing finalization integration tests.
- OrcaSlicer comparison surface: `GCode.cpp`, `AvoidCrossingPerimeters.cpp`, `Brim.cpp`, and `WipeTower.cpp`.

## Architecture Constraints

- Selected approach: perform travel reconciliation after finalization, because that is the first point where both finalization geometry and base travel policy decisions are visible together.
- The reconciliation pass may change travel transitions and their paired policy markers, but it must not reorder model extrusion entities.
- The packet must remain host-side; finalization modules and path-optimization modules keep their existing stage responsibilities.

## Code Change Surface

- Selected approach:
  - add focused host tests for brim-aware, wipe-aware, no-op, and preserve-order behavior
  - add a host-side reconciliation helper that consumes finalized layers plus travel policy hints
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/gcode_emit.rs`
  - `crates/slicer-host/src/postpass.rs`
  - `crates/slicer-host/tests/finalization_aware_travel_tdd.rs`
  - Note: `finalization_live_tdd.rs` is intentionally unchanged — unit-level coverage in `finalization_aware_travel_tdd.rs` exercises the reconciliation pass directly; live-path integration for reconciliation is deferred to packet 21 (Benchy evidence).
- Rejected alternatives that were considered and why they were not chosen:
  - moving finalization geometry prediction earlier into `path-optimization-default`: rejected because the geometry does not exist yet at that stage
  - bundling the work into packet `15`: rejected because packet `15` intentionally stops at base travel policy before finalization entities appear

## Data and Contract Notes

- IR or manifest contracts touched:
  - finalized `LayerCollectionIR.ordered_entities`
  - `ExtrusionRole::Skirt` and `ExtrusionRole::WipeTower`
  - whatever travel-policy markers packet `15` stores for later reconciliation
- WIT boundary considerations:
  - no WIT widening is expected; the packet stays on host-side finalized layer data
- Determinism or scheduler constraints:
  - the reconciliation pass must be deterministic and preserve model extrusion order

## Locked Assumptions and Invariants

- Packet `15` remains the owner of base travel policy.
- Packets `16` and `17` remain the owners of finalization geometry generation.

## Risks and Tradeoffs

- Risk: reconciliation could accidentally reorder model entities. Mitigation: a preserve-order negative test is mandatory.
- Risk: adding reconciliation too late could obscure debugging. Mitigation: keep the pass host-local and heavily regression-guarded.

## Open Questions

- None. The packet chooses a host-side post-finalization reconciliation pass.
---
status: implemented
packet: finalization-aware-travel-coordination
task_ids:
  - TASK-152
  - TASK-152f
---

# 20_finalization-aware-travel-coordination

## Goal

Coordinate live travel decisions with finalization-generated `Skirt` and `WipeTower` geometry so brim and wipe detours stop being ignored once finalization entities are present on the completed layer set.

## Problem Statement

The path-optimization and travel packets can only see the pre-finalization layer graph. Once finalization appends brim or wipe geometry, the current travel behavior can still pretend those entities do not exist. This packet owns that last gap by reconciling travel transitions after finalization geometry is present, without reopening geometry generation or base retract policy.

## Architecture Constraints

- Selected approach: perform travel reconciliation after finalization, because that is the first point where both finalization geometry and base travel policy decisions are visible together.
- The reconciliation pass may change travel transitions and their paired policy markers, but it must not reorder model extrusion entities.
- The packet must remain host-side; finalization modules and path-optimization modules keep their existing stage responsibilities.

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

---
status: superseded
packet: path-optimization-entity-ordering
task_ids:
  - TASK-152
  - TASK-152a
  - TASK-152d
  - TASK-152e
---

# 18_path-optimization-entity-ordering

## Goal

Replace the current mostly pass-through entity ordering on the live path with one deterministic path-ordering surface that handles nearest-neighbor ordering for same-object entities, cross-object ordering within a layer, and bridge/overhang-sensitive prioritization before the path-optimization stage emits travel policy.

## Problem Statement

The current layer assembly order is mostly whatever `assemble_ordered_entities()` receives from perimeter, infill, and support IRs. That is too weak for the remaining DEV-023 ordering slice. The packet narrows the work to three deterministic ordering behaviors: same-object nearest-neighbor ordering, cross-object ordering, and bridge/overhang prioritization. It intentionally leaves retract policy, tool sequencing, and cooling outside the boundary.

## Architecture Constraints

- Selected approach: implement the ordering helper on the host pre-path-optimization surface, because the current path-optimization WIT surface is not the right place to rewrite the whole entity list safely.
- The module still owns downstream travel policy, but the host owns the canonical ordered entity sequence it consumes.
- Exact `PrintEntity` identity, `region_key`, and `topo_order` must remain stable after reordering.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `LayerCollectionIR.ordered_entities`
  - `PrintEntity.topo_order`
  - `PrintEntity.region_key.object_id`
  - `ExtrusionRole::BridgeInfill`
- WIT boundary considerations:
  - no WIT widening is expected; the packet uses the host-owned ordered entity surface
- Determinism or scheduler constraints:
  - every ordering rule must be deterministic for identical inputs

## Locked Assumptions and Invariants

- The host owns canonical entity order.
- Packet `15` continues to own travel policy decisions once that order is fixed.

## Risks and Tradeoffs

- Risk: object-crossing heuristics can silently regress determinism. Mitigation: repeated-run order assertions.
- Risk: bridge prioritization could overreach into travel policy. Mitigation: keep the packet on ordering only.

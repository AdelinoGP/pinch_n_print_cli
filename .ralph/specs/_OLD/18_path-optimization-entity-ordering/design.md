# Design: path-optimization-entity-ordering

## Controlling Code Paths

- Primary host path: `crates/slicer-host/src/layer_executor.rs` and `assemble_ordered_entities()`.
- Supporting commit path: `crates/slicer-host/src/dispatch.rs`.
- Module visibility surface: `modules/core-modules/path-optimization-default/src/lib.rs` consumes the reordered sequence after the host helper runs.
- Neighboring tests or fixtures: existing `dispatch_tdd.rs` path-optimization tests plus new `path_ordering_tdd.rs`.
- OrcaSlicer comparison surface: `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp`.

## Architecture Constraints

- Selected approach: implement the ordering helper on the host pre-path-optimization surface, because the current path-optimization WIT surface is not the right place to rewrite the whole entity list safely.
- The module still owns downstream travel policy, but the host owns the canonical ordered entity sequence it consumes.
- Exact `PrintEntity` identity, `region_key`, and `topo_order` must remain stable after reordering.

## Code Change Surface

- Selected approach:
  - add focused host tests for same-object, cross-object, bridge-priority, and no-op cases
  - add one host ordering helper before `Layer::PathOptimization`
  - keep the path-optimization module as the consumer of the reordered sequence, not the owner of full-list mutation
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/path_ordering_tdd.rs`
  - `modules/core-modules/path-optimization-default/src/lib.rs` — Step 4 only: add a narrow test-harness hook (or assertion log) so the integration test can confirm the module's received `ordered_entities` matches the host-reordered sequence. No functional change to travel policy.
- Rejected alternatives that were considered and why they were not chosen:
  - forcing full-list reordering into the current module output surface: rejected because the host already owns `ordered_entities` and the WIT surface is a poor fit for whole-list mutation
  - bundling tool ordering into this packet: rejected because packet `19` owns that next slice

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

## Overhang Scope Clarification

`ExtrusionRole` contains no `OverhangWall` or `OverhangInfill` variant in the current IR. The "bridge/overhang-sensitive prioritization" in TASK-152e is delivered exclusively via `ExtrusionRole::BridgeInfill`. This covers the primary structural-overhang infill case (bridges). Overhang wall prioritization (if ever needed) would require a future IR extension and is explicitly out of scope for this packet. The acceptance criterion (AC-3) tests the BridgeInfill case only.

## Locked Assumptions and Invariants

- The host owns canonical entity order.
- Packet `15` continues to own travel policy decisions once that order is fixed.

## Risks and Tradeoffs

- Risk: object-crossing heuristics can silently regress determinism. Mitigation: repeated-run order assertions.
- Risk: bridge prioritization could overreach into travel policy. Mitigation: keep the packet on ordering only.

## Open Questions

- None. The packet explicitly chooses host-side ordering before the path-optimization stage.
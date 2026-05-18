# Requirements: path-optimization-entity-ordering

## Packet Metadata

- Grouped task IDs:
  - `TASK-152` — expand `path-optimization-default` beyond comment-only output for the ordering slice
  - `TASK-152a` — deterministic nearest-neighbor ordering
  - `TASK-152d` — cross-object ordering within one layer
  - `TASK-152e` — role-aware bridge and overhang prioritization
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented`

## Problem Statement

The current layer assembly order is mostly whatever `assemble_ordered_entities()` receives from perimeter, infill, and support IRs. That is too weak for the remaining DEV-023 ordering slice. The packet narrows the work to three deterministic ordering behaviors: same-object nearest-neighbor ordering, cross-object ordering, and bridge/overhang prioritization. It intentionally leaves retract policy, tool sequencing, and cooling outside the boundary.

## In Scope

- same-object ordering by travel cost
- cross-object ordering by travel cost
- bridge and overhang prioritization at ordering time
- deterministic repeated-run coverage

## Out of Scope

- retract/no-retract policy and Z hops
- mixed-tool ordering and cooling
- final text emission
- finalization-aware travel reconciliation

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/04_host_scheduler.md`
- `docs/07_implementation_status.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp`
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp`
- `OrcaSlicerDocumented/tests/fff_print/test_extrusion_entity.cpp`

## Acceptance Summary

### Positive Cases

- Same-object nearest-neighbor ordering is applied before path optimization.
- Cross-object ordering can interleave objects instead of preserving object-isolated order.
- Bridge-sensitive entities outrank generic infill when distances are comparable.
- The resulting sequence is deterministic across repeated runs.

### Negative Cases

- Single-entity or already-optimal sequences remain unchanged.

### Measurable Outcomes

- Acceptance tests assert exact start-point (`.x`/`.y` in mm) or `object_id` order, not just vague “shorter travel.”
- Bridge prioritization is defined as: `BridgeInfill` wins when both candidates' `path.points[0]` are equidistant (within 0.001 mm) from the current position. No `OverhangWall` variant exists in `ExtrusionRole`; overhang wall prioritization is deferred to a future packet.
- Host integration proves the reordered sequence is the one consumed by the live path (`reordered_sequence_is_consumed_by_path_optimization_stage` test).

### Cross-Packet Impact

- Packet `19` builds on this stable ordering foundation for mixed-tool sequencing.
- Packet `21` uses this packet to assert non-comment-only path-ordering evidence on Benchy.

## Verification Commands

- `cargo test -p slicer-host --test path_ordering_tdd same_object_nearest_neighbor_ordering_is_applied_before_path_optimization -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd cross_object_ordering_resequences_entities_by_travel_cost -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd bridge_sensitive_entities_are_prioritized_ahead_of_generic_infill -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd path_ordering_is_deterministic_across_repeated_runs -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd reordered_sequence_is_consumed_by_path_optimization_stage -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd single_or_already_optimal_sequence_is_left_unchanged -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the next ordering behavior is isolated to one helper or host stage surface
- Postcondition: one exact ordering rule is observable on the live host path
- Falsifying check: a focused sequence assertion fails if the rule regresses
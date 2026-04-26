---
status: draft
packet: path-optimization-entity-ordering
task_ids:
  - TASK-152
  - TASK-152a
  - TASK-152d
  - TASK-152e
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: path-optimization-entity-ordering

## Goal

Replace the current mostly pass-through entity ordering on the live path with one deterministic path-ordering surface that handles nearest-neighbor ordering for same-object entities, cross-object ordering within a layer, and bridge/overhang-sensitive prioritization before the path-optimization stage emits travel policy.

## Scope Boundaries

- In scope:
  - deterministic nearest-neighbor-style ordering for same-object entities
  - deterministic cross-object ordering within one layer
  - role-aware bridge and overhang prioritization when ordering candidate entities
  - host integration proving the reordered sequence is the one consumed by the path-optimization stage
- Out of scope:
  - retract/no-retract policy and Z-hop planning (packet `15`)
  - tool ordering and cooling policy (packet `19`)
  - final text emission (packet `11`)
  - finalization-aware travel reconciliation (packet `20`)

## Prerequisites and Blockers

- Depends on:
  - packet `14` and packet `15` for the minimum viable path-optimization surface
  - the host pre-path-optimization assembly path in `layer_executor.rs`
- Unblocks:
  - packet `19` mixed-tool ordering, which assumes a stable entity ordering foundation
  - packet `21` Benchy evidence for path-optimization behavior beyond comment markers
- Activation blockers:
  - None. The packet is `draft` by default.

## Acceptance Criteria

- **Given** three same-object entities whose first points (in `path.points[0].x`, `path.points[0].y` mm) are `(0.0, 0.0)`, `(30.0, 0.0)`, and `(10.0, 0.0)` in the raw assembled order, **when** the path-ordering helper runs, **then** the resulting `LayerCollectionIR.ordered_entities[*].path.points[0].x` sequence is exactly `0.0, 10.0, 30.0` and the corresponding `topo_order` values are `0`, `1`, and `2`. | `cargo test -p slicer-host --test path_ordering_tdd same_object_nearest_neighbor_ordering_is_applied_before_path_optimization -- --exact --nocapture`
- **Given** a mixed-object layer fixture whose raw order is all object `A` entities followed by all object `B` entities but whose nearest-next travel makes `A -> B -> B -> A` cheaper, **when** the host ordering helper runs, **then** `ordered_entities[*].region_key.object_id` follows exactly `["A", "B", "B", "A"]`. | `cargo test -p slicer-host --test path_ordering_tdd cross_object_ordering_resequences_entities_by_travel_cost -- --exact --nocapture`
- **Given** one `ExtrusionRole::BridgeInfill` entity and one `ExtrusionRole::SparseInfill` entity whose `path.points[0]` start coordinates are equidistant (within 0.001 mm) from the current position, **when** the ordering helper ranks them, **then** the `BridgeInfill` entity appears at a lower index in `ordered_entities` than the `SparseInfill` entity. | `cargo test -p slicer-host --test path_ordering_tdd bridge_sensitive_entities_are_prioritized_ahead_of_generic_infill -- --exact --nocapture`
- **Given** the same layer fixture executed twice, **when** the host ordering helper runs before `Layer::PathOptimization`, **then** the resulting ordered entity sequence is byte-identical across both runs. | `cargo test -p slicer-host --test path_ordering_tdd path_ordering_is_deterministic_across_repeated_runs -- --exact --nocapture`
- **Given** a layer fixture where the host ordering helper resequences entities from their raw assembled order, **when** `Layer::PathOptimization` runs after the ordering helper, **then** the path-optimization module receives entities in the reordered sequence confirmed by asserting the first entity's `region_key.object_id` and `path.points[0].x` match the expected post-ordering values, not the pre-ordering values. | `cargo test -p slicer-host --test path_ordering_tdd reordered_sequence_is_consumed_by_path_optimization_stage -- --exact --nocapture`

## Negative Test Cases

- **Given** a layer with a single entity or an already-optimal sequence, **when** the ordering helper runs, **then** the original `ordered_entities` order is preserved unchanged. | `cargo test -p slicer-host --test path_ordering_tdd single_or_already_optimal_sequence_is_left_unchanged -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test path_ordering_tdd same_object_nearest_neighbor_ordering_is_applied_before_path_optimization -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd cross_object_ordering_resequences_entities_by_travel_cost -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd bridge_sensitive_entities_are_prioritized_ahead_of_generic_infill -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd path_ordering_is_deterministic_across_repeated_runs -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd reordered_sequence_is_consumed_by_path_optimization_stage -- --exact --nocapture`
- `cargo test -p slicer-host --test path_ordering_tdd single_or_already_optimal_sequence_is_left_unchanged -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — path-optimization responsibilities and determinism requirements
- `docs/02_ir_schemas.md` — `LayerCollectionIR`, `PrintEntity`, and `ExtrusionRole`
- `docs/04_host_scheduler.md` — host ordering and per-layer execution order
- `docs/07_implementation_status.md` — TASK-152a / TASK-152d / TASK-152e scope

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp`
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp`
- `OrcaSlicerDocumented/tests/fff_print/test_extrusion_entity.cpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
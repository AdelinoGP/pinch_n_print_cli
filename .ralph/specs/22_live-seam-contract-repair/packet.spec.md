---
status: implemented
packet: 22_live-seam-contract-repair
task_ids:
  - TASK-120c
  - TASK-151
backlog_source: docs/07_implementation_status.md
supersedes: 14-rev1_live-seam-placement-and-consumption
---

# Packet Contract: 22_live-seam-contract-repair

## Goal

Repair the live seam contract on the current `Layer::PerimetersPostProcess` and `Layer::PathOptimization` path without widening scope into PrePass planning. This packet corrects four concrete defects in the current implementation: `seam-placer` must choose from `PerimeterIR.regions[*].seam_candidates` instead of waiting for a pre-populated `resolved_seam`; `convert_perimeter_output` must apply a chosen seam only to the emitting origin region instead of broadcasting it to every bucket; seam rotation must preserve sibling wall loops in the same region instead of letting `rotated_wall_loops` replace the region with a single rotated wall; and `path_optimization_emit_layer_markers = false` must suppress all marker output.

## Scope Boundaries

- In scope:
  - `modules/core-modules/seam-placer/src/lib.rs` candidate selection and full-region wall emission
  - `crates/slicer-host/src/wit_host.rs` origin-scoped `resolved_seam` commit behavior
  - sibling-wall preservation when `rotated_wall_loops` are present
  - `modules/core-modules/path-optimization-default/src/lib.rs` honoring `path_optimization_emit_layer_markers`
  - targeted host/module regressions for the live seam slice
- Out of scope:
  - new PrePass seam planning IR or stage routing (packet `23`)
  - travel ordering, retract/no-retract, or z-hop policy (packet `15`)
  - tool ordering, cooling policy, or mixed-object path optimization (packets `19` and `20`)
  - Benchy acceptance closure beyond seam-specific evidence (`TASK-135` remains downstream)

## Prerequisites and Blockers

- Depends on:
  - existing `push-resolved-seam` and `push-reordered-wall-loop` WIT host surfaces already implemented in `crates/slicer-host/src/wit_host.rs`
  - packet `11` remaining the owner of final Orca-facing GCode text contracts
- Unblocks:
  - packet `23` prepass seam-planning design by restoring a correct apply-stage baseline
  - `TASK-135` seam-start evidence on the Benchy path
- Activation blockers: none — packet `15_live-travel-retraction-policy` was moved to `draft` (2026-04-22), removing the activation blocker that kept this packet in `draft` status

## Acceptance Criteria

- **Given** a `PerimeterRegionView` whose `seam_candidates` contain `(x=5.0, y=0.0, z=0.2, score=0.9)` and `(x=10.0, y=0.0, z=0.2, score=0.2)` on `PerimeterIR.regions[0].walls[0]`, **when** the real `Layer::PerimetersPostProcess` dispatch runs `seam-placer` with `seam_mode = "nearest"`, **then** `PerimeterIR.regions[0].resolved_seam.point.x == 10.0`, `PerimeterIR.regions[0].resolved_seam.point.y == 0.0`, `PerimeterIR.regions[0].resolved_seam.point.z == 0.2`, and `PerimeterIR.regions[0].resolved_seam.wall_index == 0`. | `cargo test -p slicer-host --test live_seam_path_tdd seam_placer_selects_lowest_effective_score_candidate -- --exact --nocapture`
- **Given** `PerimeterIR.regions[0].walls.len() == 2` and the chosen seam belongs to `PerimeterIR.regions[0].walls[0]`, **when** the same dispatch commits rotated geometry, **then** `PerimeterIR.regions[0].walls.len()` remains `2`, `PerimeterIR.regions[0].walls[0].path.points[0]` equals the chosen seam point, and `PerimeterIR.regions[0].walls[1].path.points` remains byte-identical to the pre-dispatch wall-loop order. | `cargo test -p slicer-host --test live_seam_path_tdd seam_rotation_preserves_non_target_walls -- --exact --nocapture`
- **Given** two origin-tagged perimeter regions where only region `(object_id="obj-a", region_id=0)` emits a chosen seam, **when** `convert_perimeter_output` buckets the collected wall-postprocess output, **then** only `PerimeterIR.regions[*]` matching `(object_id="obj-a", region_id=0)` receives `resolved_seam = Some(...)` and the sibling region keeps `resolved_seam = None`. | `cargo test -p slicer-host --test live_seam_path_tdd resolved_seam_is_applied_only_to_origin_region -- --exact --nocapture`
- **Given** config key `path_optimization_emit_layer_markers = false`, **when** `Layer::PathOptimization` dispatches `path-optimization-default`, **then** the layer arena emits zero deferred annotations and zero marker comments for that layer. | `cargo test -p slicer-host --test dispatch_tdd path_optimization_emit_layer_markers_false_suppresses_output -- --exact --nocapture`
- **Given** the same seam fixture executed twice with identical `seam_candidates`, wall geometry, and region origins, **when** the wall-postprocess stage commits both runs, **then** the serialized `PerimeterIR.regions[0].resolved_seam` and `PerimeterIR.regions[0].walls[*].path.points` are byte-identical across the two runs. | `cargo test -p slicer-host --test live_seam_path_tdd seam_contract_is_deterministic_across_repeated_dispatch -- --exact --nocapture`

## Negative Test Cases

- **Given** a selected seam point whose coordinates do not appear in `PerimeterIR.regions[0].walls[0].path.points`, **when** `seam-placer` attempts to rotate that wall, **then** the stage returns a fatal error naming the missing seam point and the layer arena does not commit `PerimeterIR`. | `cargo test -p slicer-host --test live_seam_path_tdd seam_candidate_missing_from_target_wall_rejects_dispatch -- --exact --nocapture`
- **Given** `push-reordered-wall-loop` receives a wall loop where `feature_flags.len() != path.points.len()`, **when** the host commits wall-postprocess output, **then** the call is rejected with `CARDINALITY_MISMATCH` and no `PerimeterIR` is set in the arena. | `cargo test -p slicer-host --test live_seam_path_tdd rotated_points_cardinality_mismatch_rejected -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test live_seam_path_tdd seam_placer_selects_lowest_effective_score_candidate -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd seam_rotation_preserves_non_target_walls -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd resolved_seam_is_applied_only_to_origin_region -- --exact --nocapture`
- `cargo test -p slicer-host --test dispatch_tdd path_optimization_emit_layer_markers_false_suppresses_output -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd seam_contract_is_deterministic_across_repeated_dispatch -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd seam_candidate_missing_from_target_wall_rejects_dispatch -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd rotated_points_cardinality_mismatch_rejected -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — claim semantics, seam-placer ownership, per-layer stage contract
- `docs/02_ir_schemas.md` — `PerimeterIR`, `PerimeterRegion`, `WallLoop`, `SeamCandidate`, `SeamPosition`
- `docs/03_wit_and_manifest.md` — `perimeter-output-builder` methods and `Layer::PathOptimization` output contract
- `docs/04_host_scheduler.md` — `Layer::PerimetersPostProcess` and `Layer::PathOptimization` routing order
- `docs/07_implementation_status.md` — `TASK-120c` and `TASK-151` backlog scope

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp`
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

---
status: implemented
packet: 23-rev1_prepass-seam-planning-orca-parity
task_ids:
  - TASK-159
backlog_source: docs/07_implementation_status.md
supersedes: 23_prepass-seam-planning-orca-parity
---

# Packet Contract: 23-rev1_prepass-seam-planning-orca-parity

## Goal

Fix `PrePass::SeamPlanning` so `seam-planner-default` receives actual mesh geometry (`MeshObjectView` with `vertices` and `triangles`) through the WIT boundary, enabling the module to produce valid seam entries during the live pipeline. The live Benchy run must see non-zero `SeamPlanIR` entries for at least one `(layer, object, region)` tuple.

## Scope Boundaries

- In scope:
  - Fix `wit/world-prepass.wit` `run-seam-planning` to accept `list<MeshObjectView>` instead of `list<object-id>`
  - Update `crates/slicer-host/src/dispatch.rs` to convert `ObjectMesh` → `MeshObjectView` via `wit_host::object_mesh_to_wit_mesh_object_view` for `PrePass::SeamPlanning`
  - Fix the `slicer-macros/src/lib.rs` seam_arm to pass `sdk_objects: Vec<MeshObjectView>` instead of `Vec<ObjectId>`
  - Fix the curvature threshold in `seam-planner-default` (lower from 0.5 to 0.2) so ordinary cube corners produce candidates
  - Verify all 7 acceptance criteria from the original packet `23` still pass, plus 2 new ones for geometry receipt
- Out of scope:
  - Any changes to `host-api.wit` (no new mesh-query services needed)
  - Any changes to `Layer::PerimetersPostProcess` or `seam-placer` apply stage
  - Travel policy, retract/unretract, or z-hop
  - Benchy acceptance gate beyond seam evidence (TASK-135 still downstream)

## Prerequisites and Blockers

- Depends on:
  - `packet 22_live-seam-contract-repair` (already implemented) — applies-stage seam contract must be correct before this fix
- Unblocks:
  - TASK-135 seam evidence on the real Benchy path
  - full `SeamPlanIR` population during live prepass

## Acceptance Criteria

- **Given** a real `MeshIR` with at least one object containing triangles, **when** `PrePass::SeamPlanning` runs with `seam-planner-default`, **then** `SeamPlanIR.entries` has at least one entry for that object. | `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture`
- **Given** a real mesh with a cube-like geometry, **when** the seam planner evaluates corners, **then** at least one corner candidate is produced with a finite position. | `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture`
- **Given** `SeamPlanIR.entries[0]` with a matching `RegionKey`, **when** `Layer::PerimetersPostProcess` dispatches `seam-placer`, **then** `PerimeterIR.regions[0].resolved_seam` is set and `PerimeterIR.regions[0].walls[chosen_candidate.wall_index].path.points[0]` equals the chosen seam vertex. | `cargo test -p slicer-host --test live_seam_path_tdd seam_plan_ir_is_injected_into_wall_postprocess_region_view -- --exact --nocapture`
- **Given** the full live pipeline with a real STL, **when** the pipeline completes, **then** at least one DEBUG line `seam_plan_ir has N entries, looking for layer=` appears in stderr with N > 0. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_prepass_seam_plan_matches_live_outer_wall_start -- --exact --nocapture 2>&1 | grep "seam_plan_ir has"`
- **Given** a `SeamPlanIR.entries[*]` whose `RegionKey` matches the first `PerimeterRegionView` of `Layer::PerimetersPostProcess`, **when** that layer stage dispatches `seam-placer`, **then** `PerimeterIR.regions[0].resolved_seam` equals `SeamPlanIR.entries[0].chosen_candidate`. | `cargo test -p slicer-host --test live_seam_path_tdd seam_plan_ir_is_injected_into_wall_postprocess_region_view -- --exact --nocapture`
- **Given** the canonical scheduler stage list, **when** `STAGE_ORDER` is evaluated, **then** `"PrePass::SeamPlanning"` sorts after `"PrePass::LayerPlanning"` and before `"PrePass::PaintSegmentation"`. | `cargo test -p slicer-host --test execution_plan_tdd prepass_seam_planning_stage_orders_between_layer_planning_and_paint_segmentation -- --exact --nocapture`
- **Given** two `SeamPlanIR.entries[*]` with the same `(global_layer_index, object_id, region_id)` key, **when** the prepass seam-planning output commits, **then** the host rejects the output as a duplicate-key contract error. | `cargo test -p slicer-host --test dispatch_tdd seam_plan_ir_rejects_duplicate_region_keys -- --exact --nocapture`
- **Given** `PrePass::SeamPlanning` runs before `LayerPlanIR` has been committed, **when** the prepass runner validates required slots, **then** it fails with `MissingRequiredPrepass { slot: LayerPlan }`. | `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_requires_layer_plan_slot -- --exact --nocapture`

## Negative Test Cases

- **Given** a `SeamPlanIR` entry whose `chosen_candidate.point` has `NaN` coordinates, **when** the entry is committed to the blackboard, **then** the blackboard commit returns an error or the entry is rejected. | `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture`
- **Given** `seam-planner-default` receives an empty `MeshObjectView` (zero vertices, zero triangles), **when** `run_seam_planning` is called, **then** it produces zero entries without panicking. | `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture`
- `cargo test -p slicer-host --test execution_plan_tdd prepass_seam_planning_stage_orders_between_layer_planning_and_paint_segmentation -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd seam_plan_ir_is_injected_into_wall_postprocess_region_view -- --exact --nocapture`
- `cargo test -p slicer-host --test dispatch_tdd seam_plan_ir_rejects_duplicate_region_keys -- --exact --nocapture`
- `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_requires_layer_plan_slot -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_prepass_seam_plan_matches_live_outer_wall_start -- --exact --nocapture`
- `cargo test -p slicer-host --test core_module_ir_access_contract_tdd seam_planner_default_declares_prepass_contract_roots -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — pipeline tiers, stage ownership, claim semantics
- `docs/02_ir_schemas.md` — `MeshIR`, `MeshObjectView`, `SeamPlanIR`, `SeamPosition`
- `docs/03_wit_and_manifest.md` — WIT world design, `world-prepass.wit` interface rules
- `docs/04_host_scheduler.md` — prepass stage routing and blackboard commit flow
- `docs/05_module_sdk.md` — SDK builder expectations, seam_arm macro behavior

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — visibility scoring algorithm (seam placement uses raycasting to compute visibility scores)
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — data structures, `SeamCandidate`, visibility scoring constants

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

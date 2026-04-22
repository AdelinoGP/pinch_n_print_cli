---
status: draft
packet: 23_prepass-seam-planning-orca-parity
task_ids:
  - TASK-159
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 23_prepass-seam-planning-orca-parity

## Goal

Introduce an explicit `PrePass::SeamPlanning` contract that computes Orca-inspired seam choices once from global mesh and layer-plan context, stores them in a new `SeamPlanIR`, and feeds those chosen seams into `Layer::PerimetersPostProcess` as an apply-only surface. This packet is intentionally separate from packet `22`: packet `22` restores the current live layer-stage seam contract, while packet `23` adds the larger scheduler/IR/WIT slice required for deeper Orca parity.

## Scope Boundaries

- In scope:
  - new `PrePass::SeamPlanning` stage inserted between `PrePass::LayerPlanning` and `PrePass::PaintSegmentation`
  - new `SeamPlanIR` blackboard surface carrying per-region planned seams
  - extension of `wit/world-prepass.wit`, `slicer-sdk`, and `slicer-macros` for seam-planning output
  - a new core module `seam-planner-default` that uses existing prepass host services to score candidates
  - host injection of planned seams into `Layer::PerimetersPostProcess` so `seam-placer` becomes apply-only
  - focused acceptance evidence tying the new prepass plan back to downstream `TASK-135` seam coverage
- Out of scope:
  - travel policy, retract/unretract, z-hop, or generic path optimization
  - tool ordering, cooling overrides, or finalization-aware travel reconciliation
  - unrelated support, wipe tower, or skirt/brim packets
  - final packet activation while packet `15` remains active

## Prerequisites and Blockers

- Depends on:
  - packet `22_live-seam-contract-repair` restoring a correct layer-stage apply contract first
  - the existing prepass host-services surface in `world-prepass.wit` (`raycast-z-down`, `surface-normal-at`, `object-bounds`)
- Unblocks:
  - stronger seam evidence under `TASK-135`
- Activation blockers:
  - packet `15_live-travel-retraction-policy` is currently `active`
  - packet `22_live-seam-contract-repair` remains the required apply-stage prerequisite

## Acceptance Criteria

- **Given** a prepass seam-planning module that emits one entry for `region_key.global_layer_index = 0`, `region_key.object_id = "obj-a"`, and `region_key.region_id = 0`, **when** `PrePass::SeamPlanning` commits its output, **then** `SeamPlanIR.entries[0].region_key.global_layer_index == 0`, `SeamPlanIR.entries[0].region_key.object_id == "obj-a"`, `SeamPlanIR.entries[0].region_key.region_id == 0`, `SeamPlanIR.entries[0].chosen_candidate.point.{x,y,z}` match the emitted seam, and `SeamPlanIR.entries[0].chosen_candidate.wall_index` matches the planned wall. | `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture`
- **Given** the canonical scheduler stage list, **when** `STAGE_ORDER` and manifest-stage validation are evaluated, **then** `"PrePass::SeamPlanning"` sorts after `"PrePass::LayerPlanning"`, before `"PrePass::PaintSegmentation"`, and is accepted as a valid prepass stage in manifest ingestion. | `cargo test -p slicer-host --test execution_plan_tdd prepass_seam_planning_stage_orders_between_layer_planning_and_paint_segmentation -- --exact --nocapture`
- **Given** a `SeamPlanIR.entries[0]` whose `region_key` matches the first `PerimeterRegionView` of `Layer::PerimetersPostProcess`, **when** that layer stage dispatches `seam-placer`, **then** `PerimeterIR.regions[0].resolved_seam` equals `SeamPlanIR.entries[0].chosen_candidate` and `PerimeterIR.regions[0].walls[chosen_candidate.wall_index].path.points[0]` equals the same chosen seam vertex. | `cargo test -p slicer-host --test live_seam_path_tdd seam_plan_ir_is_injected_into_wall_postprocess_region_view -- --exact --nocapture`
- **Given** the new core module `seam-planner-default` at `PrePass::SeamPlanning`, **when** manifest IR-access contract tests run, **then** its manifest declares read roots `MeshIR`, `SurfaceClassificationIR`, and `LayerPlanIR` and write root `SeamPlanIR`, with no undeclared layer-stage writes. | `cargo test -p slicer-host --test core_module_ir_access_contract_tdd seam_planner_default_declares_prepass_contract_roots -- --exact --nocapture`
- **Given** the Benchy seam regression fixture with prepass seam planning enabled, **when** the full seam-planning-plus-apply slice runs, **then** the evidence artifact records at least one planned seam entry in `SeamPlanIR.entries[*]` and at least one matching seam-started outer wall on the live path for the same `(global_layer_index, object_id, region_id)` tuple. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_prepass_seam_plan_matches_live_outer_wall_start -- --exact --nocapture`

## Negative Test Cases

- **Given** two `SeamPlanIR.entries[*]` with the same `(global_layer_index, object_id, region_id)` key, **when** the prepass seam-planning output commits, **then** the host rejects the output as a duplicate-key contract error and the blackboard does not commit `SeamPlanIR`. | `cargo test -p slicer-host --test dispatch_tdd seam_plan_ir_rejects_duplicate_region_keys -- --exact --nocapture`
- **Given** `PrePass::SeamPlanning` runs before `LayerPlanIR` has been committed, **when** the prepass runner validates required slots, **then** it fails with `MissingRequiredPrepass { slot: LayerPlan }` and does not invoke the guest export. | `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_requires_layer_plan_slot -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture`
- `cargo test -p slicer-host --test execution_plan_tdd prepass_seam_planning_stage_orders_between_layer_planning_and_paint_segmentation -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd seam_plan_ir_is_injected_into_wall_postprocess_region_view -- --exact --nocapture`
- `cargo test -p slicer-host --test core_module_ir_access_contract_tdd seam_planner_default_declares_prepass_contract_roots -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_prepass_seam_plan_matches_live_outer_wall_start -- --exact --nocapture`
- `cargo test -p slicer-host --test dispatch_tdd seam_plan_ir_rejects_duplicate_region_keys -- --exact --nocapture`
- `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_requires_layer_plan_slot -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — claim semantics and stage ownership
- `docs/02_ir_schemas.md` — `LayerPlanIR`, `RegionKey`, `SeamCandidate`, `SeamPosition`, and the new `SeamPlanIR` surface to be added
- `docs/03_wit_and_manifest.md` — `world-prepass` contract and manifest stage/WIT mapping
- `docs/04_host_scheduler.md` — prepass stage order and blackboard commit flow
- `docs/05_module_sdk.md` — prepass SDK builder expectations
- `docs/07_implementation_status.md` — canonical ownership under `TASK-159` plus downstream evidence obligations under `TASK-135`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp`
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.hpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

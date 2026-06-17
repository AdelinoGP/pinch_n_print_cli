# Requirements: 23_prepass-seam-planning-orca-parity

## Packet Metadata

- Grouped task IDs:
  - `TASK-159` — add `PrePass::SeamPlanning` and a canonical `SeamPlanIR` blackboard contract that injects planned seams into `Layer::PerimetersPostProcess`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

Packet `22` restores the current layer-stage seam contract, but it does not close the larger Orca parity gap: seam selection still happens too late and with too little global context. OrcaSlicer’s seam initialization logic scores candidate points against global mesh and planning context before the live wall-application phase. Pinch 'n Print currently has no prepass seam-planning stage, no blackboard slot for planned seams, and no mechanism to inject a precomputed seam choice into `Layer::PerimetersPostProcess`.

This packet defines that missing architecture slice:

1. add `PrePass::SeamPlanning` to the canonical stage order
2. add `SeamPlanIR` as a host-owned prepass artifact keyed by `RegionKey`
3. extend `world-prepass` / SDK / macro glue so a prepass module can emit planned seams
4. add `seam-planner-default` as a claim-free prepass core module
5. keep `seam-placer` as the apply-stage module, but feed it chosen seams from `SeamPlanIR` instead of rescoring live

## In Scope

- Stage-order, manifest, dispatch, and blackboard wiring for `PrePass::SeamPlanning`
- A new `SeamPlanIR` contract with deterministic `entries: Vec<SeamPlanEntry>`
- `world-prepass.wit`, `slicer-sdk`, and `slicer-macros` updates for seam-planning output
- New `modules/core-modules/seam-planner-default/` packet-owned module surface
- Host-side injection of `SeamPlanIR.entries[*].chosen_candidate` into the matching `PerimeterRegionView.resolved_seam`
- Focused Benchy evidence for the new seam-planning path

## Out of Scope

- Travel ordering, retract/no-retract policy, or z-hop decisions
- Replacing existing host-services with brand-new mesh-query APIs; this packet uses the current prepass host-service surface
- Reworking non-seam claims or unrelated prepass stages
- Closing all Benchy acceptance items outside seam planning

## Authoritative Docs

- `docs/01_system_architecture.md` — claim matrix and stage ownership
- `docs/02_ir_schemas.md` — existing IR roots and the new `SeamPlanIR` definition to add
- `docs/03_wit_and_manifest.md` — `world-prepass` and manifest stage validation
- `docs/04_host_scheduler.md` — blackboard prepass lifecycle and stage execution order
- `docs/05_module_sdk.md` — prepass-module SDK contract
- `docs/07_implementation_status.md` — backlog tie-in for `TASK-159` and downstream seam evidence support for `TASK-135`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — precomputed seam selection behavior
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — seam-planner interface-level semantics
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` and `.hpp` — candidate provenance and loop identity assumptions

## Acceptance Summary

**Measured success path:**

- `STAGE_ORDER` and manifest validation both recognize `PrePass::SeamPlanning` in the slot immediately after `PrePass::LayerPlanning`
- `SeamPlanIR.entries[*]` commits to the blackboard with deterministic `RegionKey` and `chosen_candidate` fields
- `Layer::PerimetersPostProcess` consumes the chosen seam from `SeamPlanIR` without reintroducing ad hoc live scoring
- the new core module `seam-planner-default` declares the correct prepass read/write roots
- Benchy seam evidence can trace one planned seam entry to one live seam-started outer wall on the same region key

**Explicit negative cases:**

- duplicate `RegionKey` entries are rejected at commit time
- running the stage without `LayerPlanIR` committed fails before guest invocation

## Cross-Packet Dependencies and Unblockers

- Depends on packet `22_live-seam-contract-repair` for a correct apply-stage baseline
- Implements the canonical architecture slice now tracked by `TASK-159`
- Provides the seam-planning architecture that downstream `TASK-135` evidence can consume for deeper Benchy seam validation
- Keeps `seam-placer` as the apply-stage module for compatibility; the new planner module is claim-free in this packet so the existing `seam-placer` claim remains stable per `docs/01`

## Verification Commands

- `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture`
- `cargo test -p slicer-host --test execution_plan_tdd prepass_seam_planning_stage_orders_between_layer_planning_and_paint_segmentation -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd seam_plan_ir_is_injected_into_wall_postprocess_region_view -- --exact --nocapture`
- `cargo test -p slicer-host --test core_module_ir_access_contract_tdd seam_planner_default_declares_prepass_contract_roots -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_prepass_seam_plan_matches_live_outer_wall_start -- --exact --nocapture`
- `cargo test -p slicer-host --test dispatch_tdd seam_plan_ir_rejects_duplicate_region_keys -- --exact --nocapture`
- `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_requires_layer_plan_slot -- --exact --nocapture`

## Step Completion Expectations

Each implementation step must make clear:

- which new stage, IR, or SDK surface it introduces
- which narrow test proves the new surface exists and is wired correctly
- whether the step changes packet `22`’s apply-stage behavior or only feeds it precomputed data

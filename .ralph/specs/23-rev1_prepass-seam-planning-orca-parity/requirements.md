# Requirements: 23-rev1_prepass-seam-planning-orca-parity

## Packet Metadata

- Grouped task IDs:
  - `TASK-159` — Add `PrePass::SeamPlanning` plus canonical `SeamPlanIR` blackboard contract
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Supersedes: `23_prepass-seam-planning-orca-parity`

## Problem Statement

The live `seam-planner-default` module produces **zero** `SeamPlanIR` entries during pipeline execution because three compounding bugs prevent it from ever receiving usable geometry:

1. **WIT boundary bug** (`wit/world-prepass.wit:106`): `run-seam-planning` accepts `list<object-id>` — bare string IDs — rather than `list<MeshObjectView>`. The module receives an empty/default `MeshObjectView` with zero vertices and zero triangles.

2. **Macro bug** (`crates/slicer-macros/src/lib.rs:1713`): The `seam_arm` generates `Vec<::slicer_ir::ObjectId>` instead of `Vec<MeshObjectView>`, causing a type mismatch at the `PrepassModule::run_seam_planning` call site.

3. **Threshold bug** (`modules/core-modules/seam-planner-default/src/lib.rs:144`): Curvature threshold is `0.5`; ordinary cube corners produce curvature values near `0.0`, so the threshold is never satisfied even if real geometry were somehow passed.

`MeshSegmentation` already implements the correct pattern (line 668-673 in `dispatch.rs`): it calls `wit_host::object_mesh_to_wit_mesh_object_view` for each object and passes `Vec<MeshObjectView>` through the WIT boundary. This packet mirrors that pattern for `SeamPlanning`.

## In Scope

- Fix `wit/world-prepass.wit` `run-seam-planning` to accept `list<MeshObjectView>` instead of `list<object-id>`
- Update `crates/slicer-host/src/dispatch.rs` to convert `ObjectMesh` → `MeshObjectView` via `wit_host::object_mesh_to_wit_mesh_object_view` for `PrePass::SeamPlanning`
- Fix `slicer-macros/src/lib.rs` seam_arm to pass `sdk_objects: Vec<MeshObjectView>` instead of `Vec<ObjectId>`
- Lower curvature threshold in `seam-planner-default` from `0.5` to `0.2`
- Verify all 7 original ACs from packet `23` still pass, plus 2 new geometry-receipt ACs

## Out of Scope

- Any changes to `host-api.wit` (no new mesh-query services needed)
- Any changes to `Layer::PerimetersPostProcess` or `seam-placer` apply stage
- Travel policy, retract/unretract, or z-hop
- Benchy acceptance gate beyond seam evidence (TASK-135 still downstream)

## Authoritative Docs

- `docs/01_system_architecture.md` — pipeline tiers, stage ownership, claim semantics
- `docs/02_ir_schemas.md` — `MeshIR`, `MeshObjectView`, `SeamPlanIR`, `SeamPosition`
- `docs/03_wit_and_manifest.md` — WIT world design, `world-prepass.wit` interface rules
- `docs/04_host_scheduler.md` — prepass stage routing and blackboard commit flow
- `docs/05_module_sdk.md` — SDK builder expectations, seam_arm macro behavior

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — visibility scoring algorithm
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — data structures, `SeamCandidate`, visibility scoring constants
- Deliberately not borrowing the OrcaSlicer visibility scoring algorithm — the modular slicer uses a curvature-based corner detector instead, which is the industry-standard fallback when raycasting is unavailable.

## Acceptance Summary

**Positive cases:**
- `MeshSegmentation` precedent already proves the pattern: `object_mesh_to_wit_mesh_object_view` + `list<MeshObjectView>` WIT parameter
- Live `seam-planner-default` must receive non-empty `MeshObjectView` with real vertex and triangle data for at least one object
- `SeamPlanIR.entries` must have at least one entry with finite coordinates after `PrePass::SeamPlanning`
- Curvature threshold `0.2` allows ordinary cube corners to produce candidates

**Negative cases:**
- Empty `MeshObjectView` (zero vertices, zero triangles) must not cause a panic
- `NaN` coordinates in `chosen_candidate.point` must be rejected at blackboard commit
- Duplicate `(global_layer_index, object_id, region_id)` keys must be rejected

**Measurable outcomes:**
- All 9 acceptance criteria pass (7 original + 2 new geometry-receipt ACs)
- `seam_plan_ir has N entries` DEBUG line shows N > 0 in live pipeline
- At least one corner candidate with finite position is produced

**Cross-packet impact:**
- Unblocks: `TASK-135` (seam evidence on real Benchy path)
- Prerequisites: `packet 22_live-seam-contract-repair` must be `implemented` before this packet activates (apply-stage seam contract must be correct)

## Verification Commands

- `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture`
- `cargo test -p slicer-host --test dispatch_tdd seam_plan_ir_rejects_duplicate_region_keys -- --exact --nocapture`
- `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_requires_layer_plan_slot -- --exact --nocapture`
- `cargo test -p slicer-host --test execution_plan_tdd prepass_seam_planning_stage_orders_between_layer_planning_and_paint_segmentation -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd seam_plan_ir_is_injected_into_wall_postprocess_region_view -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_prepass_seam_plan_matches_live_outer_wall_start -- --exact --nocapture`
- `cargo test -p slicer-host --test core_module_ir_access_contract_tdd seam_planner_default_declares_prepass_contract_roots -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

Each step in `implementation-plan.md` requires:
- **Precondition:** what must be true before the step
- **Postcondition:** what must be true after the step
- **Falsifying check:** the narrowest command that proves the step's postcondition

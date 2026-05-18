# Implementation Plan: 23_prepass-seam-planning-orca-parity

## Execution Rules

- Land packet `22` first; packet `23` assumes a correct apply-stage seam contract already exists
- Introduce the new prepass stage in the narrowest possible order: IR first, then scheduler/blackboard, then WIT/SDK, then module implementation, then layer-stage injection, then Benchy evidence
- Re-run the focused test added in each step before moving outward

## Steps

### Step 1: Lock the stage-order and blackboard contract in tests

- Task IDs:
  - `TASK-159`
- Objective:
  Add focused failing tests for the new stage position, missing-slot validation, and duplicate-key rejection.
- Precondition:
  The repo has no `PrePass::SeamPlanning` stage, no `SeamPlanIR`, and no seam-plan blackboard slot.
- Postcondition:
  Focused tests exist and fail for the expected reasons: unknown stage id, missing prepass slot, or absent `SeamPlanIR` commit route.
- Likely files or subsystems touched:
  - `crates/slicer-host/tests/execution_plan_tdd.rs`
  - `crates/slicer-host/tests/dispatch_tdd.rs`
  - `crates/slicer-host/tests/blackboard_layer_arena_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md`
  - `docs/07_implementation_status.md`
- OrcaSlicer refs:
  - none
- Narrow verification commands:
  - `cargo test -p slicer-host --test execution_plan_tdd prepass_seam_planning_stage_orders_between_layer_planning_and_paint_segmentation -- --exact --nocapture`
  - `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_requires_layer_plan_slot -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - At least one new stage-order test and one missing-slot test fail against the unmodified code.

### Step 2: Add `SeamPlanIR` and blackboard storage

- Task IDs:
  - `TASK-159`
- Objective:
  Introduce the new IR type and host-owned blackboard slot before routing any guest output.
- Precondition:
  Step 1 tests fail because the new slot and IR do not exist.
- Postcondition:
  `SeamPlanIR` and `SeamPlanEntry` exist in `slicer-ir`, `BlackboardPrepassSlot::SeamPlan` exists, and `Blackboard` exposes `commit_seam_plan(...)` plus `seam_plan()`.
- Likely files or subsystems touched:
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-ir/src/lib.rs`
  - `crates/slicer-host/src/blackboard.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - none
- Narrow verification commands:
  - `cargo test -p slicer-host --test dispatch_tdd seam_plan_ir_rejects_duplicate_region_keys -- --exact --nocapture`
  - `cargo test -p slicer-host --test blackboard_layer_arena_tdd seam_plan_blackboard_slot_is_write_once -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - The new IR and slot compile, and duplicate-key enforcement can target a real `SeamPlanIR` surface.

### Step 3: Add the new prepass stage to scheduler and manifest validation

- Task IDs:
  - `TASK-159`
- Objective:
  Make the host recognize `PrePass::SeamPlanning` as a canonical routed stage.
- Precondition:
  `SeamPlanIR` and the blackboard slot exist.
- Postcondition:
  `STAGE_ORDER`, manifest validation, and prepass required-slot rules all recognize `PrePass::SeamPlanning` immediately after `PrePass::LayerPlanning`.
- Likely files or subsystems touched:
  - `crates/slicer-host/src/execution_plan.rs`
  - `crates/slicer-host/src/manifest.rs`
  - `crates/slicer-host/src/prepass.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - none
- Narrow verification commands:
  - `cargo test -p slicer-host --test execution_plan_tdd prepass_seam_planning_stage_orders_between_layer_planning_and_paint_segmentation -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - A manifest declaring `stage = "PrePass::SeamPlanning"` ingests successfully and the stage sorts in the correct slot.

### Step 4: Extend `world-prepass`, SDK, and macro glue for seam-planning output

- Task IDs:
  - `TASK-159`
- Objective:
  Allow a prepass module to emit `SeamPlanEntry` records through the existing prepass world.
- Precondition:
  The host recognizes the new stage but no WIT export or SDK builder exists yet.
- Postcondition:
  `world-prepass.wit` exposes `seam-planning-output` and `run-seam-planning`, `PrepassModule` exposes `run_seam_planning(...)`, and the macro glue drains `SeamPlanningOutput` back through the WIT boundary.
- Likely files or subsystems touched:
  - `wit/world-prepass.wit`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-sdk/src/prepass_builders.rs`
  - `crates/slicer-sdk/src/prepass_types.rs`
  - `crates/slicer-sdk/src/prelude.rs`
  - `crates/slicer-sdk/src/lib.rs`
  - `crates/slicer-macros/src/lib.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/05_module_sdk.md`
- OrcaSlicer refs:
  - none
- Narrow verification commands:
  - `cargo build -p slicer-sdk`
  - `cargo build -p slicer-macros`
  - `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - A minimal seam-planning guest can emit one `SeamPlanEntry` through the real prepass boundary.

### Step 5: Add `seam-planner-default` and commit `SeamPlanIR`

- Task IDs:
  - `TASK-159`
- Objective:
  Implement the new core module and host conversion path that commits `SeamPlanIR`.
- Precondition:
  The prepass boundary can carry seam-plan output.
- Postcondition:
  `seam-planner-default` emits deterministic `SeamPlanEntry` records, `dispatch.rs` converts them into `SeamPlanIR`, and the blackboard commits exactly one seam plan.
- Likely files or subsystems touched:
  - `modules/core-modules/seam-planner-default/`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/02_ir_schemas.md`
  - `docs/03_wit_and_manifest.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`
- Narrow verification commands:
  - `cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture`
  - `cargo test -p slicer-host --test core_module_ir_access_contract_tdd seam_planner_default_declares_prepass_contract_roots -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - One committed seam-plan entry exists on the blackboard and the new module manifest declares the correct prepass roots.

### Step 6: Inject planned seams into `Layer::PerimetersPostProcess`

- Task IDs:
  - `TASK-159`
- Objective:
  Feed `SeamPlanIR.entries[*].chosen_candidate` into the existing layer-stage seam apply surface.
- Precondition:
  `SeamPlanIR` commits successfully from Step 5.
- Postcondition:
  `dispatch_layer_call` resolves the matching `RegionKey`, populates `PerimeterRegionView.resolved_seam`, and `seam-placer` applies the chosen seam without rescoring.
- Likely files or subsystems touched:
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/wit_host.rs`
  - `modules/core-modules/seam-placer/src/lib.rs`
  - `crates/slicer-host/tests/live_seam_path_tdd.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
- Narrow verification commands:
  - `cargo test -p slicer-host --test live_seam_path_tdd seam_plan_ir_is_injected_into_wall_postprocess_region_view -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - The chosen seam from `SeamPlanIR` appears unchanged in the committed `PerimeterIR` and starts the correct wall loop.

### Step 7: Add Benchy seam evidence supporting `TASK-135`

- Task IDs:
  - `TASK-159`
- Objective:
  Prove the planned seam and the live seam-started wall can be matched on the Benchy path.
- Precondition:
  Step 6 passes with focused layer-stage tests.
- Postcondition:
  Benchy evidence captures both the planned seam entry and the matching live outer-wall start for the same `(global_layer_index, object_id, region_id)`.
- Likely files or subsystems touched:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
- Authoritative docs:
  - `docs/07_implementation_status.md`
  - `docs/12_architecture_gate_metrics.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
- Narrow verification commands:
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_prepass_seam_plan_matches_live_outer_wall_start -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - The evidence test proves one prepass seam plan corresponds to one live seam-started outer wall.

### Step 8: Packet completion gate

- Task IDs:
  - `TASK-159`
- Objective:
  Re-run the packet acceptance slice and confirm the packet still belongs in draft status until backlog/active-packet blockers clear.
- Precondition:
  Steps 1-7 pass.
- Postcondition:
  Every pipe-suffixed command in `packet.spec.md` passes, packet `22` remains the layer-stage prerequisite, and the only remaining blockers are backlog ratification plus the active packet policy.
- Likely files or subsystems touched:
  - packet-local docs only if self-review adjustments are needed
- Authoritative docs:
  - `docs/07_implementation_status.md`
  - `docs/11_operational_governance_and_acceptance_gate.md`
- OrcaSlicer refs:
  - none
- Narrow verification commands:
  - all commands listed in `packet.spec.md`
  - `cargo clippy --workspace -- -D warnings`
- Cheapest falsifying check / exit condition:
  - The full packet acceptance slice passes without reopening packet `22` defects.

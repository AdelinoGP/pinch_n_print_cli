# Task Map: 23_prepass-seam-planning-orca-parity

Use this file because the packet introduces a brand-new stage, maps one primary task across multiple architecture surfaces, and still needs to show where downstream `TASK-135` evidence is produced.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | Notes |
| --- | --- | --- | --- | --- |
| `TASK-159` | `Step 1` | `docs/04_host_scheduler.md`, `docs/07_implementation_status.md` | `crates/slicer-host/tests/execution_plan_tdd.rs`, `crates/slicer-host/tests/dispatch_tdd.rs` | Lock failing tests for stage order, missing-slot validation, and duplicate-key rejection |
| `TASK-159` | `Step 2` | `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` | `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-host/src/blackboard.rs` | Add `SeamPlanIR` and the write-once blackboard slot |
| `TASK-159` | `Step 3` | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/src/execution_plan.rs`, `crates/slicer-host/src/manifest.rs`, `crates/slicer-host/src/prepass.rs` | Introduce `PrePass::SeamPlanning` and wire scheduler validation |
| `TASK-159` | `Step 4` | `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md` | `wit/world-prepass.wit`, `crates/slicer-sdk/src/*`, `crates/slicer-macros/src/lib.rs` | Extend the prepass WIT/SDK/macro surface for seam planning |
| `TASK-159` | `Step 5` | `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md` | `modules/core-modules/seam-planner-default/`, `crates/slicer-host/src/dispatch.rs` | Commit `SeamPlanIR` from a new prepass core module |
| `TASK-159` | `Step 6` | `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/src/dispatch.rs`, `modules/core-modules/seam-placer/src/lib.rs`, `crates/slicer-host/tests/live_seam_path_tdd.rs` | Inject planned seams into the apply stage without changing the layer-world seam handle |
| `TASK-159` | `Step 7` | `docs/07_implementation_status.md`, `docs/12_architecture_gate_metrics.md` | `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` | Add Benchy evidence that supports downstream `TASK-135` seam validation |
| `TASK-159` | `Step 8` | `docs/11_operational_governance_and_acceptance_gate.md` | packet-local docs and acceptance reruns | Keep the packet draft until packet `22` lands and the active-packet blocker clears |

## Why `TASK-159` Is the Right Mapping

- `TASK-159` is now the canonical docs/07 owner for the prepass seam-planning architecture slice: new stage, new IR, new module, and apply-stage injection all live under that single backlog row
- downstream `TASK-135` evidence remains relevant, but it is consumer-facing evidence of the `TASK-159` architecture rather than the owning scheduler/IR task itself

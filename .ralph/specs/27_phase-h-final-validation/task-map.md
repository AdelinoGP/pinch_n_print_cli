# Task Map: 27_phase-h-final-validation → docs/07

This packet maps to Workstream 3 task TASK-120 (Phase H final validation and Benchy acceptance).

## Task ID Mapping

| Packet Step | docs/07 Task | Notes |
|---|---|---|
| Step 1 | TASK-120 | Rebuild WASM artifacts via `build-core-modules.sh` |
| Step 2 | TASK-120, TASK-124 | Run `core_module_ir_access_contract_tdd` |
| Step 3 | TASK-120, TASK-123b | Run `pipeline_tdd` |
| Step 4 | TASK-120, TASK-145 | Run `wit_drift_detection_tdd` |
| Step 5 | TASK-120, TASK-120b | Run `live_support_generation_tdd` |
| Step 6 | TASK-120, TASK-120c | Run `live_seam_path_tdd` |
| Step 7 | TASK-120, TASK-120b | Run `benchy_end_to_end_tdd` |
| Step 10 | TASK-120, TASK-120b | Update `docs/07_implementation_status.md` TASK-120/TASK-120b status |

## Dependency Note

- Packet 27 is the terminal validation packet. It depends on Packets 24, 25, and 26 all completing before it can run.
- If any of the dependent packets fail to complete, Packet 27 cannot reach its completion gate.

## Superseding Relationship

- Packet 27 does NOT supersede any prior packet. It is the final validation gate for the four review-finding packets.

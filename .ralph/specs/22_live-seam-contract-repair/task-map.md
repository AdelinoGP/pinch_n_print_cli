# Task Map: 22_live-seam-contract-repair

Use this file because the packet spans two task IDs and explicitly supersedes packet `14-rev1_live-seam-placement-and-consumption`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | Notes |
| --- | --- | --- | --- | --- |
| `TASK-120c` | `Step 1` | `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/tests/live_seam_path_tdd.rs` | Lock failing regressions for candidate selection, region scoping, and sibling-wall preservation |
| `TASK-120c` | `Step 2` | `docs/01_system_architecture.md`, `docs/02_ir_schemas.md` | `modules/core-modules/seam-placer/src/lib.rs` | Choose from `seam_candidates` and emit the full region wall-loop set |
| `TASK-120c` | `Step 3` | `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md` | `crates/slicer-host/src/wit_host.rs` | Scope `resolved_seam` to the emitting origin region |
| `TASK-151` | `Step 1` | `docs/03_wit_and_manifest.md`, `docs/07_implementation_status.md` | `crates/slicer-host/tests/dispatch_tdd.rs` | Lock the marker-suppression config failure before changing the module |
| `TASK-151` | `Step 4` | `docs/03_wit_and_manifest.md` | `modules/core-modules/path-optimization-default/src/lib.rs` | Honor `path_optimization_emit_layer_markers` exactly |
| `TASK-120c` | `Step 5` | `docs/03_wit_and_manifest.md` | `crates/slicer-host/tests/live_seam_path_tdd.rs` | Preserve fatal validation on malformed rotated wall loops |
| `TASK-120c` / `TASK-151` | `Step 6` | `docs/07_implementation_status.md`, `docs/11_operational_governance_and_acceptance_gate.md` | `.ralph/specs/14-rev1_live-seam-placement-and-consumption/packet.spec.md` | Mark the old packet superseded and rerun the acceptance slice |

## Why This Packet Supersedes 14-rev1

Packet `14-rev1` fixed the earlier replay-at-PathOptimization idea, but it still left the live seam path in an inconsistent state:

1. the module did not actually choose from `PerimeterIR.regions[*].seam_candidates`
2. one chosen seam could leak into every origin bucket
3. rotating a single target wall could accidentally erase sibling walls because `rotated_wall_loops` replace the canonical wall list

Packet `22` narrows the repair to those concrete defects and keeps PrePass seam planning out of scope.

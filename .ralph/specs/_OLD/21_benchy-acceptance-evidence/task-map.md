# Task Map: benchy-acceptance-evidence

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-135` | Step 1 | `docs/07_implementation_status.md`, `docs/11_operational_governance_and_acceptance_gate.md` | `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`, `LayerRegion.cpp` | Adds final text evidence for support, top surface, and bottom surface on the real Benchy path. |
| `TASK-135` | Step 2 | `docs/07_implementation_status.md`, `docs/12_architecture_gate_metrics.md` | `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` | Adds retract-balance and seam-evidence assertions on the live path. |
| `TASK-135` | Step 3 | `docs/11_operational_governance_and_acceptance_gate.md`, `docs/12_architecture_gate_metrics.md` | `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` | Makes failures actionable and keeps determinism evidence intact. |
| `TASK-135` (negative) | Step 3 | `docs/11_operational_governance_and_acceptance_gate.md` | `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` | Missing feature families must produce targeted diagnostics naming the missing family. |
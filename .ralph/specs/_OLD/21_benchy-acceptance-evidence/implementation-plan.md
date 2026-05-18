# Implementation Plan: benchy-acceptance-evidence

## Execution Rules

- One atomic step at a time.
- Extend the existing real Benchy suite instead of creating a second parallel acceptance harness.

## Steps

### Step 1: Add failing support and top/bottom feature-evidence assertions

- Task IDs:
  - `TASK-135`
- Objective:
  Freeze the final text evidence for support, top surface, and bottom surface on the real Benchy path.
- Precondition:
  The producer packets for support and top/bottom fill are ready or intentionally staged to fail.
- Postcondition:
  `benchy_end_to_end_tdd.rs` contains failing feature-evidence tests for support, top surface, and bottom surface fragments.
- Files expected to change:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
- Authoritative docs:
  - `docs/07_implementation_status.md`
  - `docs/11_operational_governance_and_acceptance_gate.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
  - `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp`
- Verification:
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_support_feature_evidence -- --exact --nocapture`
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_top_and_bottom_surface_evidence -- --exact --nocapture`
- Exit condition:
  The focused Benchy feature-evidence tests exist and fail only because the upstream feature packets have not yet fully landed.

### Step 2: Add retract/unretract and seam-evidence assertions

- Task IDs:
  - `TASK-135`
- Objective:
  Extend the suite to assert balanced retract/unretract pairs and live seam evidence.
- Precondition:
  Step 1 tests are in place.
- Postcondition:
  The Benchy suite has focused failing tests for retract balance and seam evidence.
- Files expected to change:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
- Authoritative docs:
  - `docs/07_implementation_status.md`
  - `docs/12_architecture_gate_metrics.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
- Verification:
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_balanced_retract_and_unretract_pairs -- --exact --nocapture`
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_live_path_contains_resolved_seam_evidence_before_emit -- --exact --nocapture`
- Exit condition:
  Both focused tests exist and fail only because the upstream feature packets are not yet fully green.

### Step 3: Add targeted failure messages and preserve determinism

- Task IDs:
  - `TASK-135`
- Objective:
  Make the suite fail with targeted diagnostics naming the missing feature family and keep deterministic output assertions intact.
- Precondition:
  Steps 1 and 2 are in place.
- Postcondition:
  The targeted-failure test and the determinism guard are both green once the producer packets land.
- Files expected to change:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md`
  - `docs/12_architecture_gate_metrics.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- Verification:
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_feature_evidence_failures_name_the_missing_family -- --exact --nocapture`
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_mvp_content_is_deterministic -- --exact --nocapture`
- Exit condition:
  Missing-feature failures are targeted and determinism remains green.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated for `TASK-135`.
- Packet evidence is ready for TASK-140 acceptance-gate evaluation.

## Acceptance Ceremony

- Re-run all acceptance commands from `packet.spec.md`.
- Confirm the targeted diagnostics name the missing feature family if the suite is still red.
- Record any remaining packet-local risk before status changes.
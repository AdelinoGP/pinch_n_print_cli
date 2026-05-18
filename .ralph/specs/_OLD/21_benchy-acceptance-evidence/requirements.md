# Requirements: benchy-acceptance-evidence

## Packet Metadata

- Grouped task IDs:
  - `TASK-135` — add Benchy regression assertions for supports, top/bottom fills, seams, and retract/unretract pairs
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

The repo already has a real Benchy end-to-end test harness, but it stops at structural or MVP-content checks. The remaining Workstream 3 closure gap is feature evidence: once the underlying producer packets land, the acceptance suite needs to assert that the final live output actually contains support, top/bottom fill, seam, and retract/unretract evidence. This packet deliberately avoids byte-for-byte golden diffing because the repo does not stage an Orca golden Benchy artifact. Instead it chooses feature-fragment assertions on the real path.

## In Scope

- final Benchy evidence for support, top, bottom, seams, and retract/unretract pairs
- deterministic repeated-run evidence on the real Benchy path
- targeted diagnostics when a feature family is missing

## Out of Scope

- feature implementation work from packets `11` through `20`
- byte-for-byte comparison to an external Orca golden file

## Authoritative Docs

- `docs/07_implementation_status.md`
- `docs/11_operational_governance_and_acceptance_gate.md`
- `docs/12_architecture_gate_metrics.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp`
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`

## Acceptance Summary

### Positive Cases

- Benchy output contains support evidence.
- Benchy output contains top and bottom surface evidence.
- Benchy output contains balanced retract/unretract evidence.
- Benchy live path contains seam evidence before final emit.
- Benchy output remains deterministic.

### Negative Cases

- Missing feature families produce targeted diagnostics naming the missing family.

### Measurable Outcomes

- The acceptance suite asserts exact feature fragments and intermediate seam evidence, not only gross extrusion counts.

### Cross-Packet Impact

- TASK-140 cannot close honestly without this packet.

## Verification Commands

- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_support_feature_evidence -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_top_and_bottom_surface_evidence -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_balanced_retract_and_unretract_pairs -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_live_path_contains_resolved_seam_evidence_before_emit -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_feature_evidence_failures_name_the_missing_family -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_mvp_content_is_deterministic -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the relevant producer packets are green or the test is explicitly staged to fail until they are
- Postcondition: one exact feature-evidence assertion is observable on the real Benchy path
- Falsifying check: the narrowest feature-specific assertion fails with a targeted message
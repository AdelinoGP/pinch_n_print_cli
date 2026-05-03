---
status: superseded
packet: benchy-acceptance-evidence
task_ids:
  - TASK-135
backlog_source: docs/07_implementation_status.md
superseded_by: 34_retraction-mode-firmware-vs-gcode
superseded_reason: |
  AC-3 here asserted balanced M207/M208 retract/unretract pairs, but M207/M208
  are Marlin firmware-retraction *configuration* setters, not the retract action.
  OrcaSlicer parity for firmware retraction is G10/G11. Packet 34 absorbs the
  retract/unretract acceptance evidence, adds a retract_mode toggle (gcode default,
  firmware optional), reframes the failing assertion against G1 E- pairs, and
  adds a separate firmware-mode test that asserts balanced G10/G11.
---

# Packet Contract: benchy-acceptance-evidence

## Goal

Extend the real Benchy end-to-end acceptance suite so it asserts the final live output now contains support, top/bottom fill, seam, and retract/unretract evidence on the canonical Workstream 3 path, using feature-fragment assertions instead of a byte-for-byte Orca golden file.

## Scope Boundaries

- In scope:
  - real Benchy end-to-end assertions on support, top surface, bottom surface, seam, and retract/unretract evidence
  - use of the existing `resources/benchy.stl` fixture and live `modules/core-modules/` tree
  - deterministic repeated-run evidence for the same Benchy output path
- Out of scope:
  - implementing any missing feature logic from packets `11` through `20`
  - byte-for-byte comparison against an external Orca golden GCode artifact not committed to this repo

## Prerequisites and Blockers

- Depends on:
  - packet `11` for emitted feature labels and comment contract
  - packet `12` for top/bottom fill generation
  - packet `13` for live support generation
  - packet `14` for seam evidence
  - packet `15` for retract/unretract policy
- Unblocks:
  - TASK-140 architecture acceptance-gate evaluation
- Activation blockers:
  - The feature-producing packets above must land first. This packet remains `draft` until then.

## Acceptance Criteria

- **Given** the real Benchy STL and live `modules/core-modules/` tree with support generation enabled, **when** the real `slicer-host` binary runs end-to-end, **then** the emitted `.gcode` contains at least one `;TYPE:Support` block and at least one extrusion move after that block. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_support_feature_evidence -- --exact --nocapture`
- **Given** the same run, **when** the emitted `.gcode` is inspected, **then** it contains at least one `;TYPE:Top surface` block and at least one `;TYPE:Bottom surface` block. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_top_and_bottom_surface_evidence -- --exact --nocapture`
- **Given** the same run, **when** the emitted `.gcode` is inspected for travel commands, **then** the count of retract lines and unretract lines is greater than `0` and the counts are exactly equal. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_balanced_retract_and_unretract_pairs -- --exact --nocapture`
- **Given** the same live Benchy path before final text serialization, **when** the captured perimeter/path-optimization surfaces are inspected, **then** at least one resolved seam influences a replayed wall-loop start on a real layer. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_live_path_contains_resolved_seam_evidence_before_emit -- --exact --nocapture`
- **Given** two identical end-to-end Benchy runs after the feature-evidence assertions land, **when** the output is compared, **then** the resulting `.gcode` remains byte-identical across both runs. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_mvp_content_is_deterministic -- --exact --nocapture`

## Negative Test Cases

- **Given** any one of the required feature families (support, top surface, bottom surface, seam evidence, retract/unretract pairs) is missing from the final Benchy path, **when** the acceptance suite runs, **then** it fails with a targeted diagnostic naming the missing feature family instead of a generic “Benchy parity failed” message. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_feature_evidence_failures_name_the_missing_family -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_support_feature_evidence -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_top_and_bottom_surface_evidence -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_balanced_retract_and_unretract_pairs -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_live_path_contains_resolved_seam_evidence_before_emit -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_feature_evidence_failures_name_the_missing_family -- --exact --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_mvp_content_is_deterministic -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/07_implementation_status.md` — TASK-135 scope and Workstream 3 ordering
- `docs/11_operational_governance_and_acceptance_gate.md` — acceptance-gate expectations
- `docs/12_architecture_gate_metrics.md` — runtime-evidence thresholds and acceptance framing

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp`
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp`
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
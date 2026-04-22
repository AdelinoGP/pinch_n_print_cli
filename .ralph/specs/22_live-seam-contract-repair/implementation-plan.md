# Implementation Plan: 22_live-seam-contract-repair

## Execution Rules

- Stay inside the existing layer-stage seam surface; no new IR or prepass routing in this packet
- TDD first for each defect, then implementation, then the narrowest rerun of the same test
- Do not expand from seam repair into packet `15` travel policy work

## Steps

### Step 1: Lock the current failure cases with focused host tests

- Task IDs:
  - `TASK-120c`
  - `TASK-151`
- Objective:
  Add or refresh the minimal failing regressions for seam candidate selection, sibling-wall preservation, origin-scoped seam commit, and marker suppression.
- Precondition:
  Current code still uses `region.resolved_seam()` in `seam-placer`, still broadcasts `resolved_seam` across buckets, and still emits marker comments when `emit_layer_markers = false`.
- Postcondition:
  The following tests exist and fail for the current reasons described in this packet: `seam_placer_selects_lowest_effective_score_candidate`, `seam_rotation_preserves_non_target_walls`, `resolved_seam_is_applied_only_to_origin_region`, and `path_optimization_emit_layer_markers_false_suppresses_output`.
- Likely files or subsystems touched:
  - `crates/slicer-host/tests/live_seam_path_tdd.rs`
  - `crates/slicer-host/tests/dispatch_tdd.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/03_wit_and_manifest.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
- Narrow verification commands:
  - `cargo test -p slicer-host --test live_seam_path_tdd seam_placer_selects_lowest_effective_score_candidate -- --exact --nocapture`
  - `cargo test -p slicer-host --test dispatch_tdd path_optimization_emit_layer_markers_false_suppresses_output -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - At least one of the new seam tests and the marker-suppression test fail on the unmodified code for the reasons captured in this packet.

### Step 2: Repair `seam-placer` candidate selection and full-region wall emission

- Task IDs:
  - `TASK-120c`
- Objective:
  Make `run_wall_postprocess` choose from `region.seam_candidates()` and emit the full wall-loop set for the region, rotating only the selected wall.
- Precondition:
  Step 1 regressions exist and fail.
- Postcondition:
  `run_wall_postprocess` no longer depends on pre-populated `region.resolved_seam()`, calls `push_resolved_seam(...)` for the chosen candidate, and emits all region wall loops through `push_reordered_wall_loop(...)` in canonical order.
- Likely files or subsystems touched:
  - `modules/core-modules/seam-placer/src/lib.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/02_ir_schemas.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp`
- Narrow verification commands:
  - `cargo test -p slicer-host --test live_seam_path_tdd seam_placer_selects_lowest_effective_score_candidate -- --exact --nocapture`
  - `cargo test -p slicer-host --test live_seam_path_tdd seam_rotation_preserves_non_target_walls -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - The chosen seam now comes from `seam_candidates`, and sibling walls remain present after commit.

### Step 3: Scope `resolved_seam` to the emitting origin bucket in `convert_perimeter_output`

- Task IDs:
  - `TASK-120c`
- Objective:
  Stop broadcasting one chosen seam to every region bucket.
- Precondition:
  `seam-placer` can now emit chosen seams and full-region rotated walls.
- Postcondition:
  `convert_perimeter_output` only assigns `resolved_seam` to the bucket whose origin emitted the seam, and non-emitting sibling regions keep `resolved_seam = None`.
- Likely files or subsystems touched:
  - `crates/slicer-host/src/wit_host.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
- Narrow verification commands:
  - `cargo test -p slicer-host --test live_seam_path_tdd resolved_seam_is_applied_only_to_origin_region -- --exact --nocapture`
  - `cargo test -p slicer-host --test live_seam_path_tdd seam_contract_is_deterministic_across_repeated_dispatch -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - A two-region fixture commits a seam on exactly one region and remains deterministic across reruns.

### Step 4: Honor `path_optimization_emit_layer_markers` exactly

- Task IDs:
  - `TASK-151`
- Objective:
  Gate marker emission on the parsed config flag so the live path can be silent when requested.
- Precondition:
  Step 1 proved the config flag is parsed but ignored.
- Postcondition:
  `run_path_optimization` emits the comment only when `emit_layer_markers == true` and emits nothing when false.
- Likely files or subsystems touched:
  - `modules/core-modules/path-optimization-default/src/lib.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/07_implementation_status.md`
- OrcaSlicer refs:
  - none; this is a ModularSlicer-local config contract
- Narrow verification commands:
  - `cargo test -p slicer-host --test dispatch_tdd path_optimization_emit_layer_markers_false_suppresses_output -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - The focused suppression test passes with zero deferred annotations.

### Step 5: Preserve hard failure semantics for malformed rotated output

- Task IDs:
  - `TASK-120c`
- Objective:
  Confirm the repair did not relax the existing host validation on malformed rotated wall loops.
- Precondition:
  Steps 2-4 pass.
- Postcondition:
  Missing seam-point and cardinality-mismatch tests pass with fatal errors and no partial commit.
- Likely files or subsystems touched:
  - `crates/slicer-host/tests/live_seam_path_tdd.rs`
  - `modules/core-modules/seam-placer/src/lib.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
- OrcaSlicer refs:
  - none
- Narrow verification commands:
  - `cargo test -p slicer-host --test live_seam_path_tdd seam_candidate_missing_from_target_wall_rejects_dispatch -- --exact --nocapture`
  - `cargo test -p slicer-host --test live_seam_path_tdd rotated_points_cardinality_mismatch_rejected -- --exact --nocapture`
- Cheapest falsifying check / exit condition:
  - Both malformed-input tests fail closed and leave the arena empty.

### Step 6: Packet completion gate

- Task IDs:
  - `TASK-120c`
  - `TASK-151`
- Objective:
  Re-run the full packet acceptance slice and record that packet `14-rev1` is superseded by this narrower repair packet.
- Precondition:
  Steps 1-5 pass.
- Postcondition:
  All pipe-suffixed commands in `packet.spec.md` pass, packet `14-rev1` is marked superseded, and no packet-local blocker remains besides the active-packet policy.
- Likely files or subsystems touched:
  - `.ralph/specs/14-rev1_live-seam-placement-and-consumption/packet.spec.md`
  - `docs/07_implementation_status.md` only if closure notes are recorded after implementation
- Authoritative docs:
  - `docs/07_implementation_status.md`
  - `docs/11_operational_governance_and_acceptance_gate.md`
- OrcaSlicer refs:
  - none
- Narrow verification commands:
  - `cargo test -p slicer-host --test live_seam_path_tdd -- --nocapture`
  - `cargo test -p slicer-host --test dispatch_tdd path_optimization_emit_layer_markers_false_suppresses_output -- --exact --nocapture`
  - `cargo clippy --workspace -- -D warnings`
- Cheapest falsifying check / exit condition:
  - Every packet acceptance command passes without reopening PrePass seam work.

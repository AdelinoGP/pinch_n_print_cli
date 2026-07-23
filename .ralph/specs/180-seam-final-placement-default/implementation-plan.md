# Implementation Plan: 180-seam-final-placement-default

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Continuous wall projection

- Task IDs: `TASK-293`
- Objective: replace vertex-only snap in `aligned_seam_target` with continuous projection onto the nearest wall segment point, inserting a new point when the target is not on a vertex, interpolating `feature_flags` and `width_profile`, and re-closing the loop.
- **FORWARD-DEP**: `TASK-291` (`178-seam-region-aware-planning`) and `TASK-292` (`179-seam-canonical-algorithm-fidelity`) are both status: implemented. This packet's Step 1 can be implemented and unit-tested in isolation (with synthetic PerimeterRegionView fixtures mirroring packet 178's input), and AC-4's e2e test requires the full pipeline that 178 + 179 deliver.
- Postcondition: AC-1 passes (projected point is on the wall segment, flags/widths are parallel, loop is closed).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-placer/src/lib.rs` - lines 121-183 and 245-353
  - `docs/01_system_architecture.md` - lines 986-998
  - `docs/02_ir_schemas.md` - lines 1000-1075
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-placer/src/lib.rs`
  - `modules/core-modules/seam-placer/tests/seam_continuous_projection_tdd.rs` (new)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...`
  - `target/`, `Cargo.lock`, generated code, vendored dependencies
  - Packet 178 WIT/IR files
  - Packet 179 scoring/visibility/spline files
- Blast-radius discipline: not applicable — no new struct fields or schema constants in this step.
- Expected sub-agent dispatches:
  - Question: canonical `place_seam` nearest-point projection behavior — does OrcaSlicer project onto the nearest segment point (continuous) or snap to the nearest vertex? What is the exact projection formula?; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`; return: `SUMMARY` (≤200 words); purpose: inform continuous projection implementation.
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` - range 986-998
  - `docs/02_ir_schemas.md` - range 1000-1075
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` - delegate; never load
- Verification:
  - `cargo test -p seam-placer --test seam_continuous_projection_tdd -- projects_onto_nearest_segment_point 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail
- Exit condition: projected point is on the wall segment, `feature_flags` and `width_profile.widths` are parallel to `path.points` after insertion, loop is closed, empty loop produces non-fatal error without panic.

### Step 2: Degraded fallback and non-fatal reporting

- Task IDs: `TASK-293`
- Objective: when no `SeamPlanIR` entry matches an active region in aligned mode, emit `ModuleError::non_fatal` identifying the missing `(layer, object, region_id, variant_chain)` key, apply canonical local candidate selection as fallback, preserve all walls, and ensure the slice continues with degraded status.
- Precondition: Step 1 compiles (continuous projection infrastructure is in place).
- Postcondition: AC-2 passes (missing plan produces non-fatal error, walls preserved, degraded status observable).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/error.rs` - lines 1-40
  - `crates/slicer-runtime/src/progress_events.rs` - lines 121-186
  - `modules/core-modules/seam-placer/src/lib.rs` - lines 265-353
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-placer/src/lib.rs`
  - `modules/core-modules/seam-placer/tests/seam_degraded_fallback_tdd.rs` (new)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...`
  - `crates/slicer-wasm-host/**`
  - `crates/slicer-runtime/**` (except the read-only range above)
- Blast-radius discipline: not applicable — no new struct fields or schema constants in this step.
- Expected sub-agent dispatches:
  - Question: how does the existing non-fatal error progress event surface in the runtime? What is the exact `ModuleError::non_fatal` constructor signature and the progress event path?; scope: `crates/slicer-runtime/src/progress_events.rs`; return: `LOCATIONS` (file:line + 1-line context, ≤10 entries); purpose: wire degraded fallback error emission.
- Context cost: `M`
- Authoritative docs:
  - `crates/slicer-sdk/src/error.rs` - range 1-40
  - `crates/slicer-runtime/src/progress_events.rs` - range 121-186
- OrcaSlicer refs: none (degraded fallback is a PNP diagnostic extension, not a canonical feature).
- Verification:
  - `cargo test -p seam-placer --test seam_degraded_fallback_tdd -- missing_plan_emits_non_fatal_and_preserves_walls 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail
- Exit condition: missing plan produces `ModuleError::non_fatal` with the correct key, all walls are preserved, degraded status is observable through the progress event channel.

### Step 3: Default mode change and end-to-end closure

- Task IDs: `TASK-293`
- Objective: change `default = "nearest"` to `default = "aligned"` in both `seam-placer.toml` and `seam-planner-default.toml`, add an end-to-end test proving the aligned default works for multi-region prints, and verify no regression in existing nearest/rear/random suites.
- Precondition: Steps 1-2 pass (continuous projection and degraded fallback are implemented and tested).
- Postcondition: AC-3, AC-4, AC-5, AC-N2 pass (default is aligned in both manifests, e2e test passes, no regression, unknown mode still rejected).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-placer/seam-placer.toml` - full file (~44 lines)
  - `modules/core-modules/seam-planner-default/seam-planner-default.toml` - full file (~44 lines)
  - `docs/15_config_keys_reference.md` - lines 166-226
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-placer/seam-placer.toml`
  - `modules/core-modules/seam-planner-default/seam-planner-default.toml`
  - `crates/slicer-runtime/tests/e2e/scenario_traces_tdd.rs` (add a new test function to the existing registered e2e file; do NOT create a new test file)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/...`
  - Packet 178 WIT/IR files
  - Packet 179 scoring/visibility/spline files
- Blast-radius discipline: not applicable — no new struct fields or schema constants in this step.
- Expected sub-agent dispatches: none.
- Context cost: `M`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - range 166-226
- OrcaSlicer refs: none (default value is a config choice, not a canonical algorithm).
- Verification:
  - `grep -q 'default = "aligned"' modules/core-modules/seam-placer/seam-placer.toml && grep -q 'default = "aligned"' modules/core-modules/seam-planner-default/seam-planner-default.toml && grep -q '"aligned"' modules/core-modules/seam-placer/seam-placer.toml && grep -q '"aligned"' modules/core-modules/seam-planner-default/seam-planner-default.toml && echo PASS` - FACT pass/fail
  - `cargo test -p seam-placer 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail (AC-5 regression check)
  - `cargo test -p slicer-runtime --test e2e -- seam_aligned_default_e2e 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail (AC-4)
  - `cargo xtask build-guests --check` - FACT pass/fail
- Exit condition: default is `"aligned"` in both manifests, e2e test passes, existing nearest/rear/random suites pass, unknown mode rejection still works.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Continuous projection; one OrcaSlicer dispatch |
| Step 2 | M | Degraded fallback; one runtime dispatch |
| Step 3 | M | Default change + e2e; no dispatches |
| **Aggregate** | **M** | All steps fit within M band |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.

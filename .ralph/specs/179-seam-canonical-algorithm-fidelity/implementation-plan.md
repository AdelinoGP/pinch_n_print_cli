# Implementation Plan: 179-seam-canonical-algorithm-fidelity

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-292`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract; do not discover struct-literal fallout after the step.

## Steps

### Step 1: Canonical comparator and point-picking

- Task IDs: `TASK-292`
- Objective: Port `SeamComparator::is_first_better`, `is_first_not_much_worse`, `compute_angle_penalty`, `gauss`, `pick_seam_point`, `pick_nearest_seam_point_index`, `pick_random_seam_point`, and `position_hash_rand` with exact canonical constants and units. Add `layer_angle: f32` to the internal `SeamCandidate` struct. Port `EnforcedBlockedSeamPoint` enum semantics. Consume seam paint annotations to set `point_type` and `central_enforcer` before comparator use.
- Precondition: Packet 178's per-region seam-planning view is available.
- Postcondition: Unit tests prove canonical ordering for every comparator gate; `seam_planner_tdd.rs` and `seam_aligned_planning_tdd.rs` regressions pass with updated fixtures.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-planner-default/src/comparator.rs` - full file, 590 lines max.
  - `modules/core-modules/seam-planner-default/src/visibility.rs` - lines 384-463 (candidate construction).
  - `modules/core-modules/seam-planner-default/src/lib.rs` - lines 68-199 (aligned driver).
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-planner-default/src/comparator.rs`
  - `modules/core-modules/seam-planner-default/tests/seam_canonical_comparator_tdd.rs` (new)
  - `modules/core-modules/seam-planner-default/src/visibility.rs`
- Blast-radius discipline: adding `layer_angle` to `SeamCandidate` invalidates every struct literal in `comparator.rs`, `visibility.rs`, `align.rs`, and existing tests; dispatch a `LOCATIONS` worker for all `SeamCandidate {` literals before editing and include them in the same step.
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`, `target/**`, `Cargo.lock`, host crates, packet 178 WIT/IR files.
- Expected sub-agent dispatches:
  - Question: exact `SeamComparator::is_first_better`, `is_first_not_much_worse`, `compute_angle_penalty`, `gauss`, and `EnforcedBlockedSeamPoint` enum with all constants and units; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` and `SeamPlacer.hpp`; return: `SNIPPETS` (≤3 × ≤30 lines).
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct coordinate contract.
  - `docs/ORCASLICER_ATTRIBUTION.md` - direct header text.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` - delegate comparator and penalty functions.
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` - delegate struct fields and constants.
- Verification:
  - `cargo test -p seam-planner-default --test seam_canonical_comparator_tdd 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail.
  - `cargo test -p seam-planner-default 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail (regression).
- Exit condition: every comparator gate matches canonical ordering; `layer_angle` field is present and populated; paint annotations set `point_type`/`central_enforcer` before scoring.

### Step 2: Canonical visibility and overhang/embedding

- Task IDs: `TASK-292`
- Objective: Port `raycast_visibility` with canonical 30000 samples × 25 hemisphere rays per sample, using a deterministic per-object seed and canonical area-uniform sampling distribution. Port `calculate_candidates_visibility`/`calculate_point_visibility` weighted neighborhood lookup. Port `calculate_overhangs_and_layer_embedding` with `layer_angle` and resolved per-region flow width. Add a BVH or AABB tree for ray-triangle performance if needed without reducing sample/ray counts.
- Precondition: Step 1 comparator compiles with `layer_angle`.
- Postcondition: visibility scores are in canonical range, sample/ray counts match exactly, two runs are bit-identical, and flow width comes from packet 178's resolved input.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-planner-default/src/visibility.rs` - full file, 599 lines max.
  - `modules/core-modules/seam-planner-default/src/comparator.rs` - lines 79-111 (SeamCandidate struct).
  - `modules/core-modules/seam-planner-default/src/contours.rs` - lines 254-299 (signed-distance helpers).
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-planner-default/src/visibility.rs`
  - `modules/core-modules/seam-planner-default/tests/seam_canonical_visibility_tdd.rs` (new)
  - `modules/core-modules/seam-planner-default/src/lib.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`, `target/**`, host crates, packet 178 WIT/IR files.
- Expected sub-agent dispatches:
  - Question: `raycast_visibility` sampling scheme, ray directions, sample/ray counts, and visibility fold-in formula; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`; return: `SUMMARY` (≤200 words).
  - Question: `calculate_overhangs_and_layer_embedding` distance conventions and `layer_angle` usage; scope: same file; return: `SUMMARY`.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct coordinate contract.
  - `docs/ORCASLICER_ATTRIBUTION.md` - direct header text.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` - delegate visibility and overhang functions.
- Verification:
  - `cargo test -p seam-planner-default --test seam_canonical_visibility_tdd 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail.
- Exit condition: canonical sample/ray counts are exact, determinism holds, and flow width is sourced from resolved input.

### Step 3: Canonical chaining, retry, and gap anchor

- Task IDs: `TASK-292`
- Objective: Port `find_next_seam_in_layer`, `find_seam_string`, and `align_seam_points` with the alternative-start retry loop (step size `1 + size/20`, keep longest string). Add the bounded continuity anchor for active-region gaps: no inactive-layer entries, last real seam retained, canonical `seam_align_tolerable_dist_factor * flow_width` resume search, new string when no candidate qualifies. Port `curling_influence` using `layer_angle`.
- Precondition: Step 2 visibility produces canonical candidates.
- Postcondition: short strings trigger retry, gap-bridged regions use the bounded anchor, and `curling_influence` is canonical.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-planner-default/src/align.rs` - full file, 662 lines max.
  - `modules/core-modules/seam-planner-default/src/comparator.rs` - lines 145-300 (comparator predicates).
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-planner-default/src/align.rs`
  - `modules/core-modules/seam-planner-default/tests/seam_canonical_alignment_tdd.rs` (new)
  - `modules/core-modules/seam-planner-default/src/lib.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`, `target/**`, host crates, spline solver (Step 4).
- Expected sub-agent dispatches:
  - Question: `align_seam_points` alternative-start retry loop, `find_seam_string` bidirectional walk, `curling_influence` computation, and `seam_align_*` constants; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`; return: `SUMMARY` + ≤3 `SNIPPETS` ≤30 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - direct coordinate contract.
  - `docs/ORCASLICER_ATTRIBUTION.md` - direct header text.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` - delegate alignment functions.
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` - delegate alignment constants.
- Verification:
  - `cargo test -p seam-planner-default --test seam_canonical_alignment_tdd 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail.
- Exit condition: alternative-start retry finds longer strings, gap anchor bridges correctly, and determinism holds.

### Step 4: Full-pivot B-spline solver

- Task IDs: `TASK-292`
- Objective: Replace the normal-equation Gaussian elimination solver with unconditional `faer::linalg::solvers::ColPivQr`, the canonical full-pivot Householder QR equivalent. Enforce AC-N1 pivot-threshold zeroing and non-finite-result sanitization on the way out. Never use normal equations. Port `CubicBSplineKernel` and `fit_cubic_bspline` faithfully from `Curves.hpp`/`Bicubic.hpp`.
- Precondition: Step 3 alignment produces real observations for fitting.
- Postcondition: the solver produces canonical rank-deficient handling (zero for rank-deficient control points, not NaN/inf) and the fitted curve matches canonical within float tolerance.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-planner-default/src/align.rs` - lines 298-486 (existing solver and B-spline).
  - `modules/core-modules/seam-planner-default/Cargo.toml` - full file.
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-planner-default/src/align.rs`
  - `modules/core-modules/seam-planner-default/Cargo.toml`
  - `modules/core-modules/seam-planner-default/tests/seam_canonical_spline_tdd.rs` (new)
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`, `target/**`, host crates.
- Expected sub-agent dispatches:
  - Question: does `faer` compile for `wasm32-unknown-unknown` and expose Householder QR with the needed pivoting; scope: `faer` docs and a guest build spike; return: `FACT`.
  - Question: `fit_curve`/`fit_cubic_bspline`/`CubicBSplineKernel` algorithm and `fullPivHouseholderQr` solve; scope: `OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp` and `Bicubic.hpp`; return: `SUMMARY`.
- Context cost: `M`
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` - delegated guest dependency policy.
  - `docs/ORCASLICER_ATTRIBUTION.md` - direct header text.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp` - delegate `fit_curve` and `fullPivHouseholderQr`.
  - `OrcaSlicerDocumented/src/libslic3r/Geometry/Bicubic.hpp` - delegate `CubicBSplineKernel`.
- Verification:
  - `cargo test -p seam-planner-default --test seam_canonical_spline_tdd 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail.
  - `cargo xtask build-guests --check` - FACT pass/fail.
- Exit condition: production uses `faer::linalg::solvers::ColPivQr`, rank-deficient control points are zero, non-finite results are sanitized, and the guest builds successfully.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Comparator port with `layer_angle` and paint priority. |
| Step 2 | M | Canonical visibility and overhang/embedding. |
| Step 3 | M | Chaining, retry, and gap anchor. |
| Step 4 | M | Full-pivot QR solver and B-spline port. |

Split before activation if any step becomes L or if the packet's aggregate exceeds M.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` receives the `TASK-292` crosswalk through a worker dispatch.
- `D-168-SEAM-PREPASS-SOURCE` algorithm reductions are closed with evidence; source-geometry reduction remains for packet 180's final projection mitigation.
- `packet.spec.md` is ready for `status: implemented` only after packet 180 can consume its canonical seam target.

## Acceptance Ceremony

- Re-dispatch every AC and packet-level gate command.
- Re-run `cargo xtask build-guests --check` after all module edits.
- Record the exact canonical constants ported, the settled `faer::linalg::solvers::ColPivQr` production solver, AC-N1 enforcement, and any remaining performance limitation.
- Confirm context stayed within the standard packet budget.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the command supports that flag.

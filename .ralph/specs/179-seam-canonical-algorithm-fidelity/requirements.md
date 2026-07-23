# Requirements: 179-seam-canonical-algorithm-fidelity

## Packet Metadata

- Grouped task IDs: `TASK-292`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented`
- Aggregate context cost: `M`

## Problem Statement

Packet 168 ported the aligned seam algorithm but shipped with documented
reductions: 2000×9 visibility samples vs canonical 30000×25, Halton
low-discrepancy sampling vs canonical RNG, normal-equation Gaussian elimination
vs Eigen `fullPivHouseholderQr`, fixed `curling_influence = 1.0` (no
`layer_angle` field), no alternative-start retry for short strings, and a
hardcoded `0.4 mm` flow width. These reductions are recorded in
`D-168-SEAM-PREPASS-SOURCE`. This packet closes those algorithm reductions by
restoring canonical behavior, using packet 178's per-region input view to supply
real flow width, seam paint annotations, and per-region polygon candidates.

## In Scope

- Port canonical `SeamComparator::is_first_better` and `is_first_not_much_worse` including `central_enforcer` priority, `EnforcedBlockedSeamPoint` ordering, overhang gate, embedded-distance gate, rear-Y branch, and `spRear`/`spAligned`/`spAlignedBack`/`spNearest`/`spRandom` setup behavior.
- Port canonical `compute_angle_penalty`, `gauss`, and all scoring constants with exact values and units from `SeamPlacer.hpp`.
- Port canonical `raycast_visibility` with 30000 samples × 25 hemisphere rays per sample, using a deterministic per-object seed and canonical area-uniform sampling distribution (not Halton).
- Port canonical `calculate_candidates_visibility` (weighted neighborhood lookup) and `calculate_overhangs_and_layer_embedding` (signed-distance overhang/embedding penalties).
- Add `layer_angle` to the internal `SeamCandidate` struct and port canonical `curling_influence` from `align_seam_points`.
- Consume seam enforcer/blocker segment annotations before candidate construction; set `point_type` and `central_enforcer` per canonical `EnforcedBlockedSeamPoint` semantics.
- Port canonical `align_seam_points` alternative-start retry loop (step size `1 + size/20`, keep longest string).
- Add bounded continuity anchor for active-region gaps: no inactive-layer entries, last real seam retained, canonical `seam_align_tolerable_dist_factor * flow_width` resume search, new string when no candidate qualifies.
- Replace normal-equation Gaussian elimination with `faer::linalg::solvers::ColPivQr` (the canonical full-pivot Householder QR equivalent), with AC-N1 pivot-threshold zeroing and non-finite-result sanitization enforced on the way out; no local fallback or normal equations.
- Use the resolved per-active-region outer-wall scoring width from packet 178 instead of the hardcoded `0.4 mm` default.
- Port `pick_seam_point`, `pick_nearest_seam_point_index`, `pick_random_seam_point` canonical selection logic.
- Add unit tests proving canonical ordering for every comparator gate, deterministic visibility, retry behavior, gap-anchor behavior, painted priority, solver rank handling, and flow-width sourcing.

## Out of Scope

- WIT input contract changes, host scheduling, and perimeter-region identity; packet 178 owns those.
- Continuous final-wall projection, path-point insertion, default-mode change, and degraded fallback diagnostics; packet 180 owns those.
- Changes to OrcaSlicer source.
- Host-native alignment policy.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — `SeamComparator::is_first_better`, `is_first_not_much_worse`, `are_similar`, `compute_angle_penalty`, `gauss`, `raycast_visibility`, `calculate_candidates_visibility`, `calculate_point_visibility`, `calculate_overhangs_and_layer_embedding`, `find_next_seam_in_layer`, `find_seam_string`, `align_seam_points` (including alternative-start retry loop), `pick_seam_point`, `pick_nearest_seam_point_index`, `pick_random_seam_point`.
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — `SeamCandidate` struct fields, `Perimeter` struct, `EnforcedBlockedSeamPoint` enum, all scoring constants with units.
- `OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp` — `fit_curve`, `fit_cubic_bspline`, `CubicBSplineKernel`, `PiecewiseFittedCurve::get_fitted_value`, `T.fullPivHouseholderQr()` solve.
- `OrcaSlicerDocumented/src/libslic3r/Geometry/Bicubic.hpp` — `CubicBSplineKernel` basis coefficients and `kernel_span`.

## Acceptance Summary

- Positive: `AC-1` through `AC-8` prove canonical comparator ordering, canonical visibility constants, alternative-start retry, bounded gap anchor, painted seam priority, full-pivot solver, resolved flow width, and attribution headers.
- Negative: `AC-N1` through `AC-N2` prove rank-deficient solver handling and determinism.
- Cross-packet impact: packet 180 consumes the canonical seam target and fallback semantics; packet 178's per-region view supplies the inputs.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p seam-planner-default --test seam_canonical_comparator_tdd` | Canonical comparator ordering and painted priority | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p seam-planner-default --test seam_canonical_visibility_tdd` | Canonical visibility constants, determinism, flow width | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p seam-planner-default --test seam_canonical_alignment_tdd` | Alternative-start retry and bounded gap anchor | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p seam-planner-default --test seam_canonical_spline_tdd` | Full-pivot QR solver and rank-deficient handling | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo xtask build-guests --check` | Guest artifact freshness after module edits | FACT pass/fail |
| `cargo check --workspace --all-targets` | Compilation including any `faer` dependency | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint gate | FACT pass/fail |

## Step Completion Expectations

Canonical algorithmic behavior is the invariant shared by every step. No step
may retain a reduced substitute (lower sample count, normal equations, missing
retry, fixed curling influence, or hardcoded flow width) as a temporary
compatibility shortcut.

## Context Discipline Notes

OrcaSlicer source reads must be delegated; the implementer must never load
`SeamPlacer.cpp`, `SeamPlacer.hpp`, `Curves.hpp`, or `Bicubic.hpp` directly. The
`faer` 0.24.4 dependency decision is settled by the production guest-build gate;
`cargo xtask build-guests --check` remains required after module edits.

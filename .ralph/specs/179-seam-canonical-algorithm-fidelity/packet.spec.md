---
status: draft
packet: 179-seam-canonical-algorithm-fidelity
task_ids:
  - TASK-282
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 179-seam-canonical-algorithm-fidelity

## Goal

Restore canonical OrcaSlicer seam comparator, seeded visibility sampling, seam-string retry with bounded gap anchoring, painted seam enforcer/blocker priority, prepass scoring width, and full-pivot B-spline fitting inside `seam-planner-default`, preferring `faer` with a local exact fallback.

## Scope Boundaries

This packet consumes packet 1's per-region seam-planning view and variant-aware identity to replace packet 168's reduced algorithm substitutes with canonical behavior. It does not change the WIT input contract, the host scheduling, the perimeter-region identity, or the final wall projection; those belong to packets 1 and 3. It adds `faer` as a guest dependency or falls back to a local full-pivot Householder QR.

## Prerequisites and Blockers

- Depends on: `TASK-281` (packet 1) generating the per-region seam-planning view and variant-aware `SeamPlanEntry`.
- Unblocks: `TASK-283` and the final placement/default packet.
- Activation blockers: none known; packet remains draft until preflight and guest freshness gates pass.

## Acceptance Criteria

- **AC-1. Given** the canonical `SeamComparator` predicates ported from OrcaSlicer `SeamPlacer.cpp`, **when** unit tests compare concave vs convex candidates, enforced vs blocked vs neutral, overhanging vs supported, and rear vs front, **then** `is_first_better` and `is_first_not_much_worse` return the same ordering as canonical for every gate, including `central_enforcer` priority for `Aligned`/`AlignedBack`. | `cargo test -p seam-planner-default --test seam_canonical_comparator_tdd 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-2. Given** canonical visibility constants (`raycasting_visibility_samples_count = 30000`, `sqr_rays_per_sample_point = 5` → 25 rays per sample), **when** `compute_global_visibility` runs on a fixed mesh with a stable per-object seed, **then** every sample's visibility score is in `[0, 1]` (or `[0, 2]` for `AlignedBack`), the sample count and ray count match canonical exactly, and two consecutive runs produce bit-identical results. | `cargo test -p seam-planner-default --test seam_canonical_visibility_tdd 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-3. Given** a seam string shorter than `SEAM_ALIGN_MINIMUM_STRING_SEAMS` (6) from the initial start, **when** `align_seam_points` runs, **then** it retries from alternative starts spaced `1 + size/20` apart, keeps the longest string, and only finalizes perimeters whose final string length meets the minimum. | `cargo test -p seam-planner-default --test seam_canonical_alignment_tdd -- alternative_start_retry_finds_longer_string 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-4. Given** an active region that disappears for one or more layers and reappears, **when** alignment chains across the gap, **then** no `SeamPlanEntry` is emitted for inactive layers, the last real seam is retained as a continuity anchor, the next layer's search uses canonical `seam_align_tolerable_dist_factor * flow_width` radius, and a new string starts when no candidate qualifies. | `cargo test -p seam-planner-default --test seam_canonical_alignment_tdd -- bounded_continuity_anchor_bridges_gap 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-5. Given** seam enforcer and blocker segment annotations on a region's contour, **when** candidates are built, **then** blocked candidates are excluded, enforced candidates carry `Enforced` type and `central_enforcer` is set for the central region, and both participate in `SeamComparator` before angle/visibility scoring. | `cargo test -p seam-planner-default --test seam_canonical_comparator_tdd -- painted_seam_priority_before_chaining 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-6. Given** a weighted least-squares B-spline fit over real observations, **when** the design matrix is solved, **then** the solver uses `faer` Householder QR with full pivoting or, if `faer` is unavailable in the guest, falls back to a local full-pivot Householder QR implementation, never to `ColPivQR`, `FullPivLU`, or normal equations. | `cargo test -p seam-planner-default --test seam_canonical_spline_tdd 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-7. Given** a non-0.4-mm nozzle config, **when** `build_seam_candidates` scores overhang and embedding, **then** the `flow_width` used is the resolved per-active-region outer-wall scoring width from packet 1's input, not a hardcoded `0.4`. | `cargo test -p seam-planner-default --test seam_canonical_visibility_tdd -- flow_width_from_resolved_config 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-8. Given** the `SeamPlannerDefault` module, **when** grepped, **then** the file begins with the standard OrcaSlicer attribution header including `Original C++ source path:`. | `grep -q 'Original C++ source path' modules/core-modules/seam-planner-default/src/comparator.rs && grep -q 'Original C++ source path' modules/core-modules/seam-planner-default/src/visibility.rs && grep -q 'Original C++ source path' modules/core-modules/seam-planner-default/src/align.rs && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a candidate set where the solver would be rank-deficient, **when** the B-spline fit runs, **then** rank-deficient control points are set to zero (matching canonical `fullPivHouseholderQr` rank handling), not propagated as `NaN` or `inf`. | `cargo test -p seam-planner-default --test seam_canonical_spline_tdd -- rank_deficient_fit_produces_zero_not_nan 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-N2. Given** two runs of `align_seam_points` on the same candidate fixture, **when** outputs are compared, **then** the finalized perimeters, seam indices, and final positions are bit-identical, proving determinism. | `cargo test -p seam-planner-default --test seam_canonical_alignment_tdd -- alignment_is_deterministic 2>&1 | tee target/test-output.log | grep '^test result'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated document-map for normative architecture docs.
- `docs/02_ir_schemas.md` - delegated `SeamPlanIR` and `SeamCandidate` identity locations.
- `docs/08_coordinate_system.md` - direct coordinate contract; seam data path is f32 millimetres, not integer units.
- `docs/11_operational_governance_and_acceptance_gate.md` - delegated guest dependency policy and closure-gate locations.
- `docs/adr/0046-aligned-seam-in-seam-planning-prepass.md` - accepted prepass placement decision.
- `docs/DEVIATION_LOG.md` - `D-168-SEAM-PREPASS-SOURCE` predecessor deviation; this packet closes its enumerated algorithm reductions.

## Doc Impact Statement (Required)

- `docs/05_module_sdk.md` seam-candidate convention and painted seam priority - `rg -q 'seam_enforcer|central_enforcer|SeamComparator' docs/05_module_sdk.md`
- `docs/05_module_sdk.md` seam-candidate convention and paint priority - `rg -q 'seam_enforcer|central_enforcer|SeamComparator' docs/05_module_sdk.md`
- `docs/15_config_keys_reference.md` `seam_mode` values and scoring width - `rg -q 'seam_mode|flow_width.*seam' docs/15_config_keys_reference.md`
- `docs/DEVIATION_LOG.md` closure of `D-168-SEAM-PREPASS-SOURCE` algorithm reductions - `rg -q 'D-168-SEAM-PREPASS-SOURCE' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — canonical `SeamComparator::is_first_better`, `is_first_not_much_worse`, `compute_angle_penalty`, `gauss`, `raycast_visibility`, `calculate_candidates_visibility`, `calculate_overhangs_and_layer_embedding`, `find_next_seam_in_layer`, `find_seam_string`, `align_seam_points` (including alternative-start retry loop), `pick_seam_point`, `pick_nearest_seam_point_index`, `pick_random_seam_point`.
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — `SeamCandidate` struct fields (visibility, overhang, embedded_distance, local_ccw_angle, central_enforcer, point_type, layer_angle), `Perimeter` struct, `angle_importance_aligned`/`angle_importance_nearest` constants, `seam_align_score_tolerance`, `seam_align_tolerable_dist_factor`, `seam_align_minimum_string_seams`, `seam_align_mm_per_segment`, `sharp_angle_snapping_threshold`.
- `OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp` — `fit_curve`, `fit_cubic_bspline`, `CubicBSplineKernel`, `PiecewiseFittedCurve::get_fitted_value`, and the `T.fullPivHouseholderQr()` solve.
- `OrcaSlicerDocumented/src/libslic3r/Geometry/Bicubic.hpp` — `CubicBSplineKernel` basis coefficients and `kernel_span = 4`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
# Design: 179-seam-canonical-algorithm-fidelity

## Controlling Code Paths

- Primary code path: `SeamPlannerDefault::run_aligned_planning` -> `build_seam_candidates` (visibility + overhang + embedding) -> `pick_seam_point` (comparator) -> `align_seam_points` (chaining + retry + spline) -> `SeamPlanEntry` emission. Files: `modules/core-modules/seam-planner-default/src/{comparator.rs,visibility.rs,align.rs,lib.rs}`.
- Neighboring tests/fixtures: `seam_canonical_comparator_tdd`, `seam_canonical_visibility_tdd`, `seam_canonical_alignment_tdd`, `seam_canonical_spline_tdd` (all new), plus existing `seam_planner_tdd.rs` and `seam_aligned_planning_tdd.rs` regression suites.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

- All canonical scoring constants must be ported with exact values and units; no reduced substitutes are permitted under the algorithmic parity target.
- The production solver is `faer::linalg::solvers::ColPivQr`, the canonical `fullPivHouseholderQr` equivalent. No local fallback, `FullPivLU`, or normal-equation substitute is permitted.
- Determinism is mandatory: visibility sampling must use a stable per-object seed derived from object identity, not OrcaSlicer's process-wide RNG; two consecutive runs on the same input must produce bit-identical results.
- `layer_angle` must be added to the internal `SeamCandidate` struct so canonical `curling_influence` can be computed; the existing fixed `1.0` substitution is removed.
- Seam paint annotations (enforcer/blocker) must participate before cross-layer chaining, matching canonical `EnforcedBlockedSeamPoint` priority.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Caveat for this packet: the seam data path (`SeamCandidate.position`, `Point3WithWidth`, visibility distances, flow width) is f32 millimetres, not integer units. OrcaSlicer constants already in mm (angles in radians, dimensionless weights) pass through unchanged. State the unit in a comment beside every ported constant.

## Code Change Surface

- Selected approach: replace packet 168's reduced algorithm substitutes in `comparator.rs`, `visibility.rs`, and `align.rs` with faithful canonical ports. Add `layer_angle` to the internal `SeamCandidate` struct. Use `faer::linalg::solvers::ColPivQr` as the production full-pivot Householder QR solver. Consume seam paint annotations from packet 178's per-region input. Port `align_seam_points` alternative-start retry. Add bounded continuity anchor for active-region gaps.
- Exact functions, traits, manifests, tests, and fixtures:
  - `comparator.rs`: port `SeamComparator::is_first_better`, `is_first_not_much_worse`, `compute_angle_penalty`, `gauss`, `pick_seam_point`, `pick_nearest_seam_point_index`, `pick_random_seam_point`, `position_hash_rand`; add `layer_angle` field to `SeamCandidate`; port `EnforcedBlockedSeamPoint` semantics.
  - `visibility.rs`: port `raycast_visibility` with 30000×25 canonical budget and seeded canonical sampling; port `calculate_candidates_visibility`/`calculate_point_visibility`; port `calculate_overhangs_and_layer_embedding` with `layer_angle`; consume seam enforcer/blocker annotations; use resolved flow width.
  - `align.rs`: port `find_next_seam_in_layer`, `find_seam_string`, `align_seam_points` with alternative-start retry; add bounded continuity anchor; replace `solve_gaussian` with `faer::linalg::solvers::ColPivQr`; port `CubicBSplineKernel` and `fit_cubic_bspline` faithfully.
  - `lib.rs`: wire `run_aligned_planning` to consume packet 178's per-region input and pass resolved flow width + annotations through.
  - New test files: `seam_canonical_comparator_tdd.rs`, `seam_canonical_visibility_tdd.rs`, `seam_canonical_alignment_tdd.rs`, `seam_canonical_spline_tdd.rs`.
  - `modules/core-modules/seam-planner-default/Cargo.toml`: add `faer` as a regular production dependency.
- Rejected alternatives and reasons:
  - Keep reduced visibility budget: violates algorithmic parity target.
  - Keep normal-equation solver: violates algorithmic parity target; numerically unstable for rank-deficient design matrices.
  - Use a non-`faer` solver: unnecessary after the `ColPivQr` production decision and its verified guest build.
  - Skip alternative-start retry: loses canonical longer-string selection for short initial strings.
  - Fixed `curling_influence = 1.0`: ignores canonical `layer_angle` influence on seam-string weight.
  - Hardcoded `0.4 mm` flow width: non-canonical for non-default nozzle configs.

## Files in Scope (read + edit)

- `modules/core-modules/seam-planner-default/src/comparator.rs` - role: canonical comparator and point-picking; expected change: faithful port with `layer_angle` and `EnforcedBlockedSeamPoint`.
- `modules/core-modules/seam-planner-default/src/visibility.rs` - role: canonical visibility and overhang/embedding; expected change: 30000×25 seeded sampling, resolved flow width, paint annotations.
- `modules/core-modules/seam-planner-default/src/align.rs` - role: canonical chaining, retry, spline; expected change: alternative-start retry, gap anchor, full-pivot QR solver.

## Read-Only Context

- `modules/core-modules/seam-planner-default/src/lib.rs` - lines 68-199 only - aligned driver wire-up.
- `modules/core-modules/seam-planner-default/tests/seam_aligned_planning_tdd.rs` - full file - regression fixture idioms.
- `crates/slicer-sdk/src/prepass_types.rs` - lines 240-304 - packet 178's new input view types (consumed, not edited).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load directly.
- `target/`, `Cargo.lock`, generated code, and vendored dependencies - never load.
- WIT/IR identity and host scheduling - packet 178.
- Continuous wall projection and default-mode changes - packet 180.
- `crates/slicer-runtime/**`, `crates/slicer-wasm-host/**` - delegate symbol lookups only; do not browse.

## Expected Sub-Agent Dispatches

- Question: exact `SeamComparator::is_first_better` and `is_first_not_much_worse` predicate bodies with all constants and units; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`; return: `SNIPPETS` (≤3 × ≤30 lines); purpose: Step 1 comparator port.
- Question: `raycast_visibility` sampling scheme, ray directions, sample/ray counts, and visibility formula; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`; return: `SUMMARY` (≤200 words); purpose: Step 2 visibility port.
- Question: `align_seam_points` alternative-start retry loop, `find_seam_string` bidirectional walk, `curling_influence` computation, and `seam_align_*` constants; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`; return: `SUMMARY` + ≤3 `SNIPPETS` ≤30 lines; purpose: Step 3 alignment port.
- Question: `fit_curve`/`fit_cubic_bspline`/`CubicBSplineKernel` algorithm and `fullPivHouseholderQr` solve; scope: `OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp` and `Bicubic.hpp`; return: `SUMMARY`; purpose: Step 4 spline port.
- Question: does `faer` compile for `wasm32-unknown-unknown` and expose the needed QR infrastructure; scope: `faer` docs and a guest build spike; return: `FACT`; purpose: Step 4 dependency decision.

## Data and Contract Notes

- IR/manifest contracts: the internal `SeamCandidate` struct in `comparator.rs` gains a `layer_angle: f32` field; this is module-local and does not cross WIT or IR boundaries.
- WIT boundary: no WIT changes in this packet; packet 178's input view is consumed as-is.
- Determinism/scheduler constraints: visibility sampling must be deterministic across runs; the seed is derived from a stable object identifier, not from process memory or thread timing.

## Locked Assumptions and Invariants

- Canonical visibility constants are 30000 samples × 25 rays per sample; no reduced budget is acceptable.
- The solver is `faer::linalg::solvers::ColPivQr`, the canonical full-pivot Householder QR equivalent; no local fallback, `FullPivLU`, or normal-equation substitute is acceptable.
- Alternative-start retry is mandatory for strings shorter than `SEAM_ALIGN_MINIMUM_STRING_SEAMS`.
- Bounded continuity anchor is a PNP extension to canonical gap handling; it is documented as such, not claimed as canonical.
- Seam paint annotations participate before chaining, matching canonical `EnforcedBlockedSeamPoint` priority.
- Flow width comes from packet 178's resolved per-active-region scoring width, not a hardcoded default.

## Risks and Tradeoffs

- `faer` 0.24.4 guest compatibility is settled by the workspace guest-build verification; the production path is unconditional.
- 30000×25 visibility sampling is computationally expensive in WASM; a BVH or AABB tree may be needed for large meshes, but the sample/ray counts must not be reduced.
- Adding `layer_angle` to the internal struct changes the module's test fixtures; all existing `seam_planner_tdd.rs` and `seam_aligned_planning_tdd.rs` assertions must be updated in the same step.
- The alternative-start retry loop changes alignment output for fixtures that previously produced unfinalized short strings; existing regression tests may need updated expected values.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk closure evidence: `cargo xtask build-guests --check` confirms the settled `faer` production dependency.

## Open Questions

- `[FWD]` The implementer may choose the exact deterministic seeding scheme (hash of object ID + mesh vertex count, or a canonical per-print seed) provided it is stable across runs and process restarts.
- `[FWD]` The implementer may choose whether to add a BVH/AABB tree for visibility raycasting performance, provided the sample/ray counts and scoring formula remain canonical.

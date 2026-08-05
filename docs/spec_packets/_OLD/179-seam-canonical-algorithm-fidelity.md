---
status: implemented
packet: 179-seam-canonical-algorithm-fidelity
task_ids:
  - TASK-292
---

# 179-seam-canonical-algorithm-fidelity

## Goal

Restore canonical OrcaSlicer seam comparator, seeded visibility sampling, seam-string retry with bounded gap anchoring, painted seam enforcer/blocker priority, prepass scoring width, and full-pivot B-spline fitting inside `seam-planner-default`, using `faer::linalg::solvers::ColPivQr` unconditionally.

## Problem Statement

Packet 168 ported the aligned seam algorithm but shipped with documented
reductions: 2000×9 visibility samples vs canonical 30000×25, Halton
low-discrepancy sampling vs canonical RNG, normal-equation Gaussian elimination
vs Eigen `fullPivHouseholderQr`, fixed `curling_influence = 1.0` (no
`layer_angle` field), no alternative-start retry for short strings, and a
hardcoded `0.4 mm` flow width. These reductions are recorded in
`seam source-geometry deviation`. This packet closes those algorithm reductions by
restoring canonical behavior, using packet 178's per-region input view to supply
real flow width, seam paint annotations, and per-region polygon candidates.

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

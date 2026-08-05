---
status: implemented
packet: 117_support-planner-geometric-correctness
task_ids: [TASK-281, TASK-282]
---

# 117_support-planner-geometric-correctness

## Goal

Correct `support_planner::tapered_radius`'s tip geometry (restore the 45° tip cone) and route support-outline avoidance through the existing guest-compatible `slicer_sdk::host::offset_polygons` API with preserved `ExPolygon` holes and explicit mm/scaled-unit boundaries.

## Problem Statement

`tapered_radius` clamped its expansion to `[branch_radius, MAX_BRANCH_RADIUS_MM]`, so `dist_to_top = 0` returned the branch radius instead of a point tip (Orca's `calc_branch_radius` produces a 45° tip cone). The B6 avoidance path used a DIY vertex-offset `inflate_polygon` that was geometrically wrong on non-convex outlines and silently flattened holes. The planner must use the SDK geometry seam (`slicer_sdk::host::offset_polygons`, `OffsetJoinType::Miter`) because `slicer-core` is a host-only crate and ADR-0023/guest-build rules forbid adding it to the guest graph.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Use `Point2::from_mm` / `mm_to_units` at every mm↔unit boundary (`docs/08_coordinate_system.md`).
- No direct `slicer-core` dependency added to the guest graph: the existing `slicer_sdk::host::offset_polygons` API is the sanctioned seam (host-side `slicer-sdk` is the only host-crate seam).
- IR/WIT/scheduler/manifest contracts untouched; `avoid_inflate` remains a millimeter scalar.

## Data and Contract Notes

- B5 `tapered_radius`: `mm_to_top = dist_to_top * effective_layer_height`; tip-cone branch `mm_to_top.max(0.0)` when `mm_to_top <= branch_radius` (linear 0 → branch_radius, 45° cone); linear-above-cone branch `branch_radius + (mm_to_top - branch_radius) * tan_diameter_angle`; final clamp `[0.0, MAX_BRANCH_RADIUS_MM]`. No interface-aware widening ported (out of scope).
- B6: `inflate_polygon` deleted; its sole call in `run_support_geometry` replaced with `host::offset_polygons(&polys, avoid_inflate, OffsetJoinType::Miter)` over the complete input `ExPolygon`; `LayerCollisionCache` reshaped to retain holes; containment/clamping helpers compare planner mm coordinates against scaled integer polygons via canonical conversion helpers.
- Closure: TASK-281 (B5) + TASK-282 (B6) added to `docs/07_implementation_status.md` and closed 2026-07-19.

## Locked Assumptions and Invariants

- `tapered_radius(0) == 0.0` (tip), NOT the old branch-radius floor — AC-N1 pins the floor is gone; `radius_tapers_with_distance_to_top` fixture migrated to the tip-cone expectation.
- The offset call consumes complete `ExPolygon` values; the cache must not flatten holes to preserve an old `Vec<Vec<[f32;2]>>` shape.
- The exact SDK signature (`&[ExPolygon]`, millimeter `f32` delta, `OffsetJoinType`) is authoritative — no stale `JoinType` name, no arc-tolerance argument absent from the SDK seam.

## Risks and Tradeoffs

- Offset of concave polygons can self-intersect — pinned by test-local edge-intersection invariant (AC-6, L-shape).
- Downstream wedge goldens shift after radius/avoidance changes; packet 119 re-captures (baseline regeneration is expected, reviewed as "intended different").
- Interface-aware widening (Orca's wider second overload) deliberately not ported — function doc-comment describes the two-piece formula without claiming it.

## Implementation Deviations (recorded at close)

None. Doc Impact: `none` — public IR/WIT shape and user-facing schema do not change.

---
status: implemented
packet: 120_support-modules-paint-segment-annotations-migration
task_ids: [TASK-285]
---

# 120_support-modules-paint-segment-annotations-migration

## Goal

Replace the centroid-based paint-eligibility logic in `paint_policy_for` with a polygon-intersection-based eligibility check, extract the fixed helper to a new `crates/slicer-core/src/paint_policy` module shared by both host-side support consumers, and clean the three module manifests' stale `PaintRegionIR` reads (the IR was deleted by packet 95).

## Problem Statement

The centroid probe was geometrically wrong: an L-shaped enforcer overlapping the L's vertical arm was gated by a vertex-mean centroid that lay in the L's notch (outside the polygon) and produced `DefaultEligible` instead of `Enforced`. Post-P95 the paint data lives in `SlicedRegion.segment_annotations[SupportEnforcer | SupportBlocker]`; the centroid helper `expolygon_centroid` / `regions_cover_point` and the dead `PaintRegionIR` ir-access reads were stale. The existing square fixtures did NOT exercise the bug (their centroids fall inside the painted region) — a new L-shape regression test per module does.

## Architecture Constraints

- The helper is a region-area floor, not a measure of overlap with a separate annotation polygon: the IR stores paint as per-polygon per-vertex `Some(_)` flags, not separate annotation polygons. Threshold: region area `> 1e-6 mm²`.
- Precedence unchanged: `Blocked` wins, then `Enforced`, then `DefaultEligible` (docs/01 "Support Stage Paint Precedence").
- `paint_policy_for` becomes a thin compatibility wrapper: filters `SliceIR.regions[*]` by a call-side expoly contour-vertex probe (`region_covers_expoly` / `point_in_polygon_ring`, private to the SDK), then aggregates `slicer_core::paint_policy::support_eligibility` per matching region with blocker-wins precedence.
- Canonical `SupportPaintPolicy` enum moved to `slicer_ir::paint_policy`, re-exported from both `slicer-core` and `slicer-sdk` (avoids the slicer-core→slicer-sdk dep cycle).

## Data and Contract Notes

- NEW `crates/slicer-core/src/paint_policy.rs`: `pub enum SupportPaintPolicy { Blocked, Enforced, DefaultEligible }` + `pub fn support_eligibility(region_polygons, segment_annotations) -> SupportPaintPolicy`.
- Host shim cleanup: `HostPaintRegionLayerView` kebab→snake semantic-name keys (`support_enforcer`/`support_blocker`), three `runtime_reads.push("PaintRegionIR")` deleted.
- Manifests: `tree-support`, `traditional-support`, `support-planner` drop `"PaintRegionIR"` from `[ir-access].reads`; `support-planner` keeps `MeshIR` (facet-based operation).
- L-shape regression: `enforcer_works_when_centroid_outside_paint_region` added to both `enforcer_blocker_tdd.rs` files (RED on old logic, GREEN after).
- Task renumber: source-plan `TASK-261` → `TASK-285` (TASK-261 now tracks infill-parity integration per packet 136).

## Locked Assumptions and Invariants

- The three `match paint.paint_policy_for(expoly)` call sites in tree-support/traditional-support are unchanged in shape (enum + signature stable).
- Empty `segment_annotations` map → `DefaultEligible` (no panic); blocker-overrides behavior preserved at commit level (`live_layer_support_tdd` AC-N1/AC-N2 stay green).
- 03_wit_and_manifest.md's `PaintRegionIR` read-attribution NOTE remains valid for `Layer::Perimeters` modules (classic/arachne) — only the SUPPORT modules drop the read.

## Risks and Tradeoffs

- Region-ownership via contour-vertex probe: a non-convex call-side expoly whose centroid lies in its own notch still matches the owning region (this is the point of the fix).
- `docs/05_module_sdk.md` gains a one-paragraph Shared-helpers entry for `support_eligibility`.

## Implementation Deviations (recorded at close)

The packet's original polygon-intersection classification wording was superseded by the implemented presence-flag + region-area-floor design (documented in `docs/05_module_sdk.md`); no DEVIATION_LOG entry needed — the docs describe the implemented contract.

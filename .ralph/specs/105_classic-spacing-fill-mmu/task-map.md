# Task Map: 105_classic-spacing-fill-mmu

Maps packet task IDs (T-050..T-054c, T-060..T-065, T-P96-A0/B/C0/C1/C2) to their source rows in the roadmap and to the implementation-plan steps that deliver them.

Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md` Phase 5 (T-050..T-054c), Phase 6 (T-060..T-065), and "Inherited from P96" (T-P96-A0/B/C0/C1/C2).

## Phase 5 — Classic spacing model

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-050 | Port `Flow::new_from_width_height` math (width→spacing conversion) to `slicer-core::flow` | Phase 5 | Step 3 | pending |
| T-051 | Replace single `line_width` with `outer_wall_line_width` + `inner_wall_line_width` in `classic-perimeters` | Phase 5 | Step 3 | pending |
| T-052 | Implement `ext_perimeter_spacing2` (outer↔first-inner) vs `perimeter_spacing` (inner↔inner) arithmetic | Phase 5 | Step 3 | pending |
| T-053 | Register and implement `precise_outer_wall` mode (gated on `wall_sequence == InnerOuter`) | Phase 5 | Step 3 | pending |
| T-054 | Register `wall_sequence` enum in perimeter manifests; deregister from `path-optimization-default` | Phase 5 | Step 4 | pending |
| T-054b | Implement `OuterInner` and `InnerOuter` modes in `slicer-perimeter-utils::wall_sequence_reorder` | Phase 5 | Step 4 | pending |
| T-054c | Implement `InnerOuterInner` sandwich mode (per-outer-contour grouping using in-module wall tree) | Phase 5 | Step 4 | pending |

## Phase 6 — Thin-walls + gap-fill

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-060 | Register `detect_thin_wall` config key | Phase 6 | Step 5 | pending |
| T-061 | Implement thin-wall detection cascade (`offset2_ex` + `opening_ex` + `medial_axis`) | Phase 6 | Step 5 | pending |
| T-062 | Emit ThinWall geometry as `WallLoop { loop_type: ThinWall, role: ThinWall }` with width profile from `ThickPolyline` | Phase 6 | Step 5 | pending |
| T-062b | Add `LoopType::GapFill` and `ExtrusionRole::GapFill` variants; add `bisector_edge_skip_mask: Vec<bool>` to `SlicedRegion`; bump schema 4.3.0 → 4.4.0 | Phase 6 | Step 2 | pending |
| T-063 | Implement gap collection per-inset using `diff_ex` | Phase 6 | Step 5 | pending |
| T-064 | Run `medial_axis` over collected gaps; filter by `filter_out_gap_fill`; emit as `WallLoop { loop_type: GapFill }` | Phase 6 | Step 5 | pending |
| T-065 | Register `gap_infill_speed` and `filter_out_gap_fill` config keys | Phase 6 | Step 5 | pending |

## Inherited from P96 — MMU per-color outer-wall fragmentation

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-P96-A0 | OrcaSlicer-source investigation: audit MMU per-color outer-wall emission path; produce `docs/specs/orca-mmu-perimeter-investigation.md` | Phase 0 (P96-inherited) | Step 1 | pending |
| T-P96-B | Revert `external_contour` consumption in classic-perimeters and arachne-perimeters | Phase 1/2 (P96-inherited) | Step 6 | pending |
| T-P96-C0 | Resurrect `bisector_edge_skip_mask: Vec<bool>` on `SlicedRegion`; host computes mask at paint-segmentation commit | Phase 1 (P96-inherited) | Step 2 | pending |
| T-P96-C1 | classic-perimeters consumes `bisector_edge_skip_mask`: skip outer-wall edges where mask is `true` | Phase 4/5 (P96-inherited) | Step 6 | pending |
| ~~T-P96-C2~~ | **DROPPED** — `variable-width-perimeters` deleted under P108 (D-110-DROP-VARIABLE-WIDTH); real-Arachne MMU coverage deferred to T-P96-E in M2. | — | — | dropped |

## Cross-Packet Contracts

- **P103 prerequisite**: `offset2_ex`, `opening_ex`, `medial_axis`, `ThickPolyline`, `polygon_tree` must be present in `slicer-core` — all delivered by P103.
- **P102 prerequisite**: `slicer_core::perimeter_utils` shared crate (T-010) and widened `WallBoundaryType::MaterialBoundary` (T-013).
- **T-P96-A0 gates T-P96-C0**: the tie-break rule documented in the one-pager grounds the host bisector-mask computation.

## Deferred / Deviation Registrations

| Deviation ID | Reason | Registered in Step |
| --- | --- | --- |
| `D-96-AC22-EXTERNAL-CONTOUR` | Superseded by this packet; `bisector_edge_skip_mask` makes `external_contour` removable (P109 deletes it) | Step 1 (supersession note) |

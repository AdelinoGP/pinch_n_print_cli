# Task Map: 105_classic-spacing-fill-mmu

Maps packet task IDs (T-050..T-054c, T-060..T-065, T-P96-A0/B) to their source rows in the roadmap and to the implementation-plan steps that deliver them. T-P96-C0 and T-P96-C1 are DROPPED (Model A — no mask field, no mask consumer). T-P96-C2 was already dropped (D-110-DROP-VARIABLE-WIDTH).

Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md` Phase 5 (T-050..T-054c), Phase 6 (T-060..T-065), and "Inherited from P96" (T-P96-A0/B).

## Phase 5 — Classic spacing model

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-050 | Port `Flow::new_from_width_height` math (width→spacing conversion) to `slicer-core::flow` | Phase 5 | Step 4 | pending |
| T-051 | Replace single `line_width` with `outer_wall_line_width` + `inner_wall_line_width` in `classic-perimeters` | Phase 5 | Step 4 | pending |
| T-052 | Implement `ext_perimeter_spacing2` (outer↔first-inner) vs `perimeter_spacing` (inner↔inner) arithmetic | Phase 5 | Step 4 | pending |
| T-053 | Register and implement `precise_outer_wall` mode (gated on `wall_sequence == InnerOuter`) | Phase 5 | Step 4 | pending |
| T-054 | Register `wall_sequence` enum in perimeter manifests; deregister from `path-optimization-default` | Phase 5 | Step 5 | pending |
| T-054b | Implement `OuterInner` and `InnerOuter` modes in `slicer-perimeter-utils::wall_sequence_reorder` | Phase 5 | Step 5 | pending |
| T-054c | Implement `InnerOuterInner` sandwich mode (per-outer-contour grouping using in-module wall tree) | Phase 5 | Step 5 | pending |

## Phase 6 — Thin-walls + gap-fill

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-060 | Register `detect_thin_wall` config key | Phase 6 | Step 6 | pending |
| T-061 | Implement thin-wall detection cascade (`offset2_ex` + `opening_ex` + `medial_axis`) | Phase 6 | Step 6 | pending |
| T-062 | Emit ThinWall geometry as `WallLoop { loop_type: ThinWall, role: ThinWall }` with width profile from `ThickPolyline` | Phase 6 | Step 6 | pending |
| T-062b | Add `LoopType::GapFill` and `ExtrusionRole::GapFill` variants; bump schema 4.3.0 → 4.4.0 | Phase 6 | Step 2 | pending |
| T-063 | Implement gap collection per-inset using `diff_ex` | Phase 6 | Step 6 | pending |
| T-064 | Run `medial_axis` over collected gaps; filter by `filter_out_gap_fill`; emit as `WallLoop { loop_type: GapFill }` | Phase 6 | Step 6 | pending |
| T-065 | Register `gap_infill_speed` and `filter_out_gap_fill` config keys | Phase 6 | Step 6 | pending |

## Inherited from P96 — MMU per-color outer-wall fragmentation

> **Note:** T-P96-A0 found Model A (partition/both-trace; no skip mask; each per-color region traces its own outer wall independently). This finding is source-confirmed against OrcaSlicer and grounds the rewritten ADR-0013. T-P96-C0 and T-P96-C1 are retired; T-P96-B is the only remaining MMU task in this packet.

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| T-P96-A0 | OrcaSlicer-source investigation: audit MMU per-color outer-wall emission path; produce `docs/specs/orca-mmu-perimeter-investigation.md`; confirm Model A | Phase 0 (P96-inherited) | Step 1 | done |
| T-P96-B | Remove `external_contour` union-trace consumption in BOTH perimeter modules → per-color fragmentation (Model A); arachne deletes `by_object` shared-boundary branch; classic verified already correct | Phase 1/2 (P96-inherited) | Step 7 | pending |
| ~~T-P96-C0~~ | **DROPPED** — Model A needs no host-side bisector mask. `bisector_edge_skip_mask`, `compute_bisector_edge_skip_mask`, WIT accessor, view accessor, and `paint_segmentation_bisector_mask_tdd.rs` are all removed per D-105-BISECTOR-MASK-DROPPED. | — | — | dropped |
| ~~T-P96-C1~~ | **DROPPED** — No mask to consume; per-color fragmentation achieved by T-P96-B (Model A). See D-105-BISECTOR-MASK-DROPPED. | — | — | dropped |
| ~~T-P96-C2~~ | **DROPPED** — `variable-width-perimeters` deleted under P108 (D-110-DROP-VARIABLE-WIDTH); real-Arachne MMU coverage deferred to T-P96-E in M2. | — | — | dropped |

## Cross-Packet Contracts

- **P103 prerequisite**: `offset2_ex`, `opening_ex`, `medial_axis`, `ThickPolyline`, `polygon_tree` must be present in `slicer-core` — all delivered by P103.
- **P102 prerequisite**: `slicer_core::perimeter_utils` shared crate (T-010) and widened `WallBoundaryType::MaterialBoundary` (T-013).
- **T-P96-A0 gates T-P96-B**: the Model A confirmation documented in the one-pager grounds the approach (remove external_contour union-trace; no mask).

## Deferred / Deviation Registrations

| Deviation ID | Reason | Registered in Step |
| --- | --- | --- |
| `D-105-MMU-MODEL-PIVOT` | P105 pivots from Model B (bisector skip mask) to Model A (partition/both-trace; per-color independent tracing) after T-P96-A0 source-confirmed OrcaSlicer uses Model A. Supersedes earlier Model B planning in P96 and P105 drafts. | Step 1 |
| `D-105-BISECTOR-MASK-DROPPED` | `bisector_edge_skip_mask` IR field, `compute_bisector_edge_skip_mask`, WIT `bisector-edge-skip-mask` accessor, `slicer-sdk` view accessor, `edge_offset_for_polygon`, and `paint_segmentation_bisector_mask_tdd.rs` are all removed. Model A needs none of these. Any prior codebase draft is deleted in this packet. | Step 1 |
| `D-105-AC22-PARITY-RESHAPE` | Supersedes `D-96-AC22-EXTERNAL-CONTOUR`. The AC-22b assertion is reshaped to assert per-color fragmentation count = N colors per layer + tool changes, and absence of `external_contour()` call sites. | Step 7 |

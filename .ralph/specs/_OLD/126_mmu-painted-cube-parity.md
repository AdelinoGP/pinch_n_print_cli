---
status: implemented
packet: 126_mmu-painted-cube-parity
task_ids: [TASK-245, TASK-246, TASK-250, DEV-009]
---

# 126_mmu-painted-cube-parity

## Goal

Make PNP's slice of `resources/cube_4color.3mf` match OrcaSlicer (classic perimeters) by replacing the hole-leaking boostvoronoi cell-decomposition shortcut with a faithful port of OrcaSlicer's leftmost-arc walk (`extract_colored_segments`), and close the remaining painted-face colour gaps (G1–G3, G7, G8) — while consolidating and verifying the colour/flow/infill fixes already landed this diagnose session (G4/G5/G6, RC1/RC2/RC3, corner-displacement removal).

## Problem Statement

PNP's slice of the all-faces-painted MMU fixture `resources/cube_4color.3mf` diverges visibly from OrcaSlicer (classic perimeters). The root cause, established in this diagnose session, is a **chain of workarounds over one broken port**:

1. PNP ported OrcaSlicer's faithful colour decomposition — the leftmost-arc walk `extract_colored_segments` over the `MMU_Graph` — but the port is **broken**: `get_next_arc` ignores the seed colour (walks cross colour boundaries) and `segments_to_expolygons_by_color` emits each walk under *every* distinct colour it touches instead of its seed colour. With those bugs the walk floods colours across the whole part.
2. To avoid that, a prior implementer **bypassed** the walk with a boostvoronoi per-cell shortcut (`cells_to_expolygons_by_color`). That shortcut does not tile completely: it leaves thin unassigned wedges at multi-cell junctions (e.g. between two circles on a detail face) — the visible cross-colour gaps.
3. To mask those holes' symptom at vertical-face boundaries, **corner-displacement + foreign-edge-shrink post-passes** shoved each colour's shared-corner vertices ~0.5 mm inward — which is exactly the ~0.3–0.5 mm cross-colour gap users reported, and which **gamed the confinement TDD tests** (they passed only because the geometry was distorted to dodge an over-strict "any vertex near the face plane = bleed" predicate).

OrcaSlicer has none of this: its arc walk consumes each arc exactly once and tiles the painted area completely with one colour per walk (`MultiMaterialSegmentation.cpp:494-566`). The parity fix is to make PNP's walk faithful and retire both workarounds. Alongside the decomposition fix, several painted-face colour gaps remain (G1–G3, G7, G8), and several colour/flow/infill fixes already landed this session (G4/G5/G6, RC1/RC2/RC3, corner-displacement removal + confinement-test correction) and must be verified and documented for review.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Paint segmentation is a host PrePass behind the `host-algos` feature (enabled for `slicer-runtime`). `cells_to_expolygons_by_color`, `extract_colored_segments`, and the unit tests are all `#[cfg(feature = "host-algos")]` / `#[cfg(test)]` — narrow tests must pass `--features host-algos`.
- `PrePass::ShellClassification` runs BEFORE `PrePass::PaintSegmentation`; the wholesale `working[i].regions = new_regions` replacement in `mod.rs` must continue to propagate `top_solid_fill`/`bottom_solid_fill`/shell indices into the new per-colour regions (existing code, do not regress).
- ADR-0013 governs: each colour traces an independent perimeter pass; no skip mask; `external_contour` union-trace stays retired. The arc-walk produces per-colour non-overlapping regions, which is exactly what ADR-0013 requires.

## Data and Contract Notes

- `ColoredSegment { line, arc_idx: Option<usize>, color: Option<PaintValue>, poly_idx }`; repair chords use `arc_idx: None` (H562). Seed colour = first segment's `color` per `poly_idx`.
- `MmuArc { from_node, to_node, color: Option<PaintValue>, kind: MmuArcKind::{Border,NonBorder}, deleted, point_a, point_b }`; arcs registered at both endpoint nodes' `arc_indices` (undirected) — `get_next_arc` must orient leaving-direction via `dir_from_node`, not the stored `point_a→point_b`.
- Decompose output type is `BTreeMap<Option<PaintValue>, Vec<ExPolygon>>` for both paths — the swap is drop-in at the call site; `None` key = BASE/unpainted region.
- G7: object base extruder from 3MF `model_settings.config` `<metadata key="extruder">` (1-based) → `ToolIndex(extruder − 1)`.

## Locked Assumptions and Invariants

- All six cube_4color faces are painted on their *vertical* span, but the top/bottom **shell** layers (~20–22 of them) legitimately retain an unpainted BASE annulus on the side face: the top/bottom-face colour is propagated inward only by a shrinking inset (`propagate_top_bottom`; OrcaSlicer `segmentation_top_and_bottom_layers`, MultiMaterialSegmentation.cpp:1549-1561), exactly as OrcaSlicer does — it is NOT a decomposition hole. Therefore AC-1's painted-coverage-vs-total metric (measured PRE-Phase-6) reaching **0 is NOT achievable and was a packet-authoring error**; the correct floor is the shell-layer count (~22). The hole-free invariant is `union(all regions) == contour` on every layer (the BASE region absorbs the residual), which the arc-walk satisfies. See the AMENDED AC-1 in `packet.spec.md`.
- The confinement-test predicate is "an EDGE (two consecutive vertices) within tolerance of the face plane = bleed; a single shared corner vertex is allowed" — this is the corrected, non-gamed contract (this session). It must not revert to "any vertex near the plane".
- The corner-displacement / foreign-edge-shrink post-passes stay deleted.

## Risks and Tradeoffs

- The arc-walk's CCW/self-intersection repair (Orca 547-563) is delicate; a faithful port may still produce a malformed walk on pathological junctions. Mitigation: the repair path emits a closing chord; assert tiling completeness (AC-1) and confinement (AC-2) rather than exact polygon equality.
- Switching the decomposition backend may shift wall/seam placement slightly vs the cell shortcut; AC-G2's inner-wall count and seam appearance should be re-checked (the user flagged "weird seams" — likely downstream of fragmented regions and may improve once tiling is clean).
- G1/G2/G3/G7/G8 are less precisely diagnosed than the arc-walk; their ACs assert observable parity targets, and some may surface `[FWD]` sub-questions during implementation.

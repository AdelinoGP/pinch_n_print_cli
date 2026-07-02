# ADR-0013 — MMU Multi-Color Perimeters Partition Per-Color, Both Sides Trace Independently

## Status

Accepted (rewritten 2026-06-23, supersedes the prior skip-mask decision).

## Context

OrcaSlicer's multi-material (MMU) painted perimeters are produced by a **geometric partition** of the painted interior into per-color regions, each of which runs a **complete, independent perimeter-offset pass**. There is no skip mask, no per-edge ownership, and no tie-break rule.

Source-grounded evidence (OrcaSlicer):

- `MultiMaterialSegmentation.cpp:523` — per-color ExPolygon emission from the leftmost-arc walk; each color's cell is a standalone closed region.
- `MultiMaterialSegmentation.cpp:547-548` — adjacent per-color cells share the bisector as a common boundary edge; the partition is non-overlapping (a single shared `used_arcs` flag consumes each interior arc exactly once).
- `MultiMaterialSegmentation.cpp:2224-2225` — segmented per-color expolygons are assigned to per-color `LayerRegion`s (the intersection/steal that enforces the partition).
- `PerimeterGenerator.cpp:1599-1629` — each per-color region independently offsets its full contour inward by `ext_perimeter_width/2` (`offset_ex(expolygon, -ext_perimeter_width/2)`); no cross-region coordination.
- `PerimeterGenerator.hpp:35` — the generator operates on a single region's surfaces; one independent pass per `LayerRegion`.

This packet (P105) supersedes the earlier revision of this ADR, whose Decision section was based on a **flawed geometric mental model**. The prior model assumed two adjacent colors' outer walls would *coincide spatially* along the shared bisector, requiring one side to "own" the edge and the other to "skip" it (a per-edge `bisector_edge_skip_mask`, lower-color-ID owns). That assumption is wrong: each color offsets its contour **half a line-width inward from its own side** of the shared bisector. The two resulting walls are **parallel and separated by ~one line-width** — they never coincide, so there is nothing to deduplicate. Both sides trace independently, exactly as OrcaSlicer does.

The earlier revision also introduced `SlicedRegion.external_contour` (P96) — a host-side `union_ex` of all sibling painted cells, traced once per object — as a pragmatic close-out of `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` (AC-22b). That union-trace makes multi-color models print their outer wall as if monochrome. It is a divergence from OrcaSlicer and is retired here.

## Decision

**Multi-color models emit per-color outer-wall fragments by tracing each per-color region independently.**

- Paint segmentation produces a **non-overlapping partition** of per-color `SlicedRegion`s. Adjacent cells share the bisector as a boundary edge.
- Each per-color `SlicedRegion` runs a **complete, independent perimeter pass** starting from its full contour (bisector edge included), offsetting inward by `ext_perimeter_width/2`.
- There is **no skip mask, no per-edge ownership, and no tie-break rule.** `bisector_edge_skip_mask` is **not** introduced (and any prior draft of it is removed — see P105).
- Near a bisector, both adjacent colors produce their own outer wall, parallel and ~one line-width apart. This is the correct OrcaSlicer geometry.
- Tool changes (`T<N>`) are emitted between adjacent per-color fragments via the existing `RegionKey.region_id → ToolChange` pipeline (packet 50b). No new emit path — the existing path simply sees per-color fragments.
- `SlicedRegion.external_contour` **consumption is removed from both perimeter modules** and the union-trace is retired. (`classic` already traced per-cell, i.e. it was already correct under this model; `arachne`'s union-trace branch is removed so it also fragments per-color.)

If a future seam-placement or role-distinction packet needs per-edge bisector metadata, it can be introduced cheaply **at that point, with a real consumer** — not speculatively here.

## Consequences

- **Multi-color models match OrcaSlicer MMU output** at the outer-wall layer. Per-layer outer-wall extrusion-sequence count = the number of distinct colors present on that layer. Each fragment is preceded by `T<N>` matching its `ToolIndex`.
- **No `bisector_edge_skip_mask` field, host populator, WIT accessor, or view accessor** is added; any draft of those is removed (P105).
- **`external_contour` consumption removed from both modules.** `classic` is unchanged in behavior (already per-cell). `arachne`'s union-trace branch is deleted so each painted cell traces its own outer wall.
- **One-time test reshape**: `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` is renamed and reshaped to assert per-color fragmentation with tool changes (`cube_4color_per_layer_per_color_fragmentation_with_tool_changes`); its G-code SHA is re-baselined as `P105_CUBE_4COLOR_PARITY_SHA`. This reshape lands in **P105 (not deferred)** because Model A makes the old assertion impossible. (P109 later finalized this as `cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes` and replaced the byte-SHA `P105_CUBE_4COLOR_PARITY_SHA` golden with structural Model-A assertions, due to documented `boostvoronoi` medial-axis non-determinism — see D-109-AC22-PARITY-RESHAPE.)
- **No effect on single-color models** at any IR or G-code layer (a single color = a single region = one independent pass = one outer wall, identical to unpainted).

## Rejected alternatives

- **Union-trace (P96 `external_contour`).** Rejected: traces a single merged outer wall, printing multi-color models as if monochrome. Diverges from OrcaSlicer, which offsets each region independently (`PerimeterGenerator.cpp:1599-1629`). Retired by this revision.
- **Per-edge skip mask / single-owner bisector (prior revision of this ADR).** Rejected: based on a geometrically wrong premise that adjacent colors' walls coincide at the bisector. OrcaSlicer offsets each color half-width inward from opposite sides (`PerimeterGenerator.cpp:1599-1629`); the walls are parallel and ~one line-width apart, never coincident — there is no edge to own or skip. OrcaSlicer contains no such mechanism (`MultiMaterialSegmentation.cpp:523/547-548/2224-2225` — partition only, no ownership/skip/dedup).
- **Recompute the partition (or any mask) in the guest.** Rejected: guest WASM cannot perform boolean polygon ops; the partition is host-computed in paint segmentation and consumed as independent per-color regions.

## Future reviewers

- Do **not** re-introduce a skip mask, per-edge ownership, or any tie-break mechanism without **source-grounded evidence overriding** `MultiMaterialSegmentation.cpp:523/547-548/2224-2225` and `PerimeterGenerator.cpp:1599-1629`. The earlier skip-mask revision was retired precisely because it had no basis in OrcaSlicer source.
- Do **not** re-suggest the union-trace simplification ("just trace the outer contour once"). It fails MMU parity and was deliberately retired. If a non-parity simplified mode is ever wanted, expose it as an opt-in config gate, not the default.
- Per-edge bisector metadata may be added later **only when a concrete consumer (seam placement, role distinction) exists** — do not ship the infrastructure speculatively.

Prior revision (skip-mask, lower-color-ID owns the bisector edge) retired 2026-06-23; see git history.

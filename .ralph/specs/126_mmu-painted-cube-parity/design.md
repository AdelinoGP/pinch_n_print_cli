# Design — Packet 126: MMU Painted-Cube OrcaSlicer Parity

## Controlling Code Paths

Paint segmentation (host-native, `host-algos` feature) in `crates/slicer-core/src/algos/paint_segmentation/`:

- `mod.rs::execute_paint_segmentation` — per-layer driver. Builds contours → `collect_painted_lines` → `colorize_contours` → `MMU_Graph::from_colored_lines` → prune → **decompose**. The decompose call (~line 577) currently invokes `graph.cells_to_expolygons_by_color(&contour_bbox, &layer_total_contours)` (the hole-leaking shortcut). It must call the faithful `extract_colored_segments` walk + `segments_to_expolygons_by_color`.
- `extract_segments.rs` — `extract_colored_segments` (leftmost-arc walk) + `get_next_arc`. The faithful path; currently `#[allow(dead_code)]`. Bugs: `get_next_arc` ignores the seed colour; angle/orientation uses the arc's stored direction regardless of traversal; no CCW/repair handling matching Orca.
- `mod.rs::segments_to_expolygons_by_color` — currently emits one polygon per *distinct colour in the walk*; must emit one polygon per walk under its *seed* colour.
- `voronoi_graph.rs::from_colored_lines` + `MmuArc`/`MmuNode`/`MmuArcKind` — graph construction (BORDER vs NON_BORDER arcs); must match Orca's `build_graph`. `cells_to_expolygons_by_color` lives here and is retired once the walk is live (corner-displacement Step 9/10 already removed this session).
- `top_bottom.rs::propagate_top_bottom` — G1 (diagonal-seam sliver) and the Phase-7 precedence merge in `mod.rs`.
- `painted_line_collection.rs` / `colorize.rs` — G2 (spurious inner walls), G3 (first-layer/bottom-shell colour).
- `mod.rs` painted_subsets `None` arm — G7 (default extruder) and G8 (subdivided-horizontal-face strokes).

Already-landed (verify-only) surfaces: `crates/slicer-gcode/src/emit.rs` (G5 flow), `crates/slicer-ir/src/{slice_ir.rs,resolved_config.rs}` + `crates/slicer-macros/src/lib.rs` + `crates/slicer-wasm-host/src/{host.rs,marshal/leaf.rs}` + infill modules `solid_fill_role()` (G4), `crates/slicer-model-io/src/loader.rs` + `crates/slicer-gcode/src/serialize.rs` (RC1).

OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Paint segmentation is a host PrePass behind the `host-algos` feature (enabled for `slicer-runtime`). `cells_to_expolygons_by_color`, `extract_colored_segments`, and the unit tests are all `#[cfg(feature = "host-algos")]` / `#[cfg(test)]` — narrow tests must pass `--features host-algos`.
- `PrePass::ShellClassification` runs BEFORE `PrePass::PaintSegmentation`; the wholesale `working[i].regions = new_regions` replacement in `mod.rs` must continue to propagate `top_solid_fill`/`bottom_solid_fill`/shell indices into the new per-colour regions (existing code, do not regress).
- ADR-0013 governs: each colour traces an independent perimeter pass; no skip mask; `external_contour` union-trace stays retired. The arc-walk produces per-colour non-overlapping regions, which is exactly what ADR-0013 requires.

## Selected Approach

**Complete the faithful `extract_colored_segments` port and make it the live decomposition; retire the cell shortcut.** This is the only approach that tiles completely (no holes) AND assigns one colour per region — matching OrcaSlicer (`MultiMaterialSegmentation.cpp:494-566`).

Rejected alternatives:
- *Keep the cell shortcut + fill residual holes heuristically* (base-fill or nearest-neighbour-overlap, both prototyped this session) — band-aids: base-fill mis-colours holes between two painted circles; neighbour-fill is ambiguous for thin wedges and shifts boundaries. The user explicitly rejected these.
- *Keep the cell shortcut + corner-displacement* — the status quo being torn out; it gaps adjacent colours by ~0.5mm and games the confinement tests.

## Code Change Surface (target ≤ 3 primary files per step)

Primary (arc-walk):
- `crates/slicer-core/src/algos/paint_segmentation/extract_segments.rs` — `get_next_arc` colour filter + Orca angle convention + traversal-orientation fix; CCW/closure handling in `extract_colored_segments`.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` — swap decompose call; fix `segments_to_expolygons_by_color` to seed-colour; remove dead `cells_to_expolygons_by_color` call site; G1/G3/G7/G8 logic.
- `crates/slicer-core/src/algos/paint_segmentation/voronoi_graph.rs` — verify/repair `from_colored_lines` arc construction; delete `cells_to_expolygons_by_color` once unused.

Secondary (open gaps): `top_bottom.rs` (G1), `painted_line_collection.rs` + `colorize.rs` (G2/G3), `loader.rs` (G7/G8 base-extruder read).

Tests (new/updated): `extract_segments.rs` unit tests; `voronoi_graph.rs` 4-colour-square guard (present); `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` + `cube_fuzzy_painted_tdd.rs` (confinement predicates corrected this session; add `cube_4color_left_face_circles_tile_without_gap`, `cube_4color_first_layer_perimeter_colour_matches_bottom_face`); `slicer-core` unit tests for G7/G8.

## Read-Only Context

- `scratchpad/wip_extract_segments_arcwalk.rs` — the in-progress arc-walk edits (colour filter + seed-colour + angle attempt) from this session; resume from here. Also `git stash@{0}` "wip-arcwalk-parity-port".
- `scratchpad/discarded_holefill.rs` — the rejected heuristic fills (for context on why they were rejected; do not reapply).
- `docs/adr/0013-...md` (full, small).
- `docs/02_ir_schemas.md` — `SlicedRegion` + `ExtrusionRole` sections only (file >600 lines; use line ranges).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate only (see obligations).
- `crates/**/target/**`, any `*.wasm`, lockfiles.
- `arachne-perimeters` module sources.
- `docs/01`, `docs/02` in full — delegate/line-range only.

## Expected Sub-Agent Dispatches

- *Graph-construction parity* — Q: "Does `from_colored_lines` create one BORDER arc per coloured contour edge and NON_BORDER arcs per Voronoi bisector, registered at both endpoint nodes, matching `build_graph` (MMU_Graph) in MultiMaterialSegmentation.cpp:1714?" Scope: `voronoi_graph.rs::from_colored_lines` + delegated Orca SUMMARY. Return: SUMMARY ≤200 words.
- *Orca angle convention* — Q: "Exact leftmost-arc angle computation and tie-break in `get_next_arc` (MMU)." Scope: `OrcaSlicerDocumented/.../MultiMaterialSegmentation.cpp:414-456`. Return: SNIPPETS ≤30 lines.
- *cargo runs* — every `cargo test`/`check`/`clippy` → FACT pass/fail + first failing assertion.
- *G3 bottom-shell colouring* — Q: "How does OrcaSlicer colour bottom-shell perimeters from the bottom surface?" Scope: `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` + `LayerRegion`. Return: SUMMARY ≤200 words.

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

## Context Cost Estimate

- Aggregate: L across the whole packet (6 workstreams) — **this is why each step in `implementation-plan.md` is individually ≤ M and the packet is explicitly multi-step**; implementers should land one step per session and hand off.
- Largest single step: Step 3 (arc-walk port) — M (extract_segments + mod.rs + graph-parity dispatch).
- Highest-risk dispatch: Orca angle convention SNIPPETS (must be ≤30 lines; do not let it pull in the whole 1700-line file).

## Open Questions

- `[FWD]` G3: does first-layer colour inheritance belong in `painted_line_collection` (project bottom surface onto layer-0 wall contours) or in the Phase-7 merge? Resolve via the G3 Orca dispatch before editing.
- `[FWD]` G2: is the residual inner-wall excess caused by duplicate painted lines (facet + stroke) or by the decomposition? Re-measure after Step 3 (arc-walk) lands — it may shrink on its own.
- `[FWD]` AC-4 test shape: assert via segmentation-level tiling (no residual polygon between adjacent colour regions at z=4/z=18) rather than gcode parsing, to keep it a fast `slicer-core` unit test.
- No `[BLOCK]` items — the central arc-walk approach is decided; user approved status `active`.

# Requirements: 103_slicer-helpers-polygon-ops

## Packet Metadata

- Grouped task IDs:
  - `T-040` — Port `offset2_ex(polys, -d, +d)` and `opening_ex(polys, d)` to `slicer-helpers`
  - `T-041` — Port `ExPolygon::medial_axis(min_width, max_width, &out)` to `slicer-helpers`
  - `T-042` — Add `ThickPolyline` and `Point2WithWidth` IR types; `variable_width()` converter to `ExtrusionPath3D`
  - `T-043` — Port hole/contour containment + tree-builder analogous to OrcaSlicer's `PerimeterGeneratorLoop`
  - `T-044` — Port `keep_largest_contour_only` (spiral-vase support)
  - `T-045` — Promote `ray_to_polygons`, `nearest_point_on_polygons`, `point_to_segment_nearest` from `arachne-perimeters` to `slicer-helpers::geometry`
- Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Phase 5 (Classic spacing model) and Phase 6 (thin-walls + gap-fill) of the perimeter parity roadmap cannot start until five OrcaSlicer-canonical polygon primitives exist in `slicer-helpers`: `offset2_ex` (the open-close offset pair used to detect narrow channels and erode-then-dilate gap polygons), `opening_ex` (single-pass open), `medial_axis` (centerline extraction producing variable-width polylines from thin shapes), the hole/contour tree builder (so `process_classic`-style nested traversal can later happen in-module), and `keep_largest_contour_only` (spiral vase + narrow-island handling). All five are absent today. Additionally, the M2 Arachne port will need the same ray-cast helpers that `arachne-perimeters` currently inlines for its width-sampling approximation — those need to live in `slicer-helpers::geometry` so the M2 real-Arachne module can reuse them.

This packet adds all six primitives in one place. It is fully infrastructural — no perimeter module's wall-emission geometry changes here. The primitives are validated against analytic golden fixtures (a 10 mm square offset-then-expand, a 1×10 mm rectangle medial-axis, etc.) so the work is independently falsifiable without OrcaSlicer-recorded reference outputs.

## In Scope

- New file `crates/slicer-helpers/src/medial_axis.rs` exporting `pub fn medial_axis(input: &ExPolygon, min_width: f32, max_width: f32, out: &mut Vec<ThickPolyline>)`.
- New file `crates/slicer-helpers/src/polygon_tree.rs` exporting `pub fn build_polygon_tree(polygons: &[ExPolygon]) -> Vec<PolygonTreeNode>` where `PolygonTreeNode { polygon_index: u32, is_contour: bool, children: Vec<PolygonTreeNode> }`.
- New file `crates/slicer-helpers/src/geometry.rs` exporting `pub fn ray_to_polygons(ray: &Ray, polygons: &[ExPolygon]) -> Option<f64>`, `pub fn nearest_point_on_polygons(point: &Point2, polygons: &[ExPolygon]) -> NearestPointResult`, and `pub fn point_to_segment_nearest(point: &Point2, a: &Point2, b: &Point2) -> NearestPointResult`. (Helper types `Ray { origin: Point2, direction: (f64, f64) }` and `NearestPointResult { point: Point2, distance: f64 }` live in the same file.)
- Additions to `crates/slicer-helpers/src/polygon_ops.rs`: `pub fn offset2_ex`, `pub fn opening_ex`, `pub fn keep_largest_contour_only(polys: &mut Vec<ExPolygon>)`.
- New IR types in `crates/slicer-ir/src/slice_ir.rs`: `ThickPolyline { points: Vec<Point2WithWidth> }` and `Point2WithWidth { x: f32, y: f32, width: f32 }`. New `pub fn variable_width(thick: &ThickPolyline, role: ExtrusionRole) -> ExtrusionPath3D` converter.
- Schema-version bump per the additive rule (additive — IR types added, existing fields untouched).
- WIT mirror in `crates/slicer-schema/wit/deps/ir-types.wit` for `ThickPolyline` and `Point2WithWidth` (record types).
- Migrate `arachne-perimeters/src/lib.rs` to `use slicer_helpers::geometry::*`; delete the local `Ray` struct, `ray_to_polygons`, `nearest_point_on_polygons`, `point_to_segment_nearest`.
- Six new TDD files (one per AC), each analytic so they run in seconds without external fixture recording.
- Doc updates per the Doc Impact Statement.

## Out of Scope

- Wiring any of the primitives into `classic-perimeters` or `arachne-perimeters` thin-wall / gap-fill / spacing logic — that's Phase 5 / Phase 6 work (later packets).
- Phase 1 shared utils (paint/seam helpers) — that's packet `102_perimeter-modules-foundations`.
- Phase 2 per-vertex flag propagation — packet `104_perimeter-propagation-and-surface-rules`.
- Real `boostvoronoi` integration or `SkeletalTrapezoidationGraph` — M2 work.
- Any OrcaSlicer reference recording or recorded-output golden fixtures. All golden tests here are analytic (computable from inputs).
- Performance optimisation. The primitives must be correct against the analytic fixtures; benchmark-quality is a follow-up concern.

## Authoritative Docs

| Doc | Size | Read strategy |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | ~600 lines | Range-read §"Phase 4 — `slicer-helpers` polygon-op primitives" (~20 lines). |
| `docs/13_slicer_helpers_crate.md` | ~250 lines | Read directly — fits in budget; needed to align new exports with crate convention. |
| `docs/02_ir_schemas.md` | ~900 lines | Delegate SUMMARY for §"Variable-width geometry" and §"Schema Versioning". Read directly only the lines around `ExtrusionPath3D` and `Point3WithWidth`. |
| `docs/08_coordinate_system.md` | ~250 lines | Read directly — required for every mm↔unit boundary in the new primitives. |
| `docs/03_wit_and_manifest.md` | ~400 lines | Range-read §"WIT/Type Changes Checklist" only (≈ 30 lines). |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/ClipperUtils.cpp` — `offset2_ex` and `opening_ex` parameter conventions, `ClipperSafetyOffset` constant. Delegate a SUMMARY (≤ 100 words).
- `OrcaSlicerDocumented/src/libslic3r/Geometry/MedialAxis.cpp` (canonical) or `Polygon.cpp` (if MedialAxis lives there) — `medial_axis(min, max, &out)` semantics; what happens on degenerate inputs. Delegate a SUMMARY.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1630` — `keep_largest_contour_only` semantic confirmation. Delegate a FACT.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1727-1779` — hole/contour nesting algorithm used by `traverse_loops`. Delegate a SUMMARY (≤ 150 words) describing the containment test and child-ordering rule.

## Acceptance Summary

- Positive cases: `AC-1` (`offset2_ex` round-trip), `AC-2` (`medial_axis` on rectangle), `AC-3` (`variable_width` converter), `AC-4` (polygon tree on holed-square), `AC-5` (`keep_largest_contour_only`), `AC-6` (ray ops promoted from `arachne-perimeters`).
- Negative cases: `AC-N1` (medial_axis on degenerate input no-ops), `AC-N2` (`offset2_ex` collapse returns empty Vec).
- Refinements not captured in Given/When/Then:
  - `offset2_ex(polys, neg_delta, pos_delta, …)`'s argument order MUST match OrcaSlicer (`ClipperUtils.cpp`) — open-then-close (negative first, positive second). The implementer confirms via the OrcaSlicer SUMMARY dispatch before writing the signature.
  - `medial_axis` tolerance: ±0.05 mm on Y centerline and per-vertex width is calibrated for a 1 mm × 10 mm rectangle. For other geometries the tolerance scales with feature size; document this in `docs/13_slicer_helpers_crate.md` rather than hardcoding.
- Cross-packet impact: unblocks all Phase 5 + Phase 6 work in the perimeter roadmap and the M2 Arachne pre-processing pipeline. Independent of packet `100`.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Compile after IR/WIT/helper additions | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace clippy gate | FACT pass/fail |
| `cargo test -p slicer-helpers --test offset2_ex_tdd` | AC-1 round-trip | FACT pass/fail |
| `cargo test -p slicer-helpers --test offset2_ex_collapse_tdd` | AC-N2 collapse-to-empty | FACT pass/fail |
| `cargo test -p slicer-helpers --test medial_axis_rectangle_tdd` | AC-2 medial axis on rectangle | FACT pass/fail |
| `cargo test -p slicer-helpers --test medial_axis_degenerate_input_tdd` | AC-N1 degenerate no-op | FACT pass/fail |
| `cargo test -p slicer-ir --test thick_polyline_variable_width_tdd` | AC-3 IR converter | FACT pass/fail |
| `cargo test -p slicer-helpers --test polygon_tree_tdd` | AC-4 polygon tree | FACT pass/fail |
| `cargo test -p slicer-helpers --test keep_largest_contour_only_tdd` | AC-5 largest-only | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence after WIT change | FACT clean / STALE list |

## Step Completion Expectations

- Cross-step invariant: `arachne-perimeters` `boundary_paint_tdd` and `arachne_perimeters_tdd` tests must stay green at every step that touches the module file. Step 5 (ray-op promotion) is the only step that edits `arachne-perimeters/src/lib.rs`; re-run those two tests before declaring Step 5 done.
- Step ordering rationale: IR types (`ThickPolyline`, `Point2WithWidth`) are added in the same step as `medial_axis` because `medial_axis`'s signature returns them — testing `medial_axis` without the types existing yet is impossible. Polygon-tree comes before `keep_largest_contour_only` because the tree exercises the same containment-test plumbing and surfaces any AABB / signed-area bugs first.
- Shared scratch state: none.

## Context Discipline Notes

- `crates/slicer-helpers/src/polygon_ops.rs` may already approach 300 lines after Clipper2 helpers have accumulated; the implementer should `wc -l` first and range-read if so.
- `crates/slicer-ir/src/slice_ir.rs` is ~1700 lines — range-read by `rg -n 'ExtrusionPath3D|Point3WithWidth|ExtrusionRole|CURRENT_SLICE_IR_SCHEMA_VERSION'` first, then ±40 lines.
- Likely temptation reads (skip): `OrcaSlicerDocumented/src/libslic3r/Geometry/MedialAxis.cpp` body. Delegate the SUMMARY; do not load the implementation. The `medial_axis` port targets the documented interface, not the C++ implementation.
- Sub-agent return-format for the heaviest dispatch: "OrcaSlicer `medial_axis` signature SUMMARY" must return ≤ 100 words — enough to confirm parameter order, return shape, and degenerate-input handling. Anything longer is wasted budget for this packet.

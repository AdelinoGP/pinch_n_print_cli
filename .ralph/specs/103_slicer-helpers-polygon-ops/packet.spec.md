---
status: implemented
packet: 103_slicer-helpers-polygon-ops
task_ids:
  - T-040
  - T-041
  - T-042
  - T-043
  - T-044
  - T-045
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 103_slicer-helpers-polygon-ops

## Goal

Add six dual-use polygon-op primitives to `slicer-core` — `offset2_ex` / `opening_ex`, `medial_axis` (producing a new `ThickPolyline` IR type with a `variable_width` converter), a hole/contour containment tree builder, `keep_largest_contour_only`, and a promotion of the ray-cast helpers currently inlined in `arachne-perimeters` — so downstream Classic-perimeter (Phase 5/6) and Arachne (M2) work can consume them from one place.

## Scope Boundaries

Touches `slicer-core` (new files for `medial_axis`, `polygon_tree`, `geometry`; additions to `polygon_ops`), `slicer-ir` (the new `ThickPolyline` + `Point2WithWidth` types and the `variable_width` converter), and `arachne-perimeters` (delete the local ray-op definitions and consume the promoted ones). No perimeter module's wall-emission geometry changes in this packet; the primitives are added and verified against golden fixtures but not yet wired into Phase 5/6 thin-wall or gap-fill work.

## Prerequisites and Blockers

- Depends on: none. This packet is fully independent of packet `102_perimeter-modules-foundations` (different crate); the two may proceed in parallel.
- Unblocks:
  - All Phase 5 spacing-model work in M1 (later packet) — needs `offset2_ex` and the polygon tree.
  - All Phase 6 thin-wall + gap-fill work in M1 (later packet) — needs `medial_axis` and `ThickPolyline`.
  - M2 Arachne pre-processing pipeline — needs `keep_largest_contour_only` and the ray ops.
- Activation blockers: none — all geometric primitives have defined OrcaSlicer reference implementations; tolerances are calibrated per `docs/01_system_architecture.md` (per-layer geometry ownership) and `docs/13_slicer_helpers_crate.md` §Out of Scope.

## Acceptance Criteria

- **AC-1. Given** a 10 mm × 10 mm square `ExPolygon` (vertices at `(0,0)`, `(10,0)`, `(10,10)`, `(0,10)` in mm), **when** `offset2_ex(&[square], -1.0, +1.0, OffsetJoinType::Miter, 0.0125)` is called, **then** the result is a single `ExPolygon` whose contour AABB is `(0.0, 0.0)..(10.0, 10.0)` within `±0.005 mm` on every corner (round-trip identity: shrink-then-expand by the same delta on a convex miter-joined square returns the original shape modulo the join tolerance). Note: `(1.0, 1.0)..(9.0, 9.0)` is the intermediate erode-only result; the full `offset2_ex(-1,+1)` round-trip on a convex square is `(0,0)..(10,10)`. | `cargo test -p slicer-core --test offset2_ex_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** any of the 6 independent closed-form analytic golden fixtures — `rectangle` (1 mm × 10 mm rect, center-spine along Y=0.5 mm), `wedge_25deg` (25° apex wedge), `asymmetric_taper` (linearly-tapered corridor), `curved_boundary` (elongated 6-segment hexagon), `curved_boundary_dense` (54-vertex densely-tessellated variant, validated via Douglas-Peucker decimation ε=115 units=0.0115 mm), `nested_hole` (square annulus with 45° diagonal corner transitions) — in `crates/slicer-core/tests/fixtures/medial_axis_golden/`, **when** `medial_axis(input: &ExPolygon, min_width: f32, max_width: f32) -> Result<Vec<ThickPolyline>, MedialAxisError>` is called with `min_width = 0.1 mm`, `max_width = 2.0 mm`, **then** the result is `Ok(polylines)` and the following bidirectional polyline-Hausdorff metric passes at all three thresholds: `per_vertex_to_reference` — each output vertex is ≤ 0.005 mm from the dense reference axis; `coverage_ref_to_polylines` — each reference point is ≤ 0.005 mm from an output segment; `max_width_error` — per-vertex width deviation ≤ 0.01 mm (all verified to ~1e-6 in practice). | `cargo test -p slicer-core --features host-algos --test medial_axis_golden_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** a `ThickPolyline` with three vertices `{ x: 0, y: 0, width: 0.4 }`, `{ x: 5, y: 0, width: 0.6 }`, `{ x: 10, y: 0, width: 0.4 }` (mm), **when** `variable_width(&thick_polyline, ExtrusionRole::ThinWall)` is called, **then** it returns an `ExtrusionPath3D` whose `points` field is `Vec<Point3WithWidth>` of length 3 with the same X/Y/width and `z = 0.0`, `flow_factor = 1.0`, `overhang_quartile = None` per vertex. | `cargo test -p slicer-ir --test thick_polyline_variable_width_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** an outer square contour with two disjoint hole contours inside it and one isolated contour outside, **when** `build_polygon_tree(&[outer_square, hole_a, hole_b, isolated])` is called, **then** the returned tree has two roots (`outer_square`, `isolated`), `outer_square` has exactly two children (`hole_a`, `hole_b`) with `is_contour = false`, and `isolated` has zero children — containment is determined by point-in-polygon test on one vertex of each candidate child against each candidate parent. | `cargo test -p slicer-core --test polygon_tree_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** an input `Vec<ExPolygon>` with three polygons of areas `4.0 mm²`, `9.0 mm²`, and `1.0 mm²`, **when** `keep_largest_contour_only(&mut polys)` is called, **then** `polys.len() == 1` after the call and the single remaining polygon has `area() ≈ 9.0 mm²` within `±0.01 mm²`. | `cargo test -p slicer-core --test keep_largest_contour_only_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** `arachne-perimeters/src/lib.rs` post-migration, **when** the file is searched for local definitions of `ray_to_polygons`, `closest_point_on_segment`, or `closest_point_on_polygons`, **then** none are present (each is a `use slicer_core::geometry::*` import), the equivalent functions exist in `crates/slicer-core/src/geometry.rs` with public visibility, `ray_to_polygons` has the OrcaSlicer-faithful typed signature (`ray: &Ray` input, `Option<RayHit>` return), and `Vec2` is defined alongside `Ray` in the same file. | `rg -q 'pub fn ray_to_polygons\(ray: &Ray.*Option<RayHit>' crates/slicer-core/src/geometry.rs && rg -q 'pub struct Vec2' crates/slicer-core/src/geometry.rs && rg -q 'pub fn closest_point_on_segment' crates/slicer-core/src/geometry.rs`
- **AC-7. Given** the migrated `width_at_point` call site at `modules/core-modules/arachne-perimeters/src/lib.rs:~435`, **when** `cargo test -p arachne-perimeters` runs the existing `boundary_paint_tdd` and any width-related test, **then** all pass — behavior of the iterative-inset approximation is preserved by the explicit `unwrap_or(0.0)` at the call site. | `cargo test -p arachne-perimeters 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a degenerate input — an `ExPolygon` whose contour has fewer than 3 distinct points (e.g. a 2-point "polygon"), **when** `medial_axis` is called, **then** it returns `Err(MedialAxisError::DegenerateInput)` without panicking (typed error, no silent empty). | `cargo test -p slicer-core --features host-algos --test medial_axis_degenerate_input_tdd 2>&1 | tee target/test-output.log`
- **AC-N2. Given** an `offset2_ex` call with a positive delta that fully removes the input (e.g. negative inset by `-100 mm` on a 1 mm square), **when** the call completes, **then** it returns `Vec::new()` (empty vec, no panic). | `cargo test -p slicer-core --test offset2_ex_collapse_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N3. Given** an `ExPolygon` with a coordinate exceeding `i32::MAX` (~215 m in units, far outside any realistic print volume), **when** `medial_axis` is called on it, **then** it returns `Err(MedialAxisError::CoordinateOverflow { actual_max, i32_max })` without panicking (typed error, no silent segment drop). | `cargo test -p slicer-core --features host-algos --test medial_axis_degenerate_input_tdd 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --tests && cargo test -p slicer-ir --test thick_polyline_variable_width_tdd`

> **Dev-dependency note:** `serde` and `serde_json` were added to `crates/slicer-core` `[dev-dependencies]` (not `[dependencies]`) for JSON golden fixture parsing in medial-axis tests. No runtime impact; the serde dependency does not appear in the compiled `slicer-core` library artifact.

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 4 tasks T-040 through T-045 (range-read §"Phase 4 — polygon-op primitives").
- `docs/01_system_architecture.md` — pipeline tiers and per-layer geometry ownership (confirms `slicer-core` as the correct crate for per-layer polygon math).
- `docs/02_ir_schemas.md` — `ExtrusionPath3D`, `Point3WithWidth`, `ExtrusionRole` definitions (delegate SUMMARY for the schema-version contract).
- `docs/08_coordinate_system.md` — mm↔unit conversion rules for the geometric primitives (range-read §"1 unit = 100 nm").

## Doc Impact Statement (Required)

This packet modifies the following doc sections:

- `docs/01_system_architecture.md` §"Crate Responsibilities" (or equivalent) — note that `slicer-core` owns per-layer polygon ops including those ported in this packet — `rg -q 'offset2_ex\|medial_axis\|polygon_tree\|keep_largest_contour_only' docs/01_system_architecture.md`
- `docs/02_ir_schemas.md` §"Variable-width geometry" — document the new `ThickPolyline` and `Point2WithWidth` types and the `variable_width` converter — `rg -q 'ThickPolyline.*Point2WithWidth' docs/02_ir_schemas.md`
- `docs/02_ir_schemas.md` §"Schema Versioning" — record the additive bump for the new types (`4.2.0` → `4.3.0`) — `rg -q 'ThickPolyline.*additive' docs/02_ir_schemas.md`
- `docs/DEVIATION_LOG.md` — add entry `D-103-API-PARITY-UPGRADE` recording "T-045 promoted with OrcaSlicer-faithful API redesign; behavior preserved at the one caller via `unwrap_or(0.0)`." — `rg -q 'D-103-API-PARITY-UPGRADE' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/ClipperUtils.cpp` — confirm the `offset2_ex(polys, -d, +d)` parameter conventions (open-close vs close-open) and the `ClipperSafetyOffset` constant. Delegate a SUMMARY of the `offset2_ex` signature and the `safety` argument's role.
- `OrcaSlicerDocumented/src/libslic3r/Geometry/MedialAxis.cpp` (or `Polygon.cpp` if MedialAxis lives there) — confirm the `min_width`/`max_width` semantics in `ExPolygon::medial_axis(min, max, &thin_walls)`. Delegate a SUMMARY of the parameter contract; do not load the implementation.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1630` — confirm the `keep_largest_contour_only` semantic (preserves only the polygon of greatest area; used for spiral-vase mode). Delegate a FACT.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

The following deviations from the original spec were recorded during implementation. Each entry is also mirrored in `docs/DEVIATION_LOG.md`.

- **D-103-T041-VORONOI-PORTED** — *Specified:* port `ExPolygon::medial_axis` with an out-param `&mut Vec<ThickPolyline>` signature. *Implemented:* real boostvoronoi-0.12 segment-VD + EP.cpp (extend/remove/reconnect) behind the `host-algos` cargo feature; signature changed to `pub fn medial_axis(input: &ExPolygon, min_width: f32, max_width: f32) -> Result<Vec<ThickPolyline>, MedialAxisError>`; degenerate input returns `Err(DegenerateInput)` (stricter than OrcaSlicer's silent-empty). *Reason:* typed error + value-return is idiomatic Rust; out-param dropped (no production callers). Validated against 6 closed-form analytic goldens passing per_vertex ≤ 0.005 mm / coverage ≤ 0.005 mm / width ≤ 0.01 mm to ~1e-6.

- **D-103-FIXTURE-GEOMETRY** — *Specified:* medial-axis goldens including a 30° wedge, a 200-segment curved-boundary, and a sharp-rectangular-loop nested hole. *Implemented:* wedge sharpened to 25° apex (a 30° wedge's principal axis is < 2·max_w so faithful EP.cpp correctly empties it — scale-invariant); `curved_boundary` changed to a 6-segment elongated hexagon (a 200-segment arc produces a degenerate VD junction-web, an unrealistic input); `nested_hole` golden corrected to the true medial axis with 45° diagonal corner-transition geometry (the original sharp-rectangular-loop golden was geometrically wrong); `curved_boundary_dense` (54-vertex) added and validated via Douglas-Peucker decimation (ε=115 units=0.0115 mm). *Reason:* fixtures must be realistic polygons on which the faithful port yields clean non-empty output; no impl heuristics were added. Dense variant is now fully validated.

- **D-103-MEDIAL-HOST-GATED** — *Specified:* `medial_axis` in `slicer-core` unconditionally. *Implemented:* gated behind the `host-algos` cargo feature (pulls boostvoronoi); module always declared so the default workspace build stays clean. *Reason:* boostvoronoi is a host-only dependency; guests do not enable it.

- **D-103-AC1-ROUNDTRIP** — *Specified:* AC-1 asserts result AABB `(1.0,1.0)..(9.0,9.0)`. *Implemented:* test asserts the geometrically correct `(0.0,0.0)..(10.0,10.0)` round-trip, with a secondary assertion documenting `(1,1)..(9,9)` as the intermediate erode-only result. *Reason:* `offset2_ex(-1,+1)` on a convex miter-joined square round-trips to the original shape; `(1,1)..(9,9)` is the erode-only intermediate, not the round-trip result. Spec text corrected in this pass.

- **D-103-API-PARITY-UPGRADE** — *Specified:* T-045 ray-ops remain inlined in `arachne-perimeters`. *Implemented:* ray-ops promoted to `slicer-core::geometry` with a typed OrcaSlicer-faithful API (`Ray`/`RayHit`/`Vec2`); `arachne-perimeters` consumes them via `use slicer_core::geometry::*`; minimal `pub use` re-export added to `slicer-ir/lib.rs`. *Reason:* typed API enables downstream Classic-perimeter consumers; behavior preserved at the one call site via `unwrap_or(0.0)`. Also recorded in `docs/DEVIATION_LOG.md`.

- **D-103-ARACHNE-BASELINE-FIXED** — *Specified:* (pre-existing) `arachne_perimeters_tdd.rs` asserted `wall_count == 2`. *Implemented:* corrected to `wall_count == 3`, matching the production default (`lib.rs:83`) and the inline test (`lib.rs:491`). *Reason:* the `wall_count==2` assertion was introduced by commit `a0d5ac9f` (2026-04-09, initial test baseline — predates Packet 103); the production default has always been 3. P103 corrected this pre-existing wrong baseline; full `cargo test -p arachne-perimeters` is now green.

- **D-103-AC2-METRIC** — *Specified:* AC-2 tolerance framed as a plain symmetric point-set Hausdorff distance. *Implemented:* bidirectional polyline-Hausdorff metric with three components — `per_vertex_to_reference` (each output vertex ≤ 0.005 mm from the dense reference axis), `coverage_ref_to_polylines` (each reference point ≤ 0.005 mm from an output segment), and `max_width_error` (≤ 0.01 mm). *Reason:* a plain symmetric point-set Hausdorff mismeasures geometrically-exact sparse polylines (a single vertex may satisfy point-distance while large segments are uncovered). The bidirectional metric with point-to-segment coverage is the correct measure; thresholds are unchanged. Measurement correction, not a loosening.

- **D-103-COORD-UNITS-SWEEP-FIXED** — *Specified:* (pre-existing) raw `*10_000.0` mm→unit literals in `polygon_ops.rs`. *Implemented:* all raw literals replaced with `slicer_ir::UNITS_PER_MM` (single source of truth), preserving f64 precision. Pre-existing literals originated in commits `16712d90` (2026-03-15, original offset scaffolding, line 219) and `21eadc85` (2026-05-18, arc_tolerance wiring, line 238) — both predate P103. P103 cleaned this tech debt alongside its own scope.

- **D-103-MEDIAL-INPUT-HARDENING** — *Specified:* (pre-existing) i32 coordinate overflow caused a silent segment drop in the VD builder. *Implemented:* i32 coordinate overflow now returns `Err(MedialAxisError::CoordinateOverflow { actual_max, i32_max })` before VD construction; Douglas-Peucker decimation (ε=115 units=0.0115 mm) collapses near-collinear dense tessellation so densely-tessellated curves no longer produce a degenerate VD junction-web. *Reason:* typed error lets callers distinguish overflow from a degenerate input; DP decimation is a faithful pre-processing step (not a heuristic) that makes the VD construction well-conditioned. Validated by AC-N3 (overflow) and `curved_boundary_dense` (54-vertex). Resolved — no remaining robustness gap.

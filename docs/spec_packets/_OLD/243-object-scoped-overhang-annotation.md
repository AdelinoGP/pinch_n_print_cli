---
status: implemented
packet: 243-object-scoped-overhang-annotation
task_ids:
  - TASK-353
---

# 243-object-scoped-overhang-annotation

## Goal

Make both host-only overhang annotation maps on `SurfaceClassificationIR` object-scoped
(`HashMap<ObjectId, HashMap<u32, …>>`) with a major `SurfaceClassificationIR` schema bump
(1.3.0 → 2.0.0), so the marshal and the two perimeter consumers stop measuring against other
objects' boundaries in multi-object scenes.

## Problem Statement

`SurfaceClassificationIR` carries two host-only overhang annotation maps —
`overhang_quartile_polygons` and `prev_layer_boundaries` — both keyed by *global layer index only*.
`commit_overhang_annotation_builtin`
(`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`) merges every object's
per-layer results into one flat map per layer, and the marshal
(`crates/slicer-wasm-host/src/marshal/in_.rs`, `sliced_region_to_data`) hands each region whichever
polygons share its layer index regardless of object identity. In a multi-object scene where two
objects overlap in XY, one object's previous-layer boundary and quartile bands leak into the other
object's `SliceRegionView`, so `classic-perimeters` / `arachne-perimeters` measure
`overhang_distance_mm` against a foreign object's boundary and the quartile gate is supplied by an
overlapping sibling. The host's own bridge gate is already object-scoped
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs` builds `lower_layer_polygons` keyed
`(ObjectId, u32)` before `gate_bridge_areas_by_unsupported_span`), so the prepass computes the right
data and discards the object dimension only when it writes the flat view maps. This packet restores
the object dimension in the two maps.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The maps are host-only and never cross WIT: the object dimension is resolved at the marshal
  lookup (`region.object_id` is already in scope there), so no WIT accessor signature changes and
  no guest rebuild is required. The `slicer:types/geometry` package is unversioned (ADR-0044), but
  this packet does not touch WIT at all.
- Schema/version constant: `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` is the single source of
  truth; production constructors (`mesh_analysis.rs`, `overhang_annotation_producer.rs`) already
  read the constant, not a literal. The bump ripples into exactly one hard-asserting test
  (`ir_tests.rs::bridge_detector_schema_versions_are_constant_sourced`, which pins
  `SemVer { major: 1, minor: 3, patch: 0 }`) — author the bump and that test edit in the same step.

## Data and Contract Notes

- IR contract: `SurfaceClassificationIR` is host-only aggregation; the two maps are `#[serde(default)]`
  and never mirrored in WIT. The field-type change is a **major** bump per the IR Versioning Contract
  table ("Field type changed → Major (1.x → 2.0)").
- WIT boundary: unchanged. `SliceRegionView.overhang_quartile_polygons()` /
  `prev_layer_boundary()` keep their signatures; the marshal resolves the object dimension.
- Determinism: per-object insertion preserves `mesh.objects` iteration order; the inner
  `HashMap<u32, …>` keeps the existing layer-keyed determinism. No ordering contract changes.

## Locked Assumptions and Invariants

- The inner `Vec<QuartileBand>` per (object, layer) keeps the existing "at most one band per
  quartile, sorted by quartile" invariant — only the outer key gains the object dimension.
- `ObjectId = String` is `Serialize + Deserialize + Eq + Hash` (already a `HashMap` key in
  `per_object`), so it is a valid outer key with no new derives.

## Risks and Tradeoffs

- The blast radius is larger than the plan's "three fixture files" claim: the field-type change also
  breaks `visual_debug_render.rs` (production) and three additional test files
  (`algo_prepass_slice_tdd.rs`, `overhang_pipeline_e2e_tdd.rs`,
  `slice_region_view_overhang_areas_non_empty_tdd.rs`). Pre-baked in §Files in Scope; no discovery
  left to a follow-up `cargo check`.
- The plan lists "the two perimeter consumers" as blast radius; they are not — they consume the
  unchanged view accessor. The desired side effect (no cross-object measurement) is delivered by the
  marshal re-keying alone.

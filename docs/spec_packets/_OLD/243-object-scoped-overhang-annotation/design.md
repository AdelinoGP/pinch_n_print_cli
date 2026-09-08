# Design: 243-object-scoped-overhang-annotation

## Controlling Code Paths

- Primary code path: `commit_overhang_annotation_builtin`
  (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`) → writes
  `SurfaceClassificationIR.overhang_quartile_polygons` / `prev_layer_boundaries`; consumed by
  `sliced_region_to_data` (`crates/slicer-wasm-host/src/marshal/in_.rs`) → `SliceRegionView`
  accessors → `classic-perimeters` / `arachne-perimeters`.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/executor/prepass_overhang_annotation_stage_order_tdd.rs`
  (producer, incl. `two_object_overhang_mesh()`), `crates/slicer-runtime/tests/contract/slice_region_view_overhang_areas_non_empty_tdd.rs`
  (marshal), `crates/slicer-ir/tests/ir_tests.rs` (schema constant).
- OrcaSlicer comparison: none — this is host/IR plumbing, not a parity surface.

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

## Code Change Surface

- Selected approach: nested serializable maps keyed object-first
  (`HashMap<ObjectId, HashMap<u32, Vec<…>>>`). A literal `HashMap<(ObjectId, u32), …>` tuple key is
  rejected because tuple keys do not serialize as a normal JSON map.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-ir/src/slice_ir.rs` — `SurfaceClassificationIR` struct fields
    `overhang_quartile_polygons` / `prev_layer_boundaries` (lines ~697/702), `Default` impl
    (lines ~705-714), `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` (lines ~192-196), and the
    doc-comment provenance block (lines ~183-191).
  - `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` — `commit_overhang_annotation_builtin`
    (line 115): replace the flat `per_quartile` / `prev_layer_boundaries` merge (lines 134-187) with
    per-object insertion keyed by `object.id`.
  - `crates/slicer-wasm-host/src/marshal/in_.rs` — `sliced_region_to_data` (line 386): the two
    lookups at lines 453 and 493 become object-then-layer.
  - `crates/slicer-runtime/src/visual_debug_render.rs` — `sc.overhang_quartile_polygons.values()`
    (line 516) and `.get(&layer_index)` (line 1073) iterate the nested map.
- Rejected alternatives and reasons:
  - `HashMap<(ObjectId, u32), …>` tuple key — rejected: tuple keys serialize as a JSON array, not a
    map, breaking the "normal JSON map" serialization contract.
  - A new `ObjectOverhangAnnotation` wrapper struct — rejected: adds a type for a shape a nested map
    already expresses; the plan (D1) specifies nested maps.
  - Keeping flat maps and adding a parallel object-index side table — rejected: two sources of truth
    for the same data.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs` - role: struct + constant + Default; expected change: field
  types, version bump, doc-comment provenance.
- `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` - role: producer; expected
  change: per-object map insertion.
- `crates/slicer-wasm-host/src/marshal/in_.rs` - role: marshal; expected change: object-then-layer
  lookups.
- `crates/slicer-runtime/src/visual_debug_render.rs` - role: visual-debug consumer; expected change:
  nested-map iteration.
- `crates/slicer-ir/tests/ir_tests.rs` - role: schema hard-assert; expected change: `{ 2, 0, 0 }`.
- `crates/slicer-runtime/tests/executor/prepass_overhang_annotation_stage_order_tdd.rs` - role:
  producer tests; expected change: re-key reads, rewrite the multi-object merge test to assert
  object isolation, add `overhang_annotation_scopes_bands_by_object`.
- `crates/slicer-runtime/tests/contract/slice_region_view_overhang_areas_non_empty_tdd.rs` - role:
  marshal test; expected change: re-key the `.iter().next()` destructure, add
  `overhang_areas_object_scoped_no_cross_object_leak`.
- `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` - role: test fallout; expected change:
  re-key the `sc.overhang_quartile_polygons.insert(...)` at line 238.
- `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs` - role: test fallout;
  expected change: re-key the field reads (lines ~437-499).
- `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs` - role: test fallout; expected
  change: re-key the `insert(0u32, …)` at line 118 and the field comparison at lines 390-391.
- `docs/02_ir_schemas.md` - role: docs; expected change: IR 2 section version + keying prose.

## Read-Only Context

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - lines 180-240 only - purpose: the
  already-object-scoped `lower_layer_polygons: HashMap<(ObjectId, u32), Vec<ExPolygon>>` reference
  shape (do not edit).
- `crates/slicer-core/src/algos/prepass_slice.rs` - lines 270-300 only - purpose: confirm
  `gate_bridge_areas_by_unsupported_span` is unchanged.
- `crates/slicer-core/src/perimeter_utils.rs` - lines 310-320 only - purpose: confirm
  `signed_distance_to_boundary` is unchanged (the side-effect beneficiary, not a change target).

## Out-of-Bounds Files

- `modules/core-modules/classic-perimeters/**`, `modules/core-modules/arachne-perimeters/**` - they
  consume the unchanged view accessor; do not edit.
- `crates/slicer-sdk/**`, `crates/slicer-macros/**`, `crates/slicer-schema/**` - no WIT/SDK change.
- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` - already object-scoped; read-only.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.

## Expected Sub-Agent Dispatches

- Question: list every remaining `SurfaceClassificationIR` struct-literal site and every
  `.overhang_quartile_polygons` / `.prev_layer_boundaries` field read/insert across `crates/*/src`
  and `crates/*/tests` that is NOT already in §Files in Scope; scope: `crates/**/*.rs`; return:
  `LOCATIONS`; purpose: confirm the blast radius is complete before Step 1 (belt-and-braces against
  a missed consumer).
- Question: does any test other than `ir_tests.rs::bridge_detector_schema_versions_are_constant_sourced`
  hard-assert `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` or the literal `1.3.0`; scope:
  `crates/**/tests/**/*.rs`; return: `LOCATIONS`; purpose: confirm the constant-bump fallout is
  exactly one test.

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

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 1 field-type replacement + major bump, which owns the full blast radius)
- Highest-risk dispatch and required return format: the blast-radius completeness `LOCATIONS` sweep
  (≤ 20 entries).

## Open Questions

None.

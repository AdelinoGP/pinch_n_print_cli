---
status: implemented
packet: bridge-detector-orca-parity-fixes
task_ids:
  - TASK-168
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
supersedes: 36_bridge-detector-orca-parity
reopens:
  - TASK-167
  - DEV-035
  - DEV-036
implemented: 2026-05-05
---

# Packet Contract: bridge-detector-orca-parity-fixes

## Goal

Replace packet 36's bbox-and-centroid heuristics with the project's honest PrePass mesh-adjacency analog of OrcaSlicer's `BridgeDetector`. Packet 36 shipped on green tests over heuristic implementations: the mesh-adjacency analysis seeded clusters from `FacetClass::TopSurface` (top-facing facets — bridges are down-facing overhangs), computed `anchor_width_mm` and `xy_footprint` from cluster bounding boxes rather than anchor-edge runs and facet projections, hardcoded `bridge_direction_deg` to 0° or 90° from bbox aspect ratio, used a centroid containment heuristic (not a polygon set difference) for `infill_areas \ bridge_areas` in `rectilinear-infill`, and validated the result with one tautological schema-version test and one fake sharp-anchor test that uses a rectangle. This packet rewrites those pieces, adds rotated-bridge fixtures so the bbox shortcut cannot regress, replaces the broken AC tests with tests that actually enforce their claims, and reopens the DEV-035 / DEV-036 / TASK-167 closure markers that were prematurely flipped.

This packet does **not** attempt full Orca parity for `BridgeDetector::detect_angle`. Per packet 12-rev1's documented divergence, `docs/04_host_scheduler.md` forbids new fine-layer slicing passes and `par_iter` per-layer execution forbids N±1 layer synchronization. Orca's `detect_angle` requires `lower_slices` (the prior layer's filled regions). The honest project policy is to derive `bridge_direction_deg` from the 3D anchor-edge orientation at PrePass over MeshIR, not from a per-layer 2D line-coverage sweep. The "Orca default" doc-comment attribution on `min_bridge_length_mm` and `anchor_width_mm` is removed; values are documented as project policy with explicit rationale.

## Scope Boundaries

- In scope:
  - **mesh adjacency rewrite** in `crates/slicer-host/src/mesh_analysis.rs`:
    - Cluster seed: down-facing / overhang-classified facets, not `FacetClass::TopSurface`.
    - `anchor_width_mm`: shortest perpendicular run length of contiguous anchor edges (edges shared with a non-bridge neighbor), computed from the existing half-edge map. Replace the bbox-side approximation. Remove the `#[allow(dead_code)]` on the anchor-edge logic.
    - `xy_footprint`: union of facet XY projections (one `ExPolygon` per cluster, possibly multi-contour), not the cluster AABB.
    - `bridge_direction_deg`: orientation of the longest anchor-edge run, not bbox aspect.
  - **`MeshAnalysisConfig` field rename + consolidation**: `min_anchor_width_mm` → `anchor_width_mm` (matches packet 36 contract); consolidate `overhang_threshold_deg` into the struct (currently a separate function parameter).
  - **Drop "Orca default" attribution**: doc comments on `MeshAnalysisConfig` fields are rewritten to "project policy" with the rationale referencing 12-rev1's architectural divergence.
  - **Slice-time fixes** in `crates/slicer-host/src/layer_slice.rs`:
    - `OffsetJoinType::Square` → `OffsetJoinType::Miter` (per packet 36's `design.md:124`).
    - Defensive guard: skip bridge contribution when `expansion_margin_mm < 0.0` or NaN.
  - **`rectilinear-infill` fixes** in `modules/core-modules/rectilinear-infill/src/lib.rs`:
    - Replace `partition_expoly_by_bridges` centroid heuristic with real `intersection` + `difference` from `slicer_core::polygon_ops`. Remove the inline "geometry ops not yet available" comment.
    - Fix the `is_bridge && bridge_areas.is_empty()` branch so it does NOT emit `BridgeInfill` role at the sparse alternating angle. Either skip BridgeInfill emission entirely or assert against this state.
  - **Schema-version single source of truth** in `crates/slicer-ir/src/slice_ir.rs`:
    - Expose `pub const CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION: SemVer = SemVer { major: 1, minor: 1, patch: 0 };` and `pub const CURRENT_SLICE_IR_SCHEMA_VERSION: SemVer = SemVer { major: 1, minor: 2, patch: 0 };`.
    - Both `Default` impls (or all literal constructors used in production) populate `schema_version` from these constants.
  - **`validate_polygon_simplicity` helper** in `crates/slicer-core/src/polygon_ops.rs` — wraps the existing clipper2 validity check; returns `Result<(), PolygonSimplicityError>` listing failing contour indices. Used by the new AC-neg-1 sharp-anchor pipeline test.
  - **Spec amendment: `slicer-core::polygon_ops` is the polygon-ops home**. Update `docs/13_slicer_helpers_crate.md` to remove the polygon/geometry claim and cross-reference `slicer-core::polygon_ops`. Register one new DEV-### in `docs/DEVIATION_LOG.md` documenting the spec amendment.
  - **Closure reversal**: `docs/DEVIATION_LOG.md` flips DEV-035 and DEV-036 from `Closed` back to `Open` with the rationale "Closure rationale was incorrect — algorithms were heuristic stubs (bbox/centroid). Reopened by packet 36-rev1 / TASK-168." `docs/07_implementation_status.md` reopens TASK-167 (`[x]` → `[ ]`) with a reopen note pointing at TASK-168, and adds TASK-168.
  - **`docs/02_ir_schemas.md`** updated for both schema bumps and new fields on `BridgeRegion` + `SlicedRegion`; remove the stale "until packet 36 populates" comment near `is_bridge`.
  - **Test rewrites** in `crates/slicer-host/tests/bridge_detector_tdd.rs`:
    - Replace `bridge_detector_schema_versions_are_correct` (tautology) with a real constant-sourced check.
    - Replace `sharp_anchor_offset_does_not_self_intersect` (rectangle, calls offset directly) with a V-shape pipeline test asserting `validate_polygon_simplicity` and no panic.
    - Restructure `slice_assembles_expanded_bridge_polygons` so `infill_areas` strictly contains `xy_footprint` by ≥ 2 mm in all directions; assert the +1.5 mm expansion is observable in the bbox of every output polygon.
    - Strengthen `valid_bridge_passes_min_length_filter` to also assert `anchor_width_mm` matches the perpendicular run length within 0.1 mm.
    - Strengthen `non_bridge_region_has_empty_bridge_areas` and `narrow_anchor_fails_anchor_width_filter` per the gaps in the review.
    - Add rotated-bridge fixtures (30° and/or 45°) and corresponding tests for AC-2, AC-3, AC-4, AC-5, AC-6 below.
    - Clean stale TDD scaffolding comments (`unreachable_unchecked`, `todo!`, "RED state", "stub").
  - **Test rewrites** in `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs`:
    - Add `straddling_expoly_partitioned_via_set_difference` (AC-8).
    - Add `bridge_paths_use_bridge_orientation_not_sparse_alternation` (AC-9).
    - Add `empty_bridge_areas_emits_no_bridge_infill_even_when_is_bridge_true` (NEG-2).
  - **Test tighten** in `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`: change substring from `";TYPE:Bridge"` to `";TYPE:Bridge infill"` (AC-11).
  - **Test rewrite** in `crates/slicer-ir/tests/ir_tests.rs`: replace `bridge_detector_schema_versions_are_correct` with `bridge_detector_schema_versions_are_constant_sourced` (AC-10).
  - **WASM rebuild** for all infill-stage core modules after `rectilinear-infill` changes.
  - **Verify host bindgen impls** for `bridge_areas`/`bridge_orientation_deg` accessor methods on the WIT resource trait in `crates/slicer-host/src/wit_host.rs` (the spec-review dispatch could not surface them via grep; if missing, add).
- Out of scope:
  - Slice-time `_anchor_regions` refinement à la Orca's `detect_angle` (would require N-1 layer access — separate scheduler-primitive packet if pursued).
  - `bridge_speed` / `bridge_flow_ratio` thermal/cooling overrides.
  - Bridge-direction overrides via paint regions.
  - Variable-density bridge fill.
  - Moving polygon ops from `slicer-core` to `slicer-helpers`.
  - `gyroid-infill` / `lightning-infill` bridge support.
  - Multi-cluster `bridge_orientation_deg` algorithm changes (the existing "longest valid bridge wins" tie-break stays).

## Prerequisites and Blockers

- Depends on:
  - packet `36_bridge-detector-orca-parity` (this packet supersedes it; the IR/WIT/SDK plumbing it added is kept).
  - packet `12-rev1_external-surface-classification-at-slice` (architectural-divergence rationale we cite).
- Unblocks:
  - Any future packet that consumes `BridgeRegion.anchor_width_mm`, `xy_footprint`, or `bridge_direction_deg` semantics (currently none, but packets 37/38 may).
- Activation blockers:
  - Packet 36's `packet.spec.md` flipped to `status: superseded` (done as part of Step 1 of this packet).
  - No other packet currently `active`.

## Acceptance Criteria

### Positive

- **AC-1 (cluster seed)**: **Given** an `ObjectMesh` containing both up-facing top facets and a 5 mm × 20 mm cluster of down-facing overhang facets, **when** `execute_mesh_analysis_with(MeshAnalysisConfig::default())` runs, **then** every `facet_indices[i]` of every produced `BridgeRegion` references a facet whose normal Z component is ≤ 0 (down-facing) AND no facet whose `FacetClass` is `TopSurface` appears in any `bridge_regions[*].facet_indices`. | `cargo test -p slicer-host --test bridge_detector_tdd bridge_cluster_seeded_from_downfacing_facets_only -- --exact --nocapture`

- **AC-2 (anchor_width from edge run)**: **Given** a 5 mm × 20 mm overhang rotated 30° about Z with surrounding solid wall, **when** `execute_mesh_analysis_with(MeshAnalysisConfig::default())` runs, **then** `bridge_regions[0].anchor_width_mm` is within 0.1 mm of 5.0 (the true short-edge anchor-run length perpendicular to the 30° bridge axis), and is NOT within 0.1 mm of the rotated cluster AABB short side (which is ≈ 5·cos 30° + 20·sin 30° ≈ 14.3 mm). | `cargo test -p slicer-host --test bridge_detector_tdd anchor_width_from_anchor_edge_run_not_bbox -- --exact --nocapture`

- **AC-3 (xy_footprint is facet projection)**: **Given** the same 30°-rotated 5 mm × 20 mm bridge cluster, **when** mesh analysis runs, **then** the area of `bridge_regions[0].xy_footprint[0]` is within 5% of 100 mm² (the true facet-projection area), and is NOT within 5% of the rotated AABB area (≈ 240 mm² for that rotation). | `cargo test -p slicer-host --test bridge_detector_tdd xy_footprint_is_facet_projection_not_aabb -- --exact --nocapture`

- **AC-4 (direction from longest anchor edge)**: **Given** the same 30°-rotated bridge cluster, **when** mesh analysis runs, **then** `bridge_regions[0].bridge_direction_deg` is within ±2° of 30.0 (the orientation of the longest anchor-edge run), and is NOT 0.0 or 90.0. | `cargo test -p slicer-host --test bridge_detector_tdd bridge_direction_follows_anchor_edge_orientation -- --exact --nocapture`

- **AC-5 (rotated min-length filter)**: **Given** the 30°-rotated 5 mm × 20 mm bridge cluster with `min_bridge_length_mm = 25.0`, **when** mesh analysis runs, **then** `bridge_regions[0].is_valid == false` AND `bridge_regions[0].bridge_length_mm` is within 0.1 mm of 20.0 (not the rotated AABB diagonal). | `cargo test -p slicer-host --test bridge_detector_tdd rotated_short_bridge_fails_min_length_filter -- --exact --nocapture`

- **AC-6 (rotated anchor-width filter)**: **Given** a 2 mm × 40 mm needle overhang rotated 45° with `anchor_width_mm = 5.0` (filter), **when** mesh analysis runs, **then** at least one `bridge_regions[*]` exists for the cluster (positive-detection precondition) AND every such region has `is_valid == false`. | `cargo test -p slicer-host --test bridge_detector_tdd rotated_narrow_anchor_fails_anchor_width_filter -- --exact --nocapture`

- **AC-7 (expansion margin observable)**: **Given** an axis-aligned 5 mm × 20 mm bridge with `xy_footprint` bbox `[0,0]–[20,5]` and a region `infill_areas` strictly containing `[-3,-3]–[23,8]` (a 3 mm border on every side), **when** `assemble_bridge_areas` runs with `expansion_margin_mm = 1.5`, **then** for every output `bridge_areas[i]` the bbox extends at least 1.5 mm beyond the corresponding input footprint bbox in both X and Y, AND every `bridge_areas[i]` is contained in the region's `infill_areas`. | `cargo test -p slicer-host --test bridge_detector_tdd expansion_margin_grows_polygon_observably -- --exact --nocapture`

- **AC-8 (real set difference)**: **Given** an `infill_areas` expoly `[0,0]–[20,20]` straddling a `bridge_areas` expoly `[5,5]–[15,15]`, **when** `rectilinear-infill` runs, **then** the union of all `BridgeInfill` paths' XY footprints lies inside `[5,5]–[15,15]` (within ±0.5 line-spacing tolerance) AND the union of all `SparseInfill` paths' XY footprints lies inside `[0,0]–[20,20] \ [5,5]–[15,15]` (within ±0.5 line-spacing tolerance) AND there is **no overlap** between any BridgeInfill and SparseInfill path within ±0.5 line-spacing. | `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd straddling_expoly_partitioned_via_set_difference -- --exact --nocapture`

- **AC-9 (bridge orientation precedence)**: **Given** a `SliceRegionView` with `bridge_orientation_deg = 37.0` and non-empty `bridge_areas` AND a layer index that would otherwise alternate sparse infill to 90° (or 0°), **when** `rectilinear-infill` runs, **then** every emitted path with `role == ExtrusionRole::BridgeInfill` has direction within ±1° of 37.0, AND no BridgeInfill path is at the layer-alternating angle. | `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd bridge_paths_use_bridge_orientation_not_sparse_alternation -- --exact --nocapture`

- **AC-10 (schema versions are constant-sourced)**: **Given** `slicer_ir::CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` and `slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION` constants, **when** the test runs, **then** both equal `SemVer { major: 1, minor: 1, patch: 0 }` and `SemVer { major: 1, minor: 2, patch: 0 }` respectively, AND a freshly-default-constructed `SurfaceClassificationIR` and `SliceIR` have `schema_version` equal to their respective constants (i.e., the constants ARE the source of the values, not duplicated literals). | `cargo test -p slicer-ir bridge_detector_schema_versions_are_constant_sourced -- --exact --nocapture`

- **AC-11 (Benchy E2E exact marker)**: **Given** the unmodified Benchy STL run end-to-end with default config, **when** the slicer produces G-code, **then** the output contains at least one line equal to `;TYPE:Bridge infill` (exact match on the trimmed line), NOT any other `;TYPE:Bridge*` substring. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_exact_bridge_infill_marker -- --exact --nocapture`

### Negative

- **NEG-1 (sharp anchor pipeline)**: **Given** a real V-shaped sharp-anchor bridge footprint (two long edges meeting at an interior angle of 30°) and `expansion_margin_mm = 1.5`, **when** `assemble_bridge_areas` runs end-to-end (not the `offset` primitive in isolation), **then** for every output `bridge_areas[i]` `slicer_core::polygon_ops::validate_polygon_simplicity(&poly)` returns `Ok(())` AND no panic occurs. | `cargo test -p slicer-host --test bridge_detector_tdd vshape_sharp_anchor_pipeline_produces_simple_polygons -- --exact --nocapture`

- **NEG-2 (empty bridge_areas inhibits BridgeInfill)**: **Given** a `SliceRegionView` with `region.is_bridge() == true` AND `region.bridge_areas().is_empty()` (the inconsistent state currently mishandled at `rectilinear-infill/src/lib.rs:303-312`), **when** the module runs, **then** zero paths are emitted with `role == ExtrusionRole::BridgeInfill`. | `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd empty_bridge_areas_emits_no_bridge_infill_even_when_is_bridge_true -- --exact --nocapture`

- **NEG-3 (top-only mesh produces no bridges)**: **Given** an `ObjectMesh` whose every facet is up-facing (no overhangs anywhere), **when** `execute_mesh_analysis_with(MeshAnalysisConfig::default())` runs, **then** for every object `surface_classification.per_object[obj].bridge_regions.is_empty()`. | `cargo test -p slicer-host --test bridge_detector_tdd topsurface_only_mesh_produces_no_bridge_regions -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/02_ir_schemas.md` — `BridgeRegion`, `SurfaceClassificationIR`, `SliceIR`, `SlicedRegion` schemas; additive-minor versioning. Read directly; relevant sections only. **Updated by this packet** (schema banners, new field listings, stale-comment removal).
- `docs/03_wit_and_manifest.md` — § "WIT/Type Changes Checklist". Read directly. Used to verify host bindgen impls.
- `docs/04_host_scheduler.md` — § "PrePass Execution"; § "Per-Layer Execution" — cited as the architectural reason this packet does **not** port `detect_angle`. Delegate SUMMARY.
- `docs/08_coordinate_system.md` — 100 nm/unit conversions for offsets and footprints. Read directly.
- `docs/13_slicer_helpers_crate.md` — **Updated by this packet** to remove the polygon-utility claim and cross-reference `slicer-core::polygon_ops`.
- `docs/DEVIATION_LOG.md` — DEV-035 + DEV-036 reopened; one new DEV-### registered for the slicer-helpers boundary amendment.
- `docs/07_implementation_status.md` — TASK-167 reopened; TASK-168 added.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` / `.cpp` — read-only context for the divergence rationale paragraph in `requirements.md` and `design.md`. **Do NOT port `detect_angle` directly** — it requires per-layer `lower_slices` access, which packet 12-rev1's documented architectural divergence forbids. Delegate FACT/SUMMARY only.
- `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` — `stBottomBridge` enum semantics. FACT only.

All OrcaSlicer reads MUST be delegated.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list
- delegate every cargo run, every doc read, every OrcaSlicer reference
- stop reading at 60% context and hand off at 85%

This packet has higher-than-usual review risk because it is a remediation packet: every implementation step must be checked against the corresponding spec-review finding it was meant to fix, not just against the new test passing. A green test is no longer sufficient evidence of remediation — the test itself must enforce the AC text.

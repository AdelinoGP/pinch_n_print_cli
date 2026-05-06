---
status: superseded
superseded_by: 36-rev1_bridge-detector-orca-parity-fixes
packet: bridge-detector-orca-parity
task_ids:
  - TASK-166
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

> **Superseded by `36-rev1_bridge-detector-orca-parity-fixes` on 2026-05-05.**
>
> Spec review found the algorithmic implementations behind this packet's green tests were heuristic stubs: mesh-adjacency clustered the wrong facet class (`TopSurface` instead of down-facing overhangs), `anchor_width_mm` and `xy_footprint` were bbox approximations, `bridge_direction_deg` was hardcoded 0°/90°, the `infill_areas \ bridge_areas` set difference in `rectilinear-infill` was a centroid heuristic, the schema-version test was tautological, and the sharp-anchor self-intersection test used a rectangle. DEV-035 and DEV-036 were closed with rationale that did not hold up under audit. Packet `36-rev1` reopens DEV-035, DEV-036, and TASK-167; rewrites the algorithms; and replaces the broken AC tests. The IR / WIT / SDK plumbing introduced here is preserved.

# Packet Contract: bridge-detector-orca-parity

## Goal

Replace packet 12-rev1's coarse `is_bridge: bool` heuristic with full Orca-parity bridge detection: adjacency-based bridge metrics computed at `PrePass::MeshAnalysis`, polygon-level expansion at slice time, and a per-region `bridge_areas` polygon set in `SlicedRegion` that drives `BridgeInfill` path generation in the live infill module. Adds `anchor_width_mm`, `min_bridge_length`, and `expansion_margin` config controls matching Orca defaults.

## Scope Boundaries

- In scope:
  - extending `slicer_ir::BridgeRegion` with: `anchor_width_mm: f32`, `bridge_length_mm: f32`, `expansion_margin_mm: f32`, `is_valid: bool`, `xy_footprint: Vec<ExPolygon>`
  - schema bump `SurfaceClassificationIR` to `1.1.0` (additive minor on `BridgeRegion`); schema bump `SliceIR` to `1.2.0` for the new `bridge_areas` and `bridge_orientation_deg` fields on `SlicedRegion`
  - mesh-half-edge adjacency analysis in `crates/slicer-host/src/mesh_analysis.rs` to compute the new `BridgeRegion` fields
  - bridge-config plumbing: `MeshAnalysisConfig { anchor_width_mm: f32, min_bridge_length_mm: f32, expansion_margin_mm: f32 }` resolved from global config and consumed by `execute_mesh_analysis_with`
  - polygon-level expansion at slice time in `classify_region_surfaces` / a new `assemble_bridge_areas` helper using `slicer-helpers` Minkowski offset + intersect
  - new fields on `SlicedRegion`: `bridge_areas: Vec<ExPolygon>`, `bridge_orientation_deg: f32`
  - **WIT signature extension** on the `slice-region-data` host record: add `bridge_areas: list<expolygon>` and `bridge_orientation_deg: f32`
  - SDK extension on `SliceRegionView`: getter for `bridge_orientation_deg()` and `bridge_areas()`
  - update `modules/core-modules/rectilinear-infill/src/lib.rs` to emit `BridgeInfill` paths over `bridge_areas` (using `bridge_orientation_deg`) and `SparseInfill` over `infill_areas \ bridge_areas`
  - WASM rebuild for all infill-stage core modules (`./modules/core-modules/build-core-modules.sh`)
  - new TDD coverage at three levels: mesh-analysis unit tests, slice-time polygon assembly, and end-to-end Benchy bridge evidence
  - retire two deviations registered in 12-rev1: any-vertex-in-polygon approximation; Benchy-evidence bridge heuristic
- Out of scope:
  - multi-layer top/bottom thickness (packet 35; this packet runs after it)
  - per-surface fill pattern variation (packet 37)
  - top-surface ironing (packet 38)
  - bridge-aware perimeter ordering (covered by closed TASK-152e)
  - thermal/cooling overrides for bridge fill (separate config concern)
  - bridge-direction overrides via paint regions (deferred)

## Prerequisites and Blockers

- Depends on:
  - packet `12-rev1_external-surface-classification-at-slice` (provides the `is_bridge` field on `SlicedRegion` and the `classify_region_surfaces` helper this packet replaces)
  - packet `35_multi-layer-top-bottom-thickness` (provides the `RegionMapIR` + config plumbing pattern this packet reuses for `MeshAnalysisConfig`)
- Unblocks:
  - none directly; packet 37 and 38 are independent of bridge polygon details
  - Deviation closure: closes DEV-035 and DEV-036 registered by packet `12-rev1_external-surface-classification-at-slice`
- Activation blockers:
  - packets 12-rev1 and 35 must be `implemented`
  - confirm `slicer-helpers` exposes a Minkowski/polygon-offset utility (FACT in Step 0); else add one in scope of this packet

## Acceptance Criteria

- **Given** an `ObjectMesh` with a 5 mm × 20 mm overhang cluster of bridge-eligible facets and `min_bridge_length_mm = 10.0`, **when** `execute_mesh_analysis_with(MeshAnalysisConfig { min_bridge_length_mm: 10.0, .. })` runs, **then** `SurfaceClassificationIR.per_object[obj].bridge_regions[0].is_valid == true` AND `bridge_length_mm >= 20.0` AND `anchor_width_mm` matches the supporting non-bridge edge run perpendicular to the bridge direction. | `cargo test -p slicer-host --test bridge_detector_tdd valid_bridge_passes_min_length_filter -- --exact --nocapture`
- **Given** the same mesh with `min_bridge_length_mm = 25.0`, **when** mesh-analysis runs, **then** `bridge_regions[0].is_valid == false`. | `cargo test -p slicer-host --test bridge_detector_tdd short_bridge_fails_min_length_filter -- --exact --nocapture`
- **Given** a needle-like overhang whose anchor edge run is shorter than `anchor_width_mm = 2.0`, **when** mesh-analysis runs, **then** `bridge_regions[0].is_valid == false`. | `cargo test -p slicer-host --test bridge_detector_tdd narrow_anchor_fails_anchor_width_filter -- --exact --nocapture`
- **Given** a valid bridge with `expansion_margin_mm = 1.5` and a region polygon containing the bridge XY footprint plus surrounding solid area, **when** `execute_layer_slice` runs at the bridge layer, **then** `SliceIR.regions[0].bridge_areas` is non-empty AND each `ExPolygon` in `bridge_areas` ⊆ `region.infill_areas` AND each `bridge_areas` polygon extends ≥ 1.5 mm into the surrounding solid area beyond the raw bridge footprint. | `cargo test -p slicer-host --test bridge_detector_tdd slice_assembles_expanded_bridge_polygons -- --exact --nocapture`
- **Given** a `SliceRegionView` with non-empty `bridge_areas`, **when** the canonical `rectilinear-infill` module runs, **then** `InfillIR.regions[0].solid_infill` contains at least one path whose `role == ExtrusionRole::BridgeInfill` AND whose direction within ±1° of `bridge_orientation_deg`. | `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd bridge_areas_emit_bridge_infill_at_oriented_angle -- --exact --nocapture`
- **Given** the unmodified Benchy STL run end-to-end with default Orca bridge config, **when** the slicer produces G-code, **then** the output contains at least one `;TYPE:Bridge infill` block. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_bridge_infill_evidence -- --exact --nocapture`
- **Given** `SurfaceClassificationIR.schema_version` and `SliceIR.schema_version`, **when** any new IRs are produced after this packet, **then** `SurfaceClassificationIR.schema_version == { major: 1, minor: 1, patch: 0 }` AND `SliceIR.schema_version == { major: 1, minor: 2, patch: 0 }`. | `cargo test -p slicer-ir bridge_detector_schema_versions_are_correct -- --exact --nocapture`

## Negative Test Cases

- **Given** a region with no bridge facets at any layer, **when** `execute_layer_slice` runs, **then** `SliceIR.regions[0].bridge_areas.is_empty()` AND no `BridgeInfill` paths are emitted by `rectilinear-infill`. | `cargo test -p slicer-host --test bridge_detector_tdd non_bridge_region_has_empty_bridge_areas -- --exact --nocapture`
- **Given** a bridge polygon whose Minkowski offset by `expansion_margin_mm` would self-intersect at a sharp anchor corner, **when** `assemble_bridge_areas` runs, **then** the result is a well-formed `Vec<ExPolygon>` with no self-intersecting contours (verified via `slicer-helpers::validate_polygon_simplicity`) and no panic. | `cargo test -p slicer-host --test bridge_detector_tdd sharp_anchor_offset_does_not_self_intersect -- --exact --nocapture`
- **Given** an invalid bridge (`is_valid == false` from min-length / anchor filters), **when** slice-time assembly runs, **then** that bridge contributes nothing to `SliceIR.regions[*].bridge_areas`. | `cargo test -p slicer-host --test bridge_detector_tdd invalid_bridge_excluded_from_slice_areas -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/02_ir_schemas.md` — `BridgeRegion`, `SurfaceClassificationIR`, `SliceIR`, `SlicedRegion` schemas; additive-minor versioning rule. Read directly; relevant sections only.
- `docs/03_wit_and_manifest.md` — WIT signature change checklist (this packet adds fields to `slice-region-data`). Read directly; § "WIT/Type Changes Checklist".
- `docs/13_slicer_helpers_crate.md` — polygon offset, intersect, validate utilities. Read directly.
- `docs/04_host_scheduler.md` — § "PrePass Execution"; § "Per-Layer Execution"; § "Blackboard Structure" (delegate SUMMARY each).
- `docs/08_coordinate_system.md` — 100 nm/unit conversions for offsets and footprints.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` — `BridgeDetector` class declaration; bridge-direction candidates and coverage scoring. Delegate FACT (signatures and method names only).
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` — implementation including anchor expansion and minimum-span filter. Delegate FACT (default values for `anchor_width_mm`, `min_bridge_length`, `expansion_margin_mm`; algorithmic overview ≤ 5 sentences).
- `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` — `stBottomBridge` enum; role taxonomy.

All OrcaSlicer reads MUST be delegated.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list
- delegate every cargo run, every doc read, every OrcaSlicer reference
- stop reading at 60% context and hand off at 85%

This is the highest-risk packet in the 12-rev1 → 38 chain because it changes WIT, requires WASM rebuild, and adds non-trivial geometry. The `implementation-plan.md` MUST keep every step at S/M and split if any step trends to L.

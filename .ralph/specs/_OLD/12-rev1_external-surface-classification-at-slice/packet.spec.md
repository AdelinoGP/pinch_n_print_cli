---
status: implemented
packet: external-surface-classification-at-slice
task_ids:
  - TASK-164
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: external-surface-classification-at-slice

## Goal

Wire the per-facet `FacetClass::TopSurface` / `FacetClass::BottomSurface` and `BridgeRegion` data already produced by `PrePass::MeshAnalysis` into three new `SlicedRegion` fields (`is_top_surface`, `is_bottom_surface`, `is_bridge`) at slice time, so the live infill module can emit `TopSolidInfill`, `BottomSolidInfill`, and `BridgeInfill` roles and the G-code emitter can produce `;TYPE:Top surface`, `;TYPE:Bottom surface`, and `;TYPE:Bridge infill` blocks on the real Benchy path.

This packet closes the host-side gap left by packet `12_live-top-bottom-surface-fill`: that packet shipped the SDK fields, the `rectilinear-infill` role-selection logic, and the commit-side preservation, but `crates/slicer-host/src/wit_host.rs:2545-2547` still hardcodes the three flags to `false`.

## Scope Boundaries

- In scope:
  - extending `SlicedRegion` with `is_top_surface`, `is_bottom_surface`, `is_bridge` boolean fields
  - extending `execute_layer_slice` to consume `Option<&SurfaceClassificationIR>` and adjacent-layer Z values, and to populate the new fields per region via a private `classify_region_surfaces` helper
  - threading `surface_classification` and adjacent-layer Z lookups from the blackboard through `layer_executor.rs` into `execute_layer_slice`
  - replacing the three hardcoded `false`s in `sliced_region_to_data` with reads off `SlicedRegion`
  - SliceIR schema version bump to 1.1.0 (additive minor) and updating literal `SlicedRegion` constructions across tests
  - new TDD coverage on `classify_region_surfaces` and one host-level regression on `execute_layer_slice`
- Out of scope:
  - polygon-polygon intersection between facet footprints and region polygons (any-vertex-in-polygon stays as the v1 approximation)
  - multi-layer top/bottom thickness (`top_solid_layers > 1`, `bottom_solid_layers > 1`) — packet 35
  - full Orca bridge-detector parity (anchor width, min-bridge-length, expansion margins) — packet 36
  - fill-pattern variation by surface role — packet 37
  - top-surface ironing — packet 38
  - WIT, SDK, manifest, dispatch, scheduler, blackboard-slot, prepass-stage changes
  - WASM core-module rebuilds

## Prerequisites and Blockers

- Depends on:
  - `PrePass::MeshAnalysis` (`crates/slicer-host/src/mesh_analysis.rs`) producing `FacetClass::TopSurface` / `FacetClass::BottomSurface` and `BridgeRegion.facet_indices` — already in place
  - `Blackboard::surface_classification()` accessor — already in place
  - `LayerPlanIR.global_layers` ordering — already in place
- Unblocks:
  - packet `35_multi-layer-top-bottom-thickness` (extends the same classifier with a wider Z window driven by per-region config)
  - packet `36_bridge-detector-orca-parity` (replaces the boolean `is_bridge` flag with validated bridge polygons)
- Activation blockers:
  - none

## Acceptance Criteria

- **Given** an `ObjectMesh` with one facet classified `FacetClass::TopSurface` whose world-space `z_min` lies in `[layer.z, next_layer_z)` and whose centroid XY is contained in the region's polygons, **when** `classify_region_surfaces` runs against a `SurfaceClassificationIR` that lists this facet, **then** the helper returns `(is_top_surface=true, is_bottom_surface=false, is_bridge=false)`. | `cargo test -p slicer-host --test external_surface_classification_tdd top_surface_facet_within_window_flags_top -- --exact --nocapture`
- **Given** a `BottomSurface` facet whose world-space `z_max` lies in `(prev_layer_z, layer.z]` and whose centroid XY is in the region polygons, **when** the same helper runs, **then** it returns `(is_top_surface=false, is_bottom_surface=true, is_bridge=false)`. | `cargo test -p slicer-host --test external_surface_classification_tdd bottom_surface_facet_within_window_flags_bottom -- --exact --nocapture`
- **Given** a facet listed in `BridgeRegion.facet_indices` whose world-space Z range straddles `layer.z` and whose centroid XY is in the region polygons, **when** the helper runs, **then** it returns `is_bridge=true`. | `cargo test -p slicer-host --test external_surface_classification_tdd bridge_facet_in_z_span_flags_bridge -- --exact --nocapture`
- **Given** `execute_layer_slice` invoked with `Some(&surface_classification_ir)`, `next_layer_z=Some(z_next)`, `prev_layer_z=Some(z_prev)` against a `MeshIR` whose object has a `TopSurface` facet at the current layer top, **when** the call returns, **then** the resulting `SliceIR.regions[0].is_top_surface == true` and `is_bottom_surface == false` and `is_bridge == false`. | `cargo test -p slicer-host --test external_surface_classification_tdd execute_layer_slice_writes_top_flag_on_sliced_region -- --exact --nocapture`
- **Given** the unmodified Benchy STL run end-to-end through the host, **when** the slicer produces G-code, **then** the output contains at least one `;TYPE:Top surface` block AND at least one `;TYPE:Bottom surface` block. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_top_and_bottom_surface_evidence -- --exact --nocapture`
- **Given** the same Benchy run, **when** the feature-evidence test inspects the output, **then** the `top_surface` family is no longer reported as missing. | `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_feature_evidence_failures_name_the_missing_family -- --exact --nocapture`
- **Given** `SliceIR.schema_version`, **when** any new `SliceIR` is produced after this packet, **then** the version is `SemVer { major: 1, minor: 1, patch: 0 }` (additive-minor bump documented in `docs/02_ir_schemas.md`). | `cargo test -p slicer-ir slice_ir_schema_version_is_one_one_zero -- --exact --nocapture`

## Negative Test Cases

- **Given** `execute_layer_slice` invoked with `surface_classification: None` (e.g. integration tests that pre-seed the arena), **when** the call returns, **then** every `SliceIR.regions[*].is_top_surface`, `is_bottom_surface`, and `is_bridge` is `false` — preserving the pre-packet behavior for callers that don't supply classification. | `cargo test -p slicer-host --test external_surface_classification_tdd execute_layer_slice_without_classification_keeps_flags_false -- --exact --nocapture`
- **Given** a `TopSurface` facet whose centroid XY is OUTSIDE the region polygons but whose world-Z is inside the window, **when** `classify_region_surfaces` runs, **then** `is_top_surface == false` (no false positive on geographic non-overlap). | `cargo test -p slicer-host --test external_surface_classification_tdd top_facet_outside_polygon_does_not_flag_top -- --exact --nocapture`
- **Given** a `TopSurface` facet whose `z_min` lies OUTSIDE `[layer.z, next_layer_z)` (e.g. far above), **when** the helper runs, **then** `is_top_surface == false`. | `cargo test -p slicer-host --test external_surface_classification_tdd top_facet_outside_z_window_does_not_flag_top -- --exact --nocapture`

## Verification

- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — pipeline tiers, claim system, memory model (delegate SUMMARY of §"Tier 2 — Per-Layer" only; do not load full file)
- `docs/02_ir_schemas.md` — `SurfaceClassificationIR`, `SliceIR`, `SlicedRegion` schemas and additive-minor versioning rule (read directly; only relevant sections)
- `docs/04_host_scheduler.md` — Per-Layer Execution and Blackboard structure (delegate; document is large; only §"Per-Layer Execution" and §"Blackboard Structure" matter)
- `docs/08_coordinate_system.md` — 100 nm/unit, `Point2::from_mm` / `mm_to_units`; Z convention (read directly)

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — `LayerRegion::process_external_surfaces()` polygon-subtraction parity touchstone; we use a different (mesh-facet-projection) approach but the role/output expectations match
- `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` — `stTop`, `stBottom`, `stBottomBridge` enum semantics (terminology source)

All OrcaSlicer reads MUST be delegated; never load this tree into the implementer's own context.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md` (this packet narrows packet `12_live-top-bottom-surface-fill`; map required)

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

# Requirements: bridge-detector-orca-parity

## Packet Metadata

- Grouped task IDs:
  - `TASK-166` (NEW — to be added to `docs/07_implementation_status.md`)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 12-rev1 ships a coarse `is_bridge: bool` flag computed from any-vertex-in-polygon containment of facets in `SurfaceClassificationIR.bridge_regions[*].facet_indices`. This is sufficient for Benchy *evidence* parity but not for actual print quality. Real Orca-parity bridge handling needs:

1. **Adjacency-based bridge metrics**: anchor width and bridge span computed from mesh half-edge adjacency at PrePass time. Without these, `min_bridge_length` and `anchor_width_mm` filters cannot be applied.
2. **Polygon-level expansion**: Orca expands the raw bridge polygon by `expansion_margin_mm` into surrounding solid material so the bridge filament has anchored ends. A boolean `is_bridge` cannot represent the expanded polygon.
3. **Per-region bridge polygons + orientation**: the live infill module needs the bridge polygon and the optimal `bridge_orientation_deg` to lay extrusions across the gap, not parallel to it.

This packet replaces 12-rev1's heuristic with the proper mesh-adjacency analysis and polygon-level expansion. It retires two deviations registered by 12-rev1.

## In Scope

- `slicer_ir::BridgeRegion` extension (additive minor on `SurfaceClassificationIR`):
  - `anchor_width_mm: f32` — shortest perpendicular run of contiguous anchor edges.
  - `bridge_length_mm: f32` — longest unsupported span across the cluster.
  - `expansion_margin_mm: f32` — frozen at PrePass from `MeshAnalysisConfig`.
  - `is_valid: bool` — pass/fail of the min-length + anchor-width filters.
  - `xy_footprint: Vec<ExPolygon>` — facet-cluster XY projection in 100 nm units, computed once at PrePass.
- `slicer_ir::SlicedRegion` extension (additive minor on `SliceIR`):
  - `bridge_areas: Vec<ExPolygon>` — per-layer expanded bridge polygons, ⊆ `infill_areas`.
  - `bridge_orientation_deg: f32` — best bridge direction across all valid bridge regions intersecting this slice region.
- `MeshAnalysisConfig { anchor_width_mm: f32, min_bridge_length_mm: f32, expansion_margin_mm: f32, overhang_threshold_deg: f32 }` (consolidates the existing `overhang_threshold_deg` parameter into a single config struct). Defaults from Orca; resolved from global config.
- Mesh half-edge adjacency utilities in `crates/slicer-host/src/mesh_analysis.rs` (or an internal `crates/slicer-host/src/bridge_metrics.rs` module): edge sharing, anchor-edge identification, span computation, anchor-width computation.
- Slice-time helper `assemble_bridge_areas(layer_z, region_polygons, valid_bridge_regions, expansion_margin_mm)` in `crates/slicer-host/src/layer_slice.rs` (or sibling module). Uses `slicer-helpers` Minkowski offset + intersect.
- WIT signature change: `slice-region-data` host record gets `bridge_areas: list<expolygon>` and `bridge_orientation_deg: f32`. Update SDK `SliceRegionView`, macros, and all WASM core modules.
- `rectilinear-infill` updates: emit `BridgeInfill` over `bridge_areas` at `bridge_orientation_deg`; subtract `bridge_areas` from `infill_areas` for sparse fill.
- New TDD coverage:
  - `crates/slicer-host/tests/bridge_detector_tdd.rs` — mesh-analysis unit tests + slice-time assembly tests.
  - `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` — module-level role + orientation assertions.
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — append `benchy_gcode_contains_bridge_infill_evidence`.
- WASM rebuild for all infill-stage core modules.
- Retire deviations registered by 12-rev1: any-vertex-in-polygon approximation; Benchy-evidence bridge heuristic. Update `docs/DEVIATION_LOG.md`.

## Out of Scope

- Multi-layer top/bottom thickness (packet 35).
- Per-surface fill pattern variation (packet 37).
- Top-surface ironing (packet 38).
- Bridge-aware perimeter ordering (closed TASK-152e).
- Thermal/cooling overrides (`bridge_speed`, `bridge_flow_ratio`) — separate config concern; flag in `docs/DEVIATION_LOG.md` for follow-up.
- Bridge direction overrides via paint regions.
- Variable-density bridge fill.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `BridgeRegion`, `SurfaceClassificationIR`, `SliceIR`, `SlicedRegion`. Read directly; relevant sections.
- `docs/03_wit_and_manifest.md` — WIT signature change checklist. Read directly; § "WIT/Type Changes Checklist".
- `docs/13_slicer_helpers_crate.md` — polygon offset and validation. Read directly.
- `docs/04_host_scheduler.md` — § PrePass Execution + Blackboard Structure. Delegate SUMMARY.
- `docs/08_coordinate_system.md` — 100 nm/unit conversions. Read directly.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` — `BridgeDetector` class. Delegate FACT for: method names, default `min_bridge_length`, `anchor_width_mm`, `expansion_margin`.
- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` — anchor expansion + min-span filter. Delegate SUMMARY ≤ 200 words for the algorithm.
- `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` — `stBottomBridge` enum. FACT only.

All OrcaSlicer reads MUST be delegated.

## Acceptance Summary

- Positive cases: see `packet.spec.md` Acceptance Criteria — covers (a) min-length filter, (b) anchor-width filter, (c) expansion margin, (d) module-level emission, (e) Benchy E2E bridge evidence, (f) schema version bumps.
- Negative cases: empty `bridge_areas` for non-bridge regions; offset self-intersection guard; invalid bridges excluded from slice-time areas.
- Measurable outcomes:
  - Both deviations registered by 12-rev1 retired in `docs/DEVIATION_LOG.md`.
  - `cargo test --workspace` PASS.
  - `./modules/core-modules/build-core-modules.sh` succeeds.
  - `cargo clippy --workspace -- -D warnings` PASS.
- Cross-packet impact: none directly; packets 37 and 38 are independent.

## Verification Commands

- `cargo test -p slicer-host --test bridge_detector_tdd -- --nocapture`
- `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_bridge_infill_evidence -- --nocapture`
- `cargo test -p slicer-ir bridge_detector_schema_versions_are_correct -- --nocapture`
- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition stated explicitly.
- Postcondition observable.
- Falsifying check: a single command that fails until the step is done.
- Files allowed to read with line ranges where > 300 lines.
- Files allowed to edit ≤ 3.
- Expected sub-agent dispatches.
- Step context cost: S or M (no L). If a step trends to L (e.g., a wide WIT change touching many files), it MUST be split.

## Context Discipline Notes

- Large files in the read-only path:
  - `crates/slicer-host/src/wit_host.rs` (> 3000 lines) — read only `slice-region-data` fields and `sliced_region_to_data` (lines `135-175, 2517-2580`). Out of scope to read in full.
  - `crates/slicer-host/src/mesh_analysis.rs` (~700 lines) — read full file in Step 1; otherwise range-read.
  - `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` — delegate; never load.
- OrcaSlicer trees the implementer must NOT load directly: all of `OrcaSlicerDocumented/`.
- Likely temptation reads:
  - `crates/slicer-host/src/dispatch.rs` — out of scope; do not open.
  - Other infill modules (`gyroid-infill`, `lightning-infill`) — read only the manifest WIT signature alignment if WASM rebuild fails; otherwise skip.
- Sub-agent return formats:
  - cargo runs → FACT pass/fail.
  - Build script → FACT pass/fail with the failing module name (≤ 5 lines).
  - OrcaSlicer FACT delegations → one-line FACT each.
  - `BridgeDetector.cpp` algorithm summary → SUMMARY ≤ 200 words.

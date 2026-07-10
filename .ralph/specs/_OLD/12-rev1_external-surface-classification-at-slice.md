---
status: implemented
packet: external-surface-classification-at-slice
task_ids:
  - TASK-164
---

# 12-rev1_external-surface-classification-at-slice

## Goal

Wire the per-facet `FacetClass::TopSurface` / `FacetClass::BottomSurface` and `BridgeRegion` data already produced by `PrePass::MeshAnalysis` into three new `SlicedRegion` fields (`is_top_surface`, `is_bottom_surface`, `is_bridge`) at slice time, so the live infill module can emit `TopSolidInfill`, `BottomSolidInfill`, and `BridgeInfill` roles and the G-code emitter can produce `;TYPE:Top surface`, `;TYPE:Bottom surface`, and `;TYPE:Bridge infill` blocks on the real Benchy path.

This packet closes the host-side gap left by packet `12_live-top-bottom-surface-fill`: that packet shipped the SDK fields, the `rectilinear-infill` role-selection logic, and the commit-side preservation, but `crates/slicer-host/src/wit_host.rs:2545-2547` still hardcodes the three flags to `false`.

## Problem Statement

Two acceptance tests in `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` are failing on the live Benchy path:

- `benchy_gcode_contains_top_and_bottom_surface_evidence` — expects `;TYPE:Top surface` and `;TYPE:Bottom surface` blocks.
- `benchy_feature_evidence_failures_name_the_missing_family` — names `top_surface` as the missing family.

Packet `12_live-top-bottom-surface-fill` shipped only the *receiving half* of the contract: SDK fields on `SliceRegionView` (`crates/slicer-sdk/src/views.rs:35,38,41`), role-selection logic in `modules/core-modules/rectilinear-infill/src/lib.rs:108-116`, and `convert_infill_output` role preservation through commit. The *sending half* — populating those flags from prepass classification down into the live `SliceRegionData` resource — was never wired. `crates/slicer-host/src/wit_host.rs:2545-2547` hardcodes the three flags to `false`, so every region reaches the infill module looking like sparse and the markers are never emitted.

This packet narrows packet 12 by closing the host-side wiring without expanding scope: it uses the per-facet classification already produced by `PrePass::MeshAnalysis` (`crates/slicer-host/src/mesh_analysis.rs:281-323`) and projects it onto layer regions inside the existing per-layer host built-in `execute_layer_slice` (`crates/slicer-host/src/layer_slice.rs:48`). No new prepass, no extra slicing pass, no parallelism changes, no IR threading through dispatch.

## Architecture Constraints

- **No new fine-layer-height slicing pass.** `docs/04_host_scheduler.md` reserves prepass slicing for coarser support layers; this packet must not add a new slicing pass.
- **No change to per-layer parallel execution.** `docs/04_host_scheduler.md §Per-Layer Execution` runs layers via `par_iter`; no synchronization between layer N and N±1 may be introduced.
- **Use `PrePass::MeshAnalysis` output as the classification source.** `SurfaceClassificationIR` is already on the blackboard (`Arc<SurfaceClassificationIR>`, immutable post-prepass).
- **No WIT, SDK, manifest, or dispatch signature changes.** The flags ride on the existing `SlicedRegion` IR struct.
- **Schema version bump is additive-minor.** New booleans default to `false`; serialized v1.0.0 deserializes via Serde defaulting (verify via `crates/slicer-ir/tests/ir_tests.rs` round-trip).

## Data and Contract Notes

- IR or manifest contracts touched:
  - `slicer_ir::SlicedRegion` — additive minor (3 boolean fields).
  - `slicer_ir::SliceIR.schema_version` — `1.0.0` → `1.1.0`.
  - `SurfaceClassificationIR` and `LayerPlanIR` — read-only; no schema change.
- WIT boundary considerations:
  - The `slice-region-data` WIT host record (`crates/slicer-host/src/wit_host.rs:140-142`) already carries `is_top_surface`, `is_bottom_surface`, `is_bridge`. This packet only fills them with non-`false` values; no WIT changes.
- Determinism or scheduler constraints:
  - `classify_region_surfaces` is a pure function over `(object_mesh, surface_data, region_polygons, layer_z, next_layer_z, prev_layer_z)`; deterministic.
  - `execute_layer_slice` remains pure given its inputs; per-layer parallelism is preserved because `SurfaceClassificationIR` is `Arc`-shared and read-only.

## Locked Assumptions and Invariants

- `PrePass::MeshAnalysis` is the sole producer of `FacetClass::TopSurface` / `BottomSurface` / `Bridge` and `BridgeRegion.facet_indices`. Any future re-classification mechanism must commit to the same `SurfaceClassificationIR` contract.
- `LayerPlanIR.global_layers` is sorted by ascending Z (per `docs/02_ir_schemas.md`); the implementer relies on `global_layers[i+1].z` and `global_layers[i-1].z` being monotonically adjacent.
- 100 nm/unit coordinate convention; facet-vertex XY conversion via `Point2::from_mm` / `mm_to_units` from `slicer-helpers`.
- `Blackboard::surface_classification()` returns `Some` whenever the prepass has run; `None` is only valid for synthetic test fixtures.

## Risks and Tradeoffs

- **Centroid / any-vertex-in-polygon false negatives.** A facet whose centroid sits outside the region polygon but whose XY footprint partially overlaps is missed. Acceptable for Benchy parity; tightening is packet 36's job. Registered in `docs/DEVIATION_LOG.md`.
- **Bridge classification depends on `SurfaceClassificationIR.bridge_regions` already being populated.** If `mesh_analysis.rs` does not currently populate `bridge_regions[*].facet_indices` for typical objects, bridge flagging will never light up. The first implementation step verifies this assumption before building on it; if `bridge_regions` is currently empty, a small additional fix in `mesh_analysis.rs` is in scope (else escalate to packet 36).
- **Schema bump cascade.** Tests pattern-matching `SlicedRegion` exhaustively must add the 3 new fields. Enumerated in the Code Change Surface; mechanical.

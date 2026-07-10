---
status: superseded
superseded_by: 36-rev1_bridge-detector-orca-parity-fixes
packet: bridge-detector-orca-parity
task_ids:
  - TASK-166
---

# 36_bridge-detector-orca-parity

## Goal

Replace packet 12-rev1's coarse `is_bridge: bool` heuristic with full Orca-parity bridge detection: adjacency-based bridge metrics computed at `PrePass::MeshAnalysis`, polygon-level expansion at slice time, and a per-region `bridge_areas` polygon set in `SlicedRegion` that drives `BridgeInfill` path generation in the live infill module. Adds `anchor_width_mm`, `min_bridge_length`, and `expansion_margin` config controls matching Orca defaults.

## Problem Statement

Packet 12-rev1 ships a coarse `is_bridge: bool` flag computed from any-vertex-in-polygon containment of facets in `SurfaceClassificationIR.bridge_regions[*].facet_indices`. This is sufficient for Benchy *evidence* parity but not for actual print quality. Real Orca-parity bridge handling needs:

1. **Adjacency-based bridge metrics**: anchor width and bridge span computed from mesh half-edge adjacency at PrePass time. Without these, `min_bridge_length` and `anchor_width_mm` filters cannot be applied.
2. **Polygon-level expansion**: Orca expands the raw bridge polygon by `expansion_margin_mm` into surrounding solid material so the bridge filament has anchored ends. A boolean `is_bridge` cannot represent the expanded polygon.
3. **Per-region bridge polygons + orientation**: the live infill module needs the bridge polygon and the optimal `bridge_orientation_deg` to lay extrusions across the gap, not parallel to it.

This packet replaces 12-rev1's heuristic with the proper mesh-adjacency analysis and polygon-level expansion. It retires two deviations registered by 12-rev1.

## Architecture Constraints

- **No new fine-layer slicing pass** (inherited).
- **No new per-layer state** crossing `par_iter` boundaries.
- **Mesh adjacency analysis happens at PrePass.** Per-layer state stays inside `execute_layer_slice`.
- **Polygon offset uses `slicer-helpers`.** Step 0 FACT confirms availability of a Minkowski/offset utility; if absent, add one in scope (small).
- **Defaults match Orca.** Confirmed via FACT delegation.
- **WIT signature change requires WASM rebuild.** All infill-stage core modules MUST be rebuilt; verify `./modules/core-modules/build-core-modules.sh` succeeds before marking the packet implemented.
- **Schema bumps:** `SurfaceClassificationIR` → `1.1.0` (additive minor on `BridgeRegion`); `SliceIR` → `1.2.0` (additive minor on `SlicedRegion`). Both are additive minors per `docs/02_ir_schemas.md` rules.
- **Deviation closure**: This packet closes two deviations registered by packet `12-rev1_external-surface-classification-at-slice`:
  - **DEV-035** (any-vertex-in-polygon approximation in `crates/slicer-host/src/layer_slice.rs::classify_region_surfaces`) — replaced by polygon-polygon intersection via `assemble_bridge_areas` (Minkowski offset + intersect).
  - **DEV-036** (`crates/slicer-host/src/mesh_analysis.rs:213` `bridge_regions` initialized empty and never pushed) — closed by mesh-half-edge adjacency analysis in `execute_mesh_analysis_with`.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `BridgeRegion` — additive minor (5 new fields).
  - `SurfaceClassificationIR.schema_version` — `1.0.0` → `1.1.0`.
  - `SlicedRegion` — additive minor (2 new fields: `bridge_areas`, `bridge_orientation_deg`).
  - `SliceIR.schema_version` — `1.1.0` → `1.2.0` (this packet bumps; 12-rev1 already moved 1.0 → 1.1).
- WIT boundary considerations:
  - `slice-region-data` host record gains 2 fields. Per `docs/03 §WIT/Type Changes Checklist` — search every `wit_host.rs`, `dispatch.rs`, and `wit_guest` for the affected type and update.
  - Verify type identity matches across boundaries (e.g., `list<expolygon>` consistent everywhere).
  - Run `cargo build --tests` after WIT changes per the checklist.
- Determinism or scheduler constraints:
  - Mesh adjacency analysis is pure over `(MeshIR, MeshAnalysisConfig)`; deterministic.
  - Slice-time bridge assembly is pure over `(layer_z, region_polygons, bridge_regions, expansion_margin_mm)`; deterministic.
  - Polygon offset operations from Clipper-style libraries are deterministic when the input is deterministic.

## Locked Assumptions and Invariants

- `MeshIR.objects[*].mesh.indices` is in triangle order (3 indices per facet); same assumption used by `mesh_analysis.rs:146-178`.
- The mesh is "manifold enough" for half-edge analysis — i.e., each interior edge is shared by exactly 2 facets. Non-manifold meshes (T-junctions, missing edges) yield degraded but valid `BridgeRegion` metrics; do NOT panic.
- 100 nm/unit coordinate convention.
- Polygon offsets use Clipper-style `MitterLimit`/`RoundJoin` semantics with a small mitter limit to handle sharp anchor corners (negative test case explicitly covers self-intersection avoidance).

## Risks and Tradeoffs

- **Mesh adjacency edge cases.** Non-manifold meshes (very common for STL files) may produce incomplete half-edge graphs. Strategy: degrade gracefully — if anchor-width cannot be computed, fall back to `f32::INFINITY` (passes the filter); if span cannot be computed, fall back to `0.0` (fails the filter). Never panic on real-world STLs.
- **Polygon offset producing degenerate output.** Mitigated by the offset round-join + an explicit validation check (negative test case).
- **WIT change blast radius.** Per `docs/03` checklist, this is the real risk. Step 4 explicitly searches every `wit_host.rs` / `dispatch.rs` / `wit_guest` site for `slice-region-data` and updates them in lockstep.
- **WASM rebuild break.** All infill-stage modules link against the SDK; if the SDK API changes in a non-additive way, every module must rebuild. Step 5 verifies via the build script before declaring victory.
- **Performance.** Mesh adjacency is one-time at PrePass (cheap for Benchy-scale meshes). Polygon offset at slice time is per-region, per-layer; for typical objects under 5 bridge regions per layer, sub-millisecond.

---
status: implemented
packet: multi-layer-top-bottom-thickness
task_ids:
  - TASK-165
---

# 35_multi-layer-top-bottom-thickness

## Goal

Honor per-region `top_shell_layers` and `bottom_shell_layers` config keys (Orca-equivalent) so a region's top/bottom solid-fill window spans N layers below/above a `TopSurface` / `BottomSurface` facet rather than the single-layer window introduced in packet `12-rev1`. The classifier widens its Z window to a sum of the next/prev N layer Zs from `LayerPlanIR.global_layers`, with N looked up per-region from `RegionMapIR.entries[*].config`.

This is a strict additive extension of packet `12-rev1`'s `classify_region_surfaces` — same algorithm, wider window, sourced from per-region resolved config.

## Problem Statement

Packet `12-rev1` flags a region as `is_top_surface=true` only on the single layer immediately below a TopSurface facet (and symmetrically for bottom). Real-world prints need multiple solid layers to cap the top/bottom of an object — the codebase defaults to `top_shell_layers = 3` and `bottom_shell_layers = 3`. With only one layer flagged, top/bottom surfaces are too thin and may delaminate or show infill bleed.

The classifier algorithm from packet 12-rev1 is correct in shape; it just needs a wider Z window driven by per-region resolved config. This packet plumbs `RegionMapIR` (already on the blackboard, produced by `PrePass::RegionMapping`) through `execute_layer_slice` into the classifier so each region's window is computed from its own `top_shell_layers` and `bottom_shell_layers` keys.

The same `RegionMapIR` plumbing primitive will be reused by packets 36 (bridge config: anchor width, min length, expansion margin) and 37 (per-claim module selection). Doing the plumbing once here pays off three packets later.

## Architecture Constraints

- **No new fine-layer-height slicing pass** (same constraint inherited from 12-rev1).
- **No change to per-layer parallel execution.**
- **Use `RegionMapIR` from blackboard** as the per-region resolved-config source. `RegionMapIR` is produced by host-built-in `PrePass::RegionMapping`; immutable post-prepass.
- **Per-region config keys reuse existing `ResolvedConfig.top_shell_layers` and `ResolvedConfig.bottom_shell_layers` fields** (already present at `crates/slicer-ir/src/slice_ir.rs:610,612`, defaults `3, 3` at `:657-658`). No schema additions required (Step 0 FACT confirmed).
- **Defaults match the existing codebase**: `top_shell_layers = 3`, `bottom_shell_layers = 3`. **Deviation**: Orca's defaults are `4 / 3`; the codebase intentionally uses `3 / 3` (matches Bambu/Prusa convention; not changed by this packet).

## Data and Contract Notes

- IR or manifest contracts touched:
  - `RegionMapIR.entries[*].config` — read-only consumer; no schema change.
  - Config schema — no additions required; `top_shell_layers` and `bottom_shell_layers` already declared (Step 0 FACT confirmed).
  - `SliceIR.schema_version` — no further bump (12-rev1 already moved to `1.1.0`; this packet's behavior is value-driven, not schema-driven).
  - Config schema — no additions required; `top_shell_layers` and `bottom_shell_layers` already present (Step 0 FACT confirmed).
- WIT boundary considerations: none.
- Determinism or scheduler constraints:
  - `RegionMapIR.entries[*].config` is `Arc`-shared from PrePass; read-only across rayon workers; deterministic.

## Locked Assumptions and Invariants

- `LayerPlanIR.global_layers[*].z` is sorted ascending and represents the actual sliced layer Z values (variable-height friendly).
- `RegionMapIR.entries` indexed by `(global_layer_index, object_id, region_id)` — same key shape used by 12-rev1 lookups.
- `top_shell_layers = 3`, `bottom_shell_layers = 3` are the codebase defaults (Step 0 FACT confirmed; Orca's defaults are `4 / 3`, deviation is intentional).
- An object's "topmost active layer" is implicit — when the window walk truncates at the global layer count, the upper-Z bound becomes `f32::INFINITY`, which makes any TopSurface facet above the layer fall inside the window. This correctly captures objects that end at the global slice ceiling.

## Risks and Tradeoffs

- **`top_shell_layers = 0` (user explicitly disables)**: requires an early-return in the helper. Tested in negative case.
- **Window walks across object boundaries**: if two objects are stacked and `LayerPlanIR.global_layers[i+1]` is the first layer of object B (not the next layer of object A), the helper still uses `global_layers[i+1].z` as the upper bound. This is correct for the FACET-projection algorithm because the facet itself belongs to a specific object — `classify_region_surfaces` already filters facets by `region.object_id`. No cross-object contamination.
- **Variable layer height + tall windows**: walking `global_layers` always uses the actual stored Z, so variable height is honored automatically.
- **Empty slice polygons** (flat top/bottom geometry): when `slice_mesh_ex` yields zero polygons for a layer, `execute_layer_slice` synthesizes a bounding-box polygon from the object mesh vertex extents for the XY-containment guard. Registered as a packet-local deviation in `docs/DEVIATION_LOG.md`.

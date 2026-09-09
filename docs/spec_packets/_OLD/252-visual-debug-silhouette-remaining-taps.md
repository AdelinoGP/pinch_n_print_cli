---
status: implemented
packet: 252-visual-debug-silhouette-remaining-taps
task_ids:
  - TASK-458
  - TASK-459
  - TASK-460
  - TASK-461
---

# 252-visual-debug-silhouette-remaining-taps

## Goal

Close the plan's D8 silhouette tap whitelist: add `PrePass::RegionMapping` (joined `SliceIR` rows, per-region slabs, deterministic `config_tint` interval classes) and `PrePass::OverhangAnnotation` (`overhang_quartile_polygons[layer]` bands on honest SliceIR-derived slabs via a new per-layer height index — never schedule z-diffs) to the silhouette renderer and validation surface, retiring packet 247's interim `SilhouetteUnsupportedForTap` rejections for exactly these two taps.

## Problem Statement

Packet 247 built the silhouette composite path but rejected two Z-attributable taps from the plan's D8 whitelist with interim `SilhouetteUnsupportedForTap` errors, and no queue row owned them (247's design.md `[FWD]`, echoed by 249/250/251). `PrePass::RegionMapping` is mechanically ready — its capture (`CapturedIr::RegionMapping`, `crates/slicer-runtime/src/layer_executor.rs`) retains the whole-print `Vec<SliceIR>`, so per-region slabs are self-contained; it needs a determinism rule for dynamic `config_tint` classes. `PrePass::OverhangAnnotation` is the hard half — its capture (`CapturedIr::SurfaceClassification`) carries the per-layer-keyed `overhang_quartile_polygons` bands but **no** per-region heights, and D1 forbids schedule z-diff slabs for SliceIR-height-capable taps (catch-up regions reach below the previous global Z). This packet closes the whitelist with honest slabs for both.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Projector single-owner rule (archived spec, binding): all silhouette pixels go through `Projector::project(x_or_y_mm, z_mm)` — this packet adds no transform and reuses 247's rectangle-emission machinery.
- Z is mm floats end-to-end; polygon X/Y read via `Point2::to_mm` only. `polygon_ops::intersection` operates in the integer polygon space *before* projection — no mm round-trip.
- D1 (binding): slabs are `[z − effective_layer_height, z]` per region for SliceIR-height-capable taps. Both taps here are SliceIR-height-capable (RegionMapping joins slice rows; overhang bands derive from slice footprints), so schedule z-diffs are **prohibited** as their slab source.
- Struct-literal churn gate (`docs/21_data_defaults_and_fixtures.md`): new test literals of watched types (`SliceIR`, `SlicedRegion`, `RegionMapIR`, `SurfaceClassificationIR`, …) need `..` FRU or an `// exhaustive: <reason>` waiver — follow `visual_debug_blackboard_tap_tdd.rs`'s seeded fixtures (`seeded_region_map`, `seeded_surface_classification`).

## Data and Contract Notes

- No WIT, IR-struct, schema-version, or manifest-shape changes. `typed_capture` stays absent on silhouette entries (D7); 1.0/1.1 serialization output is untouched (no `CapturedIr` or `ImageEntry` edits).
- Determinism: captures arrive sorted (STAGE_ORDER position, then layer); rectangle emission ascending layer → class order (RGB-ascending tints / quartile-ascending bands) → interval start; band lookup is keyed (`get(&layer_index)`), height-index layers are a `BTreeMap`, band lists re-sorted by `quartile` — no `HashMap` iteration order reaches any output. `RegionMapIR.entries` is a `HashMap`: the join-key sort is what launders it (same discipline as `region_mapping_shapes`).
- `config_tint` is a pure FNV-1a function of the `ResolvedConfig` Debug form — stable across processes/builds (its doc comment pins this), so RGB-ascending class order is deterministic.
- Warnings: element-for-element deterministic; new unjoined-entry warning appended after 247's slots, deduped per group.

## Locked Assumptions and Invariants

- Band polygons are subsets of their layer's `SliceIR` region polygons (producer: footprint diffs of committed region polygons; same integer coordinate space) — the partition is exact, residue impossible.
- A `QuartileBand` carries no object/region identity and may mix objects (producer merges by quartile) — attribution is geometric only.
- Regions of one object on one global layer share `effective_layer_height` (heights are per-object layer schedule state); distinct heights on one layer imply distinct objects with physically disjoint XY footprints, so per-class intersections never double-attribute.
- Slabs: RegionMapping `[capture.layer_z − joined region's effective_layer_height, capture.layer_z]`; OverhangAnnotation `[capture.layer_z − class height, capture.layer_z]`. Neither tap ever reads `SilhouetteSlabSchedule`.
- `SILHOUETTE_TAP_STAGE_IDS` after this packet = 247's five + 249's `PostPass::LayerFinalization` + 250's `PostPass::GCodeEmit` + these two; the D8 whitelist is closed — remaining rejections (MeshAnalysis, SeamPlanning, arena) are permanent plan §8 exclusions, not interim.

## Risks and Tradeoffs

- The RegionMapping capture clones the whole-print `Vec<SliceIR>` per selected layer (pre-existing adapter behavior, same acceptance as 247's support-tap clone note); the height index instead reads the Blackboard slot once per bundle — no new per-layer clones.
- `polygon_ops::intersection` runs only on mixed-height layers with bands; single-height layers (the common case, and all single-object prints) take the no-boolean fast path. Cost unmeasured; bounded by (bands × height classes) per mixed layer.
- Two distinct `ResolvedConfig`s can hash-collide to one tint (top-down accepts the same collision); the silhouette then merges them into one class — pixels are identical either way, so nothing is hidden that the top-down would show.
- 249's styled-entry refactor moves the extraction seam this packet's RegionMapping arm lands in; the dispatch in Expected Sub-Agent Dispatches re-derives the live seam at implementation time rather than freezing it here.
- AC-N2's tool-rejection pin is authored against the post-249 tree (queue order); if this packet were ever implemented against a 247-only tree, that one test would need its interim `InvalidColorBy` form — the AC's annotation records this.

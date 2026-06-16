# ModularSlicer — System Architecture

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    FRONTEND (separate process)                  │
│              Communicates via CLI args / Unix socket            │
│         Reads config schema API  |  Displays progress events    │
└─────────────────────────┬───────────────────────────────────────┘
                          │ stdin/stdout or socket
┌─────────────────────────▼───────────────────────────────────────┐
│                     SLICER HOST (Rust binary)                   │
│                                                                 │
│  ┌─────────────────┐  ┌──────────────────────────────────────┐  │
│  │ Module Registry │  │           Execution Scheduler        │  │
│  │                 │  │                                      │  │
│  │  .wasm files    │  │  1. Ingest manifests                 │  │
│  │  .toml manifests│  │  2. Build + validate DAG             │  │
│  │  Config schemas │  │  3. Freeze ExecutionPlan             │  │
│  │                 │  │  4. Execute (pre / parallel / post)  │  │
│  └─────────────────┘  └──────────────────────────────────────┘  │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                      BLACKBOARD                         │    │
│  │  MeshIR │ SurfaceClassIR │ LayerPlanIR │ RegionMapIR    │    │
│  │  (host-owned, modules receive scoped read/write views)  │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │               ECS LAYER WORLD (per-layer)               │    │
│  │  Entity: GlobalLayer(z)                                 │    │
│  │Components:SlicePolygons │ PerimeterLoops │ InfillPaths  │    │
│  │           SupportPaths │ NonPlanarFlag │ RegionOverride │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌──────────────────┐  ┌──────────────────────────────────┐     │
│  │   Host Services  │  │      Per-Layer Arena Allocator   │     │
│  │  - Mesh raycasts │  │  Allocated per layer, freed after│     │
│  │  - Clipper ops   │  │  layer completes. No cross-layer │     │
│  │  - Logging       │  │  pointer aliasing possible.      │     │
│  │  - Timing        │  └──────────────────────────────────┘     │
│  └──────────────────┘                                           │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                  WASM INSTANCE POOLS                    │    │
│  │  parallel-safe modules: N instances (N = rayon threads) │    │
│  │  sequential modules:    1 instance  (serialized access) │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
          │                                │
    Core Modules                    Community Modules
(.wasm + .toml manifest)         (.wasm + .toml manifest)
```

---

## Pipeline Tiers

Normative terminology is maintained in `../CONTEXT.md`; end-to-end edge-case
traces are maintained in:

- `./docs/10_scenario_traces.md`

### Tier 1 — PrePass (Sequential, Whole-Model)

Runs once before any layer is sliced. Results are written to the Blackboard and
become immutable during per-layer processing. Sub-facet paint strokes are
normalized into whole-triangle assignments at model-load time by the host
loader's `split_triangle_strokes`; the prepass `PrePass::MeshSegmentation` stage
is retired (the loader's output is the authoritative normalized form).

#### PrePass Stage Order

The seven prepass stages execute in this order:

```
1. PrePass::MeshSegmentation     (retired — loader already normalizes strokes)
2. PrePass::MeshAnalysis
3. PrePass::LayerPlanning
4. PrePass::OverhangAnnotation   (introduced P106; populates SurfaceClassificationIR.overhang_quartile_polygons)
5. PrePass::PaintSegmentation    (post-slice; reads SliceIR, writes via replace_slice_ir)
6. PrePass::RegionMapping        (host-built-in; cross-product variant expansion)
7. PrePass::SupportGeometry      (host-built-in always runs; guest optional)
```

Stages 1–3 are the classic mesh-analysis and layer-planning pipeline.
`PrePass::OverhangAnnotation` (stage 4; introduced P106) runs after LayerPlanning
and populates per-layer quartile band polygons into `SurfaceClassificationIR` so
Tier 2 consumers can read pre-classified overhang data without cross-layer access.
`PrePass::PaintSegmentation` (stage 5) runs after `host:slice` and
`host:shell_classification` and writes per-variant polygons back into `SliceIR`
via `replace_slice_ir`. `PaintRegionIR` is deleted. `PrePass::RegionMapping`
(stage 6) then performs cross-product expansion: each `(layer, object,
active_region)` is split into one `RegionPlan` per canonical **variant chain**
(see §"Variant-Chain Region Splitting" below). `PrePass::SupportGeometry`
(stage 7) runs last so it can consume the fully-split `SliceIR`.

**Note:** `host:slice` and `host:shell_classification` are Layer-stage host
calls (not `PrePass::*` enum variants) that run between `PrePass::MeshAnalysis`
(stage 2) and `PrePass::PaintSegmentation` (stage 6) in the broader pipeline.
The packet's AC-1 "Given" referenced them as part of a nine-stage prepass-style
sequence; this doc enumerates the seven `PrePass::*`-tagged stages per the
post-roadmap type system, with `host:slice` and `host:shell_classification`
treated as Layer-stages that bracket stage 6.

```
PrePass::MeshSegmentation  [retired — loader does this work]
  Note: The host loader's `split_triangle_strokes` normalizes sub-facet paint
  strokes into whole-triangle `facet_values` at load time. No separate prepass
  stage is needed. The `PaintLayer.strokes` field on `MeshIR` is the
  OrcaSlicer-parity flat-leaf form consumed directly by paint-segmentation.

PrePass::MeshAnalysis
  Input:  MeshIR (loaded STL/3MF/OBJ)
  Output: SurfaceClassificationIR
  Purpose: Classify facets (slope angle, overhang, bridge, top/bottom surface).
           Identify non-planar candidate surface groups.
           Detect bridge regions.

PrePass::LayerPlanning
  Input:  MeshIR + SurfaceClassificationIR + GlobalConfig
  Output: LayerPlanIR
  Purpose: Compute global Z-plane sequence.
           Resolve per-object / per-region layer heights (including modifiers).
           Handle multi-object LCM synchronization layers.
           Assign non-planar shells to surface groups.
           Handle catch-up layers for regions with different heights.

PrePass::OverhangAnnotation  [introduced P106; owned by core-modules/overhang-annotator-default]
  Input:  MeshIR + LayerPlanIR (committed by MeshAnalysis + LayerPlanning)
  Output: SurfaceClassificationIR.overhang_quartile_polygons (per-layer HashMap<u32, Vec<QuartileBand>>)
  Purpose: For each layer Z in LayerPlanIR, compute mesh cross-sections at the current and
           prior layer Z, derive per-point distances from the previous cross-section, and
           partition the 2D footprint into 4 quartile bands (thresholds: line_width × {0.5, 1.0,
           1.5, 2.0}). Band 1 is closest to support; band 4 is the most overhanging edge.
           Consumers (perimeter modules, infill modules, seam modules) read these polygons via
           point-in-polygon without cross-layer access — the classification is pre-computed at
           PrePass time. See ADR-0012.

PrePass::RegionMapping  [host-built-in, not a module stage]
  Input:  LayerPlanIR + LoadedModules + ResolvedConfig + MeshIR.paint_data
  Output: RegionMapIR
  Purpose: For every (layer, object, region) triple, expand into one RegionPlan
           per canonical variant chain (cross-product over declared region-split
           semantics × distinct paint values present on that object). Each
           RegionPlan's config is the base config plus per-semantic overlays
           contributed by each semantic in the chain. Configs are interned via
           ConfigId. Pre-computed so per-layer hot path has zero config
           resolution work. See §"Variant-Chain Region Splitting" below and
           `docs/02_ir_schemas.md` (IR 5) for the RegionKey/RegionPlan shapes.

PrePass::PaintSegmentation
  Input:  MeshIR (with whole-triangle paint assignments normalized at load)
          SliceIR (committed by host:slice, after shell_classification)
          LayerPlanIR (authoritative global Z sequence)
          RegionMapIR (variant chains from RegionMapping)
  Output: SliceIR (per-variant SlicedRegion entries via replace_slice_ir)
  Purpose: For every layer Z and every region-split semantic, compute the 2D
           polygon regions that carry that semantic's paint value using
           OrcaSlicer-parity Voronoi-based segmentation. Writes per-variant
           polygons into SliceIR.regions via replace_split_ir. Each
           SlicedRegion carries its variant_chain. PaintRegionIR is deleted.
           See `docs/specs/orca-paint-segmentation-parity.md` for the full
           7-phase algorithm.

PrePass::SupportGeometry  [host built-in always runs; guest optional]
  Input  (host built-in): LayerPlanIR + MeshIR
  Input  (guest, if a `support-planner` module is loaded):
                          MeshIR
                          SurfaceClassificationIR
                          LayerPlanIR
                          RegionMapIR
                          SupportGeometryIR (just committed by the host built-in)
                          SliceIR (per-variant regions from PaintSegmentation)
  Output (host built-in): SupportGeometryIR
  Output (guest):         SupportPlanIR
  Purpose: Phase 1 — the host built-in computes coarse support column outlines
           via plane-triangle intersection at support layer boundaries. Support
           layer height is controlled by `support_layer_height_mm`
           (default 0 = model layer height). Near model contact zones
           (`support_top_z_distance_mm`), adds intermediate layers at model
           resolution so the top distance is honored precisely. Runs after
           `execute_prepass` so `LayerPlanIR` is always committed first.
           Phase 2 — when a `support-planner` guest is loaded, the host invokes
           it via the WIT export `run-support-geometry` after Phase 1's
           `SupportGeometryIR` is on the blackboard. The guest performs
           multi-layer organic tree-support planning: walks layers top-to-bottom,
           extracts contact points from overhang/bridge facets and SupportEnforcer
           paint, and propagates them through a per-layer Prim minimum spanning
           tree (simplified port of OrcaSlicer `TreeSupport::drop_nodes`). Emits
           per-(layer, object, region) branch geometry as `SupportPlanIR` that
           `Layer::Support` modules consume directly when present. When no
           support-planner module is installed only Phase 1 runs and
           tree-support falls back to its per-layer grid-MST filler.
```

#### Variant-Chain Region Splitting

`variant_chain` is the ordered sequence of `(paint_semantic_name, paint_value)`
pairs that distinguishes a **painted variant** from its base region. Two regions
of the same object and base region with different variant chains are distinct
for module dispatch and configuration purposes. An empty chain identifies the
base (unpainted) region.

The chain is the discriminator that splits regions: `PrePass::RegionMapping`
enumerates every subset of declared region-split semantics × distinct paint
values present on the object, producing one `RegionPlan` per canonical chain.
`PrePass::PaintSegmentation` then computes the geometric polygons for each
variant chain via per-semantic Voronoi passes and geometric composition
(`intersection_ex` / `difference_ex`), writing the results into
`SliceIR.regions` via `replace_slice_ir`. Each `SlicedRegion` carries its
`variant_chain`; modules dispatch by matching against it. See
`docs/02_ir_schemas.md` for the `RegionKey.variant_chain` and
`SlicedRegion.variant_chain` field definitions, and `docs/03_wit_and_manifest.md`
for the `[[region_split]]` manifest declaration schema.

#### Catch-Up Layer Semantics (Authoritative)

When objects have different layer heights, global layer planning uses sync points so all
objects align on common Z planes.

Example:

- Object A: 0.2mm layer height
- Object B: 0.3mm layer height
- Sync interval: LCM(0.2, 0.3) = 0.6mm

At `Z=0.6`, Object B may execute a catch-up layer spanning `Z ∈ [0.3, 0.6]`.
At that region-layer:

- `is_catchup_layer = true`
- `catchup_z_bottom = 0.3`
- `effective_layer_height = 0.3` (or wider if configured by region-specific planning)

Catch-up behavior is computed in `PrePass::LayerPlanning` and is never recomputed in Tier 2.

### Tier 2 — Per-Layer (Parallel via rayon)

Each layer runs independently. Layers share no mutable state. The Blackboard is read-only during this tier.

```
Layer::Slice
  Input:  MeshIR (immutable), LayerPlanIR (immutable)
  Output: SliceIR
  Purpose: Triangle/plane intersection. Loop chaining. Clipper union.

Layer::SlicePostProcess
  Input:  SliceIR
          PaintRegionLayerView (read-only, from Blackboard)
  Output: SliceIR (modified)
  Purpose: Non-planar surface projection onto layer polygons.
           Sub-layer anti-aliasing vertex Z deformation.
           Modifier region polygon subtraction/addition.
           PaintRegionAnnotator — reads paint region polygons from
           SliceIR / RegionMapIR, performs point-in-polygon tests, and
           writes segment_annotations onto SlicedRegion polygon contour
           points. Runs last within this stage so all polygon modifications
           are complete before annotation occurs.

Layer::Perimeters
   Input:  SliceIR (including segment_annotations from SlicePostProcess)
          PaintRegionLayerView (read-only, for paint-driven boundary detection)
  Output: PerimeterIR
  Purpose: Wall generation (Arachne variable-width or classic fixed-width).
           Seam candidate collection.
           Thin-wall detection.
           Propagates segment_annotations from SlicedRegion polygon
           contour points onto WallLoop.feature_flags for each generated
           wall segment. Sets WallLoop.boundary_type to MaterialBoundary
           where adjacent material semantic regions are detected via
           PaintRegionLayerView.
           Commit side-effect: the host computes the four canonical
           pairwise-disjoint fill polygons (`sparse_infill_area`, clipped
           `top_solid_fill`, `bottom_solid_fill`, `bridge_areas`) into the
           per-layer arena's `SliceIR` from `perimeter.infill_areas` via
           `sync_perimeter_infill_areas_into_slice`. Precedence
           `bridge > bottom > top > sparse` (OrcaSlicer parity). See
           `crates/slicer-runtime/src/region_partition.rs` and
           `docs/specs/infill-fill-partition-plan.md`.

Layer::PerimetersPostProcess
  Input:  PerimeterIR (including feature_flags and boundary_type on WallLoops)
  Output: PerimeterIR (modified)
  Purpose: Seam placement and optimization.
           Bricklayer Z-shifting.
           Interlocking wall zigzag generation.
           Phase-alternated non-planar wall modulation.
           Smoothificator multi-pass outer wall expansion.
           FuzzySkin — reads WallLoop.feature_flags.fuzzy_skin per segment
           and applies perpendicular XY perturbation only to flagged segments.
           Unflagged segments on the same loop retain their original XY path.

Layer::Infill
  Input:  SliceIR (each region's four canonical pairwise-disjoint fill polygons,
                   populated by the host at Layer::Perimeters commit)
  Output: InfillIR
  Purpose: Infill pattern generation (rectilinear, gyroid, TPMS, lightning, etc.).
           Each fill claim holder reads exactly one of:
             claim:sparse-fill → SlicedRegion.sparse_infill_area → SparseInfill
             claim:top-fill    → SlicedRegion.top_solid_fill     → TopSolidInfill
             claim:bottom-fill → SlicedRegion.bottom_solid_fill  → BottomSolidInfill
             claim:bridge-fill → SlicedRegion.bridge_areas       → BridgeInfill
           No per-region role-pick; no polygon math in modules.

Layer::InfillPostProcess
  Input:  InfillIR
  Output: InfillIR (modified)
  Purpose: Non-planar sine-wave infill modulation.
           Infill-wall interlocking.

Layer::Support
  Input:  SliceIR
          SurfaceClassificationIR
          PaintRegionLayerView (read-only — SupportEnforcer and SupportBlocker semantics)
          SupportPlanIR (read-only, optional — only modules that declare the
                         read consume it; produced by PrePass::SupportGeometry)
  Output: SupportIR
  Purpose: Traditional or tree support geometry generation.
           Enforcer/blocker priority rules (applied in this order):
             1. SupportBlocker region → no support regardless of overhang angle
             2. SupportEnforcer region → support regardless of overhang angle
             3. Otherwise → support if overhang angle exceeds config threshold
           Planner-consuming tier (TASK-161): modules holding the `support-generator`
           claim that also declare `SupportPlanIR` as a read (e.g. `tree-support`)
           emit committed branch geometry directly when the plan is present, and
           fall back to their per-layer filler otherwise. Modules whose algorithm
           is inherently per-layer (e.g. `traditional-support` with its scan-line
           fill) intentionally do not declare the read.

Layer::SupportPostProcess
  Input:  SupportIR
  Output: SupportIR (modified)
  Purpose: Support top surface ironing.
           Support interface density variation.
           Any future support geometry post-processing.

Layer::PathOptimization
  Input:  PerimeterIR + InfillIR + SupportIR
  Output: LayerCollectionIR
  Purpose: Travel minimization.
           Retraction decision.
           Pressure advance smoothing.
           Sub-layer anti-aliasing DAG topological sort (collision-aware ordering).
```

#### Paint Propagation Contract (Authoritative)

Paint-dependent behavior follows this strict sequence:

1. `Layer::SlicePostProcess` writes `SlicedRegion.segment_annotations` (runs after all polygon edits).
2. `Layer::Perimeters` maps segment annotations into `WallLoop.feature_flags` and boundary metadata.
3. `Layer::PerimetersPostProcess` consumes `feature_flags` (for example `fuzzy_skin`) for selective effects.

If step 1 is skipped or fails non-fatally, later steps continue with empty/default paint annotations,
which is a correctness degradation. Modules should return fatal errors for unrecoverable paint annotation failures.

#### Paint Annotation Failure Semantics (Normative)

Failure classes:

- Fatal:
  - `PaintRegionIR` is unavailable for a layer that declares paint-dependent semantics.
  - Annotation cardinality cannot be made parallel to contour points.
  - Overlap resolution encounters equal-precedence conflicting paint values.
- Non-fatal (degraded allowed):
  - A point cannot be classified after all polygon edits due to numerical edge ambiguity.

Required fallback behavior for non-fatal cases:

- `segment_annotations` must still be present and cardinality-aligned with contour points.
- Unresolved points must use deterministic defaults:
  - `tool_index = 0`
  - `fuzzy_skin = false`
  - `support_enforcer = false`
  - `support_blocker = false`
- Host must emit a structured warning event and mark slice result as `degraded=true`.

Recommended error codes:

- `501` = `PAINT_REGION_MISSING`
- `502` = `PAINT_ANNOTATION_CARDINALITY_MISMATCH`
- `503` = `PAINT_PRECEDENCE_CONFLICT`
- `504` = `PAINT_POINT_UNRESOLVED_FALLBACK`

#### Non-Planar Z Envelope Rules

For any module that writes path Z in Tier 2:

- Lower bound: `layer.z`
- Upper bound: `layer.z + effective_layer_height`

Stages may not emit Z outside this envelope. Violations are treated as fatal contract errors.

#### Per-Layer Error Handling Rules

- `fatal = true`: abort entire slice immediately.
- `fatal = false`: continue with last valid IR state for that layer and stage.

Non-fatal mode is only for graceful degradation, never for contract or geometry-integrity failures.

#### Module Access Contract (Normative)

All modules must declare complete IR access contracts in manifest `[ir-access].reads` / `[ir-access].writes`.

Rules:

- Undeclared reads are forbidden. Host must deny access and return a fatal contract error.
- Undeclared writes are forbidden. Host must reject commit and return a fatal contract error.
- Modules may only read fields available from upstream stages in `STAGE_ORDER`.
- Blackboard access is least-privilege: each module receives a read/write mask derived strictly from manifest declarations.

Rationale:

- Prevents hidden coupling through implicit Blackboard reads.
- Makes DAG validation and compatibility analysis deterministic.

### Tier 3 — PostPass (Sequential, Whole-Print)

All PostPass stages run after all per-layer processing is complete.
The full `Vec<LayerCollectionIR>` is visible to every stage in this tier.
None of these stages may be parallelized.

```
PostPass::LayerFinalization
  Input:  Vec<LayerCollectionIR> (all layers, mutable — may append entities or insert synthetic layers)
          Blackboard (immutable)
  Output: Vec<LayerCollectionIR> (modified in place)
  Purpose: Cross-layer features that require visibility of the full print
           before G-code can be emitted.
  Built-in modules:  WipeTower, Skirt/Brim, PartCooling
  Optional modules:  SequentialPrintOrder, MinLayerTimeEnforcer,
                     FlushVolumeCalculator, PrimeTower

PostPass::GCodeEmit  [host-built-in]
  Input:  Vec<LayerCollectionIR> (after LayerFinalization)
  Output: GCodeIR
  Purpose: Serialize ordered extrusion entities to structured G-code commands.
           Tool change / wipe tower sequencing.
           Fan speed and temperature scheduling.
           Machine-level start and end G-code (printer preamble / postamble)
           is module-owned (see machine start/end note below).

PostPass::GCodePostProcess
  Input:  GCodeIR
  Output: GCodeIR (modified)
  Purpose: Structured mutation of G-code commands.

PostPass::TextPostProcess  [last resort only]
  Input:  serialized G-code string
  Output: serialized G-code string (modified)
  Purpose: Raw text mutation for features that cannot be expressed in GCodeIR.
           Single-threaded. Use sparingly.
```

#### Stable Entity-ID Invariant (packet 39)

Entities and travels within `LayerCollectionIR` carry stable `u64 entity_id` values assigned at producer time by per-layer `LayerEntityIdGen` instances. `PostPass::LayerFinalization` may sort, insert, drop, or mutate entities without invalidating travel anchors — anchors reference IDs, not positional indices. The host validates `entity_id` resolution and rejects mutations that target unknown IDs (packet 39). See `docs/02_ir_schemas.md` IR 10 "Stable entity IDs" for the full contract.

#### Finalization Mutation API (packets 40 / 41)

`PostPass::LayerFinalization` holds the primary-responsibility role for layer mutation in the post-PrePass phase. Modules call serialisable mutation primitives: `push_entity_with_priority`, `modify_entity`, `sort_layer_by`, and `insert_synthetic_layer_after`. Packet 40 introduced the primitives; packet 41 made the mutation enums serialisable across the WIT boundary via `EntityMutation`, `SortKey`, and `SyntheticLayerData`. The closure-based draft from packet 40 is superseded by the enum-based API. See `docs/05_module_sdk.md` "Finalization mutation API".

#### PathOptimization Module Ordering (packet 33)

Nearest-neighbour entity ordering lives in the `path-optimization-default` module (packet 33). The host no longer carries an entity-ordering fallback — packet 18 is marked superseded. If no module calls `LayerCollectionBuilder::set_entity_order`, the host emits entities in the raw assembly order produced by `assemble_ordered_entities` (perimeters, then infill, then support per region, in `RegionMapIR` order). Modules that need NN ordering must explicitly request it; the host does not synthesise it.

#### Retract and Z-Hop Policy Ownership (packet 15)

`path-optimization-default` is the canonical owner of retract / no-retract policy and Z-hop planning for the `Layer::PathOptimization` stage. External travels (region-to-region or object-to-object) emit retract → travel → unretract; internal travels (within one connected region) emit travel only. Z-hop entries are deferred to the per-layer `z_hops` queue and matched at finalization. Every retract must be paired with a downstream unretract; mismatches are caught by validation at `LayerCollectionBuilder` commit.

#### Support Stage Paint Precedence (packet 13)

`Layer::Support` resolves paint-driven enforcement in this order before any geometric overhang test:

1. `PaintSemantic::SupportBlocker` region → no support, regardless of `needs_support` or overhang angle.
2. `PaintSemantic::SupportEnforcer` region → support is committed, regardless of `needs_support` or overhang angle.
3. Otherwise → fall through to the per-module algorithm (overhang angle threshold, planner consumption).

This makes paint semantics the highest authority on whether a region carries support; the geometric pass only runs when paint is silent.

#### MMU Tool-Index Propagation (packet 50b)

When `paint-segmentation` produces `PaintSemantic::Material` regions with `PaintValue::ToolIndex(n)`, the tool index is propagated end-to-end:

1. `Layer::Perimeters` writes `tool_index` onto `WallLoop.feature_flags` via paint annotation.
2. `Layer::PathOptimization`'s `assemble_ordered_entities` calls `dominant_tool_index()` on each entity's feature flags and stamps the dominant value into `RegionKey.region_id` (paint-derived regions reuse the `region_id` slot to carry tool index, since multi-material print regions are otherwise indistinguishable).
3. `PostPass::GCodeEmit` emits `GCodeCommand::ToolChange { from, to }` at each `region_id` transition, surfacing as `T{n}` in the serialized G-code.

The flow is purely data-driven; no module needs to opt in beyond declaring `paint-segmentation` reads on `Material` semantics.

#### Post-Finalization Travel Reconciliation (packet 20)

After `PostPass::LayerFinalization` commits, the host performs a second-pass travel reconciliation. Skirt, brim, wipe-tower, and prime-tower entities inserted by finalization modules have endpoints the per-layer `Layer::PathOptimization` pass could not have seen, so the host recomputes travel transitions against those new endpoints. The reconciliation is bounded:

- Only travel-move `entity_id` and endpoint XY are recomputed; **model extrusion entity ordering is invariant**.
- The reconciliation runs before `PostPass::GCodeEmit` so emit sees a consistent travel graph.
- Travel rejections (retract/unretract pairing, Z-hop matching) re-validate at this point; mismatches surface as fatal `RECONCILED_TRAVEL_INCONSISTENT` errors.

This phase has no module-visible surface; it is a host built-in tucked between `PostPass::LayerFinalization` and `PostPass::GCodeEmit`.

#### Ironing Relocation (packet 38-rev1)

Top-surface ironing is performed at `PostPass::LayerFinalization` (packet 38-rev1), not at `Layer::InfillPostProcess`. The relocation gives the ironing module the full-layer-sequence visibility needed to detect topmost-layer indices via the multi-layer `top_solid_layers` window.

#### Part Cooling Fan Modulation (packet 53)

Part cooling fan modulation lives in `PostPass::LayerFinalization`. The `PartCooling` module reads per-layer time budgets and target temperatures from `ResolvedConfig` (`fan_speed_min`, `fan_speed_max`, `slow_down_for_layer_cooling`, `min_layer_time`) and inserts `GCodeCommand::FanSpeed` entities at the appropriate layer transitions (packet 53).

#### Toolchange and Purge Integration (packet 58)

Multi-tool toolchanges flow through `GCodeCommand::ToolChange { from, to }` in the post-finalization stream. When `WipeTower` or `PrimeTower` is loaded, the finalization module that holds the corresponding claim inserts purge moves (extrude, wipe, position) between the `ToolChange` command and the next non-toolchange entity. Purge moves use `ExtrusionRole::WipeTower` or `ExtrusionRole::PrimeTower` so the per-role speed dispatch (packet 52) applies. Packet 58 wired this integration end-to-end; before this packet, toolchange G-code was emitted without coordinated purge moves. See `docs/02_ir_schemas.md` IR 11 for the `ToolChange` command shape.

#### Machine Start / End G-code Emission Boundary (packet 59)

Machine-level start and end G-code (printer preamble / postamble) is module-owned. A designated finalization module reads `machine_start_gcode` and `machine_end_gcode` from `ResolvedConfig`, expands macros (`{first_layer_temperature}`, `{bed_temperature}`, `{filament_type}`, `{nozzle_diameter}`, `{tool_count}`, `{layer_count}`, `{print_time_estimate_s}`, `{x_max}`, `{y_max}`, `{z_max}`), and emits the resulting commands before the first layer (start) and after the last layer (end). Packet 59 moved this from a host built-in to a module-owned contract; see `docs/03_wit_and_manifest.md` "Machine start / end G-code emission" for the manifest surface.

---

## Data Dependency Matrix

This section is authoritative for stage-level IR contracts. If a module manifest
declares reads/writes that contradict this table, the manifest is incorrect.

### Stage I/O Contract (Reads and Writes)

| Stage                                    | Reads                                                              | Writes                                                              |
|------------------------------------------|--------------------------------------------------------------------|---------------------------------------------------------------------|
| `PrePass::MeshAnalysis`                  | `MeshIR`                                                           | `SurfaceClassificationIR`                                                                                |
| `PrePass::LayerPlanning`                 | `MeshIR`, `SurfaceClassificationIR`, global/object/modifier config | `LayerPlanIR`                                                                                            |
| `PrePass::OverhangAnnotation`            | `MeshIR`, `LayerPlanIR`                                            | `SurfaceClassificationIR.overhang_quartile_polygons` (per-layer quartile bands; introduced P106)         |
| `PrePass::PaintSegmentation`             | `MeshIR`, `SurfaceClassificationIR`, `LayerPlanIR`                 | `SliceIR` (via `replace_slice_ir`; per-variant polygons)                                                 |
| `PrePass::RegionMapping` (host-built-in) | `LayerPlanIR`, loaded modules, resolved config                     | `RegionMapIR`                                                       |
| `PrePass::SupportGeometry` (optional)   | `MeshIR`, `LayerPlanIR`, `RegionMapIR`, `SupportGeometryIR`        | `SupportGeometryIR` (host-committed), `SupportPlanIR` (guest-emitted) |
| `Layer::Slice`                           | `MeshIR`, `LayerPlanIR`                                            | `SliceIR`                                                           |
| `Layer::SlicePostProcess`                | `SliceIR`, `PaintRegionLayerView`                                  | `SliceIR` (polygon edits, `segment_annotations`)                    |
| `Layer::Perimeters`                      | `SliceIR`, `PaintRegionLayerView`                                  | `PerimeterIR` (`feature_flags`, seam candidates, boundary metadata) |
| `Layer::PerimetersPostProcess`           | `PerimeterIR`                                                      | `PerimeterIR` (seam/geometry refinements)                           |
| `Layer::Infill`                          | `SliceIR` (infill areas and context)                               | `InfillIR`                                                          |
| `Layer::InfillPostProcess`               | `InfillIR`                                                         | `InfillIR`                                                          |
| `Layer::Support`                         | `SliceIR`, `SurfaceClassificationIR`, `PaintRegionLayerView`, `SupportPlanIR` (optional, declared per module) | `SupportIR`                                                         |
| `Layer::SupportPostProcess`              | `SupportIR`                                                        | `SupportIR`                                                         |
| `Layer::PathOptimization`                | `PerimeterIR`, `InfillIR`, `SupportIR`                             | `LayerCollectionIR`                                                 |
| `PostPass::LayerFinalization`            | `Vec<LayerCollectionIR>`, Blackboard IRs                           | `Vec<LayerCollectionIR>` (may insert synthetic layers)              |
| `PostPass::GCodeEmit` (host-built-in)    | `Vec<LayerCollectionIR>`                                           | `GCodeIR`                                                           |
| `PostPass::GCodePostProcess`             | `GCodeIR`                                                          | `GCodeIR`                                                           |
| `PostPass::TextPostProcess`              | serialized G-code text                                             | serialized g-code text                                              |

### Cross-Stage Dependency Matrix (Scheduler-Relevant)

`X` indicates the consumer stage depends on data written by the producer stage.

| Producer \ Consumer   | MeshAnalysis | LayerPlanning | PaintSegmentation | SupportGeometry | RegionMapping | Slice | SlicePostProcess | Perimeters | Infill | Support | PathOptimization | LayerFinalization | GCodeEmit |
|-----------------------|--------------|---------------|-------------------|-----------------|---------------|-------|------------------|------------|--------|---------|------------------|-------------------|-----------|
| MeshAnalysis          |              | `X`           | `X`               | `X`             |               |       |                  |            |        | `X`     |                  |                   |           |
| LayerPlanning         |              |               | `X`               | `X`             | `X`           | `X`   |                  |            |        |         |                  |                   |           |
| PaintSegmentation     |              |               |                   | `X`             |               |       | `X`              | `X`        |        | `X`     |                  |                   |           |
| SupportGeometry       |              |               |                   |                 |               |       |                  |            |        | `X`     |                  |                   |           |
| RegionMapping         |              |               |                   |                 |               | `X`   | `X`              | `X`        | `X`    | `X`     | `X`              |                   |           |
| Slice                 |              |               |                   |                   |               |       | `X`              | `X`        | `X`    | `X`     |                  |                   |           |
| SlicePostProcess      |              |               |                   |                   |               |       |                  | `X`        |        |         |                  |                   |           |
| Perimeters            |              |               |                   |                   |               |       |                  |            |        |         | `X`              |                   |           |
| PerimetersPostProcess |              |               |                   |                   |               |       |                  |            |        |         | `X`              |                   |           |
| Infill                |              |               |                   |                   |               |       |                  |            |        |         | `X`              |                   |           |
| InfillPostProcess     |              |               |                   |                   |               |       |                  |            |        |         | `X`              |                   |           |
| Support               |              |               |                   |                   |               |       |                  |            |        |         | `X`              |                   |           |
| SupportPostProcess    |              |               |                   |                   |               |       |                  |            |        |         | `X`              |                   |           |
| PathOptimization      |              |               |                   |                   |               |       |                  |            |        |         |                  | `X`               |           |
| LayerFinalization     |              |               |                   |                   |               |       |                  |            |        |         |                  |                   | `X`       |

Notes:

- `RegionMapping` dependencies are execution-context dependencies (module selection and config views), not geometry mutation.
- `Layer::Infill` may consume `SliceIR` areas derived after perimeter planning; scheduler treats this as a read dependency on current-layer staged state.
- `PostPass::TextPostProcess` intentionally bypasses structured IR and should not be used by modules that can express behavior through `GCodeIR`.

---

## Data Ownership Rules

These rules are enforced by the host at runtime. Violations trap the WASM module.

| Data                                          | Owner                             | Module Access                                                   |
|-----------------------------------------------|-----------------------------------|-----------------------------------------------------------------|
| `MeshIR`                                      | Host (permanent)                  | Query-only via host-services API (raycasts, normals, bounds)    |
| `SurfaceClassificationIR`                     | Host Blackboard                   | Read-only view                                                  |
| `LayerPlanIR`                                 | Host Blackboard                   | Read-only view                                                  |
| `SupportPlanIR`                               | Host Blackboard                   | Read-only view (only modules that declare the read see it)      |
| `RegionMapIR`                                 | Host Blackboard                   | Read-only view (own region only)                                |
| Per-layer IR (Slice, Perimeter, Infill, etc.) | Per-layer arena                   | Read view of previous stages; write builder for declared output |
| `LayerCollectionIR`                           | Per-layer arena → Host after join | Read-only in PostPass                                           |
| `GCodeIR`                                     | Host PostPass buffer              | Write builder (GCodePostProcess only)                           |

---

## Memory Model

```
Host process memory layout:

┌─────────────────────────────────────┐
│ Static allocations (program lifetime)│
│  - MeshIR (loaded once)              │
│  - ExecutionPlan (frozen at startup) │
│  - WasmInstancePools                 │
│  - Blackboard (PrePass outputs)      │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐  ← allocated before layer loop
│ LayerCollectionIR slots             │
│  Vec<Option<LayerCollectionIR>>     │
│  Size: N_layers * ~avg_layer_size   │
│  Written once per slot (no mutex)   │
└─────────────────────────────────────┘

Per rayon thread (N threads, N ≤ CPU cores):
┌─────────────────────────────────────┐  ← allocated per layer, freed after
│ LayerArena (bump allocator)         │
│  SliceIR, PerimeterIR, InfillIR,    │
│  SupportIR (intermediate, discarded │
│  once LayerCollectionIR is built)   │
└─────────────────────────────────────┘

Per WASM instance:
┌─────────────────────────────────────┐
│ WASM linear memory                  │
│  Module's own working memory        │
│  No shared memory with host         │
│  IR data crosses boundary as        │
│  serialized structs, not pointers   │
└─────────────────────────────────────┘
```

---

## Inter-Process Communication (Host ↔ Frontend)

The host emits a line-delimited JSON event stream during slicing. **For the
authoritative event schema, ordering guarantees, and transport details, see
`./docs/09_progress_events.md`.** Event types are `phase_start`,
`phase_complete`, `layer_start`, `layer_complete`, `module_error`,
`validation_error`, and `slice_complete`. The runtime emitter is implemented in
`crates/slicer-runtime/src/progress_events.rs`.

The frontend can also query the loaded modules' config schemas (one entry per
module, per field — `{key, type, values, default, display, group}`). The CLI
subcommand and JSON shape are implemented in `crates/pnp-cli/src/main.rs`
(`ConfigSchema` subcommand) and documented in `./docs/03_wit_and_manifest.md`
under "Manifest config schema query".

---

## Claim System

> **Where each facet of the claim system lives.** This section owns the concept
> and the normative **Allowed Claim Transition Matrix** (below). The manifest
> `[claims]` declaration syntax and the full known-claim catalog (including the
> four fill-role claims) live in `docs/03_wit_and_manifest.md` § "Known claim
> IDs"; the fill-role claim-holder config keys are `ResolvedConfig` fields in
> `docs/02_ir_schemas.md`; runtime claim resolution, the DAG validation passes,
> and `effective_claim_holders()` are authoritative in
> `docs/04_host_scheduler.md` § "Claim Resolution with Runtime Disable Rules".

Claims are named exclusive resource slots. They prevent two modules from both trying to generate perimeters (or infill, or supports) for the same region simultaneously.

```
Built-in claim names:
  perimeter-generator     — generates wall loops for a region
  infill-generator        — generates infill paths for a region
  support-generator       — generates support structures (Layer::Support)
  support-planner         — plans multi-layer support branches in PrePass
                            (PrePass::SupportGeometry; orthogonal to
                            support-generator)
  seam-placer             — resolves seam position
  layer-planner           — contributes to Z-plane sequence
  mesh-analyzer           — contributes to surface classification
  slice-postprocessor     — post-processes slice polygons / paint annotation
  gcode-postprocessor     — post-processes structured GCode commands
  text-postprocessor      — post-processes serialized GCode text
  gcode-emitter           — serializes GCodeIR to text (host-built-in, non-claimable)
```

A module may declare `holds = ["infill-generator"]`. Only one module per region may hold a given claim. If two modules both hold the same claim globally and no region override resolves the conflict, the host rejects the configuration at startup with a precise error.

Region overrides allow different infill generators per region:

```toml
# In print config:
[[region_override]]
modifier_mesh = "vase_top_modifier.stl"
module_overrides = { "infill-generator" = "com.community.gyroid-infill" }
```

### Claim Conflict Resolution (Normative)

Resolution order:

1. Apply global enable/disable rules.
2. Apply object-level overrides.
3. Apply region-level overrides.
4. Validate uniqueness for every `(layer, object, region, claim)`.

Determinism constraints:

- A claim holder must be stable across layers for the same `(object, claim)` unless the stage explicitly supports temporal transitions.
- If two overrides of equal precedence select different holders for the same key, startup validation fails.
- If no holder remains for a required claim, startup validation fails.

Worked example — valid:

- Object A uses `com.core.gyroid` for `infill-generator` globally.
- Region override on object top selects `com.community.tpms`.
- Result: deterministic because each region resolves to one holder and the holder does not change across layers.

Worked example — invalid:

- Layer-range override selects `com.core.gyroid` for layers `0..49` and `com.community.tpms` for `47..99` on same object claim.
- Result: rejected unless stage contract explicitly allows per-layer claim transitions.

#### Allowed Claim Transition Matrix

Unless explicitly listed as transition-capable below, the claim holder must remain stable across all layers for one `(object_id, claim)`.

| Claim                 | Allowed per-layer transition | Notes                                                                                   |
|-----------------------|------------------------------|-----------------------------------------------------------------------------------------|
| `infill-generator`    | Yes                          | Region-local transitions allowed when layer-range overrides do not overlap ambiguously. |
| `support-generator`   | Yes                          | Allowed for planned support strategy shifts across geometry phases.                     |
| `support-planner`     | No                           | PrePass global planner must be unique and stable; multi-layer propagation requires a single holder. |
| `perimeter-generator` | No                           | Must remain stable to preserve wall continuity assumptions.                             |
| `seam-placer`         | No                           | Must remain stable for seam scoring consistency.                                        |
| `layer-planner`       | No                           | PrePass global planner must be unique and stable.                                       |
| `mesh-analyzer`       | No                           | Classification baseline must not vary across layers.                                    |

Validation rule:

- If a claim is marked non-transitionable, any layer-varying holder selection is a startup validation error.

---

## Module Search Path

`pnp_cli` assembles module search roots from CLI flags, an env var, and
two platform defaults, in the priority order listed below. Within each root
the discovery contract is unchanged: `*.toml` manifests at the root level or
one subdirectory deep, each requiring a same-stem `*.wasm` companion.
Assembly lives in `crates/slicer-runtime/src/module_search_path.rs`
(`assemble_search_roots`); per-root scanning and intra-root `module.id`
deduplication live in `crates/slicer-runtime/src/manifest.rs`
(`load_modules_from_roots`).

### Priority tiers (highest first)

1. **`--module-dir <PATH>`** — CLI flag, repeatable; entries are taken in
   the order given on the command line.
2. **`SLICER_MODULE_PATH`** — env var, OS PATH-style list (split via
   `std::env::split_paths`). Entries are appended after all CLI dirs.
3. **`{config_dir}/modules/`** — resolved via
   `directories::ProjectDirs::from("", "", "modular-slicer").config_dir()`.
   On Linux this is `$XDG_CONFIG_HOME/modular-slicer/modules/` (typically
   `~/.config/modular-slicer/modules/`); on macOS,
   `~/Library/Application Support/modular-slicer/modules/`; on Windows,
   `%APPDATA%\modular-slicer\config\modules\`. Silently skipped if absent.
4. **`{executable_dir}/modules/`** — relative to the running binary via
   `std::env::current_exe()`. Silently skipped if absent.

Tiers 3 and 4 are omitted entirely when `--no-default-module-paths` is
passed. Tiers 1 and 2 always apply.

### Canonical-path deduplication of roots

Before scanning, the assembled list is deduplicated by canonical absolute
path (`std::fs::canonicalize`; falls back to `std::path::absolute` for
paths that do not yet exist on disk). The first occurrence of each path
wins; later duplicates are dropped silently. This is a user-friendly
dedup, not a configuration error.

### Intra-root `module.id` deduplication

After roots are assembled, `load_modules_from_roots` walks them in order
and dedups discovered modules by `module.id`. The first root's module
wins, and later duplicates emit a `DiagnosticLevel::Warning` on stderr.
This is independent of (and runs after) the canonical-path dedup above.

### Per-root layout

Each loadable module directory must contain:

- `<module-name>.wasm` — compiled WASM component
- `<module-name>.toml` — manifest

Modules may live flat at the root or nested one level deep in
subdirectories (the layout used by `modules/core-modules/`). The scanner
recognises both; `Cargo.toml` files inside subdirectories are excluded.

### Producing the tier-4 layout: `cargo xtask dist`

`cargo xtask dist` is the canonical way to assemble the
`{executable_dir}/modules/` layout for shipping. It rebuilds every
core-module guest WASM, builds `pnp_cli` (release by default; `--debug`
opt-in), wipes `target/dist/`, and stages:

```text
target/dist/
├── pnp_cli[.exe]
└── modules/
    └── <module-name>/
        ├── <module-name>.toml
        └── <module-name>.wasm   (one subdir per core module; 21 today)
```

Because tier 4 of the search path resolves to `current_exe()/modules/`,
running `target/dist/pnp_cli` with no `--module-dir` flags discovers all
staged modules automatically. Test-guests under
`crates/slicer-wasm-host/test-guests/` are filtered out — the bundle
contains shippable core modules only.

The wipe-then-stage step guarantees deleted or renamed core modules do
not linger in old dist bundles. Implementation lives in
`xtask/src/dist.rs` and reuses `build_guests::discover_guests` so the
shipped set tracks the same validated walk used by `cargo xtask
build-guests`.

### Diagnostics

Setting `SLICER_DEBUG_PATHS=1` causes the host to print the assembled
roots on stderr in priority order before module discovery begins.


## `PostPass::LayerFinalization` Module Constraint

Modules declaring `stage = "PostPass::LayerFinalization"` must set:

```toml
[hints]
layer-parallel-safe = false   # enforced — the host emits a warning if true is set
```

The host instantiates exactly one WASM instance for finalization modules regardless of CPU count. These modules are never pooled.

# Visual-Debug Silhouette Side Views — Plan

**Status:** Reviewed (grilled 2026-08-27; fresh-session two-axis review
2026-08-27 — all findings fixed in place; ready for batch packet generation
per D19)

This document is deliberately self-contained: it records every fact verified
during the grill session, every design decision with its rejected
alternatives and rationale, and the exact contract shapes — so a fresh
session can review it without access to the originating conversation.
Re-verify the Grounding Facts against the tree before trusting them; they
were true on 2026-08-27 on branch `parity/support-features`, and were
independently re-verified the same day by the fresh-session review (which
corrected fact 9 and the D7 `layer_z` claim — corrections are folded in
below and marked).

Prerequisite reading for the reviewer: `docs/19_visual_debug.md` (current
user-facing contract), `docs/specs/_OLD/visual-pipeline-debug.md` (archived
design spec — bundle contract, Projector single-owner rule, stage tap
inventory), `docs/08_coordinate_system.md` (units and Z convention).

---

## 1. Problem

`pnp_cli visual-debug` renders only top-down XY views. Tree-support defects —
branch tapering, interface-band placement and count, raft/base structure —
are inherently vertical: invisible from above, currently diagnosable only by
reading IR JSON or G-code text. A deterministic side view (the print
projected onto the X–Z or Y–Z plane) makes them inspectable at a glance and
comparable across pipeline stages, which is the tool's core value
proposition.

The motivating consumer is the tree-support family (`tree-support-planner`'s
`SupportPlanIR` tap, packet 221 lineage), but the design generalizes to every
tap whose captured IR can honestly place geometry on a Z axis.

## 2. Design In One Paragraph

A new visualization kind `silhouette` (schema **1.2.0**, with
`options.view: "front"` = X–Z projection, the default, or `"side"` = Y–Z)
composites all selected layers into **one image per (tap, view)**. Per
layer: each filled region's geometry is projected onto the horizontal axis
as intervals; intervals are unioned per (layer, color-class); each union
interval becomes an axis-aligned rectangle spanning that layer's Z slab;
rectangles are drawn back-to-front in a documented fixed class order through
the **existing** rasterizer (`Canvas::fill_polygon`) and the **existing**
shared `Projector`, fed `(x_or_y, z)` as its world point. The `Projector`'s
built-in y-flip means larger Z renders toward the top of the canvas — up is
up, with no new transform code.

Interval projection is *mathematically exact* for a silhouette: the
projection of a connected region onto an axis is a single interval, and holes
cannot disconnect a connected contour's projection. The information a
silhouette loses is depth (occlusion along the viewing axis) — inherent to
the view, handled by the paint-order rules and documentation caveats below.

## 3. Grounding Facts

Each fact was verified against the tree during the grill. Citations are by
symbol, per repo convention; a reviewer should re-resolve each symbol.

1. **One capture = one image today.** `render_stage_capture_styled`
   (`crates/slicer-runtime/src/visual_debug_render.rs`) takes one
   `StageCapture` (which carries `stage_id`, `layer_index`, `layer_z`, and a
   `CapturedIr`) and returns one `RenderedImage`. The bundle-assembly loop in
   `run_model_source` (`crates/pnp-cli/src/visual_debug.rs`) iterates
   `capture × visualization → ImageEntry`. A composite side view therefore
   needs a new per-bundle assembly path that groups captures by (tap, view)
   and calls a new composite render entry point once per group.

2. **`ViewportBoundsMm` is XY-only by design.** Its four fields are
   `min_x/min_y/max_x/max_y` (mm). `mesh_xy_bounds`
   (`crates/pnp-cli/src/visual_debug.rs`) reads `MeshIR::build_volume` and
   its doc comment says the Z extent is *deliberately* ignored. But
   `MeshIR::build_volume` is a `BoundingBox3`
   (`crates/slicer-ir/src/slice_ir.rs`), so a model-wide Z extent is already
   computed at load — no new geometry pass needed for the vertical axis of
   the viewport.

3. **Slab formula correction (the prior plan was wrong).** The
   carried-forward plan said rectangles span `[z, z + height]`. Two errors:
   - `GlobalLayer.z` is the layer **top**, not bottom: the layer planner
     (`modules/core-modules/layer-planner-default/src/lib.rs`) generates the
     first layer at `z = initial_layer_print_height`, and its test
     (`modules/core-modules/layer-planner-default/tests/layer_planning_tdd.rs`)
     pins `effective_layer_height == z − catchup_z_bottom` with
     `catchup_z_bottom < z`.
   - `GlobalLayer` (`crates/slicer-ir/src/slice_ir.rs`) carries **no height
     field at all** — only `index`, `z`, `active_regions`, `has_nonplanar`,
     `is_sync_layer`. `effective_layer_height` lives per-region: on
     `ActiveRegion` (inside `GlobalLayer.active_regions`), on `SlicedRegion`
     (inside `SliceIR.regions`), and on `ObjectLayerRef`. On catch-up layers
     the slab is region-dependent (a region that skipped the previous global
     Z reaches down to `catchup_z_bottom`).
   The correct slab is `[z − effective_layer_height, z]`, sourced per-region.

4. **Support Z indexing is not uniformly the model schedule.**
   - `SupportPlanIR.entries[].global_layer_index` is `i32`. The existing
     top-down renderer `support_geometry_shapes`
     (`crates/slicer-runtime/src/visual_debug_render.rs`) filters
     `global_layer_index >= 0` — negative (raft / below-model) indices exist
     in real plans and have no `GlobalLayer`/`SliceIR` row to source a Z slab
     from.
   - `SupportGeometryIR.entries` are keyed by `SupportGeometryKey.
     global_support_layer_index` on the support module's own
     `support_layer_height_mm` grid, which need not align with model layers.
     Caveat (2026-08-27 review): the field's own doc comment reads "Model
     layer index" with a `u32::MAX` sentinel for intermediate
     model-resolution layers, while `docs/02_ir_schemas.md` says "keyed by
     support-layer index" — the in-tree docs mildly contradict each other,
     and the "own grid" reading rests on docs 01/02. The packet
     implementing D9 must resolve which is true before finalizing W2's
     warning text (D9's skip-with-warning stance is safe either way).

5. **Not every blackboard tap is Z-attributable.** `BridgeRegion` and
   `OverhangRegion` (`crates/slicer-ir/src/slice_ir.rs`) carry
   `xy_footprint: Vec<ExPolygon>` plus `facet_indices` — **no Z or layer
   field**; recovering Z would require resolving facets against the loaded
   mesh (a dependency direction the render module deliberately lacks).
   `SeamPlanIR` carries seam *points*, not polygons (its top-down
   `shapes_for` arm returns an empty shape list by design).
   `SurfaceClassificationIR.overhang_quartile_polygons` **is** keyed by
   global layer index — the one per-layer-keyed field in that IR.

6. **`world_bounds_mm` byte-identity is a pinned bundle invariant.**
   `docs/19_visual_debug.md` ("Reading A Bundle") and the archived spec
   ("Bundle Contract") both document each rendered entry's `world_bounds_mm`
   as identical across every entry in a bundle, on both source modes. The
   `ImageEntry.world_bounds_mm` doc comment in
   `crates/pnp-cli/src/visual_debug.rs` repeats it ("byte for byte, because
   they all share one `viewport_bounds` binding"). A silhouette entry's
   bounds are X–Z or Y–Z — a different plane with different semantics.

7. **y-as-Z violates no coordinate invariant.** `docs/08_coordinate_system.md`
   governs IR geometry: X/Y polygonal geometry is scaled integers
   (`Point2::to_mm` to read mm), Z is mm floats end-to-end and must never
   round-trip through `mm_to_units`. The silhouette path reads X (or Y) via
   `Point2::to_mm` and Z as mm floats — both stay in their lanes. Pixel-space
   meaning is not governed by docs/08. The one binding rule is the archived
   spec's **Projector single-owner rule**: both existing render paths must
   use `Projector` (`crates/slicer-runtime/src/visual_debug_render.rs`)
   rather than defining their own transform (they each did originally and
   drifted — the spec records this). The silhouette path must feed
   `Projector::project(x_or_y, z)` and never own a transform. `Projector`
   already flips y (larger world-y → smaller row index), which for z-as-y
   yields larger Z toward the top: correct orientation for free.
   `VIEWPORT_MARGIN_MM` (2 mm, fixed, both axes) applies to the Z axis too —
   acceptable and consistent.

8. **Postpass capture shape hazard.** `run_postpass_taps`
   (`crates/pnp-cli/src/visual_debug.rs`) drives the whole pipeline prefix
   (all layers → finalization → postpass via
   `execute_postpass_with_capture`'s `PostPassCapture` sink), then builds
   **one `StageCapture` per (tap, applicable layer), each cloning the entire
   whole-print IR** (`capture.finalized_layers.clone()` /
   `capture.gcode_ir.clone()` per row). An all-layers silhouette selection
   through that path would clone the whole print once per layer — memory
   scales as (print size × layer count) on exactly the large models a side
   view is most useful for.

9. **Our emitter's flow model is rectangular; typed `Move.e` is an
   accumulated position, not a delta.** `DefaultGCodeEmitter`
   (`crates/slicer-gcode/src/emit.rs`) computes a per-move
   `e_delta = distance × point.width × height_delta × point.flow_factor /
   filament_area`, with
   `filament_area = π × (resolved_config.filament_diameter / 2)²`, then
   accumulates `e_position += e_delta` and emits
   `Move.e = Some(e_position)` for non-zero deltas and `None` for
   zero-delta moves — the typed field is the **cumulative E position**
   (corrected 2026-08-27; the grill session misread it as a per-move
   delta). `Retract`/`Unretract` are separate typed commands, but
   **negative deltas do flow through `Move.e`**: the emitter's own comment
   records that wipe-tower `generate_purge_paths` retract entities are
   deliberately emitted inline as negative-delta moves. The inversion must
   therefore recover each `Δe` by differencing consecutive `Some(e)`
   values (carrying the last seen position across `e: None` moves), treat
   `Δe < 0` as non-extruding, and only then apply
   `w = Δe × filament_area / (distance × height)` — which recovers the
   authored width **exactly** for PnP-generated G-code (modulo
   `flow_factor` — see the deposited-width caveat at the end of §4.4).
   OrcaSlicer's `Flow::mm3_per_mm` uses a stadium/rounded cross-section,
   so the rectangular inversion underestimates width on Orca-generated
   files by roughly `h(1 − π/4)` — sub-pixel at silhouette raster
   resolutions.

10. **`GCodeIR` has no layer markers.** Pinned by `gcode_shapes`'s doc
    comment (`crates/slicer-runtime/src/visual_debug_render.rs`):
    `GCodeCommand::Move` carries no `global_layer_index` and the typed IR has
    no `;LAYER_CHANGE` structure. (This is why the top-down GCodeEmit render
    draws the whole print unfiltered, and why its `filled_areas` fails closed
    via `MissingWidth` — `Move` has no width field.) Any per-layer
    silhouette over `GCodeIR` must bucket moves by Z.

11. **Seam positions carry Z.** `SeamPlanIR.entries[].chosen_candidate.point`
    is a `Point3WithWidth` in **millimeters** (pinned in the archived spec's
    Stage Tap Inventory row for `PrePass::SeamPlanning`). `LayerCollectionIR`
    carries no seam field (pinned by `layer_collection_events`'s
    `OverlayKind::Seams` arm comment), and final G-code carries no seam
    marker (pinned in `docs/19_visual_debug.md`). Seams on a side view are
    therefore model-source, SeamPlan-sourced only.

12. **Tool assignment exists only on** `LayerCollection`-family captures
    (`PrintEntity.tool_index`), `GCodeEmit` captures (`GCodeCommand::
    ToolChange` tracking, tool 0 initial), and the standalone G-code source
    (tracked `T<n>`, palette-only — no config to resolve filament colors
    from). Everything else rejects `color_by: "tool"` with
    `ToolColorUnavailable` — a pinned contract the silhouette must not
    loosen.

13. **The G-code source's parsed layer schedule** exists already:
    `visual_debug_gcode::parse_gcode` produces `ParsedLayer` rows with
    `layer_index` (parse order) and `layer_z` (from `;Z:` markers), used
    today for layer-selector resolution. `;TYPE:` role boundaries and
    per-move E values are parsed for the existing renders; extrusion-mode
    markers (absolute/relative E) are part of the documented supported
    subset.

14. **Blackboard taps are prepass-only.** `execute_blackboard_taps`
    (`crates/slicer-runtime/src/layer_executor.rs`) reads committed
    Blackboard slots after `prepare_prepass_context` — no `LayerArena`, no
    module dispatch, no per-layer execution. This is why a silhouette over
    blackboard taps costs nothing extra per layer, and why arena taps
    (`SUPPORTED_TAP_STAGE_IDS`: `Layer::Perimeters` …
    `Layer::PathOptimization`) are deferred: a side view over them would
    force executing every selected layer's truncated stage closure — for an
    all-layers silhouette, the whole print's per-layer work.

15. **Success criterion 3 of the archived spec** (model-backed requests
    don't emit G-code unless a final G-code view is selected) is preserved:
    blackboard-tap silhouettes run prepass only; PostPass-tap silhouettes go
    through the same `run_postpass_taps` machinery that already exists for
    top-down PostPass renders and is the documented minimal-closure
    deviation.

## 4. Decision Log

Every decision below was made explicitly in the grill session. Format:
**Decision** — rationale — *rejected alternatives*.

### 4.1 Geometry

- **D1. Slabs are per-region `[z − effective_layer_height, z]`** sourced
  from the tap's own IR (`SlicedRegion.effective_layer_height` for
  SliceIR-family taps). Exact for catch-up layers and multi-object mixed
  heights. Slab construction is per-(layer, region); a layer's rectangles may
  therefore have differing bottoms. — *Rejected:* uniform per-layer slab from
  consecutive `GlobalLayer.z` diffs (lies about catch-up layers — draws
  material where a region skipped a global Z); ship-uniform-now-refine-later
  (the tool's contract is "never a misleading image").

- **D2. Color classes: per-(layer, role) interval unions** — or per-(layer,
  tool) under `color_by: "tool"` (D12) — with a **documented fixed paint
  order**: body/generic roles first, support body, then interface roles
  last; tools in ascending index order. Within one class, intervals are
  unioned (overlap/touch merges; no epsilon fudging beyond exact
  interval-endpoint comparison). Classes are painted as separate rectangle
  sets in the fixed order, so a later class fully occludes an earlier one in
  overlapping X-ranges. **The occlusion caveat must be documented where an
  agent reading the PNG will see it** (docs/19 and/or manifest) — a hidden
  body interval may exist behind an interface band. — *Rejected:*
  single-color pure silhouette (interface bands — half the motivation —
  invisible); leaving paint order implementation-defined (breaks determinism
  ACs).

- **D3. Layer selection renders exactly the selected slabs, but the Z
  viewport is model-wide** from `MeshIR::build_volume`'s Z extent (unioned
  with captured geometry the same way the XY viewport unions today, so
  support/brim-like out-of-model geometry is never clipped). Consequence: a
  layers-10..20 bundle and an all-layers bundle frame identically — the
  viewport never depends on selection, preserving the documented
  comparability principle. Selecting a band renders a band (blank above and
  below). — *Rejected:* silhouette implicitly renders all layers with
  `layer_expansions` records (silently ignoring `layers` for one kind is the
  silent-drop pattern the tool bans); requiring an all-layers selection
  (makes band inspection impossible).

- **D4. Sub-pixel layers render faithfully — no minimum-row inflation, no
  rejection.** At `resolution_scale: 1` (1024 px) a 0.2 mm layer on a
  ~200 mm-tall model is <1 px and can straddle pixel centers; an individual
  thin band (e.g. a 2-layer interface) can rasterize to zero rows. The
  silhouette body stays gap-free because adjacent slabs tile; only sub-pixel
  bands can drop out. `docs/19_visual_debug.md` gains guidance: for
  interface-band inspection on tall models, raise `resolution_scale` (the Z
  frame is model-wide, so selecting a band does not zoom — scale is the
  lever). — *Rejected:* min-1px inflation (the image lies about Z geometry;
  inflated adjacent rows repaint each other unpredictably); failing closed on
  sub-pixel slabs (makes scale 1 unusable on tall models even for gross
  taper shape).

- **D5. `options.view`: `"front"` = X–Z plane (project along Y), `"side"` =
  Y–Z plane (project along X). Default `"front"`; unknown values fail closed
  with a named error.** The manifest entry records the resolved view, so a
  bundle is never ambiguous. — *Rejected:* required-no-default (inconsistent
  with `options.base`'s defaulting precedent); render-both-by-default (one
  visualization spec producing two images breaks the request-to-entry mapping
  consumers assume).

### 4.2 Bundle and schema contract

- **D6. No view mixing in a bundle.** A request containing a `silhouette`
  visualization may contain **only** silhouette visualizations; mixing with
  `filled_areas`/`filament_lines`/`diagnostic_overlay` fails closed at
  validation with a named error. This preserves the pinned `world_bounds_mm`
  byte-identity invariant (fact 6) untouched for 1.0/1.1 consumers — inside
  a silhouette bundle every entry still shares one byte-identical (X–Z or
  Y–Z) bounds value. Two bundles side-by-side cover the top+side workflow. —
  *Rejected:* per-entry `projection` field with the invariant re-scoped per
  plane (rewrites a pinned contract and every consumer assertion about it);
  a separate `silhouette_bounds_mm` field with `world_bounds_mm` null on
  rendered entries (contradicts the field's own doc comment).

- **D7. Schema 1.2.0 manifest shape.**
  - `ImageEntry.layer_index` (today a non-`Option` `i64`) becomes `Option`
    with `skip_serializing_if`; `layer_z` is **already** `Option<f64>`
    (corrected 2026-08-27 — the grill session assumed both fields change)
    and gains only `skip_serializing_if`. Both are absent on silhouette
    entries and **byte-unchanged on 1.0/1.1 bundles** (the manifest
    already mirrors the request's declared `schema_version`, so old
    requests keep producing byte-compatible output — this must be pinned
    by serialization tests, not just parsing tests).
  - New `layers_rendered` field on silhouette entries: the resolved layer
    indices the composite actually drew (list or list-of-ranges — packet
    chooses the encoding; it must round-trip losslessly).
  - New per-entry field recording the resolved `view` (`"front"`/`"side"`).
  - **No `typed_capture` on silhouette entries** — embedding every layer's
    IR would balloon the manifest by the layer count; the repo bans loading
    large JSON blobs (`docs/21` fixture rules; CLAUDE.md >1MB rule).
  - Silhouette under declared schema 1.0.0/1.1.0 rejects with a **named
    requires-1.2.0 error** (the `OptionRequiresSchema11` pattern — the
    message names the fix), *not* the generic `UnknownVisualizationKind`.
    Note the current kind check in `validate_request`
    (`crates/pnp-cli/src/visual_debug.rs`) is schema-independent; it must
    become schema-aware for this kind.
  — *Rejected:* sentinel `layer_index: -1` (a silent lie, and it collides
  with real negative raft indices); keeping `typed_capture` as an array
  (estimated tens-of-MB manifests on tall models — ~per-layer IR size ×
  layer count, unmeasured).

### 4.3 Tap coverage (model source)

- **D8. Silhouette-capable tap whitelist — the Z-attributable set:**

  | Tap | Slab source | Interval source / color classes |
  |---|---|---|
  | `Layer::Slice` | per-region `SlicedRegion.effective_layer_height` | `regions[].polygons` (+ `infill_areas` if the packet decides to distinguish them — default: polygons only as one body class) |
  | `PrePass::PaintSegmentation` | same | variant region polygons |
  | `Layer::PaintRegionAnnotation` | same | post-edit region polygons |
  | `Layer::SlicePostProcess` | same | post-edit region polygons |
  | `PrePass::RegionMapping` | same (joined `SliceIR` rows) | joined polygons, colored by the existing deterministic `config_tint` |
  | `PrePass::OverhangAnnotation` | the same layer's SliceIR region heights | `overhang_quartile_polygons[layer]` bands |
  | `PrePass::SupportGeometry` | model-layer slabs | **`SupportPlanIR` roles only** (D9) — role regions per `SupportPlanRole`, per-role colors (interface bands vs body distinct) |
  | `PostPass::LayerFinalization` | schedule z-diffs (consecutive finalized layers' z; first layer from 0) | typed `ordered_entities[].path` — segment projections inflated by `Point3WithWidth.width / 2`; roles or tools |
  | `PostPass::GCodeEmit` | schedule slabs via Z-containment bucketing (D11) | E-inversion widths (D11); roles or tools |

  Slab-source note (D1 alignment, added 2026-08-27 review): the
  `PostPass::LayerFinalization` and `PostPass::GCodeEmit` rows use schedule
  z-diffs — the source D1 rejects for SliceIR-family taps — because
  finalized layers carry no per-region `effective_layer_height`; the
  schedule diff is the only height those captures can honestly attest.
  D1's rejection targets taps whose IR *does* carry per-region heights.

  Rejected taps fail closed with a named `SilhouetteUnsupportedForTap`
  error: `PrePass::MeshAnalysis` and `PrePass::SeamPlanning` (no Z
  attribution — fact 5; the error text should say so), and every arena tap
  (`Layer::Perimeters` … `Layer::PathOptimization`) — deferred because a
  side view forces whole-print per-layer execution (fact 14). PostPass taps
  carry no such objection: they already drive the whole print by nature
  (fact 8/15). — *Rejected:* the original 2-tap set (Slice + support only —
  user chose breadth); "any blackboard tap" literally (impossible — fact 5);
  MeshAnalysis via facet re-derivation (pulls mesh access into the renderer,
  a new dependency direction).

- **D9. Support tap renders `SupportPlanIR` roles only.**
  - Coarse `SupportGeometryIR.entries` (own support-layer grid — fact 4) are
    **skipped with a manifest warning** when non-empty; they stay
    inspectable via the existing top-down view. — *Rejected:* deriving the
    support grid's Z from `support_layer_height_mm` (a Z formula this packet
    would assert without verifying against the support emitter —
    confidently-wrong-image risk); conditional rendering only when grids
    align (untestable/unexplainable conditional).
  - Negative `global_layer_index` (raft) plan entries are **skipped with a
    manifest warning naming the dropped index range**, plus a deviation-log
    row tracking proper raft rendering as a follow-up. — *Rejected:* failing
    closed on any negative entry (unusable on raft models — exactly where
    support inspection matters); deriving raft slabs (same
    confidently-wrong risk).

- **D10. Postpass capture shape: silhouette consumes a single whole-print
  capture.** One `StageCapture` carrying the whole-print IR once; the
  composite renderer iterates layers internally (which it must anyway).
  Existing per-layer rows stay untouched for top-down consumers, so no
  contract change for 1.0/1.1. Fact 8 is the motivation. — *Rejected:*
  `Arc`-ing the payload inside `CapturedIr` (changes the serialized
  `typed_capture` shape and every existing match arm); accepting the clones
  (OOM-shaped failure on large prints).

- **D11. `PostPass::GCodeEmit` renders via E-inversion** (user chose
  inclusion over the recommended LayerFinalization-only):
  - E-delta recovery (fact 9, corrected): typed `Move.e` is a cumulative
    position, so the renderer walks the command stream carrying the last
    seen `Some(e)` value and recovers `Δe` by differencing against it.
    Moves with `e: None` are travel (no width, no interval; they do not
    reset the carried position). `Δe < 0` (inline wipe-tower purge
    retracts) is non-extruding — skipped like zero-length degenerate
    moves, never drawn.
  - Width per move: `w = Δe × filament_area / (L × h)` with
    `filament_diameter` from the model source's **resolved config** (not
    config-block parsing — the model source has the real config) and `h`
    from the containing slab.
  - Layer bucketing (fact 10): each **extruding** move buckets into the
    schedule slab `[z − h, z]` containing its current Z. Out-of-slab
    extrusion (e.g. nonplanar) draws at the **nearest slab with a manifest
    warning naming the affected Z values** — material is never silently
    dropped from the profile, and the warning tells the reader precisely
    where the image is approximate. — *Rejected:* failing closed on
    out-of-slab Z (unusable on nonplanar prints); drawing at true Z without
    bucketing (breaks the per-(layer, class) interval-union model for this
    one tap).
  - Known cost, accepted by the user: this is a second implementation of the
    E-inversion math over a different type (`GCodeCommand` stream vs parsed
    text), testable mainly against itself; its unique value is the
    pre-`GCodePostProcess` state.
  — *Alternative recommended but not chosen:* LayerFinalization-only with
  GCodeEmit rejected toward the standalone gcode source.

### 4.4 G-code source (standalone `.gcode` silhouette)

- **D12-slabs. Z slabs = `[previous ;Z: marker, z]`** per parsed layer
  (first layer `[0, z]`). Inherently exact under **variable/adaptive layer
  heights** — each height is its own marker delta (this was an explicit user
  requirement). Non-monotonic or duplicate markers → manifest warning naming
  the layers, never a guess. `;Z:` markers are layer markers, not raw moves,
  so z-hops don't pollute the schedule. — *Rejected:* uniform
  `layer_height` from the config block (breaks on adaptive heights and
  first-layer height); rejecting irregular spacing (adaptive prints are
  legitimate and interesting).

- **D13-width. Flow-derived width, silhouette-only.** Per extruding move:
  `w = Δe × A_filament / (L × h)`, `A_filament` from the file's own
  `filament_diameter` config-block comment, `h` from the slab, `Δe` the
  per-move delta as resolved by the parser's absolute/relative E-mode
  handling (fact 13; in absolute mode the parser diffs raw E values —
  unlike D11, which diffs typed `Move.e` positions itself). This **contradicts** the archived
  spec's pinned rule for gcode `filled_areas` ("must not infer a physical
  bead width from E values") — resolved by scoping: `filled_areas` keeps
  `gcode_line_width_mm` and its rule **unchanged** (its tests stay green
  untouched); `docs/19_visual_debug.md` documents why silhouette differs (a
  computation from the artifact's own data, not a guess). — *Rejected:*
  also converting `filled_areas` (amends a documented fail-closed contract,
  touches its validation path and tests — scope growth); keeping
  `gcode_line_width_mm` for silhouette (flattens Arachne-variable widths to
  one constant; the user explicitly asked for flow derivation).

- **D14-fallback. Underivable width policy:** if the request supplied
  `gcode_line_width_mm`, underivable moves use it (explicit and user-stated,
  not a guess); with no fallback supplied, the bundle **fails closed** with a
  named error stating exactly which datum is missing (e.g. no
  `filament_diameter` comment; volumetric M200 extrusion). Zero-length
  E-only moves are skipped as degenerate, like every existing renderer's
  <2-point paths. — *Rejected:* always-fail (third-party gcode without the
  comment becomes un-silhouettable even when the user knows the width);
  skip-with-warning (holes in the profile that don't exist in the print —
  the misleading-image failure mode).

- **D15-roles. Unclassified `;TYPE:` moves form their own interval class**
  in the existing unclassified warning color, **first** in paint order —
  never merged into a neighboring role (the role-guessing the spec
  prohibits), never dropped (holes lie). — consistent with the final
  renderer's documented unclassified handling.

- **D16-model. Cross-section model: invert our emitter's rectangular
  formula** (fact 9) everywhere flow derivation is used (both the gcode
  source and `PostPass::GCodeEmit`). Exact for PnP files (the primary
  debugging target); underestimates on stadium-model (Orca) files by roughly
  `h(1 − π/4)` — sub-pixel at these raster sizes, but the silhouette must
  not be cited as a width-measurement tool (docs caveat). — *Rejected:*
  stadium model (systematically over-wide for our own emitter — the wrong
  bias for a parity repo); generator sniffing (fragile, and a silent
  behavioral fork between near-identical files).

- **Deposited-width caveat (docs line):** the inversion recovers the
  *deposited* width, so low-`flow_factor` moves (ironing ≈ 0.1) render
  proportionally thin. Physically correct; a reader comparing against
  authored line width should be told.

### 4.5 Multicolor

- **D17. `color_by: "tool"` on silhouette is legal only on tool-carrying
  captures**: `PostPass::LayerFinalization`, `PostPass::GCodeEmit`, and the
  gcode source (palette-only there — no config to resolve `filament_colour`
  from, matching existing behavior). Intervals union per (layer, tool);
  paint order ascending tool index; manifest `tool_palette` emitted exactly
  as today; `tool_color_source` validation rules inherited unchanged
  (`"filament"` resolves from the model source's raw config via the existing
  `filament_tool_colors`). Blackboard taps reject with `ToolColorUnavailable`
  — silhouette gets no looser rule than the top-down renderer's pinned
  contract. The occlusion caveat (D2) applies per tool exactly as per role.
  — *Rejected:* deriving tool from per-region config on blackboard taps
  (an inference the pinned contract refuses); deferring multicolor (user
  chose inclusion).

### 4.6 Seam glyphs

- **D18. Seams are model-source only, `SeamPlanIR`-sourced** (fact 11), each
  seam projected at (x or y per `view`, z) as the existing seam glyph
  (filled circle, red — legend 1.1.0). Two forms, both included:
  - `overlays: ["seams"]` keeps its **exact 1.1.0 meaning**: an isolated
    image — silhouette base painted uniformly `FAINT_BASE` gray + seam
    glyphs — with every rendered seam mirrored into the entry's
    `overlay_events`, **now including `z`** (an additive field on the seam
    event shape; verify serialization compatibility).
  - New 1.2.0 option **`composited_overlays: ["seams"]`**: additionally
    draws the glyphs directly onto the colored silhouette base image.
    Validation: legal only on a silhouette visualization with a model
    source; `"seams"` is the only legal member for now; unknown members,
    wrong kind, or gcode source fail closed with named errors.
  — *Rejected request shapes:* per-overlay mode objects
  `[{kind, mode}]` (changes the type of an existing 1.1.0 field);
  a `composite_seams` boolean (hardcodes seams as the only compositable
  kind). *Rejected rendering forms:* composited-only (glyphs over
  multi-colored bands lose legibility — the clutter problem 1.1.0 isolated
  overlays were built to escape).
  - Travel / retraction / z-hop / tool-change glyphs on silhouette remain
    **excluded** (not requested; each needs its own Z story).

### 4.7 Ordering and sequencing

- **D19. Delivery order — generated as a multi-packet batch** (resolved by
  the 2026-08-27 review; supersedes the earlier "may re-batch" hedge).
  **Packet 1 = steps 1+2** (the tracer plus the motivating support use
  case, carrying the 1.2.0 schema gate, mixing ban, and manifest shape);
  steps 3–6 follow as dependent packets in order (steps 4 and 5 may merge
  into one packet — both are postpass/D10 consumers). Step 5 (GCodeEmit
  E-inversion) stays last-or-late so it can be dropped without stranding
  anything — it is the weakest-value/highest-risk member ("testable mainly
  against itself"). The steps:
  1. `Layer::Slice` tracer — proves projection math, interval union,
     composite image path, model-wide Z framing, 1.2.0 manifest shape, the
     mixing ban, and the schema gate. Simplest slab source.
  2. Support-plan tap — the motivating use case: role colors, raft and
     coarse-entry warnings, deviation row.
  3. G-code source — `;Z:` slabs, E-inversion + fallback, unclassified
     class, variable-height support.
  4. `PostPass::LayerFinalization` + multicolor — typed widths, tool
     classes, the single whole-print capture shape (D10).
  5. `PostPass::GCodeEmit` — E-inversion against `GCodeIR`, Z-containment
     bucketing, out-of-slab warnings.
  6. Seam overlays — isolated + `composited_overlays` forms, `z` in
     `overlay_events`.
  — *Rejected:* support-plan first (riskier tracer); everything in one step
  (unreviewably large first unit); one packet covering all six steps
  (blows the generator's implementation-plan size discipline for the same
  reason).

## 5. Request And Manifest Shapes (normative sketch)

Request (model source, silhouette + composited seams):

```json
{
  "schema_version": "1.2.0",
  "source": {"kind": "model", "model": "part.stl", "config": "profile.json",
             "module_dirs": ["modules/core-modules"]},
  "layers": [{"start": 0, "end": 400}],
  "taps": ["PrePass::SupportGeometry", "PostPass::LayerFinalization"],
  "visualizations": [
    {"type": "silhouette",
     "options": {"view": "side",
                 "overlays": ["seams"],
                 "composited_overlays": ["seams"]}}
  ],
  "resolution_scale": 2
}
```

Request (gcode source):

```json
{
  "schema_version": "1.2.0",
  "source": {"kind": "gcode", "path": "reported.gcode"},
  "layers": [{"start": 0, "end": 999}],
  "taps": [],
  "visualizations": [
    {"type": "silhouette",
     "options": {"view": "front", "color_by": "tool"}}
  ],
  "gcode_line_width_mm": 0.42
}
```

(`gcode_line_width_mm` here is the *optional fallback* per D14 — the
silhouette derives widths from flow; the request value is used only for
underivable moves. Its absence plus an underivable move fails the bundle.)

Silhouette manifest entry (shape, not exhaustive):

```json
{
  "source": "model",
  "tap": "PrePass::SupportGeometry",
  "visualization": "silhouette",
  "view": "side",
  "layers_rendered": [{"start": 0, "end": 400}],
  "png_path": "images/PrePass__SupportGeometry_silhouette_side.png",
  "viewport": {"width": 2048, "height": 2048},
  "world_bounds_mm": {"min_x": -2.0, "min_y": -2.0, "max_x": 62.0, "max_y": 82.0},
  "legend_version": "…",
  "warnings": ["support plan: 3 raft entries (indices -3..-1) not rendered; raft side view is a tracked follow-up"]
}
```

Note `world_bounds_mm` reuses the existing field/type — inside a silhouette
bundle its `min_y`/`max_y` carry Z millimeters. This is legal only because of
the mixing ban (D6): a bundle is either all-XY or all-one-silhouette-plane,
and the per-entry `view` field says which. **Reuse confirmed by the
2026-08-27 fresh-session review:** `ImageEntry.world_bounds_mm` is already
`Option<ViewportBoundsMm>`, the mixing ban plus the per-entry `view` field
make the plane mechanically unambiguous, and a per-plane rename would
rewrite a pinned invariant for no mechanical gain.

Filename scheme for composite images: no `_l{layer}` suffix (they are not
per-layer); suggested `"{sanitized_tap}_silhouette_{view}.png"`, with
`_overlay_{kind}` inserted for isolated-overlay variants and `_tool` for
tool-colored variants, mirroring the existing scheme's disambiguation rules
(two visualizations must never collide on one filename — the repo fixed this
class of bug once already, see `diagnostic_overlay_base_suffix`).

## 6. Fail-Closed Matrix

Every rejection is a named error variant with a pinning test. Axes are
finite: schema(3) × source(2) × frame(2) × kind(4) × view(2+invalid) ×
color options × overlay options × tap class.

| # | Condition | Rejection |
|---|---|---|
| R1 | `silhouette` under declared schema 1.0.0/1.1.0 | named requires-1.2.0 error (message names the fix) |
| R2 | tap outside D8's table (arena taps, MeshAnalysis, SeamPlanning) | `SilhouetteUnsupportedForTap` (message states why — execution cost vs no Z attribution) |
| R3 | silhouette mixed with any non-silhouette visualization in one request | named mixing error |
| R4 | `frame: "plate"` with silhouette | named error (no machine-height concept; bed is XY-only) |
| R5 | unknown `options.view` value | named error |
| R6 | `color_by: "tool"` on a capture with no tool assignment | `ToolColorUnavailable` (existing variant) |
| R7 | `tool_color_source` without `color_by: "tool"`, unknown values | existing 1.1.0 rules, extended to silhouette |
| R8 | gcode silhouette width underivable and no `gcode_line_width_mm` fallback | named error stating the missing datum |
| R9 | `composited_overlays` with non-`"seams"` member, on a non-silhouette kind, or on a gcode source | named errors |
| R10 | `overlays: ["seams"]` on a gcode source | existing `OverlayUnsupportedOnGcode` |
| R11 | any layer selector resolving to no scheduled layer | existing `LayerSelectorResolvesToNoLayer` (unchanged) |

Warnings (manifest channel — visible, never silent; each with a pinning
test):

| # | Condition | Warning content |
|---|---|---|
| W1 | negative `global_layer_index` support-plan entries present | dropped index range + follow-up note |
| W2 | non-empty coarse `SupportGeometryIR.entries` on a support silhouette | count + "own support-layer grid; not renderable on model-layer slabs" |
| W3 | non-monotonic / duplicate `;Z:` markers (gcode source) | offending layer indices/Z values |
| W4 | out-of-slab extruding Z (`PostPass::GCodeEmit`) | affected Z values + nearest-slab placement note |

## 7. Determinism ACs

- Same (captures, view, scale, viewport) → byte-identical PNG (existing AC-5
  extended to the composite path).
- Rectangle emission order is fully specified: ascending layer index, then
  class paint order (D2/D17), then ascending interval start. Interval union
  is order-independent by construction (sorted endpoint sweep).
- All map-keyed IR iterated through sorted key order (the existing renderer's
  pattern — `surface_classification_shapes` sorts objects,
  `support_geometry_shapes` sorts entries; the silhouette path must do the
  same for any `HashMap` source).
- The composite entry's `layers_rendered`, warnings list order, and filename
  are all deterministic functions of the request + model.

## 8. Explicit Exclusions (v1)

- Arena taps (`Layer::Perimeters` … `Layer::PathOptimization`) — forces
  whole-print per-layer execution; revisit only with an explicit
  cost-accepting request shape.
- `PrePass::MeshAnalysis`, `PrePass::SeamPlanning` as silhouette *taps* (no
  Z attribution; seams still surface via D18's overlay on other taps).
- Raft slab derivation (W1 covers visibility; deviation row tracks the
  follow-up).
- Coarse `SupportGeometryIR.entries` rendering (W2).
- `frame: "plate"` (R4).
- View mixing (R3).
- Depth cues of any kind: occlusion resolution, perspective/isometric,
  arbitrary view angles, back/mirror views, cross-section (clipped) views.
- Sub-pixel band inflation (D4).
- Flow-derived width on `filled_areas` (D13 keeps the old rule there).
- Travel / retraction / z-hop / tool-change glyphs on silhouette.
- `typed_capture` on silhouette entries (D7).
- Pixel/perceptual bundle diffing (out of scope for the whole tool,
  unchanged from the archived spec).

## 9. Scope (concrete units — no line estimate)

The prior session's "~700–1,400 lines" figure was never reviewed and is
withdrawn; impact in lines is unmeasured.

Production surface:

- `crates/slicer-runtime/src/visual_debug_render.rs` — silhouette shape
  builder (interval projection + union + slab rectangles per capture class),
  a composite render entry point taking a capture *group*, seam-glyph
  projection, X–Z/Y–Z viewport computation helpers.
- `crates/pnp-cli/src/visual_debug.rs` — 1.2.0 schema gate + the full
  validation matrix (§6), per-bundle composite assembly path (groups
  captures by (tap, view)), manifest field changes (D7), silhouette
  filenames, `composited_overlays` parsing.
- `crates/pnp-cli/src/visual_debug_gcode.rs` — gcode silhouette path:
  per-layer slab derivation from `;Z:`, per-move E-delta/flow inversion,
  unclassified class, warnings W3.
- `crates/slicer-runtime/src/layer_executor.rs` /
  `crates/slicer-runtime/src/postpass.rs` surface — the single whole-print
  capture shape for silhouette postpass consumption (D10), without touching
  the existing per-layer rows.
- `slicer_runtime` re-exports for any new public types.

Docs:

- `docs/19_visual_debug.md` — silhouette section: request shape, view
  semantics, scale guidance (D4), width-rule distinction (D13), deposited-
  width caveat, paint-order occlusion caveat (D2), warnings inventory.
- Deviation-log row: raft side-view follow-up (W1).
- This plan supersedes nothing; the archived spec remains the v1 contract
  record.

Test surface (each row at least one test):

- Every R1–R11 rejection and W1–W4 warning.
- Slab math against a catch-up-layer fixture (multi-object differing layer
  heights; assert a catch-up region's rectangle bottom is
  `catchup_z_bottom`, not the previous global z).
- E-inversion round-trip: G-code produced by `DefaultGCodeEmitter` from a
  known-width path back-computes that width exactly via
  consecutive-`Some(e)` position differencing; a stream containing
  `e: None` travel moves and a negative-delta inline purge retract
  recovers the same widths (travel carries no interval and does not reset
  the carried position; the retract is skipped).
- Variable-layer-height gcode slabs (adaptive `;Z:` spacing).
- Determinism: byte-identical PNG on repeat render; stable manifest field
  order.
- Model-wide Z framing: layers-subset bundle and all-layers bundle produce
  identical `world_bounds_mm` and identical projection of a shared feature.
- 1.0/1.1 manifest **byte-compatibility** (serialization output, not just
  parsing): an existing 1.0.0 and 1.1.0 request's manifest is
  byte-identical before/after this change.
- Interval-union correctness: holes don't split intervals; disjoint islands
  produce disjoint intervals; role overlap paints in D2 order; touching
  intervals merge.
- Postpass single-capture shape: silhouette request over
  `PostPass::LayerFinalization` produces one capture (assert no per-layer
  whole-print clones); the existing top-down postpass path is unchanged.
- Seam overlay: isolated and composited forms; `overlay_events` carries `z`.
- Filename uniqueness across all silhouette variants in one bundle.

## 10. Residual Risks / Reviewer Checklist

1. **`world_bounds_mm` field reuse (§5 note) — RESOLVED: confirmed
   (2026-08-27 review).** Inside a silhouette bundle `min_y`/`max_y` mean
   Z. Confirmed because the field is already `Option<ViewportBoundsMm>`,
   the mixing ban (D6) plus the per-entry `view` field make the plane
   mechanically unambiguous, and a per-plane rename would rewrite a pinned
   invariant for no mechanical gain.
2. **Occlusion caveat placement (D2).** Verify the packet puts it where a
   PNG-reading agent will see it, not only in docs.
3. **1.0/1.1 byte-compatibility (D7).** Verify the test list covers
   serialization output, not just deserialization acceptance.
4. **GCodeEmit nearest-slab bucketing (D11)** on genuinely nonplanar prints
   draws approximated Z — confirm W4's text is specific enough to keep the
   image trustworthy.
5. **Single whole-print postpass capture (D10)** must not regress the
   existing top-down postpass path — both shapes need coverage.
6. **E-inversion on foreign G-code (D16)** underestimates width by
   ~`h(1 − π/4)` — confirm the docs caveat forecloses citing the silhouette
   as a width-measurement tool.
7. **`Move.e` semantics (fact 9) — RESOLVED (2026-08-27 review).**
   Re-verified against `DefaultGCodeEmitter`: the typed `Move.e` is the
   accumulated position (`Some(e_position)` on non-zero deltas, `None`
   otherwise), not a delta as the grill session read it, and negative
   deltas (inline purge retracts) flow through it. Fact 9 and D11 are
   rewritten to consecutive-position differencing with explicit
   negative-delta handling; the §9 round-trip test pins it.
8. **`overlay_events` seam shape gaining `z`** — confirm it is additive for
   existing consumers of the 1.1.0 event JSON.
9. **Whether `Layer::Slice` silhouette should distinguish `polygons` vs
   `infill_areas`** as classes (D8 table note) — a small open choice
   deliberately left to the packet.
10. **Packet shape — RESOLVED (2026-08-27 review): multi-packet batch**
    per D19 — Packet 1 = §4.7 steps 1+2; steps 3–6 as dependent packets.
    Batch approval 2026-08-27 chose **steps 4 and 5 as separate packets**
    (preserving GCodeEmit's independent droppability). A single packet
    covering all six steps would blow the generator's implementation-plan
    size discipline and contradict D19's own rejection of "everything in
    one step".

## Packet Queue

Approved 2026-08-27 (5 packets, steps 4/5 separate; new TASK rows
re-derived at authoring time). Numbering from 247 — verified free against
`origin/master` (`docs/spec_packets/` tops out at 246 there; 243–246 exist
only on master, so task-map TASK-ID derivation must also check
`origin/master`'s packet task-maps).

| # | packet slug | goal (one sentence) | task ids | depends on | status | packet dir |
|---|-------------|---------------------|----------|------------|--------|------------|
| 1 | 247-visual-debug-silhouette-core | Silhouette visualization kind: schema 1.2.0 gate, mixing ban (R3), composite per-(tap,view) render path, model-wide Z framing, 1.2.0 manifest shape (D7), `Layer::Slice` tracer + `PrePass::SupportGeometry` support-plan tap with W1/W2 warnings (§4.7 steps 1+2). | TASK-442..445 | - | generated | docs/spec_packets/247-visual-debug-silhouette-core |
| 2 | 248-visual-debug-silhouette-gcode-source | Standalone `.gcode` source silhouette: `;Z:` slab derivation (D12), flow-derived widths + `gcode_line_width_mm` fallback (D13/D14/D16), unclassified class (D15), W3, R8/R10, gcode half of D17 (palette-only tool coloring — deviation recorded in its requirements.md) (step 3). | TASK-446..448 | #1 | generated | docs/spec_packets/248-visual-debug-silhouette-gcode-source |
| 3 | 249-visual-debug-silhouette-postpass-multicolor | `PostPass::LayerFinalization` silhouette via the D10 single whole-print capture shape, typed `Point3WithWidth` widths, `color_by: "tool"` classes (D17, model-source half; gcode half lives in #2) (step 4). | TASK-449..451 | #1 | generated | docs/spec_packets/249-visual-debug-silhouette-postpass-multicolor |
| 4 | 250-visual-debug-silhouette-gcode-emit | `PostPass::GCodeEmit` silhouette via `Move.e` position-differencing E-inversion (fact 9/D11), Z-containment bucketing, W4; includes the grounding-surfaced `run_postpass_taps` resolved-config fidelity fix (step 5). | TASK-452..454 | #1 #2 #3 | generated | docs/spec_packets/250-visual-debug-silhouette-gcode-emit |
| 5 | 251-visual-debug-silhouette-seam-overlays | Seam glyphs on silhouette: isolated `overlays` form + new `composited_overlays` option, `z` added to seam `overlay_events`, R9 (D18, step 6). | TASK-455..457 | #1 | generated | docs/spec_packets/251-visual-debug-silhouette-seam-overlays |
| 6 | 252-visual-debug-silhouette-remaining-taps | Close the D8 whitelist: `PrePass::RegionMapping` (joined SliceIR rows, `config_tint` classes) and `PrePass::OverhangAnnotation` (quartile bands; requires sourcing per-region slab heights the capture lacks today) silhouettes, retiring 247's interim rejections for both taps (row approved 2026-08-27 after grounding surfaced the coverage gap). | TASK-458..461 | #1 | generated | docs/spec_packets/252-visual-debug-silhouette-remaining-taps |

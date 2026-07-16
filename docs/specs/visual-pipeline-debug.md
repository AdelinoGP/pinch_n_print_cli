# Visual Pipeline Debug Infrastructure

**Status:** Proposed

## Problem

Agents investigating a reported print defect must currently infer geometry from
G-code text or create ad-hoc diff scripts. Images are substantially easier to
compare: a sequence of equivalent, per-stage layer renders reveals the first
stage at which a perimeter, fill area, support, or travel defect appears.

The slicer needs a first-class, deterministic visual-debug bundle that can be
requested without modifying print behavior. It must support both typed,
intermediate IR views and a final view reconstructed from serialized G-code.

## Scope

In scope:

- `pnp_cli visual-debug`, a dedicated command that creates a visual-debug
  bundle from either a model-backed pipeline run or an existing G-code file.
- Post-stage visual taps for every scheduler stage.
- PNG renders for geometry-bearing stages and stage-specific synthetic PNG
  diagrams for stages with no directly renderable 2-D geometry.
- Versioned JSON request and bundle-manifest contracts.
- An agent-facing visual-debug skill documented alongside, but independent of,
  `.claude/skills/debug-pipeline/SKILL.md`.

Out of scope:

- Pixel or perceptual comparison between two bundles.
- Changing module manifests, WIT contracts, or IR schemas solely to make a
  tap possible.
- An HTML gallery or an extension of the HTML slicer report.
- Full OrcaSlicer G-code-preview parity. Standalone parsing supports the
  documented Pinch 'n Print emitted subset; unsupported commands are reported
  in the bundle manifest.

## Success Criteria

1. `pnp_cli visual-debug` accepts exactly one source mode: model-backed or
   standalone G-code.
2. A valid request produces a manifest and every requested PNG, or the command
   fails. A partial bundle is never reported as successful.
3. Model-backed requests execute only the scheduler dependency closure needed
   for selected taps. They do not emit G-code unless a final G-code view is
   selected.
4. Every selected image in a bundle uses the same model-wide XY viewport,
   fixed semantic palette, legend version, and requested raster scale.
5. The default raster is 1024 x 1024. `resolution_scale` is limited to `1`,
   `2`, or `3`; its pixel cost is respectively 1x, 4x, or 9x the default.
6. When visual debugging is not invoked, ordinary `pnp_cli slice` has no
   visual-debug capture, allocation, serialization, rendering, or process
   overhead.
7. A typed intermediate renderer fails to compile when a consumed IR field is
   renamed or has an incompatible type change; tap contract tests pin every
   documented source field and bundle-manifest version.
8. The final renderer preserves unclassified extrusion rather than guessing a
   role, and records unsupported G-code constructs as warnings.

## Command And Request Contract

The command is intentionally separate from `pnp_cli slice`:

```text
pnp_cli visual-debug --request request.json --output bundle-dir
pnp_cli visual-debug --request request.json --output bundle-dir --overwrite
```

`--output` is a bundle directory, not a G-code output. A non-empty directory
is rejected unless `--overwrite` is present. Directory or PNG write failure is
fatal because the command's only product is trustworthy visual evidence.

The request is a versioned JSON document. It is not the slicer's existing
print-config JSON and it never changes slice geometry. All request keys use
snake_case.

```json
{
  "schema_version": "1.0.0",
  "source": {
    "kind": "model",
    "model": "resources/benchy.stl",
    "config": "profile.json",
    "module_dirs": ["modules/core-modules"]
  },
  "layers": [0, { "start": 12, "end": 15 }],
  "taps": ["Layer::Perimeters", "Layer::Infill", "final_gcode"],
  "visualizations": ["filament_lines", "filled_areas", "diagnostic_overlay"],
  "resolution_scale": 1
}
```

Standalone mode replaces `source` with `{ "kind": "gcode", "path":
"reported.gcode" }`. It is mutually exclusive with model mode. A standalone
`filled_areas` request must additionally supply `gcode_line_width_mm`; the
renderer must not infer a physical bead width from E values.

### Dependency Closure

For model mode, a requested tap means the post-stage, post-host-hook state.
The executor runs all prerequisite stages and enabled modules needed to reach
the furthest selected tap, then stops. A request may in principle require
additional layers or whole-print work for correctness — such as consuming
overhang classification or layer finalization — beyond what the request
selected; the bundle manifest would record any such expansion and its real
reason. For the `Layer::*` per-layer taps this executor supports today
(`Layer::Perimeters` through `Layer::PathOptimization`), no such dependency
exists: Tier 2 per-layer work runs independently per layer with no shared
mutable state (`docs/01_system_architecture.md` "Tier 2 — Per-Layer"), so a
layer the request did not select is never executed at all — not merely
un-rendered. Expansion is reserved for a future tap that does have a genuine
correctness dependency; it is not the default behavior.

This follows the scheduler's fixed stage order and four-phase execution in
`docs/04_host_scheduler.md`; taps do not create scheduler edges, module
invocations, or module-visible access.

## Bundle Contract

A bundle contains `manifest.json` and PNGs. `manifest.json` is the sole
machine-readable index. Each image entry records source mode, requested tap,
layer index and Z where applicable, visualization type, PNG path, viewport,
legend version, IR schema version or G-code parser version, and warnings.

Two viewport properties are distinct and both recorded: `viewport` is the pixel
raster (width/height), while each rendered entry's `world_bounds_mm` is the
world-space (mm) extent it was projected through — byte-identical across every
rendered entry in a bundle, on **both** source modes. The manifest's `frame`
records which framing mode produced them.

### Typed Post-Stage Capture (packet 158)

Before the intermediate renderer exists, a model-backed request with a
non-empty `taps` list runs request-gated, typed post-stage capture at the
executor boundary instead of producing PNGs. This is a strict subset of the
render-path contract above: it reuses the same dependency-closure execution
and manifest shape, but each `images[]` entry's `png_path` is empty and its
`typed_capture` field carries the renderer-owned payload directly — a tagged
`{"kind": ..., "value": ...}` JSON object mirroring `CapturedIr`
(`crates/slicer-runtime/src/layer_executor.rs`): `kind` is one of
`"Perimeter"`, `"Infill"`, `"Support"`, or `"LayerCollection"`, and `value` is
that stage's committed IR (`PerimeterIR`, `InfillIR`, `SupportIR`, or
`LayerCollectionIR`), taken as an owned clone immediately after the stage's
`apply` commits — never a borrow into `LayerArena` (ADR-0037). `typed_capture`
is `null`/absent for placeholder and standalone-G-code entries. Selected-layer
retention means only `(tap, layer)` pairs the request actually selected
produce an `images[]` entry. For every tap this capture path supports, the
closure does not merely skip rendering a non-selected layer — it does not
execute that layer at all (no arena, no module invocation, no `apply` call):
those `Layer::*` stages have no cross-layer correctness dependency, per
"Dependency Closure" above.

The manifest additionally records, only for a typed-tap capture (empty for
every other request shape, including the standalone G-code path):

- `executed_stage_ids`: the truncated per-layer stage closure that actually
  ran, in fixed `STAGE_ORDER` order — every prerequisite stage through and
  including the furthest selected tap, and nothing after it.
- `executed_layer_indices`: the global layer indices the closure actually ran
  that stage closure for. Equal to the request's selected, plan-applicable
  layers today, since no supported tap has a cross-layer dependency.
- `layer_expansions`: reserved for a layer the closure had to execute (but not
  render) for a genuine correctness dependency even though it was not
  selected — each entry would carry `layer_index` and a specific,
  non-generic `reason`. Empty for every request today; a selected layer
  never appears here.

Supported taps for this capture path are exactly `SUPPORTED_TAP_STAGE_IDS`
(`crates/slicer-runtime/src/layer_executor.rs`): the `Layer::*` per-layer
stages in the "Stage Tap Inventory" table below, from `Layer::Perimeters`
through `Layer::PathOptimization`. An unsupported tap name, or a request
whose selected layers do not resolve to a real layer in the model, is
rejected before the model or modules are loaded — never a partial bundle.
This capture path produces no PNGs; rendering is a later packet
("Intermediate renderer" in Candidate Packets below).

All images use one model-wide XY extent plus a documented fixed margin. The
viewport is calculated in the canonical coordinate system: `Point2` values
remain scaled integers until projection; conversions use `units_to_mm()` and
the canonical scale of 10,000 units/mm. Any newly constructed geometry uses
`Point2::from_mm` or `mm_to_units`; rendering must never assume one unit is
one nanometer. Layer Z remains millimeters.

The margin is a fixed **absolute millimeter** distance (`VIEWPORT_MARGIN_MM`),
applied equally to both axes. A margin expressed as a fraction of each axis'
own extent is itself anisotropic and skews a non-square viewport before
projection begins.

"Model-wide" is a property of the model, not of the request: the extent is the
loaded mesh's XY bounding box (`MeshIR::build_volume`, already computed by
`load_model`), unioned with the selected captures' geometry so brim, skirt, and
support — all of which extrude beyond the model's silhouette — are never
clipped. It therefore does not vary with the layers or taps a request selected,
which is what makes two bundles over one model comparable. Bounding the
selected captures alone would reframe on every request.

Projection is **aspect-preserving**: `Projector`
(`slicer-runtime/src/visual_debug_render.rs`) scales by a single uniform
`min(width_ratio, height_ratio)` and centers the result, letterboxing the
unused axis. It is the sole owner of the world→pixel transform for **both**
render paths below — the typed-IR renderer and the standalone-G-code renderer
must never define their own. They each did originally, and drifted: the G-code
path scaled uniformly while the intermediate path normalized each axis
independently against the always-square raster, stretching every non-square
model. Tests must project through `Projector` rather than restate its
arithmetic; a test that copies the transform cannot detect the transform being
wrong.

`frame` (request, optional, default `"model"`) selects what the viewport frames
to: `"model"` for the model-wide extent above, or `"plate"` for the bed's
extent. `"plate"` frames the bed exactly — never widened to the geometry, or it
would stop denoting the plate as soon as a part sat near an edge.

Both sources support `"plate"`, each reading the only bed definition it has.
The model source resolves the `bed_shape` config key. The standalone-G-code
source resolves no printer profile, but the artifact carries the slicer's own
config block, and its `printable_area` comment is the bed polygon (OrcaSlicer
emits `; printable_area = 0x0,220x0,220x200,0x200` — `,`-separated points whose
X and Y are joined by a literal `x`). Because that is only knowable after the
file is parsed, a G-code request with no usable `printable_area` fails at
render time rather than in request validation. Neither source ever falls back
to model framing when the bed is unavailable: returning a different image than
the one requested is worse than returning none.

The palette and legend are fixed by the v1 bundle contract: outer/inner/thin
walls share the perimeter family, infill roles share the infill family,
travels are visually distinct, support and support interface are distinct,
and unclassified final extrusion is a separate warning color. The exact RGBA
values and legend version belong in the implementation packet, not in an
ad-hoc request option.

### Visualization Types

- `filament_lines`: path centerlines colored by semantic role.
- `filled_areas`: direct `ExPolygon` areas where available; for typed paths,
  the swept extrusion-width shape from `Point3WithWidth.width`; for standalone
  G-code, the request's `gcode_line_width_mm`.
- `diagnostic_overlay`: stable, labeled stage-specific details such as seams,
  travel anchors, region/object identifiers, layer bounds, or execution
  annotations. It is composable with either geometry view.

## Render Paths

### Intermediate IR Path

The runtime owns all taps. Blackboard IR is immutable during per-layer work;
per-layer IR lives in `LayerArena` and is released after its
`LayerCollectionIR` is committed. The tap adapter therefore reads a typed,
post-commit borrow at the executor boundary and produces renderer-owned data
only when a request selected that stage and layer. It never exposes a new
module, WIT, or Blackboard API.

The renderer is Rust code colocated with the runtime/CLI visual-debug path,
not an external Python or shell script. It may add the `png` crate using its
pure-Rust compression path, justified because the workspace has no existing
PNG encoder and release builds must not discover Python or ImageMagick at
runtime. The dependency packet must record enabled features and license review.

Typed adapters and tests are the schema-drift mechanism: adapters consume real
IR structs, while contract tests prove the source field, stage timing, legend,
and manifest entry for every tap. A schema change therefore requires an
intentional renderer update rather than silently dropping visual evidence.

### Final G-code Path

The final path parses the serialized text after `PostPass::TextPostProcess`,
not merely `GCodeIR`, so it reflects the artifact given to a printer. It
supports the documented Pinch 'n Print emitted `G0`/`G1` subset, extrusion mode
markers, `;LAYER_CHANGE`, `;Z:`, and `;TYPE:` role boundaries. Moves with no
recognized role render as `unclassified` and add a manifest warning.

Raw macros and commands outside that subset are not approximated. They produce
warnings with source line numbers. OrcaSlicer's `GCodeReader`,
`GCodeProcessor`, and `libvgcode` are useful future references for richer
motion support, but v1 does not claim behavioral parity. Any translated
OrcaSlicer source must carry the attribution header required by
`docs/ORCASLICER_ATTRIBUTION.md`.

## Stage Tap Inventory

The table names the exact documented source fields. A diagram source means a
stage-specific PNG of trace-relevant fields, not a fabricated model geometry.

**Documentation drift (resolved):** `docs/01_system_architecture.md` names
`SupportGeometryIR`; `docs/02_ir_schemas.md` now carries its normative
definition alongside the other prepass IRs: `support_layer_height_mm`,
`support_top_z_distance_mm`, and
`entries: HashMap<SupportGeometryKey, Vec<ExPolygon>>`, where
`SupportGeometryKey` has `global_support_layer_index`, `object_id`, and
`region_id`. This spec's tap inventory below matches that normative shape.

**LayerPlanning is not a standalone tap.** `LayerPlanIR.global_layers[].
{index,z,active_regions,has_nonplanar,is_sync_layer}` and
`object_participation` carry planning state (selected layer Z, active
regions, synchronization/catch-up), but that state has no independently
renderable geometry of its own. It is surfaced only as a `diagnostic_overlay`
annotation composed onto a geometry-bearing tap (e.g. `Layer::Slice` or
`Layer::Perimeters`) — never as its own `taps[]` entry or table row.

| Tap | Source fields | Render output |
|---|---|---|
| `PrePass::MeshAnalysis` | `SurfaceClassificationIR.per_object`; `ObjectSurfaceData.bridge_regions[].xy_footprint`; `overhang_regions[].xy_footprint`; `overhang_quartile_polygons` | Classified-footprint areas and overhang/bridge overlays. |
| `PrePass::SeamPlanning` | `SeamPlanIR.entries[].{region_key, chosen_candidate.point, chosen_candidate.wall_index, scored_candidates[].reason}`; `chosen_candidate.point` is a `Point3WithWidth` — `x`/`y`/`z`/`width` are f32 **millimeters**, not 100-nm scaled units | Seam-plan overlay/diagram. |
| `PrePass::SupportGeometry` | `SupportGeometryIR.{support_layer_height_mm,support_top_z_distance_mm,entries}`; `SupportGeometryKey.{global_support_layer_index,object_id,region_id}`; `SupportPlanIR.entries[].{global_layer_index,object_id,region_id,branch_segments}` — `branch_segments` (`Vec<ExtrusionPath3D>`) carry `Point3WithWidth` points in f32 **millimeters**, not 100-nm scaled units | Coarse support `entries` polygon areas, planned support branch lines, and support-layer settings overlay. |
| `PrePass::PaintSegmentation` | `SliceIR.{global_layer_index,z,regions}`; `SlicedRegion.{polygons,variant_chain,segment_annotations}` | Variant polygon areas and paint/segment overlays. |
| `PrePass::RegionMapping` | A `SliceIR` join, not a synthetic diagram: `RegionMapIR.entries` (keyed by `RegionKey.{global_layer_index,object_id,region_id,variant_chain}`) joined to `SliceIR.regions` by that same `RegionKey`; each joined `RegionPlan.{config,stage_modules,paint_overrides}` (`config` is a `ConfigId` — interned index into `RegionMapIR.configs`, resolved via `RegionMapIR::config_for`) | Dispatch/configuration overlay on the joined `SliceIR.regions[].polygons`; no synthetic geometry. |
| `Layer::Slice` | `SliceIR.regions[].{polygons,infill_areas}` | Slice polygon and available-infill-area render. |
| `PrePass::OverhangAnnotation` | `SurfaceClassificationIR.overhang_quartile_polygons` | Quartile-band area render. |
| `Layer::PaintRegionAnnotation` and `Layer::SlicePostProcess` | `SliceIR.regions[].{polygons,segment_annotations}` | Post-edit polygon render and annotation overlay. |
| `Layer::Perimeters` | `PerimeterIR.regions[].{walls,infill_areas,seam_candidates,resolved_seam}`; synchronized `SliceIR` `bridge_areas`, `bottom_solid_fill`, `top_solid_fill`, `sparse_infill_area` | Wall paths, seam candidates, and canonical fill-area render. |
| `Layer::PerimetersPostProcess` | `PerimeterIR.regions[].{walls,resolved_seam}` | Finalized wall paths and seam overlay. |
| `Layer::Infill` and `Layer::InfillPostProcess` | `InfillIR.regions[].{sparse_infill,solid_infill,ironing}` | Infill/ironing path lines or width sweeps. |
| `Layer::Support` and `Layer::SupportPostProcess` | `SupportIR.{support_paths,interface_paths,raft_paths,ironing_paths}` | Support-family path lines or width sweeps. |
| `Layer::PathOptimization` | `LayerCollectionIR.{ordered_entities,travel_moves,tool_changes,z_hops,annotations}` | Ordered extrusion and travel render with anchor/order overlay. |
| `PostPass::LayerFinalization` | Finalized `Vec<LayerCollectionIR>` and each collection's `ordered_entities`, `travel_moves`, `tool_changes`, `z_hops`, `annotations` | Post-finalization path/travel render, including inserted synthetic layers. |
| `PostPass::GCodeEmit` and `PostPass::GCodePostProcess` | `GCodeIR.commands`; `GCodeCommand::Move.{x,y,z,e,f,role}` | Structured-command diagram/render while the IR remains available. |
| `PostPass::TextPostProcess` / `final_gcode` | Serialized final G-code fields `G0`/`G1` X/Y/Z/E/F, `;LAYER_CHANGE`, `;Z:`, `;TYPE:` | Final parser render. This is the final-stage source of truth. |

## Agent Documentation

`docs/19_visual_debug.md` documents request authoring, bundle inspection,
warnings, and resolution-cost guidance. The future visual-debug skill is an
independent diagnostic option paired with `debug-pipeline`: use visual-debug
when a geometry defect must be localized; use debug-pipeline for timing, DAG,
and manifest diagnosis. Neither skill requires the other.

## Candidate Packets

| Packet | Scope | Depends on |
|---|---|---|
| Visual-debug request and bundle contract | Add command parsing, request validation, bundle lifecycle, overwrite behavior, and manifest model; no taps yet. | ADR-0039 |
| Typed tap capture | Add request-gated post-stage capture adapters and minimal dependency-closure execution. | Visual-debug request and bundle contract; ADR-0037 |
| Intermediate renderer | Render typed geometry, swept widths, overlays, synthetic diagrams, shared viewport, palette, and PNG output. | Typed tap capture; `png` dependency decision |
| Final G-code renderer | Implement PnP-subset G-code parsing and final PNGs, including unclassified/unsupported warnings. | Visual-debug request and bundle contract |
| Agent surface and verification | Add visual-debug skill, guide examples, tap/manifest contract tests, determinism and no-overhead measurements. | Intermediate renderer; Final G-code renderer; ADR-0038 |

The request/bundle contract precedes both render paths. Typed capture precedes
the intermediate renderer. The agent surface closes only after both render
paths establish stable artifacts.

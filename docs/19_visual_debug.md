# Visual Pipeline Debugging

`pnp_cli visual-debug` produces a visual-debug bundle: deterministic PNGs and
a `manifest.json` index for selected pipeline stages and layers. It is intended
for geometry-defect investigation, not timing or module-DAG analysis.

The complete design contract is `docs/specs/_OLD/visual-pipeline-debug.md`
(archived superseded spec, retained for the design write-up).

## When To Use It

Use visual debugging when a report says that a perimeter, infill region,
support, travel, or final toolpath looks wrong and the question is where that
shape first changes. Use `debug-pipeline` instead for slow slices, DAG edges,
claims, and manifest validation. The two tools are independent.

## Request Shape

The command consumes a versioned JSON render request and writes a directory:

```text
pnp_cli visual-debug --request visual-debug.json --output target/visual-debug
```

The request selects source mode, layers, post-stage taps, visualization types,
and `resolution_scale`. Source modes are mutually exclusive:

- Model mode runs only the pipeline dependency closure required by the taps.
- G-code mode parses an existing final G-code artifact.

`layers` is a list of selectors resolved against the schedule (model mode:
`LayerPlanIR.global_layers`; G-code mode: parsed `;Z:` markers). Each element
is one of:

- an integer index — `0`, `12`;
- an inclusive `{ "start": S, "end": E }` range — e.g.
  `"layers": [0, { "start": 12, "end": 15 }]`; the range object rejects
  unknown fields rather than silently parsing as an empty detail;
- a z-only detail selector that resolves to the layer at a printed Z (exact
  shape in `docs/specs/_OLD/visual-pipeline-debug.md` and the validator).

Layers are anonymous — there is no name selector. Selection **fails closed**:
an unknown visualization kind, a legacy composited `diagnostic_overlay` on a
G-code source, a name selector, or a selector that resolves to no real layer
is rejected before any render or bundle write. No requested visualization or
layer is ever silently dropped from a successful bundle.

## Schema 1.1.0 — Tool Colors And Isolated Overlays

`schema_version: "1.1.0"` adds per-visualization options. A `"1.0.0"` request
keeps its exact prior behavior; the new options under `"1.0.0"` are rejected
(`OptionRequiresSchema11`), never silently ignored.

**Tool coloring** — on `filled_areas` / `filament_lines`:

```json
{"type": "filament_lines",
 "options": {"color_by": "tool", "tool_color_source": "palette"}}
```

- `color_by`: `"role"` (default, the fixed semantic legend) or `"tool"` —
  geometry is colored by the entity's resolved tool index
  (`PrintEntity.tool_index` on typed captures; tracked `T<n>` on a G-code
  source). Rejected (`ToolColorUnavailable`) on taps whose IR carries no tool
  assignment — only `Layer::PathOptimization`-family (LayerCollection),
  `PostPass::LayerFinalization`, and `PostPass::GCodeEmit` captures qualify.
- `tool_color_source`: `"palette"` (default — a fixed high-contrast 8-color
  per-index palette, deliberately NOT real filament colors) or `"filament"`
  (the config `filament_colour` hex list; unresolvable entries fall back to
  the palette; a standalone G-code source always resolves to the palette).
  The manifest's `tool_palette` table records the exact RGB per tool.

**Isolated overlays** — on `diagnostic_overlay`:

```json
{"type": "diagnostic_overlay",
 "options": {"overlays": ["travel", "seams", "retractions", "z_hops", "tool_changes"]}}
```

Each named overlay renders as its **own image**: the base geometry painted
uniformly faint gray, with only that event class's glyphs on top — never a
composited clutter of all overlays. Every rendered event is also mirrored
verbatim into that image's manifest entry as `overlay_events` (positions,
lengths, heights, tool indices, travel polylines + total length in mm), so an
agent can reason numerically from the manifest and use the PNG only as
confirmation.

Glyphs are distinguished by **shape**, not color alone (legend `1.1.0`):

| Event        | Glyph                                                    |
|--------------|----------------------------------------------------------|
| seam         | filled circle (red)                                      |
| retraction   | down-triangle (magenta)                                  |
| unretraction | up-triangle (green)                                      |
| z-hop        | diamond (purple)                                         |
| tool change  | filled square (near-black)                               |
| travel       | dotted polyline (blue), open-circle origin, filled-dot destination |

Overlay availability is tap-dependent and fails closed
(`OverlayUnsupportedForTap`) when the tap's IR has no source field for the
event class (a present-but-empty field renders a valid zero-event image):
LayerCollection/LayerFinalization taps support travel/retractions/z_hops/
tool_changes; `Layer::Perimeters` and `PrePass::SeamPlanning` support seams;
`PostPass::GCodeEmit` supports travel/retractions/tool_changes. The G-code
source supports every overlay except `seams` (final G-code carries no seam
marker); its retract/unretract detection covers inline-E moves and firmware
`G10`/`G11`, z-hops are Z-only lifts above the layer's base Z, and tool
changes come from `T<n>` lines.

Wipe visualization is deliberately absent: no per-move wipe geometry exists
in the captured IR yet. Modifier-volume visualization is likewise deferred
(`ModifierVolume` is not captured by any tap; modifier influence is visible
indirectly via RegionMapping's config tint).

The default resolution is 1024 x 1024. `resolution_scale: 2` uses four times
as many pixels; `resolution_scale: 3` uses nine times as many. Select the
smallest scale that makes the suspected feature visible to avoid unnecessary
image context cost.

## Silhouette Side Views (schema 1.2.0)

`schema_version: "1.2.0"` adds the `silhouette` visualization: a **side-on
projection** of the print, rendered in a vertical plane rather than looking down
at one layer. Every other view in this document answers "what does layer N look
like"; a silhouette answers "how do these layers stack, and where do the
interface bands sit". Read it as a projection, not a section.

```json
{"schema_version": "1.2.0",
 "visualizations": [
   {"type": "silhouette", "tap": "Layer::Slice", "options": {"view": "front"}}
 ]}
```

`options.view` selects the projection plane:

| `view`     | Plane | Horizontal axis |
|------------|-------|-----------------|
| `"front"`  | X–Z   | X (default when `view` is omitted) |
| `"side"`   | Y–Z   | Y |

An unknown `view` value is rejected, and `view` on a non-silhouette
visualization is rejected. An explicit `view` key under a declared `"1.0.0"` or
`"1.1.0"` schema is **hard-rejected naming `"1.2.0"`** — it is never silently
tolerated, and the `silhouette` kind itself is likewise rejected naming
`"1.2.0"` under those schemas.

**One plane per bundle.** Silhouette never mixes with `filled_areas`,
`filament_lines`, or `diagnostic_overlay` in one request, and every silhouette
spec in a single request must resolve to the **same** `view`. This is what lets
the whole bundle share one byte-identical `world_bounds_mm`; render front and
side as two separate bundles. `frame: "plate"` plus silhouette is also rejected
— a plate frame is an XY footprint and has no Z meaning.

### Supported And Rejected Taps

Supported in this release: `Layer::Slice`, `PrePass::PaintSegmentation`,
`Layer::PaintRegionAnnotation`, `Layer::SlicePostProcess` (all `CapturedIr::Slice`),
`PrePass::SupportGeometry`, and `PostPass::LayerFinalization`. Every other tap
is rejected with a named reason rather than rendered empty — including
`Layer::Perimeters` and the other per-layer arena taps, `PrePass::MeshAnalysis`,
`PrePass::SeamPlanning`, `PrePass::RegionMapping`, and
`PrePass::OverhangAnnotation`. `PostPass::GCodeEmit` remains rejected; packet
250 owns G-code-emit silhouettes.

One further rejection is **interim**, expected to lift in a later release:
`options.composited_overlays`. `color_by: "tool"` is supported; see "Tool-Colored
Silhouettes" below. Silhouette on a standalone **G-code source** is also
supported — see "Standalone G-code Silhouettes" below.

### Postpass Silhouettes And The Single Whole-Print Capture

`PostPass::LayerFinalization` is a supported silhouette tap. A silhouette bundle
uses a **single whole-print capture**: one `StageCapture` carries the finalized
`Vec<LayerCollectionIR>` once. It never stores one whole-print clone per layer,
which would scale to an out-of-memory failure on large models. The per-layer
capture rows read by top-down consumers are unchanged and byte-stable.

Finalized-layer slabs are schedule-Z-diff slabs: each layer occupies
`[previous finalized layer z, own z]`, with the first finalized layer starting at
`0`. These are the only heights a finalized layer can honestly attest to because
`LayerCollectionIR` carries no per-region `effective_layer_height`. For a
postpass silhouette entry, `layers_rendered` is the resolved selection intersected
with finalized layer indices, compressed into maximal inclusive ranges.

`PostPass::GCodeEmit` is likewise a whole-print postpass tap, but renders the
**pre-rewrite** emit IR: the typed `GCodeIR` before `PostPass::GCodePostProcess`
rewrites it — the tap's unique value. A defect visible here yet absent from the
final `.gcode` localizes to the postprocess modules. Widths are recovered from
consecutive accumulated `Move.e` positions: Δe by differencing; an `e: None`
travel carries the carried position but contributes no interval; and a negative
delta — an inline purge retract — is non-extruding, contributes no segment,
while the carried position still updates through it. Each move is bucketed by
**Z-containment** (containment first: `z_bottom < z ≤ z_top`): a move whose Z is
outside every schedule slab draws at the nearest slab with a W4 warning naming
the affected Z, whereas a move contained in an unselected slab draws nothing —
that is selection, not loss. This E-inversion is **testable mainly against itself**: it is a second inversion, deliberately separate from the standalone
G-code source's parser-based one, and is anchored externally by the emitter
round-trip test. Images are
`PostPass__GCodeEmit_silhouette_{view}[_tool].png`.

### Manifest Shape

A silhouette entry carries `"visualization": "silhouette"`, `"view"`,
`"layers_rendered"`, and `"world_bounds_mm"`, and **omits `"layer_index"` and
`"layer_z"` entirely** — a silhouette spans layers, so there is no single layer
to name.

```json
{"visualization": "silhouette",
 "view": "front",
 "layers_rendered": [{"start": 0, "end": 11}, {"start": 20, "end": 24}],
 "world_bounds_mm": {"min_x": -2.0, "max_x": 62.0, "min_y": -2.0, "max_y": 41.5},
 "png_path": "images/Layer__Slice_silhouette_front.png"}
```

`layers_rendered` is a list of **inclusive** `{"start", "end"}` ranges: the
maximal runs of consecutive rendered layer indices, ascending, non-overlapping,
and lossless — every rendered layer appears in exactly one run.

`world_bounds_mm` reuses the top-down bounds object, but **inside a silhouette
bundle its `min_y`/`max_y` carry Z millimetres**, while `min_x`/`max_x` carry X
(`view: "front"`) or Y (`view: "side"`). Do not read a silhouette's `min_y` as a
Y coordinate.

`legend_version` for a `1.2.0` bundle is still `"1.1.0"`: silhouettes add fill
classes, not glyphs. `1.0.0` and `1.1.0` manifests are byte-unchanged by this
schema, and a G-code layer with no `;Z:` marker still serializes
`"layer_z": null`.

Entry ordering is deterministic but mildly surprising: `Layer::Slice` is not a
`STAGE_ORDER` member (the scheduler stage is `PrePass::Slice`), so a
`Layer::Slice` entry sorts **after** `Layer::SlicePostProcess`. That is observed,
correct ordering — not an unsorted manifest.

### Filenames

One image per `(tap, view)` group, written as
`images/{sanitized_tap}_silhouette_{view}.png` — e.g.
`images/Layer__Slice_silhouette_front.png`. Duplicate specs collapse into a
single group, and no two entries in a bundle ever share a `png_path`. A
 tool-colored silhouette adds the `_tool` suffix after the view suffix, for
 example `images/PostPass__LayerFinalization_silhouette_front_tool.png`.

### Framing And Scale

The Z frame is **model-wide**, exactly as the XY viewport is: it is taken from
`MeshIR::build_volume`, unioned with the captured geometry so nothing is
clipped, then margined. Selecting a layer subset therefore does **not** zoom the
image — two bundles over the same model record byte-identical `world_bounds_mm`
regardless of which layers each selected, which is what makes them directly
comparable.

Because selection cannot zoom, `resolution_scale` is the only lever for detail:
**for interface-band inspection on tall models, raise** `resolution_scale`
rather than narrowing the layer selection, which will not change the framing at
all. Select the smallest scale that makes the band visible.

### Paint Order And The Occlusion Caveat

Fill classes paint in one fixed order, back to front:

1. `SliceRegion` (body) / `Support` (body)
2. `SupportRaft`
3. `SupportBaseInterface`
4. `SupportBottomInterface`
5. `SupportInterface` (last, always on top)

**A silhouette is a projection, not a section — overlapping structures
occlude.** Where a later class overlaps an earlier one in the projected
interval, the later class's colour wins and the earlier class's extent is
hidden. When overlap actually occurs, the manifest entry carries a per-entry
occlusion warning naming the affected layer count; when no overlap occurs there
is no warning at all, so this caveat is your only notice that a hidden extent is
possible. Never conclude a body region is missing from a silhouette alone.

For a tool-colored silhouette, this caveat applies **per tool**: a later tool's
fill occludes an earlier tool's entire slab area wherever their projected areas
overlap. Paint order is ascending tool index, so an earlier tool's hidden extent
cannot be recovered from the image alone.

Vertical extents come from per-region slabs, never a uniform one. For the
Slice-family taps each region's slab is `[z − effective_layer_height, z]`
**per region**, so a catch-up region's slab bottom correctly reaches below its
neighbours'. For `PrePass::SupportGeometry` the slabs come from the layer
schedule. Holes never split a projected interval (the contour is what is
projected), and touching intervals merge into one run.

### Warnings Inventory

Nothing is ever inferred or inflated: sub-pixel bands are **not** inflated to a
minimum pixel width, and every omission is either a named warning or a named
rejection.

- **W1 — raft.** `SupportPlanIR` entries with a negative `global_layer_index`
  are skipped; the warning names the count and the dropped index range. Raft
  prefix layers have no slab in the layer schedule and so are not drawn. Tracked
  as an open deviation in `docs/DEVIATION_LOG.md`.
- **W2 — coarse support geometry.** Non-empty coarse `SupportGeometryIR.entries`
  are skipped; the warning names the count. Emit-schedule entries span multiple
  model layers (the `u32::MAX` sentinel denotes intermediate layers) and cannot
  be honestly drawn on single-layer slabs — inspect them via the top-down view
  instead.
- **W3 — unusable layer Z (G-code source only).** A layer whose `;Z:` marker
  duplicates or decreases relative to the previous accepted marker, or which
  carries no `;Z:` marker at all, is skipped entirely; the warning names the
  offending layer index and the Z values involved (or the marker's absence).
  See "Standalone G-code Silhouettes" below.
- **W4 — out-of-slab Z on an emit silhouette.** A `PostPass::GCodeEmit` move
  whose Z is outside every schedule slab is drawn at the nearest slab; the
  warning reads
  `gcode emit: extruding move at z=... outside every schedule slab; drawn at
  nearest slab ...` and names the affected Z. Containment is checked **first**:
  a move contained in an unselected slab draws nothing and emits no warning —
  selection, not loss.
- **Occlusion.** Per-entry, as described above, only when overlap occurred.

### Tool-Colored Silhouettes

`options.color_by: "tool"` is legal on silhouettes. The R7 `tool_color_source`
rules are unchanged: `tool_color_source` still requires `color_by: "tool"`, and
its values remain `"palette"` or `"filament"`. A capture with no tool assignment
(for example, a blackboard tap such as `Layer::Slice`) fails closed with
`ToolColorUnavailable`, and the error names the tap. Tool fills paint in
ascending tool-index order, the manifest emits a `tool_palette` table, and the
filename uses the `_tool` suffix described above.

The occlusion caveat is per tool: in an overlap, a later tool's fill occludes an
earlier tool's entire slab area. A tool-colored silhouette is therefore a
projection of tool-painted slabs, not a section that can reveal every tool's
full extent.

### Standalone G-code Silhouettes

A silhouette also renders from a standalone `.gcode` source, with no model and
no pipeline stages behind it. The projection, view options, paint order, and
occlusion caveat are exactly as above; what differs is where the slabs and the
segment widths come from, and both are derived from the file's own contents.

**Slabs come from `;Z:` markers only.** A layer's slab is
`[previous accepted ;Z: marker, this layer's z]`, and the **first** accepted
marker yields `[0, z]` — the first slab's bottom is always 0, never a
marker-delta guess. No layer-height config comment, no interpolation, and no
neighbour guess ever produces a slab.

**Unusable Z fails closed (W3).** A layer whose marker duplicates or decreases
against the previous accepted marker, or which has no `;Z:` marker at all, is
**skipped**: it contributes no pixels, it is excluded from `layers_rendered`,
and it does **not** advance the carried marker. A W3 warning names the layer
index and the Z values (or the marker's absence). A guessed slab is the
misleading-image failure mode, so the layer is dropped instead.

**Widths are flow-derived, per move.** Each extruding move's width comes from
inverting our emitter's rectangular extrusion model:

```
w = Δe × A_filament / (L × h)
```

`A_filament = π × (d/2)²`, with `d` read from the file's own
`; filament_diameter = …` config comment; `h` is the layer's slab height; `Δe`
is the per-move E delta, honouring `M82` absolute / `M83` relative modes and
`G92 E` resets. This is deliberately unlike `filled_areas`, which never derives
width from E and keeps `gcode_line_width_mm` **mandatory**: a side view's whole
value is showing real per-move deposition, and a single uniform width would
flatten exactly the signal you opened the view to see.

**`gcode_line_width_mm` is a fallback, not an override (D14).** When supplied in
the request it is used **only** for moves whose width is underivable; it is
never preferred over a derivable width. A move is underivable when the file
carries no usable `filament_diameter` comment, or when the move sits at or after
an `M200` line.

**Underivable and rendered, with no fallback, is an error (R8).** Such a move
fails the command with an error naming the missing datum and the
`gcode_line_width_mm` remedy, and no bundle content is written. Evaluation is
**lazy and per rendered move, in parse order**, so a layer selection that avoids
the poisoned moves still succeeds and partial inspection of a damaged file stays
possible. A later selection over the same file can therefore fail where an
earlier one passed; that is deliberate, not a flake.

**`M200` is a poison marker, not a supported mode.** From its source line
onward, flow derivation is refused rather than approximated — inverting the
linear-E model over volumetric E values silently produces wrong widths.

**The width shown is deposited width**, reconstructed from the file's own E
values: a move printed with a low `flow_factor` renders genuinely thin. **Do not
cite the silhouette as a width-measurement tool** — it shows what the G-code
asks the printer to deposit, not a measured extrusion. The `PostPass::GCodeEmit`
width recovery is the same inversion over the emit `Move.e` stream (see the
GCodeEmit paragraph above), not a separate model.

**Foreign files carry a systematic bias.** The inversion assumes our emitter's
rectangular cross-section. Foreign slicers commonly model a stadium
(rounded-end) cross-section, so widths reconstructed from a foreign `.gcode` are
still derived from that file's own data, but are not a cross-slicer-comparable
measurement. The `PostPass::GCodeEmit` E-differencing inversion carries the same
rectangular-model bias.

**Multi-tool diameters.** `; filament_diameter = …` may carry a comma-separated
per-extruder list; a segment uses the entry at its tool index, **clamped to the
last entry** when the tool index exceeds the list. A malformed comment — any
unusable entry — is rejected wholesale rather than parsed partially, because the
list is extruder-indexed and a hole would misattribute diameters.

**Tool colouring is palette-only.** `options.color_by: "tool"` is supported here
and always resolves to the fixed default palette; a standalone `.gcode` resolves
no printer or filament config, so the entry records
`"tool_color_source": "palette"`.

**Bundle shape.** Images are `images/gcode_silhouette_{view}.png` and
`images/gcode_silhouette_{view}_tool.png`. The manifest entry carries
`"source": "gcode"`, `"tap": ""` (a standalone G-code source has no pipeline
taps, and naming any tap on a G-code silhouette is rejected),
`"visualization": "silhouette"`, `"view"`, `"layers_rendered"` as inclusive
`{start, end}` ranges, `"gcode_parser_version"`, and `"world_bounds_mm"`; it has
no `"layer_index"` / `"layer_z"` keys. Framing is whole-file and
**selection-independent**: a layer-subset request and an all-layers request over
the same file record identical `world_bounds_mm`.

## Framing

Every render is **aspect-preserving**: one uniform scale is applied to both
axes, and the geometry is centered, so the unused axis becomes an even
letterbox band. A square in millimeters always renders square in pixels. Since
the raster is square by default, a wide model (a Benchy footprint is roughly
2:1) fills the width and leaves blank bands above and below — that is correct
output, not a cropping bug.

The viewport is **model-wide**, not selection-wide: it is the model's own XY
extent, unioned with the captured geometry so brim, skirt, and support are
never clipped, plus a fixed 2 mm margin on all four sides. It does **not**
depend on which layers or taps a request selected, so two bundles over one
model are directly comparable — requesting layer 3 and requesting layers 0-50
frame identically. Both source modes use the same transform: a model rendered
from a pipeline tap and the same model rendered from its final G-code line up.

`frame` selects what the viewport is framed to. It is optional and defaults to
`"model"`:

| `frame`   | Viewport                                                     |
|-----------|--------------------------------------------------------------|
| `"model"` | The model's XY extent (default). Fills the raster with the part. |
| `"plate"` | The whole bed. Shows placement; a small part renders small. |

`frame: "plate"` frames the bed **exactly** — it is never widened to the
geometry, or it would stop meaning "the plate" as soon as a part sat near an
edge. Both sources support it, reading the bed from whichever definition that
source has:

- **model**: the resolved `bed_shape` config key.
- **gcode**: the `printable_area` comment in the G-code's own config block
  (e.g. OrcaSlicer emits `; printable_area = 0x0,220x0,220x200,0x200`).

A `.gcode` with no `printable_area` has no bed to frame to, so
`frame: "plate"` against it is rejected rather than silently falling back to
model framing.

## Reading A Bundle

Read `manifest.json` before inspecting PNGs. It records each PNG's layer, tap,
view type, shared viewport, source schema/parser version, and warnings. The
manifest's `frame` records what the bundle was framed to; each rendered entry's
`world_bounds_mm` records the shared world-space (mm) viewport it was projected
through — identical across every entry in the bundle, on both source modes.

All images in a bundle share one model-wide XY viewport and a fixed semantic
legend. This makes a missing wall or shifted infill region comparable between
stages. `filament_lines` shows centerlines; `filled_areas` shows polygons or
extrusion-width sweeps; `diagnostic_overlay` adds stage-specific labels.

### Support-family visual inspection (packet 221)

For the tree family, `tree-support-planner` is the visual-debug tap capturing
the structural `SupportPlanIR` (support body / interface roles with family
attribution) before `tree-support` renders it. Inspect the family's plan tap
alongside the final `Layer::Support` output to compare planned vs. emitted
geometry for a given region.

Packet 334 adds cross-family routing diagnostics: overlapping demands from
different families are reported with their family, body, demand, and rejection
reason, and the host's routing-cell ownership (same-family union vs.
cross-family positive-area overlap rejection) is structured into these
diagnostics.

### Tap Classes And Execution Closure

`visual-debug` supports the full "Stage Tap Inventory" of
`docs/specs/_OLD/visual-pipeline-debug.md`, not only the per-layer stages. The taps
fall into three capture classes with distinct execution closures; the
manifest's `executed_stage_ids` and `executed_layer_indices` record exactly
what ran for the selected taps:

- **Blackboard-read prepass taps** — `PrePass::MeshAnalysis`,
  `PrePass::SeamPlanning`, `PrePass::SupportGeometry`,
  `PrePass::PaintSegmentation`, `PrePass::RegionMapping`,
  `PrePass::OverhangAnnotation`, `Layer::Slice`, and
  `Layer::PaintRegionAnnotation`/`Layer::SlicePostProcess` read a committed,
  whole-print Blackboard slot after the prepass. They run the prepass only,
  with no per-layer arena execution.
- **Per-layer arena taps** — `Layer::Perimeters` through
  `Layer::PathOptimization` — run the truncated per-layer stage closure over
  exactly the selected layers. These `Layer::*` stages have no cross-layer
  correctness dependency, so a non-selected layer is never executed at all,
  not merely un-rendered.
- **PostPass whole-print taps** — `PostPass::LayerFinalization` and
  `PostPass::GCodeEmit` — need the whole print (all layers → finalization →
  post-pass) before their IR exists, so the manifest records whole-print
  `executed_stage_ids`/`executed_layer_indices` even when only a subset of
  layers is rendered. They are the only documented deviation from
  minimal-closure execution.

`layer_expansions` is reserved for a layer the closure had to execute for a
genuine cross-layer correctness dependency even though it was not requested;
each entry names the `layer_index` and a specific, real `reason`. It is empty
for every request today.

Standalone G-code `filled_areas` views require `gcode_line_width_mm` in the
request. Unknown extrusion roles render as `unclassified`; unsupported commands
become warnings rather than guessed geometry.

The command fails closed rather than producing a partial bundle: a rejected
tap or selector aborts before the model or modules load, no `manifest.json` or
PNG is written, and a pre-existing bundle is never mutated. It also rejects a
non-empty output directory unless `--overwrite` is supplied.

## Related Tools

- `docs/17_agent_debugging.md` and `.agents/skills/debug-pipeline/SKILL.md`:
  timing, DAG, and manifest diagnosis.
- `docs/16_slicer_report.md`: opt-in HTML timing and allocator report; it is
  not a geometry-rendering facility.
- `docs/08_coordinate_system.md`: canonical XY and Z coordinate conventions.

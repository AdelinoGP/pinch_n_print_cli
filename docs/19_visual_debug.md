# Visual Pipeline Debugging

`pnp_cli visual-debug` produces a visual-debug bundle: deterministic PNGs and
a `manifest.json` index for selected pipeline stages and layers. It is intended
for geometry-defect investigation, not timing or module-DAG analysis.

The complete design contract is `docs/specs/visual-pipeline-debug.md`.

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
  shape in `docs/specs/visual-pipeline-debug.md` and the validator).

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

### Tap Classes And Execution Closure

`visual-debug` supports the full "Stage Tap Inventory" of
`docs/specs/visual-pipeline-debug.md`, not only the per-layer stages. The taps
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

- `docs/17_agent_debugging.md` and `.claude/skills/debug-pipeline/SKILL.md`:
  timing, DAG, and manifest diagnosis.
- `docs/16_slicer_report.md`: opt-in HTML timing and allocator report; it is
  not a geometry-rendering facility.
- `docs/08_coordinate_system.md`: canonical XY and Z coordinate conventions.

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

The default resolution is 1024 x 1024. `resolution_scale: 2` uses four times
as many pixels; `resolution_scale: 3` uses nine times as many. Select the
smallest scale that makes the suspected feature visible to avoid unnecessary
image context cost.

## Reading A Bundle

Read `manifest.json` before inspecting PNGs. It records each PNG's layer, tap,
view type, shared viewport, source schema/parser version, and warnings.

All images in a bundle share one model-wide XY viewport and a fixed semantic
legend. This makes a missing wall or shifted infill region comparable between
stages. `filament_lines` shows centerlines; `filled_areas` shows polygons or
extrusion-width sweeps; `diagnostic_overlay` adds stage-specific labels.

Standalone G-code filled-area views require `gcode_line_width_mm` in the
request. Unknown extrusion roles render as `unclassified`; unsupported commands
are warnings rather than guessed geometry.

The command fails rather than producing a partial bundle. It also rejects a
non-empty output directory unless `--overwrite` is supplied.

## Related Tools

- `docs/17_agent_debugging.md` and `.claude/skills/debug-pipeline/SKILL.md`:
  timing, DAG, and manifest diagnosis.
- `docs/16_slicer_report.md`: opt-in HTML timing and allocator report; it is
  not a geometry-rendering facility.
- `docs/08_coordinate_system.md`: canonical XY and Z coordinate conventions.

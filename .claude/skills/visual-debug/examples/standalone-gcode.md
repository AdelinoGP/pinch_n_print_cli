# Worked example — standalone G-code visual debug

Scenario: someone hands you a final `.gcode` file (no source model, no
modules available) and says the travel moves near the top of the print look
wrong. Standalone `gcode` mode parses the G-code directly — no model or
module dependency closure runs at all.

## 1. Author the request

```json
{
  "schema_version": "1.0",
  "source": {
    "mode": "gcode",
    "gcode_path": "/tmp/suspect_print.gcode",
    "gcode_line_width_mm": 0.4
  },
  "layers": [118, 119, 120],
  "taps": ["FinalGcode"],
  "views": ["filament_lines", "filled_areas"],
  "resolution_scale": 2
}
```

`gcode_line_width_mm` is required here because `filled_areas` in G-code mode
has no module/config source for extrusion width — it must be supplied
explicitly. `resolution_scale: 2` is chosen because travel-move detail near
the top of a tall print is small relative to the full-model viewport; if the
suspected feature isn't visible at scale 1, step up rather than starting
high, since a higher scale is more image context cost.

Save this as `visual-debug-gcode.json`.

## 2. Run `pnp_cli visual-debug`

```
pnp_cli visual-debug --request visual-debug-gcode.json --output target/visual-debug-gcode
```

Because this is standalone `gcode` mode, this run never touches
`modules/core-modules` and never compiles or loads any module — it is a pure
G-code parse. If the G-code file has commands the parser doesn't recognize,
the command still completes and records them as `warnings`, not as guessed
geometry; an outright-invalid file causes the command to fail rather than
emit a partial bundle.

## 3. Read `manifest.json` first

```
cat target/visual-debug-gcode/manifest.json
```

Check, per `images[]` entry for layers 118-120:

- `warnings` — look for `unclassified` extrusion roles (unknown role in the
  source G-code) or unsupported-command warnings. A cluster of warnings
  right where the reported defect is can itself be the answer — e.g. an
  unrecognized command near the top layers explains a visually broken
  travel move without needing to open a single PNG.
- the shared viewport and parser version, so you know the PNGs you're about
  to open are all directly comparable to each other.

## 4. Open the PNGs

```
target/visual-debug-gcode/layer_0118_final_gcode_filament_lines.png
target/visual-debug-gcode/layer_0119_final_gcode_filament_lines.png
target/visual-debug-gcode/layer_0120_final_gcode_filled_areas.png
```

`filament_lines` shows centerlines, which is usually the fastest way to spot
a stray or missing travel move; `filled_areas` shows the extrusion-width
sweep if the question is about wall/infill area rather than the travel path
itself.

## Summary of what this proves

Standalone `gcode` mode is the cheapest possible visual-debug path when you
already have a final G-code artifact and no need to re-run the pipeline: no
model, no modules, no dependency closure. It only ever tells you what the
G-code geometry looks like — for questions about why a slice took a long
time to produce that G-code, or which module is responsible for a stage,
switch to `debug-pipeline` instead.

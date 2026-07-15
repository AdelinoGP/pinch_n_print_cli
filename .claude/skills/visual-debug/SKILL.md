---
name: visual-debug
description: Localize a visible geometry defect (perimeter, infill, support, travel, or final toolpath looking wrong) in the slicer pipeline using `pnp_cli visual-debug` to render deterministic PNGs plus a `manifest.json` index. Use when the user asks "why does this perimeter look wrong", "where did this infill region change", "show me what layer N looked like at stage X", or provides/describes a model or standalone G-code file with a visual defect. Independent of debug-pipeline — do not use it for timing, DAG, or manifest questions.
type: anthropic-skill
version: "1.0"
metadata:
  internal: true
---

# Visual pipeline debugging

`pnp_cli visual-debug` renders a versioned JSON render request into a bundle:
deterministic PNGs (or, for taps not yet PNG-backed, typed IR captures) plus a
`manifest.json` index describing every image or typed capture the request
produced. It answers one question: **where in the geometry pipeline does a
visible defect first appear?**

Spec: `docs/specs/visual-pipeline-debug.md`.
Guide: `docs/19_visual_debug.md`.

This skill is `independent` of `debug-pipeline` — it is not a prerequisite,
and neither tool is a prerequisite for the other. Either can start an
investigation; pick whichever matches the reported symptom.

---

## Step 1 — Is this a visual-debug problem?

Use `pnp_cli visual-debug` when the report is about geometry: a perimeter,
infill region, support structure, travel move, or the final toolpath looks
wrong, is missing, or shifted, and the question is which pipeline stage first
produced the bad shape.

Route everything else to `debug-pipeline` instead:

| Question                                     | Tool                                 |
|-----------------------------------------------|---------------------------------------|
| "Why is this slice slow?"                     | `slice --instrument-stderr`           |
| "Why is there a missing/unexpected DAG edge?" | `pnp_cli dag`                         |
| "Is this module manifest tree valid?"         | `pnp_cli module diagnose`             |

For `timing`, `DAG`, or manifest questions, do not use pnp_cli visual-debug
— those stay owned by `debug-pipeline` (`docs/17_agent_debugging.md`,
`.claude/skills/debug-pipeline/SKILL.md`). Images do not expose static DAG
edges, validation diagnostics, or runtime timing, so they cannot answer those
questions even indirectly.

---

## Step 2 — Pick a source: `model` or `gcode`

The request's source modes are mutually exclusive:

- **`model` mode** — point at an STL (or other model input) plus a module
  directory. The command runs only the pipeline dependency closure required
  by the requested taps, then renders PNGs (or typed captures, for taps
  ahead of the PNG renderer) for the selected layers and stages.
- **`gcode` mode** — point at a standalone, already-produced final G-code
  file. No model or modules are involved; the command parses the G-code
  directly. Filled-area views in this mode require `gcode_line_width_mm` in
  the request. Unknown extrusion roles render as `unclassified`; unsupported
  commands become warnings, never guessed geometry.

Full worked examples:

- `examples/model-backed.md` — an STL-backed run with per-layer taps
  (Blackboard/arena style), reading `manifest.json`, then the PNGs.
- `examples/standalone-gcode.md` — a standalone-`.gcode` run.

Choose the smallest `resolution_scale` that makes the suspected feature
visible. Default resolution is 1024x1024; `resolution_scale: 2` and `3`
quadruple/9x the pixel count and image context cost accordingly — do not
reach for a higher scale than the defect requires.

---

## Step 3 — Manifest-first inspection

Always read `manifest.json` **before** opening any PNG. It records, per
entry: layer, tap, view type, the shared model-wide XY viewport, source
schema/parser version, and `warnings`. Reading it first tells you:

- which taps actually produced output (a request can under-resolve if a tap
  name or layer index doesn't exist in the model — see Step 4),
- whether a tap is PNG-backed yet or is a typed capture (`typed_capture`
  field populated, `png_path` empty) — treat a typed capture as the same
  evidence as a future PNG, just as structured JSON instead of an image,
  and read the `kind`/`value` pair (`Perimeter`, `Infill`, `Support`, or
  `LayerCollection`) directly,
- any per-entry `warnings` — e.g. an unclassified extrusion role or an
  unsupported G-code command — before you spend context opening images.

All PNGs in one bundle share a single viewport and legend, so a stage-to-stage
comparison (e.g. "did the wall disappear between `Layer::Perimeters` and
`Layer::PathOptimization`?") is a direct visual diff. `filament_lines` shows
centerlines, `filled_areas` shows polygons/extrusion-width sweeps,
`diagnostic_overlay` adds stage-specific labels.

---

## Step 4 — Failure behavior (fail-closed)

`pnp_cli visual-debug` fails rather than producing a partial bundle:

- An unsupported tap name, or a request whose layers don't resolve to a real
  layer in the model, fails before the model or modules even load.
- It rejects writing into a non-empty output directory unless `--overwrite`
  is passed — it will never silently mutate a pre-existing bundle.

If the command fails, do not attempt to interpret a partial `manifest.json`
as if it were complete — there isn't one. Fix the request (tap name, layer
index, `--overwrite`) and re-run.

---

## Output discipline

Report one short paragraph naming the stage/layer where the defect first
appears, plus the specific `manifest.json` entry (and, if needed, the one or
two PNGs) that prove it. Don't paste the whole manifest — quote the relevant
entry.

Do not claim OrcaSlicer parity, WASM runtime behavior, or coordinate-system
semantics from this skill — those are out of scope; see
`docs/08_coordinate_system.md` for coordinates and
`docs/17_agent_debugging.md`/`debug-pipeline` for timing and DAG semantics.

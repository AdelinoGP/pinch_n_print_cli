# Worked example — model-backed visual debug

Scenario: a report says the arena/Blackboard test model's infill looks wrong
starting around layer 40, and possibly earlier. We want to see the infill
region right after `Layer::Infill` and again after `Layer::PathOptimization`
to find where it changes.

## 1. Author the request

```json
{
  "schema_version": "1.0",
  "source": {
    "mode": "model",
    "model_path": "resources/arena.stl",
    "module_dir": "modules/core-modules"
  },
  "layers": [40],
  "taps": ["Layer::Infill", "Layer::PathOptimization"],
  "views": ["filled_areas", "diagnostic_overlay"],
  "resolution_scale": 1
}
```

Save this as `visual-debug.json`. Model mode runs only the dependency closure
required to satisfy these two taps for layer 40 — it does not re-run the
whole pipeline or every layer.

## 2. Run `pnp_cli visual-debug`

```
pnp_cli visual-debug --request visual-debug.json --output target/visual-debug
```

If `target/visual-debug` already exists and is non-empty, add `--overwrite`
(the command otherwise refuses to write into it, and never partially
overwrites a bundle).

## 3. Read `manifest.json` first

```
cat target/visual-debug/manifest.json
```

Look at, in order:

1. `executed_stage_ids` — confirm the closure that actually ran reaches
   through `Layer::PathOptimization`.
2. `executed_layer_indices` — confirm layer 40 (and only the layers you
   asked for — a `Layer::*` tap has no cross-layer dependency, so nothing
   else should have executed).
3. `layer_expansions` — should be empty; a non-empty entry would mean the
   closure had to pull in another layer for a genuine cross-layer
   correctness reason (not the case for any tap this packet supports today).
4. Each `images[]` entry's `warnings` array.

If the intermediate PNG renderer for these taps hasn't landed yet, the
`images[]` entries for `Layer::Infill` / `Layer::PathOptimization` at layer 40
will have an empty `png_path` and a populated `typed_capture` instead, e.g.:

```json
{
  "layer": 40,
  "tap": "Layer::Infill",
  "png_path": "",
  "typed_capture": {"kind": "Infill", "value": { "...": "committed IR" }},
  "warnings": []
}
```

Treat `typed_capture` as the same evidence a PNG would give — read the IR
directly (polygon loops, extrusion widths, etc.) instead of opening an image.

## 4. Open the PNGs (when present)

Once `png_path` is populated for a stage, open it:

```
target/visual-debug/layer_0040_infill_filled_areas.png
target/visual-debug/layer_0040_path_optimization_filled_areas.png
```

Both share the same model-wide XY viewport, so a direct visual diff between
the two PNGs shows exactly what `Layer::PathOptimization` changed relative to
`Layer::Infill`. If the infill region is present after `Layer::Infill` but
missing or shifted after `Layer::PathOptimization`, the defect is introduced
by path optimization, not by infill generation — narrow further investigation
(e.g. code review or a narrower `cargo test` run) to that stage.

## Summary of what this proves

This workflow isolates *where* in the geometry pipeline a defect first
appears. It says nothing about *why* that stage is slow or *how* its module
wiring is configured — for those questions, switch to `debug-pipeline`
(`slice --instrument-stderr`, `pnp_cli dag`, `pnp_cli module diagnose`).

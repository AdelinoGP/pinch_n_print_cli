# Worked example — model-backed visual debug

Scenario: a report says the regression wedge's infill looks wrong starting
around layer 40, and possibly earlier. We want to see the infill region right
after `Layer::Infill` and again after `Layer::PathOptimization` to find where
it changes.

## 1. Author the request

```json
{
  "schema_version": "1.0.0",
  "source": {
    "kind": "model",
    "model": "resources/regression_wedge.stl",
    "config": "resources/test_config/gate_evidence_50l.json",
    "module_dirs": ["modules/core-modules"]
  },
  "layers": [40],
  "taps": ["Layer::Infill", "Layer::PathOptimization"],
  "visualizations": ["filled_areas", "diagnostic_overlay"],
  "resolution_scale": 1
}
```

The request struct is `deny_unknown_fields`: `schema_version` must be exactly
`"1.0.0"`, the source is tagged `kind` (not `mode`), and the field is
`visualizations` (not `views`). A misspelled key fails deserialization
outright rather than being ignored.

For a `model` source, both `config` and `module_dirs` are **required** and
must be non-empty — `"config": null` is rejected with
`missing required field: config`. A config is an ordinary settings-override
JSON (`resources/test_config/` holds several).

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
  "layer_index": 40,
  "tap": "Layer::Infill",
  "png_path": "",
  "typed_capture": {"kind": "Infill", "value": { "...": "committed IR" }},
  "warnings": []
}
```

Note `typed_capture` inlines the full IR, so a model-source `manifest.json`
can run to megabytes — do not `cat` one. Strip the captures first, e.g.
`jq 'del(.images[].typed_capture)' manifest.json`.

Treat `typed_capture` as the same evidence a PNG would give — read the IR
directly (polygon loops, extrusion widths, etc.) instead of opening an image.

## 4. Open the PNGs (when present)

Once `png_path` is populated for a stage, open it:

```
target/visual-debug/images/Layer__Infill_filled_areas_l40.png
target/visual-debug/images/Layer__PathOptimization_filled_areas_l40.png
```

PNGs live in the bundle's `images/` subdirectory, named
`{tap}_{visualization}_l{layer}.png` with `::` sanitized to `__`. Always take
the path from the entry's `png_path` rather than reconstructing it.

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

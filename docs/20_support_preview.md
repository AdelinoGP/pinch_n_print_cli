# Support Preview JSON Contract

The `support-preview` verb runs the support-geometry prepass and writes a
fork-facing JSON document for coarse support visualization. Its latency
contract is **prepass only — no per-layer or G-code stages**.

## Schema Version

The document has `schema_version: "1.0.0"`. This is the document contract
version, not an IR version. Additive fields bump the minor version.

## Coordinate Units

All polygon coordinates in the JSON are in millimeters (`units` is `"mm"`).
Internally, 1 scaled integer unit is 100 nm (`10^-4 mm`); see
[`docs/08_coordinate_system.md`](08_coordinate_system.md). Conversion is
`mm = units / 10_000`.

For example, the internal point `(1234567, -89012)` becomes
`(0.1234567, -0.0089012)` in JSON.

## Document Shape

The top-level document has these fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | string | The support-preview document schema version, currently `"1.0.0"`. |
| `units` | string | Coordinate unit label, currently `"mm"`. |
| `layer_count` | u32 | Total model-layer count in `plan.global_layers`. |
| `skipped_intermediate_entries` | u32 | Number of intermediate-model-resolution support entries excluded by the sentinel rule. |
| `layers` | array of layer objects | Sparse support geometry records, ordered by `layer_index`. |

Each element of `layers` has this shape:

| Field | Type | Meaning |
| --- | --- | --- |
| `layer_index` | u32 | Model-layer index from `plan.global_layers`, not a support-only layer index. |
| `z_mm` | f64 | The model layer Z coordinate in millimeters. |
| `support` | array of polygon objects | Coarse support outline geometry for this model layer. |

Each element of `support` has this shape:

| Field | Type | Meaning |
| --- | --- | --- |
| `contour` | array of `[f64, f64]` | The polygon's exterior contour, as `[x_mm, y_mm]` points. |
| `holes` | array of arrays of `[f64, f64]` | Interior hole contours, also as `[x_mm, y_mm]` points. |

A complete example document is:

```json
{
  "schema_version": "1.0.0",
  "units": "mm",
  "layer_count": 4,
  "skipped_intermediate_entries": 1,
  "layers": [
    {
      "layer_index": 0,
      "z_mm": 0.2,
      "support": [
        {
          "contour": [
            [0.1234567, -0.0089012],
            [1.0, -0.0089012],
            [1.0, 0.5],
            [0.1234567, 0.5]
          ],
          "holes": []
        }
      ]
    },
    {
      "layer_index": 2,
      "z_mm": 0.6,
      "support": [
        {
          "contour": [
            [2.0, 2.0],
            [3.0, 2.0],
            [3.0, 3.0],
            [2.0, 3.0]
          ],
          "holes": [
            [
              [2.25, 2.25],
              [2.75, 2.25],
              [2.75, 2.75],
              [2.25, 2.75]
            ]
          ]
        }
      ]
    }
  ]
}
```

## Layer Selection And Sentinels

An entry whose `global_support_layer_index == u32::MAX` is the
intermediate-model-resolution sentinel. Such entries are excluded from
`layers` and counted in `skipped_intermediate_entries`.

Layers with no support geometry are omitted from `layers`; the array is
sparse. Use `layer_count` for the total plan layer count rather than the
length of `layers`.

The `layer_index` value is a model-layer index from `plan.global_layers`, not
a support-only layer index.

## Determinism And Scope

For identical input and configuration, output is byte-deterministic. Entries
are sorted by `(layer_index, object_id, region_id)` before emission, and
layers are sorted by `layer_index` ascending.

There is no interface split at this stage. The single `support` array is the
coarse per-layer outline. An `interface`/role split is not available until
Tier 2 per-layer execution runs, which is out of scope for this verb. A future
minor schema bump may add an `interface` array.

These outlines are approximate by design and may differ from final support
paths after Tier 2 post-plan trimming. The fork should debounce calls because
prepass cost is model-size-dependent.

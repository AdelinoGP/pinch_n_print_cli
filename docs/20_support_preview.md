# Support Preview JSON Contract

The `support-preview` verb runs the support-geometry prepass and writes a
fork-facing JSON document for coarse support visualization. Its latency
contract is **prepass only — no per-layer or G-code stages**.

## CLI Usage

```bash
pnp_cli support-preview --input <model.stl|model.obj|model.3mf> --output <path>
```

`--config <path>` optionally supplies a config file. `--module-dir <path>` is
repeatable and adds module search paths. `--no-default-module-paths` disables
the default module directories. The verb writes the JSON document only to the
requested output path and never emits G-code.

## Schema Version

The document has `schema_version: "1.1.0"`. This is the document contract
version, not an IR version. Additive fields bump the minor version.

1.1.0 adds `layers[].support_body` — the actual support structures from the
committed `SupportPlanIR` (`SupportBody` role regions), which the fork renders
as the overlay. The 1.0.0 `support` field is retained unchanged: it carries
the model's own cross-sections at support layers (coarse outlines of where
supports attach), which is not the support geometry itself.

## Coordinate Units

All polygon coordinates in the JSON are in millimeters (`units` is `"mm"`).
Internally, 1 scaled integer unit is 100 nm (`10^-4 mm`); see
[`docs/08_coordinate_system.md`](08_coordinate_system.md). Conversion is
`mm = units / 10_000`.

For example, the internal point `(1234567, -89012)` becomes
`(123.4567, -8.9012)` in JSON.

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
| `support` | array of polygon objects | Coarse support outline geometry for this model layer (the model's own cross-sections at support layers; where supports attach, not the supports themselves). |
| `support_body` | array of polygon objects | Actual support structures for this model layer (schema 1.1.0): the `SupportPlanIR` `SupportBody` role regions — the tree/traditional support cross-sections. Always present in 1.1.0 documents, possibly empty. Raft prefix entries carry no geometry and are excluded. |

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
            [123.4567, -8.9012],
            [1.0, -8.9012],
            [1.0, 0.5],
            [123.4567, 0.5]
          ],
          "holes": []
        }
      ],
      "support_body": [
        {
          "contour": [
            [2.0, 0.5],
            [3.0, 0.5],
            [3.0, 1.5],
            [2.0, 1.5]
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

## Absent Support

When support is disabled or no `SupportGeometryIR` is committed to the
blackboard, the command still exits successfully and emits `layers: []`.
`layer_count` continues to report the total number of plan layers, so the fork
can distinguish an empty overlay from a failed run.

## Invalid Input

A nonexistent or invalid input causes a nonzero exit with an error naming the
input path. No output document is produced, and no partial output file is
left behind.

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

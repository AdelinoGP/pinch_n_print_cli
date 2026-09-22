# Performance refresh — measured evidence

Generated from `results.jsonl` and instrumented logs. Times are seconds.

## All process measurements

| Run | Wall | Process CPU | CPU/wall | Other system busy CPU/wall |
| --- | ---: | ---: | ---: | ---: |
| base-warmup-0 | 507.150 | 2955.750 | 5.828 | 2.721 |
| base-baseline-1 | 502.827 | 2931.828 | 5.831 | 2.634 |
| base-baseline-2 | 503.834 | 2923.328 | 5.802 | 2.595 |
| base-baseline-3 | 496.090 | 2927.641 | 5.901 | 2.604 |
| 3dbenchy-warmup-0 | 29.602 | 177.156 | 5.985 | 2.230 |
| 3dbenchy-baseline-1 | 28.935 | 176.391 | 6.096 | 2.162 |
| 3dbenchy-baseline-2 | 30.216 | 176.531 | 5.842 | 2.389 |
| 3dbenchy-baseline-3 | 30.163 | 176.125 | 5.839 | 2.566 |
| base-instrumented-1 | 464.640 | 2910.969 | 6.265 | 1.819 |
| 3dbenchy-instrumented-1 | 26.164 | 173.938 | 6.648 | 0.743 |
| base-baseline-4 | 440.691 | 2904.328 | 6.590 | 0.887 |
| base-baseline-5 | 462.152 | 2896.531 | 6.267 | 2.085 |
| base-baseline-6 | 469.346 | 2904.422 | 6.188 | 2.065 |
| 3dbenchy-baseline-4 | 28.877 | 176.875 | 6.125 | 1.688 |
| 3dbenchy-baseline-5 | 28.926 | 177.281 | 6.129 | 1.468 |
| 3dbenchy-baseline-6 | 29.308 | 175.125 | 5.975 | 1.493 |
| base-instrumented-2 | 443.061 | 2885.094 | 6.512 | 1.383 |

Other-system work is GetSystemTimes busy delta minus child GetProcessTimes CPU.
It includes the harness, other processes, and system work; it does not prove scheduler starvation.
No samples establish a controlled quiet-machine baseline. Warmups and attribution runs are excluded from baseline summaries.

## Uninstrumented baseline batches (load-qualified)

| Model | Batch | Median wall | Wall range |
| --- | --- | ---: | --- |
| base | 1–3 | 502.827 | 496.090–503.834 |
| base | 4–6 | 462.152 | 440.691–469.346 |
| 3dbenchy | 1–3 | 30.163 | 28.935–30.216 |
| 3dbenchy | 4–6 | 28.926 | 28.877–29.308 |

## base-instrumented-1 attribution

### Phase wall

| Phase | Seconds |
| --- | ---: |
| validation | 0.002 |
| prepass | 283.773 |
| per_layer | 171.924 |
| postpass | 5.869 |

### prepass: Module elapsed wall

| Module | Seconds |
| --- | ---: |
| `com.core.tree-support-planner` | 96.630 |
| `host:slice` | 56.190 |
| `host:support_analysis` | 39.147 |
| `host:shell_classification` | 34.660 |
| `host:mesh_analysis` | 24.203 |
| `host:overhang_annotation` | 17.580 |
| `com.core.seam-planner-default` | 0.537 |
| `com.core.traditional-support-planner` | 0.172 |
| `host:paint_segmentation` | 0.085 |
| `host:support_geometry` | 0.012 |
| `host:region_mapping` | 0.006 |
| `com.core.layer-planner-default` | 0.001 |

### per_layer: Accumulated worker elapsed, NOT CPU or slice wall

| Module | Seconds |
| --- | ---: |
| `com.core.classic-perimeters` | 1725.920 |
| `com.core.tree-support` | 58.098 |
| `com.core.rectilinear-infill` | 34.672 |
| `com.core.gyroid-infill` | 34.615 |
| `com.core.support-surface-ironing` | 34.495 |
| `com.core.lightning-infill` | 34.414 |
| `com.core.top-surface-ironing` | 34.411 |
| `com.core.wave-overhangs` | 34.407 |
| `com.core.infill-linker` | 24.753 |
| `com.core.seam-placer` | 2.306 |
| `com.core.fuzzy-skin` | 2.214 |
| `com.core.path-optimization-default` | 1.429 |
| `host:paint_annotator` | 0.000 |

### postpass: Module elapsed wall

| Module | Seconds |
| --- | ---: |
| `com.core.overhang-classifier-default` | 0.931 |
| `com.core.wipe-tower` | 0.911 |
| `com.core.skirt-brim` | 0.879 |
| `com.core.part-cooling` | 0.870 |
| `host:gcode_emit` | 0.822 |
| `host:gcode_serialize` | 0.492 |
| `com.core.machine-gcode-emit` | 0.354 |

## 3dbenchy-instrumented-1 attribution

### Phase wall

| Phase | Seconds |
| --- | ---: |
| validation | 0.003 |
| prepass | 11.856 |
| per_layer | 12.810 |
| postpass | 0.715 |

### prepass: Module elapsed wall

| Module | Seconds |
| --- | ---: |
| `com.core.tree-support-planner` | 3.299 |
| `host:shell_classification` | 2.622 |
| `host:slice` | 2.403 |
| `host:support_analysis` | 1.901 |
| `host:overhang_annotation` | 0.770 |
| `host:mesh_analysis` | 0.213 |
| `com.core.seam-planner-default` | 0.090 |
| `com.core.traditional-support-planner` | 0.026 |
| `host:paint_segmentation` | 0.010 |
| `host:region_mapping` | 0.002 |
| `com.core.layer-planner-default` | 0.001 |
| `host:support_geometry` | 0.001 |

### per_layer: Accumulated worker elapsed, NOT CPU or slice wall

| Module | Seconds |
| --- | ---: |
| `com.core.classic-perimeters` | 100.133 |
| `com.core.infill-linker` | 10.117 |
| `com.core.tree-support` | 4.365 |
| `com.core.rectilinear-infill` | 1.426 |
| `com.core.support-surface-ironing` | 1.360 |
| `com.core.top-surface-ironing` | 1.347 |
| `com.core.gyroid-infill` | 1.346 |
| `com.core.wave-overhangs` | 1.335 |
| `com.core.lightning-infill` | 1.333 |
| `com.core.seam-placer` | 0.398 |
| `com.core.fuzzy-skin` | 0.393 |
| `com.core.path-optimization-default` | 0.244 |
| `host:paint_annotator` | 0.000 |

### postpass: Module elapsed wall

| Module | Seconds |
| --- | ---: |
| `com.core.part-cooling` | 0.121 |
| `host:gcode_serialize` | 0.120 |
| `com.core.overhang-classifier-default` | 0.112 |
| `com.core.skirt-brim` | 0.100 |
| `com.core.wipe-tower` | 0.100 |
| `com.core.machine-gcode-emit` | 0.079 |
| `host:gcode_emit` | 0.059 |

## base-instrumented-2 attribution

### Phase wall

| Phase | Seconds |
| --- | ---: |
| validation | 0.003 |
| prepass | 258.189 |
| per_layer | 176.010 |
| postpass | 5.903 |

### prepass: Module elapsed wall

| Module | Seconds |
| --- | ---: |
| `com.core.tree-support-planner` | 92.850 |
| `host:slice` | 46.409 |
| `host:support_analysis` | 37.791 |
| `host:shell_classification` | 32.203 |
| `host:mesh_analysis` | 22.354 |
| `host:overhang_annotation` | 11.857 |
| `com.core.seam-planner-default` | 0.535 |
| `com.core.traditional-support-planner` | 0.184 |
| `host:paint_segmentation` | 0.070 |
| `host:support_geometry` | 0.013 |
| `host:region_mapping` | 0.006 |
| `com.core.layer-planner-default` | 0.001 |

### per_layer: Accumulated worker elapsed, NOT CPU or slice wall

| Module | Seconds |
| --- | ---: |
| `com.core.classic-perimeters` | 1743.704 |
| `com.core.tree-support` | 58.411 |
| `com.core.rectilinear-infill` | 35.625 |
| `com.core.top-surface-ironing` | 35.516 |
| `com.core.gyroid-infill` | 35.477 |
| `com.core.lightning-infill` | 35.326 |
| `com.core.wave-overhangs` | 35.297 |
| `com.core.support-surface-ironing` | 35.033 |
| `com.core.infill-linker` | 25.031 |
| `com.core.seam-placer` | 2.270 |
| `com.core.fuzzy-skin` | 2.264 |
| `com.core.path-optimization-default` | 1.432 |
| `host:paint_annotator` | 0.000 |

### postpass: Module elapsed wall

| Module | Seconds |
| --- | ---: |
| `com.core.overhang-classifier-default` | 0.970 |
| `com.core.wipe-tower` | 0.935 |
| `com.core.part-cooling` | 0.908 |
| `com.core.skirt-brim` | 0.897 |
| `host:gcode_emit` | 0.716 |
| `host:gcode_serialize` | 0.492 |
| `com.core.machine-gcode-emit` | 0.359 |

## Output consistency (all runs, including warmups)

### base

G-code byte range: 30163278–30163730.

| Varying TYPE section | Minimum | Maximum |
| --- | ---: | ---: |
| `;TYPE:Inner wall` | 525 | 527 |
- `gcode_prediction_seconds`: 25113–25121
- `gcode_filament_length_mm`: 25354.486328125–25355.263671875
- `layer_count`: 495–495
- Non-fatal error counts observed: [29108]
### 3dbenchy

G-code byte range: 7640031–7641161.

| Varying TYPE section | Minimum | Maximum |
| --- | ---: | ---: |
| `;TYPE:Inner wall` | 244 | 251 |
- `gcode_prediction_seconds`: 7702–7708
- `gcode_filament_length_mm`: 6368.8369140625–6369.07275390625
- `layer_count`: 240–240
- Non-fatal error counts observed: [0]

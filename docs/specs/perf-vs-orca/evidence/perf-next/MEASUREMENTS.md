# Supports-off current-snapshot baseline

Generated from `tmp/perf-next/baseline.csv` and, when present, `tmp/perf-next/attribution/*.jsonl`. Measured numbers only; this is a supports-off current-snapshot baseline, **not** a speedup or correctness proof. Guest fuel, accumulated worker elapsed, native host wall, process CPU, and process wall are different units and scopes and are never summed.

## Baseline (non-warmup samples)

| generator | n | median wall (s) | wall range (s) | median CPU (s) | CPU range (s) | CPU/wall ratios |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| arachne | 4 | 18.1533 | 17.9063–19.7151 | 74.8828 | 73.5312–75.5469 | 4.089, 3.832, 4.171, 4.098 |
| classic | 4 | 21.9882 | 21.2958–24.2644 | 143.4141 | 141.4062–144.3906 | 6.497, 6.455, 5.951, 6.735 |

## Per-sample detail

| label | gen | warmup | wall (s) | CPU (s) | CPU/wall | status | fatal | non-fatal | layer_count | gcode_prediction_s | gcode_sha256 |
| --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | --- |
| baseline-classic-warmup | classic | 1 | 22.5402 | 142.4844 | 6.321 | ok | 0 | 0 | 240 | 5363 | f1bad4f49aaa |
| baseline-arachne-warmup | arachne | 1 | 18.4242 | 74.8906 | 4.065 | ok | 0 | 0 | 240 | 4239 | e39d8084107b |
| baseline-classic-1 | classic | 0 | 22.0714 | 143.3906 | 6.497 | ok | 0 | 0 | 240 | 5367 | d57ce8d40838 |
| baseline-arachne-1 | arachne | 0 | 18.3630 | 75.0781 | 4.089 | ok | 0 | 0 | 240 | 4239 | e39d8084107b |
| baseline-arachne-2 | arachne | 0 | 19.7151 | 75.5469 | 3.832 | ok | 0 | 0 | 240 | 4239 | e39d8084107b |
| baseline-classic-2 | classic | 0 | 21.9050 | 141.4062 | 6.455 | ok | 0 | 0 | 240 | 5367 | dffd13965e13 |
| baseline-classic-3 | classic | 0 | 24.2644 | 144.3906 | 5.951 | ok | 0 | 0 | 240 | 5365 | 906cfe3567e5 |
| baseline-arachne-3 | arachne | 0 | 17.9063 | 74.6875 | 4.171 | ok | 0 | 0 | 240 | 4239 | e39d8084107b |
| baseline-arachne-4 | arachne | 0 | 17.9437 | 73.5312 | 4.098 | ok | 0 | 0 | 240 | 4239 | e39d8084107b |
| baseline-classic-4 | classic | 0 | 21.2958 | 143.4375 | 6.735 | ok | 0 | 0 | 240 | 5366 | 5f8a8009f347 |

## G-code TYPE counts (non-warmup samples)

### arachne

| TYPE | min | max |
| --- | ---: | ---: |
| `Bottom surface` | 29 | 29 |
| `Bridge` | 86 | 86 |
| `Brim` | 1 | 1 |
| `Gap infill` | 4 | 4 |
| `Inner wall` | 692 | 692 |
| `Internal Bridge` | 10 | 10 |
| `Internal solid infill` | 6 | 6 |
| `Outer wall` | 702 | 702 |
| `Skirt` | 1 | 1 |
| `Sparse infill` | 138 | 138 |
| `Top surface` | 66 | 66 |

### classic

| TYPE | min | max |
| --- | ---: | ---: |
| `Bottom surface` | 28 | 28 |
| `Bridge` | 124 | 124 |
| `Brim` | 1 | 1 |
| `Gap infill` | 387 | 387 |
| `Inner wall` | 244 | 248 |
| `Internal Bridge` | 13 | 13 |
| `Internal solid infill` | 6 | 6 |
| `Outer wall` | 240 | 240 |
| `Skirt` | 1 | 1 |
| `Sparse infill` | 166 | 166 |
| `Top surface` | 103 | 103 |

## Attribution (optional)

### instrument captures

#### attribution-arachne-instrumented

Accumulated worker elapsed by module (seconds; NOT CPU or slice wall).

| module | seconds |
| --- | ---: |
| `com.core.arachne-perimeters` | 11.363 |
| `com.core.infill-linker` | 10.626 |
| `host:shell_classification` | 2.750 |
| `host:slice` | 2.637 |
| `com.core.gyroid-infill` | 1.413 |
| `com.core.traditional-support` | 1.150 |
| `host:overhang_annotation` | 0.857 |
| `com.core.seam-placer` | 0.317 |
| `com.core.rectilinear-infill` | 0.311 |
| `com.core.top-surface-ironing` | 0.291 |
| `com.core.wave-overhangs` | 0.256 |
| `com.core.lightning-infill` | 0.254 |
| `host:mesh_analysis` | 0.222 |
| `com.core.fuzzy-skin` | 0.216 |
| `com.core.support-surface-ironing` | 0.189 |
| `com.core.path-optimization-default` | 0.160 |
| `host:paint_annotator` | 0.103 |
| `com.core.seam-planner-default` | 0.097 |
| `com.core.machine-gcode-emit` | 0.072 |
| `host:gcode_serialize` | 0.071 |
| `host:gcode_emit` | 0.051 |
| `com.core.part-cooling` | 0.021 |
| `com.core.skirt-brim` | 0.020 |
| `com.core.traditional-support-planner` | 0.017 |
| `com.core.tree-support-planner` | 0.017 |
| `com.core.wipe-tower` | 0.017 |
| `com.core.overhang-classifier-default` | 0.016 |
| `host:paint_segmentation` | 0.009 |
| `host:region_mapping` | 0.002 |
| `com.core.layer-planner-default` | 0.001 |
| `host:support_geometry` | 0.001 |
| `host:support_analysis` | 0.000 |

#### attribution-classic-instrumented

Accumulated worker elapsed by module (seconds; NOT CPU or slice wall).

| module | seconds |
| --- | ---: |
| `com.core.classic-perimeters` | 90.893 |
| `com.core.infill-linker` | 10.733 |
| `host:shell_classification` | 2.893 |
| `host:slice` | 2.587 |
| `com.core.gyroid-infill` | 1.347 |
| `com.core.traditional-support` | 1.028 |
| `host:overhang_annotation` | 0.817 |
| `com.core.seam-placer` | 0.452 |
| `com.core.fuzzy-skin` | 0.423 |
| `com.core.path-optimization-default` | 0.285 |
| `host:mesh_analysis` | 0.226 |
| `com.core.rectilinear-infill` | 0.128 |
| `com.core.seam-planner-default` | 0.126 |
| `com.core.overhang-classifier-default` | 0.098 |
| `com.core.part-cooling` | 0.092 |
| `com.core.wipe-tower` | 0.088 |
| `com.core.skirt-brim` | 0.087 |
| `com.core.machine-gcode-emit` | 0.083 |
| `host:gcode_serialize` | 0.076 |
| `com.core.top-surface-ironing` | 0.072 |
| `com.core.lightning-infill` | 0.071 |
| `com.core.wave-overhangs` | 0.069 |
| `com.core.support-surface-ironing` | 0.065 |
| `host:gcode_emit` | 0.036 |
| `host:paint_annotator` | 0.027 |
| `com.core.traditional-support-planner` | 0.021 |
| `com.core.tree-support-planner` | 0.020 |
| `host:paint_segmentation` | 0.015 |
| `host:region_mapping` | 0.002 |
| `host:support_geometry` | 0.002 |
| `com.core.layer-planner-default` | 0.001 |
| `host:support_analysis` | 0.000 |

### profile captures

#### attribution-arachne-profile

Accumulated worker elapsed by module (seconds; profile-inflated wall, NOT CPU or slice wall).

| module | seconds |
| --- | ---: |
| `com.core.arachne-perimeters` | 12.035 |
| `com.core.infill-linker` | 11.312 |
| `host:shell_classification` | 2.751 |
| `host:slice` | 2.563 |
| `com.core.gyroid-infill` | 1.401 |
| `com.core.traditional-support` | 1.232 |
| `host:overhang_annotation` | 0.813 |
| `com.core.rectilinear-infill` | 0.356 |
| `com.core.seam-placer` | 0.323 |
| `com.core.top-surface-ironing` | 0.302 |
| `com.core.lightning-infill` | 0.273 |
| `com.core.wave-overhangs` | 0.271 |
| `host:mesh_analysis` | 0.219 |
| `com.core.fuzzy-skin` | 0.195 |
| `com.core.path-optimization-default` | 0.170 |
| `com.core.support-surface-ironing` | 0.143 |
| `com.core.seam-planner-default` | 0.105 |
| `host:paint_annotator` | 0.103 |
| `host:gcode_serialize` | 0.066 |
| `com.core.machine-gcode-emit` | 0.060 |
| `host:gcode_emit` | 0.037 |
| `com.core.part-cooling` | 0.018 |
| `com.core.traditional-support-planner` | 0.017 |
| `com.core.tree-support-planner` | 0.017 |
| `com.core.skirt-brim` | 0.016 |
| `com.core.wipe-tower` | 0.016 |
| `com.core.overhang-classifier-default` | 0.015 |
| `host:paint_segmentation` | 0.010 |
| `host:region_mapping` | 0.002 |
| `com.core.layer-planner-default` | 0.001 |
| `host:support_analysis` | 0.000 |
| `host:support_geometry` | 0.000 |

Guest fuel total: 264,745,237,103 fuel. Host wall total: 49.489 s (indicative profiling wall).

##### Guest fuel ranking (share of fuel_total)

| module | calls | total_fuel | % of fuel_total |
| --- | ---: | ---: | ---: |
| `com.core.infill-linker` | 240 | 141,040,199,177 | 53.274 |
| `com.core.arachne-perimeters` | 240 | 121,501,905,791 | 45.894 |
| `com.core.seam-planner-default` | 1 | 597,899,161 | 0.226 |
| `com.core.rectilinear-infill` | 240 | 287,969,121 | 0.109 |
| `com.core.seam-placer` | 240 | 180,366,571 | 0.068 |
| `com.core.fuzzy-skin` | 240 | 152,611,337 | 0.058 |
| `com.core.machine-gcode-emit` | 1 | 104,823,842 | 0.04 |
| `com.core.traditional-support` | 240 | 99,996,777 | 0.038 |
| `com.core.path-optimization-default` | 240 | 93,809,275 | 0.035 |
| `com.core.wave-overhangs` | 240 | 93,349,041 | 0.035 |
| `com.core.gyroid-infill` | 240 | 88,114,939 | 0.033 |
| `com.core.lightning-infill` | 240 | 86,601,547 | 0.033 |
| `com.core.support-surface-ironing` | 240 | 85,913,193 | 0.032 |
| `com.core.top-surface-ironing` | 240 | 85,600,338 | 0.032 |
| `com.core.part-cooling` | 1 | 47,070,339 | 0.018 |
| `com.core.skirt-brim` | 1 | 44,856,022 | 0.017 |
| `com.core.wipe-tower` | 1 | 44,555,609 | 0.017 |
| `com.core.overhang-classifier-default` | 1 | 44,495,354 | 0.017 |
| `com.core.tree-support-planner` | 1 | 32,243,380 | 0.012 |
| `com.core.traditional-support-planner` | 1 | 32,221,338 | 0.012 |
| `com.core.layer-planner-default` | 1 | 634,951 | 0.0 |

Top guest scopes (self_fuel):

| module | scope | calls | self_fuel | % of module total_fuel |
| --- | --- | ---: | ---: | ---: |
| `com.core.infill-linker` | `polygon_ops::clip_polygons` | 297 | 190,644,134 | 0.135 |

##### Host wall ranking (share of wall_total_ns)

| module | calls | total_wall_ns | % of wall_total |
| --- | ---: | ---: | ---: |
| `host:slice` | 5921 | 25,372,333,100 | 51.269 |
| `host:shell_classification` | 3910 | 10,242,843,200 | 20.697 |
| `host:overhang_annotation` | 3107 | 8,066,388,800 | 16.299 |
| `host:native` | 8958 | 5,681,158,400 | 11.48 |
| `host:mesh_analysis` | 343 | 126,394,700 | 0.255 |

Top host scopes (self_wall_ns):

| module | scope | calls | self_wall_ns | % of module total_wall_ns |
| --- | --- | ---: | ---: | ---: |
| `host:slice` | `polygon_ops::closing_ex` | 240 | 21,909,532,400 | 86.352 |
| `host:slice` | `polygon_ops::clip_polygons` | 5005 | 1,916,985,300 | 7.555 |
| `host:slice` | `polygon_ops::offset` | 676 | 1,545,673,000 | 6.092 |
| `host:shell_classification` | `polygon_ops::offset` | 1689 | 9,007,576,900 | 87.94 |
| `host:shell_classification` | `polygon_ops::clip_polygons` | 2052 | 1,000,865,000 | 9.771 |
| `host:shell_classification` | `polygon_ops::closing_ex` | 169 | 234,303,900 | 2.287 |
| `host:overhang_annotation` | `polygon_ops::offset` | 717 | 5,797,171,500 | 71.868 |
| `host:overhang_annotation` | `polygon_ops::clip_polygons` | 2390 | 2,269,133,500 | 28.131 |
| `host:native` | `polygon_ops::clip_polygons` | 8147 | 4,249,954,200 | 74.808 |
| `host:native` | `polygon_ops::offset` | 543 | 731,633,900 | 12.878 |
| `host:native` | `polygon_ops::offset2_ex` | 240 | 679,207,600 | 11.955 |
| `host:native` | `polygon_ops::closing_ex` | 27 | 15,194,900 | 0.267 |
| `host:native` | `polygon_ops::opening` | 1 | 4,923,800 | 0.087 |
| `host:mesh_analysis` | `polygon_ops::clip_polygons` | 343 | 126,386,100 | 99.993 |

#### attribution-classic-profile

Accumulated worker elapsed by module (seconds; profile-inflated wall, NOT CPU or slice wall).

| module | seconds |
| --- | ---: |
| `com.core.classic-perimeters` | 96.638 |
| `com.core.infill-linker` | 11.041 |
| `host:shell_classification` | 2.732 |
| `host:slice` | 2.490 |
| `com.core.gyroid-infill` | 1.377 |
| `com.core.traditional-support` | 1.056 |
| `host:overhang_annotation` | 0.813 |
| `com.core.seam-placer` | 0.482 |
| `com.core.fuzzy-skin` | 0.459 |
| `com.core.path-optimization-default` | 0.340 |
| `host:mesh_analysis` | 0.242 |
| `com.core.rectilinear-infill` | 0.157 |
| `com.core.lightning-infill` | 0.123 |
| `com.core.seam-planner-default` | 0.104 |
| `com.core.top-surface-ironing` | 0.099 |
| `com.core.overhang-classifier-default` | 0.089 |
| `com.core.wave-overhangs` | 0.088 |
| `com.core.part-cooling` | 0.082 |
| `com.core.support-surface-ironing` | 0.076 |
| `host:gcode_serialize` | 0.075 |
| `com.core.skirt-brim` | 0.072 |
| `com.core.wipe-tower` | 0.070 |
| `com.core.machine-gcode-emit` | 0.062 |
| `host:gcode_emit` | 0.037 |
| `com.core.traditional-support-planner` | 0.017 |
| `com.core.tree-support-planner` | 0.016 |
| `host:paint_annotator` | 0.015 |
| `host:paint_segmentation` | 0.011 |
| `host:region_mapping` | 0.002 |
| `com.core.layer-planner-default` | 0.001 |
| `host:support_geometry` | 0.001 |
| `host:support_analysis` | 0.000 |

Guest fuel total: 1,134,565,724,056 fuel. Host wall total: 54.291 s (indicative profiling wall).

##### Guest fuel ranking (share of fuel_total)

| module | calls | total_fuel | % of fuel_total |
| --- | ---: | ---: | ---: |
| `com.core.classic-perimeters` | 240 | 1,012,844,470,264 | 89.272 |
| `com.core.infill-linker` | 240 | 116,806,680,892 | 10.295 |
| `com.core.seam-placer` | 240 | 1,116,433,555 | 0.098 |
| `com.core.fuzzy-skin` | 240 | 937,330,655 | 0.083 |
| `com.core.seam-planner-default` | 1 | 597,899,161 | 0.053 |
| `com.core.path-optimization-default` | 240 | 486,027,952 | 0.043 |
| `com.core.rectilinear-infill` | 240 | 272,964,239 | 0.024 |
| `com.core.part-cooling` | 1 | 200,544,094 | 0.018 |
| `com.core.skirt-brim` | 1 | 198,691,742 | 0.018 |
| `com.core.wipe-tower` | 1 | 197,568,702 | 0.017 |
| `com.core.overhang-classifier-default` | 1 | 197,541,669 | 0.017 |
| `com.core.machine-gcode-emit` | 1 | 112,515,261 | 0.01 |
| `com.core.traditional-support` | 240 | 98,699,198 | 0.009 |
| `com.core.wave-overhangs` | 240 | 92,047,747 | 0.008 |
| `com.core.gyroid-infill` | 240 | 86,859,724 | 0.008 |
| `com.core.lightning-infill` | 240 | 85,347,004 | 0.008 |
| `com.core.support-surface-ironing` | 240 | 84,683,866 | 0.007 |
| `com.core.top-surface-ironing` | 240 | 84,318,662 | 0.007 |
| `com.core.tree-support-planner` | 1 | 32,243,380 | 0.003 |
| `com.core.traditional-support-planner` | 1 | 32,221,338 | 0.003 |
| `com.core.layer-planner-default` | 1 | 634,951 | 0.0 |

Top guest scopes (self_fuel):

| module | scope | calls | self_fuel | % of module total_fuel |
| --- | --- | ---: | ---: | ---: |
| `com.core.classic-perimeters` | `polygon_ops::offset2_ex` | 960 | 172,131,023,924 | 16.995 |
| `com.core.infill-linker` | `polygon_ops::clip_polygons` | 309 | 190,203,119 | 0.163 |

##### Host wall ranking (share of wall_total_ns)

| module | calls | total_wall_ns | % of wall_total |
| --- | ---: | ---: | ---: |
| `host:slice` | 5921 | 24,663,980,400 | 45.429 |
| `host:native` | 11193 | 11,329,371,300 | 20.868 |
| `host:shell_classification` | 3910 | 10,202,623,900 | 18.792 |
| `host:overhang_annotation` | 3107 | 7,962,787,500 | 14.667 |
| `host:mesh_analysis` | 343 | 132,237,500 | 0.244 |

Top host scopes (self_wall_ns):

| module | scope | calls | self_wall_ns | % of module total_wall_ns |
| --- | --- | ---: | ---: | ---: |
| `host:slice` | `polygon_ops::closing_ex` | 240 | 21,281,525,900 | 86.286 |
| `host:slice` | `polygon_ops::clip_polygons` | 5005 | 1,878,036,100 | 7.614 |
| `host:slice` | `polygon_ops::offset` | 676 | 1,504,262,700 | 6.099 |
| `host:native` | `polygon_ops::offset` | 2904 | 6,386,535,000 | 56.371 |
| `host:native` | `polygon_ops::clip_polygons` | 8261 | 4,922,242,800 | 43.447 |
| `host:native` | `polygon_ops::closing_ex` | 27 | 15,288,000 | 0.135 |
| `host:native` | `polygon_ops::opening` | 1 | 5,011,100 | 0.044 |
| `host:shell_classification` | `polygon_ops::offset` | 1689 | 8,965,429,300 | 87.874 |
| `host:shell_classification` | `polygon_ops::clip_polygons` | 2052 | 991,284,800 | 9.716 |
| `host:shell_classification` | `polygon_ops::closing_ex` | 169 | 245,808,500 | 2.409 |
| `host:overhang_annotation` | `polygon_ops::offset` | 717 | 5,728,510,600 | 71.941 |
| `host:overhang_annotation` | `polygon_ops::clip_polygons` | 2390 | 2,234,188,500 | 28.058 |
| `host:mesh_analysis` | `polygon_ops::clip_polygons` | 343 | 132,229,700 | 99.994 |

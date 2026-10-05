# t15 Baselines — criterion on-disk estimates at HEAD `3f51b7b9`

Machine-readable twin: [`baselines.json`](baselines.json) (per-leaf means, medians,
CIs, std-dev, outlier counts, sample sizes, iteration counts, sampling mode).

Source of truth: `target/criterion/**/new/estimates.json` (+ `sample.json`,
`tukey.json`, `benchmark.json`), read back from disk after each run. **The criterion
console's `time:` line is not the mean** — in Linear sampling mode it prints the
regression *slope* (`Estimates::typical` prefers `slope`), which diverges from the
JSON mean by up to +8.61% here. Both are on disk; this file quotes `mean` and
`median` from `estimates.json` and treats slope-vs-mean divergence as a caveat, not
a defect.

Captured 2026-09-23, one sequential run per bench binary, no concurrent load in
this session. 82 benchmark leaves across 7 bench binaries.

Legend: **mean** / **median** from `estimates.json`; **95% CI** on the mean;
**CV** = `std_dev`/`mean`; **outliers** recomputed from the raw per-sample times
using criterion's own Tukey fences (lm/ls/hm/hs = low-mild / low-severe /
high-mild / high-severe; **the recomputation was cross-checked against the
console's own `Found N outliers` blocks and matches on all 77 leaves that
printed one — the other 5 print no block because they have zero outliers**);
**n** = samples; **iters** = criterion's per-sample batch iteration count
**(iters)** = criterion's per-sample batch iteration count
(Linear mode ramps it, Flat mode holds it constant). One iteration is not
always one operation: `wasm_modules/export_name_for_stage`'s body loops over 13
stage-id strings, so its 6.73 ns is 13 lookups (see FINDINGS.md #4).

## `polygon_ops`

| benchmark | mean | median | 95% CI (mean) | CV | outliers | n | iters | mode |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `polygon_ops/difference/16` | 8.598 µs | 8.574 µs | 8.563 µs – 8.635 µs | 2.14% | 9 (4lm/0ls/3hm/2hs) | 100 | 111–11100 | Linear |
| `polygon_ops/difference/256` | 156.355 µs | 154.987 µs | 155.162 µs – 157.803 µs | 4.35% | 9 (3lm/0ls/3hm/3hs) | 100 | 7–700 | Linear |
| `polygon_ops/difference/64` | 32.741 µs | 32.639 µs | 32.606 µs – 32.884 µs | 2.19% | 8 (3lm/0ls/4hm/1hs) | 100 | 31–3100 | Linear |
| `polygon_ops/intersection/16` | 9.209 µs | 8.988 µs | 9.104 µs – 9.321 µs | 6.05% | 2 (0lm/0ls/2hm/0hs) | 100 | 112–11200 | Linear |
| `polygon_ops/intersection/256` | 148.828 µs | 147.440 µs | 147.745 µs – 149.997 µs | 3.87% | 14 (2lm/0ls/6hm/6hs) | 100 | 7–700 | Linear |
| `polygon_ops/intersection/64` | 34.172 µs | 33.486 µs | 33.827 µs – 34.552 µs | 5.46% | 12 (0lm/0ls/7hm/5hs) | 100 | 30–3000 | Linear |
| `polygon_ops/offset/16` | 10.862 µs | 10.669 µs | 10.740 µs – 10.996 µs | 6.04% | 7 (0lm/0ls/6hm/1hs) | 100 | 94–9400 | Linear |
| `polygon_ops/offset/256` | 242.520 µs | 246.712 µs | 234.625 µs – 250.599 µs | 16.84% | 1 (0lm/0ls/1hm/0hs) | 100 | 5–500 | Linear |
| `polygon_ops/offset/64` | 44.275 µs | 42.952 µs | 43.552 µs – 45.061 µs | 8.74% | 8 (0lm/0ls/7hm/1hs) | 100 | 23–2300 | Linear |
| `polygon_ops/union/16` | 8.488 µs | 8.443 µs | 8.392 µs – 8.592 µs | 6.02% | 3 (0lm/0ls/2hm/1hs) | 100 | 117–11700 | Linear |
| `polygon_ops/union/256` | 199.287 µs | 193.423 µs | 192.757 µs – 205.842 µs | 16.87% | 0 (0lm/0ls/0hm/0hs) | 100 | 6–600 | Linear |
| `polygon_ops/union/64` | 35.026 µs | 34.625 µs | 34.654 µs – 35.503 µs | 6.31% | 8 (1lm/0ls/3hm/4hs) | 100 | 29–2900 | Linear |

## `mesh_ops`

| benchmark | mean | median | 95% CI (mean) | CV | outliers | n | iters | mode |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `mesh_ops/decimate/cube_default` | 3.745 µs | 3.693 µs | 3.707 µs – 3.795 µs | 6.02% | 5 (1lm/0ls/2hm/2hs) | 100 | 236–23600 | Linear |
| `mesh_ops/import_step/assembly` | 2.1415 ms | 2.1293 ms | 2.1291 ms – 2.1557 ms | 3.18% | 8 (0lm/0ls/4hm/4hs) | 100 | 18 | Flat |
| `mesh_ops/import_step/cube` | 1.4445 ms | 1.4496 ms | 1.4043 ms – 1.4849 ms | 14.37% | 0 (0lm/0ls/0hm/0hs) | 100 | 1–100 | Linear |
| `mesh_ops/repair/cube` | 2.947 µs | 2.903 µs | 2.916 µs – 2.987 µs | 6.31% | 8 (0lm/0ls/5hm/3hs) | 100 | 292–29200 | Linear |

## `pipeline`

| benchmark | mean | median | 95% CI (mean) | CV | outliers | n | iters | mode |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `pipeline/allocator_fast_path/string_alloc_short` | 67.869 ns | 68.169 ns | 66.429 ns – 69.272 ns | 10.77% | 0 (0lm/0ls/0hm/0hs) | 100 | 18139–1813900 | Linear |
| `pipeline/allocator_fast_path/vec_push_1k` | 1.063 µs | 1.059 µs | 1.057 µs – 1.069 µs | 2.90% | 9 (1lm/0ls/7hm/1hs) | 100 | 947–94700 | Linear |
| `pipeline/instrumentation_collector/10` | 19.319 µs | 19.313 µs | 19.171 µs – 19.480 µs | 4.11% | 6 (2lm/0ls/2hm/2hs) | 100 | 52–5200 | Linear |
| `pipeline/instrumentation_collector/100` | 190.383 µs | 190.307 µs | 188.695 µs – 192.293 µs | 4.85% | 6 (3lm/0ls/1hm/2hs) | 100 | 6–600 | Linear |
| `pipeline/instrumentation_collector/1000` | 1.8503 ms | 1.8390 ms | 1.8384 ms – 1.8632 ms | 3.44% | 8 (2lm/0ls/3hm/3hs) | 100 | 1–100 | Linear |
| `pipeline/instrumentation_noop/10` | 1.599 µs | 1.584 µs | 1.581 µs – 1.618 µs | 6.00% | 3 (0lm/0ls/3hm/0hs) | 100 | 666–66600 | Linear |
| `pipeline/instrumentation_noop/100` | 16.836 µs | 16.242 µs | 16.556 µs – 17.134 µs | 8.81% | 16 (0lm/0ls/7hm/9hs) | 100 | 61–6100 | Linear |
| `pipeline/instrumentation_noop/1000` | 160.458 µs | 158.577 µs | 158.620 µs – 162.448 µs | 6.12% | 5 (0lm/0ls/3hm/2hs) | 100 | 7–700 | Linear |

## `per_stage`

| benchmark | mean | median | 95% CI (mean) | CV | outliers | n | iters | mode |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `per_stage/compute_serial_edges/128` | 97.255 µs | 95.244 µs | 96.001 µs – 98.632 µs | 6.92% | 6 (0lm/0ls/4hm/2hs) | 100 | 11–1100 | Linear |
| `per_stage/compute_serial_edges/32` | 8.317 µs | 8.231 µs | 8.242 µs – 8.406 µs | 5.05% | 6 (1lm/0ls/1hm/4hs) | 100 | 116–11600 | Linear |
| `per_stage/compute_serial_edges/8` | 942.580 ns | 939.955 ns | 937.220 ns – 948.072 ns | 2.95% | 11 (5lm/0ls/5hm/1hs) | 100 | 1110–111000 | Linear |

## `wasm_modules`

| benchmark | mean | median | 95% CI (mean) | CV | outliers | n | iters | mode |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `wasm_modules/compile_component/arachne-perimeters` | 49.2296 ms | 48.2032 ms | 48.3737 ms – 50.1597 ms | 9.26% | 10 (0lm/0ls/8hm/2hs) | 100 | 1 | Flat |
| `wasm_modules/compile_component/classic-perimeters` | 49.1592 ms | 49.0134 ms | 48.5852 ms – 49.7542 ms | 6.06% | 3 (0lm/0ls/3hm/0hs) | 100 | 2 | Flat |
| `wasm_modules/compile_component/fuzzy-skin` | 18.7863 ms | 18.8568 ms | 18.5517 ms – 19.0339 ms | 6.58% | 3 (1lm/0ls/1hm/1hs) | 100 | 3 | Flat |
| `wasm_modules/compile_component/gyroid-infill` | 28.2719 ms | 28.1750 ms | 27.8879 ms – 28.6725 ms | 7.12% | 1 (0lm/0ls/1hm/0hs) | 100 | 2 | Flat |
| `wasm_modules/compile_component/infill-linker` | 53.1212 ms | 52.9641 ms | 51.9851 ms – 54.2528 ms | 10.91% | 17 (11lm/0ls/5hm/1hs) | 100 | 1 | Flat |
| `wasm_modules/compile_component/layer-planner-default` | 9.1116 ms | 9.0012 ms | 8.9939 ms – 9.2664 ms | 7.77% | 3 (0lm/0ls/1hm/2hs) | 100 | 6 | Flat |
| `wasm_modules/compile_component/lightning-infill` | 21.7737 ms | 21.6823 ms | 21.5657 ms – 21.9888 ms | 4.98% | 1 (0lm/0ls/1hm/0hs) | 100 | 3 | Flat |
| `wasm_modules/compile_component/machine-gcode-emit` | 21.1208 ms | 20.9589 ms | 20.9280 ms – 21.3211 ms | 4.77% | 4 (0lm/0ls/4hm/0hs) | 100 | 3 | Flat |
| `wasm_modules/compile_component/overhang-classifier-default` | 17.3954 ms | 16.8407 ms | 17.0284 ms – 17.8072 ms | 11.53% | 9 (0lm/0ls/6hm/3hs) | 100 | 3 | Flat |
| `wasm_modules/compile_component/part-cooling` | 13.0088 ms | 12.5740 ms | 12.6962 ms – 13.4010 ms | 13.93% | 3 (0lm/0ls/2hm/1hs) | 100 | 4 | Flat |
| `wasm_modules/compile_component/path-optimization-default` | 15.6438 ms | 15.5578 ms | 15.4404 ms – 15.8505 ms | 6.75% | 1 (0lm/0ls/1hm/0hs) | 100 | 4 | Flat |
| `wasm_modules/compile_component/raft-default` | 23.0891 ms | 22.6194 ms | 22.6450 ms – 23.6131 ms | 10.77% | 6 (0lm/0ls/4hm/2hs) | 100 | 3 | Flat |
| `wasm_modules/compile_component/rectilinear-infill` | 24.1930 ms | 23.7165 ms | 23.7417 ms – 24.6682 ms | 9.82% | 5 (0lm/0ls/5hm/0hs) | 100 | 2 | Flat |
| `wasm_modules/compile_component/seam-placer` | 20.1700 ms | 20.0781 ms | 19.9076 ms – 20.4534 ms | 6.94% | 1 (0lm/0ls/0hm/1hs) | 100 | 3 | Flat |
| `wasm_modules/compile_component/seam-planner-default` | 16.1260 ms | 16.0882 ms | 15.9694 ms – 16.2931 ms | 5.14% | 3 (1lm/0ls/1hm/1hs) | 100 | 3 | Flat |
| `wasm_modules/compile_component/skirt-brim` | 12.0026 ms | 11.8487 ms | 11.8751 ms – 12.1410 ms | 5.69% | 7 (0lm/0ls/4hm/3hs) | 100 | 5 | Flat |
| `wasm_modules/compile_component/support-surface-ironing` | 18.8864 ms | 18.9430 ms | 18.7101 ms – 19.0591 ms | 4.75% | 2 (2lm/0ls/0hm/0hs) | 100 | 3 | Flat |
| `wasm_modules/compile_component/top-surface-ironing` | 21.6832 ms | 21.2367 ms | 21.3370 ms – 22.0370 ms | 8.28% | 0 (0lm/0ls/0hm/0hs) | 100 | 3 | Flat |
| `wasm_modules/compile_component/traditional-support` | 40.1228 ms | 40.2229 ms | 39.6416 ms – 40.6210 ms | 6.21% | 1 (0lm/0ls/1hm/0hs) | 100 | 2 | Flat |
| `wasm_modules/compile_component/traditional-support-planner` | 46.9054 ms | 46.5949 ms | 46.3456 ms – 47.4856 ms | 6.25% | 2 (0lm/0ls/2hm/0hs) | 100 | 2 | Flat |
| `wasm_modules/compile_component/tree-support` | 43.5449 ms | 43.4929 ms | 43.0775 ms – 44.0355 ms | 5.60% | 5 (2lm/0ls/2hm/1hs) | 100 | 2 | Flat |
| `wasm_modules/compile_component/tree-support-planner` | 96.9733 ms | 95.3762 ms | 95.3666 ms – 98.6378 ms | 8.69% | 2 (0lm/0ls/2hm/0hs) | 100 | 1 | Flat |
| `wasm_modules/compile_component/wave-overhangs` | 91.0594 ms | 89.3941 ms | 88.9223 ms – 93.3534 ms | 12.48% | 3 (0lm/0ls/1hm/2hs) | 100 | 1 | Flat |
| `wasm_modules/compile_component/wipe-tower` | 17.9135 ms | 17.7423 ms | 17.6566 ms – 18.1857 ms | 7.56% | 2 (0lm/0ls/2hm/0hs) | 100 | 3 | Flat |
| `wasm_modules/export_name_for_stage` | 6.734 ns | 6.713 ns | 6.693 ns – 6.776 ns | 3.19% | 6 (3lm/0ls/1hm/2hs) | 100 | 150714–15071400 | Linear |
| `wasm_modules/manifest_load/arachne-perimeters` | 400.621 µs | 395.406 µs | 395.923 µs – 406.018 µs | 6.45% | 9 (3lm/0ls/3hm/3hs) | 100 | 3–300 | Linear |
| `wasm_modules/manifest_load/classic-perimeters` | 322.380 µs | 318.756 µs | 318.619 µs – 326.266 µs | 6.10% | 3 (1lm/0ls/2hm/0hs) | 100 | 3–300 | Linear |
| `wasm_modules/manifest_load/fuzzy-skin` | 102.169 µs | 100.991 µs | 101.084 µs – 103.388 µs | 5.81% | 6 (0lm/0ls/3hm/3hs) | 100 | 10–1000 | Linear |
| `wasm_modules/manifest_load/gyroid-infill` | 139.763 µs | 138.514 µs | 137.928 µs – 141.719 µs | 6.97% | 6 (0lm/0ls/5hm/1hs) | 100 | 7–700 | Linear |
| `wasm_modules/manifest_load/infill-linker` | 125.159 µs | 121.547 µs | 122.511 µs – 128.060 µs | 11.43% | 12 (1lm/0ls/3hm/8hs) | 100 | 9–900 | Linear |
| `wasm_modules/manifest_load/layer-planner-default` | 111.369 µs | 107.020 µs | 109.124 µs – 113.759 µs | 10.67% | 1 (0lm/0ls/1hm/0hs) | 100 | 10–1000 | Linear |
| `wasm_modules/manifest_load/lightning-infill` | 119.231 µs | 118.305 µs | 118.031 µs – 120.533 µs | 5.38% | 4 (0lm/0ls/2hm/2hs) | 100 | 9–900 | Linear |
| `wasm_modules/manifest_load/machine-gcode-emit` | 151.914 µs | 151.010 µs | 150.206 µs – 153.712 µs | 5.94% | 8 (2lm/0ls/4hm/2hs) | 100 | 7–700 | Linear |
| `wasm_modules/manifest_load/overhang-classifier-default` | 137.002 µs | 136.636 µs | 135.780 µs – 138.239 µs | 4.59% | 3 (1lm/0ls/2hm/0hs) | 100 | 8–800 | Linear |
| `wasm_modules/manifest_load/part-cooling` | 132.085 µs | 132.334 µs | 130.786 µs – 133.480 µs | 5.19% | 9 (3lm/2ls/2hm/2hs) | 100 | 8–800 | Linear |
| `wasm_modules/manifest_load/path-optimization-default` | 109.639 µs | 108.517 µs | 108.546 µs – 110.778 µs | 5.21% | 9 (1lm/0ls/8hm/0hs) | 100 | 9–900 | Linear |
| `wasm_modules/manifest_load/raft-default` | 102.871 µs | 102.861 µs | 101.510 µs – 104.257 µs | 6.86% | 7 (2lm/0ls/3hm/2hs) | 100 | 10–1000 | Linear |
| `wasm_modules/manifest_load/rectilinear-infill` | 212.905 µs | 208.598 µs | 210.310 µs – 215.787 µs | 6.60% | 10 (0lm/0ls/7hm/3hs) | 100 | 5–500 | Linear |
| `wasm_modules/manifest_load/seam-placer` | 85.899 µs | 85.779 µs | 85.195 µs – 86.642 µs | 4.31% | 2 (0lm/0ls/2hm/0hs) | 100 | 12–1200 | Linear |
| `wasm_modules/manifest_load/seam-planner-default` | 88.715 µs | 88.318 µs | 87.627 µs – 89.862 µs | 6.44% | 14 (5lm/0ls/4hm/5hs) | 100 | 12–1200 | Linear |
| `wasm_modules/manifest_load/skirt-brim` | 114.996 µs | 114.729 µs | 114.118 µs – 115.902 µs | 3.99% | 4 (0lm/0ls/4hm/0hs) | 100 | 9–900 | Linear |
| `wasm_modules/manifest_load/support-surface-ironing` | 107.222 µs | 106.203 µs | 106.103 µs – 108.537 µs | 5.81% | 3 (0lm/0ls/2hm/1hs) | 100 | 9–900 | Linear |
| `wasm_modules/manifest_load/top-surface-ironing` | 110.181 µs | 110.044 µs | 108.935 µs – 111.595 µs | 6.19% | 3 (0lm/0ls/1hm/2hs) | 100 | 9–900 | Linear |
| `wasm_modules/manifest_load/traditional-support` | 130.810 µs | 129.966 µs | 129.473 µs – 132.230 µs | 5.37% | 11 (4lm/0ls/6hm/1hs) | 100 | 8–800 | Linear |
| `wasm_modules/manifest_load/traditional-support-planner` | 165.323 µs | 164.673 µs | 163.516 µs – 167.265 µs | 5.81% | 8 (1lm/0ls/6hm/1hs) | 100 | 6–600 | Linear |
| `wasm_modules/manifest_load/tree-support` | 127.751 µs | 126.803 µs | 126.592 µs – 128.968 µs | 4.76% | 1 (0lm/0ls/0hm/1hs) | 100 | 8–800 | Linear |
| `wasm_modules/manifest_load/tree-support-planner` | 244.846 µs | 238.146 µs | 241.195 µs – 248.889 µs | 8.05% | 13 (0lm/0ls/5hm/8hs) | 100 | 5–500 | Linear |
| `wasm_modules/manifest_load/wave-overhangs` | 209.582 µs | 197.653 µs | 204.502 µs – 214.949 µs | 12.79% | 1 (0lm/0ls/1hm/0hs) | 100 | 6–600 | Linear |
| `wasm_modules/manifest_load/wipe-tower` | 154.039 µs | 159.350 µs | 150.545 µs – 157.461 µs | 11.54% | 0 (0lm/0ls/0hm/0hs) | 100 | 7–700 | Linear |

## `shell_classification`

| benchmark | mean | median | 95% CI (mean) | CV | outliers | n | iters | mode |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `shell_classification/objs=16_layers=120` | 49.0620 ms | 48.3877 ms | 47.8768 ms – 50.8014 ms | 5.28% | 1 (0lm/0ls/0hm/1hs) | 10 | 2–20 | Linear |
| `shell_classification/objs=16_layers=480` | 131.7252 ms | 130.4851 ms | 130.0956 ms – 133.9221 ms | 2.50% | 1 (0lm/0ls/0hm/1hs) | 10 | 1–10 | Linear |
| `shell_classification/objs=1_layers=120` | 2.5453 ms | 2.5314 ms | 2.5290 ms – 2.5653 ms | 1.23% | 2 (0lm/0ls/2hm/0hs) | 10 | 32–320 | Linear |
| `shell_classification/objs=1_layers=240` | 3.9181 ms | 3.8765 ms | 3.8551 ms – 3.9980 ms | 3.17% | 2 (0lm/0ls/0hm/2hs) | 10 | 20–200 | Linear |
| `shell_classification/objs=1_layers=480` | 7.1503 ms | 7.1271 ms | 7.0771 ms – 7.2389 ms | 1.94% | 1 (0lm/0ls/1hm/0hs) | 10 | 11–110 | Linear |

## `gate_evidence`

| benchmark | mean | median | 95% CI (mean) | CV | outliers | n | iters | mode |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `gate_evidence/full_slice_50_layers` | 885.8680 ms | 890.0888 ms | 877.0772 ms – 893.0212 ms | 1.55% | 1 (1lm/0ls/0hm/0hs) | 10 | 1 | Flat |

## Totals

- 82 leaves; all have `base/` and `new/` on disk (first-run baseline,
  `base == new`).
- 433 outliers total, 130 of them high-severe; CV median 6.05%, max 16.87%.
- Sampling mode is criterion's own choice (`Linear` ramps `iters` to hit the target
  time; `Flat` holds it) — not a bench-author decision.

# t150 — the MeshAnalysis bridge product is retired

**Headline.** `PrePass::MeshAnalysis` no longer computes `bridge_regions`. The
stage drops from **4,445 ms to 732 ms (−3,713 ms, −83.5 %)** on base
classic-off and **4,616 → 582 ms (−87.4 %)** on base classic-on; the whole slice
gets **12.08 %** faster on base classic-off and **4.06 %** on base classic-on
(paired medians, 15.1x and 5.1x the 0.80 % floor), and the other six cells move
−1.77 % … −9.37 %. **All 16 output runs are byte-identical to the t148 active
reference set** in both arms, with zero diagnostics and zero exact-Z
rejections. This is a tranche, not a destination step: base classic-off's
indicative ratio moves **2.078x → 1.827x** and its required cut **51.88 % →
45.27 %**.

## 1. What changed

`classify_object` (`crates/slicer-core/src/algos/mesh_analysis.rs`) no longer
calls `compute_bridge_metrics`; it returns `bridge_regions: Vec::new()`. The IR
type, field and schema version are unchanged — an empty vector is the same
schema, exactly as t142 did for `overhang_quartile_polygons`.

Deleted because nothing else reached them:

- `find_bridge_clusters`, `build_half_edge_map`, `edge_key`, `DirectedEdge`,
  `HalfEdgeEntry`, `BridgeCluster`, `is_bridge_candidate`
- `build_anchor_runs`, `AnchorRun`, `metrics_projection_axis_deg`,
  `compute_anchor_width_mm`, `compute_bridge_length_mm`, `compute_bridge_metrics`
- the `facet_normals` vector in `classify_object` (only the clusterer read it)

Kept, per the ticket and t149's reader census:

- `compute_xy_footprint`, `union_footprint_partitioned`, `expolygon_bbox`,
  `bboxes_overlap`, `facet_xy_bbox`, `footprints_overlap`
  (`overhang_regions[].xy_footprint` is the only consumed output)
- `classify_facet`, `region_needs_support`, the `OverhangRegion` construction
- `facet_classes` and `surface_groups` — equally unread, but they cost memory,
  not the measured wall; dropping them is a separate decision

`MeshAnalysisConfig`'s three bridge fields (`min_bridge_length_mm`,
`anchor_width_mm`, `expansion_margin_mm`) are retained so callers constructing
the struct keep compiling, with doc comments that state they no longer affect
the output. `classify_object`'s now-unused `config` parameter was dropped.

**Diagnostic surfaces.** `visual_debug_render.rs` still renders
`bridge_regions` and now draws nothing there;
`estimated_surface_classification_bytes` (`crates/slicer-runtime/src/blackboard.rs`)
no longer adds per-region footprint sizes, so the blackboard's surface estimate
drops by that term. This is a diagnostic disclosure, not a G-code change.

**Correction to the ticket's item 4.** The ticket expected the
`whole-print-input-path` test guest's `bridges=<n>` observation to become
`bridges=0`. It does not: that guest is fed a **literal** `SurfaceClassificationIR`
(`Product::Literal` in `crates/slicer-wasm-host/tests/contract/whole_print_input_driver.rs`,
`literal_surface`), which seeds one synthetic `BridgeRegion` to exercise the
transport path. That fixture is independent of mesh analysis, and its
`bridges=1` assertion still describes what it feeds. No change was made there;
the field still exists in the IR and still needs transport coverage.

## 2. Identity — 16/16 runs, 8/8 cells

One run per (cell, arm), `RAYON_NUM_THREADS=12`, one shared module tree
(`cargo xtask build-guests --check` exits 0). Baseline is the pre-change tree
(HEAD, built from a stash of the two candidate files); candidate is this
ticket's change. Every hash equals
[`../t148-print-plane-carve/reference.json`](../t148-print-plane-carve/reference.json):

| cell | sha256 (both arms, all A/B runs) | g-code bytes |
| --- | --- | ---: |
| benchy classic-off | `1ae0cfa84d1b…` | 4,085,300 |
| benchy classic-on | `37788518229d…` | 8,673,104 |
| benchy arachne-off | `8faf02b1b473…` | 4,227,096 |
| benchy arachne-on | `f86d88975725…` | 8,815,235 |
| base classic-off | `c0b9014f8eff…` | 21,186,923 |
| base classic-on | `eccbd5238d2d…` | 39,732,305 |
| base arachne-off | `17f63e9387bd…` | 21,046,723 |
| base arachne-on | `1811425ffccd…` | 39,594,635 |

Every A/B pair in §4 also carries the same hash on both arms. `degraded:
false` and **zero** `exact-Z occupancy` lines in all 16 identity runs and all
54 A/B runs (27 pairs: the pre-declared 24 plus base classic-on's three extra
pairs) and the 12 null-control runs.

## 3. Stage attribution

`--instrument-stderr` runs, one per arm, ordinary build. These are stage
readings, not wall claims.

| cell | baseline MeshAnalysis | candidate | delta |
| --- | ---: | ---: | ---: |
| base classic-off | 4,445 ms | **732 ms** | −3,713 ms (−83.53 %) |
| base classic-on | 4,616 ms | **582 ms** | −4,034 ms (−87.39 %) |
| benchy classic-off | 168 ms | **95 ms** | −73 ms (−43.45 %) |

t149 predicted the stage would fall to 762.6 ms on base classic-off after the
unread half went; the candidate measures **732 ms**, 4 % under that prediction.
The stage saving is **13.48 %** of the 27.54 s base classic-off identity wall,
against t149's 12.95 % share for the bridge half alone.

## 4. Paired A/B

Interleaved pairs per cell, order alternating by pair parity, one discarded
baseline warmup per cell. Wall is process creation-to-exit; CPU is kernel+user
from the process handle. Negative values mean the candidate is faster.

| cell | pairs | pair deltas | median wall | floor | median CPU |
| --- | ---: | --- | ---: | ---: | ---: |
| base classic-off | 3 | −13.69 / −12.08 / −5.31 % | **−12.08 %** | **15.09x** | −1.77 % |
| base classic-on | 6 | −3.74 / −11.90 / −2.37 / −0.24 / −4.39 / −5.23 % | **−4.06 %** | **5.08x** | −0.10 % |
| base arachne-off | 3 | −12.44 / −5.33 / −9.37 % | −9.37 % | 11.72x | −1.67 % |
| base arachne-on | 3 | −3.19 / −14.64 / −5.78 % | −5.78 % | 7.22x | −0.46 % |
| benchy classic-off | 3 | −1.77 / +5.28 / −5.09 % | −1.77 % | 2.21x | −0.41 % |
| benchy classic-on | 3 | −5.97 / +3.16 / −4.95 % | −4.95 % | 6.19x | −1.29 % |
| benchy arachne-off | 3 | −0.07 / −4.87 / −7.47 % | −4.87 % | 6.09x | −0.32 % |
| benchy arachne-on | 3 | −6.25 / +4.01 / −3.20 % | −3.20 % | 4.00x | −0.19 % |

**Null control (identical arm, candidate vs candidate, base classic-off, 6
pairs):** wall deltas −3.67 / −2.37 / +1.62 / +2.20 / +1.62 / +0.51 % —
**band [−3.67 %, +2.20 %], median +1.06 %**; CPU deltas −0.17 / +0.02 / +0.50 /
−0.11 / +0.21 / +0.44 % — **band [−0.17 %, +0.50 %]**.

Read against that band:

- **base classic-off's 12.08 % is 3.3x the null band's worst wall delta**, and
  every pair is candidate-faster.
- **base classic-on's 4.06 % clears the floor by 5.08x but is only 1.1x the
  null band's worst wall delta.** All six pairs are candidate-faster, including
  the −0.24 % one. This is the weakest cell of the take and is disclosed as
  such.
- **benchy classic-off's −1.77 % is inside the null wall band.** The ticket's
  ceiling for that cell was ~95 ms of ~3.7 s; it is reported, not claimed.
- CPU is **lower or flat in every cell** (−1.77 … −0.10 %), so the CPU clause
  is not contradicted anywhere. No CPU exception was invented after the fact:
  the band above is measured, and the strict "CPU lower than the null band's
  lower edge" form holds in 7/8 cells; base classic-on (−0.10 %) is flat inside
  it.

## 5. Route arithmetic (indicative)

Paired wall deltas applied to the ratios of record (supports-off from t145,
supports-on from t148). The ratio is measured in another window, so this is a
composition, **not** a re-taken scoreboard.

| cell | ratio of record | post-t150 | required cut |
| --- | ---: | ---: | ---: |
| base classic-off | 2.078x | **1.827x** | 51.88 % → **45.27 %** |
| base classic-on | 3.356x | 3.220x | 70.20 % → 68.94 % |
| base arachne-off | 1.984x | 1.798x | 49.60 % → 44.38 % |
| base arachne-on | 3.248x | 3.060x | 69.21 % → 67.32 % |
| benchy classic-off | 2.043x | 2.007x | 51.05 % → 50.17 % |
| benchy classic-on | 2.705x | 2.571x | 63.03 % → 61.11 % |
| benchy arachne-off | 1.853x | 1.763x | 46.03 % → 43.27 % |
| benchy arachne-on | 2.829x | 2.739x | 64.65 % → 63.48 % |

The 8 of 8 destination is unchanged. Every supports-on cell is still ≥ 61 % of
its wall to cut; the two benchy supports-off cells remain the nearest, at
~50 %.

## 6. Test retirement

`crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` loses its mesh-analysis
half — AC-1 … AC-6 and NEG-3, eight cases whose only subject was
`bridge_regions`. The file header now states the justification per
`docs/22_test_quality.md` (the product is gone; deleting the assertion and
keeping the name would have been the failure mode). The four remaining cases
drive `execute_prepass_slice_single_layer_with_cache`, whose whole-set
`diff(this, previous)` bridge detection is a **different, live** product.

Pruned with them: the `make_rotated_bridge_mesh` wrapper, the
`make_topfacing_only_mesh` fixture, the `expoly_bbox_mm` helper, and the
`WallPlacement` enum; the surviving builder is `bridge_mesh_with_l_walls`. The
file-level `#![allow(dead_code)]` was removed, so rot of that kind cannot hide
again.

`algo_mesh_analysis_tdd` needed no retirement: the ticket expected bridge
assertions there, but the file has none (its subject is facet classification,
`needs_support`, determinism and the config default).

## 7. Disclosures

- **One box, one window, load not measured.** 12 logical cores,
  `RAYON_NUM_THREADS=12`. No idle-box claim is made; the identical-arm null
  control above is the only noise bound quoted.
- **The accelerated timing arm is not taken** — `cargo xtask dist
  --accelerated` still fails in `libmimalloc-sys` (unchanged since t142), and a
  manual-cfg host is barred as a timing artifact by `docs/23`. Output identity
  is measured on the ordinary arm only.
- **base classic-on was extended from 3 to 6 pairs.** Its 3-pair median (4.67x
  the floor) sat only 1.1x outside the null band's worst wall delta, so four
  more pairs were taken and **all six are retained**, including the −0.24 %
  pair. The null control was extended to six pairs for the same reason.
- **Single-run prepass totals do not equal the stage delta.** In the
  instrumented pair, base classic-off's prepass goes 20,536 → 18,076 ms
  (−2,460 ms) while the stage falls 3,713 ms; other prepass stages absorb part
  of the difference in one run. The paired A/B medians, not those single runs,
  are the wall evidence.
- **The stage saving exceeds the measured wall saving on base classic-off**
  (3,713 ms = 13.48 % of 27.54 s against a 12.08 % paired median). Both are
  reported; neither is presented as the other.
- **`bridges=0` in the whole-print-input guest was not delivered**, because
  that guest is fed a literal surface product (§1). The visual-debug surface
  does go quiet, and that is recorded here rather than tested.
- **Ratios are compositions.** §5 applies this take's paired deltas to
  cross-window ratios of record; it is not a scoreboard re-derivation and
  carries no Orca arm.

## 8. Evidence index

- `census.json` — the 16 identity rows, every A/B pair, the six null-control
  pairs, the stage readings, the route composition and the disclosures.
- `reduce_t150.py` — regenerates `census.json` from the gitignored campaign
  CSV tree.
- `verify.py` — 70 checks re-derive every published number from `census.json`;
  `--self-test` rejects 5/5 negative controls (flipped hash, moved pair,
  null outlier, stage time, composed ratio).
- `run_t150.ps1` — the campaign driver (`-Mode output`, `-Mode ab`,
  `-Mode null`, `-Append`, `-PairFrom`/`-PairTo`).
- `REPRODUCE.md` — build, arms, campaign commands and the verification step.

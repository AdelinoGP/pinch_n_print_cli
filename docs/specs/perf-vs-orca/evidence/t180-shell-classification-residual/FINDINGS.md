# Ticket 180 — `ShellClassification` Post-t172 Residual Attribution and Speedup Screen

**Date:** 2026-11-22
**Measured HEAD:** `7b82f188` (code equivalent to post-t175 `5b138f5c`; zero production code edits landed in this take; observation-only probe saved in [`probe.patch`](probe.patch) and cleanly reverted; both ordinary and accelerated guest freshness gates exit `0`)
**Verifier:** `python verify.py` (`VERIFY PASS: 7/7 check groups green`) and `python verify.py --self-test` (`SELF-TEST PASS: 15/15 negative controls rejected`) — Python 3 stdlib only, strictly from committed files under `docs/specs/perf-vs-orca/evidence/`.

---

## 1. Executive Summary

After `t166` / `t169` / `t172` brought Clipper2 per-call offset cost close to canonical C++, `PrePass::ShellClassification` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs::commit_shell_classification_builtin`) remains the **#1 single stage on every supports-off cell** (`27.88 / 27.03 %` on `benchy`, `36.96 / 34.22 %` on `base`) and **`12.45–13.85 %`** on every supports-on cell, with a puzzling **`4.50–4.57×` `benchy` vs `8.08–8.61×` `base` effective parallelism gap** in `t177`.

Our observation-only probe (`15/15` runs byte-identical to `t175/reference.json`, `3/3` `--instrument-stderr` runs `degraded = false`, `0` fatal/non-fatal/code-1200, `100.0 %` call-count reconciliation against `t177`'s `--profile` scopes on all 4 op kinds) and 12-thread exact IR replay (`240/240` `benchy` layers, `496/496` `base` layers) show **this stage is far from its canonical floor**:

1. **Why `benchy` stalls at `5.10×` (`4.57×` Clipper-only) while `base` runs at `8.91×` (`8.29–8.61×`):**
   - **30.19 % (`141.19 ms`) of `benchy` (`24.70 %` / `841.28 ms` of `base`) is strictly serial (`1.00×`):** dominated by `B5d_ib_phaseB_com` (`117.44 ms` = **`25.11 %`** of `benchy`; `558.53 ms` = **`16.40 %`** of `base`) — because `gate_internal_bridge_sites` runs the Phase-B commit loop serially despite `crates/slicer-core/src/algos/bridge_over_infill.rs::cluster_candidate_layers` already existing, re-runs full-layer `gather_areas_w_depth` on `None` layers, and burns `111.60 ms` (`benchy`) of unmarked pure-CPU `O(S \times A) + O(S^2)` work in `determine_bridging_angle` (`crates/slicer-core/src/algos/bridge_over_infill.rs`) — plus `B2b + B2d + B2e` serial Pass-2 shadow merges and `internal_solid_fill` diffs (`19.24 ms` = `4.11 %` `benchy`; `254.85 ms` = `7.48 %` `base`).
   - **One single layer (`L1`) monopolizes `B5b_ib_phaseA` on `benchy` (`128.89 ms` of `128.96 ms`, `2.57×` par):** `unsupported_span_areas` (`crates/slicer-core/src/algos/bridge_over_infill.rs`) runs a full-layer `+1.6 mm` dilation (`ib_unsup_solids_grow = 102.40 ms` on `L0`'s `#3DBenchy` recessed bottom lettering) and returns `0` polygons while `L1`'s `internal_solid_fill` is a tiny local strip (`21.96 %` of `lower_fills` / `26.11 %` of `lower_solids` vertices lie inside `internal_solid_fill`'s padded bbox on `benchy`, and only **`12.12 %` / `14.04 %`** on `base`!).
   - **`B5c_ib_phaseB_pre` throws away `53.50 %` (`benchy`) and `79.75 %` (`base`, `6,445.9 ms` thread!) of its work:** **`72.09 %` (`31/43` `benchy`)** and **`93.16 %` (`109/117` `base`)** of Phase-B precomputed layers return `None`, yet every one runs whole-layer `gather_areas_w_depth` and whole-layer `offset(&deep, +1.5s)` (`ib_comp_pos_offset = 4,016.4 ms` — the **#1 Clipper call site on `base`**!) *before* checking whether any surface qualifies via `qualify_internal_bridge_surface` or survives the already-windowed `-4.5s` erosion (`internal_bridge_offset_window`).
   - **Unmarked pure-CPU work invisible to `--profile`:** `281.83 ms` thread on `benchy` (`ib_ang_determine_deg` `111.60 ms`, `ext_bridge_clip_polylines` `90.20 ms`, `ext_bridge_score_cpu` `80.03 ms`) and `2,154.64 ms` thread on `base` (`ext_bridge_clip_polylines` `1,230.18 ms`, `ext_bridge_score_cpu` `834.09 ms`, `ib_ang_determine_deg` `90.37 ms`).
2. **A 100 % `EXACT_BIT_IDENTICAL` four-mechanism bundle (`EXACT_BUNDLE`: `E1 + E2 + E3 + E4a`) cuts stage wall by `−56.06 %` (`benchy`) and `−44.58 %` (`base`) with ZERO output changes across all 240 `benchy` and 496 `base` layers:**
   - `benchy`: **`470.77 → 206.83 ms` (`−263.93 ms`, `−56.06 %`)**, `EXACT_BIT_IDENTICAL`.
   - `base`: **`3,307.57 → 1,833.22 ms` (`−1,474.35 ms`, `−44.58 %`)**, `EXACT_BIT_IDENTICAL`.
   - Because `ShellClassification` call counts and vertex totals are **100 % invariant** across Classic vs Arachne and supports-off vs supports-on (verified on 4/4 pairs), `EXACT_BUNDLE` projects (`ESTIMATE ONLY`) to clear the literal standing **all-8-cell `0.80 %` gate at `6.94×–20.60×` floor** (`15.63 / 15.16 %` `benchy-off`, `7.76 / 7.53 %` `benchy-on`, `16.48 / 15.25 %` `base-off`, `5.55 / 5.74 %` `base-on`) with **zero G-code drift (`16/16` against `t175/reference.json`) and zero geometry exceptions**.
   - **Route impact (`ESTIMATE ONLY`, `t174 × t175 × t180`):** `EXACT_BUNDLE` **alone crosses `< 1.0x` on BOTH `benchy` supports-off cells** (`benchy-classic-off` `1.1438x → 0.9650x` ord / `0.9432x` acc; `benchy-arachne-off` `1.1548x → 0.9798x` ord / `0.9028x` acc), pushes `base` supports-off to `0.6420x / 0.6713x` ord, and moves binding `benchy-classic-on` `1.5700x → 1.4481x` (`1.2291x` stacked with `t183 + t182 + t181`).
3. **Sub-micron / parity candidates (`L1`, `L2`, `L3`, `E4b`) cleanly separated & falsified where unsafe:**
   - `E4b` (rect-clipping `B4` anchors), `L1` (`bbox <= 2r || area <= pi*r^2` sliver prefilter), and `L3` (prototype per-`ExPolygon` opening with `1× miter_limit` CC pad) were **falsified as exact** by replay (`89.9°` angle shift on 118/423 layers; `13.97 / 5.28 µm` Hausdorff; `499.2 / 213.3 µm` Hausdorff).
   - `L2` (canonical `ClipperUtils.cpp::offset_expolygon_inner`-style per-`ExPolygon` negative offset in Pass-2 `project_shadow`) cuts `B2p` by `~40 %` (`0.174 / 0.825 µm` sub-micron scanbeam shift), but its **marginal gain over `EXACT_BUNDLE` clears `0.80 %` in only `2/8` cells** (`0.37–0.76 %` on all four `benchy` cells and `0.67–0.69 %` on `base` supports-on) while breaking bit-identity.
4. **Graduation decision:** Graduate **one pure-exact implementation ticket** — [Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](../../issues/184-shell-classification-exact-acceleration.md) (`t184`, literal standing all-8-cell `0.80 %` paired ordinary + accelerated gate, `16/16` byte-identical to `t175/reference.json`, zero waivers, zero geometry exceptions). Park `L2` + fixed `L3` (canonical per-`ExPolygon` `offset_expolygon_inner` / `opening_ex`) cleanly in `map.md ## Not yet specified` for the follow-on `<= 0.50x` program.

---

## 2. Qualification, `t177` Reconciliation, and Invariance Proof

### 2.1 Capture Qualification (`reduced/probe.json` vs `t175/reference.json`)

| Metric | Result |
|---|---|
| Total probed production runs (`RAYON_NUM_THREADS=12`) | **15 / 15** (`exit 0`) |
| Byte-identical SHA-256 + byte length vs `t175/reference.json` | **15 / 15** (`100 %`) |
| `--instrument-stderr` qualification runs (`benchy-classic-off`, `benchy-classic-on`, `base-classic-off`) | **3 / 3** clean (`degraded = false`, `fatal = 0`, `non_fatal = 0`, `code-1200 = 0`) |
| Exact call-count & vertex-total invariance (`classic-off vs classic-on`, `classic vs arachne` on both models) | **4 / 4 pairs IDENTICAL (`0` differing sites)** |

### 2.2 `t177` 8-Cell Reconciliation (`reduced/arithmetic.json::t177_reconciliation`)

| Cell | `t177` Stage (ms) | `% Cell` | `% Prepass` | `--profile` Thread (s) | Calls | `prof/wall` | Post-t175 Ord (`EST`) | Ord If Zero (`EST`) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `benchy-classic-off` | 474 | 27.88 % | 50.59 % | 2.167 | 4,086 | 4.57× | 1.1438x | 0.8249x |
| `benchy-classic-on` | 476 | 13.85 % | 24.42 % | 2.164 | 4,086 | 4.55× | 1.5700x | 1.3526x |
| `benchy-arachne-off` | 482 | 27.03 % | 51.01 % | 2.169 | 4,086 | 4.50× | 1.1548x | 0.8426x |
| `benchy-arachne-on` | 481 | 13.43 % | 24.40 % | 2.179 | 4,086 | 4.53× | 1.3885x | 1.2021x |
| `base-classic-off` | 3,664 | 36.96 % | 50.70 % | 31.547 | 8,912 | 8.61× | 0.7687x | 0.4846x |
| `base-classic-on` | 3,675 | 12.45 % | 16.05 % | 31.653 | 8,912 | 8.61× | 1.2206x | 1.0687x |
| `base-arachne-off` | 3,624 | 34.22 % | 51.14 % | 29.294 | 8,912 | 8.08× | 0.7921x | 0.5211x |
| `base-arachne-on` | 3,720 | 12.88 % | 16.79 % | 30.821 | 8,912 | 8.29× | 1.2012x | 1.0465x |

**Exact op-kind call reconciliation (`t180` probe vs `t177` `--profile`):** `diff = 0` on every op kind (`clip_polygons`, `offset`, `offset2_ex`, `closing_ex`) across all three multi-pass cells:
- `benchy` (`classic-off` & `classic-on`): `clip = 2,308`, `offset = 1,062`, `offset2_ex = 462`, `closing_ex = 254` → **total `4,086`** (`diff = 0`).
- `base` (`classic-off`): `clip = 5,020`, `offset = 2,474`, `offset2_ex = 974`, `closing_ex = 444` → **total `8,912`** (`diff = 0`).

Because `PrePass::ShellClassification` runs strictly before `PrePass::SupportAnalysis` / `PrePass::SupportGeometry` and before `Layer::Perimeters`, its inputs depend only on the model (`SliceIR` + `RegionMapIR`), never on wall generator or support enablement — proven empirically by `4/4` zero-diff invariance checks. Thus `benchy` replay savings apply identically across all 4 `benchy` cells and `base` replay savings apply identically across all 4 `base` cells.

---

## 3. Sub-Phase Decomposition (`commit_shell_classification_builtin`)

All bracket sums reconcile to total stage wall within **`0.024–0.029 %`** (`0.14 ms` `benchy`, `0.83 ms` `base`). `benchy-classic-on` (`483.75 ms`, `31.19 %` serial, `4.96×` par) mirrors `benchy-classic-off` (`467.71 ms`, `30.19 %` serial, `5.10×` par) across every bracket.

### 3.1 `benchy-classic-off` (`467.71 ms` wall, `2,387.02 ms` thread, `5.10×` eff par; `SER = 141.19 ms / 30.19 %`, `PAR = 326.38 ms / 69.78 %`)

| Bracket | Mode | Wall (ms) | `% Stage` | Thread (ms) | Eff Par | `p50` / `p95` / `max` (ms) | Primary Bottleneck |
|---|---|---:|---:|---:|---:|---|---|
| `B5b_ib_phaseA` | PAR | **128.96** | **27.57 %** | 330.85 | **2.57×** | `0.001 / 5.49 / 128.89 (L1)` | Single-layer straggler `L1`: `ib_unsup_solids_grow = 102.41 ms` on `#3DBenchy` letter holes (`0` output polys!) |
| `B5d_ib_phaseB_com` | **SER** | **117.44** | **25.11 %** | 117.44 | **1.00×** | — | 12 serial recomputes (`L31, L41, L101, L190..L198`); `ib_ang_determine_deg` burns `111.60 ms` pure CPU |
| `B1_pass1` | PAR | 65.72 | 14.05 % | 776.54 | 11.82× | `2.45 / 6.59 / 25.71 (L1)` | `p1_bot_open 270.5`, `p1_bot_diff 187.3`, `p1_top_diff 161.5`, `p1_top_open 153.9 ms`; `81.98 %` slivers |
| `B5c_ib_phaseB_pre` | PAR | 51.09 | 10.92 % | 482.99 | 9.45× | `9.55 / 23.06 / 36.90 (L40)` | `31/43` (`72.09 %`) layers return `None`, wasting `53.50 %` (`258.4 ms`) thread (`gather` + `pos_offset`) |
| `B4_ext_bridge` | PAR | 37.33 | 7.98 % | 362.97 | 9.72× | `1.28 / 3.58 / 19.77 (L2)` | Unmarked CPU `ext_bridge_clip_polylines` (`90.2 ms`) + `ext_bridge_score_cpu` (`80.0 ms`, $O(C \times E)$) |
| `B2c_p2b_proj` | PAR | 33.35 | 7.13 % | 186.99 | **5.61×** | `0.78 / 1.68 / 32.99 (L2)` | `L2` straggler (`32.99 ms`): batched `p2b_offset` on 10 disjoint polys takes `28.11 ms` vs `~5.5 ms` per-ExPoly |
| `B2b + B2d + B2e` | **SER** | 19.24 | 4.11 % | 19.24 | **1.00×** | — | Serial Pass-2 merges (`4.19 + 7.19 ms`) + `internal_solid_fill` diff (`7.86 ms`; `68.92 %` have no Pass-2 contrib) |
| `B2a_p2t_proj` | PAR | 9.93 | 2.12 % | 105.49 | 10.62× | `0.69 / 1.45 / 3.66 (L2)` | Pass-2 top shadow projection |
| `B0 + B3 + B5a + B5e + B6` | SER | 4.52 | 0.96 % | 4.52 | 1.00× | — | Bookkeeping; `extra_bridge_enabled = 0` |

### 3.2 `base-classic-off` (`3,405.91 ms` wall, `30,359.29 ms` thread, `8.91×` eff par; `SER = 841.28 ms / 24.70 %`, `PAR = 2,563.80 ms / 75.27 %`)

| Bracket | Mode | Wall (ms) | `% Stage` | Thread (ms) | Eff Par | `p50` / `p95` / `max` (ms) | Primary Bottleneck |
|---|---|---:|---:|---:|---:|---|---|
| `B5c_ib_phaseB_pre` | PAR | **711.34** | **20.89 %** | 8,082.38 | 11.36× | `64.49 / 119.53 / 326.26 (L48)` | **`109/117` (`93.16 %`) layers return `None`**, wasting **`79.75 %` (`6,445.9 ms`)** (`ib_comp_pos_offset 4,016.4 ms` #1 site!) |
| `B1_pass1` | PAR | **698.72** | **20.51 %** | 8,256.02 | 11.82× | `16.23 / 30.14 / 47.25 (L178)` | `p1_top_open 2,688`, `p1_bot_open 2,224`, `p1_top_diff 1,688`, `p1_bot_diff 1,633 ms`; `79.53 %` slivers |
| `B5d_ib_phaseB_com` | **SER** | **558.53** | **16.40 %** | 558.53 | **1.00×** | — | 7 serial recomputes (`L38, L41=214.4, L42=85.5, L49=85.3, L60=52.9, L100=56.0, L379=31.6 ms`); **6/7 are `None`**! |
| `B2a + B2c` | PAR | 457.06 | 13.42 % | 5,152.82 | 11.27× | `4.70 / 13.56 / 36.69 (L156)` | `p2t_offset 1,600`, `p2b_offset 1,494`, `p2t_inter 1,075`, `p2b_inter 954 ms`; `54.48 %` slivers |
| `B5b_ib_phaseA` | PAR | 350.75 | 10.30 % | 3,963.07 | 11.30× | `0.002 / 45.45 / 133.73 (L157)` | `ib_unsup_solids_shrink 2,339`, `ib_unsup_erode 1,210 ms`; only `12.12 %` `lf` & `14.04 %` `ls` verts inside bbox! |
| `B4_ext_bridge` | PAR | 345.93 | 10.16 % | 4,063.71 | 11.75× | `7.18 / 16.92 / 27.67 (L41)` | Unmarked `ext_bridge_clip_polylines` (`1,230 ms`) + `ext_bridge_score_cpu` (`834 ms`) + 3 Clipper ops (`1,989 ms`) |
| `B2b + B2d + B2e` | **SER** | 254.85 | 7.48 % | 254.85 | **1.00×** | — | Serial merges (`38.33 + 47.47 ms`) + `p2e` (`169.05 ms`; `364/481 = 75.68 %` have zero Pass-2 contribs!) |
| `B0 + B3 + B5a + B5e + B6` | SER | 27.91 | 0.82 % | 27.91 | 1.00× | — | Bookkeeping; `extra_bridge_enabled = 0` |

### 3.3 Key Structural & Locality Invariants (`reduced/probe.json`)

- **Internal-bridge depth window fan-in:** `gather_layers = {layers_1: 55, layers_gt1: 0}` (`benchy`) and `{layers_1: 124, layers_gt1: 0}` (`base`) — on constant `0.20 mm` layers, `BRIDGE_FLOW_HEIGHT_FACTOR * 0.20 = 0.32 mm` always spans strictly `1` lower layer (`L - 1`).
- **Dead `exp_diff` in `internal_bridge_angles` (`crates/slicer-core/src/algos/bridge_over_infill.rs`):** `ib_b_qual_cands_max = 1` on `base` (`102.6 ms` spent computing `difference(&remaining_expansion, &expanded)` on the final candidate when the loop immediately exits and discards it!).
- **Unused canonical clustering:** `crates/slicer-core/src/algos/bridge_over_infill.rs::cluster_candidate_layers` already exists in-tree, yet `gate_internal_bridge_sites` executes the entire Phase-B commit serially.

---

## 4. Canonical OrcaSlicer Comparison (`OrcaSlicerDocumented/src/libslic3r/`, File + Function Only)

Audited directly against the in-workspace `OrcaSlicerDocumented` checkout (strictly file + function names, zero line numbers):

1. **`PrintObject::detect_surfaces_type` (`PrintObject.cpp`):** skips BOTH `diff_ex` and `opening_ex` on first/last layers when no adjacent layer exists; uses plain whole-layer `diff_ex` / `intersection_ex` (neither `diff_ex_by_piece` nor `intersection_ex_by_piece` exists).
2. **`PrintObject::discover_vertical_shells` vs `PrintObject::discover_horizontal_shells` (`PrintObject.cpp`):** mutually exclusive per region. Under default `evstAll`, `discover_horizontal_shells` does **zero** Clipper work; `discover_vertical_shells` runs two `tbb::parallel_for` passes with early-outs on vertical walls (vs PnP's serial Pass-2 shadow merges + serial `B2e` `internal_solid_fill` diff).
3. **`PrintObject::bridge_over_infill` (`PrintObject.cpp`):** groups dependent candidate layers via `clustered_layers_for_threads` and runs independent Z-clusters in parallel via `tbb::parallel_for`, and uses `AABBTreeLines::LinesDistancer` (`AABBTreeLines.hpp`) inside `determine_bridging_angle` (avoiding PnP's brute-force $O(S \times A)$ segment scan). *Premise correction vs `t129`'s summary:* `OrcaSlicerDocumented`'s `PrintObject::bridge_over_infill` has neither a `boundary_by_spacing` cache nor `clip_clipper_polygons_with_subject_bbox` (`clip_clipper_polygon_with_subject_bbox_templ` in `ClipperUtils.cpp` is used in `RegionExpansion.cpp::propagate_wave_from_boundary` and `TreeSupport.cpp`).
4. **`offset_expolygon_inner` / `opening_ex` (`ClipperUtils.cpp`):** executes negative offsets (`delta < 0`) **per `ExPolygon`**, skipping hole processing when the contour collapses and avoiding cross-`ExPolygon` sweep work on disjoint islands — directly explaining why PnP's batched `p2b_offset` on `benchy` `L2` (10 disjoint polys) takes `28.1 ms` vs `~5.5 ms` per-`ExPolygon`, and why `L2` cuts `B2p` by `~40 %`.
5. **`LayerRegion::process_external_surfaces` (`LayerRegion.cpp`):** runs per-layer external bridge direction detection using angular candidate evaluation over clipped overhang boundaries.

---

## 5. Offline 12-Thread Exact IR Replay & Candidate Screen

Using exact serialized prepass inputs + expected output snapshots (`benchy.shell_io.postcard` = `240` layers, `base.shell_io.postcard` = `496` layers) replayed against clean reverted `slicer-core` (`RAYON_NUM_THREADS=12`, `3` interleaved rounds, `M0_baseline` reproduces `expected_out` **100 % bit-identically** and tracks probed production stage wall within `+0.65 %` on `benchy` and `−2.89 %` on `base`):

### 5.1 Replay Timing & Exactness Matrix (`reduced/replay_benchy.json`, `reduced/replay_base.json`)

| Mechanism | Class | `benchy` Med (ms) | `benchy` Save | `benchy` Exactness | `base` Med (ms) | `base` Save | `base` Exactness |
|---|---|---:|---:|---|---:|---:|---|
| `M0_baseline` | baseline | 470.77 | 0.00 % | `EXACT_BIT_IDENTICAL` | 3,307.57 | 0.00 % | `EXACT_BIT_IDENTICAL` |
| `E1_p2_par_merge_diff` | **exact** | 457.67 | +2.78 % (`B2s 18.8→5.2`) | `EXACT_BIT_IDENTICAL` | 3,113.00 | +5.88 % (`B2s 255.9→20.7`) | `EXACT_BIT_IDENTICAL` |
| `E2_ib_lazy_cluster_cache` | **exact** | 366.45 | +22.16 % (`B5c 49.2→22.6`, `B5d 118.7→29.8`) | `EXACT_BIT_IDENTICAL` | 2,409.76 | +27.14 % (`B5c 718.5→272.2`, `B5d 558.4→77.8`) | `EXACT_BIT_IDENTICAL` |
| `E3_ib_phaseA_windowed` | **exact** | 361.68 | +23.17 % (`B5b 127.9→12.1`) | `EXACT_BIT_IDENTICAL` | 3,022.59 | +8.62 % (`B5b 336.2→58.9`) | `EXACT_BIT_IDENTICAL` |
| `E4a_ext_fast_score` | **exact** | 459.06 | +2.49 % (`B4 36.9→29.2`) | `EXACT_BIT_IDENTICAL` | 3,339.03\* | −0.95 %\* (`B4 338.5→280.1`) | `EXACT_BIT_IDENTICAL` |
| `E5_p2_bbox_filter_nbr` | **exact** | 471.70 | −0.20 % (speed-neutral) | `EXACT_BIT_IDENTICAL` | 3,322.81 | −0.46 % (speed-neutral) | `EXACT_BIT_IDENTICAL` |
| **`EXACT_BUNDLE` (`E1+E2+E3+E4a`)** | **exact** | **206.83** | **+56.06 % (`−263.93 ms`)** | **`EXACT_BIT_IDENTICAL`** | **1,833.22** | **+44.58 % (`−1,474.35 ms`)** | **`EXACT_BIT_IDENTICAL`** |
| `E4ab_ext_score_rect_clip` | falsified | 450.86 | +4.23 % | `DIFF` (`118` L, `89.94°`) | 3,185.68 | +3.69 % | `DIFF` (`423` L, `88.31°`) |
| `L1_sliver_prefilter` | falsified | 473.83 | −0.65 % | `DIFF` (`19` L, `13.97 µm`) | 3,095.73 | +6.40 % | `DIFF` (`271` L, `5.28 µm`) |
| `L2_p2_per_poly_neg` | sub-µm | 468.89 | +0.40 % (`B2p 46.8→29.4`) | `DIFF` (`6` L, `0.174 µm`) | 3,144.45 | +4.93 % (`B2p 410.6→246.0`) | `DIFF` (`56` L, `0.825 µm`) |
| `L3_p1_canon_open` | falsified | 479.23 | −1.80 % | `DIFF` (`132` L, `499.21 µm`) | 3,155.45 | +4.60 % | `DIFF` (`476` L, `213.28 µm`) |
| `EXACT_PLUS_P2_PERPOLY` | sub-µm | 193.92 | +58.81 % (`+2.74 pp` over exact) | `DIFF` (`6` L, `0.174 µm`) | 1,656.09 | +49.93 % (`+5.36 pp` over exact) | `DIFF` (`56` L, `0.825 µm`) |
| `EXACT_PLUS_P1_P2_CANON` | falsified | 186.15 | +60.46 % | `DIFF` (`136` L, `499.21 µm`) | 1,463.83 | +55.74 % | `DIFF` (`476` L, `213.28 µm`) |

*\*Note on `E4a` standalone `base` total: `B4` itself dropped cleanly from `338.5 → 280.1 ms` (`−58.4 ms`, identical to `285.8 ms` in `EXACT_BUNDLE`), while `B5b` had a single-round `+67 ms` noise blip (`336.2 → 403.9 ms`) in the standalone pass.*

### 5.2 Exactness & Falsification Analysis

- **Why `EXACT_BUNDLE` is mathematically exact (`EXACT_BIT_IDENTICAL` across all `736` layers):**
  1. **`E1` (parallel Pass-2 merges + parallel/skip `internal_solid_fill` diff):** grouping shadow contributions by target `slice_idx` and running `union` / `difference` across independent target layers in parallel (with exact skip when `top_solid_seed` is empty or no Pass-2 contribution landed on that layer, `68.92 %` `benchy` / `75.68 %` `base`) performs the exact same per-layer boolean algebra in deterministic order.
  2. **`E2` (lazy internal-bridge precompute + monotonic Z-cluster commit + cached region invariants + BVH/$O(S)$ `determine_bridging_angle` + skip dead final `exp_diff`):**
     - `qualify_internal_bridge_surface` depends only on `(s, unsupported, qsp, nofilter)`, not `deep_infill_area` — running it first eliminates `gather_areas_w_depth` + `offset(&deep, +1.5s)` on every layer with zero qualifying polygons; when qualifying polygons exist, windowing `gather_areas_w_depth` to the candidate bbox (plus exact erosion reach `4.5 * qsp + miter_pad`) proves whether any polygon survives `iua` (`offset(-4.5s)`) before running full-layer `gather` / `offset(+1.5s)`.
     - **Monotonicity of `None`:** subtracting lower-layer committed bridges shrinks `deep` (`deep' ⊆ deep`), so `iua' ⊆ iua` and `dca' ⊆ dca`; therefore any layer whose unsubtracted precompute returned `None` (`6/7` recomputes on `base`!) is guaranteed `None` after subtraction.
     - **Z-cluster parallelism:** active layers separated by `dz > target_flow_height * BRIDGE_FLOW_HEIGHT_FACTOR + SCALED_EPSILON_MM` cannot read each other's committed bridges (note: `t184` MUST cluster using the full `1.6 * target_flow_height` radius — not `1.0 * target_flow_height` — because `build_bridge_depth_layers` reads `lower.bridge_areas` across the `1.6 * tfh` window; on uniform `0.20 mm` layers both give identical clusters while `1.6 * tfh` stays 100 % sound on variable layer heights!).
     - **`determine_bridging_angle` + `internal_bridge_angles`:** replacing brute-force nearest-anchor scan with an exact 2D segment-bbox BVH (exact squared-distance pruning) and $O(S)$ angular vote sweep + hoisting `total_fill` anchor polylines and skipping the unread final `difference(&remaining_expansion, &expanded)` preserves exact bit-identical angles (`0.0000°` diff).
  3. **`E3` (exact rectangular windowing in `unsupported_span_areas`):** `unsupported_span_areas` computes `difference(erode(lower_fills, s), closing(grow(lower_solids, 3s), 3s))`, whose result is only ever intersected with `internal_solid_fill` (buffered by `s` inside `qualify_internal_bridge_surface` with miter limit `3.0`). Clipping `lower_fills` and `lower_solids` to `bbox(internal_solid_fill)` expanded by `11 * spacing` (exceeding every mitered reach in the 4-op chain) leaves the interior of the active window mathematically unchanged (`EXACT_BIT_IDENTICAL` on all 736 layers, cutting `B5b` by **`−90.7 %`** on `benchy` and **`−81.3 %`** on `base`).
  4. **`E4a` ($O((C+E)\log(C+E))$ angular prefix-sum sweep in `detect_bridging_direction_deg`):** evaluates the exact same angular objective over the exact same candidate angles and edges as the quadratic $O(C \times E)$ loop (exact sort + two-pointer / prefix sum over normalized `[0, pi)` edge orientations; `0.0000°` diff).
  5. **`E5` (bbox-filtering Pass-2 neighbor polygons):** exact (`EXACT_BIT_IDENTICAL`), but speed-neutral (`−0.20 %` / `−0.46 %`) because Clipper2 already rejects non-overlapping bboxes cheaply — **omitted from `t184`** to keep the diff minimal.
- **Why rejected candidates were falsified:**
  - `E4b` (rect-clipping `B4` anchors): clipping anchor polygons to a window rectangle manufactures synthetic edges along the window border that `floating_edges_of_gated_area`'s `clip_polylines` misclassifies, shifting `bridge_orientation_deg` on `118` `benchy` / `423` `base` layers by up to `89.94°`.
  - `L1` (`bbox <= 2r || area <= pi*r^2` sliver prefilter): dropped slivers that survived Clipper2's `Miter` offset + edge decimation (`13.97 µm` `benchy`, `5.28 µm` `base`) while saving `−0.65 %` on `benchy`.
  - `L3` (prototype `canonical_opening_ex`): used a `1× miter_limit * r` CC pad instead of `2× miter_limit * r` plus `L1`'s sliver prefilter, causing `499.2 µm` / `213.3 µm` Hausdorff diffs; even unfixed, its marginal speed over `EXACT_PLUS_P2_PERPOLY` is only `0.23 %` on `benchy-classic-on` and `0.72 %` on `base-classic-on` (sub-floor on supports-on).
  - `L2` (per-`ExPolygon` negative offset in Pass 2): valid sub-micron (`0.174 / 0.825 µm`) Clipper2 distant-scanbeam parity candidate matching canonical `offset_expolygon_inner`, but **marginal over `EXACT_BUNDLE` clears `0.80 %` in only `2/8` cells** (`0.37–0.76 %` on all 4 `benchy` cells, `0.67–0.69 %` on `base` supports-on) while breaking bit-identity (`EXACT_BUNDLE` already captures **`95.34 %`** of `EXACT_PLUS_P2_PERPOLY` on `benchy` and **`89.27 %`** on `base`!).

---

## 6. Standing 8-Cell Gate & Route Arithmetic (`ESTIMATE ONLY`)

All ratios below are **`ESTIMATE ONLY`** (`t174 × t175 × t180`, never ratios of record).

### 6.1 `EXACT_BUNDLE` (`t184`) Across All 8 Cells (100 % `EXACT_BIT_IDENTICAL`)

| Cell | Stage Share (`t177`) | Stage Save (Replay) | Proj. Stage Save (`t177` ms) | Proj. Cell Cut | `0.80 %` Floor Mult | Post-t175 Ord (`EST`) | After `t184` Ord (`EST`) | After `t184` Acc (`EST`) | `% of 1.0x Cut` |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `benchy-classic-off` | 27.88 % | 56.06 % | 265.7 ms | **15.63 %** | **19.54×** | 1.1438x | **0.9650x** (`<1x`) | **0.9432x** (`<1x`) | **124.3 %** |
| `benchy-classic-on` | 13.85 % | 56.06 % | 266.9 ms | **7.76 %** | **9.70×** | 1.5700x | 1.4481x | 1.4773x | 21.4 % |
| `benchy-arachne-off` | 27.03 % | 56.06 % | 270.2 ms | **15.16 %** | **18.94×** | 1.1548x | **0.9798x** (`<1x`) | **0.9028x** (`<1x`) | **113.0 %** |
| `benchy-arachne-on` | 13.43 % | 56.06 % | 269.7 ms | **7.53 %** | **9.41×** | 1.3885x | 1.2840x | 1.3031x | 26.9 % |
| `base-classic-off` | 36.96 % | 44.58 % | 1,633.4 ms | **16.48 %** | **20.60×** | 0.7687x | **0.6420x** | **0.6394x** | _(won; 47.1 % of 0.5x)_ |
| `base-classic-on` | 12.45 % | 44.58 % | 1,638.3 ms | **5.55 %** | **6.94×** | 1.2206x | 1.1529x | 1.1043x | 30.7 % |
| `base-arachne-off` | 34.22 % | 44.58 % | 1,615.6 ms | **15.25 %** | **19.07×** | 0.7921x | **0.6713x** | **0.6558x** | _(won; 41.4 % of 0.5x)_ |
| `base-arachne-on` | 12.88 % | 44.58 % | 1,658.4 ms | **5.74 %** | **7.18×** | 1.2012x | 1.1322x | 1.0871x | 34.3 % |

### 6.2 Disjoint-Stage Stacked Frontier Context (`ESTIMATE ONLY`: `t184 + t183 + t182 + t181`)

| Cell | Post-t175 Ord | `+ t184` (`Shell` Exact) | `+ t183` (`PerLayer` Exact) | `+ t182` (`Phase3` `M9n`) | `+ t181` (`HostTail` Ceiling) Ord / Acc |
|---|---:|---:|---:|---:|---:|
| `benchy-classic-off` | 1.1438x | **0.9650x** | **0.8477x** | **0.8477x** | **0.8477x / 0.8285x** |
| `benchy-arachne-off` | 1.1548x | **0.9798x** | **0.9184x** | **0.9184x** | **0.9184x / 0.8461x** |
| `base-classic-off` | 0.7687x | **0.6420x** | **0.6228x** | **0.6228x** | **0.6228x / 0.6203x** |
| `base-arachne-off` | 0.7921x | **0.6713x** | **0.6546x** | **0.6546x** | **0.6546x / 0.6395x** |
| `base-classic-on` | 1.2206x | 1.1529x | 1.0772x | **0.9023x** | **0.8075x / 0.7735x** |
| `base-arachne-on` | 1.2012x | 1.1322x | 1.0532x | **0.8843x** | **0.8004x / 0.7685x** |
| `benchy-arachne-on` | 1.3885x | 1.2840x | 1.1908x | 1.1493x | 1.0649x / 1.0807x |
| `benchy-classic-on` | 1.5700x | 1.4481x | 1.3756x | 1.3269x | 1.2291x / 1.2538x |

---

## 7. Surviving `ShellClassification` Residual Post-`t184`

After `EXACT_BUNDLE` (`t184`), `ShellClassification` drops to `206.8 ms` (`benchy`) / `1,833.2 ms` (`base`):
- **`B1_pass1` (`64.8 ms` `benchy` / `676.2 ms` `base`, `31–37 %` of residual):** saturated at `11.82×` across `difference` + `opening_ex`; further cuts require canonical `offset_expolygon_inner` per-`ExPolygon` erosion + exact `2× miter_limit * r` CC dilation (`L3` fixed) or dual-sweep difference optimization.
- **`B2p_pass2_proj` (`39.8 ms` `benchy` / `412.6 ms` `base`, `19–23 %` of residual):** per-`ExPolygon` negative offset (`L2`) cuts this by `~40 %` (`−12.9 ms` `benchy`, `−177.1 ms` `base`, sub-micron `0.174 / 0.825 µm` shift).
- **`B4_ext_bridge` (`29.1 ms` `benchy` / `285.8 ms` `base`, `14–16 %` of residual):** remaining work is `floating_edges_of_gated_area` (`clip_polylines` + anchor offset + complement diff).
- **`B5c + B5d + B5b` (`63.5 ms` `benchy` / `414.7 ms` `base`, `23–31 %` of residual):** remaining necessary Clipper ops on actually-surviving internal bridge layers.

# `ShellClassification` post-t172 residual attribution and speedup screen

Type: task / attribution + candidate screen (no production commit in this take;
probe reverted if used)
Label: wayfinder:task
Status: resolved 2026-11-22 — attribution + candidate screen complete (`15/15` probed runs byte-identical to `t175/reference.json`, `probe.patch` cleanly reverted, both guest freshness checks exit `0`, zero production edits); graduated 100 % exact `EXACT_BUNDLE` (`−56.06 %` `benchy` / `−44.58 %` `base` stage wall, clears all-8-cell `0.80 %` gate at `6.94×–20.60×` floor with zero output drift, crosses `< 1.0x` on both `benchy` supports-off cells alone) as [Exact `ShellClassification` acceleration](184-shell-classification-exact-acceleration.md)
Assignee: agent (2026-11-22)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 177 (resolved — unblocked 2026-11-21; see `evidence/t177-post-t175-attribution/FINDINGS.md` §6 for post-t175 stage shares, `clip_polygons`-dominant op split, and 4.5× `benchy` vs 8.1–8.6× `base` parallelism gap)
Graduated from: [Destination ruling: CPU-corroboration semantics and sample count for cell closure](176-destination-cpu-corroboration-ruling.md),
because `PrePass::ShellClassification` was the #1 stage across all four
supports-off cells (**32.2–44.0 %** of cell wall in pre-t172
[t171](171-post-t169-prepass-reprice.md), plus **11.6–15.0 %** on every
supports-on cell) — making its surviving post-t172 residual a direct multi-cell
lever that can simultaneously close `benchy` supports-off (`1.14–1.15x → < 1.0x`),
chip away at all four supports-on cells (including binding `benchy-classic-on`),
and push `base` supports-off (`0.77–0.79x`) toward the follow-on `<= 0.50x`
target.

## Question

**After [t166](166-shell-opening-canonical-parity.md),
[t169](169-canonical-offset-edge-decimation-and-flat-opening.md), and
[t172](172-port-offset-concave-crossing.md) brought Clipper2 per-call offset
cost to ~1.04–1.30x of canonical C++, what remains inside
`crates/slicer-runtime/src/slice_postprocess_prepass.rs::commit_shell_classification_builtin`
— and what work-set, locality, or parallelism candidates cut its surviving
post-t172 wall across all eight cells?**

Concretely:

1. **Reconcile against [t177](177-post-t175-full-matrix-attribution.md) first:**
   confirm `PrePass::ShellClassification`'s exact post-t175 wall share,
   effective parallelism (`thread / wall`), and host clipper op breakdown
   (`offset`, `clip_polygons`, `offset2_ex`, `closing_ex`) across all 8 cells.
2. **Decompose `commit_shell_classification_builtin`
   (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) into named
   sub-phases and call sites** on `benchy-classic-off`, `benchy-classic-on`,
   and `base-classic-off`:
   - Pass 1 top/bottom surface detection (`apply_opening` + `difference`/`intersection`),
   - Pass 2 shell-thickness shadow/propagation cascade,
   - Internal-bridge site computation (`compute_internal_bridge_layer` /
     `gate_internal_bridge_sites` — noting `t133` S5 already windowed the
     negative erosion and `t171` priced S5's residual at `0.83 %` pre-t172),
   - Serial vs parallel brackets (checking whether any serial commit/merge
     loop like [t75](75-shellclassification-phase-b-commit.md) or
     low-fan-out pass still caps effective parallelism below 12 threads),
   - Work-set comparison against canonical `v2.5.0` (`40ecc66f78`)
     `PrintObject::detect_surfaces_type` / `process_external_surfaces` /
     `discover_vertical_shells` (citing file + function only, never line
     numbers): are there still redundant full-layer booleans/offsets or
     whole-layer operands where canonical operates on a narrower active set or
     fewer passes?
3. **Price concrete candidate shapes:** state each candidate's ceiling in ms
   and % of cell wall across the supports-off and supports-on cells, classify
   each as **exact output-preserving** vs **canonical-parity under the existing
   t111/t128 prepass exception**, and verify whether it clears the standing
   all-8-cell `0.80 %` gate.
4. **Decide graduation:** graduate implementation ticket(s) for any candidate
   that materially closes `benchy` supports-off (`1.14–1.15x → < 1.0x`) and/or
   cuts `benchy-classic-on` + `base` supports-off toward `<= 0.50x`, or record
   explicitly if `ShellClassification` is already at its canonical floor.

## Scope

- Blocked by [177](177-post-t175-full-matrix-attribution.md) so the screen
  starts from verified post-t175 stage shares.
- Attribution and candidate pricing only; no production commit in this take,
  any diagnostic probe cleanly reverted and saved as `probe.patch`.

## Pre-declared gate

**Attribution + screen only (no landing gate).** Correctness bar:

- `cargo xtask build-guests --check` exits 0 before and after probe revert.
- Baseline attribution passes reproduce `t175`'s active `reference.json`
  byte-for-byte (`exit 0`, `degraded = false`, `0` code-1200).
- Every candidate's ceiling is re-derived from measured per-subphase thread
  time and measured stage parallelism on the post-t175 tree.

## Acceptance

- Evidence directory `docs/specs/perf-vs-orca/evidence/t180-shell-classification-residual/`
  with `FINDINGS.md`, `REPRODUCE.md`, reduced JSONs, `probe.patch` (if used),
  and checkout-only `verify.py` (`verify.py` + `verify.py --self-test` exit 0).
- Updated `## Answer` in this ticket, `map.md` update, and graduated
  implementation ticket(s) for any surviving route candidate.

## Answer

**Resolved 2026-11-22 (attribution + candidate screen only; zero production code edits landed; `probe.patch` cleanly reverted, clean release `pnp_cli.exe` rebuilt, both `cargo xtask build-guests --check` and `cargo xtask build-guests --accelerated --check` exit `0`).** Evidence: [`evidence/t180-shell-classification-residual/FINDINGS.md`](../evidence/t180-shell-classification-residual/FINDINGS.md) (`verify.py` PASS `7/7`, `--self-test` PASS `15/15`, Python 3 stdlib only).

1. **Qualification, `t177` reconciliation, and invariance:**
   - **15/15** probed production runs (`RAYON_NUM_THREADS=12`) exit `0` and reproduce `t175/reference.json` byte-for-byte (**3/3** `--instrument-stderr` runs `degraded = false`, `0` fatal/non-fatal/code-1200).
   - **100.0 % exact call-count reconciliation** against `t177` `--profile` scopes across all 4 op kinds (`diff = 0`): `benchy` `4,086` calls (`clip = 2,308`, `offset = 1,062`, `offset2_ex = 462`, `closing_ex = 254`), `base` `8,912` calls (`clip = 5,020`, `offset = 2,474`, `offset2_ex = 974`, `closing_ex = 444`).
   - **4/4 pair invariance (`0` differing sites):** `ShellClassification` call counts and vertex totals are bit-identical across Classic vs Arachne and supports-off vs supports-on on both `benchy` and `base`, so model-level speedups transfer across all 8 cells (`27.88 / 27.03 %` `benchy-off`, `13.85 / 13.43 %` `benchy-on`, `36.96 / 34.22 %` `base-off`, `12.45 / 12.88 %` `base-on`).

2. **Root causes of the surviving residual and `4.5×` `benchy` vs `8.1–8.6×` `base` concurrency gap:**
   - **Serial bottleneck (`30.19 %` / `141.19 ms` on `benchy-classic-off`; `24.70 %` / `841.28 ms` on `base-classic-off`):**
     - `B5d_ib_phaseB_com` (`117.44 ms` = **`25.11 %`** `benchy`; `558.53 ms` = **`16.40 %`** `base`): `gate_internal_bridge_sites` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) runs Phase-B commit serially even though `cluster_candidate_layers` (`crates/slicer-core/src/algos/bridge_over_infill.rs`) already exists, re-runs full-layer `gather_areas_w_depth` on `6/7` `None` recomputes on `base`, burns `111.60 ms` (`benchy`) of unmarked pure-CPU $O(S \times A) + O(S^2)$ work in `determine_bridging_angle`, and runs a dead final `difference` in `internal_bridge_angles` (`102.6 ms` on `base` where `max_cands = 1`).
     - `B2b + B2d + B2e` (`19.24 ms` = `4.11 %` `benchy`; `254.85 ms` = `7.48 %` `base`): serial Pass-2 shadow merges and `internal_solid_fill` diff (`68.92 %` `benchy` / `75.68 %` `base` have zero Pass-2 contributions so `top_solid_fill == top_solid_seed`).
   - **Single-layer `B5b_ib_phaseA` straggler (`128.96 ms` = `27.57 %` `benchy`, `2.57×` par; `350.75 ms` = `10.30 %` `base`):** `benchy` `L1` alone takes `128.89 ms` (`ib_unsup_solids_grow = 102.41 ms` on `#3DBenchy` letter holes) only to output `0` polygons; only `21.96 % / 26.11 %` (`benchy`) and `12.12 % / 14.04 %` (`base`) of `lower_fills` / `lower_solids` vertices lie inside `internal_solid_fill`'s padded bbox.
   - **Wasted `None` precomputes in `B5c_ib_phaseB_pre` (`51.09 ms` `benchy`, `711.34 ms` = `20.89 %` `base`):** `72.09 %` (`31/43` `benchy`) and **`93.16 %` (`109/117` `base`)** of layers return `None`, wasting **`53.50 %` (`258.4 ms`)** and **`79.75 %` (`6,445.9 ms`)** of thread time on eager whole-layer `gather_areas_w_depth` + `offset(&deep, +1.5s)` (`ib_comp_pos_offset = 4,016.4 ms`, the #1 Clipper site on `base`!).
   - **Unmarked pure-CPU work invisible to `--profile`:** `281.83 ms` (`benchy`) and `2,154.64 ms` (`base`) thread-time across `ib_ang_determine_deg`, `ext_bridge_clip_polylines`, and quadratic `ext_bridge_score_cpu` (`detect_bridging_direction_deg`, `crates/slicer-core/src/algos/prepass_slice.rs`).
   - **Canonical OrcaSlicer comparison (`OrcaSlicerDocumented/src/libslic3r/`, file + function only):** canonical `PrintObject::bridge_over_infill` (`PrintObject.cpp`) clusters dependent layers (`clustered_layers_for_threads`) in parallel and uses `AABBTreeLines::LinesDistancer` (`AABBTreeLines.hpp`); `PrintObject::discover_vertical_shells` runs two `tbb::parallel_for` passes; `offset_expolygon_inner` (`ClipperUtils.cpp`) offsets negative deltas per `ExPolygon` (explaining PnP's `L2` `B2c` `28.1 ms` vs `~5.5 ms` blowup). *(Premise correction vs `t129`: `OrcaSlicerDocumented`'s `PrintObject::bridge_over_infill` has no `boundary_by_spacing` cache and does not call `clip_clipper_polygon_with_subject_bbox_templ`.)*

3. **Priced candidates & exactness classification (12-thread exact IR replay on 240 `benchy` + 496 `base` layers):**
   - **`EXACT_BUNDLE` (`E1` parallel Pass-2 merge/diff + `E2` lazy/monotonic/clustered IB commit + BVH/$O(S)$ angle + region-invariant cache + dead-op skip + `E3` exact `11 * spacing` windowed `unsupported_span_areas` + `E4a` $O((C+E)\log(C+E))$ `detect_bridging_direction_deg`):**
     - **Class:** **100 % `EXACT_BIT_IDENTICAL`** across all 240 `benchy` and 496 `base` layers (zero output drift, zero geometry exceptions).
     - **Stage cut:** `benchy` **`470.77 → 206.83 ms` (`−56.06 %`, `−263.93 ms`)**; `base` **`3,307.57 → 1,833.22 ms` (`−44.58 %`, `−1,474.35 ms`)**.
     - **Standing 8-cell `0.80 %` gate (`ESTIMATE ONLY`):** clears `0.80 %` in **8/8 cells at `6.94×–20.60×` floor** (`benchy-classic-off` `15.63 % / 19.54×`, `benchy-arachne-off` `15.16 % / 18.94×`, `benchy-classic-on` `7.76 % / 9.70×`, `benchy-arachne-on` `7.53 % / 9.41×`, `base-classic-off` `16.48 % / 20.60×`, `base-arachne-off` `15.25 % / 19.07×`, `base-classic-on` `5.55 % / 6.94×`, `base-arachne-on` `5.74 % / 7.18×`).
     - **Route impact (`ESTIMATE ONLY`, `t174 × t175 × t180`):** **alone crosses `< 1.0x` on BOTH `benchy` supports-off cells** (`benchy-classic-off` `1.1438x → 0.9650x` ord / `0.9432x` acc; `benchy-arachne-off` `1.1548x → 0.9798x` ord / `0.9028x` acc), pushes `base` supports-off to `0.6420x / 0.6713x` ord, and moves binding `benchy-classic-on` `1.5700x → 1.4481x` (`1.2291x` stacked with `t183 + t182 + t181`, where **6/8 cells sit `< 1.0x`**).
   - **`E5` (bbox-filtering Pass-2 neighbors):** exact (`EXACT_BIT_IDENTICAL`) but speed-neutral (`−0.20 %` / `−0.46 %`) — omitted.
   - **`L2` (per-`ExPolygon` negative offset in Pass 2, `offset_expolygon_inner` style):** **canonical-parity sub-micron scanbeam drift (`0.174 / 0.825 µm` Hausdorff)**; marginal over `EXACT_BUNDLE` is `−12.91 ms` (`benchy`) / `−177.13 ms` (`base`), clearing `0.80 %` in only **`2/8` cells** (`0.37–0.76 %` on all 4 `benchy` cells, `0.67–0.69 %` on `base-on`) — **not bundled into `t184`** (parked in `map.md ## Not yet specified` for the `<= 0.50x` follow-on).
   - **Falsified / rejected:** `E4b` (rect-clipping `B4` anchors — synthetic cut edges shift `118 / 423` layers by up to `89.94°`), `L1` (`bbox <= 2r || area <= pi*r^2` sliver prefilter — `13.97 / 5.28 µm` Hausdorff, zero gain on `benchy`), and prototype `L3` (per-`ExPolygon` Pass-1 opening with `1× miter_limit` CC pad — `499.2 / 213.3 µm` Hausdorff and sub-floor marginal on supports-on).

4. **Graduation:**
   - Graduated [Exact `ShellClassification` acceleration: windowed `unsupported_span_areas`, lazy/clustered internal-bridge commit, parallel Pass-2 merge/diff, and $O((C+E)\log(C+E))$ external-bridge scoring](184-shell-classification-exact-acceleration.md) (AFK, unblocked, zero WIT/guest changes, literal 8-cell `0.80 %` gate, `16/16` byte-identical to `t175/reference.json`).

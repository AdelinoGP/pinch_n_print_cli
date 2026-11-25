# t178 findings — planner Phase-3 batched union attribution and mechanism screen

Ticket: [Planner Phase-3 batched union attribution and mechanism screen](../../issues/178-planner-phase3-union-attribution.md).
Take: 2026-11-21, ordinary arm at HEAD `0c242fa5` (last code commit `5b138f5c`, t175), `RAYON_NUM_THREADS=12`.
**Zero production edits landed.** The temporary capture probe (`probe.patch`) is cleanly reverted (`git diff -- crates/ modules/ vendor/ xtask/` empty) and both guest freshness gates (`cargo xtask build-guests --check` and `--accelerated --check`) exit `0`.

**Attribution & offline screen only.** Production Phase-3 batch wall and thread-time come from 3 clean uninstrumented passes (`base-classic-on`, `benchy-classic-on`) plus 1 pass (`base-arachne-on`, `benchy-arachne-on`) with zero dump I/O and zero `--instrument-stderr` stream, alongside 4 full `--instrument-stderr` qualification passes. All 12 runs exit `0`, `degraded = false`, `fatal = 0`, `non_fatal = 0`, `code-1200 = 0`, and match `t175`'s active [`reference.json`](../t175-tiled-collision-carve/reference.json) **12/12** byte-for-byte. Every mechanism comparison comes from 3-round interleaved native replay (`t178-phase3-replay`, `mimalloc`, `vendor/clipper2-rust`, `RAYON_NUM_THREADS=12`) over **100 % of captured real Phase-3 payloads** (`480/480` `base` + `202/202` `benchy`, both passing **exact vertex-for-vertex baseline replay with 0 mismatches**). All post-mechanism route ratios are explicitly **`ESTIMATE ONLY`** (`t174` quiet board × `t175` paired saving × `t178` Phase-3 projection), never a three-tool scoreboard ratio of record.

---

## Verdict

**Five findings explain why Phase-3 union burns `4.43 s` wall / `45.52 s` thread-time on `base` supports-on and identify one clear winning mechanism (`M9` / `M9n` host-side spatial hierarchical union + fast convex-containment pruning, graduated as [t182](../../issues/182-host-spatial-hierarchical-phase3-union.md)):**

1. **Why Phase-3 burns `45.5 s` thread / `4.43 s` wall on `base` (`15.68 %` of the `28.28 s` cell wall):**
   - `ROLE_BODY` owns **99.99 %** (`45,536 ms` of `45,542 ms`, `431/480` calls, `136,256/136,440` polygons, `13.48 M / 13.50 M` vertices; `99.95 %` on `benchy`). `ROLE_ROOF` is `5.0 ms` (`0.01 %`), `ROLE_FLOOR` is `0.46 ms`, `ROLE_BASE` is `0` calls. Classic and Arachne emit **100 % identical** Phase-3 operands (`classic_arachne_operand_identity = {benchy: true, base: true}`).
   - Phase 3 is **not** a small-polygon or conversion problem: WIT/IR marshalling is **0.23 %** of request thread-time on `base` (`0.65 %` on `benchy`), and `136,440` heavily overlapping 100-vertex branch ellipses (`13,499,633` input vertices, **7.64× area overlap factor**, **28.11 % strictly contained** inside a single larger convex sibling!) collapse **11.0×** into only `2,723` output polygons (`1,222,994` vertices, `vertex_collapse_ratio = 0.0906`; `benchy`: `22,058 → 926` polys, `2.19 M → 351 k` verts, `3.50×` area overlap, `28.72 %` strictly contained).
   - Dumping up to **1,024 overlapping 100-gons (`102,400` vertices)** into one monolithic Clipper2 sweep burns up to **414.2 ms** on a single layer (`p50 = 85.2 ms`, `p95 = 291.2 ms`, `p99 = 400.0 ms`, `max = 414.2 ms`).
2. **Mechanisms A, B, C, and `subject=all` are all FALSIFIED as sufficient levers:**
   - **Mechanism A (`M2` bbox-connected components + singleton bypass):** falsified as a standalone lever. Only **0.39 %** (`530/136,440` polys on `base`, `63/22,058 = 0.29 %` on `benchy`) are singletons (100 % canonical/simple: `530/530` and `63/63` pass winding + self-union exactness), while **88.5 %** of all `base` vertices sit in giant connected components (`65..256` polys = `59.1 %`, `257+` = `29.4 %`, max **`1,023` polys** in one component). Standalone `M2` oscillates within box noise (`−8.7 %` to `+12.4 %` wall across takes; `+0.4…+18.5 %` thread) and fails `t137`'s 4-cell floor.
   - **Mechanism B (`expolygons_simplify` / pre-union simplification):** `avg_nodes` is **`8`** on `base` and **`5`** on `benchy` (far below `COARSE_CIRCLE_NODE_THRESHOLD = 200`), so Phase-4 `expolygons_simplify` fires on **0/480 `base` and 0/202 `benchy` calls**. Forced pre-union RDP (`M6_50u` / `M6_125u`) is both far slower (`+7.3 %` / `+47.4 %` wall) and catastrophically lossy (`870 mm²` / `3,902 mm²` XOR area, `583 µm` / `3,420 µm` boundary Hausdorff). **Rejected.**
   - **Mechanism C (scheduling / batching tail):** `prod_eff_par` is already **`10.27×`** on `base` (`10.03×` on `benchy`). `sum(batch_max_clip)` is `99.1 %` of batch wall because 12 layers map 1:1 onto 12 threads; LPT within a 12-layer batch (`M5a`) is neutral (`−1.5 %`), and even merging all 37 batches into one global 480-call batch (`M5b`) hits the `45.5 s / 12 = 3.79 s` floor (`+12.9 %` wall, `0 %` CPU saved).
   - **`subject = all, clip = []` (`M1`):** byte-identical (`0/480` diffs) and speed-neutral (`−5.8 %` to `+0.7 %` within noise).
   - **Exact polygon / cross-layer deduplication:** `0` exact duplicate polygons within calls and `0` identical repeated `(layer, role)` input sets on both `base` and `benchy`.
3. **Winning mechanism — `M9` / `M9n` (Host-side fast exact convex-containment pruning + bbox-connected components + Morton Z-order spatial hierarchical union, `k = 8` primary / sweep `k ∈ {4, 8, 16}`):**
   - **Mechanism E alone (`M3`, fast inscribed-disk + $O(\log V)$ convex-containment pruning):** removes **38,351 / 136,440 (`28.11 %`)** `base` polygons and **6,335 / 22,058 (`28.72 %`)** `benchy` polygons that lie strictly inside a single larger convex sibling, cutting Clipper thread-time **−43.3 %** on `base` and **−53.3 %** on `benchy` (`0.224 µm` max boundary Hausdorff).
   - **Mechanism D + E combined inside host worker (`M9_host_worker_prune_hier_k8` / `M9n_host_nested_prune_hier_k8`):** sorting each connected component's survivors by 2D Morton Z-order and reducing in small spatially-contiguous chunks (`k = 8`) collapses ~90 % of interior overlapping edges inside tiny 800-vertex sweeps before merging outer hulls. On real 12-thread replay:
     - **`base-classic-on`:** **`−90.89 %` (`M9_k8`) to `−91.38 %` (`M9n_k8`)** Phase-3 wall (`5,080.5 → 437.7–463.0 ms`, **11.0×–11.6× faster**; `M9_k4` reaches **−92.12 %** / `400.4 ms` = **12.7×**), **`−92.4 %` to `−93.5 %` Clipper thread-time** (`50.36 s → 3.28–3.81 s`), with **`max_boundary_hausdorff_um = 0.949 µm`** at `k = 8` (< 1 µm G-code precision; total XOR area `0.00876 mm²` out of `534,190 mm²` = **`1.64 × 10⁻⁸`**!).
     - **`benchy-classic-on`:** **`−67.78 %` (`M9_k8`) to `−69.26 %` (`M9n_k8`)** Phase-3 wall (`182.2 → 56.0–58.7 ms`), **`−74.1 %` to `−74.7 %` Clipper thread-time**, with **`max_boundary_hausdorff_um = 0.400 µm`** (total XOR `0.00053 mm²` / `108,110 mm²` = **`4.93 × 10⁻⁹`**!).
4. **Architectural placement discovery (`clip_polygons_batch` has ONLY ONE caller in the entire repository):**
   - `git grep clip_polygons_batch` proves `modules/core-modules/tree-support-planner/src/lib.rs::run_clip_batches` is the **sole caller** of `clip_polygons_batch` across the entire repo.
   - Placing `union_spatial_tree` in `crates/slicer-core/src/polygon_ops.rs` and dispatching it from `crates/slicer-wasm-host/src/host.rs::clip_polygons_batch` (plus `crates/slicer-sdk/src/host_batch.rs` native fallback) when `op == ClipOperation::Union` and `total_polys > k` requires **zero WIT/schema changes**, **zero guest WASM changes**, **zero extra WIT round-trips** (stays at 37 batches / 480 requests), parallelizes prep across all 12 native host threads (`0 ms` serial guest prep vs `350–400 ms` guest-side), and is **100 % structurally inert on all 4 supports-off cells** (`t137` compliant by construction, zero part-geometry risk).
5. **Clipper2 distant-scanbeam proof (100.0 % explained) & route pricing (`ESTIMATE ONLY`):**
   - Exhaustive dummy-far-Y scanbeam auditing on `M2` proves **147/147 (100.0 %) `base`** and **27/27 (100.0 %) `benchy`** differing calls are caused solely by `vendor/clipper2-rust/src/engine.rs::add_new_intersect_node` clamping shallow intersections to global `scanline_list` Y coordinates from unrelated sibling components (**0 unexplained**; true boundary Hausdorff = **`0.162 µm`** on `base`, **`0.130 µm`** on `benchy`). Any Phase-3 decomposition therefore requires an explicit human KEEP ruling on the `< 1 µm` `Support` / `Support interface` hash move (matching `t175`'s precedent).
   - **Route impact (`ESTIMATE ONLY`):** `M9n_k8` projects **`−4,052 ms` (`−14.33 %` cell, `17.91×` floor)** on `base-classic-on`, **`−4,025 ms` (`−14.06 %`, `17.58×`)** on `base-arachne-on`, **`−107.0 ms` (`−3.10 %`, `3.88×`)** on `benchy-classic-on`, and **`−106.5 ms` (`−2.99 %`, `3.74×`)** on `benchy-arachne-on` — clearing `t137`'s 4-cell supports-on floor cleanly (**3.7×–17.9×**). Because clean Phase 3 is `15.68 %` / `15.39 %` of `base` supports-on against `18.07 %` / `16.75 %` required cuts, Phase 3 alone closes **79.3 % / 84.0 %** of the remaining ordinary gap (`1.221x → ~1.046x` ord / `~1.002x` acc on `base-classic-on`; `1.201x → ~1.032x` ord / **`~0.991x` acc crossing `< 1.0x`** on `base-arachne-on`), and stacking with **half to full [t181](../../issues/181-support-geometry-post-planner-host-tail.md)** (`7.8 % / 7.0 %` post-planner host tail) crosses `< 1.0x` cleanly on **both** ordinary `base` cells (**`~0.998x → 0.951x`** and **`~0.990x → 0.948x`** → **4/8**).

---

## 1. Qualification & production Phase-3 baseline (`reduced/prod_timing.json`)

All 12 capture runs (`4` qualification `--instrument-stderr` + `8` clean uninstrumented timing passes) exit `0`, `degraded = false`, `fatal = 0`, `non_fatal = 0`, `code-1200 = 0`, and match `t175`'s active `reference.json` byte-for-byte.

| Cell | Clean cell wall (med) | Phase-3 wall (med, runs) | Phase-3 % cell | Phase-3 clip thread (med) | `eff_par` | `sum(batch_max)` (% wall) | Ideal 12t floor | WIT/IR conv % | `est_ord` / `acc` if Phase-3 = 0 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `benchy-classic-on` | 3,448.4 ms | **154.5 ms** (157.6/154.1/154.5) | **4.48 %** (5.6×) | 1,548.6 ms | 10.03× | 152.3 ms (**98.6 %**) | 129.0 ms | 0.65 % | 1.500x / 1.530x |
| `benchy-arachne-on` | 3,556.4 ms | **153.8 ms** (1 pass) | **4.32 %** (5.4×) | 1,547.4 ms | 10.06× | 150.6 ms (**98.0 %**) | 128.9 ms | 0.69 % | 1.329x / 1.348x |
| `base-classic-on` | 28,278.1 ms | **4,434.3 ms** (4419/4434/4441) | **15.68 %** (19.6×) | 45,519.2 ms | **10.27×** | 4,395.3 ms (**99.1 %**) | 3,795.1 ms | **0.23 %** | 1.029x / **0.986x** |
| `base-arachne-on` | 28,625.1 ms | **4,404.6 ms** (1 pass) | **15.39 %** (19.2×) | 45,261.3 ms | **10.28×** | 4,347.4 ms (**98.7 %**) | 3,771.8 ms | **0.23 %** | 1.016x / **0.976x** |

*Note on `4.43 s` vs `t173`'s `5.21 s`: `t173` ran with `--profile` plus per-call `clip_locality` probe overhead; today's clean uninstrumented Phase-3 capture (`4,434 ms` wall / `45.52 s` thread) removes that observer inflation (`28.28 s` cell wall matches `t175`'s `28.24 s` within `0.15 %`).*

### Role & call distribution (`median_pass`, classic ≡ arachne byte-exact on operands)

| Model / Role | Calls | In polys (`kept` / `clip`) | In holes | In verts | Out polys (`holes`) | Out verts | Clip thread (share) | Quantiles `p50 / p90 / p95 / p99 / max` (ms) | Max call (polys / verts) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| **`base` `ROLE_BODY`** | **431** | **136,256** (105,781 / 30,475) | 2 | **13,480,889** | 2,662 (367) | 1,214,204 | **45,536.1 ms (99.99 %)** | 85.2 / 156.6 / 291.2 / 400.0 / **414.2** | **1,024 / 102,400** |
| `base` `ROLE_ROOF` | 43 | 167 (47 / 120) | 6 | 17,044 | 54 (3) | 7,913 | 5.0 ms (0.01 %) | max 1.07 | 19 / 2,247 |
| `base` `ROLE_FLOOR` | 6 | 17 (9 / 8) | 0 | 1,700 | 7 (0) | 877 | 0.46 ms (0.00 %) | max 0.23 | 6 / 600 |
| `base` `ROLE_BASE` | 0 | 0 | 0 | 0 | 0 | 0 | 0.0 ms | — | — |
| **`benchy` `ROLE_BODY`** | **197** | **22,018** (16,873 / 5,145) | 0 | **2,184,977** | 918 (35) | 349,730 | **1,547.8 ms (99.95 %)** | 7.1 / 14.8 / 17.0 / 18.4 / **19.6** | **179 / 17,632** |
| `benchy` `ROLE_ROOF` | 2 | 8 (0 / 8) | 0 | 800 | 2 (0) | 355 | 0.13 ms (0.01 %) | max 0.07 | 4 / 400 |
| `benchy` `ROLE_FLOOR` | 3 | 32 (22 / 10) | 0 | 3,200 | 6 (0) | 1,113 | 0.63 ms (0.04 %) | max 0.30 | 14 / 1,400 |

- **Heaviest `base` layers:** lower trunk layers `L13` (`1,005` polys / `100,451` verts → `6` polys / `5,118` verts, **`414.2 ms`**), `L14` (`996` polys → `5` polys, **`413.3 ms`**), `L12` (`1,013` polys → `5` polys, **`412.0 ms`**), `L16` (`976` polys → `5` polys, **`411.5 ms`**). Every one collapses ~20× in vertices.
- **Pre-Phase-3 singletons already skipped (`len() <= 1`):** `37` (`base`), `2` (`benchy`).

---

## 2. Structural census: connected components, singletons, containment, duplicates

Derived from 100 % of captured payloads (`reduced/replay_base.json`, `reduced/replay_benchy.json`):

### Bbox-connected component size distribution (before Phase 3)

| Component size | `base` comps | `base` polys (% total) | `base` verts (% total) | `benchy` comps | `benchy` polys (% total) | `benchy` verts (% total) |
|---|---:|---:|---:|---:|---:|---:|
| **`1` (singleton)** | 530 | 530 (**0.39 %**) | 52,941 (**0.39 %**) | 63 | 63 (**0.29 %**) | 6,239 (**0.29 %**) |
| `2..4` | 459 | 1,259 (0.92 %) | 123,619 (0.92 %) | 159 | 444 (2.01 %) | 44,145 (2.02 %) |
| `5..16` | 360 | 3,043 (2.23 %) | 298,807 (2.21 %) | 292 | 2,454 (11.13 %) | 243,546 (11.13 %) |
| `17..64` | 313 | 10,880 (7.97 %) | 1,072,643 (7.95 %) | 210 | 6,160 (27.93 %) | 609,261 (27.83 %) |
| **`65..256`** | 604 | 80,591 (**59.07 %**) | 7,972,499 (**59.06 %**) | 124 | 12,937 (**58.65 %**) | 1,285,786 (**58.74 %**) |
| **`257+`** | 60 | 40,137 (**29.42 %**) | 3,979,124 (**29.48 %**) | 0 | 0 (0.00 %) | 0 (0.00 %) |
| **Total / Max** | **2,326** | **136,440** (max **1,023**) | **13,499,633** (max **102,300**) | **848** | **22,058** (max **129**) | **2,188,977** (max **12,716**) |

### Canonicality, overlap redundancy, and simplification census

- **Singleton canonicality & simplicity audit (100 % clean):** `base` **`530/530`** (`268` from `Kept`, `262` from `Clip`) and `benchy` **`63/63`** (`21` `Kept`, `42` `Clip`) pass `canonical_winding_ok`, `self_union_area_exact` (`<= 1 unit²`), and `self_union_canonical_ring_exact` (`0` failures on any check). Canonical singletons require zero Clipper2 normalization sweep.
- **Overlap depth & exact convex containment (huge discovery):**
  - `base`: `total_in_area = 4,079,313 mm²` vs `total_out_area = 534,190 mm²` (**`7.636×` area overlap factor**); **`38,351 / 136,440` (`28.11 %`, `3.74 M` verts)** are **strictly contained inside another single convex CCW sibling** in the same `(layer, role)` (`A ⊆ B ⇒ A ∪ B = B` mathematically exact!).
  - `benchy`: `378,709 mm²` vs `108,110 mm²` (**`3.503×` area overlap**); **`6,335 / 22,058` (`28.72 %`, `622 k` verts)** strictly contained inside a single larger convex sibling.
- **Phase-4 simplification:** `avg_nodes = 8` (`base`) and `5` (`benchy`), both `<= 200`, so `simplify_active_calls = 0/480` and `0/202`.
- **Exact duplicates:** `0` exact duplicate polygons within calls and `0` repeated `(layer, role)` input sets on both models.

---

## 3. Mechanism screen & geometric audit (`RAYON_NUM_THREADS=12`, 3 interleaved rounds)

All 29 mechanisms replayed on 100 % of real payloads under exact 12-layer batch boundaries (`reduced/replay_base.json`, `reduced/replay_benchy.json`). `haus` = exact symmetric vertex-to-boundary-edge Hausdorff distance (µm); `vtx` = raw nearest-vertex distance (µm, inflated by collinear tangent vertex placement along 100-gon chords).

### `base-classic-on` (480 calls, 37 batches, `13.50 M` verts; `M0` wall = `5,080.5 ms`, thread = `50,355.7 ms`)

| Mechanism | Wall med (ms) | Wall Δ vs M0 | Thread med (ms) | Thread Δ vs M0 | Prep (ms) | Calls (rounds) | Diff calls | Total XOR (mm²) | Max boundary Hausdorff (`haus`) | Raw `vtx` shift |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `M0_baseline_head_tail` | 5,080.5 | 0.00 % | 50,355.7 | 0.00 % | 38.8 | 480 (37) | 0/480 | 0 | 0.000 µm | 0.0 µm |
| `M1_subj_all_clip_empty` | 5,376.7 | −5.83 % | 53,332.4 | −5.91 % | 38.4 | 480 (37) | 0/480 | 0 | 0.000 µm | 0.0 µm |
| `M2_cc_bypass_head_tail` (Mech A) | 5,522.9* | −8.71 %* | 50,132.2* | +0.44 %* | 66.6 | 1,796 (37) | 147/480 | 1.00×10⁻³ | **0.162 µm** | 68.0 µm |
| `M3_convex_containment_prune` (Mech E) | 3,198.5 | **+37.04 %** | 28,536.0 | **+43.33 %** | 230.5 | 480 (37) | 171/480 | 9.14×10⁻⁴ | **0.224 µm** | 37.8 µm |
| `M5a_lpt_within_batch12` (Mech C) | 5,154.3 | −1.45 % | 50,844.6 | −0.97 % | 37.2 | 480 (37) | 0/480 | 0 | 0.000 µm | 0.0 µm |
| `M5b_global_single_batch` (Mech C) | 4,425.5 | +12.89 % | 52,325.9 | −3.91 % | 0.0 | 480 (1) | 0/480 | 0 | 0.000 µm | 0.0 µm |
| `M4c_hier_morton_2lvl_k16` (Guest 2-lvl) | 832.0 | **+83.62 %** | 7,382.1 | **+85.34 %** | 126.8 | 10,425 (71) | 366/480 | 7.20×10⁻³ | **0.316 µm** | 75.2 µm |
| `M4_hier_morton_k8_head_tail` (Batched) | 827.4 | **+83.71 %** | 5,953.3 | **+88.18 %** | 209.3 | 23,272 (154) | 385/480 | 8.90×10⁻³ | **0.949 µm** | 75.2 µm |
| `M7b_prune_hier_2lvl_k16` (Guest 2-lvl) | 764.6 | **+84.95 %** | 5,052.3 | **+89.97 %** | 315.9 | 7,924 (71) | 363/480 | 6.76×10⁻³ | **0.721 µm** | 96.6 µm |
| `M8_host_worker_hier_k8` (Host, no prune) | 568.9 | **+88.80 %** | 5,041.8 | **+89.99 %** | 0.0† | 23,272 (37) | 385/480 | 8.90×10⁻³ | **0.949 µm** | 75.2 µm |
| `M9_host_worker_prune_hier_k16` (Host) | 585.5 | **+88.48 %** | 4,998.6 | **+90.07 %** | 0.0† | 8,586 (37) | 363/480 | 6.96×10⁻³ | **0.721 µm** | 96.6 µm |
| `M9_host_worker_prune_hier_k8` (Host) | 463.0 | **+90.89 %** | 3,785.6 | **+92.48 %** | 0.0† | 17,131 (37) | 379/480 | 8.76×10⁻³ | **0.949 µm** | 96.6 µm |
| **`M9n_host_nested_prune_hier_k8` (PRIMARY)** | **437.7** | **+91.38 %** | **3,813.5** | **+92.43 %** | **0.0†** | 17,131 (37) | 379/480 | **8.76×10⁻³** | **0.949 µm** | 96.6 µm |
| `M9_host_worker_prune_hier_k4` (Host k=4) | **400.4** | **+92.12 %** | **3,284.2** | **+93.48 %** | 0.0† | 39,433 (37) | 395/480 | 1.03×10⁻² | 2.309 µm | 113.3 µm |
| `M9n_host_nested_prune_hier_k4` (Host k=4) | 422.5 | **+91.68 %** | 3,503.2 | **+93.04 %** | 0.0† | 39,433 (37) | 395/480 | 1.03×10⁻² | 2.309 µm | 113.3 µm |
| `M6_presimplify_50u` (Mech B — REJECT) | 4,712.0 | +7.25 % | 41,325.7 | +17.93 % | 446.6 | 480 (37) | 480/480 | **870.2** | **583.2 µm** | 813.0 µm |
| `M6_presimplify_125u` (Mech B — REJECT) | 2,671.5 | +47.42 % | 23,430.8 | +53.47 % | 280.7 | 480 (37) | 480/480 | **3,902.2** | **3,420.4 µm** | 3,420.4 µm |

*\*In earlier 3-round runs of the same binary, `M2` read `+6.35 %` / `+12.36 %` wall (`+15.6…+18.5 %` thread); because 88.5 % of vertices stay in giant components, `M2`'s small wall delta sits inside single-box noise. †For `M8`/`M9`/`M9n`, prep runs inside the 12 parallel workers and is fully included inside `Wall med`.*

### `benchy-classic-on` (202 calls, 17 batches, `2.19 M` verts; `M0` wall = `182.2 ms`, thread = `1,624.4 ms`)

| Mechanism | Wall med (ms) | Wall Δ vs M0 | Thread med (ms) | Thread Δ vs M0 | Diff calls | Total XOR (mm²) | Max boundary Hausdorff (`haus`) |
|---|---:|---:|---:|---:|---:|---:|---:|
| `M0_baseline_head_tail` | 182.2 | 0.00 % | 1,624.4 | 0.00 % | 0/202 | 0 | 0.000 µm |
| `M2_cc_bypass_head_tail` | 146.5 | +19.59 % | 1,246.0 | +23.29 % | 27/202 | 1.38×10⁻⁴ | **0.130 µm** |
| `M3_convex_containment_prune` | 119.9 | +34.21 % | 759.2 | +53.26 % | 13/202 | 3.67×10⁻⁵ | **0.120 µm** |
| `M4c_hier_morton_2lvl_k16` | 83.4 | +54.23 % | 668.7 | +58.83 % | 92/202 | 3.57×10⁻⁴ | **0.400 µm** |
| `M8_host_worker_hier_k8` | 76.5 | +58.01 % | 540.3 | +66.74 % | 121/202 | 6.53×10⁻⁴ | **0.400 µm** |
| `M9_host_worker_prune_hier_k8` | 58.7 | **+67.78 %** | 411.1 | **+74.69 %** | 115/202 | 5.33×10⁻⁴ | **0.400 µm** |
| **`M9n_host_nested_prune_hier_k8` (PRIMARY)** | **56.0** | **+69.26 %** | **420.0** | **+74.14 %** | 115/202 | **5.33×10⁻⁴** | **0.400 µm** |
| `M9_host_worker_prune_hier_k4` | 61.8 | +66.10 % | 382.5 | +76.45 % | 140/202 | 7.01×10⁻⁴ | **0.400 µm** |

---

## 4. Clipper2 distant-scanbeam proof (Question 3)

Exhaustive dummy-far-Y auditing (`t178-phase3-replay::audit_cc_scanbeam`) tested **every single differing call** of `M2` by injecting non-overlapping far-away dummy triangles (`far_x = max_x + 50 mm`) carrying the omitted sibling components' Y coordinates into each component's union call and filtering the far triangles back out:

- **`base-classic-on`:** **`147 / 147` (100.0 %) `dummy_far_y_match = true`, `0` unexplained.**
- **`benchy-classic-on`:** **`27 / 27` (100.0 %) `dummy_far_y_match = true`, `0` unexplained.**

Why raw nearest-vertex shift (`68–113 µm`) vastly overstates true geometric error (`0.16–0.95 µm` Hausdorff): when two 100-gon circles intersect at a shallow tangent angle, a 100 nm Y scanbeam clamp shifts or eliminates a nearly-collinear vertex *along* the tangent chord while moving the actual polygon boundary by `< 1 µm` (`M2`: `0.162 µm`; `M9_k8`: `0.949 µm` on `base`, `0.400 µm` on `benchy`; relative `XOR` area = `1.64 × 10⁻⁸` on `base` and `4.93 × 10⁻⁹` on `benchy`). Just like [t175](../../issues/175-tiled-collision-carve.md), any Clipper2 operand decomposition on `base.stl` therefore moves `Support` / `Support interface` hashes at the sub-micron/1 µm level (with **zero** part-geometry movement and **4/4 supports-off byte-identity**) and requires an explicit human KEEP ruling in the implementation ticket (`t182`).

---

## 5. Route pricing (`ESTIMATE ONLY`, `reduced/arithmetic.json`)

Applying each mechanism's measured offline wall cut fraction conservatively to today's clean production Phase-3 wall (`4,434.3 ms` `base-classic-on`, `4,404.6 ms` `base-arachne-on`, `154.5 ms` `benchy-classic-on`, `153.8 ms` `benchy-arachne-on`), composed against `t174 × t175` ratios:

| Mechanism | `t137` 4-cell floor? | `base-classic-on` (pre `1.221x` ord / `1.169x` acc, cut `18.1 %`) | `base-arachne-on` (pre `1.201x` ord / `1.153x` acc, cut `16.7 %`) | `benchy-classic-on` (pre `1.570x`, cut `36.3 %`) | `benchy-arachne-on` (pre `1.389x`, cut `28.0 %`) |
|---|---|---|---|---|---|
| `M2_cc_bypass` (Mech A) | **NO** | noise (`−1.37 %` to `+1.9 %`) | noise | −30 ms (`−0.88 %`, 1.1×) | −30 ms (`−0.85 %`, 1.1×) |
| `M3_containment` (Mech E) | YES (1.9–7.3×) | −1,643 ms (`−5.81 %`) → `1.150x` | −1,632 ms (`−5.70 %`) → `1.133x` | −53 ms (`−1.53 %`) → `1.546x` | −53 ms (`−1.48 %`) → `1.368x` |
| `M4c_2lvl_k16` (Guest 2-lvl) | YES (2.9–16.4×) | −3,708 ms (`−13.11 %`, 16.4×) → `1.060x` | −3,683 ms (`−12.87 %`, 16.1×) → `1.047x` | −84 ms (`−2.43 %`, 3.0×) → `1.532x` | −83 ms (`−2.34 %`, 2.9×) → `1.356x` |
| `M9_worker_k8` (Host k=8) | **YES (3.7–17.8×)** | −4,030 ms (**−14.25 %**, 17.8×) → `1.047x` (`1.003x` acc) | −4,003 ms (**−13.99 %**, 17.5×) → `1.033x` (**`0.992x` acc**) | −105 ms (**−3.04 %**, 3.8×) → `1.522x` | −104 ms (**−2.93 %**, 3.7×) → `1.348x` |
| **`M9n_nested_k8` (PRIMARY)** | **YES (3.7–17.9×)** | **−4,052 ms (−14.33 %, 17.9×) → `1.046x` (`1.002x` acc; `+t181`: `0.951x`!)** | **−4,025 ms (−14.06 %, 17.6×) → `1.032x` (`0.991x` acc; `+t181`: `0.948x`!)** | **−107 ms (−3.10 %, 3.9×) → `1.521x`** | **−107 ms (−2.99 %, 3.7×) → `1.347x`** |
| `M9_worker_k4` (Host k=4) | **YES (3.6–18.1×)** | −4,085 ms (**−14.45 %**, 18.1×) → `1.044x` (`1.000x` acc) | −4,057 ms (**−14.17 %**, 17.7×) → `1.031x` (**`0.990x` acc**) | −102 ms (**−2.96 %**, 3.7×) → `1.524x` | −102 ms (**−2.86 %**, 3.6×) → `1.349x` |

- **Why `base` supports-on needs `t182` + `t181` on binding `pnp-ordinary`:** clean Phase-3 wall is `15.68 %` / `15.39 %` of `base` supports-on against `18.07 %` / `16.75 %` required cuts, so even zeroing 100 % of Phase 3 lands at `1.029x` / `1.016x` ord (`0.986x` / `0.976x` acc). `M9n_k8` captures **79.3 % / 84.0 %** of the remaining ordinary gap (and already pushes `base-arachne-on` accelerated under `1.0x`!), leaving only **~3–4 %** (`~0.9–1.2 s`) — which stacking with **half to full `t181`** (`7.8 % / 7.0 %` post-planner host tail) closes cleanly to **`~0.95x`** (**cells 3 & 4 of 8**).

---

## 6. Methodological disclosures & verification

1. **Offline replay vs production wall:** offline replay (`M0 = 5,080.5 ms` `base`, `182.2 ms` `benchy`) ran inside the Windows sandbox while production capture (`4,434.3 ms` `base`, `154.5 ms` `benchy`) ran unsandboxed; all projections multiply the *dimensionless* offline percentage cut (`wall_vs_m0_pct`) by the clean unsandboxed production Phase-3 wall. Real end-to-end paired A/B wall/CPU is deferred to the graduated implementation ticket [t182](../../issues/182-host-spatial-hierarchical-phase3-union.md).
2. **Host-side `M9` prep inclusion:** in `M8`/`M9`/`M9n`, containment pruning, connected-component union-find, and Morton sorting run inside the Rayon workers, so their cost is fully included in `wall_ms_median` (not excluded as zero-overhead).
3. **Cross-window `ESTIMATE ONLY` ratios:** no OrcaSlicer run was performed here; all route ratios compose `t174` × `t175` × `t178` projections and are never ratios of record.
4. **Checkout-only verification:** `python docs/specs/perf-vs-orca/evidence/t178-planner-phase3-union/verify.py --self-test` exits `0` (all checks PASS, 14/14 negative controls rejected). See [`REPRODUCE.md`](REPRODUCE.md).

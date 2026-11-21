# t177 findings — post-t175 full-matrix attribution and 0.50x headroom map

Ticket: [Post-t175 full-matrix attribution and 0.50x headroom map](../../issues/177-post-t175-full-matrix-attribution.md).
Take: 2026-11-21, ordinary arm at HEAD `24848d9e` (last code commit `5b138f5c`, t175), `target/release/pnp_cli.exe` sha256 `f3075232…`, 24 external core modules under `modules/core-modules/` (tree sha256 `b46eb4cf…`, `tree-support-planner.wasm` `c56f07b3…`), `RAYON_NUM_THREADS=12`.
**Zero production code edits, zero reference moves, no candidate A/B, no OrcaSlicer scoreboard re-run.**

**Attribution only.** Every phase and sequential stage share below comes from `--instrument-stderr` (median of 3 unsandboxed passes/cell); every host per-op total and guest fuel total comes from `--profile` (1 pass/cell). Neither instrument measures uninstrumented wall, and every cross-window post-t175 ratio (`t174` quiet-board official ratio × `t175` paired saving on supports-on; `t174` ratio unchanged on structurally inert supports-off) and zero-cost ceiling is explicitly **`ESTIMATE ONLY`**, never a three-tool scoreboard ratio of record.

---

## Verdict

**Six post-t175 findings reshape the route to 8/8 `< 1.0x` and establish the ceiling map for the planned follow-on `<= 0.50x` (2× faster) effort:**

1. **Prepass fell across all 8 cells (−2.4 to −7.5 pp vs `t171`), yet still dominates every cell (`53.5–77.7 %`):** per-layer sits at `14.3–34.3 %`, postpass rose to `6.8–11.8 %` (highest on `benchy` supports-on at **11.8 %**), and outside-phases is `0.3–4.1 %`. All 32 attribution runs (`24` instrumented + `8` `--profile`) exit `0`, `degraded = false`, `0` code-1200 diagnostics, and match `t175`'s active `reference.json` **8/8** byte-for-byte.
2. **All 8 cells are now `clip_polygons`-dominated in host Clipper thread-time (reversing `t171`'s supports-off offset dominance):** `t172` + `t175` cut total `--profile` host Clipper thread-time by **−36.8…−40.3 %** on supports-off (`88.85 → 53.40 s` `base-classic-off`) and **−58.4…−59.7 %** on supports-on (`343.38 → 140.34 s` `base-classic-on`). `clip_polygons` now leads in **8/8** cells (**46.5–51.6 %** supports-off, **54.0–58.6 %** supports-on) ahead of `offset` (**30.8–37.8 %**).
3. **5 of the 6 open cells now have a single-stage `< 1.0x` ceiling, leaving ONLY binding `benchy-classic-on` (`~1.570x` est., 36.3 % cut) requiring a multi-track combination:**
   - `base-classic-on` (`~1.221x`, 18.1 % cut) & `base-arachne-on` (`~1.201x`, 16.7 % cut): `PrePass::SupportGeometry` is **44.4 % / 43.4 %** (**2.46× / 2.59×** cut; zero → **0.678x / 0.679x**). Closing ~40 % of remaining `SupportGeometry` closes both cells to **4/8**.
   - `benchy-classic-off` (`1.144x`, 12.6 % cut) & `benchy-arachne-off` (`1.155x`, 13.4 % cut): **now single-stage winnable** via `PrePass::ShellClassification` (**27.9 % / 27.0 %**, **2.22× / 2.02×** cut; zero → **0.825x / 0.843x**), OR via `per-layer` (**32.4 % / 34.3 %** → **0.774x / 0.758x**).
   - `benchy-arachne-on` (`~1.389x`, 28.0 % cut): `SupportGeometry` (**30.4 %** → **0.966x**) crosses marginally at zero (needing 92 % of the stage), so realistic closure is multi-track (`SupportGeometry` + `ShellClassification` + `tree-support` = **53.3 %** → **0.648x**).
   - `benchy-classic-on` (**binding**, `~1.570x`, 36.3 % cut): **no single stage suffices** (`SupportGeometry` **31.7 %** → **1.071x**, `per-layer` **19.2 %** → **1.268x**, `ShellClassification` **13.8 %** → **1.353x**, `postpass` **11.8 %** → **1.385x**, `SupportAnalysis` **7.9 %** → **1.446x**). Furthermore, `ShellClassification + entire per-layer` (**33.1 %** → **1.051x**) still cannot close it without cutting `SupportGeometry`. Realistic closure requires **3–4 tracks**: `SupportGeometry` (31.7 %) + `ShellClassification` (13.8 %) + `tree-support` (9.9 %) + `SupportAnalysis` (7.9 %) = **63.4 %** addressable.
4. **Two new unattributed bottlenecks surfaced directly inside the `t177` event streams:**
   - **New lead (`graduated as t181`) — `PrePass::SupportGeometry` post-planner host tail:** across all four supports-on cells, **`2,293 ms` (7.8 % cell, 17.5 % of stage)** on `base-classic-on`, **`2,017 ms` (7.0 % cell, 16.1 %)** on `base-arachne-on`, **`215 ms` (6.2 % cell, 19.6 %)** on `benchy-classic-on`, and **`216 ms` (6.1 % cell, 20.0 %)** on `benchy-arachne-on` elapse *after* `com.core.tree-support-planner::module_complete` and *before* `PrePass::SupportGeometry::stage_complete` (0 ms on supports-off). On `benchy` supports-on this host tail is larger than Phase-3 union itself.
   - **Critical input for `t179` — `benchy` `L2` `infill-linker` single-layer straggler tail + fuel-vs-wall inversion:** on `benchy` supports-off, `infill-linker` on **one single layer (`L2`, 191–226 ms vs `p50 = 3 ms`)** finishes dead last, leaving **11 of 12 cores completely idle for `134 ms` (24.4 % of `per-layer`, 7.9 % of the entire cell!)** on `benchy-classic-off` and **`104 ms` (17.0 % of `per-layer`, 5.8 % of cell!)** on `benchy-arachne-off` — closing **>60 %** of `benchy-classic-off`'s 12.6 % gap via 1:1 serial wall savings if fixed or front-loaded! Meanwhile, `perimeters` (not `infill-linker`) is **#1 in per-layer elapsed thread-time** on supports-off (`classic-perimeters` 44.8 % vs `infill-linker` 36.9 % on `benchy-classic-off`; `arachne-perimeters` 59.3 % vs `infill-linker` 28.5 % on `benchy-arachne-off`) because host Clipper/Voronoi calls are not fuel-metered.
5. **Critical input for `t180` — `PrePass::ShellClassification` parallelism halves on `benchy`:** `host:shell_classification` is 100.0 % marked Clipper2 ops (`<module self>` 102–243 µs = 0.0 %), yet its `--profile` thread / instrumented-wall ratio is **8.08–8.61×** on `base` vs only **4.50–4.57×** on `benchy` (leaving >7 of 12 cores idle on `benchy` where `ShellClassification` owns 27.0–27.9 % of the supports-off cells).
6. **`<= 0.50x` (2× faster) headroom map — NO `benchy` cell can reach `<= 0.50x` by prepass deletion alone:** zeroing the *entire* `prepass` leaves `benchy` at **0.513x / 0.521x / 0.537x / 0.501x** (all four short of `0.50x`!). Reaching `<= 0.50x` on all 8 cells structurally requires BOTH prepass AND per-layer cuts (`prepass + per-layer` zeroing leaves `0.060x–0.219x`), and on `benchy-classic-on` (`postpass + outside` = `13.8 %` → `0.217x`) even `SupportGeometry + ShellClassification + per-layer` (`64.8 %` → `0.552x`) falls short without also cutting `SupportAnalysis`, `Slice`, and/or `postpass`. On the two provisionally won `base` supports-off cells (`0.769x / 0.792x`, needing `35.0 % / 36.9 %` cuts to hit `0.50x`), `ShellClassification + per-layer` (`58.2 % / 59.5 %` → `0.321x / 0.321x`) provides ample combined headroom.

---

## 1. Stamped identities & correctness gate

| Field | Value |
| --- | --- |
| Git HEAD | `24848d9e07dbf87d0c5dade687ed34de3fc44b2e` (last code commit `5b138f5c`, t175) |
| Guest freshness check | `cargo xtask build-guests --check` exit **0** (`EXIT_FRESH`) |
| `pnp_cli.exe` | `target/release/pnp_cli.exe`, sha256 `f30752321eac2bac4a45be37a1e82f1e646515ffdb95af150ed6fb595e177fb3` |
| Core modules | 24 `.wasm` under `modules/core-modules/`, sorted tree sha256 `b46eb4cfc648bf2ec633773f43be0bb269802b5f9a0c842a8dd4e696322eb4db` (`tree-support-planner.wasm` = `c56f07b3…`) |
| Thread pin | `RAYON_NUM_THREADS=12`, `--no-default-module-paths` |
| Reference currency | [`evidence/t175-tiled-collision-carve/reference.json`](../t175-tiled-collision-carve/reference.json) |
| Run integrity | **24/24** instrumented passes + **8/8** `--profile` passes = **32/32** `exit=0`, `degraded=false`, `fatal=0`, `non_fatal=0`, `code-1200=0`, **8/8** byte-identical SHA-256 + bytes to `t175` |
| Bracket reconciliation | Phase shares sum to `1.000000` on **24/24** passes; max prepass stage-vs-phase delta **4.09 %**; max postpass stage-vs-phase delta **2.04 %** |

---

## 2. Post-t175 phase split (`--instrument-stderr`, 3 passes/cell)

Median of 3 unsandboxed passes/cell (`reduced/instrumented.json`, `reduced/arithmetic.json::split`). Instrumented `slice_complete_ms` sits **−5.7 % to +4.6 %** of `t175`'s uninstrumented candidate medians (different windows; `slice_complete_ms` excludes CLI startup/model-load I/O — attribution only, never a wall claim).

| Cell | `slice_complete` (spread) | `prepass` (Δ vs t171) | `per-layer` (Δ vs t171) | `postpass` (Δ vs t171) | `outside` | `validation` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | 1,700 ms (1.74 pp) | **55.1 %** (−3.1 pp) | **32.4 %** (−0.0 pp) | 8.2 % (+2.1 pp) | 4.1 % | 0.18 % |
| `benchy-classic-on` | 3,452 ms (0.66 pp) | **66.8 %** (−4.8 pp) | **19.2 %** (+0.7 pp) | **11.8 %** (+3.4 pp) | 2.0 % | 0.09 % |
| `benchy-arachne-off` | 1,783 ms (0.76 pp) | **53.5 %** (−2.4 pp) | **34.3 %** (−0.4 pp) | 8.1 % (+1.9 pp) | 3.9 % | 0.17 % |
| `benchy-arachne-on` | 3,553 ms (5.04 pp) | **63.9 %** (−6.1 pp) | **22.1 %** (+1.7 pp) | **11.8 %** (+3.7 pp) | 2.0 % | 0.08 % |
| `base-classic-off` | 9,913 ms (1.84 pp) | **70.9 %** (−5.1 pp) | **21.3 %** (+2.7 pp) | 7.0 % (+2.0 pp) | 0.8 % | 0.03 % |
| `base-classic-on` | 29,533 ms (4.32 pp) | **77.7 %** (−6.0 pp) | **14.3 %** (+2.2 pp) | 7.7 % (+3.5 pp) | 0.3 % | 0.01 % |
| `base-arachne-off` | 10,591 ms (1.83 pp) | **66.9 %** (−7.5 pp) | **25.2 %** (+5.0 pp) | 6.8 % (+2.0 pp) | 0.8 % | 0.03 % |
| `base-arachne-on` | 28,879 ms (0.65 pp) | **76.7 %** (−6.5 pp) | **16.1 %** (+3.5 pp) | 7.3 % (+3.3 pp) | 0.3 % | 0.01 % |

**Postpass stage detail (`reduced/arithmetic.json::split`):**
- `benchy` supports-on (`11.8 %` total): `PostPass::LayerFinalization` **4.8–5.1 %** (`166–181 ms`), `PostPass::GCodeSerialize` **2.9–3.0 %** (`103–104 ms`), `PostPass::GCodePostProcess` **2.6–2.7 %** (`89–95 ms`), `PostPass::GCodeEmit` **1.3–1.4 %** (`45–47 ms`).
- `base` supports-on (`7.3–7.7 %` total): `LayerFinalization` **2.3–2.6 %** (`673–776 ms`), `GCodeSerialize` **1.9 %** (`537–553 ms`), `GCodeEmit` **1.5 %** (`442–446 ms`), `GCodePostProcess` **1.4 %** (`393–411 ms`).
- Supports-off (`6.8–8.2 %` total): `GCodeSerialize` **2.4–2.6 %**, `GCodePostProcess` **1.9–2.5 %**, `LayerFinalization` **1.7–2.3 %**, `GCodeEmit` **0.6–0.7 %**.

---

## 3. Sequential prepass stage split & single-stage `< 1.0x` ceilings (`ESTIMATE ONLY`)

Cross-window route ratios (`reduced/arithmetic.json::route`, matching `t175/reduced/route.json` leaf-for-leaf): `t174` quiet-board official ratio × `(1 − t175_saving)` on supports-on, `t174` official ratio unchanged on structurally inert supports-off. `ratio_if_zero = est_ratio_ord × (1 − stage_share)`.

### Supports-off cluster

| Cell (`est_ord` / `est_acc`, `<1x` cut) | `ShellClassification` | `Slice` | `SeamPlanning` | `MeshAnalysis` | `SupportGeometry` | `OverhangAnnotation` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` (`1.144x` / `1.118x`, **12.6 %**) | **27.9 % → 0.825x** (474 ms) | 9.4 % → 1.036x (160 ms) | 6.5 % → 1.069x (111 ms) | 5.1 % → 1.085x (87 ms) | 2.2 % → 1.118x (38 ms) | 1.1 % → 1.131x (19 ms) |
| `benchy-arachne-off` (`1.155x` / `1.064x`, **13.4 %**) | **27.0 % → 0.843x** (482 ms) | 9.1 % → 1.049x (163 ms) | 6.2 % → 1.083x (111 ms) | 4.8 % → 1.099x (86 ms) | 2.2 % → 1.129x (39 ms) | 1.0 % → 1.143x (18 ms) |
| `base-classic-off` (**`0.769x`** / `0.766x`, **WIN**) | **37.0 % → 0.485x** (3,664 ms) | 14.0 % → 0.661x (1,392 ms) | 8.0 % → 0.707x (790 ms) | 6.1 % → 0.722x (606 ms) | 2.3 % → 0.751x (225 ms) | 1.6 % → 0.756x (158 ms) |
| `base-arachne-off` (**`0.792x`** / `0.774x`, **WIN**) | **34.2 % → 0.521x** (3,624 ms) | 13.8 % → 0.683x (1,458 ms) | 7.5 % → 0.733x (790 ms) | 5.8 % → 0.746x (611 ms) | 2.0 % → 0.776x (215 ms) | 1.5 % → 0.780x (163 ms) |

### Supports-on cluster

| Cell (`est_ord` / `est_acc`, `<1x` cut) | `SupportGeometry` | `ShellClassification` | `SupportAnalysis` | `Slice` | `SeamPlanning` | `MeshAnalysis` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-on` (**`1.570x`** / `1.602x`, **36.3 %**, **BINDING**) | **31.7 % → 1.071x** (1,096 ms) | 13.8 % → 1.353x (478 ms) | 7.9 % → 1.446x (273 ms) | 4.7 % → 1.496x (163 ms) | 3.3 % → 1.518x (114 ms) | 2.5 % → 1.531x (86 ms) |
| `benchy-arachne-on` (`1.389x` / `1.409x`, **28.0 %**) | **30.4 % → 0.966x** (1,080 ms) | 13.4 % → 1.202x (477 ms) | 7.8 % → 1.280x (277 ms) | 4.6 % → 1.324x (165 ms) | 3.1 % → 1.346x (109 ms) | 2.4 % → 1.355x (86 ms) |
| `base-classic-on` (`1.221x` / `1.169x`, **18.1 %**) | **44.4 % → 0.678x** (13,125 ms) | 12.4 % → 1.069x (3,676 ms) | 10.0 % → 1.098x (2,968 ms) | 4.8 % → 1.162x (1,419 ms) | 3.0 % → 1.184x (878 ms) | 2.0 % → 1.196x (593 ms) |
| `base-arachne-on` (`1.201x` / `1.153x`, **16.7 %**) | **43.4 % → 0.679x** (12,545 ms) | 12.9 % → 1.046x (3,720 ms) | 9.5 % → 1.087x (2,740 ms) | 5.0 % → 1.141x (1,455 ms) | 2.7 % → 1.168x (791 ms) | 2.0 % → 1.177x (591 ms) |

**New `SupportGeometry` post-planner host tail (`stage_complete − com.core.tree-support-planner::module_complete`, median of 3 passes):**

| Cell | `PrePass::SupportGeometry` | `tree-support-planner` | `traditional-support-planner` | **Post-planner host tail** | Share of stage | **Share of cell** | Est. wall (t174 scale) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-on` | 1,096 ms | 849 ms | 33 ms | **215 ms** | **19.6 %** | **6.2 %** | 0.26 s |
| `benchy-arachne-on` | 1,080 ms | 831 ms | 33 ms | **216 ms** | **20.0 %** | **6.1 %** | 0.27 s |
| `base-classic-on` | 13,125 ms | 10,623 ms | 209 ms | **2,293 ms** | **17.5 %** | **7.8 %** | 2.91 s |
| `base-arachne-on` | 12,545 ms | 10,330 ms | 208 ms | **2,017 ms** | **16.1 %** | **7.0 %** | 2.53 s |

On supports-off cells this tail is **0 ms**. It sits outside `com.core.tree-support-planner` (host commit / `SupportGeometryIR` decode / `validate_entry` exact-Z occupancy gate / drop), making it a distinct lever from [t178](../../issues/178-planner-phase3-union-attribution.md)'s Phase-3 union — graduated as [t181](../../issues/181-support-geometry-post-planner-host-tail.md).

---

## 4. Host Clipper per-op census & deterministic guest fuel (`--profile`, 1 pass/cell)

`--profile` host times are **accumulated thread-time** (inflated by mark overhead, summed across concurrent workers — never wall). Guest fuel is **deterministic executed WASM instructions**.

### Host Clipper thread-time by operation (`reduced/arithmetic.json::census`)

| Cell | Host total (vs t171) | `clip_polygons` | `offset` | `offset2_ex` | `closing_ex` | Dominant op |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `benchy-classic-off` | **4.51 s** (0.597×) | **51.2 %** (2.31 s) | 32.0 % (1.44 s) | 13.6 % (0.61 s) | 3.2 % (0.15 s) | `clip_polygons` |
| `benchy-arachne-off` | **4.54 s** (0.632×) | **51.6 %** (2.35 s) | 30.8 % (1.40 s) | 14.1 % (0.64 s) | 3.6 % (0.16 s) | `clip_polygons` |
| `base-classic-off` | **53.40 s** (0.601×) | **46.5 %** (24.86 s) | 37.8 % (20.17 s) | 12.8 % (6.83 s) | 2.9 % (1.54 s) | `clip_polygons` |
| `base-arachne-off` | **50.13 s** (0.608×) | **46.9 %** (23.49 s) | 37.0 % (18.55 s) | 13.1 % (6.58 s) | 3.0 % (1.50 s) | `clip_polygons` |
| `benchy-classic-on` | **8.56 s** (0.416×) | **54.0 %** (4.62 s) | 37.2 % (3.18 s) | 7.1 % (0.60 s) | 1.8 % (0.15 s) | `clip_polygons` |
| `benchy-arachne-on` | **8.65 s** (0.406×) | **54.0 %** (4.67 s) | 36.9 % (3.19 s) | 7.3 % (0.63 s) | 1.8 % (0.16 s) | `clip_polygons` |
| `base-classic-on` | **140.34 s** (0.409×) | **57.6 %** (80.81 s) | 36.4 % (51.02 s) | 4.9 % (6.92 s) | 1.1 % (1.59 s) | `clip_polygons` |
| `base-arachne-on` | **138.42 s** (0.403×) | **58.6 %** (81.14 s) | 35.1 % (48.62 s) | 5.2 % (7.15 s) | 1.1 % (1.47 s) | `clip_polygons` |

### Host Clipper owner split (representative cells)

| Host built-in | `benchy-classic-off` | `benchy-classic-on` | `base-classic-off` | `base-classic-on` |
| --- | --- | --- | --- | --- |
| `host:native` | 1.28 s (10,302: clip 0.92, off 0.36) | **4.51 s** (41,722: clip **3.04**, off 1.47) | 10.78 s (22,077: clip 6.89, off 3.89) | **76.50 s** (134,563: clip **59.29**, off 17.21) |
| `host:shell_classification` | **2.17 s** (4,086: clip 0.84, off 0.72, o2 0.46, cl 0.15) | 2.16 s (4,086: clip 0.85, off 0.73, o2 0.44, cl 0.15) | **30.65 s** (8,912: off 12.93, clip 10.84, o2 5.35, cl 1.54) | 31.65 s (8,912: off 13.27, clip 11.34, o2 5.45, cl 1.59) |
| `host:support_analysis` | 0 s | 0.86 s (1,366: off 0.62, clip 0.25) | 0 s | **20.47 s** (4,430: **off 17.26**, clip 3.21) |
| `host:slice` | 0.82 s (1,410: off 0.36, clip 0.27, o2 0.16) | 0.77 s (1,410: off 0.37, clip 0.24, o2 0.17) | 7.29 s (2,954: off 3.35, clip 2.45, o2 1.49) | 7.21 s (2,954: off 3.28, clip 2.46, o2 1.47) |
| `host:mesh_analysis` | 0.08 s (1: clip 0.08) | 0.08 s (1: clip 0.08) | 2.78 s (49: clip 2.78) | 2.78 s (49: clip 2.78) |
| `host:overhang_annotation` | 0.19 s (239: clip 0.19) | 0.18 s (239: clip 0.18) | 1.90 s (495: clip 1.90) | 1.73 s (495: clip 1.73) |

### Deterministic guest WASM fuel & per-layer elapsed thread-time (`reduced/arithmetic.json::census` + `split`)

| Cell | Total fuel (Δ vs t171) | Top guest modules (fuel share) | Per-layer wall (`eff_par`) | Top per-layer modules (thread share → proportional cell share) | Single-layer straggler tail |
| --- | ---: | --- | ---: | --- | --- |
| `benchy-classic-off` | 20.27 B (−16.3 %) | `infill-linker` **50.7 %** (10.27 B), `classic-perimeters` 38.6 % (7.82 B) | 550 ms (**4.43×**) | `classic-perimeters` **44.8 % → 14.5 %**, `infill-linker` 36.9 % → 11.9 %, `raft-default`* 10.7 % → 3.5 % | **`L2` `infill-linker` 191 ms → 134 ms idle tail (7.9 % cell!)** |
| `benchy-classic-on` | 68.74 B (−9.1 %) | `tree-support` **61.0 %** (41.94 B), `infill-linker` 14.9 %, `classic-perimeters` 11.4 %, `tree-support-planner` 8.5 % (5.85 B) | 660 ms (7.84×) | `tree-support` **51.7 % → 9.9 %**, `classic-perimeters` 20.4 % → 3.9 %, `infill-linker` 18.8 % → 3.6 % | `L2` `infill-linker` 292 ms (ends last, 1 ms tail) |
| `benchy-arachne-off` | 20.48 B (−14.5 %) | `infill-linker` **54.2 %** (11.11 B), `arachne-perimeters` 34.8 % (7.13 B) | 612 ms (**5.71×**) | `arachne-perimeters` **59.3 % → 20.3 %**, `infill-linker` 28.5 % → 9.8 %, `raft-default`* 7.2 % → 2.5 % | **`L2` `infill-linker` 226 ms → 104 ms idle tail (5.8 % cell!)** |
| `benchy-arachne-on` | 68.95 B (−8.5 %) | `tree-support` **60.8 %** (41.94 B), `infill-linker` 16.1 %, `arachne-perimeters` 10.3 %, `tree-support-planner` 8.5 % (5.85 B) | 784 ms (7.92×) | `tree-support` **43.0 % → 9.5 %**, `arachne-perimeters` 32.7 % → 7.2 %, `infill-linker` 16.5 % → 3.6 % | `L57` `tree-support` 27 ms (4 ms tail) |
| `base-classic-off` | 123.27 B (−11.2 %) | `classic-perimeters` **61.5 %** (75.81 B), `infill-linker` 30.4 % (37.46 B) | 2,085 ms (8.51×) | `classic-perimeters` **49.0 % → 10.3 %**, `infill-linker` 22.1 % → 4.7 %, `raft-default`* 11.9 % → 2.5 % | 1 ms (balanced) |
| `base-classic-on` | 563.92 B (−4.9 %) | `tree-support` **68.9 %** (388.79 B), `classic-perimeters` 13.4 %, `tree-support-planner` 8.6 % (48.56 B), `infill-linker` 6.6 % | 4,212 ms (10.02×) | `tree-support` **58.5 % → 8.3 %**, `classic-perimeters` 20.2 % → 2.9 %, `infill-linker` 9.2 % → 1.3 % | 14 ms (balanced) |
| `base-arachne-off` | 112.55 B (−4.7 %) | `arachne-perimeters` **55.3 %** (62.22 B), `infill-linker` 35.9 % (40.36 B) | 2,661 ms (9.09×) | `arachne-perimeters` **60.8 % → 15.3 %**, `infill-linker` 17.2 % → 4.3 %, `raft-default`* 8.7 % → 2.2 % | 0 ms (balanced) |
| `base-arachne-on` | 553.19 B (−3.3 %) | `tree-support` **70.3 %** (388.79 B), `arachne-perimeters` 11.2 %, `tree-support-planner` 8.8 % (48.56 B), `infill-linker` 7.3 % | 4,693 ms (10.26×) | `tree-support` **50.5 % → 8.2 %**, `arachne-perimeters` 30.0 % → 4.9 %, `infill-linker` 8.7 % → 1.4 % | 4 ms (balanced) |

*\*Note on `com.core.raft-default`: `raft-default` burns only 0.2–0.3 % of guest fuel (rafts are off), yet accounts for `252–284 ms` (benchy) / `2,107–2,213 ms` (base) of `per-layer` thread-time because `first_in_stage` credits `99 %` of its elapsed time (`2,093 / 2,101 ms` on `base-classic-off`) when it is the first module scheduled in `Layer::Infill` — paying the host prepared-region projection cost (`t155`), not raft compute.*

---

## 5. Multi-track `< 1.0x` closure and `<= 0.50x` (2× faster) headroom map (`ESTIMATE ONLY`)

All ratios below are `ESTIMATE ONLY` on the binding `pnp-ordinary` arm (`pnp-accelerated` disclosed in `reduced/arithmetic.json::combination_budget`), using `t174` quiet-board Orca medians (`t174` PnP/Orca CPU ratios disclosed alongside per [t176](../../issues/176-destination-cpu-corroboration-ruling.md)).

| Cell | Est. ratio (`t174` CPU) | Cut `<1.0x` | Cut `<=0.50x` | Best single stage (`ratio_if_zero`) | `prepass` zero | `SG + Shell` zero | `SG + tree-sup + Shell` zero | `Shell + per-layer` zero | `SG + Shell + per-layer` zero | `prepass + per-layer` zero |
| --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | 1.144x (1.48x) | **12.6 %** | 56.3 % | `Shell` 27.9 % → **0.825x** | 0.513x (no 0.5x) | 0.799x | 0.796x | **0.455x** (0.5x) | 0.429x | **0.143x** |
| `benchy-arachne-off` | 1.155x (1.28x) | **13.4 %** | 56.7 % | `Shell` 27.0 % → **0.843x** | 0.537x (no 0.5x) | 0.817x | 0.815x | **0.446x** (0.5x) | 0.421x | **0.141x** |
| **`benchy-classic-on`** | **1.570x** (2.00x) | **36.3 %** | **68.2 %** | `SG` 31.7 % → **1.071x** (short!) | 0.521x (no 0.5x) | **0.854x** | **0.699x** | 1.051x (short!) | 0.552x (no 0.5x) | **0.219x** |
| `benchy-arachne-on` | 1.389x (1.86x) | **28.0 %** | 64.0 % | `SG` 30.4 % → **0.966x** (92 %!) | 0.501x (no 0.5x) | **0.780x** | **0.648x** | **0.896x** | **0.474x** (0.5x) | **0.195x** |
| `base-classic-off` | **0.769x** (2.13x) | **WIN** | **35.0 %** | `Shell` 37.0 % → **0.485x** (0.5x) | **0.224x** | 0.467x | 0.463x | **0.321x** (0.5x) | 0.304x | **0.060x** |
| `base-arachne-off` | **0.792x** (1.84x) | **WIN** | **36.9 %** | `Shell` 34.2 % → 0.521x | **0.262x** | 0.505x | 0.501x | **0.321x** (0.5x) | 0.305x | **0.062x** |
| `base-classic-on` | 1.221x (2.94x) | **18.1 %** | 59.0 % | `SG` 44.4 % → **0.678x** | **0.272x** | 0.526x | **0.424x** (0.5x) | **0.895x** | **0.352x** (0.5x) | **0.098x** |
| `base-arachne-on` | 1.201x (2.72x) | **16.7 %** | 58.4 % | `SG` 43.4 % → **0.679x** | **0.280x** | 0.525x | **0.426x** (0.5x) | **0.853x** | **0.332x** (0.5x) | **0.087x** |

**Key strategic readings:**
- **Path to 4/8 (`base` supports-on):** `t178` (Phase-3 union inside `SupportGeometry`: ~51.6 s of 59.3 s `host:native` clip thread) + `t181` (post-planner host tail inside `SupportGeometry`: **7.0–7.8 %** of cell) directly address **>25 %** of cell wall against a **16.7–18.1 %** cut.
- **Path to 6/8 (`benchy` supports-off):** `t180` (`ShellClassification`: **27.0–27.9 %** of cell, running at only **4.5×** parallelism on `benchy`) + `t179` (`L2` `infill-linker` straggler tail: **5.8–7.9 %** serial idle-core wall + `infill-linker`/`perimeters` kernel cuts) easily clear **12.6–13.4 %**.
- **Path to 8/8 (`benchy` supports-on, binding `benchy-classic-on` 36.3 %):** requires stacking wins across **all four open frontier tracks**: `t178` (Phase-3 union) + `t181` (`SupportGeometry` post-planner host tail, **6.1–6.2 %**) + `t180` (`ShellClassification`, **13.4–13.8 %**) + `t179` (`tree-support` **9.5–9.9 %** + `infill-linker` **3.6 %**), with `PrePass::SupportAnalysis` (**7.8–7.9 %** benchy-on, **9.5–10.0 %** base-on) and `PostPass::LayerFinalization` (**4.8–5.1 %** benchy-on) available as reserve levers if needed.
- **Path to follow-on `<= 0.50x` (fresh map after 8/8):** **structurally impossible from prepass alone on `benchy`** (`prepass` zeroing leaves `0.501x–0.537x`). Achieving `<= 0.50x` across 8/8 requires high-leverage cuts across BOTH prepass (`SupportGeometry`, `ShellClassification`, `SupportAnalysis`, `Slice`) AND per-layer (`tree-support`, `classic/arachne-perimeters`, `infill-linker`) plus trimming `benchy-classic-on`'s `11.8 %` postpass.

---

## 6. Sharpened inputs for unblocked tickets `178`, `179`, `180`, and new `181`

All numbers below come directly from `reduced/arithmetic.json::ticket_inputs`:

- **[t178](../../issues/178-planner-phase3-union-attribution.md) (`SupportGeometry` Phase-3 union):** `PrePass::SupportGeometry` is **44.4 % / 43.4 %** (`13,125 / 12,545 ms`, est. `16.69 / 15.74 s`) on `base` supports-on and **31.7 % / 30.4 %** (`1,096 / 1,080 ms`, est. `1.35 / 1.34 s`) on `benchy` supports-on. Inside it, `com.core.tree-support-planner` itself is `10,623 / 10,330 ms` (`48.56 B` fuel = `8.6–8.8 %`) on `base` and `849 / 831 ms` (`5.85 B` = `8.5 %`) on `benchy`; `host:native` clip thread is **59.29 / 59.91 s** (`75,191 / 75,686` calls) on `base` and **3.04 / 3.05 s** (`23,691 / 23,904` calls) on `benchy`.
- **[t181](../../issues/181-support-geometry-post-planner-host-tail.md) (NEW — `SupportGeometry` post-planner host tail):** `2,293 / 2,017 ms` (**7.8 / 7.0 %** of cell, **17.5 / 16.1 %** of `SupportGeometry`) on `base` supports-on and `215 / 216 ms` (**6.2 / 6.1 %** of cell, **19.6 / 20.0 %** of stage) on `benchy` supports-on sitting between `com.core.tree-support-planner::module_complete` and `PrePass::SupportGeometry::stage_complete`.
- **[t179](../../issues/179-per-layer-tree-support-and-infill-linker-screen.md) (UNBLOCKED — `tree-support` & `infill-linker`):**
  - `com.core.tree-support`: **61.0 / 60.8 %** of guest fuel (`41.94 B`) and **51.7 / 43.0 %** of per-layer thread (`2,677 / 2,673 ms` → **9.9 / 9.5 %** implied cell share, est. `0.42 s`) on `benchy` supports-on; **68.9 / 70.3 %** of fuel (`388.79 B`) and **58.5 / 50.5 %** of per-layer thread (`24,698 / 24,340 ms` → **8.3 / 8.2 %** implied cell share, est. `3.12 / 2.97 s`) on `base` supports-on (`96.8–97.1 %` `<module self>`, `2.9–3.2 %` in-guest `clip_polygons`).
  - `com.core.infill-linker`: **50.7 / 54.2 %** of guest fuel (`10.27 / 11.11 B`) on `benchy` supports-off (`100.0 %` `<module self>`), **36.9 / 28.5 %** of per-layer thread (`897 / 996 ms` → **11.9 / 9.8 %** proportional cell share), **PLUS** the **134 ms (`7.9 %` cell) / 104 ms (`5.8 %` cell) `L2` single-layer straggler tail** where 11/12 cores sit idle waiting on `L2` (`191 / 226 ms` vs `p50 = 3 ms`). On supports-on: **14.9–16.1 %** fuel / **3.6 %** implied cell (`benchy`, where `L2` takes `292–306 ms`!) and **6.6–7.3 %** fuel / **1.3–1.4 %** implied cell (`base`).
  - Perimeter generators (for completeness, since they rank #1 in supports-off per-layer thread): `classic-perimeters` **14.5 %** implied cell (`benchy-classic-off`, `7.82 B` fuel, `19.3 %` `offset2_ex`) and **10.3 %** (`base-classic-off`); `arachne-perimeters` **20.3 %** (`benchy-arachne-off`, `7.13 B` fuel) and **15.3 %** (`base-arachne-off`).
- **[t180](../../issues/180-shell-classification-post-t172-residual-screen.md) (UNBLOCKED — `ShellClassification` & sibling prepass stages):**
  - `PrePass::ShellClassification`: **27.9 / 27.0 %** (`474 / 482 ms`, est. `0.66 / 0.73 s`, **0.825x / 0.843x** at zero) on `benchy` supports-off; **13.8 / 13.4 %** (`478 / 477 ms`, est. `0.59 s`) on `benchy` supports-on; **37.0 / 34.2 %** (`3,664 / 3,624 ms`, est. `5.03 / 5.09 s`, **0.485x / 0.521x** at zero) on `base` supports-off; **12.4 / 12.9 %** (`3,676 / 3,720 ms`, est. `4.67 s`) on `base` supports-on. `host:shell_classification` `<module self>` is `0.0 %` (`102–243 µs`); `--profile` thread / instrumented-wall ratio is **8.08–8.61×** on `base` vs only **4.50–4.57×** on `benchy` — flagging low `benchy` concurrency as a primary screen target alongside op volume (`clip_polygons` 39 % / `offset` 33 % / `offset2_ex` 21 % / `closing_ex` 7 % on `benchy`; `offset` 42 % / `clip_polygons` 35 % / `offset2_ex` 17 % / `closing_ex` 5 % on `base`).
  - Sibling prepass stages: `PrePass::Slice` **9.1–9.4 %** (`benchy-off`, `4.7–5.1×`), **13.8–14.0 %** (`base-off`, `4.5–5.2×`), **4.6–5.0 %** (supports-on); `PrePass::SupportAnalysis` **7.8–7.9 %** (`benchy-on`, `3.2–3.4×`), **9.5–10.0 %** (`base-on`, `6.9–7.0×`, `host:support_analysis` **84 % `offset`**); `PrePass::SeamPlanning` **6.2–8.0 %** (off) / **2.7–3.3 %** (on); `PrePass::MeshAnalysis` **4.8–6.1 %** (off) / **2.0–2.5 %** (on); `PrePass::OverhangAnnotation` **0.5–1.6 %** (`10.0–12.0×`).

---

## 7. Methodological disclosures

1. **Cross-window `ESTIMATE ONLY` route chain:** every post-t175 ratio and `ratio_if_zero` composes `t174`'s quiet-window official ratio with `t175`'s paired supports-on wall saving and `t177`'s instrumented shares. No OrcaSlicer run was performed here; per `t161`, no cross-window ratio is a ratio of record.
2. **Proportional per-layer module shares:** `per-layer` runs layers concurrently on 12 Rayon threads (`eff_par` `4.43–10.26×`), so per-module `implied_wall_share_of_cell` distributes `module_busy_union_ms / slice_complete_ms` proportionally by `module_thread_sum_ms`. On `benchy` supports-off, the measured `L2` `infill-linker` single-layer straggler tail (`134 / 104 ms`) demonstrates that proportional shares *understate* critical-path tail modules and is reported explicitly beside them.
3. **Discarded sandboxed first capture:** an initial capture executed inside the Windows sandbox inflated `per-layer` by ~2.8× on `benchy` (`benchy-classic-off` `3.93 s` vs `1.72 s` uninstrumented, blowing up short per-layer WASM calls via sandbox syscall/stderr-write overhead). Those raw streams were discarded and all 32 runs in this evidence set were captured cleanly unsandboxed (`slice_complete_ms` within `−5.7 % to +4.6 %` of `t175`'s uninstrumented medians).
4. **No accelerated timing arm:** per ticket scope, attribution ran on the binding `pnp-ordinary` arm only; `pnp-accelerated` zero-stage ratios in `reduced/arithmetic.json` apply the ordinary instrumented shares to `t174 × t175` accelerated ratios as an estimate.

---

## 8. Verification & reproduction

- `python verify.py` — **PASS** (5/5 checks: `payload_recomputes` + cross-check vs `t175/reduced/route.json`, `runs_match_reference`, `instrumented_valid`, `profile_valid`, `brackets_reconcile`).
- `python verify.py --self-test` — **PASS** (**12/12** negative controls rejected).
- `cargo xtask build-guests --check` — exit **0**.

See [`REPRODUCE.md`](REPRODUCE.md) for the exact command sequence.

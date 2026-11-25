# t186 findings — post-t185 full-matrix attribution and 8/8 + 0.50x headroom map

Ticket: [Post-t185 full-matrix attribution and 8/8 closure headroom map](../../issues/186-post-t185-full-matrix-attribution.md).
Take: 2026-11-23, ordinary arm at HEAD `92b62d9a` (last code commit `748efbf5`, t185), `target/release/pnp_cli.exe` sha256 `e661baa5…`, 24 external core modules under `modules/core-modules/` (tree sha256 `8d33516c…`, `tree-support-planner.wasm` `cf917686…`), `RAYON_NUM_THREADS=12`, unsandboxed.
**Zero production code edits, zero reference moves, no candidate A/B, no OrcaSlicer scoreboard re-run.**

**Attribution only.** Every phase and sequential stage share below comes from `--instrument-stderr` (median of 3 unsandboxed passes/cell); every host per-op total and guest fuel total comes from `--profile` (1 unsandboxed pass/cell). Neither instrument measures uninstrumented wall, and every cross-window post-t185 ratio (`t185/reduced/route.json`: `t174 × t175 × t182 × t183 × t184 × t185`) and zero-cost ceiling is explicitly **`ESTIMATE ONLY`**, never a three-tool scoreboard ratio of record.

---

## Verdict

**Six post-t185 findings chart the final push to 8/8 `< 1.0x` (closing `benchy-classic-on` at est. `1.204x` / `16.9 %` cut and `benchy-arachne-on` at est. `1.085x` / `7.8 %` cut) and refresh the `<= 0.50x` headroom map:**

1. **All 32 attribution runs (`24` instrumented + `8` `--profile`) pass 100 % cleanly** (`exit=0`, `degraded=false`, `fatal=0`, `non_fatal=0`, `code-1200=0`, byte-identical SHA-256 + byte size to `t182`'s active `reference.json`). Compared to `t177`, instrumented `slice_complete_ms` dropped **−14.9…−22.5 %** on `benchy` (`benchy-classic-off` `1,700 → 1,317 ms`, `benchy-classic-on` `3,452 → 2,801 ms`) and **−30.5…−33.2 %** on `base` supports-on (`29,533 → 19,737 ms`, `28,879 → 20,073 ms`).
2. **All four `t182`–`t185` targets are visibly cleared in the event streams:**
   - **Deterministic guest fuel fell −69.9…−74.0 % on supports-on** (`68.74 → 20.72 B` `benchy-classic-on`, `563.92 → 156.91 B` `base-classic-on`) and **−24.1…−40.4 %** on supports-off (`t183`). `com.core.tree-support` dropped out of the top slot; `com.core.tree-support-planner` is now #2 in guest fuel on supports-on (**28.2–28.9 %** `benchy`, **31.0–33.7 %** `base`).
   - **Host Clipper thread-time fell −38.4 % on `base-classic-on` (`140.34 → 86.46 s`), flipping `base` supports-on BACK to `offset`-dominant** (**51.1–51.9 %** `offset` vs **38.4–39.6 %** `clip_polygons`), while `benchy` supports-on is nearly tied (`47.9–48.2 %` clip vs `41.4–41.9 %` offset).
   - **`PrePass::SupportGeometry` post-planner host tail is gone** (`215/216 → 4/4 ms` `benchy-on`, `2,293/2,017 → 17/17 ms` `base-on`; `t185`).
   - **`L2` single-layer `infill-linker` straggler tail is gone** (`134/104 → 14/3 ms` `benchy-off`, `5/4 ms` `benchy-on`; `t183`).
   - **`PrePass::ShellClassification` halved on `benchy` (`474–482 → 217–236 ms`)** as parallelism jumped from `4.5×` to **`7.3–8.0×`** (`t184`).
3. **Only ONE single stage can close binding `benchy-classic-on` (`1.204x`, `16.9 %` cut) alone: `PrePass::SupportGeometry` (`825 ms`, `29.45 %` = `36.8×` floor → `0.849x`; `benchy-arachne-on` `27.50 %` → `0.786x`; `base-on` `30.4–33.0 %` = `38.1–41.2×` floor).**
   - Inside it, `com.core.tree-support-planner` owns **`785 / 791 ms`** (`benchy-on`) and **`6,296 / 5,884 ms`** (`base-on`), burning **`5.85 B` / `48.56 B`** guest fuel plus `host:native` `offset` (**`1.43 s` / `17.49 s`**) and remaining `clip_polygons` (**`1.72 s` / `14.82 s`**). Graduated as [t187](../../issues/187-tree-support-planner-post-t182-residual-screen.md) (qualifies for standing `t137` 4-cell planner gate).
4. **`PrePass::SupportAnalysis` is now #2 in prepass on all four supports-on cells (`292 / 290 ms` = `10.42 / 9.60 %` `benchy-on`, `2,853 / 2,892 ms` = `14.46 / 14.41 %` `base-on`, `12.0–18.1×` floor) and runs at only `2.95–3.00×` concurrency on `benchy` (vs `6.47–7.44×` on `base`).**
   - `t181` already observed single-threaded `slice_mesh_ex` inside `collect_print_plane_occupancy` (`~85 ms` `benchy` / `~745 ms` `base`), and `host:support_analysis` is **84 % `offset`** (`0.61 s` `benchy` / `15.54–18.11 s` `base`). Stacking `SupportGeometry + SupportAnalysis` addresses **`39.9 %`** of `benchy-classic-on` (`→ 0.724x`), **`37.1 %`** of `benchy-arachne-on` (`→ 0.682x`), and **`47.4 %`** of `base-classic-on` (`→ 0.443x`). Graduated as [t188](../../issues/188-support-analysis-attribution-and-speedup-screen.md) (qualifies for standing `t137` 4-cell planner gate; `0 ms` on supports-off).
5. **`PostPass` surged to `14.36–15.09 %` (`403–449 ms`) on `benchy` supports-on — now larger than `SupportAnalysis` and `ShellClassification` — and clears the `0.80 %` standing gate floor in ALL 8 CELLS (`7.56–15.09 %`):**
   - `PostPass::LayerFinalization`: **`6.21–6.65 %`** (`174–201 ms`) `benchy-on`, `3.40–3.49 %` `base-on`, `2.93–2.96 %` `benchy-off`, `1.83–1.89 %` `base-off` (**2.3–8.3× floor in 8/8**).
   - `PostPass::GCodeSerialize`: **`3.50–3.74 %`** (`98–113 ms`) `benchy-on`, `2.67–2.96 %` `base-on`, `2.73–3.49 %` supports-off (**3.3–4.7× floor in 8/8**).
   - `PostPass::GCodePostProcess`: **`2.91–2.93 %`** (`82–88 ms`) `benchy-on`, `2.10–2.17 %` `base-on`, `2.23–3.04 %` supports-off (**2.6–3.8× floor in 8/8**).
   - Cutting `PostPass` helps close `benchy-on` AND widens `benchy-off`'s `< 1.0x` margin (`0.869x`/`0.880x`) before the closing scoreboard. Graduated as [t189](../../issues/189-postpass-attribution-and-speedup-screen.md).
6. **Updated `<= 0.50x` (2× faster) headroom:** unlike `t177` (where zeroing all of `prepass` still left all 4 `benchy` cells at `0.501x–0.537x`), post-t185 `prepass` zeroing now clears `<= 0.50x` in **8/8** (`0.196x–0.430x`), while practical `<= 0.50x` closure still spans prepass (`SupportGeometry`, `SupportAnalysis`, `ShellClassification`, `Slice` at `4.4–5.5×` concurrency), `PostPass`, and `per-layer` (`classic`/`arachne-perimeters` at `12.5–23.9 %` supports-off implied wall).

---

## 1. Stamped identities & correctness gate

| Field | Value |
| --- | --- |
| Git HEAD | `92b62d9a307d542247669101165fd1a815943049` (last code commit `748efbf5`, t185) |
| Guest freshness check | `cargo xtask build-guests --check` exit **0** (`EXIT_FRESH`) |
| `pnp_cli.exe` | `target/release/pnp_cli.exe`, sha256 `e661baa515c64b6d1c79a3216e1388eaf593ece3948e0bf8ec09430d66685d37` |
| Core modules | 24 `.wasm` under `modules/core-modules/`, sorted tree sha256 `8d33516c5352a5d682316f5834a61f8fa39300e0101b61b5c87b381bc60c705b` (`tree-support-planner.wasm` = `cf917686…`) |
| Thread pin | `RAYON_NUM_THREADS=12`, `--no-default-module-paths`, unsandboxed |
| Reference currency | [`evidence/t182-spatial-hierarchical-phase3-union/reference.json`](../t182-spatial-hierarchical-phase3-union/reference.json) |
| Run integrity | **24/24** instrumented + **8/8** `--profile` = **32/32** `exit=0`, `degraded=false`, `fatal=0`, `non_fatal=0`, `code-1200=0`, **8/8** byte-identical SHA-256 + bytes |
| Bracket reconciliation | Phase shares sum to `1.000000` on **24/24** passes; max postpass stage-vs-phase delta **1.46 %**; max prepass stage-vs-phase delta **6.21 %** (fixed `35–42 ms` inter-stage overhead on shrunken `676 ms` `benchy-classic-off` prepass) |

---

## 2. Post-t185 phase split (`--instrument-stderr`, median of 3 unsandboxed passes/cell)

| Cell | `slice_complete` (Δ vs t177, spread) | `prepass` (Δ pp) | `per-layer` (Δ pp) | `postpass` (Δ pp) | `outside` |
| --- | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | 1,317 ms (−22.5 %, 1.18 pp) | **51.5 %** (−3.6) | **32.3 %** (−0.1) | **10.5 %** (+2.3) | 5.5 % |
| `benchy-classic-on` | 2,801 ms (−18.9 %, 0.99 pp) | **64.2 %** (−2.6) | **18.7 %** (−0.6) | **14.4 %** (+2.6) | 2.6 % |
| `benchy-arachne-off` | 1,502 ms (−15.8 %, 2.17 pp) | **48.5 %** (−5.0) | **35.2 %** (+0.9) | **10.0 %** (+1.9) | 5.1 % |
| `benchy-arachne-on` | 3,022 ms (−14.9 %, 1.72 pp) | **60.9 %** (−3.0) | **21.9 %** (−0.2) | **15.1 %** (+3.3) | 2.6 % |
| `base-classic-off` | 9,436 ms (−4.8 %, 3.39 pp) | **68.5 %** (−2.4) | **22.6 %** (+1.4) | **7.9 %** (+1.0) | 0.8 % |
| `base-classic-on` | 19,737 ms (−33.2 %, 1.00 pp) | **76.8 %** (−0.9) | **12.6 %** (−1.7) | **10.4 %** (+2.6) | 0.4 % |
| `base-arachne-off` | 9,322 ms (−12.0 %, 2.88 pp) | **64.8 %** (−2.1) | **26.8 %** (+1.5) | **7.6 %** (+0.8) | 0.8 % |
| `base-arachne-on` | 20,073 ms (−30.5 %, 7.31 pp*) | **73.4 %** (−3.3) | **15.4 %** (−0.7) | **10.8 %** (+3.5) | 0.4 % |

*\*`base-arachne-on` pass 3 ran `24.0 s` vs `21.6–21.7 s` on passes 1–2 (`7.31 pp` spread); all other 7 cells sit within `0.99–3.39 pp`.*

---

## 3. Sequential prepass & postpass stage budgets (`ESTIMATE ONLY`)

All ratios compose `t185/reduced/route.json` (`ESTIMATE ONLY`) with this take's instrumented shares (`ratio_if_zero = est_ord × (1 − share)`). Gate-floor multiples divide `share` by `0.80 %`.

### Supports-on cluster (closing the last 2 open cells)

| Cell (`est_ord` / `acc`, `<1x` cut) | `SupportGeometry` | `SupportAnalysis` | `PostPass` (total / `LayerFin`) | `ShellClass` | `Slice` | `Per-layer` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| **`benchy-classic-on`** (**`1.204x`** / `1.222x`, **16.9 %**) | **29.5 % → 0.849x** (825 ms, `36.8×`) | **10.4 % → 1.078x** (292 ms, `13.0×`, **`2.95×` par**) | **14.4 %** / `6.2 %` (403 / 174 ms) | 8.1 % (227 ms, `7.7×` par) | 6.1 % (171 ms, `4.4×` par) | 18.7 % (519 ms) |
| **`benchy-arachne-on`** (**`1.085x`** / `1.076x`, **7.8 %**) | **27.5 % → 0.786x** (831 ms, `34.4×`) | **9.6 % → 0.981x** (290 ms, `12.0×`, **`3.00×` par**) | **15.1 %** / `6.7 %` (449 / 201 ms) | 7.3 % (220 ms, `7.9×` par) | 5.8 % (175 ms, `4.4×` par) | 21.9 % (661 ms) |
| `base-classic-on` (`0.843x` / `0.768x`, est. `<1x`) | **33.0 % → 0.565x** (6,512 ms, `41.2×`) | **14.5 % → 0.722x** (2,853 ms, `18.1×`, `6.47×` par) | **10.4 %** / `3.4 %` (2,043 / 671 ms) | 12.4 % (2,437 ms, `9.8×` par) | 7.7 % (1,510 ms, `4.9×` par) | 12.6 % (2,559 ms) |
| `base-arachne-on` (`0.833x` / `0.731x`, est. `<1x`) | **30.4 % → 0.580x** (6,111 ms, `38.1×`) | **14.4 % → 0.713x** (2,892 ms, `18.0×`, `7.44×` par) | **10.8 %** / `3.5 %` (2,162 / 701 ms) | 12.2 % (2,450 ms, `10.4×` par) | 7.4 % (1,492 ms, `4.8×` par) | 15.4 % (3,095 ms) |

### Supports-off cluster (all 4 est. `< 1.0x`; `<= 0.50x` headroom)

| Cell (`est_ord` / `acc`, `<=0.50x` cut) | `ShellClass` | `Slice` | `SeamPlanning` | `MeshAnalysis` | `PostPass` | `Per-layer` (top module implied) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` (`0.869x` / `0.907x`, 42.4 %) | 16.5 % (217 ms) | 12.5 % (164 ms) | 8.7 % (114 ms) | 6.6 % (87 ms) | 10.5 % (138 ms) | 32.3 % (427 ms; `classic-perim` 18.2 %) |
| `benchy-arachne-off` (`0.880x` / `0.817x`, 43.2 %) | 15.7 % (236 ms) | 11.5 % (173 ms) | 7.7 % (116 ms) | 5.9 % (89 ms) | 10.0 % (150 ms) | 35.2 % (528 ms; `arachne-perim` 23.9 %) |
| `base-classic-off` (`0.626x` / `0.616x`, 20.2 %) | 29.2 % (2,757 ms) | 16.0 % (1,507 ms) | 8.9 % (835 ms) | 6.9 % (654 ms) | 8.0 % (710 ms) | 22.6 % (2,133 ms; `classic-perim` 12.5 %) |
| `base-arachne-off` (`0.680x` / `0.640x`, 26.5 %) | 26.7 % (2,492 ms) | 16.3 % (1,519 ms) | 8.9 % (828 ms) | 6.7 % (625 ms) | 7.6 % (704 ms) | 26.8 % (2,494 ms; `arachne-perim` 18.4 %) |

---

## 4. Host Clipper per-op census & deterministic guest fuel (`--profile`, 1 pass/cell)

| Cell | Host total (vs t177) | `clip_polygons` | `offset` | `offset2_ex` | `closing_ex` | Guest fuel (vs t177) | Top guest modules |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `benchy-classic-off` | 4.08 s (0.906×) | 54.3 % | 28.9 % | 14.7 % | 2.1 % | 12.68 B (−37.4 %) | `classic-perimeters` 61.7 %, `infill-linker` 21.0 % |
| `benchy-classic-on` | 6.78 s (0.792×) | **47.9 %** | **41.9 %** | 8.8 % | 1.3 % | 20.72 B (−69.9 %) | `classic-perimeters` 37.8 %, `tree-support-planner` 28.2 %, `infill-linker` 12.9 % |
| `benchy-arachne-off` | 4.05 s (0.892×) | 54.9 % | 27.4 % | 15.6 % | 2.1 % | 12.21 B (−40.4 %) | `arachne-perimeters` 58.5 %, `infill-linker` 23.2 % |
| `benchy-arachne-on` | 6.78 s (0.784×) | **48.2 %** | **41.4 %** | 9.1 % | 1.3 % | 20.24 B (−70.6 %) | `arachne-perimeters` 35.3 %, `tree-support-planner` 28.9 %, `infill-linker` 14.0 % |
| `base-classic-off` | 48.86 s (0.915×) | 50.3 % | 32.1 % | 15.7 % | 2.0 % | 93.62 B (−24.1 %) | `classic-perimeters` 81.0 %, `infill-linker` 8.3 % |
| `base-classic-on` | 86.46 s (0.616×) | 39.6 % | **51.1 %** | 8.1 % | 1.2 % | 156.91 B (−72.2 %) | `classic-perimeters` 48.4 %, `tree-support-planner` 31.0 %, `tree-support` 7.3 % |
| `base-arachne-off` | 51.97 s (1.037×) | 51.1 % | 32.3 % | 14.0 % | 2.5 % | 80.71 B (−28.3 %) | `arachne-perimeters` 77.2 %, `infill-linker` 10.5 % |
| `base-arachne-on` | 90.86 s (0.656×) | 38.4 % | **51.9 %** | 8.5 % | 1.2 % | 144.00 B (−74.0 %) | `arachne-perimeters` 43.3 %, `tree-support-planner` 33.7 %, `tree-support` 7.9 % |

**Host built-in split on supports-on (`benchy-classic-on` / `base-classic-on`):**
- `host:native`: `3.15 s` (`clip 1.72 s / 23,578`, `offset 1.43 s / 18,031`) / **`32.31 s`** (`offset 17.49 s / 59,372`, `clip 14.82 s / 86,549`)
- `host:shell_classification`: `1.75 s` / `23.96 s`
- `host:support_analysis`: `0.86 s` (`offset 0.61 s / 401`, `clip 0.25 s / 965`) / **`18.45 s`** (`offset 15.54 s / 1,231`, `clip 2.90 s / 3,199`)
- `host:slice`: `0.76 s` / `7.41 s`

---

## 5. Graduated frontier tickets to close 8/8 `< 1.0x`

1. **[t187](../../issues/187-tree-support-planner-post-t182-residual-screen.md) — `PrePass::SupportGeometry` remainder (`com.core.tree-support-planner`) attribution and speedup screen** (`29.5 / 27.5 %` `benchy-on`, `33.0 / 30.4 %` `base-on`; standing `t137` 4-cell planner gate). Sole single-stage lever capable of closing both remaining open cells; combines `5.85 / 48.56 B` guest fuel with `host:native` `offset` (`17.5 s` base) and remaining phase-1/2/3/4 `clip_polygons` (`14.8 s` base).
2. **[t188](../../issues/188-support-analysis-attribution-and-speedup-screen.md) — `PrePass::SupportAnalysis` attribution and speedup screen** (`10.4 / 9.6 %` `benchy-on` at only `2.95–3.00×` concurrency, `14.5 / 14.4 %` `base-on`; standing `t137` 4-cell planner gate, `0 ms` supports-off). Combines serial `collect_print_plane_occupancy` (`~85 / ~745 ms`) with `host:support_analysis` `offset` (`84 %` of built-in thread-time).
3. **[t189](../../issues/189-postpass-attribution-and-speedup-screen.md) — `PostPass` (`LayerFinalization` + `GCodeSerialize` + `GCodePostProcess`) attribution and speedup screen** (`14.4 / 15.1 %` `benchy-on`, `10.4 / 10.8 %` `base-on`, `7.6–10.5 %` supports-off; all-8-cell standing gate). Every constituent stage clears `0.80 %` at `2.3–8.3×` floor in all 8 cells, closing `benchy-on` while widening `benchy-off`'s `< 1.0x` safety margin.

---

## 6. Verification & reproduction

- `python docs/specs/perf-vs-orca/evidence/t186-post-t185-attribution/verify.py --self-test` — **PASS** (`5/5` checks + **`12/12`** negative controls rejected).
- `cargo xtask build-guests --check` — exit **0**.

See [`REPRODUCE.md`](REPRODUCE.md) for the exact command sequence.

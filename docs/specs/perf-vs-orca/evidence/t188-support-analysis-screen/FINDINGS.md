# t188 findings — `PrePass::SupportAnalysis` attribution and exact speedup screen

Ticket: [`PrePass::SupportAnalysis` attribution and speedup screen](../../issues/188-support-analysis-attribution-and-speedup-screen.md).
Take: 2026-11-23, ordinary arm at HEAD `a5aea3ae`, `RAYON_NUM_THREADS=12`, unsandboxed, `modules/core-modules/` untouched (both `cargo xtask build-guests --check` and `cargo xtask build-guests --accelerated --check` exit `0`).
**Zero production code edits landed; all temporary probe/screen patches cleanly reverted (`git diff -- crates/ modules/ vendor/ xtask/` empty); `100 %` byte-identical to `t182/reference.json` across all `16/16` probe runs and `52/52` (`50` measured + `2` warmup) screen runs.**

---

## Verdict

1. **Why `PrePass::SupportAnalysis` only achieved `2.95–3.00×` concurrency on `benchy` (`348.5 ms`) and `6.5–7.4×` on `base` (`3,429.4 ms`):**
   - **Half the stage is 100 % single-threaded (`50.4 %` on `benchy`, `43.6 %` on `base`):**
     - `collect_print_plane_occupancy` (`ppo_ns`: **`122.68 ms` = `35.2 %` `benchy`**, **`1,088.96 ms` = `31.8 %` `base`**) runs single-threaded `slicer_core::slice_mesh_ex` — splitting into serial `slice_make_lines` (`23.78 / 427.60 ms` over `225,706 / 2,461,234` triangles) and serial per-layer `chain_lines_to_expolygons` (`93.30 / 658.82 ms` over `240 / 496` layers; max single layer only `3.30 / 3.56 ms`).
     - `object_layer_union` (`layer_union_ns`: **`48.14 ms` = `13.8 %` `benchy`**, **`389.20 ms` = `11.4 %` `base`**) runs a 100 % serial `.into_iter().map(union_ex)` over `240 / 496` layers.
   - **Inside parallel `contacts_par` (`182.65 ms` `benchy` / `1,905.70 ms` `base`), two bottlenecks dominate:**
     - **A two-layer serial Rayon chunk straggler on `benchy`:** `contact_work` is sorted ascending `(L1, L2, …)`, placing `L1` (`70.59 ms`, `grown_offset = 65.12 ms`) and `L2` (`94.43 ms`, `grown = 31.21 ms` + `cantilever_offset = 53.67 ms`) into the **same first Rayon chunk** (`165.0 ms` serial on one thread out of `182.65 ms` total wall, vs `p50 = 2.07 ms`!).
     - **Redundant `+3.0 mm` `cantilever_offset` (`45.0 %` of contact thread-time on `base`, `25.7 %` on `benchy`):** `detect_support_contacts_with_annotations` calls `offset(lower, +3.0 mm, Square)` on every contact layer (`81` `benchy` / `368` `base`) solely for the boolean test `!difference_ex(&contacts, &supported_extent).is_empty()`, consuming **`9,948.55 ms`** (`base`) and **`262.76 ms`** (`benchy`, including `53.67 ms` on `L2`) even though only **`2 / 81` (`benchy`)** and **`11 / 368` (`base`)** layers are cantilevers!
     - **Empty-slice `intersection_ex` bug (`enforced_blocked_intersections`):** `support_analysis_producer.rs` calls `intersection_ex(&geometry, enforcers)` and `intersection_ex(polygons, blockers)` unconditionally even when `enforcers` and `blockers` are empty (`&[]`), burning **`482.88 ms`** (`736` wasted Clipper2 calls on `base`) and **`25.92 ms`** (`162` wasted calls on `benchy`).
   - **Classic ≡ Arachne invariance:** Inside `PrePass::SupportAnalysis`, every structural count, op call count, and input/output vertex total is **100 % identical** between `classic-on` and `arachne-on` on both `benchy` and `base`.

2. **Exact (`0`-drift) winning bundle `SA-1 + SA-2 + SA-3 + SA-4 + SA-5 + SA-6` clears the standing `t137` 4-cell planner gate at `8.39×–14.15×` floor (`12/12` pairwise wall wins, `4/4` CPU corroborating, `52/52` byte-identical to `t182/reference.json`, supports-off structurally inert `2/2`):**
   - Cuts `PrePass::SupportAnalysis` stage wall **`−58.8…−70.4 %` (`2.42×–3.38×` speedup)**: `benchy` `328 → 97 ms` (`classic`) / `293 → 113 ms` (`arachne`); `base` `3,069 → 1,266 ms` (`classic`) / `3,382 → 1,149 ms` (`arachne`).
   - Cuts `host:support_analysis` Clipper2 calls **`−23.4 %` (`1,366 → 1,046` `benchy`)** and **`−32.5 %` (`4,430 → 2,990` `base`)**, cutting `host:support_analysis` thread-time **`−22.8…−22.9 %` (`benchy`)** and **`−39.7…−43.4 %` (`base`, saving `9.1–9.7 s` thread-time)**.
   - Uninstrumented 3-round interleaved A/B wall savings: `benchy-classic-on` **`+6.72 %`** (`8.39×` floor, CPU `+4.88 %`), `benchy-arachne-on` **`+7.94 %`** (`9.93×` floor, CPU `+3.72 %`), `base-classic-on` **`+11.32 %`** (`14.15×` floor, CPU `+7.07 %`), `base-arachne-on` **`+10.57 %`** (`13.22×` floor, CPU `+4.75 %`).
   - **Strict monotonic ablation:** every mechanism contributes positive wall reduction on both `benchy` and `base` (`baseline → SA-1 → SA-1+2+3 → SA-1..4 → full SA-1..6`).

3. **Falsified / rejected mechanisms (disclosed):**
   - `G_holes`, `G_win`, `G_island_win` (filtering holes or clipping `lower` to `overhang_areas` windows before `grown_offset`): **falsified** — `99.5 %` of `base` lower-layer vertices (`2,501,360 / 2,514,234`) lie on outer contours, `raw` wraps the perimeter, they are slower or neutral (`339.7 → 360.2–419.9 ms` `benchy`, `5,913.8 → 5,678.2–6,210.8 ms` `base`), and they introduce Clipper2 scanbeam/ordering drift on `2` `benchy` / `41–121` `base` layers.
   - `G_cert` (distance certificate on `grown_offset` at `δ ≈ 0.33 mm`): **rejected** — slower than `ClipperOffset` on `benchy` (`880.7` vs `339.7 ms`) with only `50 / 495` hits on `base`.

4. **Decision:** Graduate [Exact `PrePass::SupportAnalysis` acceleration: parallel print-plane slicing, exact cantilever distance certificate, parallel layer union, and LPT overlapped scheduling](../../issues/191-support-analysis-exact-acceleration.md) as an AFK implementation + standing `t137` 4-cell planner gate ticket (host-only `slicer-core` + `slicer-runtime`, zero WIT/guest changes, supports-off structurally inert).

---

## 1. Substage Attribution (`probe_wall.json`, median of 3 passes, `16/16` byte-identical)

| Substage / Metric | `benchy-classic-on` | `base-classic-on` | Parallelism / Notes |
| --- | ---: | ---: | --- |
| **`PrePass::SupportAnalysis` total wall** | **`348.53 ms`** | **`3,429.38 ms`** | Probed (`328 / 3,069 ms` unprobed inst) |
| `ppo_ns` (`collect_print_plane_occupancy`) | **`122.68 ms` (`35.2 %`)** | **`1,088.96 ms` (`31.8 %`)** | **100 % serial** (`make_lines 23.8 / 427.6`, `chain 93.3 / 658.8`) |
| `layer_union_ns` (`object_layer_union`) | **`48.14 ms` (`13.8 %`)** | **`389.20 ms` (`11.4 %`)** | **100 % serial** (`240 / 496` layers) |
| `gather_ns` + `post_contacts_ns` | `4.82 ms` (`1.4 %`) | `16.63 ms` (`0.5 %`) | Serial (`2×` redundant 2.5M-vertex clones) |
| `eligibility_ns` (`derive_needs_support`) | `2.40 ms` (`0.7 %`) | `23.92 ms` (`0.7 %`) | Already parallel (`240 / 496` all `true`) |
| `contacts_par_ns` wall | **`182.65 ms` (`52.4 %`)** | **`1,905.70 ms` (`55.6 %`)** | Thread-sum `1,023.1 / 22,101.1 ms` (`5.6× / 11.6×`) |
| ↳ `cantilever_offset` (`+3.0 mm` Square) | **`262.76 ms` (`25.7 %`)** | **`9,948.55 ms` (`45.0 %`)** | `81 / 368` calls; only **`2 / 11` hits**; `L2 = 53.7 ms` |
| ↳ `grown_offset` (`+δ ≈ 0.33 mm` Square) | **`525.86 ms` (`51.4 %`)** | **`9,206.71 ms` (`41.7 %`)** | `239 / 495` calls; `benchy L1 = 65.1 ms` |
| ↳ `trimmed_diff` | `162.10 ms` (`15.8 %`) | `1,433.84 ms` (`6.5 %`) | `239 / 495` calls (`81 / 368` non-empty) |
| ↳ `expand_back` (`offset` + `intersect/diff`) | `79.90 ms` (`7.8 %`) | `1,308.62 ms` (`5.9 %`) | `81 / 368` calls |
| ↳ `enforced_blocked_intersections` (empty `&[]`!) | **`25.92 ms` (`2.5 %`)** | **`482.88 ms` (`2.2 %`)** | **`162 / 736` wasted Clipper2 calls on empty slices** |
| ↳ `max` / `p50` single contact layer | `94.43 / 2.07 ms` | `129.28 / 39.99 ms` | `benchy` `L1 (70.6) + L2 (94.4)` in same Rayon chunk |

---

## 2. Exact Candidate Screen (`SA-1`..`SA-6`, 3-round interleaved A/B, `52/52` byte-identical)

Bundle mechanisms (all 100 % bit-exact, host-only, supports-off structurally inert):
- **`SA-1`:** `slicer_core::triangle_mesh_slicer::slice_mesh_ex_parallel` (chunk-parallel `slice_make_lines` merged in exact triangle order + `into_par_iter()` `chain_lines_to_expolygons`), called strictly from `collect_print_plane_occupancy` (`slice_mesh_ex` untouched → supports-off 100 % inert).
- **`SA-2`:** Parallel `object_layer_union` (`into_par_iter()`) + zero-clone single-region slice borrow.
- **`SA-3`:** Empty `enforcers` / `blockers` guard before `intersection_ex`.
- **`SA-4`:** Exact sufficient geometric certificate `certify_covered_by_square_offset` inside `detect_support_contacts_with_annotations` (exact `0.005 * delta` ring decimation + `1,000`-unit / `0.10 mm` conservative margin `r_in = 2.90 mm` + 2-level coarse-to-fine box cover; certifies `79 / 79` `benchy` and `352 / 357` `base` false layers with `0` errors in `8.5 / 83.4 ms`, falling back to unchanged `ClipperOffset` on any uncertain/true layer).
- **`SA-5`:** `rayon::join` overlapping `collect_print_plane_occupancy` with `(object_layer_union → modifiers → contacts_par)`.
- **`SA-6`:** LPT (descending `lower_verts + region_verts`) scheduling with `with_max_len(1)` + deterministic index re-sort + zero-clone `ContactWork<'a>`.

| Cell | Uninst. Wall (med of 3) | Wall Saved (`×` floor, wins) | CPU Saved | `SupportAnalysis` Stage | `host:support_analysis` (`--profile`) |
| --- | ---: | ---: | ---: | ---: | ---: |
| **`benchy-classic-on`** | `3.3239 → 3.1007 s` | **`+6.72 %`** (`8.39×`, `3/3`) | **`+4.88 %`** | `328 → 97 ms` (**`−70.4 %`**, `3.38×`) | `0.983 → 0.759 s` (`1,366 → 1,046`) |
| **`benchy-arachne-on`** | `3.4424 → 3.1690 s` | **`+7.94 %`** (`9.93×`, `3/3`) | **`+3.72 %`** | `293 → 113 ms` (**`−61.4 %`**, `2.59×`) | `1.023 → 0.789 s` (`1,366 → 1,046`) |
| **`base-classic-on`** | `25.9686 → 23.0296 s` | **`+11.32 %`** (`14.15×`, `3/3`) | **`+7.07 %`** | `3,069 → 1,266 ms` (**`−58.8 %`**, `2.42×`) | `22.886 → 13.800 s` (`4,430 → 2,990`) |
| **`base-arachne-on`** | `26.5024 → 23.7002 s` | **`+10.57 %`** (`13.22×`, `3/3`) | **`+4.75 %`** | `3,382 → 1,149 ms` (**`−66.0 %`**, `2.94×`) | `22.355 → 12.652 s` (`4,430 → 2,990`) |

### Monotonic Ablation Ladder (`PrePass::SupportAnalysis` stage ms, `--instrument-stderr`)

| Configuration | `benchy-classic-on` (`Δ` step) | `base-classic-on` (`Δ` step) | Byte-identical |
| --- | ---: | ---: | :---: |
| Baseline | `328 ms` | `3,069 ms` | `2/2` |
| `SA-1` alone (parallel PPO) | `265 ms` (`−63 ms`) | `2,510 ms` (`−559 ms`) | `2/2` |
| `SA-4` alone (cantilever cert) | `259 ms` (`−69 ms`) | `2,489 ms` (`−580 ms`) | `2/2` |
| `SA-1 + SA-2 + SA-3` | `216 ms` (`−49 ms`) | `2,255 ms` (`−255 ms`) | `2/2` |
| `SA-1 + SA-2 + SA-3 + SA-4` | `138 ms` (`−78 ms`) | `1,474 ms` (`−781 ms`) | `2/2` |
| **Full (`SA-1`..`SA-6`)** | **`97 ms` (`−41 ms`, `−231 ms` tot)** | **`1,266 ms` (`−208 ms`, `−1,803 ms` tot)** | `2/2` |

---

## 3. Route Projection (`ESTIMATE ONLY`)

Cross-window composition against `t185/reduced/route.json` (`ESTIMATE ONLY`, not a scoreboard ratio of record):

| Cell | Post-`t185` est. ord | Standalone `+t191` (A/B) | Stacked `t190 (t187) + t191 (t188)` (A/B / stage) | Remaining cut to `< 1.0x` |
| --- | ---: | ---: | ---: | ---: |
| `benchy-classic-on` | `1.2036x` | `1.1227x` | **`1.0258x` / `1.0172x`** | **`2.51 %`** (covered by `t189` `PostPass` `14.4 %`) |
| `benchy-arachne-on` | `1.0846x` | **`0.9985x`** | **`0.9074x` / `0.9321x`** | `0.00 %` (`< 1.0x`) |
| `base-classic-on` | `0.8434x` | `0.7480x` | **`0.7183x` / `0.7055x`** | `0.00 %` (`< 1.0x`) |
| `base-arachne-on` | `0.8334x` | `0.7452x` | **`0.6635x` / `0.6915x`** | `0.00 %` (`< 1.0x`) |

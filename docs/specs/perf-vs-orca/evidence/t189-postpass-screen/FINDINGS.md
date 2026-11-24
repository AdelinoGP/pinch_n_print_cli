# t189 findings — `PostPass` (`LayerFinalization` + `GCodeEmit` + `GCodePostProcess` + `GCodeSerialize`) attribution and exact speedup screen

Ticket: [`PostPass` (`LayerFinalization` + `GCodeSerialize` + `GCodePostProcess`) attribution and speedup screen](../../issues/189-postpass-attribution-and-speedup-screen.md).
Take: 2026-11-24, ordinary arm at HEAD `07b7ecf6` (last code commit `748efbf5`, t185), `RAYON_NUM_THREADS=12`, unsandboxed, `modules/core-modules/` untouched (both `cargo xtask build-guests --check` and `cargo xtask build-guests --accelerated --check` exit `0`).
**Zero production code edits landed; all temporary probe/screen patches cleanly reverted (`git diff -- crates/ modules/ vendor/ xtask/` empty); `100 %` byte-identical to `t182/reference.json` across all `32/32` probe runs and `94/94` (`92` measured + `2` warmup) screen runs.**

---

## Verdict

1. **Where `PostPass` + `run.rs` tail (`144–407 ms` = `11.0–15.4 %` of `benchy` wall; `698–2,044 ms` = `8.1–11.0 %` of `base` wall) actually goes across all 8 matrix cells:**
   - **`PostPass::LayerFinalization` (`40–169 ms` `benchy`, `170–666 ms` `base`) — eager full-stack WIT marshaling before any module runs:**
     - `build_finalization_world_glue` (`crates/slicer-macros/src/lib.rs`) eagerly calls `wit_layer.ordered_entities()` on **every layer** (`240` `benchy` / `496` `base`; up to `24,292` entities and `2.57 M` points on `base-on`) for **all 4 core finalization modules** before `run_finalization` executes a single line.
     - **`48.5–53.1 %`** of `LayerFinalization` wall (`19.2–82.6 ms` `benchy`, `83.6–335.6 ms` `base`) is burned by **two no-op modules** that immediately return `Ok(())` with `pushes_count = 0`: `com.core.wipe-tower` (`!self.enabled`) and `com.core.overhang-classifier-default` (`!overhang_annotations_consumed(config)`).
     - `com.core.skirt-brim` (`skirt_height = 1`) reads only layer 0 (`0.42 %` of `benchy` / `0.20 %` of `base` layers; **`99.58–99.80 %`** of marshaled layers unused), and `SkirtBrim::run_finalization` (`modules/core-modules/skirt-brim/src/lib.rs`) additionally mapped all `layers.iter()` instead of `.take(needed_layers)`.
     - `com.core.part-cooling` (`9.1–38.1 ms` `benchy`, `39.9–161.3 ms` `base`) reads only `entity.path.role == BridgeInfill` (zero point coordinates touched) and had a per-entity `cooling_decision_for_event` recomputation inside the inner loop; its residual cost after hoisting is WIT `ordered-entities()` marshaling.
     - On the host side, `LayerCollectionViewData::from_ir` (`crates/slicer-wasm-host/src/host.rs`) + store teardown (`push_views_ns + store_drop_ns`) deep-cloned and dropped `Arc::new(ir.ordered_entities.clone())` **4 times per slice** (`4.6–22.0 ms` `benchy`, `20.8–90.9 ms` `base` = **`11.7–15.6 %`** of `LayerFinalization`).
     - `apply_finalization_pushes` (`crates/slicer-runtime/src/layer_finalization.rs`) is negligible (`0.01–0.49 ms`; `took_rollback_clone = false` across all 4 modules on all 8 cells).
   - **`PostPass::GCodeSerialize` (`47–101 ms` `benchy`, `246–510 ms` `base`) — `99.2–99.7 %` serial heap-allocating decimal formatting:**
     - `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-gcode/src/serialize.rs`) accounts for **`99.20–99.70 %`** of wrapper wall (`461–521 ns/cmd`, `77.6–86.1 MB/s` across `101,902–976,526` commands / `4.08–39.61 MB`): single-threaded, un-preallocated `String::new()`, allocating 2 temporary heap `String`s per coordinate via `format_xyz` / `format_coord` + `core::fmt`.
   - **`PostPass::GCodeEmit` + `crates/slicer-runtime/src/run.rs` tail (`11–45 + 4.4–9.6 ms` `benchy`, `58–413 + 25.2–50.8 ms` `base`):**
     - **Single largest surprise on supports-on:** **`69.7–69.8 %`** of `GCodeEmit` on `base-on` (`283.0–287.9 ms` of `405.5–413.0 ms`) and **`36.1–36.8 %`** on `benchy-on` (`16.1–16.2 ms`) is **serial `drop(layer_irs)`** inside `emit_gcode_owned` (`crates/slicer-gcode/src/emit.rs`), freeing `240–496` `LayerIr` structs with support geometry vectors on one thread!
     - `simplify_ns` (`2.6–10.0 ms` `benchy`, `14.1–44.3 ms` `base`) allocates 4 intermediate `Vec`s per entity serially; `estimator_ns` (`3.8–9.2 ms` `benchy`, `22.5–40.8 ms` `base`) duplicates all segments serially — AND **`crates/slicer-runtime/src/run.rs` runs `estimate_print(&ir, ...)` a SECOND time** after `GCodePostProcess` (`3.4–7.3 ms` `benchy`, `19.9–40.6 ms` `base`) plus a serial `40 MB` `;LAYER_CHANGE` scan (`0.9–1.9 ms` `benchy`, `4.5–8.6 ms` `base`).
   - **`PostPass::GCodePostProcess` (`40–82 ms` `benchy`, `198–408 ms` `base`) — `+2` commands via `102K–977K` round-trip host calls:**
     - `com.core.machine-gcode-emit` (`modules/core-modules/machine-gcode-emit/src/lib.rs`) lowers all `101,900–976,524` commands by value across WIT and makes `101,902–976,526` individual `wit.push_*` host calls (`call_run = 85.1–91.3 %` of stage) solely to prepend `machine_start_gcode` and append `machine_end_gcode` (**`net_added_cmds == 2`** on all 8 cells). Because `slicer:postpass-gcode-postprocess` passes `list<gcode-command>` by value, fixing this at the root requires a WIT interface extension (`PP-4`).

2. **Exact Zero-WIT Candidate Bundle (`PP-1 + PP-2 + PP-3`) clears the standing `0.80 %` wall gate on ALL 8/8 cells at `4.45×–11.88×` floor (`24/24` pairwise wall wins, `94/94` byte-identical to `t182/reference.json`, zero WIT changes, zero geometry exceptions, zero gate waivers):**
   - Cuts `PostPass::GCodeSerialize` **`−88.8…−97.0 %`** (`8.9×–33.7×` faster: `46→2`, `101→3`, `254→11`, `509→20 ms`).
   - Cuts `PostPass::LayerFinalization` **`−70.7…−77.7 %`** (`3.4×–4.5×` faster: `38→11`, `171→47`, `175→39`, `650→155 ms`) and deterministic `LayerFinalization` WASM guest fuel **`−73.98 %`** (`benchy-classic-on`, `597.3 M → 155.4 M`) and **`−75.08 %`** (`base-classic-on`, `2,433.7 M → 606.4 M`) — with `wipe-tower` and `overhang-classifier-default` dropping **`−99.87…−99.94 %`** (`753×–1,755×`) and `skirt-brim` dropping **`−92.39…−98.53 %`** (`13.1×–68.2×`).
   - Cuts `PostPass::GCodeEmit` **`−35.2…−73.8 %`** (`11→5`, `41→17`, `57→33`, `404→106 ms`).
   - Cuts total `PostPass` phase wall **`−53.7…−63.6 %`** across all 8 cells (`398→153 ms` `benchy-classic-on`, `2,021→736 ms` `base-classic-on`).
   - Uninstrumented 3-round interleaved A/B total-slice wall savings: **`+5.39 %` / `+9.42 %` / `+6.88 %` / `+9.51 %`** (`benchy` `classic-off/on`, `arachne-off/on`) and **`+5.05 %` / `+7.87 %` / `+3.56 %` / `+6.02 %`** (`base` `classic-off/on`, `arachne-off/on`) — **`8/8` cells at `4.45×–11.88×` of the `0.80 %` floor, `24/24` pair wins**.
   - **Honest CPU corroboration disclosure (for Ticket 192):** Process CPU was lower on `4/8` cells (`+0.43 %` to `+3.64 %`) and flat-to-slightly-higher on `4/8` (`−0.18 %` to `−1.80 %`, within 3-round noise on `11–110 s` total CPU), because Rayon worker wakeup + parallel `drop(layer_irs)` Windows heap-lock contention offset part of the serial CPU saved (`~0.1–1.2 s`). Ticket 192 MUST verify CPU corroboration on the quiet 5-pair/cell standing gate and benchmark bounded Rayon concurrency / serial-vs-parallel `drop(layer_irs)` and `estimator` so the landed change wins both wall and CPU cleanly.

3. **Decision & Route Impact:**
   - **Graduate** [Exact Zero-WIT `PostPass` acceleration: lazy `LayerFinalization`, parallel zero-alloc `GCodeSerialize`, and `GCodeEmit`/tail cleanup](../../issues/192-postpass-exact-acceleration.md) as an AFK implementation + literal all-8-cell `0.80 %` standing gate ticket (zero WIT changes, zero geometry exceptions, zero gate waivers).
   - **Route (`ESTIMATE ONLY`):** Stacking the three graduated exact AFK tickets (`t190` + `t191` + `t192`) pushes **all 8/8 matrix cells `< 1.00x`** ordinary (`benchy-classic-on`: `1.2036x → 1.0258x → 0.9292x` A/B / `0.9249x` stage — crossing `< 1.00x` with **`> 7 %` headroom**!).
   - **Park priced WIT-extension leads (`PP-4` + `PP-1b`)** in `map.md` `## Not yet specified` for the post-`1.0x` (`<= 0.50x`) phase (`51–121 ms` = `3.9–4.8 %` of post-`t192` `benchy` wall; `235–673 ms` = `2.4–3.5 %` of post-`t192` `base` wall).

---

## 1. Substage Attribution (`reduced/probe_wall.json`, 3-pass medians, `32/32` byte-identical)

### 1a. Stage & Tail Wall Overview (All 8 Cells)

| Cell | `slice_complete` | `PostPass` phase | `run.rs` tail | `PostPass + tail` (`%`) | `LayerFin` | `GCodeEmit` | `GCodePostProc` | `GCodeSerialize` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | `1,282 ms` | `140 ms` | `4.42 ms` | `144.4 ms` (`11.3 %`) | `40 ms` | `11 ms` | `41 ms` | `47 ms` |
| `benchy-classic-on` | `2,591 ms` | `389 ms` | `9.31 ms` | `398.3 ms` (`15.4 %`) | `164 ms` | `43 ms` | `80 ms` | `99 ms` |
| `benchy-arachne-off` | `1,367 ms` | `146 ms` | `4.78 ms` | `150.8 ms` (`11.0 %`) | `43 ms` | `11 ms` | `43 ms` | `49 ms` |
| `benchy-arachne-on` | `2,692 ms` | `397 ms` | `9.59 ms` | `406.6 ms` (`15.1 %`) | `169 ms` | `45 ms` | `83 ms` | `101 ms` |
| `base-classic-off` | `8,288 ms` | `702 ms` | `26.46 ms` | `728.5 ms` (`8.8 %`) | `171 ms` | `76 ms` | `212 ms` | `247 ms` |
| `base-classic-on` | `18,640 ms` | `1,994 ms` | `49.80 ms` | `2,043.8 ms` (`11.0 %`) | `666 ms` | `405 ms` | `412 ms` | `510 ms` |
| `base-arachne-off` | `8,576 ms` | `673 ms` | `25.17 ms` | `698.2 ms` (`8.1 %`) | `170 ms` | `58 ms` | `201 ms` | `246 ms` |
| `base-arachne-on` | `18,920 ms` | `1,974 ms` | `50.78 ms` | `2,024.8 ms` (`10.7 %`) | `666 ms` | `413 ms` | `388 ms` | `504 ms` |

### 1b. Internal Breakdown on Representative Cells (`benchy-classic-on` & `base-classic-on`)

| Substage / Metric | `benchy-classic-on` | `base-classic-on` | Root Cause |
| --- | ---: | ---: | --- |
| **`LayerFinalization`** (`4` modules, `240 / 496` layers) | **`163.60 ms`** | **`663.64 ms`** | Eager `ordered_entities()` across all layers × 4 |
| ↳ `overhang-classifier-default` (`pushes = 0`) | `40.06 ms` | `173.26 ms` | Early-returns on line 1 (`!overhang_annotations_consumed`) |
| ↳ `wipe-tower` (`pushes = 0`) | `39.22 ms` | `159.22 ms` | Early-returns on line 1 (`!self.enabled`) |
| ↳ `skirt-brim` (`pushes = 38 / 6`, reads layer 0 only) | `46.65 ms` | `169.39 ms` | `99.58 % / 99.80 %` of layers marshaled unused |
| ↳ `part-cooling` (`pushes = 285 / 591`) | `37.68 ms` | `161.78 ms` | Reads only `path.role == BridgeInfill`; inner-loop hoist bug |
| ↳ Host `push_views + store_drop` (sum across 4) | `21.47 ms` (`13.1 %`) | `87.51 ms` (`13.2 %`) | `4×` deep clone + drop of `Arc<Vec<LayerEntity>>` |
| **`GCodeEmit`** (`211,943 / 976,524` cmds) | **`43.64 ms`** | **`405.50 ms`** | Serial drop + allocs + serial estimator |
| ↳ `drop_layers_ns` (`drop(layer_irs)`) | **`16.07 ms` (`36.8 %`)** | **`282.95 ms` (`69.8 %`)** | Serial teardown of support-heavy `LayerIr` vectors |
| ↳ `simplify_ns` + `emit_moves_ns` | `9.38 + 5.96 ms` | `44.28 + 25.62 ms` | `4` `Vec` allocs/entity + serial simplification |
| ↳ `estimator_ns` + `inject_m73_ns` | `8.99 + 1.71 ms` | `39.88 + 7.90 ms` | Double segment copy in `estimator.rs` |
| **`GCodePostProcess`** (`machine-gcode-emit`) | **`78.98 ms`** | **`408.23 ms`** | `211,945 / 976,526` WIT push calls for **`+2` cmds** |
| ↳ `call_run_ns` | `71.46 ms` (`90.5 %`) | `347.38 ms` (`85.1 %`) | Deferred to WIT extension (`PP-4`) |
| **`GCodeSerialize`** (`8.54 / 39.61 MB`) | **`99.21 ms`** | **`510.38 ms`** | `99.39 % / 99.69 %` in serial `commands_loop` |
| ↳ Throughput (`ns/cmd` / `MB/s`) | `465.2 ns` / `86.1 MB/s` | `521.0 ns` / `77.6 MB/s` | `2` heap `String` allocs/coord + `core::fmt` |
| **`run.rs` tail** (outside `PostPass` phase timer) | **`9.31 ms`** | **`49.80 ms`** | Redundant 2nd `estimate_print` (`7.10 / 39.67 ms`) + `;LAYER_CHANGE` scan (`1.82 / 8.58 ms`) |

---

## 2. Exact Zero-WIT Candidate Screen (`PP-1 + PP-2 + PP-3`, `94/94` byte-identical)

Mechanisms screened (zero WIT changes, zero geometry exceptions, zero gate waivers):
- **`PP-1` (Host + 4 `LayerFinalization` guests):** Zero-clone `LayerCollectionViewData::from_ir_shared` (`crates/slicer-wasm-host/src/host.rs`, `std::mem::take` into `Arc` before `call_run`, `Arc::try_unwrap` restore after `drop(store)`), lazy `OnceCell` `LayerCollectionView::ordered_entities` (`crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs`), `SkirtBrim::run_finalization` `.take(needed_layers)` (`modules/core-modules/skirt-brim/src/lib.rs`), and hoisted `cooling_decision_for_event` (`modules/core-modules/part-cooling/src/lib.rs`).
- **`PP-2` (Host `GCodeSerialize`):** Zero-alloc exact decimal formatter (`push_fixed_trimmed` / `push_fixed_e5` guarded by `|frac - 0.5| > 1e-4` with `core::fmt` fallback) + Rayon chunk-parallel `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-gcode/src/serialize.rs`) with backward-scanned `seed_e_accumulator`.
- **`PP-3` (Host `GCodeEmit` + estimator + `run.rs` tail):** Parallel `layer_irs.into_par_iter().for_each(drop)`, parallel zero-alloc `simplify_and_prune_indices`, pre-allocated `commands` + indexed `annotations` (`crates/slicer-gcode/src/emit.rs`), single-pass parallelized `estimate_print` (`crates/slicer-gcode/src/estimator.rs`), cached `PrintEstimate` reuse across `GCodePostProcess` gated by parallel kinematic invariance check (`crates/slicer-runtime/src/run.rs` / `crates/slicer-runtime/src/postpass.rs`), and parallel `;LAYER_CHANGE` scan in `crates/slicer-runtime/src/run.rs`.

### 2a. Full 8-Cell Interleaved Uninstrumented A/B (3 pairs/cell) & Stage Reductions

| Cell | Uninst. Wall (med of 3) | Wall Saved (`×` floor, wins) | CPU Saved | `PostPass` Phase | `LayerFin` | `GCodeEmit` | `GCodeSerialize` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | `1.3358 → 1.2638 s` | **`+5.39 %`** (`6.74×`, `3/3`) | `+3.64 %` | `136 → 63 ms` (`−53.7 %`) | `38 → 11 ms` | `11 → 5 ms` | `46 → 2 ms` (`23.0×`) |
| **`benchy-classic-on`** | `2.7014 → 2.4470 s` | **`+9.42 %`** (`11.77×`, `3/3`) | `−1.80 %` | `398 → 153 ms` (`−61.6 %`) | `171 → 47 ms` | `41 → 17 ms` | `101 → 3 ms` (`33.7×`) |
| `benchy-arachne-off` | `1.4393 → 1.3402 s` | **`+6.88 %`** (`8.60×`, `3/3`) | `+2.75 %` | `144 → 65 ms` (`−54.9 %`) | `41 → 12 ms` | `11 → 6 ms` | `48 → 2 ms` (`24.0×`) |
| `benchy-arachne-on` | `2.7649 → 2.5020 s` | **`+9.51 %`** (`11.88×`, `3/3`) | `+1.96 %` | `396 → 157 ms` (`−60.4 %`) | `169 → 47 ms` | `45 → 18 ms` | `100 → 4 ms` (`25.0×`) |
| `base-classic-off` | `9.7137 → 9.2229 s` | **`+5.05 %`** (`6.31×`, `3/3`) | `−0.18 %` | `705 → 290 ms` (`−58.9 %`) | `175 → 39 ms` | `57 → 33 ms` | `254 → 11 ms` (`23.1×`) |
| **`base-classic-on`** | `20.3349 → 18.7338 s` | **`+7.87 %`** (`9.84×`, `3/3`) | `+0.43 %` | `2,021 → 736 ms` (`−63.6 %`) | `650 → 155 ms` | `404 → 106 ms` | `509 → 20 ms` (`25.5×`) |
| `base-arachne-off` | `10.0761 → 9.7174 s` | **`+3.56 %`** (`4.45×`, `3/3`) | `−0.24 %` | `658 → 285 ms` (`−56.7 %`) | `161 → 38 ms` | `54 → 35 ms` | `246 → 10 ms` (`24.6×`) |
| `base-arachne-on` | `20.5504 → 19.3143 s` | **`+6.02 %`** (`7.52×`, `3/3`) | `−1.31 %` | `2,014 → 879 ms` (`−56.4 %`) | `686 → 157 ms` | `404 → 133 ms` | `527 → 59 ms` (`8.9×`) |

### 2b. Ablation Ladder (`PostPass` phase ms & targeted stage ms, `--instrument-stderr`, `24/24` byte-identical)

| Config | `benchy-classic-off` (`PP` / `LF` / `Emit` / `Ser`) | `benchy-classic-on` (`PP` / `LF` / `Emit` / `Ser`) | `base-classic-off` (`PP` / `LF` / `Emit` / `Ser`) | `base-classic-on` (`PP` / `LF` / `Emit` / `Ser`) |
| --- | ---: | ---: | ---: | ---: |
| `baseline` | `138` (`39 / 10 / 46`) | `386` (`166 / 41 / 98`) | `662` (`161 / 54 / 249`) | `2,163` (`650 / 579 / 467`) |
| `pp2_only` | `91` (`38 / 10 / 2`) | `299` (`171 / 41 / 4`) | `437` (`162 / 57 / 11`) | `1,881` (`856 / 559 / 24`) |
| `pp3_only` | `134` (`38 / 6 / 46`) | `364` (`165 / 17 / 98`) | `662` (`173 / 32 / 249`) | `1,643` (`655 / 104 / 494`) |
| `pp2_pp3_host_only` | `88` (`38 / 6 / 2`) | `265` (`162 / 17 / 4`) | `406` (`163 / 31 / 11`) | `1,306` (`645 / 105 / 21`) |
| `pp1_only` | `110` (`12 / 11 / 46`) | `277` (`47 / 46 / 99`) | `551` (`38 / 58 / 247`) | `1,696` (`158 / 421 / 632`) |
| **`full_pp1_pp2_pp3`** | **`64`** (`12 / 6 / 2`) | **`157`** (`47 / 17 / 4`) | **`284`** (`38 / 33 / 9`) | **`667`** (`155 / 106 / 19`) |

### 2c. Deterministic Guest Fuel (`--profile`, `4/4` byte-identical)

| Module | `benchy-classic-on` (base → cand) | Saved (`×`) | `base-classic-on` (base → cand) | Saved (`×`) |
| --- | ---: | ---: | ---: | ---: |
| `com.core.wipe-tower` | `147.08 M → 0.195 M` | **`−99.87 %`** (`753.0×`) | `599.59 M → 0.346 M` | **`−99.94 %`** (`1,733.2×`) |
| `com.core.overhang-classifier-default` | `143.93 M → 0.190 M` | **`−99.87 %`** (`756.7×`) | `599.38 M → 0.342 M` | **`−99.94 %`** (`1,754.6×`) |
| `com.core.skirt-brim` | `154.51 M → 11.757 M` | **`−92.39 %`** (`13.1×`) | `606.57 M → 8.898 M` | **`−98.53 %`** (`68.2×`) |
| `com.core.part-cooling` | `151.77 M → 143.266 M` | `−5.60 %` (`1.06×`) | `628.21 M → 596.806 M` | `−5.00 %` (`1.05×`) |
| **Combined `LayerFinalization` (4 mods)** | **`597.29 M → 155.408 M`** | **`−73.98 %`** | **`2,433.74 M → 606.391 M`** | **`−75.08 %`** |
| `com.core.machine-gcode-emit` (WIT boundary) | `217.71 M → 217.71 M` | `0.00 %` | `979.00 M → 979.00 M` | `0.00 %` |

---

## 3. Route Projection (`ESTIMATE ONLY`) & Priced Residual WIT Leads

Cross-window composition against `t185/reduced/route.json` and `t188/reduced/arithmetic.json` (`ESTIMATE ONLY` — not a scoreboard ratio of record):

| Cell | Post-`t185` est. ord | Pre-`t192` (`t190+t191` est. ord A/B) | Standalone `+t192` (A/B) | **Stacked `t190 + t191 + t192` (A/B / stage)** | Residual WIT leads (`PP-4 + PP-1b`) |
| --- | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | `0.8686x` | `0.8686x` | `0.8218x` | **`0.8218x` / `0.8212x`** | `43 + 8 = 51 ms` (`4.04 %`) |
| **`benchy-classic-on`** | `1.2036x` | `1.0258x` | `1.0902x` | **`0.9292x` / `0.9249x`** | `83 + 35 = 118 ms` (`4.82 %`) |
| `benchy-arachne-off` | `0.8802x` | `0.8802x` | `0.8196x` | **`0.8196x` / `0.8319x`** | `43 + 9 = 52 ms` (`3.88 %`) |
| `benchy-arachne-on` | `1.0846x` | `0.9074x` | `0.9815x` | **`0.8211x` / `0.8516x`** | `86 + 35 = 121 ms` (`4.84 %`) |
| `base-classic-off` | `0.6263x` | `0.6263x` | `0.5947x` | **`0.5947x` / `0.5996x`** | `204 + 37 = 241 ms` (`2.61 %`) |
| `base-classic-on` | `0.8434x` | `0.7183x` | `0.7770x` | **`0.6618x` / `0.6609x`** | `452 + 145 = 597 ms` (`3.19 %`) |
| `base-arachne-off` | `0.6798x` | `0.6798x` | `0.6556x` | **`0.6556x` / `0.6547x`** | `200 + 35 = 235 ms` (`2.42 %`) |
| `base-arachne-on` | `0.8334x` | `0.6635x` | `0.7832x` | **`0.6236x` / `0.6533x`** | `528 + 145 = 673 ms` (`3.48 %`) |

**Result:** Stacking `t190 + t191 + t192` reaches **`8/8` cells `< 1.00x`** on both A/B median and conservative PostPass-only stage-delta (`ESTIMATE ONLY`), before touching any WIT interface.

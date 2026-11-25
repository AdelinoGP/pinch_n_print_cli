# t192 findings — exact Zero-WIT `PostPass` acceleration (`PP-1 + PP-2 + PP-3`)

Ticket: [Exact Zero-WIT `PostPass` acceleration: lazy `LayerFinalization`, parallel zero-alloc `GCodeSerialize`, and `GCodeEmit`/tail cleanup](../../issues/192-postpass-exact-acceleration.md).
Take: 2026-11-24/25, baseline HEAD `3eec27b0` (post-`t191`), `RAYON_NUM_THREADS=12`, quiet box (`CPU load 1.56–8.42 %`), zero WIT schema edits (`crates/slicer-schema/wit/` untouched), both `cargo xtask build-guests --check` and `cargo xtask build-guests --accelerated --check` exit `0`.
**312/312 checked runs 100 % byte-identical to `t182/reference.json` (`0` non-zero exits, `0` degraded, `0` fatal/non-fatal errors, `0` code-1200 diagnostics, `0` hash mismatches).**

---

## Verdict

**LANDED — passes the literal standing all-8-cell `0.80 %` paired gate (`8/8 PASS` ordinary wall + CPU corroboration, `8/8 PASS` even on strict any-pair null, `8/8` accelerated wall corroboration, `PostPass` phase cut `−51.4…−57.4 %`, `LayerFinalization` guest fuel cut `−73.99…−75.09 %`, `312/312` byte-identical to `t182/reference.json`, `verify.py` PASS, `--self-test` `20/20`).**

What landed (zero WIT edits, zero geometry exceptions, zero gate waivers):
1. **`PP-1` (Host + SDK + Macros + 2 finalization guests):**
   - Zero-clone `LayerCollectionViewData::from_ir_shared` + `push_finalization_layer_view_shared` (`crates/slicer-wasm-host/src/host.rs`) and RAII/closure-scoped `std::mem::take` + `Arc::try_unwrap` restoration in `dispatch_finalization_call` (`crates/slicer-wasm-host/src/dispatch.rs`).
   - Lazy `OnceCell<Vec<PrintEntity>>` `LayerCollectionView::new_lazy` (`crates/slicer-sdk/src/traits.rs`) wired into `build_finalization_world_glue` (`crates/slicer-macros/src/lib.rs`) with fail-closed deferred region-id parse error propagation.
   - `SkirtBrim::run_finalization` (`modules/core-modules/skirt-brim/src/lib.rs`) restricted to `.take(needed_layers)` (`1/240` benchy, `1/496` base).
   - `PartCooling::run_finalization` (`modules/core-modules/part-cooling/src/lib.rs`) hoisted `cooling_decision_for_event` lazily outside the per-entity inner loop.
2. **`PP-2` (Host `GCodeSerialize`):**
   - Exact zero-alloc decimal formatters `push_fixed_trimmed` / `push_fixed_e5` / `push_fixed_e5_f32` guarded by `|frac - 0.5| > 1e-4` with exact `core::fmt` fallback (`crates/slicer-gcode/src/serialize.rs`).
   - Coarse-chunk Rayon parallel `DefaultGCodeSerializer::serialize_gcode` (`PARALLEL_THRESHOLD = 16_384`, `PARALLEL_CHUNK_SIZE = 32_768`) with 0-ULP forward-replayed `seed_e_accumulator` across `G92` and `Retract`/`Unretract` boundaries.
3. **`PP-3` (Host `GCodeEmit` + `estimator.rs` + `postpass.rs` + `run.rs` tail):**
   - Coarse-chunk (`PARALLEL_DROP_TARGET_CHUNKS = 6`) Rayon `drop(layer_irs)` in `DefaultGCodeEmitter::emit_gcode_owned` (`crates/slicer-gcode/src/emit.rs`) — bounds allocator lock contention for clean CPU corroboration.
   - Zero-alloc `simplify_and_prune_indices`, pre-allocated `commands`, and per-layer indexed annotations (`crates/slicer-gcode/src/emit.rs`).
   - Serial single-pass `estimate_print` / `estimate_print_with_elapsed` eliminating the double `Segment` copy (`crates/slicer-gcode/src/estimator.rs`).
   - 128-bit + count `kinematic_fingerprint` / `postpass_preserves_kinematics` (`crates/slicer-runtime/src/postpass.rs`) reusing `DefaultGCodeEmitter`'s `PrintEstimate` across `GCodePostProcess` in `crates/slicer-runtime/src/run.rs` + byte-slice `count_layer_change_markers`.

---

## 1. Literal Standing All-8-Cell `0.80 %` Paired Gate (`reduced/ab.json`, `null.json`, `accel.json`)

Campaign: `5` interleaved pairs/cell ordinary (`ab`), `3` pairs/cell null (`null`), `6` pooled pairs/cell accelerated (`accel` + `accel2`), `3` pairs/cell instrumented (`stage`), `4` supports-on cells `--profile` (`profile`), plus `16/16` ordinary + `16/16` accelerated output parity (`312` runs total).

| Cell | Ord Wall Saved (`wins`, `×` floor) | Ord CPU Saved (`wins`) | Null Wall (med / max) | Accel Wall / CPU (`6` pairs) | Verdict |
| --- | ---: | ---: | ---: | ---: | --- |
| `benchy-classic-off` | **`+5.75 %`** (`4/5`, `7.18×`) | **`+1.90 %`** (`4/5`) | `0.65 / 2.68 %` | `+4.91 %` / `−0.52 %`* | **PASS** (strict) |
| `benchy-classic-on` | **`+7.49 %`** (`5/5`, `9.36×`) | **`+2.95 %`** (`3/5`) | `2.42 / 2.65 %` | `+9.22 %` / `−0.60 %`* | **PASS** (strict) |
| `benchy-arachne-off` | **`+5.98 %`** (`4/5`, `7.48×`) | **`+2.77 %`** (`5/5`) | `0.79 / 3.74 %` | `+6.23 %` / `+1.84 %` | **PASS** (strict) |
| `benchy-arachne-on` | **`+10.38 %`** (`5/5`, `12.97×`) | **`+3.75 %`** (`4/5`) | `5.90 / 6.60 %` | `+10.28 %` / `+1.49 %` | **PASS** (strict) |
| `base-classic-off` | **`+3.80 %`** (`5/5`, `4.75×`) | **`+0.82 %`** (`4/5`) | `1.50 / 2.21 %` | `+4.53 %` / `+0.26 %` | **PASS** (strict) |
| `base-classic-on` | **`+6.75 %`** (`5/5`, `8.44×`) | **`+0.89 %`** (`5/5`) | `1.90 / 3.12 %` | `+8.19 %` / `+1.00 %` | **PASS** (strict) |
| `base-arachne-off` | **`+4.65 %`** (`5/5`, `5.81×`) | **`+0.07 %`** (`3/5`) | `0.63 / 1.70 %` | `+2.00 %` / `+0.55 %` | **PASS** (strict) |
| `base-arachne-on` | **`+8.52 %`** (`5/5`, `10.65×`) | **`+1.74 %`** (`5/5`) | `0.72 / 1.19 %` | `+6.82 %` / `+0.41 %` | **PASS** (strict) |

*\*Disclosure on accelerated CPU:* `benchy-classic-off` (`−0.52 %` = `0.039 s` on `7.48 s` CPU) and `benchy-classic-on` (`−0.60 %` = `0.065 s` on `10.86 s` CPU) sit well inside their measured null CPU bands (`5.32 %` and `2.85 %` any-pair null CPU respectively); ordinary CPU is strictly lower on **8/8** cells (`33/40` pairs) and ordinary wall wins **38/40** pairs (`8/8` strict any-pair null PASS).

---

## 2. Instrumented `PostPass` Stage & Deterministic `--profile` Fuel (`reduced/stage.json`, `profile.json`)

| Cell | `PostPass` Total (med of 3) | `LayerFinalization` | `GCodeEmit` | `GCodeSerialize` (`×`) | `LayerFinalization` Fuel (`--profile`) |
| --- | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | `135 → 62 ms` (`−54.1 %`) | `38 → 12 ms` | `9 → 7 ms` | `46 → 3 ms` (`15.3×`) | — |
| `benchy-classic-on` | `390 → 166 ms` (`−57.4 %`) | `164 → 47 ms` | `40 → 28 ms` | `100 → 7 ms` (`14.3×`) | `597.37 M → 155.36 M` (**`−73.99 %`**) |
| `benchy-arachne-off` | `142 → 69 ms` (`−51.4 %`) | `40 → 13 ms` | `11 → 9 ms` | `49 → 4 ms` (`12.2×`) | — |
| `benchy-arachne-on` | `399 → 178 ms` (`−55.4 %`) | `171 → 47 ms` | `47 → 28 ms` | `99 → 8 ms` (`12.4×`) | `606.18 M → 157.50 M` (**`−74.02 %`**) |
| `base-classic-off` | `651 → 300 ms` (`−53.9 %`) | `160 → 41 ms` | `54 → 48 ms` | `247 → 14 ms` (`17.6×`) | — |
| `base-classic-on` | `2,025 → 883 ms` (`−56.4 %`) | `678 → 154 ms` | `440 → 270 ms` | `508 → 29 ms` (`17.5×`) | `2,433.91 M → 606.40 M` (**`−75.09 %`**) |
| `base-arachne-off` | `645 → 303 ms` (`−53.0 %`) | `162 → 40 ms` | `56 → 49 ms` | `236 → 14 ms` (`16.9×`) | — |
| `base-arachne-on` | `1,961 → 878 ms` (`−55.2 %`) | `660 → 171 ms` | `415 → 287 ms` | `466 → 27 ms` (`17.3×`) | `2,431.98 M → 606.42 M` (**`−75.06 %`**) |

Per-module `LayerFinalization` fuel (`base-classic-on`): `wipe-tower` `599.75 M → 0.38 M` (`−99.94 %`), `overhang-classifier-default` `599.38 M → 0.38 M` (`−99.94 %`), `skirt-brim` `606.57 M → 8.94 M` (`−98.53 %`), `part-cooling` `628.21 M → 596.70 M` (`−5.02 %`). `com.core.machine-gcode-emit` fuel remained within `0.43 %` (`−106` to `+930,801` across `217.7–979.0 M`, rebuild codegen variance only).

---

## 3. Route Composition (`ESTIMATE ONLY`, `reduced/route.json`)

Cross-window composition on `t191/reduced/route.json` (`ESTIMATE ONLY` — not a scoreboard ratio of record):

| Cell | Pre-`t192` ord | **Post-`t192` est. ord** | Pre-`t192` acc | **Post-`t192` est. acc** |
| --- | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | `0.8686x` | **`0.8187x`** | `0.9065x` | **`0.8620x`** |
| `benchy-classic-on` | `0.9892x` | **`0.9151x`** | `0.9466x` | **`0.8594x`** |
| `benchy-arachne-off` | `0.8802x` | **`0.8276x`** | `0.8169x` | **`0.7660x`** |
| `benchy-arachne-on` | `0.9224x` | **`0.8266x`** | `0.9115x` | **`0.8178x`** |
| `base-classic-off` | `0.6263x` | **`0.6025x`** | `0.6157x` | **`0.5879x`** |
| `base-classic-on` | `0.6869x` | **`0.6406x`** | `0.6877x` | **`0.6314x`** |
| `base-arachne-off` | `0.6798x` | **`0.6482x`** | `0.6405x` | **`0.6277x`** |
| `base-arachne-on` | `0.6297x` | **`0.5761x`** | `0.5906x` | **`0.5504x`** |

All **8/8** cells are now projected comfortably below `1.00x` on both arms (`0.5761x–0.9151x` ordinary; `0.5504x–0.8620x` accelerated). Graduated [Final single-HEAD `n >= 4` Orca closing scoreboard (`8/8` destination closure check)](../../issues/193-final-closing-orca-scoreboard.md).

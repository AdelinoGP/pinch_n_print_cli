# Exact Zero-WIT `PostPass` acceleration: lazy `LayerFinalization`, parallel zero-alloc `GCodeSerialize`, and `GCodeEmit`/tail cleanup

Type: task / measured candidate (100 % output-preserving Zero-WIT host + 4-guest acceleration + literal standing all-8-cell `0.80 %` gate; zero WIT changes, zero geometry exceptions, zero gate waivers)
Label: wayfinder:task
Status: open
Assignee: unassigned
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [`PostPass` (`LayerFinalization` + `GCodeSerialize` + `GCodePostProcess`) attribution and speedup screen](189-postpass-attribution-and-speedup-screen.md) ([`evidence/t189-postpass-screen/FINDINGS.md`](../evidence/t189-postpass-screen/FINDINGS.md)), which attributed all four `PostPass` stages plus the `crates/slicer-runtime/src/run.rs` tail across all 8 cells (`32/32` probe runs byte-identical to `t182/reference.json`) and screened the exact Zero-WIT bundle `PP-1 + PP-2 + PP-3` across `94/94` (`92` measured + `2` warmup) runs (`100 %` byte-identical to `t182/reference.json`, `24/24` A/B wall wins, `+3.56…+9.51 %` wall = `4.45×–11.88×` floor across all 8 cells, `PostPass` `−53.7…−63.6 %`, `LayerFinalization` fuel `−73.98 / −75.08 %`, monotonic ablation across all 4 representative cells).

## Question

**Does landing the 100 % output-preserving, Zero-WIT bundle `PP-1 + PP-2 + PP-3` across `crates/slicer-wasm-host`, `crates/slicer-sdk`, `crates/slicer-macros`, `crates/slicer-gcode`, `crates/slicer-runtime`, and the 4 `PostPass::LayerFinalization` core guests (`skirt-brim`, `overhang-classifier-default`, `part-cooling`, `wipe-tower`) — with zero WIT schema edits — pass the literal standing all-8-cell `0.80 %` paired gate (`>= 0.80 %` ordinary + accelerated wall and CPU corroboration on all 8 cells, `16/16` byte-identical to `t182/reference.json`)?**

## What is already known (t189)

- **Screen results (`PP-1 + PP-2 + PP-3`, `RAYON_NUM_THREADS=12`, unsandboxed, `94/94` runs byte-identical to `t182/reference.json`):**
  - `benchy-classic-off`: **`+5.39 %`** wall (`1.3358 → 1.2638 s`, `6.74×` floor, `3/3`), CPU `+3.64 %`, `PostPass` `136 → 63 ms` (`−53.68 %`: `LF 38→11`, `Emit 11→5`, `Ser 46→2`).
  - `benchy-classic-on`: **`+9.42 %`** wall (`2.7014 → 2.4470 s`, `11.77×` floor, `3/3`), CPU `−1.80 %`, `PostPass` `398 → 153 ms` (`−61.56 %`: `LF 171→47`, `Emit 41→17`, `Ser 101→3`), `LayerFinalization` fuel `597.29 M → 155.41 M` (`−73.98 %`).
  - `benchy-arachne-off`: **`+6.88 %`** wall (`1.4393 → 1.3402 s`, `8.60×` floor, `3/3`), CPU `+2.75 %`, `PostPass` `144 → 65 ms` (`−54.86 %`: `LF 41→12`, `Emit 11→6`, `Ser 48→2`).
  - `benchy-arachne-on`: **`+9.51 %`** wall (`2.7649 → 2.5020 s`, `11.88×` floor, `3/3`), CPU `+1.96 %`, `PostPass` `396 → 157 ms` (`−60.35 %`: `LF 169→47`, `Emit 45→18`, `Ser 100→4`).
  - `base-classic-off`: **`+5.05 %`** wall (`9.7137 → 9.2229 s`, `6.31×` floor, `3/3`), CPU `−0.18 %`, `PostPass` `705 → 290 ms` (`−58.87 %`: `LF 175→39`, `Emit 57→33`, `Ser 254→11`).
  - `base-classic-on`: **`+7.87 %`** wall (`20.3349 → 18.7338 s`, `9.84×` floor, `3/3`), CPU `+0.43 %`, `PostPass` `2,021 → 736 ms` (`−63.58 %`: `LF 650→155`, `Emit 404→106`, `Ser 509→20`), `LayerFinalization` fuel `2,433.74 M → 606.39 M` (`−75.08 %`).
  - `base-arachne-off`: **`+3.56 %`** wall (`10.0761 → 9.7174 s`, `4.45×` floor, `3/3`), CPU `−0.24 %`, `PostPass` `658 → 285 ms` (`−56.69 %`: `LF 161→38`, `Emit 54→35`, `Ser 246→10`).
  - `base-arachne-on`: **`+6.02 %`** wall (`20.5504 → 19.3143 s`, `7.52×` floor, `3/3`), CPU `−1.31 %`, `PostPass` `2,014 → 879 ms` (`−56.36 %`: `LF 686→157`, `Emit 404→133`, `Ser 527→59`).
- **Route projection (`ESTIMATE ONLY`, cross-window composition against `t185/reduced/route.json` and `t188/reduced/arithmetic.json`):**
  - Standalone (`t185 × t189`): `benchy` `0.822 / 1.090 / 0.820 / 0.982x` ord, `base` `0.595 / 0.777 / 0.656 / 0.783x` ord (`7/8 < 1.0x`).
  - Stacked with [Exact `com.core.tree-support-planner` acceleration](190-tree-support-planner-exact-acceleration.md) and [Exact `PrePass::SupportAnalysis` acceleration: parallel print-plane slicing, exact cantilever distance certificate, parallel layer union, and LPT overlapped scheduling](191-support-analysis-exact-acceleration.md) (`t190 + t191 + t192`): **ALL `8/8` cells `< 1.00x`** on both A/B median and conservative PostPass-only stage-delta (`benchy-classic-on`: `1.2036x → 1.0258x → 0.9292x` A/B / `0.9249x` stage; `benchy-arachne-on` `0.8211x`; `benchy-off` `0.8218x / 0.8196x`; `base` `0.5947x / 0.6618x / 0.6556x / 0.6236x` ord).
- **Important CPU corroboration finding from t189 (MUST address during implementation):**
  - In the 3-round screen, process CPU improved on `4/8` cells (`+0.43 %` to `+3.64 %`) and was flat-to-slightly-higher on `4/8` (`−0.18 %` to `−1.80 %`). The likely cause is Rayon worker wakeup + Windows heap-lock contention during parallel `drop(layer_irs)` (`crates/slicer-gcode/src/emit.rs`) and fine-grained parallel `estimate_print` / `simplify_and_prune_indices` when total serial CPU saved (`~0.1–1.2 s`) is small relative to `11–110 s` total process CPU.
  - Before locking the final implementation for the 5-pair/cell standing gate, measure CPU vs wall trade-offs of: (a) bounding chunk granularity / thread fan-out in `drop(layer_irs)` (or coarser chunks / `par_chunks_mut` to minimize Windows heap lock contention), (b) keeping `estimate_print` single-pass serial (already zero-copy vs baseline's double-copy + eliminated 2nd call in `run.rs`) vs parallel, and (c) coarse-chunk parallel `serialize_gcode` (`>= 16K–32K` cmds/chunk so worker wakeup cost is negligible). Target clean **both-wall-and-CPU corroboration across all 8/8 cells** on the quiet standing gate!
- **Explicit non-goals (parked for `<= 0.50x` follow-on):**
  - **Zero WIT schema edits:** do NOT change `crates/slicer-schema/wit/`. `PP-4` (`GCodePostProcess` stream-view + `push-input-range` for `com.core.machine-gcode-emit`, `43–528 ms`) and `PP-1b` (`part-cooling` `entity-roles` query, `8–145 ms`) are priced and parked in `map.md` `## Not yet specified`.

## Scope

Production edits across `crates/slicer-wasm-host`, `crates/slicer-sdk`, `crates/slicer-macros`, `crates/slicer-gcode`, `crates/slicer-runtime`, and `modules/core-modules/{skirt-brim, overhang-classifier-default, part-cooling, wipe-tower}` (zero `crates/slicer-schema/wit/` edits):

1. **`PP-1` — Zero-clone host `LayerFinalization` views + lazy `LayerCollectionView` + guest loop fixes:**
   - **Host zero-clone (`crates/slicer-wasm-host/src/host.rs`, `crates/slicer-wasm-host/src/dispatch.rs`, `crates/slicer-runtime/src/layer_finalization.rs`):**
     - Add `LayerCollectionViewData::from_ir_shared` (or move `ir.ordered_entities` via `std::mem::take` into `Arc<Vec<LayerEntity>>` once before `call_run`, push `Arc::clone` into `Store`, and restore ownership after `drop(store)` via `Arc::try_unwrap` with fallback clone if any handle leaked) so `LayerFinalization` stops deep-cloning & dropping the entire layer stack 4 times per slice.
   - **Lazy `LayerCollectionView` (`crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs`):**
     - Make `LayerCollectionView::ordered_entities()` lazily materialized via `std::cell::OnceCell<Vec<LayerEntity>>` backed by an internal closure/loader populated by `build_finalization_world_glue` (`crates/slicer-macros/src/lib.rs`), keeping `LayerCollectionView::new(...)` eager constructor working for tests and non-WASM callers.
     - Keep `LayerFinalization` WIT bindings completely unchanged — the guest simply holds the `wit_layer` resource handle and calls `wit_layer.ordered_entities()` only on layers whose `ordered_entities()` method is actually invoked by the module!
   - **Guest fixes (`modules/core-modules/skirt-brim/src/lib.rs`, `modules/core-modules/part-cooling/src/lib.rs`):**
     - In `SkirtBrim::run_finalization` (`modules/core-modules/skirt-brim/src/lib.rs`), iterate only `layers.iter().take(needed_layers)` (instead of mapping all `layers.iter()`) so `skirt_height = 1` materializes only layer 0 (`1/240` or `1/496`).
     - In `PartCooling::run_finalization` (`modules/core-modules/part-cooling/src/lib.rs`), hoist `self.cooling_decision_for_event(view)` out of the per-entity inner loop (it depends only on layer-level `view`, not the individual entity).
     - Rebuild all affected guests via `cargo xtask build-guests` and `cargo xtask build-guests --accelerated` (and verify both `--check` commands exit `0`).
2. **`PP-2` — Zero-alloc exact decimal formatter + Rayon chunk-parallel `GCodeSerialize` (`crates/slicer-gcode/src/serialize.rs`):**
   - Implement exact zero-alloc decimal formatting (`push_fixed_trimmed` for 3-decimal `X/Y/Z` and `push_fixed_e5` for 5-decimal `E`, preserving `-0.0` normalization, trailing-zero stripping, and guarding `|frac - 0.5| > 1e-4` with exact fallback to `core::fmt` `write!` so IEEE-754 ties-to-even rounding is **100 % bit-identical** to `format!("{:.3}", v)` / `format!("{:.5}", v)` on every finite `f64`).
   - Pre-size per-chunk `String` buffers and parallelize `DefaultGCodeSerializer::serialize_gcode` across command chunks (falling back to serial for small streams), seeding each chunk's `e_accumulator` (in relative/absolute extrusion mode) via an exact backward scan from the chunk start to the nearest `G92`/`ResetE` or stream start (`seed_e_accumulator`), then concatenating chunks in exact index order.
3. **`PP-3` — `GCodeEmit` + `estimator.rs` + `crates/slicer-runtime/src/run.rs` tail cleanup:**
   - **`crates/slicer-gcode/src/emit.rs` (`emit_gcode_owned`):**
     - Replace serial `drop(layer_irs)` with low-contention parallel drop (benchmark chunked Rayon drop vs serial drop on CPU + wall; keep whichever wins both wall and CPU cleanly).
     - Replace 4-`Vec`-allocating serial entity simplification with zero-alloc `simplify_and_prune_indices` (parallelized per-layer when beneficial).
     - Pre-allocate `commands` capacity from total point count and use indexed annotation tracking instead of linear scans.
   - **`crates/slicer-gcode/src/estimator.rs` (`estimate_print`):**
     - Eliminate the double full-stream `Segment` copy; compute print time / filament length in a single pass (evaluate serial single-pass vs coarse-chunk parallel for optimal CPU corroboration).
   - **`crates/slicer-runtime/src/postpass.rs` & `crates/slicer-runtime/src/run.rs`:**
     - Reuse the `PrintEstimate` already computed inside `GCodeEmit` across `GCodePostProcess` whenever a fast kinematic invariance check (`postpass_preserves_kinematics`) proves `GCodePostProcess` added/modified zero kinematic motion/retraction/feedrate/extrusion commands (true for `com.core.machine-gcode-emit`, which only prepends/appends raw start/end G-code); fall back to full `estimate_print` if any post-processor mutated kinematic commands!
     - Optimize the `;LAYER_CHANGE` count scan in `crates/slicer-runtime/src/run.rs` (byte-slice search via `memchr` / chunked scan or count directly from emitted `GCodeIr` when exact).
4. **Required unit & regression tests:**
   - `crates/slicer-gcode/src/serialize.rs`: exhaustive boundary tests for `push_fixed_trimmed` and `push_fixed_e5` vs `format!` (`0.0`, `-0.0`, `-0.0001`, half-way ties `0.0005`, `0.0015`, `0.0025`, `0.000005`, `0.000015`, large coordinates, negative coordinates, non-finite fallback) + parallel vs serial `serialize_gcode` byte-identity tests across `G92` resets, absolute & relative `E`, and chunk boundaries.
   - `crates/slicer-gcode/src/emit.rs` & `crates/slicer-gcode/src/estimator.rs`: `simplify_and_prune_indices` bit-exact parity with legacy simplification; single-pass `estimate_print` exact equality with baseline `PrintEstimate` (within 0 ULP or exact deterministic order); `postpass_preserves_kinematics` positive (start/end raw G-code) and negative (inserted `G1`, altered feedrate, altered `E`, reordered moves) tests.
   - `crates/slicer-sdk/src/traits.rs` & `crates/slicer-wasm-host`: lazy `LayerCollectionView` unit test verifying loader closure is called 0 times when `ordered_entities()` is untouched and exactly once (cached in `OnceCell`) on repeated reads; host `from_ir_shared` round-trip ownership restoration test (`Arc::try_unwrap` succeeds without cloning).
5. **Literal standing all-8-cell `0.80 %` paired ordinary + accelerated + null-band gate (`RAYON_NUM_THREADS=12`, unsandboxed):**
   - Rebuild BOTH ordinary and accelerated guests (`cargo xtask build-guests` and `cargo xtask build-guests --accelerated`; verify both `--check` exit `0`) and BOTH ordinary and accelerated host binaries.
   - **Output parity (fail-closed, zero reference change):** baseline AND candidate MUST reproduce `t182/reference.json` byte-for-byte on **16/16** `(8 cells × {ordinary, accelerated})` runs (`exit 0`, `degraded = false`, `0` fatal/non-fatal errors, `0` code-1200 diagnostics).
   - **Timing (literal all-8-cell gate):** 5 interleaved pairs/cell ordinary + 3-pair null control + 3-pair accelerated corroboration across **all 8 matrix cells**, verifying `>= 0.80 %` wall saving AND corroborating lower process CPU + accelerated corroboration, plus `--profile` deterministic `LayerFinalization` fuel delta.
   - **Pre-commit code gates:** targeted unit/integration tests across `slicer-gcode`, `slicer-sdk`, `slicer-wasm-host`, `slicer-runtime`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask check-literals`; `cargo xtask check-test-quality --report`; `cargo xtask build-guests --check`; `cargo xtask build-guests --accelerated --check`.

## Acceptance

- Clean, tested production implementation with zero WIT schema edits and both `cargo xtask build-guests --check` and `--accelerated --check` exiting `0`.
- Evidence directory `docs/specs/perf-vs-orca/evidence/t192-postpass-exact-accel/` with reduced paired ordinary + null + accelerated + `--profile` JSONs, `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`verify.py --self-test` exit `0`).
- Updated `## Answer`, `map.md` update, and clean commit after all gates pass (zero reference-set change).

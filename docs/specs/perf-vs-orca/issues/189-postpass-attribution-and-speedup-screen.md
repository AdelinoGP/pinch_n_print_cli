# `PostPass` (`LayerFinalization` + `GCodeSerialize` + `GCodePostProcess`) attribution and speedup screen

Type: task / attribution + mechanism screen (no production code edit)
Label: wayfinder:task
Status: resolved (2026-11-24 — attribution + exact Zero-WIT mechanism screen, zero production code edits, `32/32` probe + `94/94` screen runs byte-identical to `t182/reference.json`, `verify.py --self-test` PASS `14/14`; graduated `192`)
Assignee: 2026-11-24 Antigravity wayfinder session (resolved)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t185 full-matrix attribution and 8/8 closure headroom map](186-post-t185-full-matrix-attribution.md)

## Question

**Now that `PostPass` owns `14.4–15.1 %` (`403–449 ms`) of `benchy` supports-on, `10.4–10.8 %` (`2,043–2,162 ms`) of `base` supports-on, and `7.6–10.5 %` of supports-off — with all three top stages (`LayerFinalization` `1.8–6.7 %`, `GCodeSerialize` `2.7–3.7 %`, `GCodePostProcess` `2.1–3.0 %`) clearing the `0.80 %` floor at `2.3–8.3×` in ALL 8 CELLS — where does the serial/host time go inside each stage, and what exact (byte-identical to `t182/reference.json`) bundle clears the literal all-8-cell `0.80 %` standing gate?**

## Scope

- Substage attribution inside:
  - `PostPass::LayerFinalization` (`174–201 ms` `benchy-on`, `671–701 ms` `base-on`; WASM `skirt-brim`, `overhang-classifier-default`, `part-cooling`, `wipe-tower`, and host view/marshal/merge overhead),
  - `PostPass::GCodeSerialize` (`98–113 ms` `benchy-on`, `526–595 ms` `base-on`; `host:gcode_serialize` formatting/buffering/parallelism),
  - `PostPass::GCodePostProcess` (`82–88 ms` `benchy-on`, `414–436 ms` `base-on`; `com.core.machine-gcode-emit` + host marshal),
  - `PostPass::GCodeEmit` (`42–46 ms` `benchy-on`, `425–429 ms` `base-on`).
- Screen exact (zero G-code drift) speedups across all 8 cells; note the dual payoff of cutting `benchy-on` toward `8/8 < 1.0x` while widening `benchy-off`'s `< 1.0x` margin (`0.869x`/`0.880x`) before the closing Orca scoreboard.
- Zero production code edits landed in this ticket; graduate an implementation & standing-gate ticket for the winning exact bundle.

## Pre-declared gate

- Attribution & screen only (no production code edits); any temporary probe patch reverted before resolution, `cargo xtask build-guests --check` exit 0, and `100 %` byte-identical to `t182/reference.json`.
- Any graduated implementation candidate must be priced against the literal all-8-cell `0.80 %` standing gate.

## Acceptance

- Reduced evidence under `docs/specs/perf-vs-orca/evidence/t189-postpass-screen/` with `FINDINGS.md`, `REPRODUCE.md`, and checkout-only `verify.py` (`--self-test` exit 0).
- Updated `## Answer`, `map.md`, and graduated implementation ticket (if a priced exact mechanism clears the all-8-cell gate).

## Answer

**Resolved 2026-11-24 — zero production code edits landed (`git diff -- crates/ modules/ vendor/ xtask/` empty; `cargo xtask build-guests --check` and `--accelerated --check` both exit `0`; `32/32` probe runs and `94/94` screen runs `100 %` byte-identical to `t182/reference.json`; checkout-only `verify.py --self-test` PASS + `14/14` negative controls).**

1. **Full 8-cell substage attribution ([`evidence/t189-postpass-screen/FINDINGS.md`](../evidence/t189-postpass-screen/FINDINGS.md), `reduced/probe_wall.json`):**
   - `PostPass` + `crates/slicer-runtime/src/run.rs` tail consumes **`144.4–406.6 ms`** (`11.0–15.4 %`) on `benchy` and **`698.2–2,043.8 ms`** (`8.1–11.0 %`) on `base`.
   - **`PostPass::LayerFinalization`** (`40–169 ms` `benchy`, `170–666 ms` `base`): `build_finalization_world_glue` (`crates/slicer-macros/src/lib.rs`) eagerly marshals `wit_layer.ordered_entities()` across **all `240 / 496` layers** (up to `2.57 M` points) for **all 4 core modules** before `run_finalization` runs — burning **`48.5–53.1 %`** (`19.2–82.6 ms` `benchy`, `83.6–335.6 ms` `base`) on two immediate early-return no-op modules (`com.core.wipe-tower` and `com.core.overhang-classifier-default`, `pushes_count = 0`), marshaling **`99.58–99.80 %`** unused layers for `com.core.skirt-brim` (`skirt_height = 1`), deep-cloning + dropping `Arc<Vec<LayerEntity>>` **4 times** on the host (`11.7–15.6 %` of stage), and re-running `cooling_decision_for_event` inside `com.core.part-cooling`'s per-entity loop (`apply_finalization_pushes` is `< 0.5 ms` with `took_rollback_clone = false` everywhere).
   - **`PostPass::GCodeSerialize`** (`47–101 ms` `benchy`, `246–510 ms` `base`): **`99.20–99.70 %`** is single-threaded `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-gcode/src/serialize.rs`, `461–521 ns/cmd`, `77.6–86.1 MB/s`) performing 2 heap `String` allocations per coordinate + `core::fmt` into an un-preallocated `String`.
   - **`PostPass::GCodeEmit` + `run.rs` tail** (`11–45 + 4.4–9.6 ms` `benchy`, `58–413 + 25.2–50.8 ms` `base`): **`69.7–69.8 %`** of `GCodeEmit` on `base-on` (`283.0–287.9 ms`) and **`36.1–36.8 %`** on `benchy-on` (`16.1–16.2 ms`) is **serial `drop(layer_irs)`** inside `emit_gcode_owned` (`crates/slicer-gcode/src/emit.rs`), alongside 4-`Vec`/entity serial simplification, double-copy `estimate_print`, a **redundant 2nd `estimate_print` call** in `crates/slicer-runtime/src/run.rs` (`3.4–7.3 / 19.9–40.6 ms`), and a serial `;LAYER_CHANGE` scan (`0.9–1.9 / 4.5–8.6 ms`).
   - **`PostPass::GCodePostProcess`** (`40–82 ms` `benchy`, `198–408 ms` `base`): `com.core.machine-gcode-emit` lowers `101,900–976,524` commands by value and issues `101,902–976,526` host `push_*` calls (`85.1–91.3 %` `call_run`) solely to prepend start G-code and append end G-code (`net_added_cmds == 2`). Root fix requires a WIT extension (`PP-4`), so it is priced and parked with `PP-1b` (`part-cooling` `entity-roles`) in `map.md` `## Not yet specified`.

2. **Exact Zero-WIT candidate screen (`PP-1 + PP-2 + PP-3`, `reduced/screen.json`, `94/94` byte-identical to `t182/reference.json`):**
   - Clears the literal all-8-cell `0.80 %` wall floor in **`8/8` cells at `4.45×–11.88×` floor (`24/24` pairwise wall wins)**:
     - `benchy-classic-off` **`+5.39 %`** (`6.74×`), `benchy-classic-on` **`+9.42 %`** (`11.77×`), `benchy-arachne-off` **`+6.88 %`** (`8.60×`), `benchy-arachne-on` **`+9.51 %`** (`11.88×`);
     - `base-classic-off` **`+5.05 %`** (`6.31×`), `base-classic-on` **`+7.87 %`** (`9.84×`), `base-arachne-off` **`+3.56 %`** (`4.45×`), `base-arachne-on` **`+6.02 %`** (`7.52×`).
   - Cuts `PostPass` phase **`−53.7…−63.6 %`** (`GCodeSerialize` `−88.8…−97.0 %` / `8.9×–33.7×`, `LayerFinalization` `−70.7…−77.7 %` / `3.4×–4.5×`, `GCodeEmit` `−35.2…−73.8 %`), and cuts deterministic `--profile` `LayerFinalization` WASM fuel **`−73.98 %`** (`benchy-classic-on`) and **`−75.08 %`** (`base-classic-on`) while keeping `machine-gcode-emit` fuel unchanged (`0.00 %`).
   - **CPU disclosure:** Process CPU improved on `4/8` cells (`+0.43…+3.64 %`) and was flat-to-slightly-higher on `4/8` (`−0.18…−1.80 %`) due to Rayon wakeup / parallel `drop(layer_irs)` Windows heap contention; Ticket 192 explicitly gates and tunes CPU corroboration on the quiet 5-pair/cell standing gate.

3. **Graduated implementation ticket & route impact (`ESTIMATE ONLY`):**
   - Graduated [Exact Zero-WIT `PostPass` acceleration: lazy `LayerFinalization`, parallel zero-alloc `GCodeSerialize`, and `GCodeEmit`/tail cleanup](192-postpass-exact-acceleration.md) (AFK, unblocked, zero WIT changes).
   - Cross-window `ESTIMATE ONLY` composition stacking `t190 + t191 + t192` reaches **all `8/8` matrix cells `< 1.00x`** (`benchy-classic-on`: `1.2036x → 1.0258x → 0.9292x` A/B / `0.9249x` stage; `benchy-arachne-on` `0.8211x`; `benchy-off` `0.8218x / 0.8196x`; `base` `0.5947x / 0.6618x / 0.6556x / 0.6236x` ord).

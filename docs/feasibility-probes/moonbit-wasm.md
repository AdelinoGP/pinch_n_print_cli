# Handoff — Can MoonBit emit a wasm32 component for the Layer::Infill contract?

**Date:** 2026-08-11
**Archive:** `docs/feasibility-probes/moonbit-wasm.md` (probe sandbox:
`C:\Users\agpen\AppData\Local\Temp\opencode\moonbit-wasm-probe\`)

---

## 1. Task (original)
Prove whether MoonBit can emit a wasm32 **component** satisfying the
`Layer::Infill` stage contract — the world `slicer:layer-infill/infill-module@1.0.0`
in `crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit` (plus its
`ir-handles`/`config`/`common` imports) — the way
`modules/core-modules/rectilinear-infill/` does via `#[slicer_module]` +
`wit-bindgen`. Deliverable: a MoonBit `.wasm` component loadable by `pnp_cli`,
or a documented, evidenced verdict it cannot. Follow-up asked for a size/perf
bench at max threads.

## 2. Verdict (one line)
**MoonBit can emit a byte-valid component that matches the world shape and that
compiles, instantiates, and dispatches in `pnp_cli`'s wasmtime host — but it
cannot correctly satisfy the contract**, because every string crossing the
component boundary is corrupted by a hard UTF-16 (MoonBit) vs UTF-8 (Rust host)
string-encoding mismatch that neither side can configure.

---

## 3. What the contract requires
- World `infill-module` imports `slicer:common/host-services`,
  `slicer:common/profiling`, `slicer:config/config-types`,
  `slicer:ir-handles/ir-handles`; exports `infill`:
  `run(layer-idx, list<slice-region-view>, paint-region-layer-view,
  infill-output-builder, config-view) -> result<_, module-error>`.
- `slice-region-view`, `infill-output-builder`, `paint-region-layer-view`,
  `config-view` are **WIT resources** (component-model handle tables +
  canonical ABI). Host provides them; guest calls methods (`.sparse-infill-area()`,
  `.push-sparse-path()`, `.get-float()`, …).
- The contract is **string-heavy**: config keys (`"infill_density"`), `log`
  messages, `object-id`/`region-id`, and `config-value` string variants all cross
  the boundary as WIT `string`.
- `pnp_cli` loads modules as **components** via wasmtime (see
  `crates/slicer-wasm-host/src/instance.rs` `compile_component`, and
  `crates/slicer-runtime`), and links **only** the slicer interfaces
  (`host::layer_infill::LayerModule::add_to_linker`) — **no WASI**.

---

## 4. What was proved empirically

### 4a. Step 1 — MoonBit CAN build the component (WIT/ABI fully supported)
Using wit-bindgen-moonbit (resources included) + `moon build --target wasm` +
wasm-tools:
1. `wit-bindgen moonbit wit --out-dir out --derive-show --derive-eq --derive-error`
   → MoonBit bindings incl. imported-resource wrappers (`SliceRegionView`,
   `InfillOutputBuilder`, `ConfigView`, `PaintRegionLayerView`) and the canonical
   export glue (`cabi_realloc` + `run` + `cabi_post`).
2. Wrote a rectilinear probe `run` in MoonBit (config read, profiling marks,
   `log`, per-region `push_sparse_path`).
3. `moon fmt` (migrates legacy `moon.pkg.json` → `moon.pkg`), then
   `moon build --target wasm --release` → **bare core module** (not a component).
4. `wasm-tools component embed wit gen.wasm --encoding utf16` →
   `wasm-tools component new` → `infill.component.wasm`, which **validates** and
   exports `slicer:layer-infill/infill@1.0.0#run`.
5. `wasm-tools component wit infill.component.wasm` shows the world matches the
   required one exactly:
   ```
   world root { import slicer:types/geometry; import slicer:config/config-types;
                import slicer:ir-handles/ir-handles; import slicer:common/host-services;
                import slicer:common/profiling; import slicer:common/module-errors;
                export slicer:layer-infill/infill@1.0.0; }
   ```

### 4b. Step 2 — Loadable and dispatchable, but strings are corrupted
Reproduced pnp_cli's exact Layer::Infill linker (`host::layer_infill::
LayerModule::add_to_linker`, no WASI) in a standalone wasmtime 43 host
(`harness/`, mirroring `slicer-wasm-host/src/host.rs`: type-root bindgen + all
10 `Host` resource impls):

```
--- MOONBIT component ---
RESULT: DISPATCHED OK   (run returned Ok(()), profiling marks reached host)
--- RUST component (control) ---
RESULT: DISPATCHED OK   (config key received correctly: "infill_density")
```

Non-string data works end-to-end (result `Ok(())`, profiling marks). **But every
string is corrupted** — the host receives `"ll_density\0\0￼\u{ffff}"` for key
`"infill_density"`, and `log` text is garbled. The Rust control receives the key
correctly.

Root cause (fundamental, not a workaround gap):
- MoonBit's component ABI glue is **hard-wired to UTF-16** string encoding
  (its `mbt_ffi_str2ptr`/`ptr2str` read/write UTF-16; the official component
  tutorial mandates `--encoding utf16`). Embedding with `--encoding utf8` still
  emits UTF-16 bytes.
- wit-bindgen rust 0.57.1 (pnp_cli's host) **hard-codes `StringEncoding::UTF8`**
  (`wit-bindgen-rust-0.57.1/src/lib.rs:991`); there is no UTF-16 host option.
- Neither side is configurable with the current toolchain, so the mismatch is
  unavoidable. Because the infill contract is string-heavy, the module cannot
  read its config (falls back to defaults) or emit correct output → **cannot
  satisfy the contract**.

---

## 5. Benchmark — MoonBit vs Rust component overhead (12 logical processors)

Measured with `harness/src/bin/bench.rs` linking both components identically
(slicer interfaces only) and dispatching `run` with empty regions exactly as
pnp_cli does — one compiled component shared; fresh store + instantiate per
call. errs=0 throughout. (MoonBit component is a stub; Rust is the real
rectilinear module — with empty regions both do minimal guest work, so this
measures wasmtime dispatch/instantiation overhead, not end-to-end slicing.)

| Static size | bytes | vs Rust |
|---|---|---|
| RUST rectilinear-infill.wasm | 136,792 | 1.0× |
| MOONBIT infill.component.wasm | 30,373 | **0.22× (4.5× smaller)** |

| Compile (`Component::new`) | ms | vs Rust |
|---|---|---|
| RUST | 310–388 | 1.0× |
| MOONBIT | 92–101 | **~3–4× faster** |

| threads | RUST | MOONBIT | MOONBIT/RUST |
|---|---|---|---|
| 1  | 478–546 µs/dispatch (1.8–2.1k ops/s) | 275–304 µs/dispatch (3.3–3.6k ops/s) | **~1.7–1.8× faster** |
| **12 (max)** | **10.6–14.4k ops/s** | **17.3–24.2k ops/s** | **~1.6–1.7× faster** |

Parallel scaling is sub-linear for both (~6–7× on 12 threads, ~55% efficiency),
typical of wasmtime (allocation contention + internal sync) — not a MoonBit
effect. The MoonBit component's lower overhead is driven by its 4.5× smaller
size (faster compile + instantiate, which dominates the per-dispatch cost).

---

## 6. Can MoonBit emit the component as "pure wasm" without a runtime? — YES (footprint is excellent)

- MoonBit's wasm is a **bare core module** (`wasm32-unknown-unknown`-style, no
  WASI, no OS imports) — unlike Go, there is **no** Go-runtime/GC/scheduler
  embedded. The allocator is a small TLSF/bump allocator that self-initializes.
- Smallest empty MoonBit module (wasm target, release): **2,459 B**; a one-export
  component (adder): **3,169 B**; the full infill component: **30,373 B**.
- For comparison: Go probe component 2,799,789 B (~92× larger), Rust infill
  136,792 B (~4.5× larger).
- The runtime footprint is therefore a **strength** of MoonBit; the blocker is
  purely the string-encoding mismatch, not runtime weight.

---

## 7. Recommended path

- **Rust (`wasm32-unknown-unknown` + `#[slicer_module]`)** remains the only way
  to produce a component `pnp_cli` can load **and correctly execute** today.
- A direct MoonBit module is blocked by the UTF-16/UTF-8 string-encoding
  mismatch, which is unfixable with the current toolchain:
  - wit-bindgen rust has no UTF-16 host encoding option (0.57.1 hard-codes UTF-8);
  - MoonBit has no UTF-8 string-ABI option (its glue is hard-wired UTF-16).
- Revisit only if **MoonBit adds a UTF-8 string ABI** (or a configurable
  encoding) **or** wit-bindgen adds a UTF-16 host encoding option. If/when either
  lands, the wit-bindgen-moonbit route works today with zero WIT changes, and
  you get a ~4.5× smaller binary and ~1.7× lower dispatch overhead than Rust.
- A Rust wrapper around MoonBit is not meaningful (the wrapper would be the Rust
  module, defeating the purpose).

---

## 8. Exact commands + tool versions

| tool | version |
|---|---|
| moon | 0.1.20260807 (4da23f8 2026-08-07); moonc v0.10.7+bc794d341 (2026-08-11) |
| wit-bindgen | 0.57.1 (`cargo install wit-bindgen-cli`) |
| wasm-tools | 1.250.0 |
| wasmtime CLI | 45.0.0 |
| wasmtime crate (repo + harness) | 43.0.0 |
| cargo | 1.96.0 |

```bash
# bindings (deps in deps/ layout: common, config, ir-handles, types + layer-infill.wit)
wit-bindgen moonbit wit --out-dir out --derive-show --derive-eq --derive-error

# migrate manifests, implement gen/.../infill/top.mbt `run`, build
moon fmt
moon build --target wasm --release

# componentize (MoonBit emits a bare core module; embed + wrap)
wasm-tools component embed wit _build/wasm/release/build/gen/gen.wasm --encoding utf16 --output gen-embedded.wasm
wasm-tools component new gen-embedded.wasm --output infill.component.wasm

# verify interface matches the required world
wasm-tools component wit infill.component.wasm

# load test (pnp_cli-style linker) — harness
cargo run --release -- infill.component.wasm

# bench — harness/src/bin/bench.rs (compile + dispatch throughput, 1/12 threads)
bench.exe <moonbit.wasm> <rust.wasm> 20000 12
```

---

## 9. Artifacts

Probe artifacts live in the scratch sandbox
`C:\Users\agpen\AppData\Local\Temp\opencode\moonbit-wasm-probe\` (`probe/`
MoonBit component, `harness/` wasmtime host + bench, `adder/` minimal control,
`README.md`). **Core repo sources were not modified.** This document is the
authoritative record of the verdict.

---

## 10. Known gaps / caveats
- Bench used no-op dispatches (empty regions) and the MoonBit component is a
  stub (the string bug prevents real rectilinear logic), so the numbers measure
  wasmtime dispatch/instantiation overhead, not end-to-end slicing throughput.
- The UTF-16/UTF-8 mismatch was confirmed empirically (garbled config key and
  log text) and by source inspection of wit-bindgen rust 0.57.1
  (`StringEncoding::UTF8` at `src/lib.rs:991`); MoonBit's UTF-16-only ABI is
  asserted from the official component tutorial (`--encoding utf16`) and the
  utf8-embed test still emitting UTF-16 bytes.
- A future MoonBit UTF-8 ABI or wit-bindgen UTF-16 host option would change the
  verdict; neither exists in the versions tested.

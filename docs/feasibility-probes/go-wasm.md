# Handoff — Can Go emit a wasm32 component for the Layer::Infill contract?

**Date:** 2026-08-11
**Archive:** `docs/feasibility-probes/go-wasm.md` (original probe sandbox removed)

---

## 1. Task (original)
Prove whether Go can emit a wasm32 **component** satisfying the `Layer::Infill`
stage contract — the world `slicer:layer-infill/infill-module@1.0.0` in
`crates/slicer-schema/wit/deps/layer-infill/layer-infill.wit` (plus its
`ir-handles`/`config`/`common` imports) — the way
`modules/core-modules/rectilinear-infill/` does via `#[slicer_module]` +
`wit-bindgen`. Deliverable: a Go `.wasm` component loadable by `pnp_cli`, or a
documented, evidenced verdict it cannot. Follow-up asked for a size/perf bench
at max threads and whether the component can be "pure wasm without a Go runtime".

## 2. Verdict (one line)
**Go can emit a byte-valid component that matches the world shape, but it cannot
be loaded by `pnp_cli` as-is**, and no Go toolchain can produce the Rust-style
WASI-free, runtime-free pure-wasm module this contract needs.

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
- `pnp_cli` loads modules as **components** via wasmtime (see
  `crates/slicer-wasm-host/src/instance.rs` `compile_component`, and
  `crates/slicer-runtime`), and links **only** the slicer interfaces
  (`host::layer_infill::LayerModule::add_to_linker`) — **no WASI**.

---

## 4. What was proved empirically

### 4a. Step 1 — Go CAN build the component (WIT/ABI fully supported)
Using wit-bindgen-go (resources included) + Go wasip1 + wasm-tools:
1. `wit-bindgen go -w slicer:layer-infill/infill-module@1.0.0 <deps>` → Go
   bindings incl. imported-resource wrappers (`SliceRegionView`,
   `InfillOutputBuilder`, `ConfigView`, `PaintRegionLayerView`).
2. Wrote a real rectilinear scan-line infill in Go (`Run`).
3. `GOOS=wasip1 GOARCH=wasm go build -buildmode=c-shared -ldflags=-checklinkname=0 .`
   → `core.wasm` (2.69 MB).
4. `wasm-tools component embed -w infill-module <wit> core.wasm` →
   `core-with-wit.wasm`.
5. `wasm-tools component new --adapt wasi_snapshot_preview1.reactor.wasm …` →
   `component.wasm`, which **validates** and exports
   `slicer:layer-infill/infill@1.0.0`.

### 4b. Step 2 — NOT loadable by pnp_cli (empirically confirmed)
Reproduced pnp_cli's exact Layer::Infill linker (`host::layer_infill::
LayerModule::add_to_linker`, no WASI) in a probe:

```
--- GO component ---
RESULT: INSTANTIATION FAILED
component imports instance `wasi:cli/environment@0.2.6`, but a matching implementation was not found in the linker
--- RUST component (control) ---
RESULT: INSTANTIATED OK
```

Root cause (fundamental, not a workaround gap):
- Rust builds for `wasm32-unknown-unknown` (**no WASI**); its component imports
  only the slicer interfaces.
- Go has **only** `js/wasm` and `wasip1/wasm` targets — no
  `wasm32-unknown-unknown`. Its wasip1 runtime always links
  `wasi_snapshot_preview1`; the `--adapt` step rewrites those into a full set of
  **WASI preview2** imports (`wasi:cli/environment@0.2.6`, `wasi:cli/exit@0.2.6`,
  `wasi:io/*`, `wasi:clocks/*`, `wasi:filesystem/*`, `wasi:random/*`, …).
- `crates/slicer-wasm-host` has **zero** WASI support (no `wasi-common`, no
  preview2 — confirmed by grep), so a Go component can never instantiate there.

---

## 5. Benchmark — Go vs Rust component overhead (12 logical processors)

Measured with a `hostprobe` release build linking both components identically
(slicer interfaces + WASI preview2) and dispatching a no-op `run` (empty
regions) exactly as pnp_cli does — one compiled component shared; fresh store +
instantiate per call. errs=0 throughout.

| Static size | bytes | vs Rust |
|---|---|---|
| RUST rectilinear-infill.wasm | 136,792 | 1.0× |
| GO component.wasm | 2,799,789 | **20.5×** |

| Compile (`Component::new`) | ms | vs Rust |
|---|---|---|
| RUST | 24 (range 21–27) | 1.0× |
| GO | 290 (range 276–298) | **11–13×** |

| threads | RUST | GO | GO/RUST |
|---|---|---|---|
| 1  | 16,600–19,200 ops/s (0.052–0.060 ms) | 1,680–1,850 ops/s (0.53–0.60 ms) | **9.5–11× slower** |
| 4  | 32,800–49,300 (0.020–0.030 ms) | 4,640–5,910 (0.17–0.21 ms) | **7–8× slower** |
| 8  | 35,100–50,900 (0.020–0.028 ms) | 5,900–7,630 (0.13–0.17 ms) | **6–6.7× slower** |
| **12 (max)** | **38,600–43,800 (0.022–0.026 ms)** | **6,100–7,800 (0.13–0.16 ms)** | **≈5.5–6× slower** |

Per-instance linear memory (post-instantiate, wasmtime limiter):
**RUST 1,088 KB vs GO 3,200 KB (2.9×)**.

Interpretation: these are no-op dispatches. The per-dispatch overhead Go pays is
the per-instance Go-runtime init (GC/scheduler/allocator), which pnp_cli incurs
on every per-layer dispatch call because it instantiates fresh per call. On a
busy multi-region layer the (identical) geometry dominates wall time, but the Go
runtime overhead is paid regardless. At max threads Go saturates ~6–7.8k ops/s
vs Rust ~39–44k ops/s.

---

## 6. Can Go emit the component as "pure wasm" without the Go runtime? — NO

- Stock Go always embeds its runtime in the wasm: `core.wasm` contains 67+
  `runtime.*` symbols (GC, goroutine scheduler, `runtime.Pinner`,
  `runtime.AddCleanup`, `runtime.mallocgc`).
- The wit-bindgen-go generated glue itself depends on `runtime.Pinner` /
  `runtime.AddCleanup`, and the component's `cabi_realloc`/allocator come from
  the runtime.
- Go has no `wasm32-unknown-unknown` target (`go tool dist list` → only
  `js/wasm`, `wasip1/wasm`), so every Go wasm is runtime-bearing **and**
  WASI-dependent. The WASI-import blocker is therefore unavoidable, not a
  build-flag fix.
- **TinyGo** produces small runtime-light binaries but does not support the
  `runtime.Pinner`/`runtime.AddCleanup` calls these bindings require, and its
  wasm is still `wasip1`-based — so it neither runs this bindings set nor removes
  the WASI blocker.

---

## 7. Recommended path

- **Rust (`wasm32-unknown-unknown` + `#[slicer_module]`)** remains the only way
  to produce a component `pnp_cli` can load today.
- A Go module is reachable only if the **host** later adds WASI preview2 to
  `slicer-wasm-host`'s linker (a core-repo change, out of scope). If/when that
  lands, the wit-bindgen-go route works today with zero WIT changes — but you
  still accept ~6× per-dispatch overhead, ~3× memory, ~20× binary size.
- Direct Go (or TinyGo) "pure wasm without a Go runtime" is **not achievable**
  for this contract with any current Go toolchain.

---

## 8. Exact commands + tool versions

| tool | version |
|---|---|
| go | 1.26.5 (windows/amd64); wasm targets: `js/wasm`, `wasip1/wasm` only |
| wit-bindgen | 0.57.1 (`cargo install wit-bindgen-cli --version 0.57.1 --features go`) |
| wasm-tools | 1.250.0 |
| wasmtime CLI | 45.0.0 |
| wasmtime crate (repo + bench) | 43.0.1 / 43.0.2 |
| go.bytecodealliance.org/pkg | v0.2.1 |
| rustc | 1.96.0 |

```bash
# bindings (deps first, dependency order)
wit-bindgen go -w slicer:layer-infill/infill-module@1.0.0 \
  wit/types wit/config wit/ir-handles wit/common wit/prepass-types wit/layer-infill.wit

# core module
GOARCH=wasm GOOS=wasip1 go build -o core.wasm -buildmode=c-shared -ldflags=-checklinkname=0 .

# component
wasm-tools component embed -w infill-module <wit-with-deps> core.wasm -o core-with-wit.wasm
wasm-tools component new --adapt wasi_snapshot_preview1.reactor.wasm core-with-wit.wasm -o component.wasm

# load test (pnp_cli-style linker) — hostprobe
# bench — hostprobe (compile + dispatch throughput + memory, 1/4/8/12 threads)
```

---

## 9. Artifacts

Probe artifacts (scratch) were mirrored to the repo's gitignored `tmp/`
directory under `tmp/go-wasm-infill-probe/` (`build.sh`, `infill.go`, `wit/`,
`hostprobe/`, `FINDINGS.md`, both `.wasm` artifacts). **Core repo sources were
not modified.** This document is the authoritative record of the verdict.

---

## 10. Known gaps / caveats
- Bench used no-op dispatches (empty regions). Real geometry (identical
  scan-line logic) would narrow the relative ratio on busy layers but not remove
  the per-call Go-runtime init cost.
- TinyGo incompatibility with `runtime.Pinner`/`runtime.AddCleanup` is asserted
  from knowledge, not tested here (TinyGo not installed in the probe sandbox).
- WASI preview2 was provided to the bench linker only to get the Go component
  to instantiate for measurement; pnp_cli does **not** provide it in production.

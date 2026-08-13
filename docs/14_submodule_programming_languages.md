# Submodule / Community Module Programming Languages

How modules in Pinch 'n Print can be authored in languages other than Rust, given
what the [bytecodealliance/wit-bindgen](https://github.com/bytecodealliance/wit-bindgen)
project supports and what this project's host actually requires.

**When to read this:** when choosing (or reviewing) a language for an external /
community module, or assessing whether a proposed module language can satisfy a
PnP **stage contract**. Paired with `docs/03_wit_and_manifest.md` (WIT worlds and
manifest) and `docs/05_module_sdk.md` (the Rust SDK surface).
See `docs/specs/community-modules-dragon-curve-infill.md` for the first community
module and its Go/MoonBit feasibility probes.

---

## This project's guest pipeline (the constraint that matters)

A PnP module is a **WebAssembly component** that satisfies one **stage contract**:
it imports the stage's host services (`slicer:common/host-services`,
`slicer:config/config-types`, `slicer:ir-handles/ir-handles`) and exports the
stage's `run` function (e.g. `slicer:layer-infill/infill.run`).

Today the only shipped guest language is **Rust**:

- Generated bindings come from `wit-bindgen` (`Cargo.toml` workspace:
  `wit-bindgen = "0.60.0"`).
- The `#[slicer_module]` macro (`crates/slicer-macros`) emits a `wit_bindgen::generate!`
  call gated behind `#[cfg(target_arch = "wasm32")]`, plus the component export
  shims.
- Guests are built for **`wasm32-unknown-unknown`** — deliberately **no WASI**.
  `crates/slicer-sdk/src/host.rs` records "Guests build for `wasm32-unknown-unknown`
  (no WASI)"; the host (wasmtime, `crates/slicer-wasm-host`) supplies the WIT world
  directly.
- The SDK surface (`slicer-sdk` builders, views, config-resolution helpers,
  error handling) is a **Rust crate**. Community modules in the temporary workflow
  re-declare the WIT types they need rather than depending on the SDK as a crate.

The consequence for other languages: **wit-bindgen language support is necessary
but not sufficient.** A non-Rust module must also reproduce the SDK's host-service
interaction in that language (push paths into builders, read region/config views)
and emit a component importable by the wasmtime host without WASI.

---

## Languages supported by wit-bindgen

Status is as reported by the `bytecodealliance/wit-bindgen` README at the time of
writing. "Generator" = wit-bindgen emits bindings for that language.

| Language | wit-bindgen generator | Target / toolchain | Component creation | PnP feasibility |
|---|---|---|---|---|
| **Rust** | Yes (first-class) | `wasm32-wasip2` native (rustc ≥ 1.82), or `wasm32-unknown-unknown` | component directly | **Only shipped language today.** Full SDK + `#[slicer_module]`. |
| **C** | Yes (`wit-bindgen c`) | `wasm32-wasip1` via WASI SDK clang | `wasm-tools component new` | Possible; needs wasip1→component and manual host-service calls (no SDK). |
| **C++ (C++-17+)** | Yes (`wit-bindgen` cpp crate) | `wasm32-wasip1` via WASI SDK | `wasm-tools component new` | Possible; same caveats as C. |
| **C# (.NET)** | Yes (`wit-bindgen csharp`) | `wasi-wasm` RID, native-aot, `componentize-dotnet` | `dotnet publish` | Possible; heavier toolchain, native-aot GC into the module. |
| **Go** | Yes (`wit-bindgen-go`) | `GOOS`/`GOARCH` wasm — **only `js/wasm` and `wasip1/wasm`** (no `wasm32-unknown-unknown`) | `wasm-tools component new` | **Verdict (2026-08-11): NOT loadable by `pnp_cli`.** Component builds & validates, but always imports WASI preview2 (host has none). ~20× size, ~6× dispatch, ~3× memory. See §Community-module context. |
| **TinyGo** | Deprecated (was `go.bytecodealliance.org`) | — | — | Do not use; migrate to Go. |
| **MoonBit** | Yes (`wit-bindgen moonbit`) | `moon build --target wasm` (bare core module) | `wasm-tools component embed` + `component new` | **Verdict (2026-08-11): loadable & dispatchable, but NOT correct.** Every string crossing the boundary is corrupted (MoonBit UTF-16 vs Rust host UTF-8, neither configurable). 4.5× smaller, ~1.7× faster dispatch. See §Community-module context. |
| **Java** | **Removed** (TeaVM-WASI unmaintained) | — | — | Not supported. |
| **JavaScript** | Via `componentize-js` (separate) | JS/WASI | componentize-js | Interpreted; not a PnP stage-contract fit today. |
| **Python** | Via `componentize-py` (separate) | Python/WASI | componentize-py | Interpreted; not a PnP stage-contract fit today. |

### MoonBit ABI caveat
MoonBit bindings emit **inline wasm helpers** for strings/bytes/arrays. MoonBit
changed the ABI layout for these (the data pointer no longer needs the old
8-byte offset), which is a **breaking change** the MoonBit compiler cannot detect
at the type level. Generated bindings must be regenerated with a `wit-bindgen`
version matching the MoonBit toolchain's ABI layout; mixing old bindings with a
newer toolchain compiles but corrupts strings/bytes or traps at runtime. Regenerate
`**/stub.mbt` accordingly (`--ignore-stub` skips touching `moon.pkg.json` /
`moon.mod.json`).

---

## Why "wit-bindgen supports it" ≠ "PnP supports it"

A PnP module must do three things beyond having bindings:

1. **Import the PnP WIT world** (`slicer:common`, `slicer:config`,
   `slicer:ir-handles`) and **export** the stage `run`.
2. **Drive the host services** — construct paths, push them into output builders,
   read region/config views, resolve per-region config, report errors. In Rust this
   is the SDK; in any other language it is hand-written in that language against
   the generated bindings.
3. **Load in the wasmtime host without WASI.** This project targets
   `wasm32-unknown-unknown`; languages whose toolchains assume `wasip1` (C/C++ via
   WASI SDK, C# RID, Go wasip1/2) must componentize and be adapted so the component
   imports only the PnP world.

Rust remains the only language with first-class support because it is the only
language for which `slicer-sdk` + `#[slicer_module]` exist. Every other language is
possible in principle but requires re-authoring the SDK's host-service surface.

---

## Community-module context

The temporary community-module workflow (before the SDK is authored as a
dependency) tested **Go** and **MoonBit** as alternative authoring languages;
both probes are complete and neither is loadable-and-correct in `pnp_cli` (Go:
WASI blocker; MoonBit: string-encoding mismatch). The design lives in
`docs/specs/community-modules-dragon-curve-infill.md`, but that spec may be
archived — this doc is the **living record**, so verdicts live here (below), not
in the spec. No new language is enabled in the host until a probe proves the
component loads and runs correctly under `pnp_cli`.

### Go probe — verdict (2026-08-11): not loadable

Go can emit a **byte-valid component** that matches the `Layer::Infill` world
shape, but it **cannot be loaded by `pnp_cli`**, and no Go toolchain can produce
the WASI-free, runtime-free pure-wasm module this contract needs.

- **It works up to componentization.** `wit-bindgen-go` (v0.2.1, resources
  included) + `GOOS=wasip1 GOARCH=wasm go build -buildmode=c-shared` + `wasm-tools
  component embed/new --adapt` yields a component that validates and exports
  `slicer:layer-infill/infill@1.0.0`.
- **It fails to instantiate.** Reproducing `pnp_cli`'s exact `Layer::Infill`
  linker (slicer interfaces only, no WASI) gives
  `INSTANTIATION FAILED: imports wasi:cli/environment@0.2.6, no implementation in
  the linker`. Go has **only** `js/wasm` and `wasip1/wasm` targets — no
  `wasm32-unknown-unknown` — so its wasip1 runtime always links WASI preview1,
  and `--adapt` rewrites that into a full set of **WASI preview2** imports. The
  host (`crates/slicer-wasm-host`) has zero WASI support, so the blocker is
  fundamental, not a build-flag fix.
- **No "pure wasm" option.** Stock Go always embeds its runtime (67+ `runtime.*`
  symbols; the wit-bindgen-go glue depends on `runtime.Pinner`/`runtime.AddCleanup`,
  which come from the runtime). TinyGo is runtime-light but doesn't support those
  calls and is still wasip1-based.
- **Overhead (no-op dispatch, this machine):** size 2,799,789 B vs Rust 136,792 B
  (**20.5×**); `Component::new` 290 ms vs 24 ms (**11–13×**); dispatch ~6–7.8k
  ops/s at 12 threads vs Rust ~39–44k (**~5.5–6× slower**); per-instance memory
  3,200 KB vs 1,088 KB (**2.9×**).

**Recommended path:** Rust (`wasm32-unknown-unknown` + `#[slicer_module]`) remains
the only way to produce a loadable component today. A Go module is reachable only
if the host later adds WASI preview2 to `slicer-wasm-host`'s linker (a core-repo
change, out of scope for the community-module workflow). Evidence: full probe
record at `docs/feasibility-probes/go-wasm.md`.

### MoonBit probe — verdict (2026-08-11): loadable, not correct

MoonBit **compiles, instantiates, and dispatches** in `pnp_cli`'s wasmtime host
(unlike Go) — but it **cannot satisfy the contract**, because every string
crossing the component boundary is corrupted by a hard **UTF-16 (MoonBit) vs
UTF-8 (Rust host)** encoding mismatch that neither side can configure.

- **It works up to componentization.** `wit-bindgen moonbit` + `moon build
  --target wasm --release` yields a bare core module (no WASI, no runtime), then
  `wasm-tools component embed --encoding utf16` + `component new` validates and
  exports `slicer:layer-infill/infill@1.0.0`. Non-string data (result `Ok(())`,
  profiling marks) flows end-to-end.
- **It fails on strings.** The host receives `"ll_density\0\0\u{ffff}"` for config
  key `"infill_density"`; `log` text is garbled. Root cause: MoonBit's component
  glue is hard-wired to **UTF-16** (embedding with `--encoding utf8` still emits
  UTF-16 bytes), while `wit-bindgen` rust 0.57.1 (the host) hard-codes
  `StringEncoding::UTF8` with no UTF-16 option. The infill contract is
  string-heavy, so the module can't read its config (falls back to defaults) or
  emit correct output.
- **Footprint is a strength.** Bare core module, no embedded runtime; smallest
  empty module 2,459 B, full infill component 30,373 B (**4.5× smaller** than
  Rust, ~92× smaller than the Go probe). Dispatch ~1.7× faster at max threads.
  The blocker is purely the encoding mismatch, not runtime weight.
- **No meaningful wrapper path.** A Rust wrapper around MoonBit would be the Rust
  module, defeating the purpose (unlike Go).

**Recommended path:** Rust (`wasm32-unknown-unknown` + `#[slicer_module]`) remains
the only way to produce a component `pnp_cli` can load **and correctly execute**
today. Revisit MoonBit only if it gains a UTF-8 string ABI (or a configurable
encoding) **or** `wit-bindgen` gains a UTF-16 host encoding option; either would
make the current route work with zero WIT changes. Evidence: full probe record at
`docs/feasibility-probes/moonbit-wasm.md`.


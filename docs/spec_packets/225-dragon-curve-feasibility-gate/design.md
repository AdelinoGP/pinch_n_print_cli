# Design: 225-dragon-curve-feasibility-gate

## Controlling Code Paths

- Primary code path: the wasmtime host surface in `crates/slicer-wasm-host` (`src/instance.rs` engine/store construction, `src/host.rs` `wasmtime::component::bindgen!` + `add_to_linker` + `ResourceLimiter`, `src/dispatch.rs` linker construction) plus `crates/slicer-runtime/src/run.rs` engine reuse.
- Neighboring tests/fixtures: `crates/slicer-wasm-host/tests/contract/host_services_tdd.rs` (host-services bindgen smoke), `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs` (WIT/guest drift gate), and the full `test-guests/` + `wit-guest/` tree rebuilt by `cargo xtask build-guests`.
- OrcaSlicer comparison: n/a (this packet has no OrcaSlicer parity consultation).

## Architecture Constraints

- **Toolchain versions are exact.** wasmtime must be `47.0.3` and wit-bindgen `0.60.0`; no "47.x" or "0.60.x" looseness. The `call-hook` feature must be retained verbatim — it is the fuel-sampling mechanism pinned by ADR-0055 (`docs/adr/0055-*`; the workspace comment at `Cargo.toml:57-60` states this explicitly).
- **The gate measures, it does not assume.** The Go probe must reproduce pnp_cli's exact `Layer::Infill` linker (slicer interfaces only, zero WASI) against wasmtime 47 and record the instantiation result honestly. The known hypothesis (host still must link WASI preview2, so wasmtime 47 alone likely does not change the blocker) is a prior, not a conclusion — if the linker still lacks `wasi:cli/environment@0.2.6`, the honest result is `INSTANTIATION FAILED` and the fallback is confirmed.
- **MoonBit is not re-run.** `moon` is absent on this machine; the verdict table must record `not re-run (toolchain absent)` and the Go result is the sole gate-deciding evidence. No fabricated MoonBit re-check.
- **Guest WASM staleness.** Bumping wit-bindgen invalidates every guest's generated bindings; the mandatory closure is `cargo xtask build-guests` (rebuild) followed by `cargo xtask build-guests --check` (freshness gate green). Do not skip the rebuild or the re-check.
- No schema/version constant, no WIT change, and no new ADR. This packet changes no IR shape, no WIT identifier, no claim vocabulary, and no manifest contract. ADR-0044 and ADR-0058 are untouched.

## Code Change Surface

- Selected approach: pin bump at the workspace root first, sweep for stale in-tree pins, then drive the compile gate and let `cargo check`/`clippy`/`build-guests` enumerate the API fallout. The fallout fix list is bounded to the wasmtime API surface and the wit-bindgen generated-shape surface; both are grounded below.
- Exact functions, traits, manifests, tests, and fixtures:
  - `Cargo.toml:61-62` — `wasmtime = { version = "47.0.3", features = ["call-hook"] }`, `wit-bindgen = "0.60.0"`.
  - `crates/slicer-wasm-host/src/instance.rs` — `WasmEngine::with_profiling` (`wasmtime::Config::new`, profiling flags, `consume_fuel`), `new_store` (`Store::new`, `set_fuel`, `call_hook`, `get_fuel`, `CallHook::CallingHost`), `WasmEngine` wrapper.
  - `crates/slicer-wasm-host/src/host.rs` — the `wasmtime::component::bindgen!({ ... with: { ... } })` invocations (all stage worlds), `impl wasmtime::ResourceLimiter for MemTracker` (`memory_growing`, `table_growing`), `ResourceTable`, and the `add_to_linker::<_, wasmtime::component::HasSelf<_>>` wiring.
  - `crates/slicer-wasm-host/src/dispatch.rs` — every `wasmtime::component::Linker::<HostExecutionContext>::new(engine)` and `add_to_linker` call site (one per stage family).
  - `crates/slicer-runtime/src/run.rs` — `wasmtime::Engine` reuse (`WasmInstancePool` handle).
  - wit-bindgen 0.60 generated-shape consumers: `crates/slicer-sdk/src/host.rs` `__sdk_host_services_import` / `__sdk_host_log_import` / `__sdk_host_medial_axis_import` / `__sdk_host_arachne_import` `generate!` blocks, `crates/slicer-macros/src/lib.rs` (`__slicer_path_ir_to_wit` / `__slicer_path_wit_to_ir` / `__slicer_ir_path_to_wit` and the per-world generated glue), and every `modules/*/wit-guest/` + `crates/slicer-wasm-host/test-guests/*/wit-guest/` generated binding.
- Rejected alternatives and reasons:
  - **Bump only `slicer-wasm-host`'s local wasmtime dep.** Rejected: `slicer-runtime` also depends on wasmtime through the workspace pin, and a mixed 43/47 graph risks two `wasmtime::Engine` generations with incompatible component compilation; the workspace pin is the single source of truth.
  - **Fix fallout by reading `host.rs` end-to-end.** Rejected: >5,000 lines; the API surface is enumerable by grep, so a `LOCATIONS` dispatch bounds the read and keeps context cost at M.
  - **Re-run MoonBit by installing it.** Rejected: out of scope and not present; the plan file (Grounding facts) explicitly forbids treating MoonBit as gate-deciding on this machine.

## Files in Scope (read + edit)

- `Cargo.toml` — role: primary pin bump; expected change: two-line version bump.
- `crates/**/Cargo.toml`, `modules/**/Cargo.toml` — role: sweep stale pins; expected change: the 24 inline `wit-bindgen = "0.57.1"` manifests become `0.60.0` (21 `test-guests/*/Cargo.toml` + 3 `wit-guest/Cargo.toml`); the `wit-bindgen.workspace = true` crates inherit the root bump with no edit.
- `crates/slicer-wasm-host/src/instance.rs` — role: wasmtime 47 engine/store API fallout; expected change: config/store/call-hook/fuel surface updates.
- `crates/slicer-wasm-host/src/host.rs` — role: wasmtime 47 bindgen/resource-limiter fallout; expected change: bindgen/resource-table/limiter signature updates.
- `crates/slicer-wasm-host/src/dispatch.rs` — role: linker construction fallout; expected change: linker generic-parameter updates if wasmtime 47 changed them.
- `crates/slicer-runtime/src/run.rs` — role: engine reuse fallout; expected change: none beyond what `cargo check` reports.
- `docs/14_submodule_programming_languages.md` — role: verdict record; expected change: §Community-module context edits.
- `docs/feasibility-probes/go-wasm.md` — role: evidence record; expected change: appended dated re-check section.

Justification for >3 files: a workspace toolchain bump is by definition cross-cutting; the scope is bounded to the wasmtime-API surfaces plus two docs, with all fallout enumerated by the compile gate rather than browsed.

## Read-Only Context

- `crates/slicer-wasm-host/src/host.rs` — grep-delegated ranges only (bindgen blocks near lines 331–600 and 914–1060; `ResourceLimiter` near 1087–1124; `add_to_linker` in dispatch) — purpose: verify the exact wasmtime 47 API surface without a full-file read.
- `crates/slicer-macros/src/lib.rs` — grep-delegated `__slicer_path_ir_to_wit` / `__slicer_path_wit_to_ir` / `__slicer_ir_path_to_wit` ranges (near 1290–1330, 2599–2603, 2739–2750) — purpose: confirm the generated path converters survive the wit-bindgen 0.60 shape.
- `docs/feasibility-probes/go-wasm.md` §8 — lines 151–177 only — purpose: the exact probe commands to reproduce.
- `docs/feasibility-probes/moonbit-wasm.md` §2/§8 — delegated SUMMARY — purpose: original MoonBit verdict/commands (no re-run).

## Out-of-Bounds Files

- `target/`, `Cargo.lock`, generated code (everything under `**/wit-guest/` is read-only — regenerated by build-guests, never hand-edited), vendored dependencies.
- `OrcaSlicerDocumented/` — delegate; never load (no parity obligation here).
- `docs/specs/community-modules-dragon-curve-infill.md` §1–§5 and §7 — not edited; this packet implements §6 only.
- `docs/DEVIATION_LOG.md`, `docs/adr/` — no deviation/ADR authored in this packet.

## Expected Sub-Agent Dispatches

- Question: list every `wasmtime::component::bindgen!`, `add_to_linker`, `ResourceLimiter`, `call_hook`, `get_fuel`, `Store::new`, `Engine` occurrence with line numbers in `crates/slicer-wasm-host/src/host.rs`, `dispatch.rs`, `instance.rs`, and `crates/slicer-runtime/src/run.rs`; scope: those four files; return: `LOCATIONS`; purpose: Step 4 fallout bounding.
- Question: confirm the exact reproduction commands and `RESULT:`/`INSTANTIATION FAILED` output shape from `docs/feasibility-probes/go-wasm.md` §8 and §4b; scope: that file; return: `SNIPPETS`; purpose: Step 5 probe brief.
- Question: confirm §Community-module context line ranges and the three paragraphs to edit in `docs/14_submodule_programming_languages.md`; scope: lines 96–171; return: `SNIPPETS`; purpose: Step 6 doc edit.
- Question: `go version`, `wasm-tools --version`, `cargo install wit-bindgen-cli --version 0.60.0 --features go` availability, and `moon --version` (expect absence); scope: shell; return: `FACT`; purpose: Step 5 toolchain confirmation.

## Data and Contract Notes

- IR/manifest contracts: none changed.
- WIT boundary: none changed (the Go probe's WIT is a scratch re-declaration of the existing `slicer:layer-infill/infill-module@1.0.0` world; the in-tree WIT is untouched).
- Determinism/scheduler constraints: the gate verdict must be a single recorded line, not a heuristic; reproducibility of the Go build is byte-level (same tool versions in the record).

## Locked Assumptions and Invariants

- wasmtime target is exactly `47.0.3`; wit-bindgen target is exactly `0.60.0`; `call-hook` feature retained.
- MoonBit verdict = `not re-run (toolchain absent)`; Go verdict is gate-deciding.
- The slicer-only linker in the Go probe is byte-for-byte pnp_cli's Layer::Infill linker (no WASI); no WASI is added to the probe to make it "pass".
- The gate verdict is one of exactly two strings (see requirements §In Scope).

## Risks and Tradeoffs

- wasmtime 47 may rename/deprecate a config or component-model API used pervasively in `host.rs`/`dispatch.rs`; the fallout could exceed the bounded read. Mitigation: the compile gate enumerates it, and each fix is mechanical; if any single step's fallout balloons to L, the packet is split before activation (context-cost rule).
- Re-running the Go probe costs a full scratch build (~minutes) and a wit-bindgen-cli 0.60.0 `--features go` install; this is a machine-local step, not committed to the repo. The scratch artifacts live in `$COMMANDCODE_SCRATCHPAD` and are never written into the tree.
- If the Go verdict still fails, the fallback is confirmed and packet 227 must switch to a Rust `#[slicer_module]` authoring plan; that is a downstream planning change, not a defect in this packet.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 4, toolchain fallout absorption — bounded by a LOCATIONS dispatch, not a full read)
- Highest-risk dispatch and required return format: the `LOCATIONS` grep over `host.rs`/`dispatch.rs`/`instance.rs`/`run.rs` (Step 4) — return `LOCATIONS` with line numbers; a SUMMARY there would fail to bound the read.

## Open Questions

- [FWD] Whether wasmtime 47.0.3 changed `CallHook`, `get_fuel`/`set_fuel`, `ResourceLimiter`, or `Store::new` signatures in a way that ripples beyond the four in-scope files. Resolve at activation via `cargo check --workspace --all-targets`; the fix surface stays within the wasmtime-API files named above.
- [FWD] Whether the Go 1.26.5 wasip1 runtime still emits the same WASI preview2 import set after `wasm-tools component new --adapt`. The probe measures it; the recorded `RESULT:` is authoritative either way.

None is an activation blocker; the packet's own gate outcome is the measurement this packet exists to produce.

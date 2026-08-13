# Design: 225a-host-wasi-accomodation

## Controlling Code Paths

- Primary code path: `crates/slicer-wasm-host/src/dispatch.rs` production component-linker construction, especially text postprocess at lines 1721-1753.
- Neighboring tests/fixtures: `foreign_language_feasibility_tdd.rs`; `docs/feasibility-probes/foreign-language-text-postprocess/`; the four candidate records.

## Architecture Constraints

- Register the full wasmtime-wasi preview2 surface using `wasmtime_wasi::p2::add_to_linker_sync::<T: WasiView>`; default-deny `WasiCtx::builder()` must not call capability-granting methods.
- `HostExecutionContext` and `HostState` must each own a `WasiCtx` and implement `WasiView`, so runtime and direct instantiation use equivalent store capabilities.
- Apply the shared linker helper `add_wasi_to_linker(&mut linker)` at all 15 production linker sites; text postprocess is the acceptance anchor, not the only affected path.
- WASM-staleness snippet intentionally omitted: this host-only linker/state change does not feed guest WASM builds, and the probes build foreign components in scratch.
- No Orca delegation and no coordinate-system constraints apply.

## Code Change Surface

- Selected approach: add workspace/crate dependency, embed default-deny WASI state, and centralize preview2 registration in a helper invoked immediately after every production linker construction.
- Exact functions, traits, manifests, tests, and fixtures: workspace dependencies; `HostExecutionContext`, `HostExecutionContextBuilder`, `HostState`, their `WasiView` implementations; dispatch linker helper/call sites; the independent oracle test; four evidence records; docs/14 verdict.
- Rejected alternatives and reasons: a slicer-only linker incorrectly treats mandatory-WASI toolchains as language failures; per-site ad hoc registrations invite missed stages; capability inheritance violates ADR-0060's preserved sandbox.

## Files in Scope (read + edit)

- `Cargo.toml` - role: workspace pin; expected change: add `wasmtime-wasi` at `47.0.x`.
- `crates/slicer-wasm-host/Cargo.toml` - role: crate dependency; expected change: consume workspace `wasmtime-wasi`.
- `crates/slicer-wasm-host/src/host.rs` - role: production execution context; expected change: own default-deny `WasiCtx` and implement `WasiView`.
- `crates/slicer-wasm-host/src/instance.rs` - role: direct instance state; expected change: own default-deny `WasiCtx` and implement `WasiView`.
- `crates/slicer-wasm-host/src/dispatch.rs` - role: all production linkers; expected change: shared WASI-linker helper at all construction sites.
- `crates/slicer-wasm-host/tests/integration/foreign_language_feasibility_tdd.rs` - role: independent oracle; expected change: WASI-enabled linker/store.
- `docs/feasibility-probes/{moonbit,assemblyscript,cpp,go}-text-postprocess.md` - role: terminal re-measurement evidence; expected change: truthful records.
- `docs/14_submodule_programming_languages.md` - role: final selection; expected change: four-result summary and one verdict line.

## Read-Only Context

- `crates/slicer-wasm-host/src/dispatch.rs` - lines `422-430`, `500-508`, `576-584`, `649-657`, `722-730`, `789-797`, `867-875`, `934-942`, `1098-1106`, `1144-1152`, `1195-1203`, `1268-1276`, `1394-1402`, `1540-1548`, `1721-1753` only - linker inventory.
- `crates/slicer-wasm-host/src/host.rs` - lines `1134-1400` only - execution context and builder.
- `crates/slicer-wasm-host/src/instance.rs` - lines `96-220` only - direct state and engine configuration.
- `docs/adr/0060-host-wasi-accommodation-for-foreign-language-guests.md` - lines `1-16` - accepted decision.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` - no parity obligation; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load directly.
- Guest source/build paths and unrelated crates - do not edit or browse beyond delegated symbol lookups.

## Expected Sub-Agent Dispatches

- Question: enumerate every `HostExecutionContext` and `HostState` struct literal and constructor affected by adding `WasiCtx`; scope: `crates/slicer-wasm-host/**/*.rs`; return: `LOCATIONS` only; purpose: Step 1 blast-radius budget.
- Question: confirm all production `Linker::<HostExecutionContext>::new` sites invoke the new helper; scope: `crates/slicer-wasm-host/src/dispatch.rs`; return: `FACT` plus line list; purpose: Step 1.
- Question: run each prerequisite/fork gate and oracle command without changing sources; scope: probe fixture and command environment; return: `FACT` plus <=20 failure lines; purpose: Steps 3-6.

## Data and Contract Notes

- IR/manifest contracts: unchanged.
- WIT boundary: world remains `slicer:postpass-text-postprocess/text-postprocess-module`; WASI only satisfies foreign component imports alongside existing generated bindings.
- Determinism/scheduler constraints: no ambient host input reaches guests; no preopens, env, args, network, or inherited stdio; the oracle input/output remains exact.

## Locked Assumptions and Invariants

- Use released `wit-bindgen-cli 0.60.0` for MoonBit, C++, and Go. Use only the confirmed, clean AssemblyScript fork and capture its HEAD immediately before generation.
- Every record has a real component SHA-256 and terminal result; a tooling blocker stops the candidate rather than producing a candidate result.
- The selection is the first `LOADABLE_AND_CORRECT` record in MoonBit, AssemblyScript, C++, Go order, otherwise Rust, and docs/14 contains exactly one formatted verdict line.

## Risks and Tradeoffs

- Full preview2 linking broadens imports accepted but default-deny state preserves the host capability boundary.
- MoonBit may trap or corrupt strings due to UTF-16/UTF-8 behavior; retain full diagnostics rather than normalize failure.
- Toolchain/fork availability can block the packet; blockers are explicit and must not be misreported as a negative language verdict.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: Step 1 struct-literal/linker inventory, `LOCATIONS` only.

## Open Questions

None.

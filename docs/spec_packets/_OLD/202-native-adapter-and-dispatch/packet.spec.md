---
status: implemented
packet: 202-native-adapter-and-dispatch
task_ids:
  - ADR-0056
backlog_source: docs/specs/multi-edition-distribution-plan.md
context_cost_estimate: M
---

# Packet Contract: 202-native-adapter-and-dispatch

## Goal

Give `#[slicer_module]` a native adapter emitting the same stage contract natively that its wit-guest shim emits for wasm32, and route integrated-provenance modules to a direct native call behind the ADR-0005 runner-trait seam, leaving a testable dual-path parity seam for packet 204 (ADR-0056 Decision items 3–5).

## Scope Boundaries

This packet touches `crates/slicer-macros/src/lib.rs` (native adapter emission), a new `slicer_sdk::native` module, a new `crates/slicer-wasm-host/src/marshal/native.rs`, the four runner impls in `crates/slicer-wasm-host/src/dispatch.rs`, `binding.rs`/`execution_plan_live.rs` native-entry plumbing, and `slicer-integrated-modules`' native-entry table. Pilot-module integration and the parity *gate* are packet 204; the CLI surface is 203; wasm-less builds (ADR-0056 Decision item 6) are out entirely.

## Prerequisites and Blockers

- Depends on: `201-integrated-module-registry-tier5` (draft at authoring — FORWARD-DEP: `ModuleProvenance`, `IntegratedModuleRegistration`, `load_live_modules_for_plan_with_integrated`, crate `slicer-integrated-modules`; names/shapes reconciled against 201's spec, authored in the same batch).
- Unblocks: 203 (CLI/provenance), 204 (pilot/parity), 205 (editions).
- Activation blockers: 201 must be implemented first (this packet extends `load_live_modules_for_plan_with_integrated`'s signature and consumes `ModuleProvenance`).

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** a module crate whose stage impl carries `#[slicer_module]` (e.g. `SdkLayerInfillModule` in `crates/slicer-wasm-host/test-guests/sdk-layer-infill-guest/src/lib.rs`), **when** it is compiled for a non-wasm32 target, **then** the type exposes `__slicer_native_entry()` returning a `slicer_sdk::native::NativeStageEntry` whose family variant matches the implemented SDK trait (`NativeStageEntry::Layer(..)` for `LayerModule`). | `cargo test -p slicer-runtime --test contract native_entry_layer_family`
- **AC-2. Given** identical `LayerStageInput` fixtures and config, **when** `sdk-layer-infill-guest` is dispatched once over the WASM path (`native_entry: None`, `wasm_component: Some(..)`) and once over the native path (`native_entry: Some(SdkLayerInfillModule::__slicer_native_entry())`, `wasm_component: None`) through the same `WasmRuntimeDispatcher`, **then** both return `Ok(Some(LayerStageCommit))` and the commits agree structurally: same region count, same sparse-path count per region, same per-path point counts and `ExtrusionRole`s (byte-equality of floats is NOT asserted — ADR-0056 Decision item 4 / DEV-093). | `cargo test -p slicer-runtime --test contract native_dispatch_parity_seam`
- **AC-3. Given** a `CompiledModuleLive` with `native_entry: Some(..)` and `wasm_component: None`, **when** `LayerStageRunner::run_stage` runs, **then** dispatch succeeds without any WASM instantiation (no pool lease, no `DispatchPhase::MissingComponent` error) — proving the native branch precedes component resolution. | `cargo test -p slicer-runtime --test contract native_dispatch_without_component`
- **AC-4. Given** an integrated-provenance module surviving dedup and a native-entry table containing its `module.id`, **when** `load_live_modules_for_plan_with_integrated` builds bindings, **then** that module's `LiveModuleBinding` has `native_entry: Some(..)` and `wasm_component: None`. | `cargo test -p slicer-runtime --test integration integrated_binding_attaches_native_entry`
- **AC-5. Given** the single-source SDK contract, **when** `crates/slicer-sdk/Cargo.toml` is inspected, **then** `slicer-core` with `features = ["host-algos"]` remains gated under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` (the invariant that lets module code call `slicer-core` directly when native — ADR-0033 layer 3). | `rg -q 'cfg\(not\(target_arch = "wasm32"\)\)' crates/slicer-sdk/Cargo.toml && rg -q 'host-algos' crates/slicer-sdk/Cargo.toml`

## Negative Test Cases

- **AC-N1. Given** module id `X` present in the native-entry table but whose surviving `LoadedModule` has `provenance() == ModuleProvenance::External` (an external copy won first-root-wins dedup), **when** `load_live_modules_for_plan_with_integrated` builds bindings, **then** `X`'s binding has `native_entry: None` and a compiled `wasm_component: Some(..)` — the externally-overridden integrated module dispatches over the WASM path. | `cargo test -p slicer-runtime --test integration external_override_forces_wasm_dispatch`
- **AC-N2. Given** an integrated-provenance module with **no** native-entry table match, **when** it is dispatched, **then** the result is the existing loud `DispatchPhase::MissingComponent` failure — never a silent native call and never a silent skip. | `cargo test -p slicer-runtime --test integration integrated_without_native_entry_fails_loud`
- **AC-N3 (doc grep).** Given the doc edits below, both greps pass. | `rg -q 'native_entry' docs/04_host_scheduler.md && rg -q '__slicer_native_entry' docs/05_module_sdk.md`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (macro + SDK edits invalidate every guest; must report clean after rebuild)

## Authoritative Docs

- `docs/adr/0056-integrated-modules-native-dispatch.md` — short; direct read; Decision items 3–5 are this packet's contract.
- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — short; direct read; the seam the routing hides behind.
- `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` — short; direct read; layer-3 cfg-split wrappers.
- `docs/05_module_sdk.md` — delegate; only the `#[slicer_module]` section is edited.
- `docs/04_host_scheduler.md` — delegate; only §Phase 4 Execution is edited.

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/04_host_scheduler.md` §"Phase 4 — Execution": paragraph on provenance-routed dispatch — `CompiledModuleLive.native_entry` decides native call vs WASM instantiation; marshalling boundary shared; single-threaded module logic on both paths. — `rg -q 'native_entry' docs/04_host_scheduler.md`
- `docs/05_module_sdk.md` `#[slicer_module]` section: document the emitted `__slicer_native_entry()` adapter and the single-source dual-target module model. — `rg -q '__slicer_native_entry' docs/05_module_sdk.md`
- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md`: append an `## Amendment — <date> (packet 202)` section quoting the ADR's normative `## Decision` bullet that pins `CompiledModuleLive` to 5 fields, and record the 6th (`native_entry`). Adding the field contradicts an ADR Decision clause, so it needs its own record — see `design.md` §Files explicitly out of bounds. — `rg -q '^## Amendment' docs/adr/0005-runner-traits-in-slicer-wasm-host.md`
- `docs/DEVIATION_LOG.md`: one new row `D-202-ADR-0005-AMENDED` (**re-derive the free `D-` number when writing it** — ledger fact). — `rg -q 'ADR-0005-AMENDED' docs/DEVIATION_LOG.md`

Doc greps are appended to the ACs as AC-N3.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

# Requirements: 202-native-adapter-and-dispatch

## Packet Metadata

- Grouped task IDs: `ADR-0056` (§3–5). No `docs/07_implementation_status.md` TASK rows exist for this program — see `docs/specs/multi-edition-distribution-plan.md` §"Backlog anchoring [FWD]"; do not add rows to docs/07 in this packet.
- Backlog source: `docs/specs/multi-edition-distribution-plan.md`, Packet Queue row 3 (depends on row 2 / packet 201).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

After packet 201, an integrated module loads, claims, and schedules like any module but cannot execute: its `LiveModuleBinding` carries `wasm_component: None` and dispatch dies at `DispatchPhase::MissingComponent`. Production dispatch today is solely `WasmRuntimeDispatcher` (`crates/slicer-wasm-host/src/dispatch.rs`) implementing the four runner traits in `crates/slicer-wasm-host/src/traits.rs` (`LayerStageRunner`, `PrepassStageRunner`, `PostpassStageRunner`, `FinalizationStageRunner`); per dispatch it resolves the stage export, leases a pool slot, builds a per-call `HostExecutionContext`, instantiates typed bindings, calls, and releases. ADR-0056 §3 requires provenance to decide dispatch — direct native call vs WASM instantiation — behind that same seam, with `#[slicer_module]` (`crates/slicer-macros/src/lib.rs`) emitting the native adapter from the same single-source module crate. Module bodies are already written against SDK traits (`LayerModule` etc., `crates/slicer-sdk/src/traits.rs`) with plain-Rust SDK views/builders (`crates/slicer-sdk/src/views.rs`, `builders.rs`), which is what makes a native adapter feasible without touching any module body.

## In Scope

- `slicer_sdk::native` (new module `crates/slicer-sdk/src/native.rs`, `#[cfg(not(target_arch = "wasm32"))]`): `NativeStageEntry` enum with one variant per runner-trait family (`Layer`, `Prepass`, `Postpass`, `Finalization`), each wrapping a fn pointer over per-family request/response envelopes (`NativeLayerRequest`/`NativeLayerResponse` and prepass/postpass/finalization counterparts) built from SDK view/builder-output types.
- `#[slicer_module]` macro: for the detected stage, additionally emit a `#[cfg(not(target_arch = "wasm32"))]` inherent `pub fn __slicer_native_entry() -> ::slicer_sdk::native::NativeStageEntry` whose body is `from_config` + the trait stage method + SDK-builder drain — generated from the same `slicer_schema::STAGES` table (`StageSpec`) that drives the wasm32 glue, so the two adapters cannot drift on stage identity. All four families covered.
- `crates/slicer-wasm-host/src/marshal/native.rs` (new, inside the marshalling boundary module): per-family request builders (IR/`*StageInput` → SDK views) and response committers (SDK builder output → the existing `*OutputCollected` accumulators → existing `out.rs` converters + `origin.rs` `OriginBucket` re-attribution → IR-typed runner outputs).
- Dispatch routing: each of the four `impl *StageRunner for WasmRuntimeDispatcher` blocks gains a native branch at entry — `if let Some(entry) = module.native_entry { … }` — before export resolution/pool lease; `CompiledModuleLive` gains `native_entry: Option<NativeStageEntry>` defaulting to `None` inside `CompiledModuleLive::new` (no caller churn) plus a `with_native_entry` setter.
- Plumbing: `LiveModuleBinding` gains `native_entry: Option<NativeStageEntry>`; `load_live_modules_for_plan_with_integrated` (from 201) gains a `native_entries: &[(ModuleId, NativeStageEntry)]` parameter and attaches an entry **iff** the surviving module's `provenance()` is `Integrated` and the table contains its id; `crates/slicer-runtime/src/run.rs` passes `slicer_integrated_modules::native_entries()`.
- `slicer-integrated-modules`: `pub fn native_entries() -> Vec<(ModuleId, NativeStageEntry)>`, per-module feature arms (empty by default; populated per-module by 204/205).
- The parity seam (an export of this packet): the dual-path dispatch pattern of AC-2 — same dispatcher, two `CompiledModuleLive` values (native vs wasm) on identical inputs — demonstrated by `native_dispatch_parity_seam_tdd.rs`, with `sdk-layer-infill-guest` compiled natively via a dev-dependency. 204 reuses this pattern per pilot module.
- Doc edits per `packet.spec.md` §Doc Impact.

## Out of Scope

- Pilot integration of `classic-perimeters`/`arachne-perimeters`/`support-planner`, tolerance-based IR comparison gates, ADR-0042 structural-invariant assertions beyond AC-2's counts, deviation filing for residual divergence (packet 204).
- `--no-integrated-modules`, provenance in `pnp_cli module` output (203); editions/xtask/CI (205); wasm-less builds where wasmtime is compiled out (ADR-0056 §6 — deferred phase 4).
- Native capture parity for the runner traits' `last_*`/`take_*` instrumentation accessors (profiling marks, fuel, batch calls): the native branch returns empty captures in this packet; see design.md [FWD].
- Any change to module crate bodies (`modules/core-modules/*/src/**` stage logic), to WIT files, to the wasm32 glue's behavior, or to `slicer-sdk/src/host.rs` wrappers (the cfg-split native arms already exist per ADR-0033; wasm32 bridge arms are packet 200 / DEV-094).
- Internal parallelism in module logic — forbidden on both paths (ADR-0056 §5); no rayon in adapters.

## Authoritative Docs

- `docs/adr/0056-integrated-modules-native-dispatch.md` — short; direct read (§3–5).
- `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` — direct read; IR-typed seam invariants that the native branch must respect (no `HostExecutionContext` across the trait boundary — trivially satisfied: the native path never builds one).
- `docs/adr/0033-host-service-bridge-for-host-only-algorithms.md` — direct read; layer-3 cfg-split SDK wrappers.
- `docs/03_wit_and_manifest.md` — delegate; WIT untouched, consult only if glue questions arise.
- `docs/05_module_sdk.md`, `docs/04_host_scheduler.md` — delegate; edited sections only.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-5`. Refinement to AC-2: the fixture input must exercise ≥1 region with non-empty polygons so the guest emits ≥1 sparse path (an empty-input "parity" pass proves nothing).
- Negative: `AC-N1` through `AC-N3`.
- Cross-packet impact: 204 consumes the parity-seam pattern, `__slicer_native_entry` on the three pilot modules, and `slicer_integrated_modules::native_entries()`; 203 surfaces `native_entry`-derived provenance in listings; 205 flips per-module features. 201's `load_live_modules_for_plan_with_integrated` signature changes here — 201's design notes this planned extension.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test contract native_entry_layer_family` | AC-1 | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-runtime --test contract native_dispatch_parity_seam` | AC-2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract native_dispatch_without_component` | AC-3 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration integrated_binding_attaches_native_entry` | AC-4 | FACT pass/fail |
| `rg -q 'cfg\(not\(target_arch = "wasm32"\)\)' crates/slicer-sdk/Cargo.toml && rg -q 'host-algos' crates/slicer-sdk/Cargo.toml` | AC-5 | FACT pass/fail (exit code) |
| `cargo test -p slicer-runtime --test integration external_override_forces_wasm_dispatch` | AC-N1 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration integrated_without_native_entry_fails_loud` | AC-N2 | FACT pass/fail |
| `rg -q 'native_entry' docs/04_host_scheduler.md && rg -q '__slicer_native_entry' docs/05_module_sdk.md` | AC-N3 doc greps | FACT pass/fail |
| `cargo xtask build-guests --check` | guest freshness after macro/SDK edits | FACT clean/STALE |
| `cargo test -p slicer-runtime --test contract` | macro/dispatch regression (includes `macro_all_worlds_roundtrip_tdd`) | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration live_module_loading` | live-loader regression | FACT pass/fail |
| `cargo check --workspace --all-targets` | native adapter compiles in all 21 workspace module crates | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |

## Step Completion Expectations

- SDK types (Step 1) precede macro emission (Steps 2–3); marshal/native (Step 4) precedes dispatch branches (Step 5); dispatch precedes the parity seam tests (Step 6); plumbing (Step 7) precedes AC-4/AC-N1/AC-N2.
- After ANY edit to `crates/slicer-macros/**` or `crates/slicer-sdk/**`, the next test run against guest artifacts must be preceded by `cargo xtask build-guests --check` (rebuild on `STALE:`) — the parity test (Step 6) loads `sdk-layer-infill-guest.component.wasm`, which goes stale on every macro edit.
- `cargo test` output must tee to `target/test-output.log` per `CLAUDE.md`; read the log, never re-run for output.

## Context Discipline Notes

- `crates/slicer-macros/src/lib.rs` and `crates/slicer-wasm-host/src/dispatch.rs` are both several times the 600-line direct-read cap — ranged reads only, located by symbol grep first (`generate_slicer_module_impl`, the per-world glue emitters, the four `impl *StageRunner` blocks). Never load either whole.
- Do not request macro expansions; trust `cargo check --workspace --all-targets` (it compiles the emitted adapter natively in every workspace module crate) and the AC tests.
- `crates/slicer-sdk/src/traits.rs` (long) — read only the signature block of the one trait per family being adapted.
- Heavy-dispatch return limits: per-family glue-shape surveys return SNIPPETS ≤30 lines each; reject larger.

---
status: implemented
packet: 202-native-adapter-and-dispatch
task_ids:
  - ADR-0056
---

# 202-native-adapter-and-dispatch

## Goal

Give `#[slicer_module]` a native adapter emitting the same stage contract natively that its wit-guest shim emits for wasm32, and route integrated-provenance modules to a direct native call behind the ADR-0005 runner-trait seam, leaving a testable dual-path parity seam for packet 204 (ADR-0056 Decision items 3–5).

## Problem Statement

After packet 201, an integrated module loads, claims, and schedules like any module but cannot execute: its `LiveModuleBinding` carries `wasm_component: None` and dispatch dies at `DispatchPhase::MissingComponent`. Production dispatch today is solely `WasmRuntimeDispatcher` (`crates/slicer-wasm-host/src/dispatch.rs`) implementing the four runner traits in `crates/slicer-wasm-host/src/traits.rs` (`LayerStageRunner`, `PrepassStageRunner`, `PostpassStageRunner`, `FinalizationStageRunner`); per dispatch it resolves the stage export, leases a pool slot, builds a per-call `HostExecutionContext`, instantiates typed bindings, calls, and releases. ADR-0056 Decision item 3 requires provenance to decide dispatch — direct native call vs WASM instantiation — behind that same seam, with `#[slicer_module]` (`crates/slicer-macros/src/lib.rs`) emitting the native adapter from the same single-source module crate. Module bodies are already written against SDK traits (`LayerModule` etc., `crates/slicer-sdk/src/traits.rs`) with plain-Rust SDK views/builders (`crates/slicer-sdk/src/views.rs`, `builders.rs`), which is what makes a native adapter feasible without touching any module body.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- **Marshalling boundary — ONE answer.** CONTEXT.md §Marshalling boundary states the one-answer rule but does *not* name a directory; the path pin below is this packet's, read off the tree: the boundary stays `crates/slicer-wasm-host/src/marshal/`. The native path is a second *transport* through the same boundary, not a second boundary: response commit re-enters at the `*OutputCollected` accumulator layer (`marshal/accumulators.rs`) so the `out.rs` converters (`convert_infill_output`, `convert_perimeter_output`, …) and `origin.rs` `OriginBucket` re-attribution run **unchanged and shared** between transports. The only dual leg is view construction (wasm: resource methods + guest glue; native: `marshal/native.rs` builds `slicer_sdk::views::*` directly from IR) — that leg is exactly what packet 204's parity gate audits. Parity implication stated explicitly: divergence between paths can originate only in (a) the input-view leg field mapping and (b) module-body libm/codegen ULP drift; origin re-attribution and output conversion cannot diverge because they are the same code.
- ADR-0005 seam invariants hold: the native branch lives *inside* the runner impls; no `HostExecutionContext` (never constructed on the native path), no wasm-host-internal type crosses the trait boundary; outputs remain IR-typed (`LayerStageCommit`, `PrepassStageOutput`, `PostpassOutput`, `FinalizationOutput`). Note the asymmetry, verified at authoring: `PrepassStageOutput` is defined in **`crates/slicer-core/src/stage_io.rs`**, not `slicer-ir` like its three peers — import it from the right crate in Step 4.
- ADR-0056 Decision item 5: single-threaded module logic on both paths — the native adapter and `marshal/native.rs` must not introduce rayon or thread spawns; host-side layer fan-out and batched host services stay the only parallelism.
- ADR-0033 layer 3 already gives SDK host-service wrappers native arms (`crates/slicer-sdk/src/host.rs` cfg-split; `slicer-sdk/Cargo.toml` enables `host-algos` only under `cfg(not(target_arch = "wasm32"))` — verified at authoring). Module code calling `slicer_sdk::host::*` therefore works natively today; this packet adds no wrapper changes (wasm32 bridge arms are DEV-094 / packet 200).
- Config crosses the native seam as the literal shared struct: `CompiledModuleLive.config_view: Arc<ConfigView>` is handed to `from_config` directly (`ConfigView` is the same `slicer_ir` type on both sides). Config keys stay snake_case.

## Data and Contract Notes

- IR/manifest contracts: none change. The native response's origin pairs `(object_id, region_id)` feed the same `OriginBucket` drain the WIT path uses — origin-based identity reconstruction has one owner.
- WIT boundary: untouched (no new WIT funcs, no bindgen changes; the `bindgen!` worlds in `crates/slicer-wasm-host/src/host.rs` are not edited — there are 15 of them, one per stage world, not one per stage family).
- Determinism/scheduler constraints: the native branch must preserve per-call ordering semantics — one module invocation per dispatch, same commit shape; instance-pool bookkeeping is skipped (a native call has no slot), which must not affect layer fan-out (pool concurrency limits WASM instances, not native calls; ADR-0056 Decision item 5 keeps module logic single-threaded so a concurrent native fan-out executes the same pure logic — flag any discovered shared-state hazard as a blocker rather than adding a lock silently).
- FORWARD-DEP contract with 201 (both drafts authored together; **201 landed 2026-08-10** — the signature below was verified against the landed tree): `ModuleProvenance` (`slicer-scheduler`), `IntegratedModuleRegistration`, `load_live_modules_for_plan_with_integrated(search_roots, host_parallelism, config_source, profile, integrated)` — this packet appends the `native_entries` parameter; 201's design.md §Risks records that extension. 201 landed with exactly two callers of the entry point (both in `crates/slicer-runtime/src/run.rs`), so the signature extension churns no other call site.

## Locked Assumptions and Invariants

- SDK builders (`crates/slicer-sdk/src/builders.rs` and prepass/postpass counterparts) are target-independent accumulators with public read accessors (verified at authoring) — the native adapter's drain relies on this; builders must stay cfg-free.
- `NativeStageEntry` values are `'static` (fn pointers + static strs), letting `CompiledModuleLive` carry them by value without lifetime plumbing.
- An integrated module with no native entry stays a loud `MissingComponent` failure (AC-N2) — never a silent skip; this preserves 201's seam semantics for misconfigured builds.
- Byte-equality across paths is explicitly NOT asserted anywhere in this packet (ADR-0056 Decision item 4, DEV-093); AC-2's structural assertions are the ceiling until 204's tolerance gate.

## Risks and Tradeoffs

- Feature-unification hazard (inherited from 201's landed registry crate): no workspace member may enable a `slicer-integrated-modules` feature in normal/dev/test profiles, or every `--workspace` build silently grows an integrated tier. 202's own tests must construct native-entry tables inline (as 201's loader tests construct `IntegratedModuleRegistration` values inline) — never via a dev-dependency feature. The only sanctioned feature consumers are 203's `pnp-cli` passthrough (`integrated-classic-perimeters`) and 205's edition features, both explicit and opt-in.
- Envelope drift risk: `marshal/native.rs` view construction vs `in_.rs` resource backing is the audited dual leg; a missed field surfaces as a 204 parity failure, not a compile error. Mitigation: `adapt_slice_regions_completeness_tdd.rs` (existing) pins the wasm leg's field completeness; Step 4 adds the mirrored field checklist against the same list.
- Macro emission compiles natively in all 21 workspace module crates at once — a signature mistake breaks `cargo check --workspace --all-targets` broadly; Step 2 gates on exactly that command before proceeding.
- Instrumentation asymmetry: native calls return empty `last_*` captures this packet; consumers (`ModuleAccessAudit`, profiling) see zeros for native dispatches — acceptable and documented, revisit in 204 if the audit needs native reads.
- Heavy layer stages (`perimeters_postprocess`, `infill_postprocess`, `path_optimization`) have the richest envelopes; they are in scope but sized as their own step to keep failure blast radius contained.

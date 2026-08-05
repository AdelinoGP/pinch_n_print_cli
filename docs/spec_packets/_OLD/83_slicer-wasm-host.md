---
status: implemented
packet: 83
task_ids: [TASK-233]
---

# 83_slicer-wasm-host

## Goal

Move `wit_host.rs` (5 259 LOC), `dispatch.rs` (3 148 LOC), `wasm_instance.rs` (299 LOC), and `instance_pool.rs` (182 LOC) — together with the four runner trait definitions (`LayerStageRunner`, `PrepassStageRunner`, `FinalizationStageRunner`, `PostpassStageRunner`) — out of `slicer-runtime/src/` into a new `slicer-wasm-host` crate; collapse `dispatch::export_name_for_stage` into a `slicer_schema::export_for_stage_id(&str) -> Option<&'static str>` lookup that reads from `STAGES.wit_export` (the documented single source of truth); and introduce `CompiledModuleLive<'s>` (a borrow holding `Arc<WasmComponent>` + `Arc<WasmInstancePool>` + a `&'s CompiledModuleStatic`) so the runner traits never take wasmtime types from `slicer-runtime` — making `slicer-runtime` a `wasmtime`-free crate transitively (wasmtime is reachable only via the new `slicer-wasm-host` dep).

## Problem Statement

`slicer-runtime` today carries 8 900 LOC of WIT/wasm-component-model marshalling that has nothing to do with orchestration. `wit_host.rs` (5 259 LOC) is four `wasmtime::component::bindgen!` invocations plus the 16 host-trait impls that bridge generated WIT shapes to host data. `dispatch.rs` (3 148 LOC) is the `WasmRuntimeDispatcher` struct that implements four runner traits and routes stage IDs to the matching WIT export. `wasm_instance.rs` and `instance_pool.rs` wrap the `wasmtime::Engine`/`wasmtime::Component`/instance-pool plumbing. Five structural consequences hurt:

1. **`slicer-runtime` directly depends on `wasmtime`.** Any future crate that consumes the runtime pays for wasmtime even if it only wants the scheduler types — and `slicer-scheduler` (packet 85) cannot be a wasmtime-free crate while `CompiledModule` exposes `Arc<WasmComponent>` accessors.
2. **The dispatcher trait impls and the executor live in the same crate**, so unit tests for orchestration logic cannot mock the dispatcher cheaply.
3. **The stage→WIT-export name lookup is duplicated.** `dispatch.rs::export_name_for_stage` (lines 47–67) and `slicer-schema::STAGES[*].wit_export` carry the same data; the schema is the documented single source of truth (per its own docstring) but the dispatcher re-hardcodes it.
4. **The runner trait defs sit in the executor modules** (`layer_executor.rs`, `prepass.rs`, etc.), entangling "what the executor needs from a runner" with "what the executor does with the runner's output". Moving the impls out is impossible without first lifting the trait defs.
5. **`CompiledModule` exposes `pub fn instance_pool() → &Arc<WasmInstancePool>` and `pub fn wasm_component() → Option<&Arc<WasmComponent>>` accessors** (`execution_plan.rs:688, 713`). These bake wasmtime into the planning type, blocking C's clean extraction in P85.

The fix is one move, one consolidation, one split. Move `wit_host.rs` + `dispatch.rs` + `wasm_instance.rs` + `instance_pool.rs` + the four runner trait defs into `slicer-wasm-host`. Collapse `export_name_for_stage` into `slicer-schema::export_for_stage_id`. Split `CompiledModule` into `CompiledModuleStatic` (manifest-resolution shape, will move to `slicer-scheduler` in P85) and `CompiledModuleLive<'s>` (the wasmtime borrow that the runner traits consume).

## Architecture Constraints

- **ADR-0002 preserved**: all four `bindgen!` invocations in one crate (`slicer-wasm-host`), with the layer world canonical and prepass/finalization/postpass using `with: { "slicer:types/geometry": super::layer::slicer::types::geometry, "slicer:config/config-types": super::layer::slicer::config::config_types }`. The `pub mod layer` declaration must precede the other three modules so the `super::layer::…` paths resolve.
- **ADR-0003 preserved**: guest-side WIT conversions remain per-world inside `#[slicer_module]` (untouched by this packet).
- **No new ADR conflicts**: this packet's two ADR follow-ups (ADR-0004, ADR-0005) are recorded at close; no existing ADR is contradicted.
- `slicer-runtime` MUST NOT regain a direct `wasmtime` dep after this packet. The build will compile without it; if any source file imports `use wasmtime::...` after the move, the move missed a site.
- **Borrow-struct pattern for trait inputs.** Runner trait signatures use `*StageInput<'_>` borrow structs defined in `slicer-wasm-host`, **not** raw `&Blackboard` / `&mut LayerArena`. This is the same pattern P87 uses for `RegionMappingPlanProjection<'a>` and is **NOT a new abstraction** — it is the established mechanism in this batch for decoupling a consumer crate from runtime-owned aggregate types. The four borrow structs (`LayerStageInput<'a>`, `PrepassStageInput<'a>`, `FinalizationInput<'a>`, `PostpassInput<'a>`) carry only the field-level borrows the dispatcher actually reads/writes; `Blackboard` and `LayerArena` themselves stay in `slicer-runtime` (unchanged). The orchestrator constructs the input struct at the call site in each executor file. Stage I/O types (`*StageOutput`, `*Error`) relocate to `slicer-ir` and `slicer-core` in Step 0.5 prework (three groups, see implementation-plan §Step 0.5) so the trait signatures compile from inside `slicer-wasm-host` with no back-edge dep on `slicer-runtime` (AC-N3).
- **Narrow runner errors.** Runner trait signatures return narrow error types (`PrepassRunnerError`) defined in `slicer-ir`, **NOT** the broader orchestrator-level error types (`PrepassExecutionError`). Broad errors stay in `slicer-runtime` with `From<NarrowError>` impls for conversion at the orchestrator call site (so the existing `?` operator handles the lift transparently). This matches **P86's `GCodeEmitError → PostpassError` pattern** and is the established batch idiom — **not a new abstraction**. The narrow type carries only the variants the wasm dispatcher actually constructs (`FatalModule`, `Blackboard`); the 7 built-in-producer variants of `PrepassExecutionError` (MeshAnalysis, RegionMapping, SupportGeometry, PaintSegmentation, Slice, ShellClassification, MissingRequiredPrepass) stay attached to their constructors in `slicer-runtime`.
- **Symmetric IR-typed trait boundary.** Runner traits use IR-typed inputs (`*StageInput<'_>` borrow structs) **AND** IR-typed outputs (`LayerStageCommitData`-style structs defined in `slicer-ir`). **Wasm-host-internal types** (`HostExecutionContext`, instance pool handles, bindgen-coupled state) **do NOT cross the trait boundary in either direction.** Deconstruction of `HostExecutionContext` into the IR-typed commit struct happens inside the runner trait impl on the wasm-host side **before** return. This makes the post-Step-4 orchestration shape symmetric: `slicer-runtime/src/layer_executor.rs` reads `&LayerArena` to build `LayerStageInput<'_>` pre-call; `slicer-runtime/src/layer_executor.rs` writes `&mut LayerArena` by consuming `LayerStageCommitData` post-call; `slicer-wasm-host`'s `WasmRuntimeDispatcher::run_stage` impl does ONLY the wasmtime call + the inline deconstruction in between. The Category-A bindgen-`Host` trait remains the in-wasmtime-call seam for any per-call host imports (no `LayerArena` access from inside it, since all 18 surveyed `arena.*` accessors classified as Category B).

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Data and Contract Notes

- `WasmComponent`, `WasmInstance`, `WasmInstancePool`, `WasmEngine`, `WasmLoadError`, `WasmCallError`, `InstancePoolError`, `InstancePoolMode`, `WasmInstanceLease`, `WasmArtifactMetadata` — public types from `wasm_instance.rs` and `instance_pool.rs`. Their shapes are preserved exactly; only their crate location changes.
- `HostExecutionContext`, `HostExecutionContextBuilder`, `ConfigViewData`, `PaintRegionLayerData`, and the ~20 `*BuilderData` / `*Collected` resource backing types from `wit_host.rs` — preserved exactly.
- The 16 host-trait impls on `HostExecutionContext` (one per WIT-interface-Host pair) — preserved exactly. The bodies do not change; only the file location.
- `CompiledModuleLive<'s>` is **non-owning**: holds an `&'s CompiledModuleStatic` borrow and `Arc<...>` for the wasm bindings. Its lifetime is per-dispatch-call; constructed on the runtime side, passed by `&` to the runner trait method.
- `slicer_schema::export_for_stage_id` returns `Option<&'static str>`. Callers handle `None` by returning a dispatch error (the same shape `export_name_for_stage` callers used before).

## Locked Assumptions and Invariants

- ADR-0002's `with:` remap is preserved exactly. Verifiable by AC-3 (the three remap occurrences plus four bindgen occurrences).
- No WIT contract change: `crates/slicer-schema/wit/**` is untouched, so guest bindgen outputs are byte-identical post-rebuild.
- No runner-trait contract change: the trait methods' inputs/outputs are unchanged except for the `&CompiledModule` → `&CompiledModuleLive<'_>` parameter shift. Implementer must NOT add or remove methods.
- `CompiledModule = CompiledModuleStatic` type alias keeps any external code that still references `slicer_runtime::CompiledModule` compiling through P83. P85 deletes the alias and migrates `CompiledModuleStatic` to `slicer-scheduler`.
- The byte-identical g-code SHA carried forward from P81 (and confirmed in P82) MUST still match after P83 closure. If guest rebuild changes the SHA, the rebuild produced different wasm artifacts despite identical WIT inputs — a flag to investigate before declaring AC-9 green.

## Risks and Tradeoffs

- **Risk: bindgen `super::layer::…` path resolution breaks** when the layer world isn't declared first in the new `lib.rs`. Mitigation: enforce ordering — `pub mod layer;` (or whichever module contains the layer bindgen) precedes the others in `slicer-wasm-host/src/lib.rs`. AC-3 partly checks this via the count of `with:` occurrences.
- **Risk: hidden `wasmtime::` imports in non-moved runtime files.** Mitigation: dispatch #3 surfaces them. Each surfaced file gets `use wasmtime::...` rewritten to use the wasm-host re-exports, OR if the import is truly internal-only, the file gets a `use slicer_wasm_host::WasmComponent;`-style indirection.
- **Risk: `cargo test --workspace` reveals a guest-staleness regression.** Mitigation: gate-3 rebuilds guests before gate-4 runs `cargo test --workspace`. The CLAUDE.md discipline is followed in step ordering.
- **Tradeoff: `slicer-runtime` gains a `slicer-wasm-host` dep instead of `wasmtime`.** Net wash for the dep tree size; the win is the explicit named seam ("we depend on the wasm host"), not a dep reduction.
- **Tradeoff: `CompiledModuleStatic` lives in `slicer-runtime` for one packet** (P83) before moving to `slicer-scheduler` (P85). Acceptable transitional state; the type alias prevents call-site churn.

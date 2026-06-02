# ADR-0005: Runner trait defs live in slicer-wasm-host with IR-typed inputs/outputs

## Status

Accepted (packet 83, 2026-06-02)

## Context

Before packet 83 the project's wasmtime marshalling, dispatch logic, and the
four runner trait definitions (`LayerStageRunner`, `PrepassStageRunner`,
`FinalizationStageRunner`, `PostpassStageRunner`) all lived in
`slicer-runtime` — a crate that also owned the orchestration layer
(`Blackboard`, `LayerArena`, the per-stage executors). That mixing had four
costs:

1. **`slicer-runtime` carried a direct `wasmtime` dep**, so any future crate
   consuming the runtime paid for wasmtime even if it only wanted scheduler
   types. P85 cannot extract `slicer-scheduler` while `CompiledModule` exposes
   `Arc<WasmComponent>` accessors that bake wasmtime into the planning type.

2. **Dispatch trait impls + executors lived in the same crate**, so unit tests
   for orchestration logic could not mock the dispatcher cheaply.

3. **`dispatch::export_name_for_stage` duplicated the `STAGES[*].wit_export`
   table** that `slicer-schema` already owned. The schema is the documented
   single source of truth; the dispatcher re-hardcoding it was technical debt
   waiting to drift.

4. **Runner trait defs sat inside the executor modules** (`layer_executor.rs`,
   `prepass.rs`, etc.), entangling "what the executor needs from a runner"
   with "what the executor does with the runner's output". Moving the impls
   out was impossible without first lifting the trait defs.

The Step 4-pre survey of `dispatch.rs`'s four `impl *StageRunner for
WasmRuntimeDispatcher` blocks revealed a fifth cost worth surfacing: 18 of 18
`arena.*` accessor sites classified as **Category B** (pre-call marshal or
post-call commit), zero Category A (bindgen-Host trait impl) and zero
Category C (encapsulation leak). The trait method bodies intermixed wasmtime
invocation (belongs in the wasm-host crate) with arena fixup logic (belongs
in the orchestrator crate).

## Decision

Runner trait definitions move to `slicer-wasm-host` along with the four
`bindgen!` invocations, `HostExecutionContext` + builder, instance pool,
wasmtime engine wrappers, and the `WasmRuntimeDispatcher` struct. The trait
signatures are redesigned for a **symmetric IR-typed boundary**:

- Inputs: `*StageInput<'a>` borrow structs defined in `slicer-wasm-host` that
  carry IR-typed borrows of the specific Blackboard/LayerArena fields the
  dispatcher reads (per the Category-B survey). `Blackboard` and `LayerArena`
  themselves stay in `slicer-runtime`; the orchestrator constructs the input
  struct at the call site by projecting field-level borrows.
- Module access: `&CompiledModuleLive<'a>` defined in `slicer-wasm-host` with
  5 fields (`module_id: &'a ModuleId`, `instance_pool: Arc<WasmInstancePool>`,
  `wasm_component: Option<Arc<WasmComponent>>`, `claims: &'a [String]`,
  `config_view: Arc<ConfigView>`). No back-edge dep on `slicer-runtime`.
- Outputs: IR-typed structs. `LayerStageRunner::run_stage` returns
  `LayerStageCommitData` (defined in `slicer-ir`), with the wasm-host runner
  impl deconstructing `HostExecutionContext` into the IR commit struct
  **inside the impl, before returning**. `PrepassStageRunner` returns
  `slicer_core::PrepassStageOutput`. `FinalizationStageRunner` and
  `PostpassStageRunner` keep their existing thin enums from `slicer-ir`.
- Errors: narrow runner-side error types in `slicer-ir`. `PrepassRunnerError`
  carries only the variants the wasm dispatcher actually constructs
  (`FatalModule`, `Blackboard`); the broader `PrepassExecutionError` stays in
  `slicer-runtime` with a `From<PrepassRunnerError>` impl so the
  orchestrator's `?` lifts narrow to broad transparently. Same idiom as P86's
  `GCodeEmitError → PostpassError`.

Dep direction: `slicer-runtime → slicer-wasm-host → wasmtime`. No back-edge.

Wasm-host-internal types (`HostExecutionContext`, instance pool handles,
bindgen-coupled state) do **NOT** cross the trait boundary in either
direction. The deconstruction happens inside the runner impl on the wasm-host
side **before** return; `commit_layer_outputs` on the runtime side consumes
only `LayerStageCommitData`.

The four `bindgen!` invocations remain co-located in `slicer-wasm-host` (one
file: `host.rs`) so ADR-0002's `with:` remap pattern continues to give all
four worlds shared Rust type identity. The `pub mod layer` declaration must
precede the other three modules so the `super::layer::…` paths resolve.

## Consequences

- `slicer-runtime` no longer carries a direct `wasmtime` dep; `cargo tree -p
  slicer-runtime --depth 1 --edges normal` does not list wasmtime. wasmtime
  reaches the runtime only transitively via `slicer-wasm-host`.
- AC-N3 (no back-edge dep on slicer-runtime from wasm-host) holds: the
  borrow-struct pattern + IR-typed outputs eliminate every previous coupling.
- P85 can extract `slicer-scheduler` cleanly because `CompiledModuleStatic`
  (the planning shape that survives in `slicer-runtime` after this packet) has
  no wasmtime-typed fields exposed publicly; the `instance_pool()` and
  `wasm_component()` accessors are deleted in P83.
- `commit_layer_outputs` and the arena-fixup helpers move from `dispatch.rs`
  into `slicer-runtime/src/layer_executor.rs` (Step 4d). Their signatures
  change from `(ctx: HostExecutionContext, …)` to `(commit:
  LayerStageCommitData, …)`. Call sites in the per-stage executor files
  construct `*StageInput<'_>` + `CompiledModuleLive<'_>` before invoking the
  trait method, then consume the returned commit data.
- The narrow-runner-error pattern (`PrepassRunnerError`) sets a precedent for
  future runner traits where the orchestrator-level error is broader than what
  the wasm dispatcher can construct. Aligns with P86's
  `GCodeEmitError → PostpassError`.

## Alternatives considered

- **Single `WasmDispatcher` super-trait collapsing the four runner traits.**
  Rejected: each runner trait has different inputs (Layer takes per-layer
  state, Prepass takes mesh-level state, Finalization takes the full layer
  collection, Postpass takes the assembled g-code IR). Collapsing forfeits
  type safety. The four-trait shape exists in the codebase today and is
  documented in `docs/04_host_scheduler.md`.
- **Adapter trait `LayerArenaAccess` in slicer-ir.** Rejected: design.md
  explicitly forbids "new abstractions" introduced just to make the move work,
  and this adds dyn-dispatch overhead per accessor call. The Category-B
  classification of 18/18 sites showed adapter traits are unnecessary — the
  arena fixup is purely runtime-side.
- **Return `HostExecutionContext` directly from the trait** (instead of
  `LayerStageCommitData`). Rejected: leaks wasm-host's internal bookkeeping
  (instance pool handles, intermediate builder state, bindgen-coupled fields)
  upward into the orchestrator. Defeats the entire arc of P83 — symmetric
  IR-typed seam, runtime-private state stays on each side of the seam.

## Verification

- `grep -cE 'wasmtime::component::bindgen!' crates/slicer-wasm-host/src/host.rs`
  returns 4 (one per world).
- `! grep -rE 'fn run.*Blackboard|fn run.*LayerArena|fn run.*PrepassExecutionError|fn run.*HostExecutionContext|HostExecutionContext.*->|->.*HostExecutionContext' crates/slicer-wasm-host/src/`
  returns no matches.
- `cargo tree -p slicer-runtime --depth 1 --edges normal` does not list
  `wasmtime`.
- `! grep -rE 'use slicer_wasm_host::HostExecutionContext' crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs`
  matches (no wasm-host-internal types in executor commit paths).

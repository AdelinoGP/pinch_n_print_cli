# CompiledModule splits Static (scheduler) and Live (wasm-host); pairing by HashMap

**Status:** accepted (packet 85, TASK-235)

## Context

P83 introduced two changes in `slicer-runtime` that together were supposed to
prepare the scheduler/wasm-host boundary for separation:

1. The `CompiledModule` struct was renamed to `CompiledModuleStatic`, with a
   transitional `pub type CompiledModule = CompiledModuleStatic;` alias so
   downstream callers kept compiling.
2. A new `CompiledModuleLive<'s>` borrowing wrapper was added in
   `slicer-wasm-host`, intended to own the wasmtime payload (an instance pool
   and an optional compiled component) and borrow the Static half by reference.

P85 set out to extract the planning subsystem (manifest ingestion, config
resolution, DAG construction + validation, execution-plan compilation, DAG-CLI
introspection — nine files, ~5 500 LOC) into a new `slicer-scheduler` crate
that does not link `wasmtime`. The packet's framing assumed P83 had completed
the Static/Live split at the **field** level: `CompiledModuleStatic` was
expected to be wasmtime-free, and `CompiledModuleLive` was expected to already
own the wasmtime fields.

Field migration was incomplete. `CompiledModuleStatic`,
`CompiledModuleBuilder`, and `ExecutionModuleBinding` still carried
`instance_pool: Arc<WasmInstancePool>` and
`wasm_component: Option<Arc<WasmComponent>>` directly. `execution_plan.rs`
also embedded a "live loader" cluster (`LiveModuleBinding`,
`build_live_execution_plan`, `LiveModuleLoadOutput`, `LiveModuleLoadError`,
`load_live_modules_for_plan`, `compile_module_component`) — six symbols that
linked against `slicer_wasm_host::*` directly. A verbatim move of the file into
the new scheduler crate would have required `slicer-scheduler` to depend on
`slicer-wasm-host`, defeating the entire point of the split.

## Decision

`CompiledModuleStatic` lives in `slicer-scheduler` and is **wasmtime-free**. It
carries only the planning-time data: `module_id`, `claims`, `config_view`,
`ir_read_mask`, `ir_write_mask`, and the like. `CompiledModuleLive<'s>` lives in
`slicer-wasm-host` and owns the wasmtime payload directly (its own
`instance_pool: Arc<WasmInstancePool>` and `wasm_component: Option<Arc<WasmComponent>>`
fields, not a borrow). The "live loader" cluster moves to a new
`slicer-wasm-host/src/execution_plan_live.rs` module alongside `binding.rs`.

The two halves are paired at runtime by a
`wasm_handles: HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>`
threaded through every executor function (`execute_prepass`,
`execute_per_layer`, `execute_layer_finalization`, `execute_postpass`, and
their `*_with_builtins` / `*_with_instrumentation` variants). At the point
where a guest needs to dispatch, the executor looks up the handles by the
Static module's `module_id` and constructs a `CompiledModuleLive` on demand:
`CompiledModuleLive::new(static_module.module_id(), pool, component, …)`.

## Rationale — HashMap pairing instead of `&'s CompiledModuleStatic` borrow

The packet's initial AC-5 phrased the pairing as a lifetime borrow:
`CompiledModuleLive<'s>` would carry `static_module: &'s CompiledModuleStatic`
and inherit the Static's fields by reference. This was rejected during P85 in
favor of the HashMap form. Three reasons:

1. **Lifetime cascade.** Threading `'s` through `LayerStageRunner::run_stage`,
   `PostpassStageRunner::run_stage`, `FinalizationStageRunner::run_stage`, and
   their trait-method counterparts would require every implementor and every
   call site to propagate the lifetime. The blast radius was significantly
   larger than the borrow's value.
2. **Per-tick reconstruction.** Several future call sites (parallel per-layer
   execution, hot reload of module config) want to build Live values fresh
   each tick from a (Static, pool, component) triple rather than carry a
   long-lived borrow that ties the executor to a particular Static instance's
   lifetime.
3. **Decoupling.** Static and Live now have independent lifetimes. A test that
   constructs a `TestModuleBundle { module, pool, component }` and calls
   `bundle.as_live()` is the canonical example: the bundle owns Static, the
   `as_live()` call materialises a Live that borrows nothing from the bundle's
   own structure (only from the `module_id()`, `claims()`, etc. accessors on
   Static).

## Consequences

- `cargo tree -p slicer-scheduler --edges normal` produces zero `wasmtime`
  entries at any depth (AC-N2). Plan-shape regression tests can run in
  `slicer-scheduler/tests/` without linking wasmtime. P85 moved 14 such tests
  into the new bucket; 128 scheduler tests now run.
- `slicer-wasm-host` gains `slicer-scheduler` as a one-way dep. `slicer-runtime`
  also adopts the dep so its executor can call back into the planning surface
  through the transitional re-export shim in `slicer-runtime/src/lib.rs`.
- The `pub type CompiledModule = CompiledModuleStatic;` alias from P83 is
  deleted. External callers reference `slicer_scheduler::CompiledModuleStatic`
  directly (or the transitional `pub use … as CompiledModule;` re-export, if
  they were already grepping `slicer_runtime::CompiledModule`).
- The runtime constructs `CompiledModuleLive::new(…)` per tick at the site
  where Static and the engine artifacts are both in scope (typically the
  per-stage dispatch). For tests that don't exercise real dispatch,
  `WasmInstancePool::placeholder()` is acceptable. For tests that DO exercise
  real dispatch (e.g., macro guest roundtrips, real-WASM live tier tests), the
  test fixture MUST populate the real pool and component — the `TestModuleBundle`
  helper in `crates/slicer-runtime/tests/common/mod.rs` is the canonical
  encapsulation.

## What future architecture reviews must not re-litigate

- **Do not re-merge Static and Live into a single struct.** The point of the
  split is that the planning crate stays wasmtime-free. Merging them re-imports
  `slicer-wasm-host` (or `wasmtime`) into the scheduler's dep tree.
- **Do not re-introduce the `&'s CompiledModuleStatic` borrow shape without
  superseding this ADR.** The lifetime cascade was costed and rejected; the
  HashMap pairing is the established contract.
- **Do not consolidate `WasmInstancePool::placeholder()` away.** It exists as
  the explicit fallback for in-process test pipelines that don't need real
  dispatch. Tests that DO need real dispatch must use a real pool — not
  silently fall back through the placeholder.

## Cross-references

- ADR-0002 (WIT marshalling type unification) — confirms what stays in
  `slicer-wasm-host` (the bindgen + dispatcher impls) vs what moves out.
- ADR-0005 (runner traits in slicer-wasm-host) — `LayerStageRunner`,
  `PostpassStageRunner`, etc. still own the dispatch contract; this ADR only
  changes what data they consume (a `CompiledModuleLive` whose Static half
  lives in a different crate).
- ADR-0006 (export-for-stage-id sole lookup) — orthogonal; the stage-id
  lookup mechanism is unaffected.
- P83 closure — supersedes the "P83 completed the split" framing. The type
  rename completed in P83; the field migration completed in P85.

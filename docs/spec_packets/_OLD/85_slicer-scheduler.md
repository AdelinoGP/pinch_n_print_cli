---
status: implemented
packet: 85
task_ids: [TASK-235]
---

# 85_slicer-scheduler

## Goal

Move the static-planning subsystem (`manifest.rs` + `config_resolution.rs` + `dag.rs` + `validation.rs` + `execution_plan.rs` + `topology.rs` + `stage_order.rs` + `module_search_path.rs` + `dag_cli.rs`, plus the *planning-side* of `instrumentation.rs` — `compute_serial_edges_for_stage`, `EdgeReason`, `SerialEdge` — together ~5 500 LOC) out of `slicer-runtime/src/` into a new `slicer-scheduler` crate; complete the `CompiledModule` split begun in P83 by relocating `CompiledModuleStatic` to the new crate and deleting the `pub type CompiledModule = CompiledModuleStatic` transitional alias; update `slicer-wasm-host::binding::CompiledModuleLive<'s>` to borrow `&'s slicer_scheduler::CompiledModuleStatic` so `slicer-scheduler` itself links zero `wasmtime` code.

## Problem Statement

The static-planning subsystem inside `slicer-runtime` — manifest loading, config resolution, DAG construction, claim validation, execution-plan freezing, instrumentation edge computation, and CLI introspection (`dag_cli`) — totals ~5 500 LOC that has zero need for `wasmtime`. Yet today the planning code shares its crate with the per-layer executor that does. Three concrete consequences:

1. **The planning crate cannot be tested without linking wasmtime.** Plan-shape regression tests sit in `slicer-runtime/tests/`, paying the wasmtime compile cost on every iteration.
2. **`CompiledModule`'s two wasmtime accessors** — `pub fn instance_pool() → &Arc<WasmInstancePool>` and `pub fn wasm_component() → Option<&Arc<WasmComponent>>` — block the extraction because they leak wasmtime types out of any consumer of `ExecutionPlan`. P83 already broke this dependency by splitting `CompiledModule` into Static (no wasmtime) + Live (the borrow that holds the wasmtime types). P85 completes the split by moving the Static half out of `slicer-runtime`.
3. **`dag_cli`'s pure-introspection commands** (`run_dag_stages`, `run_dag_stage`, `run_dag_depends`, `run_dag_claims`) live in the host library crate even though they are planning-time tools `pnp-cli` calls directly. They belong with the planning code.

The fix is one new crate (`slicer-scheduler`), one type-move-completion (`CompiledModuleStatic` → scheduler, transitional alias deleted), one instrumentation file split (planning side vs runtime side), and dep-direction adjustments in `slicer-wasm-host` (gains a `slicer-scheduler` dep) and `slicer-runtime` + `pnp-cli` (both depend on the new crate).

## Architecture Constraints

- ADR-0001 (in-stage commits) preserved: built-in commits live in `slicer-runtime/src/builtins/` (P84 territory), not in scheduler.
- ADR-0002 / ADR-0003 / ADR-0004 / ADR-0005 (from prior packets) preserved.
- **No cycle**: `slicer-scheduler → {slicer-ir, slicer-schema, slicer-sdk}`. `slicer-wasm-host → slicer-scheduler`. `slicer-runtime → {slicer-scheduler, slicer-wasm-host, slicer-core, slicer-helpers, slicer-ir, slicer-sdk}`. `pnp-cli → {slicer-runtime, slicer-scheduler, slicer-model-io}`. Verified post-move via `cargo metadata`.
- `slicer-scheduler` MUST NOT depend on `wasmtime` direct or transitive. AC-N2 verifies via `cargo tree`.
- No path in this packet's change surface feeds the guest WASM build (slicer-ir / slicer-sdk / slicer-schema / slicer-macros are untouched). The `wasm-staleness` snippet is intentionally NOT included here.

## Data and Contract Notes

- `LoadedModule`, `ExecutionPlan`, `CompiledModuleStatic`, `ConfigSchema`, `DagValidationReport`, `Producer`, `BuiltinProducer`, and other planning types preserve their public shapes exactly. Only the crate path changes.
- `BuiltinProducer` is defined in `crates/slicer-runtime/src/dag.rs` today (a trait that the P84 wrappers implement). After P85, it lives in `crates/slicer-scheduler/src/dag.rs`. The P84 wrappers in `crates/slicer-runtime/src/builtins/` rewrite their `use crate::dag::BuiltinProducer;` to `use slicer_scheduler::dag::BuiltinProducer;` (or via re-export).
- `run_dag_*` fn signatures preserve exactly — `pnp_cli`'s subcommand handlers continue to work without arg changes.

## Locked Assumptions and Invariants

- ADR-0001 / 0002 / 0003 preserved; ADR-0005 / ADR-0006 (from P83) preserved.
- New invariant codified in ADR-0007 (at packet close): `slicer-scheduler` is wasmtime-free; `CompiledModuleLive` lives in `slicer-wasm-host` and **owns the wasmtime payload** (`Arc<WasmInstancePool>`, `Option<Arc<WasmComponent>>`). Pairing with `CompiledModuleStatic` is by external key (`HashMap<ModuleId, CompiledModuleLive>` threaded through the executor structures), NOT by lifetime-borrowed reference — chosen during Step 3.5 to avoid lifetime-parameter cascade through every executor method signature in `slicer-runtime`. Future architecture reviews must not re-merge Static and Live, and must not re-introduce the borrow shape without an explicit superseding ADR.
- Byte-identical g-code: AC-9 SHA = P84 closure SHA.
- Byte-identical `pnp_cli dag` output: AC-10 SHAs = Step 0 baselines.
- `cargo xtask build-guests --check` stays clean throughout (no guest-feeding path is edited).

## Risks and Tradeoffs

- **Confirmed pre-packet contradiction (resolved by expanded scope, not deferred): P83 left `Arc<WasmInstancePool>` and `Option<Arc<WasmComponent>>` as fields on `CompiledModuleStatic`, `CompiledModuleBuilder`, and `ExecutionModuleBinding`.** P85 was originally framed as "move a clean Static struct"; the real codebase state required completing the field migration first. The expanded scope (Step 3.5 below) does the field strip + Live-cluster relocation to wasm-host as part of this packet. The original mitigation ("the build will surface it") DID surface it; we expanded scope rather than defer to a follow-up. P83's deliverable is recorded as architecturally-complete-at-the-type-level but field-incomplete; P85 closes the gap.
- **Risk: a callsite rewire is missed**, causing a runtime test failure in the workspace gate. The mechanical pattern is `compiled_module.instance_pool()` → `live_binding.instance_pool()` at every site in `slicer-runtime/src/{layer_executor,pipeline,prepass,postpass,layer_finalization}.rs`. Mitigation: post-Step-3.5 `cargo build --workspace --all-targets` surfaces every missed site as an E0599 (method not found on Static) — the error message names the file and line. Workspace test gate (Step 8) is the integration verification.
- **Risk: a moved file references something the runtime still needs internally** outside of the wasmtime cluster. E.g., `dag.rs` references `crate::instrumentation::EdgeReason`; after move, that becomes a same-crate import in `slicer-scheduler`. Mitigation: dispatch #1 surfaces SDK imports; an early `cargo build -p slicer-scheduler` against the post-move tree catches any surprise.
- **Risk: the test re-exports in `slicer-runtime/src/lib.rs` are incomplete**, breaking tests that grep `slicer_runtime::X`. Mitigation: dispatch #3 enumerates external `pub use slicer_runtime::*` references; the transitional re-export list covers them.
- **Risk: `cargo test --workspace` flakes** (a known issue in large suites). Mitigation: dispatch #9 captures the tail output, so a flaky test can be re-run individually without re-doing the whole gate.
- **Tradeoff: the transitional `pub use slicer_scheduler::*` re-exports in `slicer-runtime/src/lib.rs`** keep test compatibility but inflate the runtime crate's apparent surface. They are explicitly transitional; a follow-up packet (P89 or similar in the doc-sweep phase) deletes them.

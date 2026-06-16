# Packet 85 — Requirements

## Problem Statement

The static-planning subsystem inside `slicer-runtime` — manifest loading, config resolution, DAG construction, claim validation, execution-plan freezing, instrumentation edge computation, and CLI introspection (`dag_cli`) — totals ~5 500 LOC that has zero need for `wasmtime`. Yet today the planning code shares its crate with the per-layer executor that does. Three concrete consequences:

1. **The planning crate cannot be tested without linking wasmtime.** Plan-shape regression tests sit in `slicer-runtime/tests/`, paying the wasmtime compile cost on every iteration.
2. **`CompiledModule`'s two wasmtime accessors** — `pub fn instance_pool() → &Arc<WasmInstancePool>` and `pub fn wasm_component() → Option<&Arc<WasmComponent>>` — block the extraction because they leak wasmtime types out of any consumer of `ExecutionPlan`. P83 already broke this dependency by splitting `CompiledModule` into Static (no wasmtime) + Live (the borrow that holds the wasmtime types). P85 completes the split by moving the Static half out of `slicer-runtime`.
3. **`dag_cli`'s pure-introspection commands** (`run_dag_stages`, `run_dag_stage`, `run_dag_depends`, `run_dag_claims`) live in the host library crate even though they are planning-time tools `pnp-cli` calls directly. They belong with the planning code.

The fix is one new crate (`slicer-scheduler`), one type-move-completion (`CompiledModuleStatic` → scheduler, transitional alias deleted), one instrumentation file split (planning side vs runtime side), and dep-direction adjustments in `slicer-wasm-host` (gains a `slicer-scheduler` dep) and `slicer-runtime` + `pnp-cli` (both depend on the new crate).

## Grouped Task IDs

- **TASK-235** (new) — Extract the static-plan subsystem into `slicer-scheduler`. Recorded under "Architecture Deepening Phase II" alongside TASK-234 (P84) and TASK-236 (P86).

## In Scope

- Create `crates/slicer-scheduler/` with:
  - `Cargo.toml` declaring `slicer-ir` (path) and `slicer-schema` (path). If any moved file imports SDK trait types (confirm via dispatch #1), also `slicer-sdk` (path). External deps: `toml = "0.8"` (if manifest.rs uses it), `serde` (workspace inheritance), `thiserror`, etc. — whatever the moved code uses. NO `wasmtime`.
  - `src/lib.rs` with `pub mod manifest; pub mod config_resolution; pub mod dag; pub mod validation; pub mod execution_plan; pub mod instrumentation; pub mod topology; pub mod stage_order; pub mod module_search_path; pub mod dag_cli;` plus a careful `pub use` re-export block matching what `slicer-runtime/src/lib.rs` re-exported pre-P85 for these modules.
- Move into `slicer-scheduler/src/` (verbatim file content; internal `use crate::` paths preserved since the module hierarchy is the same):
  - `crates/slicer-runtime/src/manifest.rs` (1 168 LOC).
  - `crates/slicer-runtime/src/config_resolution.rs` (485 LOC).
  - `crates/slicer-runtime/src/dag.rs` (458 LOC).
  - `crates/slicer-runtime/src/validation.rs` (1 115 LOC).
  - `crates/slicer-runtime/src/execution_plan.rs` (1 379 LOC).
  - `crates/slicer-runtime/src/topology.rs` (70 LOC).
  - `crates/slicer-runtime/src/stage_order.rs` (94 LOC).
  - `crates/slicer-runtime/src/module_search_path.rs` (110 LOC).
  - `crates/slicer-runtime/src/dag_cli.rs` (633 LOC).
- **Split `instrumentation.rs` (842 LOC) between scheduler and runtime**:
  - **Move to `slicer-scheduler/src/instrumentation.rs`**: `compute_serial_edges_for_stage` (input: `&[LoadedModule]` — planning data), `EdgeReason` enum, `SerialEdge` struct.
  - **Keep in `slicer-runtime/src/instrumentation.rs`**: `pub trait PipelineInstrumentation`, `Phase`, `TierKind`, `compute_serial_edges_from_compiled` (input: `&[CompiledModuleLive<'_>]` — runtime data; this stays because it's called from the executor's bracket hooks).
- **Complete the `CompiledModule` Static/Live split** (begun but not finished in P83 — the structural premise of this packet):
  - **Strip the wasmtime fields from `CompiledModuleStatic`, `CompiledModuleBuilder`, and `ExecutionModuleBinding`.** P83 renamed `CompiledModule → CompiledModuleStatic` and added `CompiledModuleLive<'s>` as a borrowing wrapper, but **left the wasmtime payload on the Static side** (`instance_pool: Arc<WasmInstancePool>`, `wasm_component: Option<Arc<WasmComponent>>`). P85 finishes the split: these two fields move to the Live side. After P85, `CompiledModuleStatic` carries only manifest-static data (no `Arc<Wasm*>` anywhere).
  - **Relocate the live-loader cluster from `crates/slicer-runtime/src/execution_plan.rs` to `crates/slicer-wasm-host/src/execution_plan_live.rs`** (new file). The six symbols: `LiveModuleBinding`, `build_live_execution_plan`, `LiveModuleLoadOutput`, `LiveModuleLoadError`, `load_live_modules_for_plan`, `compile_module_component`. These all consume wasmtime types and produce `CompiledModuleLive<'_>` bindings; they belong on the wasm-host side of the split.
  - **Extend `CompiledModuleLive<'s>` to own the wasmtime payload.** Methods that today read `&Arc<WasmInstancePool>` and `Option<&Arc<WasmComponent>>` off `CompiledModuleStatic` become methods on `CompiledModuleLive` instead. The borrow shape becomes: `CompiledModuleLive<'s> { static_module: &'s CompiledModuleStatic, instance_pool: Arc<WasmInstancePool>, wasm_component: Option<Arc<WasmComponent>> }` (or the equivalent post-design shape — implementer chooses the exact field layout).
  - Move the (now-clean) `pub struct CompiledModuleStatic` from `crates/slicer-runtime/src/execution_plan.rs` into `crates/slicer-scheduler/src/execution_plan.rs`.
  - Delete the transitional `pub type CompiledModule = CompiledModuleStatic;` alias that P83 added.
  - Update the `CompiledModuleLive<'s>` borrow target so it references `&'s slicer_scheduler::CompiledModuleStatic`.
  - Add `slicer-scheduler = { path = "../slicer-scheduler" }` to `crates/slicer-wasm-host/Cargo.toml` (one-way edge; AC-N1 still forbids the reverse).
  - **Rewire callsites** in `slicer-runtime` (layer executor, pipeline, prepass/postpass orchestrators) and `pnp-cli` (if any `dag` subcommand calls `instance_pool()` on Static — investigation surfaced none, but the rewire pass verifies). Every former `compiled_module.instance_pool()` / `compiled_module.wasm_component()` callsite becomes `live_binding.instance_pool()` / `live_binding.wasm_component()` — the runtime constructs the Live binding per tick from Static + the loaded engine artifacts.
- Update `crates/slicer-runtime/src/lib.rs`:
  - Drop the nine `pub mod ...;` declarations (manifest, config_resolution, dag, validation, execution_plan, topology, stage_order, module_search_path, dag_cli).
  - Drop the matching `pub use ...;` blocks for those modules.
  - For backward source compat, optionally add `pub use slicer_scheduler::{ExecutionPlan, CompiledModuleStatic, LoadedModule, ConfigSchema, ...};` re-exports at lib.rs so external tests that grep `slicer_runtime::ExecutionPlan` continue to compile. The exact re-export list is determined by tracing call sites in `crates/slicer-runtime/tests/`.
  - Keep `pub mod instrumentation;` (the file is preserved with only the runtime-side fns; the planning-side fns are gone).
- Update `crates/slicer-runtime/Cargo.toml`:
  - Add `slicer-scheduler = { path = "../slicer-scheduler" }`.
  - Remove any direct dep that was used only by moved files (`toml` is the prime candidate — if `manifest.rs` was its sole consumer in the runtime, drop it).
- Update `crates/pnp-cli/Cargo.toml`:
  - Add `slicer-scheduler = { path = "../slicer-scheduler" }`.
- Update `crates/pnp-cli/src/main.rs` (or its `dag` subcommand module):
  - Replace `use slicer_runtime::{run_dag_stages, run_dag_stage, run_dag_depends, run_dag_claims, ...};` with `use slicer_scheduler::{run_dag_stages, ...};`.
- Migrate tests that exercised the moved subsystem from `crates/slicer-runtime/tests/` into `crates/slicer-scheduler/tests/`. The migration criterion: a test moves if its SUT is a moved symbol (`build_execution_plan`, `validate_startup_dag`, `load_modules_from_roots`, etc.); it stays if its SUT is a runtime symbol that happens to consume an `ExecutionPlan`.

## Out of Scope

- `region_mapping.rs` — P87. Its public sig leaks `ExecutionPlan`, but until P85 ships, that's runtime-internal coupling, not cross-crate leakage. P87 takes it after P85 settles.
- `gcode_emit.rs` — P86.
- The host built-ins under `slicer-runtime/src/builtins/` (P84). Their imports may need updating to use the new `slicer_scheduler::*` paths instead of `slicer_runtime::*`, but no semantic change.
- `wit_host.rs`, `dispatch.rs`, `wasm_instance.rs`, `instance_pool.rs` — already in `slicer-wasm-host` per P83.
- Touching `crates/slicer-test/`, `crates/slicer-sdk/` — concurrent work.
- WIT contract changes. None needed.
- New abstractions. The moved files preserve their public surfaces exactly.

## Authoritative Docs

- `docs/04_host_scheduler.md` — confirms the DAG, validation, four-phase execution. No content change; this packet preserves behavior.
- `docs/03_wit_and_manifest.md` — confirms the manifest TOML schema and config validation rules. No content change.
- `docs/adr/0002-wit-marshalling-type-unification.md` (60 LOC) — confirms what stays in `slicer-wasm-host` (the bindgen + dispatcher impl) vs what moves to `slicer-scheduler` (planning).
- `CLAUDE.md` §"Test Discipline" — workspace-test dispatch contract for the AC-11 checkpoint gate.
- `CLAUDE.md` §"Guest WASM Staleness" — confirms this packet edits no guest-feeding path, so `--check` should stay clean.

## Acceptance Summary

The acceptance contract is enumerated in `packet.spec.md` (AC-1..AC-11, AC-N1..AC-N3). Measurable refinements:

- **AC-10 — `pnp_cli dag` output parity**: capture pre-packet output for `pnp_cli dag stages` and `pnp_cli dag claims` in Step 0; post-packet output must be byte-identical. The implementation log records SHAs of both pre/post outputs.
- **AC-11 — workspace test gate**: per CLAUDE.md, dispatch the full run to a sub-agent that returns FACT pass/fail + duration + test count. The count delta vs the P84 baseline must explain itself (tests migrated from `slicer-runtime` to `slicer-scheduler` produce equal-and-opposite count shifts).
- **AC-N2 — `cargo tree -p slicer-scheduler` no wasmtime**: the architectural win of the Static/Live split. If wasmtime appears anywhere in that tree, the split is incorrect.

## Verification Commands

| ID | Command | Delegation hint |
|---|---|---|
| AC-1 | `test -f crates/slicer-scheduler/Cargo.toml && ! grep -qE '^(wasmtime\|slicer-wasm-host\|slicer-runtime) *=' crates/slicer-scheduler/Cargo.toml && ! cargo tree -p slicer-scheduler --edges normal 2>&1 \| grep -qE 'wasmtime'` | FACT pass/fail |
| AC-2 | `for f in manifest config_resolution dag validation execution_plan topology stage_order module_search_path dag_cli; do test ! -f crates/slicer-runtime/src/$f.rs \|\| exit 1; done` | FACT pass/fail |
| AC-3 | `rg -l 'pub fn compute_serial_edges_for_stage' crates/ \| grep -qE '^crates/slicer-scheduler/' && rg -l 'pub trait PipelineInstrumentation' crates/ \| grep -qE '^crates/slicer-runtime/'` | FACT pass/fail |
| AC-4 | `rg -l 'pub struct CompiledModuleStatic' crates/ \| grep -qE '^crates/slicer-scheduler/' && ! rg -q 'pub type CompiledModule = CompiledModuleStatic' crates/` | FACT pass/fail |
| AC-5 | `grep -rqE 'slicer_scheduler::CompiledModuleStatic\|use slicer_scheduler::CompiledModuleStatic' crates/slicer-wasm-host/src/ && grep -qE '^slicer-scheduler *=' crates/slicer-wasm-host/Cargo.toml` | FACT pass/fail |
| AC-6 | `grep -rqE 'use slicer_scheduler::.*run_dag' crates/pnp-cli/src/ && ! grep -rqE 'use slicer_runtime::.*run_dag' crates/pnp-cli/src/` | FACT pass/fail |
| AC-7 | `! grep -qE '^pub mod (manifest\|config_resolution\|dag\|validation\|execution_plan\|topology\|stage_order\|module_search_path\|dag_cli);' crates/slicer-runtime/src/lib.rs` | FACT pass/fail |
| AC-8 | `grep -qE '^slicer-scheduler *=' crates/slicer-runtime/Cargo.toml` | FACT pass/fail |
| AC-9 | `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p85.gcode && sha256sum /tmp/benchy-p85.gcode` | SNIPPET (SHA) |
| AC-10 | `for cmd in stages claims; do cargo run --bin pnp_cli --release -- dag $cmd --module-dir modules/core-modules > /tmp/p85-dag-$cmd-1.txt && cargo run --bin pnp_cli --release -- dag $cmd --module-dir modules/core-modules > /tmp/p85-dag-$cmd-2.txt && diff -q /tmp/p85-dag-$cmd-1.txt /tmp/p85-dag-$cmd-2.txt \|\| exit 1; done` (consecutive runs must produce identical output — determinism test, NOT baseline parity) | FACT pass/fail |
| AC-11 | `cargo test --workspace` | FACT pass/fail + duration + count |
| AC-N1 | `! grep -qE '^slicer-(runtime\|wasm-host) *=' crates/slicer-scheduler/Cargo.toml` | FACT pass/fail |
| AC-N2 | `! cargo tree -p slicer-scheduler 2>&1 \| grep -qE '\bwasmtime\b'` | FACT pass/fail |
| AC-N3 | `! ls crates/slicer-runtime/src/{manifest,dag,validation,execution_plan}.rs 2>/dev/null \| grep -q .` | FACT pass/fail |
| AC-N4 | `for line in $(grep -nE '^pub use slicer_scheduler::' crates/slicer-runtime/src/lib.rs \| cut -d: -f1); do next=$((line+1)); sed -n "${next}p" crates/slicer-runtime/src/lib.rs \| grep -qE '^// kept:' \|\| exit 1; done` | FACT pass/fail |
| AC-N5 | `[ -d crates/slicer-scheduler/tests ] && [ $(cargo test -p slicer-scheduler 2>&1 \| grep -oE 'test result: ok\. [0-9]+ passed' \| awk '{sum += $4} END {print sum+0}') -ge 18 ] && ! rg -e 'use slicer_(wasm_host\|runtime)::' crates/slicer-scheduler/tests/` | FACT pass/fail + count |
| gate-1 | `cargo build --workspace` | FACT pass/fail |
| gate-2 | `cargo clippy --workspace --all-targets -- -D warnings` | FACT pass/fail |
| gate-3 | `cargo xtask build-guests --check` | FACT pass/fail |

## Step Completion Expectations

- The Static-half move MUST happen together with the `CompiledModuleLive` borrow-type update; otherwise the wasm-host compile breaks (it would borrow a type that no longer lives in `slicer-runtime`).
- The instrumentation split MUST land together with the manifest/dag moves, because `dag.rs` imports `crate::instrumentation::EdgeReason`. If instrumentation is half-split when dag.rs moves, `slicer-scheduler` won't compile.
- `slicer-runtime/Cargo.toml` MUST add the `slicer-scheduler` dep AT the same commit as the `pub mod ...;` deletions; otherwise the runtime build references `crate::manifest::*` paths that no longer exist.
- Guest rebuild is NOT required (no slicer-ir / slicer-sdk / slicer-schema / slicer-macros edit); `cargo xtask build-guests --check` should stay clean. If it goes STALE, the cause is elsewhere — investigate before papering over.

## Packet-Specific Context Discipline

- The nine moved files total ~5 500 LOC. NEVER load any in full. The move is verbatim — read only the `pub` surface (top of file) and any `use crate::` lines to verify the post-move `use` rewrites are correct.
- `execution_plan.rs` (1 379 LOC) is the largest; pay particular attention to its `CompiledModuleStatic` move and the deletion of the `pub type CompiledModule = ...` alias.
- `dag_cli.rs` (633 LOC) imports `crate::dag::*` and `crate::instrumentation::{EdgeReason, SerialEdge}` — after the move, all those become same-crate (`slicer-scheduler`) imports; no rewrite needed beyond the file move.
- `OrcaSlicerDocumented/` is irrelevant. No parity surface.

# Packet 85 — Design

## Controlling Code Paths

The new crate `slicer-scheduler` sits between `slicer-ir`/`slicer-schema` (upstream) and `slicer-wasm-host` + `slicer-runtime` + `pnp-cli` (downstream). After this packet:

```
              slicer-ir, slicer-schema, slicer-sdk
                                │
                                ▼
                       slicer-scheduler  (zero wasmtime)
                       ─────────┬────────┬────────┬──────────────
                                │        │        │              │
                                ▼        ▼        ▼              ▼
                       slicer-wasm-host  pnp-cli  slicer-runtime  (future consumers)
                                                  (gains slicer-
                                                   scheduler dep)
```

`slicer-wasm-host`'s `CompiledModuleLive<'s>` finally borrows `&'s slicer_scheduler::CompiledModuleStatic` — the Static/Live split is fully realised. `pnp-cli` gains a direct `dag` subcommand path through `slicer-scheduler::run_dag_*`.

OrcaSlicer comparison surface: none. No parity behavior moves.

## Architecture Constraints

- ADR-0001 (in-stage commits) preserved: built-in commits live in `slicer-runtime/src/builtins/` (P84 territory), not in scheduler.
- ADR-0002 / ADR-0003 / ADR-0004 / ADR-0005 (from prior packets) preserved.
- **No cycle**: `slicer-scheduler → {slicer-ir, slicer-schema, slicer-sdk}`. `slicer-wasm-host → slicer-scheduler`. `slicer-runtime → {slicer-scheduler, slicer-wasm-host, slicer-core, slicer-helpers, slicer-ir, slicer-sdk}`. `pnp-cli → {slicer-runtime, slicer-scheduler, slicer-model-io}`. Verified post-move via `cargo metadata`.
- `slicer-scheduler` MUST NOT depend on `wasmtime` direct or transitive. AC-N2 verifies via `cargo tree`.
- No path in this packet's change surface feeds the guest WASM build (slicer-ir / slicer-sdk / slicer-schema / slicer-macros are untouched). The `wasm-staleness` snippet is intentionally NOT included here.

## Selected Approach

Verbatim file moves + one struct relocation (`CompiledModuleStatic`) + one file split (`instrumentation.rs`). The moves preserve every file's internal structure; only the crate boundary changes.

Rejected alternatives:

- **Merge `slicer-scheduler` into `slicer-schema`** (since both are planning-time / SSOT-shaped). Rejected: `slicer-schema` is 388 LOC of `&'static` data tables; absorbing 5 500 LOC of mutable planning logic blurs the schema's role. Two-purpose crates are the failure mode this batch is trying to escape.
- **Keep `dag_cli.rs` in `pnp-cli`** (where it's called from). Rejected: `dag_cli` operates over the same `Producer` / `LoadedModule` types as the scheduler core; co-locating with the data it manipulates is cleaner. `pnp-cli` calls into `slicer_scheduler::run_dag_*` — one extra import, no logic duplication.
- **Split `execution_plan.rs` into two files** during the move (one for `ExecutionPlan`, one for the build pipeline). Rejected: scope creep. The file's internal cohesion is fine; moving as a whole is the cheapest correct extraction.
- **Move `instrumentation.rs` entirely to `slicer-scheduler`**. Rejected: `PipelineInstrumentation` is a runtime-bracket-hook trait called from `layer_executor.rs` and `pipeline.rs` during execution. Moving it would force runtime → scheduler back-edge for the trait def (with bracket-hook signatures referencing `&StageId` and `&ModuleId` from `slicer-ir` — fine for scheduler, but the runtime callers would still need to define their own impl, which lives in runtime). The split (planning fns in scheduler, runtime trait + hooks in runtime) is the natural cut per the deep-dive on C.

## Code Change Surface

| File | Action | Notes |
|---|---|---|
| `crates/slicer-scheduler/Cargo.toml` | **CREATE** | Deps: `slicer-ir`, `slicer-schema`, `serde`, `toml`, `thiserror`. Add `slicer-sdk` IF dispatch #1 confirms any moved file imports SDK trait types. NO `wasmtime`. |
| `crates/slicer-scheduler/src/lib.rs` | **CREATE** | `pub mod` declarations for the nine moved files + `pub mod instrumentation;` for the split piece. Public re-exports mirror the pre-P85 `slicer-runtime/src/lib.rs` shape for these modules. |
| `crates/slicer-scheduler/src/manifest.rs` | **CREATE (from move)** | Verbatim from `crates/slicer-runtime/src/manifest.rs`. Internal `use crate::*` paths preserved. |
| `crates/slicer-scheduler/src/config_resolution.rs` | **CREATE (from move)** | Verbatim. |
| `crates/slicer-scheduler/src/dag.rs` | **CREATE (from move)** | Verbatim. Its `use crate::instrumentation::EdgeReason;` is now a same-crate import. |
| `crates/slicer-scheduler/src/validation.rs` | **CREATE (from move)** | Verbatim. |
| `crates/slicer-scheduler/src/execution_plan.rs` | **CREATE (from move + split)** | Move from `slicer-runtime/src/execution_plan.rs`. Move `CompiledModuleStatic` definition with it. **DELETE** the transitional `pub type CompiledModule = CompiledModuleStatic;` alias inherited from P83. |
| `crates/slicer-scheduler/src/topology.rs` | **CREATE (from move)** | Verbatim. |
| `crates/slicer-scheduler/src/stage_order.rs` | **CREATE (from move)** | Verbatim. |
| `crates/slicer-scheduler/src/module_search_path.rs` | **CREATE (from move)** | Verbatim. |
| `crates/slicer-scheduler/src/dag_cli.rs` | **CREATE (from move)** | Verbatim. |
| `crates/slicer-scheduler/src/instrumentation.rs` | **CREATE (from split)** | Contains: `compute_serial_edges_for_stage`, `EdgeReason`, `SerialEdge`. (Planning side ONLY.) |
| `crates/slicer-runtime/src/manifest.rs` | **DELETE** | |
| `crates/slicer-runtime/src/config_resolution.rs` | **DELETE** | |
| `crates/slicer-runtime/src/dag.rs` | **DELETE** | |
| `crates/slicer-runtime/src/validation.rs` | **DELETE** | |
| `crates/slicer-runtime/src/execution_plan.rs` | **DELETE** | |
| `crates/slicer-runtime/src/topology.rs` | **DELETE** | |
| `crates/slicer-runtime/src/stage_order.rs` | **DELETE** | |
| `crates/slicer-runtime/src/module_search_path.rs` | **DELETE** | |
| `crates/slicer-runtime/src/dag_cli.rs` | **DELETE** | |
| `crates/slicer-runtime/src/instrumentation.rs` | **EDIT (truncate to runtime side)** | Keep: `PipelineInstrumentation` trait, `Phase`, `TierKind`, `compute_serial_edges_from_compiled`, supporting types used by the runtime bracket hooks. Delete: `compute_serial_edges_for_stage`, `EdgeReason`, `SerialEdge` (moved to scheduler). Update `crate::*` imports where `EdgeReason`/`SerialEdge` are still referenced to `slicer_scheduler::*`. |
| `crates/slicer-runtime/src/lib.rs` | **EDIT** | Drop 9 `pub mod ...;` declarations. Keep `pub mod instrumentation;`. Update `runtime_builtins()` to reference `slicer_scheduler::*` where needed for `BuiltinProducer` trait paths. Add `pub use slicer_scheduler::{ExecutionPlan, CompiledModuleStatic, LoadedModule, ConfigSchema, build_execution_plan, build_live_execution_plan, load_modules_from_roots, validate_startup_dag, run_dag_stages, run_dag_stage, run_dag_depends, run_dag_claims, ...};` as a transitional compatibility re-export — list determined by inspecting what `slicer-runtime/tests/` consumers reference. (Listing for backward source compat; removing these re-exports is a future cleanup.) |
| `crates/slicer-runtime/Cargo.toml` | **EDIT** | Add `slicer-scheduler = { path = "../slicer-scheduler" }`. Drop `toml = "0.8"` if dispatch #2 confirms manifest.rs was its sole runtime consumer. |
| `crates/slicer-wasm-host/src/binding.rs` (or wherever `CompiledModuleLive<'s>` lives) | **EDIT** | Change the borrow type: `&'s slicer_runtime::CompiledModuleStatic` → `&'s slicer_scheduler::CompiledModuleStatic`. Add `use slicer_scheduler::CompiledModuleStatic;` if not already. |
| `crates/slicer-wasm-host/Cargo.toml` | **EDIT** | Add `slicer-scheduler = { path = "../slicer-scheduler" }`. |
| `crates/pnp-cli/Cargo.toml` | **EDIT** | Add `slicer-scheduler = { path = "../slicer-scheduler" }`. |
| `crates/pnp-cli/src/main.rs` (or the `dag` subcommand module) | **EDIT** | Change `use slicer_runtime::{run_dag_stages, ...};` to `use slicer_scheduler::{run_dag_stages, ...};`. |
| `crates/slicer-runtime/tests/**` | **EDIT or MOVE** | Tests whose SUT is a moved symbol (e.g., `build_execution_plan`, `validate_startup_dag`, `load_modules_from_roots`) move to `crates/slicer-scheduler/tests/`. Tests whose SUT is a runtime symbol consuming an `ExecutionPlan` stay but rewrite imports from `slicer_runtime::ExecutionPlan` to `slicer_scheduler::ExecutionPlan` (or rely on the lib.rs re-export). |

Primary edit target ≤ 3 files: the new `slicer-scheduler` crate (counted as one — 9 source files + 1 split file), `crates/slicer-runtime/src/lib.rs`, `crates/slicer-runtime/src/instrumentation.rs`. All other edits are mechanical follow-on.

## Files in Scope (read+edit)

The 26 files in the table above plus the conditional test files surfaced by dispatch #4.

## Read-Only Context

| File | Why | Hint |
|---|---|---|
| `crates/slicer-runtime/src/{manifest,config_resolution,dag,validation,execution_plan,topology,stage_order,module_search_path,dag_cli}.rs` | The nine moved files. NEVER load in full. | Read only their `use crate::*` lines (top of file) to confirm imports stay same-crate-relative; read `pub` surfaces only when verifying lib.rs re-exports. |
| `crates/slicer-runtime/src/instrumentation.rs` | Identify the planning-side fn (`compute_serial_edges_for_stage`, line ~78) and the runtime-side trait. | Line ranges around L1–30 (imports), L70–150 (planning fn), L190–250 (PipelineInstrumentation trait). Total ≤ 842 LOC; OK to read in line ranges. |
| `crates/slicer-runtime/src/execution_plan.rs` | Find `CompiledModuleStatic` (renamed in P83) and the `pub type CompiledModule = CompiledModuleStatic;` alias. | L640–730 in the post-P83 layout. |
| `crates/slicer-wasm-host/src/binding.rs` (or wherever P83 placed `CompiledModuleLive`) | Confirm the current borrow type to change. | Full file (small — ≤ 60 LOC). |
| `crates/slicer-runtime/src/lib.rs` | The current re-export shape determines the transitional re-export list. | Full file. |
| `crates/pnp-cli/src/main.rs` | Find the dag subcommand arms. | Grep for `run_dag_`. |
| `docs/04_host_scheduler.md` | Confirm the canonical pipeline / planning shape; no content change. | Delegate SUMMARY if > 300 LOC. |

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not consulted.
- `target/**`, `Cargo.lock` — never loaded.
- `crates/slicer-test/**`, `crates/slicer-sdk/**` — concurrent work.
- `crates/slicer-runtime/src/wit_host.rs`, `dispatch.rs`, `wasm_instance.rs`, `instance_pool.rs` — already in `slicer-wasm-host` (P83). Do not read.
- `crates/slicer-runtime/src/{model_loader,helpers_cmd,cli}.rs` — already gone (P81/P82).
- `crates/slicer-runtime/src/{mesh_analysis,paint_segmentation,prepass_slice,support_geometry,mesh_segmentation,overhang_classifier}.rs` — already gone (P84).
- `crates/slicer-runtime/src/region_mapping.rs` — P87 territory.
- `crates/slicer-runtime/src/gcode_emit.rs` — P86 territory.
- `crates/slicer-runtime/src/{blackboard,layer_executor,prepass,postpass,layer_finalization,pipeline,run}.rs` — read only their `use` lines to confirm rewrites.

## Expected Sub-Agent Dispatches

| # | Question | Scope | Return format |
|---|---|---|---|
| 1 | Do any of the nine moved files (`manifest`, `config_resolution`, `dag`, `validation`, `execution_plan`, `topology`, `stage_order`, `module_search_path`, `dag_cli`) import from `slicer_sdk`? If yes, which types? | The nine source files | FACT (yes/no + type list if yes) |
| 2 | Is `toml = "0.8"` used anywhere in `crates/slicer-runtime/src/` besides `manifest.rs`? | `crates/slicer-runtime/src/` | FACT (yes/no) |
| 3 | What `pub use slicer_runtime::*` re-export references exist in `crates/slicer-runtime/tests/`, `crates/pnp-cli/src/`, and elsewhere that name moved symbols (`ExecutionPlan`, `LoadedModule`, `CompiledModuleStatic`, `ConfigSchema`, `build_execution_plan`, `validate_startup_dag`, `run_dag_*`, etc.)? | Repo-wide | LOCATIONS (≤ 30 entries) |
| 4 | Which test files under `crates/slicer-runtime/tests/` have an SUT that is a moved symbol (manifest loading, DAG building, validation, execution-plan build)? | `crates/slicer-runtime/tests/` | LOCATIONS (≤ 30 entries) |
| 5 | What does `crates/slicer-wasm-host/src/binding.rs` (or the file holding `CompiledModuleLive`) currently look like? | The single file | SNIPPET (full file content if ≤ 60 LOC) |
| 6 | After move, `cargo build --workspace`. | repo root | FACT pass/fail + first failing crate |
| 7 | After move, `cargo clippy --workspace --all-targets -- -D warnings`. | repo root | FACT pass/fail |
| 8 | After move, `cargo xtask build-guests --check`. | repo root | FACT clean/STALE-list (expected clean since no guest-feeding path edited). |
| 9 | After move, `cargo test --workspace 2>&1 | tail -5`. | repo root | SNIPPET (count + final result) |
| 10 | Pre-packet baselines: g-code SHA + `pnp_cli dag stages` SHA + `pnp_cli dag claims` SHA. | repo root | FACT × 3 |
| 11 | Post-packet check of same three SHAs. | repo root | FACT × 3 (compare) |

## Data and Contract Notes

- `LoadedModule`, `ExecutionPlan`, `CompiledModuleStatic`, `ConfigSchema`, `DagValidationReport`, `Producer`, `BuiltinProducer`, and other planning types preserve their public shapes exactly. Only the crate path changes.
- `BuiltinProducer` is defined in `crates/slicer-runtime/src/dag.rs` today (a trait that the P84 wrappers implement). After P85, it lives in `crates/slicer-scheduler/src/dag.rs`. The P84 wrappers in `crates/slicer-runtime/src/builtins/` rewrite their `use crate::dag::BuiltinProducer;` to `use slicer_scheduler::dag::BuiltinProducer;` (or via re-export).
- `run_dag_*` fn signatures preserve exactly — `pnp_cli`'s subcommand handlers continue to work without arg changes.

## Locked Assumptions and Invariants

- ADR-0001 / 0002 / 0003 / 0004 / 0005 preserved.
- New invariant codified in ADR-0006 (at packet close): `slicer-scheduler` is wasmtime-free; `CompiledModuleLive` lives in `slicer-wasm-host` and borrows `CompiledModuleStatic` from scheduler. Future architecture reviews must not re-merge these.
- Byte-identical g-code: AC-9 SHA = P84 closure SHA.
- Byte-identical `pnp_cli dag` output: AC-10 SHAs = Step 0 baselines.
- `cargo xtask build-guests --check` stays clean throughout (no guest-feeding path is edited).

## Risks and Tradeoffs

- **Risk: a moved file references something the runtime still needs internally.** E.g., `dag.rs` references `crate::instrumentation::EdgeReason`; after move, that becomes a same-crate import in `slicer-scheduler`. But if `crate::execution_plan` or `dag.rs` indirectly references `Blackboard` or `WasmComponent` (a runtime/wasm-host type), the scheduler build breaks. Mitigation: dispatch #1 surfaces SDK imports; an early `cargo build -p slicer-scheduler` against the post-move tree catches any surprise.
- **Risk: the test re-exports in `slicer-runtime/src/lib.rs` are incomplete**, breaking tests that grep `slicer_runtime::X`. Mitigation: dispatch #3 enumerates external `pub use slicer_runtime::*` references; the transitional re-export list covers them.
- **Risk: `cargo test --workspace` flakes** (a known issue in large suites). Mitigation: dispatch #9 captures the tail output, so a flaky test can be re-run individually without re-doing the whole gate.
- **Tradeoff: the transitional `pub use slicer_scheduler::*` re-exports in `slicer-runtime/src/lib.rs`** keep test compatibility but inflate the runtime crate's apparent surface. They are explicitly transitional; a follow-up packet (P89 or similar in the doc-sweep phase) deletes them.

## Context Cost Estimate

- Aggregate: **L overall but no single step is L.** Total step count: 10.
- Largest single step: step 4 (the bulk move of nine files + the `CompiledModuleStatic` relocation + the instrumentation split), rated M. The implementer reads `use crate::*` lines only — file bodies are copied as-is without inspection.
- Highest-risk dispatch: dispatch #9 (`cargo test --workspace`) — a checkpoint gate. Dispatch hands back FACT pass/fail + duration; if fail, individual test names get re-dispatched.

## Open Questions

`None — change is reversible via reverting the move. The transitional re-export block in slicer-runtime/src/lib.rs is the rollback hatch.`

One ADR follow-up planned at packet close:

- **ADR-0006** — `CompiledModule` Static/Live split rationale (full version). Records why two types instead of one, why scheduler MUST NOT depend on wasm-host, what future architecture reviewers should not re-litigate.

# Packet 83 — Design

## Controlling Code Paths

```
new crate: slicer-wasm-host  ◄── runner traits, dispatcher impl, wit_host bindgens, instance pool
                              ◄── CompiledModuleLive<'s>
                              
slicer-runtime → slicer-wasm-host → wasmtime    (after this packet)
              ╲                  ╲
               └ (no longer)──────╲────►   (direct wasmtime dep removed)

slicer-schema gains:  pub fn export_for_stage_id(stage_id) -> Option<&'static str>
                      (single source of truth lookup; deletes dispatch::export_name_for_stage)

slicer-runtime::execution_plan:  CompiledModule → CompiledModuleStatic
                                 + pub type CompiledModule = CompiledModuleStatic
                                 (transitional alias; deleted in P85)
```

The four `bindgen!` invocations stay co-located in `slicer-wasm-host` so ADR-0002's `with:` remap continues to give all four worlds shared Rust type identity. The runner traits move to the new crate so `slicer-wasm-host` can implement them without depending on `slicer-runtime`.

OrcaSlicer comparison surface: none.

## Architecture Constraints

- **ADR-0002 preserved**: all four `bindgen!` invocations in one crate (`slicer-wasm-host`), with the layer world canonical and prepass/finalization/postpass using `with: { "slicer:types/geometry": super::layer::slicer::types::geometry, "slicer:config/config-types": super::layer::slicer::config::config_types }`. The `pub mod layer` declaration must precede the other three modules so the `super::layer::…` paths resolve.
- **ADR-0003 preserved**: guest-side WIT conversions remain per-world inside `#[slicer_module]` (untouched by this packet).
- **No new ADR conflicts**: this packet's two ADR follow-ups (ADR-0004, ADR-0005) are recorded at close; no existing ADR is contradicted.
- `slicer-runtime` MUST NOT regain a direct `wasmtime` dep after this packet. The build will compile without it; if any source file imports `use wasmtime::...` after the move, the move missed a site.
- **Borrow-struct pattern for trait inputs.** Runner trait signatures use `*StageInput<'_>` borrow structs defined in `slicer-wasm-host`, **not** raw `&Blackboard` / `&mut LayerArena`. This is the same pattern P87 uses for `RegionMappingPlanProjection<'a>` and is **NOT a new abstraction** — it is the established mechanism in this batch for decoupling a consumer crate from runtime-owned aggregate types. The four borrow structs (`LayerStageInput<'a>`, `PrepassStageInput<'a>`, `FinalizationInput<'a>`, `PostpassInput<'a>`) carry only the field-level borrows the dispatcher actually reads/writes; `Blackboard` and `LayerArena` themselves stay in `slicer-runtime` (unchanged). The orchestrator constructs the input struct at the call site in each executor file. Stage I/O types (`*StageOutput`, `*Error`) relocate to `slicer-ir` and `slicer-core` in Step 0.5 prework (three groups, see implementation-plan §Step 0.5) so the trait signatures compile from inside `slicer-wasm-host` with no back-edge dep on `slicer-runtime` (AC-N3).
- **Narrow runner errors.** Runner trait signatures return narrow error types (`PrepassRunnerError`) defined in `slicer-ir`, **NOT** the broader orchestrator-level error types (`PrepassExecutionError`). Broad errors stay in `slicer-runtime` with `From<NarrowError>` impls for conversion at the orchestrator call site (so the existing `?` operator handles the lift transparently). This matches **P86's `GCodeEmitError → PostpassError` pattern** and is the established batch idiom — **not a new abstraction**. The narrow type carries only the variants the wasm dispatcher actually constructs (`FatalModule`, `Blackboard`); the 7 built-in-producer variants of `PrepassExecutionError` (MeshAnalysis, RegionMapping, SupportGeometry, PaintSegmentation, Slice, ShellClassification, MissingRequiredPrepass) stay attached to their constructors in `slicer-runtime`.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

## Selected Approach

Direct move + manifest rewire + one schema fn addition + one struct split. No new abstractions, no new traits beyond the four runner traits being relocated (not redesigned).

Rejected alternatives:

- **Define a single `WasmDispatcher` super-trait collapsing the four runner traits.** Rejected: each runner trait has different inputs (Layer takes per-layer state, Prepass takes mesh-level state, Finalization takes the full layer collection, Postpass takes the assembled g-code IR). Collapsing forfeits type safety. The four-trait shape exists in the codebase today and is documented in `docs/04_host_scheduler.md`; keep it.
- **Keep `slicer-schema` as pure `&'static` data; put the `export_for_stage_id` lookup in `slicer-wasm-host`.** Rejected: the lookup is a planning-time concern (`dag_cli` calls it without instantiating any WASM). Putting it in wasm-host pulls a wasmtime dep into `dag_cli`. Schema is the natural home.
- **Skip the `CompiledModule` split now and revisit in P85.** Rejected: P85 needs the split to extract `slicer-scheduler` without wasmtime. Doing the split here is one extra ~50-LOC edit; doing it in P85 means P85 also has to touch `slicer-wasm-host` to extract the live half. Cleaner to do it now.

## Code Change Surface

| File | Action | Notes |
|---|---|---|
| `crates/slicer-wasm-host/Cargo.toml` | **CREATE** | Deps: `wasmtime = { workspace = true }`, `slicer-ir`, `slicer-schema`, `slicer-sdk` (all paths). NO `slicer-runtime` dep. |
| `crates/slicer-wasm-host/src/lib.rs` | **CREATE** | `pub mod host;` (wit_host content) + `pub mod dispatch;` + `pub mod instance;` (wasm_instance content) + `pub mod pool;` (instance_pool content) + `pub mod traits;` (the four runner traits) + `pub mod binding;` (`CompiledModuleLive`). Plus public re-exports. |
| `crates/slicer-wasm-host/src/host.rs` | **CREATE (from move)** | Content of `crates/slicer-runtime/src/wit_host.rs` verbatim — including all four `bindgen!` invocations. |
| `crates/slicer-wasm-host/src/dispatch.rs` | **CREATE (from move)** | Content of `crates/slicer-runtime/src/dispatch.rs` MINUS `export_name_for_stage` (deleted). Replace internal calls to `export_name_for_stage(...)` with `slicer_schema::export_for_stage_id(...)`. Trait impls updated to take `&CompiledModuleLive<'_>`. |
| `crates/slicer-wasm-host/src/instance.rs` | **CREATE (from move)** | Content of `crates/slicer-runtime/src/wasm_instance.rs` verbatim. |
| `crates/slicer-wasm-host/src/pool.rs` | **CREATE (from move)** | Content of `crates/slicer-runtime/src/instance_pool.rs` verbatim. |
| `crates/slicer-wasm-host/src/traits.rs` | **CREATE** | The four `pub trait *StageRunner` definitions lifted from `layer_executor.rs`, `prepass.rs`, `layer_finalization.rs`, `postpass.rs`. Signatures changed to take `&CompiledModuleLive<'_>` where they took `&CompiledModule`. |
| `crates/slicer-wasm-host/src/binding.rs` | **CREATE** | `pub struct CompiledModuleLive<'s> { pub stat: &'s slicer_runtime::CompiledModuleStatic, pub instance_pool: Arc<crate::pool::WasmInstancePool>, pub wasm_component: Arc<crate::instance::WasmComponent> }` with a constructor. |
| `crates/slicer-wasm-host/tests/export_lookup_tdd.rs` | **CREATE** | Iterates `slicer_schema::STAGES` and asserts `export_for_stage_id(s.stage_id) == Some(s.wit_export)`. Asserts `export_for_stage_id("NotAStage") == None`. |
| `crates/slicer-runtime/src/wit_host.rs` | **DELETE** | |
| `crates/slicer-runtime/src/dispatch.rs` | **DELETE** | |
| `crates/slicer-runtime/src/wasm_instance.rs` | **DELETE** | |
| `crates/slicer-runtime/src/instance_pool.rs` | **DELETE** | |
| `crates/slicer-runtime/src/lib.rs` | **EDIT** | Drop four `pub mod` declarations and matching `pub use ...::...;` re-exports. Add `pub use slicer_wasm_host::{LayerStageRunner, PrepassStageRunner, FinalizationStageRunner, PostpassStageRunner, WasmRuntimeDispatcher, WasmEngine, WasmComponent, WasmInstance, WasmInstancePool, WasmInstanceLease, HostExecutionContext, HostExecutionContextBuilder};` (re-exports for tests/external consumers that still grep `slicer_runtime::*` for these names). |
| `crates/slicer-runtime/src/layer_executor.rs` | **EDIT** | Delete the local `pub trait LayerStageRunner` declaration; add `use slicer_wasm_host::LayerStageRunner;`. |
| `crates/slicer-runtime/src/prepass.rs` | **EDIT** | Same for `PrepassStageRunner`. |
| `crates/slicer-runtime/src/postpass.rs` | **EDIT** | Same for `PostpassStageRunner`. |
| `crates/slicer-runtime/src/layer_finalization.rs` | **EDIT** | Same for `FinalizationStageRunner`. |
| `crates/slicer-runtime/src/execution_plan.rs` | **EDIT** | Rename `CompiledModule` → `CompiledModuleStatic`. Delete the `instance_pool` and `wasm_component` fields + their `pub fn` accessors. Add `pub type CompiledModule = CompiledModuleStatic;` (transitional). |
| `crates/slicer-runtime/Cargo.toml` | **EDIT** | Delete `wasmtime = { workspace = true }`. Add `slicer-wasm-host = { path = "../slicer-wasm-host" }`. |
| `crates/slicer-schema/src/lib.rs` | **EDIT** | Add `pub fn export_for_stage_id(stage_id: &str) -> Option<&'static str> { STAGES.iter().find(|s| s.stage_id == stage_id).map(|s| s.wit_export) }`. |
| `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` | **CREATE** | Unit test asserting the lookup is total over `STAGES` and `None` for unknown ids. |
| `Cargo.toml` (workspace) | **EDIT** | Add `"crates/slicer-wasm-host"` to `members`. |
| `crates/slicer-runtime/src/dag_cli.rs` | **EDIT (imports)** | If it references `export_name_for_stage`, switch to `slicer_schema::export_for_stage_id`. |
| `crates/slicer-runtime/tests/**` | **EDIT or DELETE** | Tests that import `crate::wit_host::*` or `crate::dispatch::*` must rewire to `slicer_wasm_host::*`. Tests that constructed `CompiledModule { instance_pool: …, wasm_component: …, … }` directly need rewriting to construct `CompiledModuleStatic` + a separate `CompiledModuleLive` borrow. |

Primary edit target ≤ 3 files: the new `slicer-wasm-host` crate (counted as one — ~9 source files but one logical unit), `crates/slicer-runtime/src/lib.rs`, `crates/slicer-runtime/src/execution_plan.rs`. All other edits are mechanical follow-on.

## Files in Scope (read+edit)

- The 20 files in the table above plus the conditional test files surfaced by dispatch #2.

## Read-Only Context

| File | Why | Hint |
|---|---|---|
| `crates/slicer-runtime/src/wit_host.rs` | Identify the four `bindgen!` blocks, the resource data structs, the `HostExecutionContext` definition. Move section-by-section. | Line ranges around L240–258 (layer bindgen), L313–326 (prepass), L490–502 (finalization), L513–525 (postpass). Use grep + line-range reads; NEVER load full 5 259 LOC. |
| `crates/slicer-runtime/src/dispatch.rs` | Identify `WasmRuntimeDispatcher` struct (~L340), the four `impl *StageRunner for WasmRuntimeDispatcher` blocks (~L2099, L2258, L2841, L3039), and `export_name_for_stage` (L47–67). | Targeted grep + ±40-line reads. |
| `crates/slicer-runtime/src/layer_executor.rs`, `prepass.rs`, `postpass.rs`, `layer_finalization.rs` | Find the `pub trait *StageRunner` declarations to lift. | Grep `^pub trait .*StageRunner` per file; read ±20 lines around the match. |
| `crates/slicer-runtime/src/execution_plan.rs` | Find `pub struct CompiledModule` (~L656) and its `pub fn instance_pool()` (L688) / `pub fn wasm_component()` (L713). | Read L650–730. |
| `crates/slicer-schema/src/lib.rs` | Confirm `STAGES` shape and add the new fn. | Full file (~390 LOC — OK to load). |
| `docs/adr/0002-wit-marshalling-type-unification.md` | Re-read in full before touching the `with:` remap. | 60 LOC. |
| `crates/slicer-runtime/Cargo.toml` | Confirm `wasmtime` direct dep entry; find the right block to remove. | Full file (~80 LOC after P82). |

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not consulted.
- `target/**`, `Cargo.lock` — never loaded.
- `crates/slicer-test/**`, `crates/slicer-sdk/**` — concurrent work (packet 78). Do not touch.
- `crates/slicer-runtime/test-guests/**` — guest fixtures; their sources are unchanged. Their `.wasm` artifacts WILL rebuild as a side-effect of the schema edit.
- `crates/slicer-runtime/src/{model_loader,helpers_cmd,cli}.rs` — already removed by P81/P82.
- `crates/slicer-runtime/src/blackboard.rs` — trait-import line only. Bodies untouched.
- `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs` — trait-import line **and** one call-site borrow-struct constructor (each file constructs the matching `*StageInput<'_>` at the existing dispatch call site). No body restructure beyond that.
- `modules/core-modules/**` — guest module sources unchanged. Their `.wasm` artifacts rebuild as a side-effect.

## Expected Sub-Agent Dispatches

| # | Question | Scope | Return format |
|---|---|---|---|
| 1 | Where exactly are the four `pub trait *StageRunner` declarations (file:line) and what are their full signatures? | `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs` | SNIPPETS (4 snippets, ≤ 30 lines each) |
| 2 | Which files under `crates/slicer-runtime/tests/` reference `wit_host::*`, `dispatch::*`, `wasm_instance::*`, `instance_pool::*`, or construct `CompiledModule` directly? | `crates/slicer-runtime/tests/` | LOCATIONS (≤ 30 entries) |
| 3 | Which files under `crates/slicer-runtime/src/` (besides the four moving) reference any of `wit_host::*`, `dispatch::*`, `wasm_instance::*`, `instance_pool::*`? | `crates/slicer-runtime/src/` | LOCATIONS (≤ 20 entries) |
| 4 | Are there callers of `dispatch::export_name_for_stage` outside `dispatch.rs` itself? | repo-wide | LOCATIONS (≤ 5 entries) |
| 5 | After move, `cargo build --workspace`. | repo root | FACT pass/fail + first failing crate |
| 6 | After move, `cargo xtask build-guests` (rebuild) followed by `cargo xtask build-guests --check`. | repo root | FACT pass/fail + STALE list if any |
| 7 | After move + guest rebuild, `cargo test --workspace`. | repo root | FACT pass/fail + duration + count delta vs baseline |
| 8 | After move, `cargo tree -p slicer-runtime --depth 1 --edges normal`. Does the output mention `wasmtime`? | repo root | FACT yes/no |
| 9 | Post-packet g-code SHA against `resources/benchy.stl`. | repo root | FACT `<hex>` |

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

## Context Cost Estimate

- Aggregate: **L overall but split into per-step M segments.** Total step count: 9. No single step rated L.
- Largest single step: step 4 (the actual file move + bindgen relocation + Cargo.toml rewire, M). The implementer reads section-by-section via grep + line-range; does NOT load any large file in full.
- Highest-risk dispatch: dispatch #6 (`cargo xtask build-guests` rebuild). If any guest fails to rebuild, the root cause is likely an unintended schema edit; investigate before retrying.

## Open Questions

None. **`None — change is reversible via reverting the move; the `pub type CompiledModule = CompiledModuleStatic` alias preserves backward source compat through P83.`**

Two ADR follow-ups planned at packet close:

- **ADR-0004** — Runner trait defs live with the dispatcher impl in `slicer-wasm-host`; dep direction is `slicer-runtime → slicer-wasm-host → wasmtime`.
- **ADR-0005** — `slicer_schema::export_for_stage_id` is the sole lookup; dispatchers must not hardcode their own copies.

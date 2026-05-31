# Packet 83 — Requirements

## Problem Statement

`slicer-runtime` today carries 8 900 LOC of WIT/wasm-component-model marshalling that has nothing to do with orchestration. `wit_host.rs` (5 259 LOC) is four `wasmtime::component::bindgen!` invocations plus the 16 host-trait impls that bridge generated WIT shapes to host data. `dispatch.rs` (3 148 LOC) is the `WasmRuntimeDispatcher` struct that implements four runner traits and routes stage IDs to the matching WIT export. `wasm_instance.rs` and `instance_pool.rs` wrap the `wasmtime::Engine`/`wasmtime::Component`/instance-pool plumbing. Five structural consequences hurt:

1. **`slicer-runtime` directly depends on `wasmtime`.** Any future crate that consumes the runtime pays for wasmtime even if it only wants the scheduler types — and `slicer-scheduler` (packet 85) cannot be a wasmtime-free crate while `CompiledModule` exposes `Arc<WasmComponent>` accessors.
2. **The dispatcher trait impls and the executor live in the same crate**, so unit tests for orchestration logic cannot mock the dispatcher cheaply.
3. **The stage→WIT-export name lookup is duplicated.** `dispatch.rs::export_name_for_stage` (lines 47–67) and `slicer-schema::STAGES[*].wit_export` carry the same data; the schema is the documented single source of truth (per its own docstring) but the dispatcher re-hardcodes it.
4. **The runner trait defs sit in the executor modules** (`layer_executor.rs`, `prepass.rs`, etc.), entangling "what the executor needs from a runner" with "what the executor does with the runner's output". Moving the impls out is impossible without first lifting the trait defs.
5. **`CompiledModule` exposes `pub fn instance_pool() → &Arc<WasmInstancePool>` and `pub fn wasm_component() → Option<&Arc<WasmComponent>>` accessors** (`execution_plan.rs:688, 713`). These bake wasmtime into the planning type, blocking C's clean extraction in P85.

The fix is one move, one consolidation, one split. Move `wit_host.rs` + `dispatch.rs` + `wasm_instance.rs` + `instance_pool.rs` + the four runner trait defs into `slicer-wasm-host`. Collapse `export_name_for_stage` into `slicer-schema::export_for_stage_id`. Split `CompiledModule` into `CompiledModuleStatic` (manifest-resolution shape, will move to `slicer-scheduler` in P85) and `CompiledModuleLive<'s>` (the wasmtime borrow that the runner traits consume).

## Grouped Task IDs

- **TASK-233** (new) — Extract WIT/wasm marshalling into `slicer-wasm-host`. Recorded under "Architecture Deepening Phase I" alongside TASK-231 (P81) and TASK-232 (P82).

## In Scope

- Create `crates/slicer-wasm-host/` with `Cargo.toml` declaring `wasmtime` (direct), `slicer-ir`, `slicer-schema`, `slicer-sdk` (path deps). NO dep on `slicer-runtime` (AC-N3).
- Move `crates/slicer-runtime/src/wit_host.rs` (5 259 LOC) into `crates/slicer-wasm-host/src/`. All four `bindgen!` invocations (layer, prepass, finalization, postpass) MUST be co-located in this crate; the layer world remains canonical with the other three using `with: { "slicer:types/geometry": super::layer::slicer::types::geometry, "slicer:config/config-types": super::layer::slicer::config::config_types }` per ADR-0002.
- Move `crates/slicer-runtime/src/dispatch.rs` (3 148 LOC), `crates/slicer-runtime/src/wasm_instance.rs` (299 LOC), `crates/slicer-runtime/src/instance_pool.rs` (182 LOC) into `crates/slicer-wasm-host/src/`. File layout may flatten or restructure.
- Lift the four runner trait definitions from their executor modules into `crates/slicer-wasm-host/src/`:
  - `LayerStageRunner` — currently in `crates/slicer-runtime/src/layer_executor.rs` (~line 177).
  - `PrepassStageRunner` — currently in `crates/slicer-runtime/src/prepass.rs` (~line 180).
  - `FinalizationStageRunner` — currently in `crates/slicer-runtime/src/layer_finalization.rs`.
  - `PostpassStageRunner` — currently in `crates/slicer-runtime/src/postpass.rs`.
- Update each executor file's imports to `use slicer_wasm_host::{LayerStageRunner, …}` (and similar). The executor function bodies are unchanged.
- Add `pub fn export_for_stage_id(stage_id: &str) -> Option<&'static str>` to `crates/slicer-schema/src/lib.rs`. Body: `STAGES.iter().find(|s| s.stage_id == stage_id).map(|s| s.wit_export)`. Add a TDD-style unit test (`crates/slicer-schema/tests/export_for_stage_id_tdd.rs`) iterating `STAGES` to confirm the lookup is total and unknown IDs return `None`.
- Delete `dispatch::export_name_for_stage`. All callers (in `slicer-wasm-host`, in `slicer-runtime::dag_cli`, in any test) switch to `slicer_schema::export_for_stage_id`.
- Split `CompiledModule` (`crates/slicer-runtime/src/execution_plan.rs:656`):
  - Rename the existing struct to `CompiledModuleStatic`. Drop the `instance_pool: Arc<WasmInstancePool>` and `wasm_component: Option<Arc<WasmComponent>>` fields. Delete the `pub fn instance_pool()` and `pub fn wasm_component()` accessors.
  - Add `pub type CompiledModule = CompiledModuleStatic;` as a transitional type alias (deleted in P85).
  - Declare `pub struct CompiledModuleLive<'s>` in `crates/slicer-wasm-host/src/binding.rs` (new file or co-located with dispatch) with fields `stat: &'s slicer_runtime::CompiledModuleStatic`, `instance_pool: Arc<WasmInstancePool>`, `wasm_component: Arc<WasmComponent>`. Provide a `pub fn new(stat: &'s _, pool: _, component: _) -> Self` constructor.
- Update the four runner traits' signatures to take `&CompiledModuleLive<'_>` instead of `&CompiledModule`. The dispatcher impl constructs `CompiledModuleLive` per call from the runtime-side execution plan + the per-call wasm bindings.
- Delete the moved `pub mod` declarations from `crates/slicer-runtime/src/lib.rs`. Re-export the trait names if any external test depends on them, via `pub use slicer_wasm_host::{LayerStageRunner, …}`.
- Update `crates/slicer-runtime/Cargo.toml`: drop the direct `wasmtime = { workspace = true }` line; add `slicer-wasm-host = { path = "../slicer-wasm-host" }`.
- Add the new crate to the workspace `Cargo.toml` `members` list.
- Rebuild guests with `cargo xtask build-guests` after editing `slicer-schema` (the new fn invalidates the guest dep). `--check` must pass after rebuild.

## Out of Scope

- `crates/slicer-test/` or `crates/slicer-sdk/` — concurrent work.
- Moving `CompiledModuleStatic` itself out of `slicer-runtime` — that is P85. P83 only renames + splits; the type lives in `slicer-runtime` for one more packet.
- Moving the rest of the planning crate (`manifest.rs`, `dag.rs`, `validation.rs`, `execution_plan.rs` beyond the `CompiledModule` split). P85.
- Touching the per-layer executor's body, the prepass/postpass orchestrators, or the blackboard. Only their `use` statements change.
- WIT contract changes in `crates/slicer-schema/wit/**`. None are needed; the `bindgen!` outputs are byte-identical because the WIT inputs are byte-identical.
- New dispatcher abstractions, new mock dispatchers (the trait defs themselves are the seam; a mock impl can be added in a later packet when a specific test calls for one).

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — WIT worlds, host-boundary enforcement, the documented `wit_export` ↔ stage mapping that `slicer-schema::STAGES` already canonicalises.
- `docs/04_host_scheduler.md` — runner-trait contract per stage family. Confirms the trait signatures and the per-call data flow.
- `docs/adr/0002-wit-marshalling-type-unification.md` (60 LOC) — read in full before any edit to a `bindgen!` block.
- `docs/adr/0003-macro-per-world-wit-conversions.md` (~40 LOC) — confirms guest side is untouched.
- `CLAUDE.md` §"WIT/Type Changes Checklist" and §"Guest WASM Staleness" — operational discipline for any edit reaching `slicer-schema`.

## Acceptance Summary

The acceptance contract is enumerated in `packet.spec.md` (AC-1..AC-11, AC-N1..AC-N3). Measurable refinements:

- **AC-9 — g-code parity**: the byte-identical g-code SHA carried forward from P81 closure. If the SHA diverges, the packet fails closure. Note: P83 rebuilds guest WASMs, which can shift their build metadata; the SHA assertion targets the `.gcode` output, not guest artifacts.
- **AC-11 — workspace test gate**: dispatched to a sub-agent that returns FACT pass/fail + duration. Implementation log captures pre/post test counts.
- **AC-N2 — schema lookup totality**: the test must iterate `STAGES.iter()` (not a hardcoded list) so that any future addition to `STAGES` automatically gets coverage.

## Verification Commands

| ID | Command | Delegation hint |
|---|---|---|
| AC-1 | `test -f crates/slicer-wasm-host/Cargo.toml && grep -qE '^wasmtime *=' crates/slicer-wasm-host/Cargo.toml && ! grep -qE '^wasmtime *=' crates/slicer-runtime/Cargo.toml` | FACT pass/fail |
| AC-2 | `test ! -f crates/slicer-runtime/src/wit_host.rs && test ! -f crates/slicer-runtime/src/dispatch.rs && test ! -f crates/slicer-runtime/src/wasm_instance.rs && test ! -f crates/slicer-runtime/src/instance_pool.rs` | FACT pass/fail |
| AC-3 | `[ $(grep -rE 'wasmtime::component::bindgen!' crates/slicer-wasm-host/src/ \| wc -l) -eq 4 ] && [ $(grep -rE '"slicer:types/geometry": super::layer::slicer::types::geometry' crates/slicer-wasm-host/src/ \| wc -l) -eq 3 ]` | FACT pass/fail |
| AC-4 | `[ $(grep -rE '^pub trait (Layer\|Prepass\|Finalization\|Postpass)StageRunner' crates/slicer-wasm-host/src/ \| wc -l) -eq 4 ]` | FACT pass/fail |
| AC-5 | `! grep -qE '^pub mod (wit_host\|dispatch\|wasm_instance\|instance_pool);' crates/slicer-runtime/src/lib.rs` | FACT pass/fail |
| AC-6 | `! grep -rqE 'pub fn export_name_for_stage' crates/ && grep -qE 'pub fn export_for_stage_id' crates/slicer-schema/src/lib.rs` | FACT pass/fail |
| AC-7 | `grep -rqE 'pub struct CompiledModuleLive' crates/slicer-wasm-host/src/ && grep -qE 'pub struct CompiledModuleStatic\|pub type CompiledModule = CompiledModuleStatic' crates/slicer-runtime/src/execution_plan.rs` | FACT pass/fail |
| AC-8 | `cargo tree -p slicer-runtime --depth 1 --edges normal 2>&1 \| grep -qE 'wasmtime'` (success = empty match) | FACT no-match/match |
| AC-9 | `cargo run --bin pnp_cli --release -- slice ... && sha256sum /tmp/benchy-p83.gcode` | SNIPPET (SHA) |
| AC-10 | `cargo test -p slicer-wasm-host -p slicer-runtime -p pnp-cli` | FACT pass/fail + counts |
| AC-11 | `cargo test --workspace` | FACT pass/fail + duration |
| AC-N1 | `rg 'use crate::(wit_host\|dispatch\|wasm_instance\|instance_pool)::' crates/slicer-runtime/src/` (success = empty) | FACT empty/non-empty |
| AC-N2 | `cargo test -p slicer-schema` | FACT pass/fail |
| AC-N3 | `! grep -qE '^slicer-runtime *=' crates/slicer-wasm-host/Cargo.toml` | FACT pass/fail |
| gate-1 | `cargo build --workspace` | FACT pass/fail |
| gate-2 | `cargo clippy --workspace --all-targets -- -D warnings` | FACT pass/fail |
| gate-3 | `cargo xtask build-guests` (rebuild) then `cargo xtask build-guests --check` | FACT pass/fail |

## Step Completion Expectations

- The four `bindgen!` invocations MUST land in one commit (or one logical landing point) so the workspace never builds in a half-extracted state where the layer-world types exist but the others remap into a non-existent path.
- Guest rebuild MUST happen after editing `slicer-schema` and BEFORE running `cargo test --workspace`. The implementation log records guest-rebuild duration and confirms `--check` clean post-rebuild.
- The `CompiledModule` → `CompiledModuleStatic` rename MUST land together with the `pub type CompiledModule = CompiledModuleStatic;` alias so callers do not break in an intermediate commit.
- `cargo test --workspace` is the closure gate; partial runs do not satisfy the deepening-batch policy at this checkpoint.

## Packet-Specific Context Discipline

- `wit_host.rs` is 5 259 LOC. **NEVER load in full.** Approach: identify section boundaries via grep for `bindgen!`, `impl ... for HostExecutionContext`, `pub struct`. Move section-by-section using line-range reads.
- `dispatch.rs` is 3 148 LOC. Same discipline. Identify the four runner trait impl blocks via grep (`impl LayerStageRunner for WasmRuntimeDispatcher`, etc.) and the `WasmRuntimeDispatcher` struct definition (~line 340).
- The four `bindgen!` invocations themselves are short (~15 lines each); they ARE OK to inspect in full because they carry the `with:` remap pattern this packet must preserve.
- `OrcaSlicerDocumented/` is irrelevant — this packet has no parity surface.

<!-- snippet: wasm-staleness -->
This packet edits `crates/slicer-schema/src/lib.rs` (adding `export_for_stage_id`), which CLAUDE.md §"Guest WASM Staleness" lists as a path that "invalidates every guest's bindgen output". Implementer MUST rebuild guests with `cargo xtask build-guests` (without `--check`) after the edit and BEFORE running host-integration tests. Then re-verify with `--check`. Stale-guest failures will look like unrelated test breakage but are caused by this edit.

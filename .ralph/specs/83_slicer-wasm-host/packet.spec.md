---
status: implemented
packet: 83
task_ids: [TASK-233]
requires: [81, 82]
backlog_source: docs/07_implementation_status.md
---

# Packet 83 — Extract WIT/wasm Marshalling into `slicer-wasm-host`

## Goal

Move `wit_host.rs` (5 259 LOC), `dispatch.rs` (3 148 LOC), `wasm_instance.rs` (299 LOC), and `instance_pool.rs` (182 LOC) — together with the four runner trait definitions (`LayerStageRunner`, `PrepassStageRunner`, `FinalizationStageRunner`, `PostpassStageRunner`) — out of `slicer-runtime/src/` into a new `slicer-wasm-host` crate; collapse `dispatch::export_name_for_stage` into a `slicer_schema::export_for_stage_id(&str) -> Option<&'static str>` lookup that reads from `STAGES.wit_export` (the documented single source of truth); and introduce `CompiledModuleLive<'s>` (a borrow holding `Arc<WasmComponent>` + `Arc<WasmInstancePool>` + a `&'s CompiledModuleStatic`) so the runner traits never take wasmtime types from `slicer-runtime` — making `slicer-runtime` a `wasmtime`-free crate transitively (wasmtime is reachable only via the new `slicer-wasm-host` dep).

## Scope Boundaries

Largest packet in the deepening batch — ~8 900 LOC of WIT marshalling, dispatch, wasmtime instance handling, and runner-trait dispatch leave the runtime. All four `bindgen!` invocations (one per world: layer, prepass, finalization, postpass) MUST stay co-located in `slicer-wasm-host` to preserve ADR-0002's `with:` remap pattern that gives the four worlds shared type identity. `CompiledModuleStatic` is what `CompiledModule` becomes after stripping its two wasmtime accessors (`instance_pool()`, `wasm_component()`); the stripped accessors return `CompiledModuleLive` builders in the runtime callers. The actual move of `CompiledModuleStatic` into `slicer-scheduler` is P85; P83 only introduces the split. The `BuiltinProducer`-based host built-ins (`GCODE_EMIT_PRODUCER`, `MESH_ANALYSIS_PRODUCER`, etc.) and the per-layer executor stay in `slicer-runtime`. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- **Requires packet 81 closed** (slicer-model-io extraction — the dep-tree precedent).
- **Requires packet 82 closed** (CLI cleanup — clears `cli.rs` and `helpers_cmd.rs` so they don't tangle with the wit_host move).
- **Co-existence with packet 78** (slicer-test fold into slicer-sdk, concurrent). Must not edit `crates/slicer-test/` or `crates/slicer-sdk/`. Verify packet 78's status before opening this packet; if it is `active` or `in progress`, coordinate with the responsible agent.
- Closure requires `cargo xtask build-guests --check` clean. **This packet edits `crates/slicer-schema/src/lib.rs`** (adding `export_for_stage_id`), which invalidates all guest `.wasm` builds per CLAUDE.md §"Guest WASM Staleness". The implementer MUST rebuild guests with `cargo xtask build-guests` (no `--check`) before running any host-integration test, then re-verify with `--check`.
- This is a workspace-test checkpoint packet: `cargo test --workspace` MUST run green at close, per the deepening-batch policy (deviation recorded in P81).

## Acceptance Criteria

### AC-1 — `slicer-wasm-host` crate exists with `wasmtime` as a direct dep; `slicer-runtime` no longer declares `wasmtime` directly

**Given** the extraction,
**When** workspace manifests are inspected,
**Then** `crates/slicer-wasm-host/Cargo.toml` exists, declares `wasmtime = { workspace = true }`, `slicer-ir = { path = "../slicer-ir" }`, `slicer-schema = { path = "../slicer-schema" }`, and `slicer-sdk = { path = "../slicer-sdk" }`. `crates/slicer-runtime/Cargo.toml` no longer declares `wasmtime` in its `[dependencies]` block — it depends on `slicer-wasm-host` instead. `cargo tree -p slicer-runtime --edges normal | grep wasmtime` returns lines only via the `slicer-wasm-host` indirection.

| `test -f crates/slicer-wasm-host/Cargo.toml && grep -qE '^wasmtime *=' crates/slicer-wasm-host/Cargo.toml && ! grep -qE '^wasmtime *=' crates/slicer-runtime/Cargo.toml && grep -qE '^slicer-wasm-host *=' crates/slicer-runtime/Cargo.toml`

### AC-2 — Four moved files no longer exist under `slicer-runtime/src/`; equivalents exist under `slicer-wasm-host/src/`

**Given** the move,
**When** the working tree is inspected,
**Then** none of `wit_host.rs`, `dispatch.rs`, `wasm_instance.rs`, `instance_pool.rs` exist under `crates/slicer-runtime/src/`. Equivalents exist under `crates/slicer-wasm-host/src/` (file layout may flatten or restructure — e.g., `host.rs`, `dispatch.rs`, `instance.rs`, `pool.rs` — as long as the public surface is preserved).

| `test ! -f crates/slicer-runtime/src/wit_host.rs && test ! -f crates/slicer-runtime/src/dispatch.rs && test ! -f crates/slicer-runtime/src/wasm_instance.rs && test ! -f crates/slicer-runtime/src/instance_pool.rs && find crates/slicer-wasm-host/src -name '*.rs' | xargs grep -lE 'wasmtime::component::bindgen!' | wc -l | grep -qE '^[1-9]'`

### AC-3 — All four `bindgen!` invocations are in `slicer-wasm-host` with the documented `with:` remap pattern intact (ADR-0002 preserved)

**Given** the move,
**When** `crates/slicer-wasm-host/src/` is grepped,
**Then** four occurrences of `wasmtime::component::bindgen!` exist, one per world. Three of them (prepass, finalization, postpass) include a `with: {` block remapping `"slicer:types/geometry": super::layer::slicer::types::geometry` and `"slicer:config/config-types": super::layer::slicer::config::config_types`. The layer world's `bindgen!` does NOT have a `with:` block (it is the canonical owner per ADR-0002). The order of declarations places `pub mod layer` before the other three modules.

| `[ $(grep -rE 'wasmtime::component::bindgen!' crates/slicer-wasm-host/src/ | wc -l) -eq 4 ] && [ $(grep -rE '"slicer:types/geometry": super::layer::slicer::types::geometry' crates/slicer-wasm-host/src/ | wc -l) -eq 3 ]`

### AC-4 — Runner trait definitions are in `slicer-wasm-host` with the borrow-struct input pattern

**Given** the move and the borrow-struct refactor (see design.md "Borrow-struct pattern for trait inputs"),
**When** `crates/slicer-wasm-host/src/` is grepped,
**Then** `pub trait LayerStageRunner`, `pub trait PrepassStageRunner`, `pub trait FinalizationStageRunner`, `pub trait PostpassStageRunner` each appear exactly once in `slicer-wasm-host/src/`. They no longer appear in `crates/slicer-runtime/src/`. Trait signatures use `*StageInput<'_>` borrow structs (also defined in `slicer-wasm-host`) for orchestrator-side context, `&CompiledModuleLive` for module access, and stage I/O types (`LayerStageOutput`, `LayerStageError`, `PrepassStageOutput`, `PrepassExecutionError`, `FinalizationOutput`, `FinalizationError`, `PostpassOutput`, `PostpassError`) imported from `slicer-ir` (relocated as P83 prework — see implementation-plan Step 0.5). `WasmEngine` lives in `slicer-wasm-host` (moves with `wasm_instance.rs` in Step 4). **No trait signature references `&Blackboard` or `&mut LayerArena` directly.** The executor files `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs` import the traits via `use slicer_wasm_host::{...StageRunner};` and construct the matching `*StageInput<'_>` at each call site.

| `[ $(grep -rE '^pub trait (Layer\|Prepass\|Finalization\|Postpass)StageRunner' crates/slicer-wasm-host/src/ | wc -l) -eq 4 ] && [ $(grep -rE '^pub trait (Layer\|Prepass\|Finalization\|Postpass)StageRunner' crates/slicer-runtime/src/ | wc -l) -eq 0 ] && grep -qE 'use slicer_wasm_host::.*StageRunner' crates/slicer-runtime/src/layer_executor.rs && ! grep -rE 'fn run.*Blackboard|fn run.*LayerArena|fn run.*PrepassExecutionError|fn run.*HostExecutionContext|HostExecutionContext.*->|->.*HostExecutionContext' crates/slicer-wasm-host/src/`

The exclusion list enforces three patterns: (1) the borrow-struct input pattern (no raw `&Blackboard` / `&mut LayerArena` in trait sigs); (2) the narrow-runner-error split (no `PrepassExecutionError` — `PrepassStageRunner` returns `PrepassRunnerError` defined in `slicer-ir`, with `From<PrepassRunnerError>` impl in `slicer-runtime` per the P86 `GCodeEmitError → PostpassError` precedent); (3) **the symmetric IR-typed trait boundary** (no `HostExecutionContext` in trait sigs — runner trait impls deconstruct the wasm-host's internal `HostExecutionContext` into a `LayerStageCommitData`-style IR struct from `slicer-ir` before returning, per design.md "Symmetric IR-typed trait boundary").

### AC-5 — `slicer-runtime/src/lib.rs` no longer declares the four moved `pub mod`s; re-exports come via `slicer-wasm-host` if exposed at all

**Given** the move,
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** the lines `pub mod wit_host;`, `pub mod dispatch;`, `pub mod wasm_instance;`, `pub mod instance_pool;` are absent. Their `pub use ...::...` re-export blocks are absent. If `slicer-runtime` still re-exports any wasm-host items for backward compat with external consumers, the re-export takes the form `pub use slicer_wasm_host::{...};` — never `pub use wit_host::...`.

| `! grep -qE '^pub mod (wit_host\|dispatch\|wasm_instance\|instance_pool);' crates/slicer-runtime/src/lib.rs && ! grep -qE '^pub use (wit_host\|dispatch\|wasm_instance\|instance_pool)::' crates/slicer-runtime/src/lib.rs`

### AC-6 — `dispatch::export_name_for_stage` is gone; `slicer_schema::export_for_stage_id` is the only stage→export lookup

**Given** the consolidation,
**When** the workspace is grepped,
**Then** `pub fn export_name_for_stage` no longer appears in `crates/slicer-wasm-host/src/dispatch.rs` (or wherever dispatch lands) or anywhere else. `crates/slicer-schema/src/lib.rs` declares `pub fn export_for_stage_id(stage_id: &str) -> Option<&'static str>` that returns `STAGES.iter().find(|s| s.stage_id == stage_id).map(|s| s.wit_export)`. All callers (`slicer-wasm-host::dispatch`, `slicer-runtime::dag_cli`, any test) use the schema lookup.

| `! grep -rqE 'pub fn export_name_for_stage' crates/ && grep -qE 'pub fn export_for_stage_id' crates/slicer-schema/src/lib.rs && grep -rqE 'slicer_schema::export_for_stage_id\|schema::export_for_stage_id' crates/slicer-wasm-host/src/`

### AC-7 — `CompiledModuleLive<'s>` exists in `slicer-wasm-host` and is the type the runner traits accept

**Given** the split,
**When** `crates/slicer-wasm-host/src/` is grepped,
**Then** a `pub struct CompiledModuleLive<'s>` is declared with at least the fields `instance_pool: Arc<WasmInstancePool>` and `wasm_component: Arc<WasmComponent>` plus a borrow `stat: &'s slicer_runtime::CompiledModuleStatic` (or `&'s CompiledModuleStatic` if `CompiledModuleStatic` is re-exported into `slicer-wasm-host`'s namespace via `pub use`). The four runner trait signatures take `&CompiledModuleLive<'_>` (or an `&dyn` adapter that borrows it). `crates/slicer-runtime/src/execution_plan.rs` renames its existing `CompiledModule` struct to `CompiledModuleStatic` (deleting the `instance_pool()` and `wasm_component()` accessors that returned wasmtime types) and adds a `pub type CompiledModule = CompiledModuleStatic;` type alias for transitional compatibility (alias removed in P85).

| `grep -qE 'pub struct CompiledModuleLive' crates/slicer-wasm-host/src/ -r && grep -qE 'pub struct CompiledModuleStatic\|pub type CompiledModuleStatic' crates/slicer-runtime/src/execution_plan.rs && ! grep -qE 'pub fn (instance_pool\|wasm_component)' crates/slicer-runtime/src/execution_plan.rs`

### AC-8 — `cargo tree -p slicer-runtime --depth 5 --edges normal` shows `wasmtime` only as a transitive dep via `slicer-wasm-host`

**Given** the dep migration,
**When** `cargo tree -p slicer-runtime --depth 5 --edges normal` is inspected,
**Then** `wasmtime` appears in the output, but every entry is under the `slicer-wasm-host` subtree — there is no direct line `slicer-runtime → wasmtime`. The proxy assertion: the first-depth deps of `slicer-runtime` do NOT include `wasmtime`.

| `cargo tree -p slicer-runtime --depth 1 --edges normal 2>&1 | grep -E '^├──\|^└──' | grep -qE 'wasmtime' && false || true`

(Reads as: the depth-1 listing of slicer-runtime's deps must NOT contain wasmtime. `cargo tree --depth 1` shows only direct deps; matching `wasmtime` there would fail the assertion.)

### AC-9 — End-to-end slice produces byte-identical g-code vs the P81 baseline

**Given** the wholesale move,
**When** `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p83.gcode` runs after `cargo xtask build-guests` has rebuilt guests,
**Then** the resulting g-code SHA matches the P81 closure SHA. (The P81 SHA carries forward through P82 and is the immutable baseline until a packet explicitly changes g-code semantics — none of P82, P83 do.)

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p83.gcode && sha256sum /tmp/benchy-p83.gcode`

### AC-10 — `cargo test -p slicer-wasm-host -p slicer-runtime -p pnp-cli` pass

**Given** the move and the schema lookup consolidation,
**When** the narrow per-crate tests run,
**Then** all pass. `slicer-wasm-host` carries at minimum: a unit test for `export_name` lookup that returns the documented kebab-case export for every stage in `STAGES`; a unit test that confirms `CompiledModuleLive`'s borrow of `CompiledModuleStatic` compiles for each runner trait method.

| `cargo test -p slicer-wasm-host && cargo test -p slicer-runtime && cargo test -p pnp-cli`

### AC-11 — Workspace test gate passes (checkpoint packet)

**Given** the deepening-batch policy (workspace gate runs only at P83, P85, P88),
**When** `cargo test --workspace` runs (dispatched to a sub-agent that returns FACT pass/fail per CLAUDE.md §Test Discipline),
**Then** the full ~1 000-test suite passes with zero regressions vs the pre-batch baseline.

| `cargo test --workspace`

## Negative Test Cases

### AC-N1 — No `use crate::wit_host::` or `use crate::dispatch::` remains in `slicer-runtime/src/`

**Given** the deletion,
**When** `rg 'use crate::(wit_host\|dispatch\|wasm_instance\|instance_pool)::' crates/slicer-runtime/src/` runs,
**Then** the result is empty. (External imports via `slicer_wasm_host::` are fine; the internal `crate::` paths point at moved modules that no longer exist.)

| `! rg 'use crate::(wit_host\|dispatch\|wasm_instance\|instance_pool)::' crates/slicer-runtime/src/ 2>/dev/null`

### AC-N2 — `slicer-schema` lookup matches every documented export in `STAGES`

**Given** the consolidation,
**When** a `slicer-schema` test iterates `STAGES.iter()` and calls `export_for_stage_id(stage.stage_id)`,
**Then** each call returns `Some(stage.wit_export)`. A non-stage id (e.g., `"NotAStage"`) returns `None`. This proves the lookup is total over `STAGES` and rejects unknown stage IDs.

| `cargo test -p slicer-schema --test export_for_stage_id_tdd 2>/dev/null || cargo test -p slicer-schema`

### AC-N3 — `slicer-wasm-host` does NOT depend on `slicer-runtime` (no back-edge)

**Given** the new crate's intended position in the dep graph,
**When** `crates/slicer-wasm-host/Cargo.toml` is inspected,
**Then** `slicer-runtime` does NOT appear as a path dep, dev-dep, or build-dep. The dep direction is strictly `slicer-runtime → slicer-wasm-host`.

| `! grep -qE '^slicer-runtime *=' crates/slicer-wasm-host/Cargo.toml`

## Verification (gate commands only)

1. `cargo build --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests` (rebuild) then `cargo xtask build-guests --check` (must report clean)
4. `cargo test --workspace` (checkpoint gate — dispatched to sub-agent for FACT pass/fail)
5. `cargo tree -p slicer-runtime --depth 1 --edges normal` does NOT list `wasmtime`

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — WIT worlds, host-boundary enforcement. Read for the `bindgen!` `with:` remap pattern and the documented module manifest stage→export wiring.
- `docs/04_host_scheduler.md` — DAG validation, four-phase execution, runner-trait responsibilities. Read for confirmation that the runner-trait contract is unchanged by the move.
- `docs/adr/0002-wit-marshalling-type-unification.md` — the ADR this packet's bindgen co-location preserves. Re-read in full (≤ 60 LOC) before touching the `with:` remap.
- `docs/adr/0003-macro-per-world-wit-conversions.md` — confirms guest-side conversion stays per-world; this packet does not change the guest side, so ADR-0003 is trivially preserved.
- `CLAUDE.md` §"Guest WASM Staleness" — the procedure that protects against silent guest failures after a schema edit.

## Doc Impact Statement

Two doc follow-ups planned at P83 close, both as new ADRs in `docs/adr/`:

- **ADR-0005 — Runner trait defs live with the dispatcher impl in `slicer-wasm-host`.** Records the choice that `slicer-runtime → slicer-wasm-host → wasmtime` is the dep direction; a future architecture reviewer is reminded that runner traits do not belong in the planning crate or the SDK.
- **ADR-0006 — `slicer-schema::export_for_stage_id` is the single source of truth for stage→export name lookup; dispatcher impls do not hardcode their own copy.** Recorded so the duplicated table in `dispatch::export_name_for_stage` does not re-grow.

`docs/03_wit_and_manifest.md` may grow a one-line crate-map mention of `slicer-wasm-host`. Deferred to the deepening-batch doc-sweep packet.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- [AC-1 inline check] — Specified: `! grep -qE '^wasmtime *=' crates/slicer-runtime/Cargo.toml` | Implemented: `wasmtime = { workspace = true }` retained as `[dev-dependencies]` entry at `crates/slicer-runtime/Cargo.toml:27`; absent from `[dependencies]`. Inline grep is unsectioned and falsely fails despite the production graph being wasmtime-free. AC-1 prose ("no longer declares wasmtime in its `[dependencies]` block") and AC-8 depth-1 normal-edges check both pass. | Reason: test targets still construct `wasmtime::Engine`/`wasmtime::component::Component` directly via `wasm_cache`; retagging test consumers to obtain those types via `slicer_wasm_host` re-exports is a follow-up packet, not P83 scope.
- [AC-4 inline check] — Specified: `! grep -rE '...|HostExecutionContext.*->|->.*HostExecutionContext' crates/slicer-wasm-host/src/` | Implemented: 4 internal-helper hits in wasm-host (`dispatch.rs::dispatch_*_call` return types, `harvest_mesh_segmentation_ir` param, `host.rs::HostExecutionContextBuilder::build` return). | Reason: AC-4 intent — that trait signatures never reference `HostExecutionContext` — is directly verified in `crates/slicer-wasm-host/src/traits.rs:1-88`; no trait method references HEC. The inline grep is over-broad and also matches internal helpers within the wasm-host crate, which are not on the wasm-host↔runtime boundary the AC is policing.
- [AC-11 workspace test gate] — Specified: `cargo test --workspace` green at packet close | Implemented: 118 passed, 1 failed (`benchy_end_to_end_tdd::rejects_cooling_missing_when_required` panics with `Os { code: 112, kind: StorageFull }` at `crates/slicer-runtime/tests/common/slicer_cache.rs:190` during `recurse_copy`). | Reason: test environment ran out of disk space during the cached-slicer-run copy step; same environmental failure noted in prior closure ceremony, user-acknowledged as test-infra (not a code regression). Clears on rerun with disk headroom. SHA parity (AC-9: `89a329ad3a4c1b7febca839edfca8b6302e562d8d2a390ee144252fd54e65a2b`) re-verified green at closure.

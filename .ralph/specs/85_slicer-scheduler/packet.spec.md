---
status: draft
packet: 85
task_ids: [TASK-235]
requires: [83]
backlog_source: docs/07_implementation_status.md
---

# Packet 85 — Extract the Static Plan into `slicer-scheduler`

## Goal

Move the static-planning subsystem (`manifest.rs` + `config_resolution.rs` + `dag.rs` + `validation.rs` + `execution_plan.rs` + `topology.rs` + `stage_order.rs` + `module_search_path.rs` + `dag_cli.rs`, plus the *planning-side* of `instrumentation.rs` — `compute_serial_edges_for_stage`, `EdgeReason`, `SerialEdge` — together ~5 500 LOC) out of `slicer-runtime/src/` into a new `slicer-scheduler` crate; complete the `CompiledModule` split begun in P83 by relocating `CompiledModuleStatic` to the new crate and deleting the `pub type CompiledModule = CompiledModuleStatic` transitional alias; update `slicer-wasm-host::binding::CompiledModuleLive<'s>` to borrow `&'s slicer_scheduler::CompiledModuleStatic` so `slicer-scheduler` itself links zero `wasmtime` code.

## Scope Boundaries

This is the largest dep-graph restructuring in the deepening batch: `slicer-scheduler` becomes the third leaf-ish host crate (alongside `slicer-wasm-host` and `slicer-gcode`) below `slicer-runtime`. The new crate's `[dependencies]` block contains `slicer-ir`, `slicer-schema`, and (if any planning code uses SDK trait types) `slicer-sdk` — but NOT `wasmtime`, NOT `slicer-wasm-host`. `slicer-wasm-host` gains a `slicer-scheduler` dep (one-way: it borrows the Static half of the split). `slicer-runtime` keeps the *runtime-side* of `instrumentation.rs` (`PipelineInstrumentation` trait, `Phase`, `TierKind`, `compute_serial_edges_from_compiled`); the planning-side functions move. `pnp-cli` adds a `slicer-scheduler` dep so its `dag` subcommands import directly. `region_mapping.rs` (P87) and `gcode_emit.rs` (P86) stay in `slicer-runtime` for this packet. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- **Requires packet 83 closed**: `CompiledModuleStatic` was renamed in `execution_plan.rs` and `CompiledModuleLive<'s>` declared in `slicer-wasm-host`. P85 completes the split by moving `CompiledModuleStatic` out of `slicer-runtime`.
- Packet 84 is NOT a prerequisite — P85 does not touch the moved algorithm crates. P84 and P85 are independent moves, but the plan orders P84 first to keep P85's diff smaller (fewer overlapping touch sites).
- Closure requires `cargo xtask build-guests --check` clean. This packet edits no `slicer-ir` / `slicer-sdk` / `slicer-schema` / `slicer-macros` paths in their guest-feeding form — only creates a new host crate. Guests should stay clean without rebuild. If they go STALE, investigate the root cause; do not paper over.
- **Workspace-test checkpoint packet**: `cargo test --workspace` MUST run green at close.

## Acceptance Criteria

### AC-1 — `slicer-scheduler` crate exists with `slicer-ir`, `slicer-schema` (+ `slicer-sdk` if needed) as path deps, and ZERO `wasmtime`-related deps direct or transitive

**Given** the extraction,
**When** workspace manifests and dep tree are inspected,
**Then** `crates/slicer-scheduler/Cargo.toml` exists, declares `slicer-ir = { path = "../slicer-ir" }` and `slicer-schema = { path = "../slicer-schema" }` (and optionally `slicer-sdk` if any moved file imports an SDK type). It does NOT declare `wasmtime`, `slicer-wasm-host`, `slicer-runtime`, `slicer-helpers`, `slicer-core`, `slicer-gcode`, `slicer-model-io` as path deps. `cargo tree -p slicer-scheduler --edges normal | grep wasmtime` returns empty.

| `test -f crates/slicer-scheduler/Cargo.toml && grep -qE '^slicer-ir *=' crates/slicer-scheduler/Cargo.toml && grep -qE '^slicer-schema *=' crates/slicer-scheduler/Cargo.toml && ! grep -qE '^(wasmtime\|slicer-wasm-host\|slicer-runtime\|slicer-helpers\|slicer-core\|slicer-gcode\|slicer-model-io) *=' crates/slicer-scheduler/Cargo.toml && ! cargo tree -p slicer-scheduler --edges normal 2>&1 | grep -qE 'wasmtime'`

### AC-2 — Nine planning-subsystem files moved; equivalents under `crates/slicer-scheduler/src/`

**Given** the moves,
**When** the working tree is inspected,
**Then** none of `manifest.rs`, `config_resolution.rs`, `dag.rs`, `validation.rs`, `execution_plan.rs`, `topology.rs`, `stage_order.rs`, `module_search_path.rs`, `dag_cli.rs` exist under `crates/slicer-runtime/src/`. Equivalents exist under `crates/slicer-scheduler/src/` (file layout flexible — may flatten or restructure into subdirs).

| `for f in manifest config_resolution dag validation execution_plan topology stage_order module_search_path dag_cli; do test ! -f crates/slicer-runtime/src/$f.rs || exit 1; done && [ $(find crates/slicer-scheduler/src -name '*.rs' | wc -l) -ge 9 ]`

### AC-3 — `instrumentation.rs` is split: planning side in `slicer-scheduler`, runtime side stays

**Given** the planning/runtime split,
**When** both crates are grepped,
**Then** `compute_serial_edges_for_stage`, `EdgeReason`, `SerialEdge` are defined under `crates/slicer-scheduler/src/` and NOT under `crates/slicer-runtime/src/`. `PipelineInstrumentation` (trait), `Phase`, `TierKind`, `compute_serial_edges_from_compiled` are defined under `crates/slicer-runtime/src/` and NOT under `crates/slicer-scheduler/src/`. `slicer-runtime`'s `crate::instrumentation::EdgeReason` etc. imports become `slicer_scheduler::EdgeReason`.

| `[ $(rg -l 'pub fn compute_serial_edges_for_stage' crates/ | wc -l) -eq 1 ] && rg -l 'pub fn compute_serial_edges_for_stage' crates/ | grep -qE '^crates/slicer-scheduler/' && [ $(rg -l 'pub trait PipelineInstrumentation' crates/ | wc -l) -eq 1 ] && rg -l 'pub trait PipelineInstrumentation' crates/ | grep -qE '^crates/slicer-runtime/'`

### AC-4 — `CompiledModuleStatic` lives in `slicer-scheduler`; the transitional `pub type CompiledModule = CompiledModuleStatic` alias is gone

**Given** the completed split,
**When** the workspace is grepped,
**Then** `pub struct CompiledModuleStatic` appears once and that occurrence is under `crates/slicer-scheduler/src/`. The transitional `pub type CompiledModule = CompiledModuleStatic;` alias (added in P83) is gone everywhere. External callers reference `slicer_scheduler::CompiledModuleStatic` directly.

| `[ $(rg -l 'pub struct CompiledModuleStatic' crates/ | wc -l) -eq 1 ] && rg -l 'pub struct CompiledModuleStatic' crates/ | grep -qE '^crates/slicer-scheduler/' && ! rg -q 'pub type CompiledModule = CompiledModuleStatic' crates/`

### AC-5 — `slicer-wasm-host::binding::CompiledModuleLive<'s>` borrows `&'s slicer_scheduler::CompiledModuleStatic`

**Given** the move,
**When** `crates/slicer-wasm-host/src/binding.rs` (or wherever `CompiledModuleLive` lives) is read,
**Then** the borrowed field type is `&'s slicer_scheduler::CompiledModuleStatic` (or `&'s CompiledModuleStatic` with a top-level `use slicer_scheduler::CompiledModuleStatic;`). `crates/slicer-wasm-host/Cargo.toml` declares `slicer-scheduler = { path = "../slicer-scheduler" }`.

| `grep -rqE '&[a-z'\'']+ +(slicer_scheduler::)?CompiledModuleStatic' crates/slicer-wasm-host/src/ && grep -qE '^slicer-scheduler *=' crates/slicer-wasm-host/Cargo.toml`

### AC-6 — `pnp-cli`'s `dag` subcommands import `run_dag_*` from `slicer-scheduler`, not from `slicer-runtime`

**Given** the `dag_cli.rs` relocation,
**When** `crates/pnp-cli/src/` is grepped,
**Then** the `dag` subcommand bodies import via `use slicer_scheduler::{run_dag_stages, run_dag_stage, run_dag_depends, run_dag_claims, ...};`. NO `pnp-cli` source imports any `dag_*` symbol from `slicer_runtime`. `crates/pnp-cli/Cargo.toml` declares `slicer-scheduler = { path = "../slicer-scheduler" }`.

| `grep -rqE 'use slicer_scheduler::.*run_dag' crates/pnp-cli/src/ && ! grep -rqE 'use slicer_runtime::.*run_dag' crates/pnp-cli/src/ && grep -qE '^slicer-scheduler *=' crates/pnp-cli/Cargo.toml`

### AC-7 — `slicer-runtime/src/lib.rs` no longer declares any of the nine moved `pub mod`s

**Given** the move,
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** none of these lines exist: `pub mod manifest;`, `pub mod config_resolution;`, `pub mod dag;`, `pub mod validation;`, `pub mod execution_plan;`, `pub mod topology;`, `pub mod stage_order;`, `pub mod module_search_path;`, `pub mod dag_cli;`. The `instrumentation` module declaration IS preserved (it still hosts the runtime-side types per AC-3).

| `! grep -qE '^pub mod (manifest\|config_resolution\|dag\|validation\|execution_plan\|topology\|stage_order\|module_search_path\|dag_cli);' crates/slicer-runtime/src/lib.rs && grep -qE '^pub mod instrumentation;' crates/slicer-runtime/src/lib.rs`

### AC-8 — `slicer-runtime/Cargo.toml` declares `slicer-scheduler` as a dep

**Given** the dep graph adjustment,
**When** `crates/slicer-runtime/Cargo.toml` is read,
**Then** `slicer-scheduler = { path = "../slicer-scheduler" }` appears in `[dependencies]`. The unused-by-runtime deps that came from the moved files (e.g., `toml` if manifest.rs was its sole consumer) are dropped from the runtime manifest IF and only if they have no other runtime consumer.

| `grep -qE '^slicer-scheduler *=' crates/slicer-runtime/Cargo.toml`

### AC-9 — End-to-end slice produces byte-identical g-code vs the P84 baseline SHA

**Given** the move,
**When** `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p85.gcode` runs,
**Then** the resulting SHA matches the P84 closure SHA. (Behavior is unchanged; the planning crate now lives in a different namespace but produces the same `ExecutionPlan` and runs the same stages.)

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p85.gcode && sha256sum /tmp/benchy-p85.gcode`

### AC-10 — `pnp_cli dag stages` and `pnp_cli dag claims` still produce identical output

**Given** `dag_cli.rs` moved,
**When** `pnp_cli dag stages` and `pnp_cli dag claims` run against the canonical module dir,
**Then** the output of each is byte-identical to the pre-P85 baseline (captured in Step 0). The implementation log records both pre/post outputs.

| `cargo run --bin pnp_cli --release -- dag stages --module-dir modules/core-modules > /tmp/p85-dag-stages.txt && cargo run --bin pnp_cli --release -- dag claims --module-dir modules/core-modules > /tmp/p85-dag-claims.txt`

### AC-11 — Workspace test gate passes (checkpoint packet)

**Given** the deepening-batch policy,
**When** `cargo test --workspace` runs (dispatched to a sub-agent that returns FACT pass/fail per CLAUDE.md §Test Discipline),
**Then** the full suite passes with zero regressions vs the P84 baseline count.

| `cargo test --workspace`

## Negative Test Cases

### AC-N1 — `slicer-scheduler` does NOT depend on `slicer-runtime` or `slicer-wasm-host` (planning is downstream of types-only)

**Given** the dep direction invariant,
**When** `crates/slicer-scheduler/Cargo.toml` is read,
**Then** neither `slicer-runtime` nor `slicer-wasm-host` appears in `[dependencies]`, `[dev-dependencies]`, or `[build-dependencies]`. The scheduler is upstream of both.

| `! grep -qE '^slicer-(runtime\|wasm-host) *=' crates/slicer-scheduler/Cargo.toml`

### AC-N2 — `cargo tree -p slicer-scheduler --edges normal` shows no `wasmtime` transitively

**Given** the wasmtime-free scheduler invariant,
**When** the dep tree is inspected,
**Then** the output contains no `wasmtime` entry at any depth. (This is the architectural win of the CompiledModule Static/Live split — `slicer-scheduler` tests link zero wasmtime.)

| `! cargo tree -p slicer-scheduler 2>&1 | grep -qE '\bwasmtime\b'`

### AC-N3 — `slicer-runtime/src/` no longer contains `manifest.rs` / `dag.rs` / `validation.rs` / `execution_plan.rs`

**Given** the move,
**When** the source tree is inspected,
**Then** the four largest planning files are absent from `slicer-runtime/src/`. (Negative form of AC-2 for the highest-leverage file subset.)

| `! ls crates/slicer-runtime/src/manifest.rs crates/slicer-runtime/src/dag.rs crates/slicer-runtime/src/validation.rs crates/slicer-runtime/src/execution_plan.rs 2>/dev/null | grep -q .`

## Verification (gate commands only)

1. `cargo build --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests --check` (must stay clean — this packet edits no guest-feeding paths)
4. `cargo test --workspace` (checkpoint gate — dispatched to sub-agent)
5. `cargo tree -p slicer-scheduler --edges normal` does NOT contain `wasmtime`

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/04_host_scheduler.md` — DAG validation, four-phase execution. Confirms what the moved code does; no contract change.
- `docs/03_wit_and_manifest.md` — manifest TOML schema, config validation. Confirms what `manifest.rs` and `config_resolution.rs` consume; no contract change.
- `docs/adr/0002-wit-marshalling-type-unification.md` — confirms the boundary between WIT-aware code (stays in `slicer-wasm-host`) and planning-time-only code (moves to `slicer-scheduler`).
- `CLAUDE.md` §"Test Discipline" — workspace-test dispatch contract for the checkpoint gate.

## Doc Impact Statement

One ADR planned at packet close:

- **ADR-0006 — `CompiledModule` splits Static (scheduler) and Live (wasm-host) to keep `wasmtime` out of the planning crate.** Completes the structural decision begun in P83 (where the split was introduced as a transitional state in `slicer-runtime`). This ADR records the *full* rationale: why two types instead of one, why scheduler MUST NOT depend on wasm-host, what future architecture reviewers should not re-litigate.

`docs/04_host_scheduler.md` may grow a one-line crate-map note for `slicer-scheduler`. Deferred to the deepening-batch doc-sweep packet.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

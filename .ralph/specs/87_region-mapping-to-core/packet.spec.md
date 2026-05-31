---
status: draft
packet: 87
task_ids: [TASK-237]
requires: [85]
backlog_source: docs/07_implementation_status.md
---

# Packet 87 — Move `region_mapping` Kernel to `slicer-core` (D Phase 2)

## Goal

Decouple `execute_region_mapping`'s public signature from `&ExecutionPlan` (which after P85 lives in `slicer-scheduler`), then move the pure-algorithm kernel (~410 LOC of the file's 628 LOC) from `crates/slicer-runtime/src/region_mapping.rs` into `crates/slicer-core/src/algos/region_mapping.rs`, leaving a thin `RegionMappingProducer` wrapper (~80 LOC, including the `BuiltinProducer` impl, the `&ExecutionPlan` → projection unpacking, and the `Blackboard` commit per ADR-0001) in `crates/slicer-runtime/src/builtins/region_mapping_producer.rs`.

## Scope Boundaries

This packet closes the D-phase work that started in P84. The previous deep-dive on D flagged `region_mapping` as the only one of seven builtins that did NOT split cleanly because its public sig took `&ExecutionPlan` (a then-`slicer-runtime` type). After P85, `ExecutionPlan` is in `slicer-scheduler`, but importing it from `slicer-core` would still be a back-edge (`slicer-core` is upstream of `slicer-scheduler` in the dep graph). The packet's first move is therefore a **signature refactor**: replace the `&ExecutionPlan` parameter with the smaller projection (`module_region_index`, `region_plans`, and whatever subset of `ExecutionPlan` fields `execute_region_mapping_inner` actually reads). The kernel then becomes a pure IR-in/IR-out fn that fits `slicer-core`. The wrapper in `slicer-runtime/src/builtins/` does the `&ExecutionPlan` → projection unpack inline (~10 LOC), preserving the existing call sites. Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- **Requires packet 85 closed**. `ExecutionPlan` and `CompiledModuleStatic` are now in `slicer-scheduler`; the wrapper imports them from there.
- Closure requires `cargo xtask build-guests --check` clean. This packet edits `slicer-core` (new algo module) but NOT `slicer-ir` / `slicer-sdk` / `slicer-schema` / `slicer-macros`. `slicer-core` is NOT in CLAUDE.md's guest-staleness list, so guests should stay clean. STALE means investigate.
- Not a workspace-test checkpoint packet — closes on narrow per-crate gates per the deepening-batch policy.

## Acceptance Criteria

### AC-1 — `region_mapping.rs` no longer exists under `slicer-runtime/src/`; kernel exists under `slicer-core/src/algos/region_mapping.rs`

**Given** the move,
**When** the working tree is inspected,
**Then** `test ! -f crates/slicer-runtime/src/region_mapping.rs` is true; `test -f crates/slicer-core/src/algos/region_mapping.rs` (or wherever P84 placed `slicer-core/src/algos/`) is true; the new file exposes `pub fn execute_region_mapping(...)` whose signature contains NO `ExecutionPlan` and NO runtime-side types.

| `test ! -f crates/slicer-runtime/src/region_mapping.rs && find crates/slicer-core/src -name 'region_mapping*' -type f | head -1 | grep -q . && ! grep -qE '\bExecutionPlan\b\|\bBlackboard\b' crates/slicer-core/src/algos/region_mapping.rs`

### AC-2 — `RegionMappingProducer` wrapper in `slicer-runtime/src/builtins/` performs the `&ExecutionPlan` → projection unpack and commits to `Blackboard`

**Given** the wrapper-keeps-glue pattern (ADR-0001),
**When** `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` is read,
**Then** it declares `pub static REGION_MAPPING_PRODUCER: BuiltinProducer = ...` with identical `stage_id`/`world_id`/`claim` as before P87. Its body extracts the necessary projection (`module_region_index`, `region_plans`, etc.) from `&ExecutionPlan`, calls `slicer_core::algos::region_mapping::execute_region_mapping(...)` with the projection, then commits the returned `RegionMapIR` via `Blackboard::replace_*` (or whatever the existing `commit_region_mapping_builtin` does). Total LOC ≤ 100.

| `test -f crates/slicer-runtime/src/builtins/region_mapping_producer.rs && grep -qE 'pub static REGION_MAPPING_PRODUCER' crates/slicer-runtime/src/builtins/region_mapping_producer.rs && grep -qE 'slicer_core::.*execute_region_mapping' crates/slicer-runtime/src/builtins/region_mapping_producer.rs && [ $(wc -l < crates/slicer-runtime/src/builtins/region_mapping_producer.rs) -le 120 ]`

### AC-3 — `slicer-runtime/src/lib.rs` no longer declares `pub mod region_mapping;`; the producer is reachable via the `builtins/` subtree

**Given** the move,
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** the line `pub mod region_mapping;` is absent. The `REGION_MAPPING_PRODUCER` entry in `runtime_builtins()` references `builtins::region_mapping_producer::REGION_MAPPING_PRODUCER` (or via the `builtins/mod.rs` re-export). The `pub use region_mapping::{execute_region_mapping, ...};` re-export block is either dropped or rewritten to `pub use slicer_core::algos::region_mapping::{...};` for backward source compat.

| `! grep -qE '^pub mod region_mapping;' crates/slicer-runtime/src/lib.rs && grep -qE 'REGION_MAPPING_PRODUCER' crates/slicer-runtime/src/lib.rs`

### AC-4 — `slicer-core/src/algos/region_mapping.rs` exposes a clean IR-in/IR-out signature with no runtime types

**Given** the decoupled kernel,
**When** the new file's `pub fn execute_region_mapping` signature is inspected,
**Then** it takes (in any order): `&LayerPlanIR`, the projection of plan data needed (e.g., `&HashMap<(u32, ModuleId), Vec<ActiveRegion>>` or a new `RegionMappingPlanProjection` struct defined in `slicer-core`), `Option<&PaintRegionIR>`, `&BTreeMap<PaintSemantic, ResolvedConfig>`, `&[ObjectMesh]`. Returns `Result<RegionMapIR, RegionMappingError>`. NO `&ExecutionPlan`, NO `&CompiledStage`, NO `&CompiledModuleStatic`, NO `&Blackboard`.

| `grep -E 'pub fn execute_region_mapping' crates/slicer-core/src/algos/region_mapping.rs | head -1 | grep -qE 'LayerPlanIR' && ! grep -qE 'ExecutionPlan\|CompiledStage\|CompiledModuleStatic\|Blackboard' crates/slicer-core/src/algos/region_mapping.rs`

### AC-5 — `runtime_builtins()` still returns the 8 producers in the documented pipeline order

**Given** the move preserves the canonical order in `docs/04_host_scheduler.md`,
**When** `crates/slicer-runtime/src/lib.rs::runtime_builtins()` is read,
**Then** the function body produces a `Vec<&'static dyn Producer>` whose entries are: MESH_PRODUCER, MESH_ANALYSIS_PRODUCER, REGION_MAPPING_PRODUCER, SLICE_PRODUCER, SHELL_CLASSIFICATION_PRODUCER, SUPPORT_GEOMETRY_PRODUCER, PAINT_SEGMENTATION_PRODUCER, GCODE_EMIT_PRODUCER (or equivalent; the count is 8, the order matches the pre-P87 order, and `REGION_MAPPING_PRODUCER` resolves to `builtins::region_mapping_producer::REGION_MAPPING_PRODUCER`).

| `[ $(grep -cE '_PRODUCER as &dyn Producer' crates/slicer-runtime/src/lib.rs) -eq 8 ] && grep -qE 'REGION_MAPPING_PRODUCER' crates/slicer-runtime/src/lib.rs`

### AC-6 — `slicer-core` does NOT gain a path dep on `slicer-scheduler` (the kernel's signature did not need it)

**Given** the decoupling,
**When** `crates/slicer-core/Cargo.toml` is read,
**Then** `slicer-scheduler` does NOT appear in `[dependencies]`, `[dev-dependencies]`, or `[build-dependencies]`. Same for `slicer-runtime`, `slicer-wasm-host`. The decoupling worked — the kernel needed only IR types (already available via `slicer-ir`).

| `! grep -qE '^slicer-(scheduler\|runtime\|wasm-host) *=' crates/slicer-core/Cargo.toml`

### AC-7 — End-to-end slice produces byte-identical g-code vs the P86 baseline SHA

**Given** the kernel is preserved verbatim (only its signature changes; the algorithm body is unchanged),
**When** `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p87.gcode` runs,
**Then** the SHA matches the P86 closure SHA. (The signature change is a pure refactor; `RegionMapIR` content is unchanged.)

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p87.gcode && sha256sum /tmp/benchy-p87.gcode`

### AC-8 — Per-algorithm unit test in `slicer-core` covers `execute_region_mapping`

**Given** the move,
**When** `cargo test -p slicer-core --test algo_region_mapping_tdd` runs (or the chosen test-file name),
**Then** the test passes. The test constructs a small `LayerPlanIR` + a manually-built `RegionMappingPlanProjection` (or whatever the new sig takes) and asserts the returned `RegionMapIR` has the expected per-(layer, object, region) shape for a two-object two-region fixture. The test imports zero `slicer_runtime::*` types.

| `cargo test -p slicer-core`

### AC-9 — Narrow per-crate test gates pass

**Given** the move,
**When** `cargo test -p slicer-core -p slicer-runtime -p pnp-cli` runs,
**Then** all three pass. `slicer-runtime` count delta = -(tests moved to `slicer-core`); `slicer-core` count delta = +(tests migrated + the new AC-8 test).

| `cargo test -p slicer-core -p slicer-runtime -p pnp-cli`

## Negative Test Cases

### AC-N1 — No file under `crates/slicer-core/src/` mentions `ExecutionPlan`, `CompiledStage`, `CompiledModuleStatic`, `Blackboard`, `BuiltinProducer`

**Given** the algorithm/glue split (analogous to P84's),
**When** `rg` runs,
**Then** the result is empty. `slicer-core` stays free of orchestration types.

| `! rg -e '\b(ExecutionPlan\|CompiledStage\|CompiledModuleStatic\|Blackboard\|BuiltinProducer)\b' crates/slicer-core/src/ 2>/dev/null`

### AC-N2 — `slicer-core`'s dep tree does NOT include `slicer-scheduler` or `slicer-wasm-host`

**Given** the architectural invariant,
**When** `cargo tree -p slicer-core --edges normal` is inspected,
**Then** neither `slicer-scheduler` nor `slicer-wasm-host` appears at any depth. (Implication: `slicer-core` tests are dispatcher-free; algorithm fixes have zero blast radius outside the geometry crate.)

| `! cargo tree -p slicer-core 2>&1 | grep -qE '\b(slicer-scheduler\|slicer-wasm-host)\b'`

## Verification (gate commands only)

1. `cargo build --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests --check` (clean; this packet edits no guest-feeding paths)
4. `cargo test -p slicer-core -p slicer-runtime -p pnp-cli`

Workspace test gate NOT run at P87 close — the next checkpoint is P88.

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `RegionMapIR`, `RegionPlan`, `ActiveRegion`, `LayerPlanIR`, `PaintRegionIR`, `ObjectMesh`. The kernel's I/O contracts. No content change.
- `docs/04_host_scheduler.md` — `PrePass::RegionMapping` stage placement; `REGION_MAPPING_PRODUCER`'s pipeline position. No content change.
- `docs/adr/0001-prepass-builtins-commit-in-stage.md` — the wrapper-keeps-commit pattern P87 preserves.
- `docs/adr/0006-compiled-module-static-live-split.md` (from P85 close) — confirms the `ExecutionPlan`-living-in-scheduler shape that the wrapper's projection-unpack reads.

## Doc Impact Statement

No doc files are edited by this packet. The signature refactor preserves the kernel's behavior; `docs/02_ir_schemas.md` already documents the IR types unchanged. A future doc-sweep packet may add a one-line crate-map mention of `slicer-core` housing the seventh algorithm (six were noted in P84's doc-sweep, this is the final one).

No ADR follow-up — the refactor preserves all existing decisions; the `RegionMappingPlanProjection` struct (if created) is a private implementation detail of the move, not an architectural decision worth recording.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

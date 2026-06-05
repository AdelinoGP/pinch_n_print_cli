---
status: implemented
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
**Then** `test ! -f crates/slicer-runtime/src/region_mapping.rs` is true; `test -f crates/slicer-core/src/algos/region_mapping.rs` is true (P84 already established `crates/slicer-core/src/algos/` as the layout — verified: 6 algos present before P87). The new file exposes BOTH `pub fn execute_region_mapping(...)` (the cap-default delegator at the current L336) AND `pub fn execute_region_mapping_with_cap(...)` (the actual cap-taking kernel at the current L365) — plus the private helpers (`execute_region_mapping_inner` at L384), the `DEFAULT_REGION_MAP_CAP` constant, the `RegionMappingError` enum (currently L68), and any other private helpers the kernel calls. None of these contain `ExecutionPlan`, `Blackboard`, or other runtime/scheduler types in their signatures.

| `test ! -f crates/slicer-runtime/src/region_mapping.rs && test -f crates/slicer-core/src/algos/region_mapping.rs && grep -qE 'pub fn execute_region_mapping\b' crates/slicer-core/src/algos/region_mapping.rs && grep -qE 'pub fn execute_region_mapping_with_cap\b' crates/slicer-core/src/algos/region_mapping.rs && grep -qE 'pub enum RegionMappingError' crates/slicer-core/src/algos/region_mapping.rs && ! grep -qE '\bExecutionPlan\b' crates/slicer-core/src/algos/region_mapping.rs && ! grep -qE '\bBlackboard\b' crates/slicer-core/src/algos/region_mapping.rs`

### AC-2 — `region_mapping_producer.rs` wrapper holds the producer static + the relocated `commit_region_mapping_builtin` body, and constructs the projection from `&ExecutionPlan`

**Given** the wrapper-keeps-glue pattern (ADR-0001),
**When** `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` is read,
**Then** it declares `pub static REGION_MAPPING_PRODUCER: BuiltinProducer = ...` with identical `stage_id`/`world_id`/`claim` as before P87 (currently L30 of `region_mapping.rs`). The relocated `commit_region_mapping_builtin` body (currently L559 of `region_mapping.rs`, ~70 LOC, takes `&ExecutionPlan`) lives in this wrapper. The wrapper builds a `RegionMappingPlanProjection` from `&plan.module_region_index` (bare borrow) and `&*plan.region_plans` (Arc deref — `region_plans` is `Arc<HashMap<...>>` per `crates/slicer-scheduler/src/execution_plan.rs:286`) and calls `slicer_core::algos::region_mapping::execute_region_mapping_with_cap(...)` (the actual kernel; the simple `execute_region_mapping` wrapper may be skipped or called depending on cap-passing). Returns `RegionMapIR` and commits via the existing `Blackboard` method. Total LOC ≤ 150 (commit body ~70 + producer static ~20 + unpack/imports ~30, with headroom).

| `test -f crates/slicer-runtime/src/builtins/region_mapping_producer.rs && grep -qE 'pub static REGION_MAPPING_PRODUCER' crates/slicer-runtime/src/builtins/region_mapping_producer.rs && grep -qE 'slicer_core::.*execute_region_mapping' crates/slicer-runtime/src/builtins/region_mapping_producer.rs && [ $(wc -l < crates/slicer-runtime/src/builtins/region_mapping_producer.rs) -le 150 ]`

### AC-3 — `slicer-runtime/src/lib.rs` no longer declares `pub mod region_mapping;`; the producer is reachable via the `builtins/` subtree

**Given** the move,
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** the line `pub mod region_mapping;` is absent. The `REGION_MAPPING_PRODUCER` entry in `runtime_builtins()` references `builtins::region_mapping_producer::REGION_MAPPING_PRODUCER` (or via the `builtins/mod.rs` re-export). The `pub use region_mapping::{execute_region_mapping, ...};` re-export block is either dropped or rewritten to `pub use slicer_core::algos::region_mapping::{...};` for backward source compat.

| `! grep -qE '^pub mod region_mapping\b' crates/slicer-runtime/src/lib.rs && grep -qE 'REGION_MAPPING_PRODUCER' crates/slicer-runtime/src/lib.rs` (word boundary catches both bare-semicolon and brace-form `pub mod region_mapping { pub use slicer_core::algos::region_mapping::*; }` shim modules — both forbidden per CLAUDE.md "no backwards-compatibility hacks", same discipline P84/P85/P86 closed under)

### AC-4 — `slicer-core/src/algos/region_mapping.rs` exposes a clean IR-in/IR-out signature with no runtime types

**Given** the decoupled kernel,
**When** the new file's `pub fn execute_region_mapping` signature is inspected,
**Then** it takes (in any order): `&LayerPlanIR`, the projection of plan data needed (e.g., `&HashMap<(u32, ModuleId), Vec<ActiveRegion>>` or a new `RegionMappingPlanProjection` struct defined in `slicer-core`), `Option<&PaintRegionIR>`, `&BTreeMap<PaintSemantic, ResolvedConfig>`, `&[ObjectMesh]`. Returns `Result<RegionMapIR, RegionMappingError>`. NO `&ExecutionPlan`, NO `&CompiledStage`, NO `&CompiledModuleStatic`, NO `&Blackboard`.

| `grep -A 8 -E 'pub fn execute_region_mapping\b' crates/slicer-core/src/algos/region_mapping.rs | head -9 | grep -qE 'LayerPlanIR' && ! grep -qE '\b(ExecutionPlan|CompiledStage|CompiledModuleStatic|Blackboard)\b' crates/slicer-core/src/algos/region_mapping.rs`

### AC-5 — `runtime_builtins()` still returns the 8 producers in the documented pipeline order

**Given** the move preserves the canonical order in `docs/04_host_scheduler.md`,
**When** `crates/slicer-runtime/src/lib.rs::runtime_builtins()` is read,
**Then** the function body produces a `Vec<&'static dyn Producer>` whose entries are: MESH_PRODUCER, MESH_ANALYSIS_PRODUCER, REGION_MAPPING_PRODUCER, SLICE_PRODUCER, SHELL_CLASSIFICATION_PRODUCER, SUPPORT_GEOMETRY_PRODUCER, PAINT_SEGMENTATION_PRODUCER, GCODE_EMIT_PRODUCER (or equivalent; the count is 8, the order matches the pre-P87 order, and `REGION_MAPPING_PRODUCER` resolves to `builtins::region_mapping_producer::REGION_MAPPING_PRODUCER`).

| `[ $(grep -cE '_PRODUCER as &dyn Producer' crates/slicer-runtime/src/lib.rs) -eq 8 ] && grep -qE 'REGION_MAPPING_PRODUCER' crates/slicer-runtime/src/lib.rs`

### AC-6 — `slicer-core` does NOT gain a path dep on `slicer-scheduler` (the kernel's signature did not need it)

**Given** the decoupling,
**When** `crates/slicer-core/Cargo.toml` is read,
**Then** `slicer-scheduler` does NOT appear in `[dependencies]`, `[dev-dependencies]`, or `[build-dependencies]`. Same for `slicer-runtime`, `slicer-wasm-host`. The decoupling worked — the kernel needed only IR types (already available via `slicer-ir`).

| `! grep -qE '^slicer-(scheduler|runtime|wasm-host) *=' crates/slicer-core/Cargo.toml`

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

### AC-9 — Narrow per-crate test gates pass with full feature flag set

**Given** the move AND the P83/P84/P85 lesson that bare `cargo test -p <crate>` silently skips feature-gated test targets and SDK-test-gated targets,
**When** `cargo test --features slicer-core/host-algos --features slicer-sdk/test -p slicer-core -p slicer-runtime -p pnp-cli` runs,
**Then** all three pass. `slicer-runtime` count delta = -(tests moved to `slicer-core`); `slicer-core` count delta = +(tests migrated + the new AC-8 test). Bare `cargo test -p` form is REJECTED — proved at P85 closure to mask real regressions.

| `cargo test --features slicer-core/host-algos --features slicer-sdk/test -p slicer-core -p slicer-runtime -p pnp-cli`

## Negative Test Cases

### AC-N1 — No file under `crates/slicer-core/src/` imports or takes by-reference any runtime/scheduler-only type (`ExecutionPlan`, `CompiledStage`, `CompiledModuleStatic`, `Blackboard`, `BuiltinProducer`)

**Given** the algorithm/glue split (analogous to P84's),
**When** `crates/slicer-core/src/` is grepped for use-statement imports and by-reference signatures referencing those types,
**Then** the result is empty. The grep is shaped to match imports (`use ...::ExecutionPlan`) and parameter types (`: &Blackboard`, `&mut ExecutionPlan`) rather than bare word matches — bare-word matching would false-positive on doc comments that mention the type for context.

| `! rg -e 'use [^;]*\b(ExecutionPlan|CompiledStage|CompiledModuleStatic|Blackboard|BuiltinProducer)\b' crates/slicer-core/src/ && ! rg -e ': *&(mut )?(ExecutionPlan|CompiledStage|CompiledModuleStatic|Blackboard|BuiltinProducer)\b' crates/slicer-core/src/`

### AC-N2 — `slicer-core`'s dep tree does NOT include `slicer-scheduler` or `slicer-wasm-host`

**Given** the architectural invariant,
**When** `cargo tree -p slicer-core --edges normal` is inspected,
**Then** neither `slicer-scheduler` nor `slicer-wasm-host` appears at any depth. (Implication: `slicer-core` tests are dispatcher-free; algorithm fixes have zero blast radius outside the geometry crate.)

| `! cargo tree -p slicer-core 2>&1 | grep -qE '\b(slicer-scheduler|slicer-wasm-host)\b'`

### AC-N3 — No undocumented `pub use slicer_core::algos::region_mapping::` re-exports remain in `slicer-runtime/src/lib.rs`

**Given** the P84/P85/P86-derived closure-cleanup rule (Step 5 pruning prunes dead re-exports; survivors carry a `// kept:` annotation either ABOVE or BELOW the line),
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** every `pub use slicer_core::algos::region_mapping::...;` re-export line that survives the cleanup is followed (OR preceded) by a one-line comment naming its surviving consumer (e.g., `// kept: consumed by crates/<x>/<y>.rs`). Re-exports without a surviving consumer must have been deleted. Same shape as P85's AC-N4 and P86's AC-N4 — structural signal that P87 closes with no backwards-compat shim accumulation.

| `for line in $(grep -nE '^pub use slicer_core::algos::region_mapping::' crates/slicer-runtime/src/lib.rs | cut -d: -f1); do prev=$((line-1)); next=$((line+1)); (sed -n "${prev}p" crates/slicer-runtime/src/lib.rs | grep -qE '^// kept:') || (sed -n "${next}p" crates/slicer-runtime/src/lib.rs | grep -qE '^// kept:') || exit 1; done`

## Verification (gate commands only)

1. `cargo build --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests --check` (clean; this packet edits no guest-feeding paths)
4. `cargo test --features slicer-core/host-algos --features slicer-sdk/test -p slicer-core -p slicer-runtime -p pnp-cli`

Workspace test gate NOT run at P87 close — the next checkpoint is P88. Corrected workspace baseline post-P86 = ~2067 passing; if running the workspace gate informally for sanity, carry the full flag set: `cargo test --features slicer-core/host-algos --features slicer-sdk/test --no-fail-fast --workspace`.

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `RegionMapIR`, `RegionPlan`, `ActiveRegion`, `LayerPlanIR`, `PaintRegionIR`, `ObjectMesh`. The kernel's I/O contracts. No content change.
- `docs/04_host_scheduler.md` — `PrePass::RegionMapping` stage placement; `REGION_MAPPING_PRODUCER`'s pipeline position. No content change.
- `docs/adr/0001-prepass-builtins-commit-in-stage.md` — the wrapper-keeps-commit pattern P87 preserves.
- `docs/adr/0007-compiled-module-static-live-split.md` (from P85 close — note numbered 0007, NOT 0006, because ADR-0004 was claimed by Packet 77 and ADR-0005/0006 by P83's runner-traits/export_for_stage_id) — confirms the `ExecutionPlan`-living-in-scheduler shape that the wrapper's projection-unpack reads.

## Doc Impact Statement

No doc files are edited by this packet. The signature refactor preserves the kernel's behavior; `docs/02_ir_schemas.md` already documents the IR types unchanged. A future doc-sweep packet may add a one-line crate-map mention of `slicer-core` housing the seventh algorithm (six were noted in P84's doc-sweep, this is the final one).

No ADR follow-up — the refactor preserves all existing decisions; the `RegionMappingPlanProjection` struct (if created) is a private implementation detail of the move, not an architectural decision worth recording.

## Deviations

- **D1 (AC-2 wrapper shape) — Projection schema mismatch.** `packet.spec.md` / `design.md` projected `module_region_index` + `region_plans` from `ExecutionPlan`. Step 1 verification showed the kernel actually reads `per_layer_stages` + `postpass_stages` (`CompiledStage` slices). Resolved by precomputing `Vec<(StageId, Vec<ModuleInvocation>)>` in the wrapper and passing as a slice into the kernel via `RegionMappingPlanProjection<'a> { stage_invocations: &'a [(StageId, Vec<ModuleInvocation>)] }`. Architecturally clean: keeps `CompiledStage` out of `slicer-core`, preserves AC-6 and AC-N2.
- **D2 (resolved) — `DEFAULT_REGION_MAP_CAP` relocated to `slicer-ir`.** Initial implementation added a `slicer-scheduler → slicer-core` dep edge to enable a `pub use`. Reviewer rejected the new edge (not in the deepening plan's graph). Resolved at Step 5.5 by moving the const to `crates/slicer-ir/src/slice_ir.rs` (where IR-shape constraints naturally live); both `slicer-scheduler` and `slicer-core` re-export from there via `pub use slicer_ir::DEFAULT_REGION_MAP_CAP`. No new dep edges introduced. `cargo tree -p slicer-scheduler | grep slicer-core` and `cargo tree -p slicer-core | grep slicer-(scheduler|wasm-host|runtime)` both empty (exit 1). D3 (per-module `host-algos` gating in `slicer-core/src/lib.rs`) reverted as a downstream consequence — `#[cfg(feature = "host-algos")] pub mod algos;` restored to the original simple form.
- **D4 (helper duplication) — `paint_semantic_namespace_key` inlined as a private fn in `slicer-core`.** Avoids importing from `slicer-scheduler`. Acceptable as a small static helper; if future scheduler-side updates require sync, consider hoisting to `slicer-ir`.
- **D5 (AC-1 prose) — `execute_region_mapping_inner` made `pub` in `slicer-core`.** AC-1 narrative called it a "private helper"; the wrapper (`commit_region_mapping_builtin`) needed direct access with `Some(host_config)`. AC-1 verification command still passes; prose amended in spirit to "internal entry point reachable from the wrapper".
- **D6 — `crates/slicer-runtime/src/prepass.rs` had an unanticipated internal import of `crate::region_mapping::*`.** Mechanical follow-on; updated to `crate::builtins::region_mapping_producer::*`. Should have been in `design.md`'s read-only context list.
- **D7 (AC-4 verification command) — `head -1` truncated multi-line Rust signature.** AC-4's original command `grep -E 'pub fn execute_region_mapping' … | head -1 | grep -qE 'LayerPlanIR'` failed because the function signature is multi-line — `pub fn execute_region_mapping(` is on one line and `layer_plan: &LayerPlanIR,` on the next. `head -1` truncated the input before `LayerPlanIR` could be seen, producing a false negative. Patched to `grep -A 8 -E 'pub fn execute_region_mapping\b' … | head -9 | grep -qE 'LayerPlanIR'` (also applied to `requirements.md`'s AC-4 row). Implementation was correct throughout; the patch makes the check actually verify the documented intent.

(D3 absorbed into D2's resolution — no standalone entry.)

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

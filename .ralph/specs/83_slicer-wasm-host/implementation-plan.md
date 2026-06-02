# Packet 83 — Implementation Plan

## Execution Rules

- Each step ends with a falsifying check that gates green before the next step starts.
- Files allowed to read per step are bounded. `wit_host.rs` (5 259 LOC) and `dispatch.rs` (3 148 LOC) are NEVER loaded in full; section-by-section grep + ±40-line reads only.
- The packet closure gate runs `cargo test --workspace` per the deepening-batch checkpoint policy; partial test runs are not sufficient.
- P81 AND P82 MUST be closed before this packet starts (Step 0 verifies).
- The schema edit triggers guest WASM staleness; Step 7 rebuilds guests and re-verifies `--check` clean before Step 8 runs the full test suite.

---

## Step 0 — Verify P81 and P82 closure, capture pre-packet baselines

**Objective.** Confirm both predecessors are closed and the workspace is in the expected post-P82 state. Capture pre-packet g-code SHA (carries forward from P81's baseline if no semantic change occurred in P82 — Step 0 explicitly re-checks).

**Precondition.** P81 and P82 statuses are `superseded`. Working tree clean.

**Postcondition.** Baselines in the implementation log: g-code SHA against `resources/benchy.stl`; `cargo test --workspace` pre-packet pass count.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: confirm `test ! -f crates/slicer-runtime/src/{cli,helpers_cmd,model_loader,model_loader_sidecar,model_writer}.rs && test -f crates/slicer-model-io/Cargo.toml`. Return FACT pass/fail.
- Dispatch: g-code SHA. `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/p83-baseline.gcode && sha256sum /tmp/p83-baseline.gcode`. Return FACT `<hex>`.
- Dispatch: `cargo test --workspace 2>&1 | tail -5`. Return SNIPPET (≤ 5 lines containing the test count + final result).

**Context cost: S.**

**Narrow verification.** P81+P82 verification pass. Baseline SHA captured. Pre-packet test count recorded.

**Falsifying check / exit condition.** Any verification fails → abort; the predecessor work is incomplete.

---

## Step 0.5 — Prework: stage I/O three-group split + narrow runner error + instance_pool narrowing

**Objective.** Resolve the AC-N3 vs trait-signature contradiction (see design.md "Borrow-struct pattern for trait inputs" and "Narrow runner errors") via four sub-steps. The post-survey reality is that the 8 stage I/O types do not all belong in the same crate: 7 are clean for `slicer-ir`, 1 (`PrepassStageOutput`) needs `slicer-core` because it carries a `PaintRegionRTreeIndex` payload, and `PrepassExecutionError` stays in `slicer-runtime` with a narrow runner-side error (`PrepassRunnerError`) introduced in `slicer-ir` per the P86 `GCodeEmitError → PostpassError` idiom. `LoadedModule` is left in place; `instance_pool.rs`'s helper is narrowed to take primitives instead of `&LoadedModule`.

**Precondition.** Step 0 green. (Steps 2/3 may already be done in a resumed session; that does not invalidate this step; it must complete before Step 4.)

**Postcondition.** Workspace builds green. The runner traits (still in `slicer-runtime` for now) can reference all return types via `slicer-ir` / `slicer-core` paths only — Step 4 then lifts the traits to `slicer-wasm-host` without a back-edge dep on `slicer-runtime`.

### Step 0.5a — Group A → `slicer-ir`

**Types (7).** `LayerStageOutput`, `FinalizationOutput`, `FinalizationError`, `PostpassOutput`, `PostpassError`, `LayerStageError`, `LayerArenaError` (the last one carved from `blackboard.rs:597` — ~20 LOC, isolated; moves with `LayerStageError` because the latter carries it in its `ArenaCommit` variant).

**Files allowed to read.** `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization,blackboard}.rs` — grep + ±40-line windows only. `crates/slicer-ir/src/lib.rs` (full file ≤ 200 LOC OK).

**Files allowed to edit.**
1. `crates/slicer-ir/src/stage_io.rs` — CREATE; receive the 7 types.
2. `crates/slicer-ir/src/lib.rs` — `pub mod stage_io; pub use stage_io::*;`.
3. `crates/slicer-runtime/src/{layer_executor,layer_finalization,postpass}.rs` — DELETE the local type defs; replace with `use slicer_ir::{…};`.
4. `crates/slicer-runtime/src/blackboard.rs` — DELETE `pub enum LayerArenaError` declaration; replace with `use slicer_ir::LayerArenaError;` at the top. **Body of blackboard.rs otherwise untouched.**
5. `crates/slicer-runtime/src/lib.rs` — add transitional `pub use slicer_ir::{LayerStageOutput, LayerStageError, LayerArenaError, FinalizationOutput, FinalizationError, PostpassOutput, PostpassError};` re-exports.

**Falsifying check.** If `LayerArenaError`'s fields reference any other slicer-runtime-internal type beyond what's already classified clean, stop and surface — apply the P86 narrow-split pattern.

**Gate.** `cargo build -p slicer-ir` green. `cargo build --workspace` green.

### Step 0.5b — Group B → `slicer-core`

**Types (5).** `PrepassStageOutput` and the 4-type `MeshAnalysisAuxiliary` cluster (`MeshAnalysisAuxiliary`, `FacetAnnotationRecord`, `FacetClassRecord`, `SurfaceGroupRecord` — currently in `crates/slicer-runtime/src/prepass.rs:64–112`).

**Rationale.** `PrepassStageOutput`'s `PaintRegions` variant carries `PaintRegionRTreeIndex` which already lives in `slicer-core`. Putting `PrepassStageOutput` in `slicer-ir` would force `slicer-ir → slicer-core`, which contaminates IR with spatial-index machinery. `slicer-core` already deps on `slicer-ir`, so the move into `slicer-core` is the natural place. `slicer-wasm-host` then takes `slicer-core` as a path dep (`slicer-core` has no `wasmtime` — no upward back-edge).

**Files allowed to read.** `crates/slicer-runtime/src/prepass.rs:1..115` only. `crates/slicer-core/src/lib.rs` (≤ 200 LOC OK).

**Sanity check before moving.** Grep `crates/slicer-runtime/src/prepass.rs:64..115` for any `use crate::` or non-`slicer_ir`/`slicer_core`/`std` references — confirm the 4 MeshAnalysisAuxiliary types have no further chains. If any of them transitively pulls in a runtime-internal type, **stop and surface** (apply narrow-split treatment).

**Files allowed to edit.**
1. `crates/slicer-core/src/stage_io.rs` (or co-locate with `paint_region.rs` — implementer's call) — CREATE; receive the 5 types.
2. `crates/slicer-core/src/lib.rs` — declare and re-export.
3. `crates/slicer-runtime/src/prepass.rs` — DELETE the 5 type defs; replace with `use slicer_core::{…};`.
4. `crates/slicer-runtime/src/lib.rs` — add `pub use slicer_core::{PrepassStageOutput, MeshAnalysisAuxiliary, FacetAnnotationRecord, FacetClassRecord, SurfaceGroupRecord};`.
5. `crates/slicer-wasm-host/Cargo.toml` — add `slicer-core = { path = "../slicer-core" }`.

**Gate.** `cargo build -p slicer-core` green. `cargo build --workspace` green.

### Step 0.5c — `PrepassRunnerError` narrow split (P86 idiom) — preceded by `BlackboardError` move

**Pattern.** Mirror P86's `GCodeEmitError → PostpassError`. Define a 2-variant `PrepassRunnerError` in `slicer-ir/src/stage_io.rs` whose `Blackboard` variant carries a `BlackboardError` payload **losslessly** (same shape as the existing `PrepassExecutionError::Blackboard.source`). Lossless requires `BlackboardError` itself to be reachable from `slicer-ir`, so move it (same pattern as `LayerArenaError` in Step 0.5a — small, isolated). The `From<PrepassRunnerError> for PrepassExecutionError` impl then becomes a one-line variant remap with no field synthesis or information loss.

**Sub-sequence (apply in order; each must build green before the next):**

#### 0.5c-i — Falsifying check on `BlackboardError`

Run `rg 'use crate::|use slicer_runtime::' crates/slicer-runtime/src/blackboard.rs | grep -i BlackboardError` and read the variant definitions of `BlackboardError`. Confirm every variant's field types are one of: std (`String`, `Vec`, `usize`, …), slicer-ir-reachable (`ModuleId`, `StageId`, `BlackboardPrepassSlot` if that one is already in slicer-ir or moves cleanly), or primitives. If any variant carries a runtime-internal aggregate (e.g., `Arc<Blackboard>`, a runtime-defined struct), **STOP and surface** — handle the fourth-order chain by either moving the dependency down or splitting the variant.

Likely shape per Step 0.5c-first-attempt survey: `DuplicatePrepassCommit { slot: BlackboardPrepassSlot }`, `MissingRequiredPrepass { slot }`, `DuplicateLayerCommit { layer_index: usize }`, `LayerSlotOutOfRange { layer_index, layer_count }`, `IncompleteLayerDrain { missing_indices: Vec<usize> }`, `LayerOutputsAlreadyDrained` (unit). The `slot: BlackboardPrepassSlot` payload is the only non-trivial sub-type — confirm it moves clean too (it's defined in `blackboard.rs:141` per Step 1 survey; verify it doesn't pull further chains).

#### 0.5c-ii — Move `BlackboardError` (and `BlackboardPrepassSlot` if needed) to `slicer-ir`

Co-locate with `LayerArenaError` in `crates/slicer-ir/src/stage_io.rs`. Add transitional `pub use slicer_ir::{BlackboardError, BlackboardPrepassSlot};` to `crates/slicer-runtime/src/lib.rs`. `blackboard.rs` deletes the moved type declarations and re-imports them via `use slicer_ir::{...};` — body otherwise untouched (same surgical-edit discipline as Step 0.5a's `LayerArenaError` carve-out).

**Gate.** `cargo build -p slicer-ir` green; `cargo build --workspace` green.

#### 0.5c-iii — Introduce `PrepassRunnerError` + `From` impl

In `crates/slicer-ir/src/stage_io.rs`:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepassRunnerError {
    FatalModule { stage_id: StageId, module_id: ModuleId, message: String },
    Blackboard  { stage_id: StageId, module_id: ModuleId, source: BlackboardError },
}

impl std::fmt::Display for PrepassRunnerError { /* hand-rolled like the other stage_io types */ }
impl std::error::Error for PrepassRunnerError {}
```

In `crates/slicer-runtime/src/prepass.rs`:
```rust
impl From<slicer_ir::PrepassRunnerError> for PrepassExecutionError {
    fn from(e: slicer_ir::PrepassRunnerError) -> Self {
        match e {
            slicer_ir::PrepassRunnerError::FatalModule { stage_id, module_id, message } =>
                Self::FatalModule { stage_id, module_id, message },
            slicer_ir::PrepassRunnerError::Blackboard { stage_id, module_id, source } =>
                Self::Blackboard { stage_id, module_id, source },
        }
    }
}
```

Lossless one-line variant remaps; no field synthesis.

**Gate.** `cargo build -p slicer-runtime` green; `cargo build --workspace` green.

### Step 0.5d — Narrow `instance_pool.rs` signature

**Refactor.** Replace `module: &LoadedModule` parameter in `instance_pool.rs`'s helper(s) with the actual fields the helper body uses: `module_id: &str, stage: &str, layer_parallel_safe: bool, host_parallelism: usize, artifact: WasmArtifactMetadata`. (Mid-flight pivot from the originally-planned `wasm_path: &Path, placeholder_wasm: bool` shape — the pre-Step survey had been wrong about which `LoadedModule` fields the body actually consumes; on-disk truth is `crates/slicer-wasm-host/src/pool.rs:86`.) Callers in `slicer-runtime` extract the fields from `LoadedModule` before invoking. `LoadedModule` stays in `slicer-runtime/src/manifest.rs` unchanged.

**Files allowed to read.** `crates/slicer-runtime/src/instance_pool.rs` (full ≤ 200 LOC OK). Call sites surfaced by grep `build_wasm_instance_pool` (Step 1 dispatch #1 already listed `instrumentation.rs:774`, `path_ordering_tdd.rs:13`, `layer_collection_builder_tdd.rs:23`, `dag_validation_tdd.rs:274`, `finalization_*_tdd.rs:24` etc.).

**Files allowed to edit.**
1. `crates/slicer-runtime/src/instance_pool.rs` — narrow the helper signature(s). Delete `use crate::LoadedModule;`.
2. All call sites — update to extract `wasm_path` + `placeholder_wasm` from `LoadedModule` before calling.

**Gate.** `cargo build --workspace` green. After this step, `cargo tree -p slicer-wasm-host` (which doesn't exist yet on the wasm-host side, but the import surface inspection) should show `instance_pool.rs` has zero remaining runtime-internal dependencies.

### Step 0.5e — Define `LayerStageCommitData` in `slicer-ir`

**Rationale.** The 18/18 Category-B classification of `arena.*` accessors in dispatch.rs (pre-Step-4 survey) confirmed that all `LayerArena` interaction happens on the runtime side of the wasm-host trait boundary. The trait method on the wasm-host side returns an IR-typed deconstruction of the wasmtime call's `HostExecutionContext`, NOT the raw `HostExecutionContext` itself. That IR-typed return shape is `LayerStageCommitData`, defined here in `slicer-ir` for symmetry with `LayerStageInput<'_>` (input side).

**Files allowed to read.** `crates/slicer-runtime/src/dispatch.rs` L2535..L2900 (the existing `commit_layer_outputs` body — the source-of-truth for which fields `LayerStageCommitData` must carry). Line-range only.

**Files allowed to edit.**
1. `crates/slicer-ir/src/stage_io.rs` — add `pub struct LayerStageCommitData { … }` with field-by-field IR-typed mirrors of the WIT-collected outputs `commit_layer_outputs` reads. Likely fields (subject to survey): `infill: Vec<InfillCommit>`, `support: Vec<SupportCommit>`, `perimeters: Vec<PerimeterCommit>`, `deferred_annotations: Vec<DeferredAnnotation>`, `deferred_tool_changes: Vec<DeferredToolChange>`, `deferred_z_hops: Vec<DeferredZHop>`, `deferred_retracts: Vec<DeferredRetract>`, `deferred_travel_moves: Vec<DeferredTravelMove>`, `entity_order_proposal: Option<EntityOrderProposal>`, `diagnostics: Vec<String>`. Exact field types come from inspecting `commit_layer_outputs`'s `ctx.*` accesses.

**Falsifying check.** If any field of `LayerStageCommitData` would naturally hold a wasm-host-coupled type (e.g., a `wasmtime::Resource<…>`, a builder handle, an `Arc<WasmInstance>`), **STOP and surface** — that field belongs on the wasm-host side of the deconstruction (i.e., `HostExecutionContext` keeps it), and only its plain-IR projection enters `LayerStageCommitData`. Any sub-type that itself only exists inside `wit_host.rs` (e.g., a `*BuilderData` resource backing type) MUST be lowered into a plain IR type before crossing the boundary.

**Expected sub-agent dispatch.** A single worker:
1. Reads `dispatch.rs::commit_layer_outputs` body to enumerate every `ctx.*` field access and its resolved type.
2. For each access, identifies the corresponding plain-IR equivalent (most should already be slicer-ir types since dispatch.rs already converts WIT resources into IR before commit).
3. Authors `LayerStageCommitData` in `crates/slicer-ir/src/stage_io.rs`.
4. Runs `cargo build -p slicer-ir`.

**Gate.** `cargo build -p slicer-ir` green; `cargo build --workspace` green (the new IR struct is unused initially — Step 4b is its first consumer).

---

### Combined Step 0.5 verification

- `cargo build --workspace` green.
- `grep -rE 'pub (enum|struct) (LayerStageOutput|LayerStageError|LayerArenaError|FinalizationOutput|FinalizationError|PostpassOutput|PostpassError)' crates/slicer-ir/` returns 7 hits.
- `grep -rE 'pub (enum|struct) (PrepassStageOutput|MeshAnalysisAuxiliary)' crates/slicer-core/` returns ≥ 2 hits (PrepassStageOutput + cluster types).
- `grep -qE 'pub enum PrepassExecutionError' crates/slicer-runtime/src/prepass.rs` — still present (unchanged).
- `grep -qE 'pub enum PrepassRunnerError' crates/slicer-ir/` — present.
- `grep -qE 'impl From<PrepassRunnerError> for PrepassExecutionError' crates/slicer-runtime/src/prepass.rs` — present.

**Context cost: S.**

**Falsifying check / exit condition (umbrella).** If any sub-step surfaces a third-order chain (e.g., a `MeshAnalysisAuxiliary` type transitively references a runtime aggregate), stop and surface — handle via the same narrow-split + `From` impl pattern at the orchestrator boundary.

---

## Step 1 — Locate trait defs, callers, and side imports

**Objective.** Build the precise lists of edit sites the packet will touch.

**Precondition.** Step 0 green.

**Postcondition.** Four lists in the implementation log:
- (a) The four `pub trait *StageRunner` declarations with exact file:line and full signatures.
- (b) Test files importing `wit_host::*`, `dispatch::*`, `wasm_instance::*`, `instance_pool::*`, or constructing `CompiledModule` directly.
- (c) Non-moving runtime files referencing the same.
- (d) Callers of `dispatch::export_name_for_stage` (expected ≤ 3 sites).

**Files allowed to read.** None directly — four dispatches.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch #1 (= design.md dispatch #1): trait declarations and signatures. SNIPPETS (4 snippets, ≤ 30 lines each).
- Dispatch #2: tests touching the four modules or constructing `CompiledModule`. LOCATIONS.
- Dispatch #3: non-moving runtime files referencing the four modules. LOCATIONS.
- Dispatch #4: `export_name_for_stage` callers. LOCATIONS.

**Context cost: S.**

**Narrow verification.** Four lists populated.

**Falsifying check / exit condition.** A site surfaces at `cargo build` in step 5 that isn't on any list → return here and widen.

---

## Step 2 — Add `export_for_stage_id` to `slicer-schema` + its test

**Objective.** Introduce the consolidated lookup before deleting the duplicate in `dispatch.rs`.

**Precondition.** Step 1 lists in hand.

**Postcondition.** `slicer-schema` builds with the new fn. The new test (`tests/export_for_stage_id_tdd.rs`) passes. Guest WASMs are now stale (per CLAUDE.md; the schema edit invalidates them).

**Files allowed to read.** `crates/slicer-schema/src/lib.rs` (full file ≤ 390 LOC — OK).
**Files allowed to edit.**
1. `crates/slicer-schema/src/lib.rs` — add `pub fn export_for_stage_id`.
2. `crates/slicer-schema/tests/export_for_stage_id_tdd.rs` — CREATE the lookup test.

**Expected sub-agent dispatches.**
- Dispatch: `cargo build -p slicer-schema && cargo test -p slicer-schema`. Return FACT pass/fail.
- Dispatch: `cargo xtask build-guests --check`. Expected: STALE for all guests (because the schema edit invalidates them).

**Context cost: S.**

**Narrow verification.** Schema builds; test passes; `--check` reports STALE (this is expected — Step 7 rebuilds).

**Falsifying check / exit condition.** Schema doesn't build → fix the fn signature/body.

---

## Step 3 — Create the `slicer-wasm-host` crate scaffold

**Objective.** New crate exists; empty `lib.rs` compiles; workspace `Cargo.toml` knows about it.

**Precondition.** Step 2 green.

**Postcondition.** `cargo build -p slicer-wasm-host` succeeds against an empty lib.rs scaffold.

**Files allowed to read.** Workspace `Cargo.toml`, `crates/slicer-runtime/Cargo.toml` (to copy `wasmtime` version).
**Files allowed to edit.**
1. Workspace `Cargo.toml` — add `"crates/slicer-wasm-host"` to `members`.
2. `crates/slicer-wasm-host/Cargo.toml` — CREATE with `wasmtime`, `slicer-ir`, `slicer-schema`, `slicer-sdk`.
3. `crates/slicer-wasm-host/src/lib.rs` — CREATE with module declarations only (`pub mod host; pub mod dispatch; pub mod instance; pub mod pool; pub mod traits; pub mod binding;`) and empty submodule files.

**Expected sub-agent dispatch.**
- Dispatch: `cargo build -p slicer-wasm-host`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Crate builds (empty).

**Falsifying check / exit condition.** Workspace inheritance issue for `wasmtime` → check `workspace.dependencies` in root `Cargo.toml`.

---

## Step 4 — Orchestration split (sub-steps 4a–e)

**Reframe (from original "bulk move" plan).** The Category-B classification of `arena.*` accessors (pre-Step-4 survey, 18/18 B) revealed that Step 4 cannot be a pure file move of `dispatch.rs`. The existing `impl LayerStageRunner for WasmRuntimeDispatcher::run_stage` body (~111 LOC) intermixes the wasmtime call (Category A-equivalent: belongs in wasm-host) with pre-call IR marshalling and post-call `commit_layer_outputs` (Category B: belongs in runtime). Step 4 therefore splits orchestration along the WIT seam:

- **moves to slicer-wasm-host**: `wit_host.rs`, `wasm_instance.rs`, `instance_pool.rs`, the `call_*` infrastructure from `dispatch.rs`, the four bindgen `Host` trait impls, `HostExecutionContext` + builder, the four runner trait defs, `CompiledModuleLive`.
- **stays in slicer-runtime**: the pre-call IR marshalling (reads `&LayerArena` to build `LayerStageInput<'_>`) and post-call `commit_layer_outputs` (consumes `LayerStageCommitData`, writes `&mut LayerArena`) — both relocate from `dispatch.rs::run_stage` body to `crates/slicer-runtime/src/layer_executor.rs` (and the equivalent per-stage executor files for prepass / finalization / postpass).

**Cost.** P83 was budgeted M+; the orchestration split makes it effectively L. Accepted — splitting P83 mid-flight to defer this work would create a multi-month "P83-finishing" follow-up packet, which is worse than absorbing the cost now.

**Precondition.** Steps 0–3 + Step 0.5 (a/b/c/d/e) all green. `LayerStageInput<'_>`, `PrepassStageInput<'_>`, `FinalizationStageInput<'_>`, `PostpassStageInput<'_>` (input borrow structs) and `LayerStageCommitData` (commit struct) defined in `slicer-wasm-host`/`slicer-ir` respectively (the wasm-host borrow structs land in 4a; `LayerStageCommitData` already lives in slicer-ir from 0.5e).

**Falsifying checks (umbrella).**
- **4b leak check:** any line in `slicer-wasm-host`'s `run_stage` impl that touches `LayerArena` is wrong (the runner trait must not see `LayerArena`).
- **4d leak check:** any line in `layer_executor.rs`'s commit path that imports `slicer_wasm_host::HostExecutionContext` is wrong (commit consumes `LayerStageCommitData`, not the wasm-host-internal builder).

### Step 4a — File moves: bindgen + wasmtime infrastructure to `slicer-wasm-host`

**Objective.** Move the wasmtime infrastructure files (`wit_host.rs`, `wasm_instance.rs`, `instance_pool.rs`) and define the borrow-struct + trait scaffolding in slicer-wasm-host. dispatch.rs's `WasmRuntimeDispatcher` struct + the `call_*` infrastructure also moves; the dispatcher's `run_stage` impl body is updated to the new shape in 4b. After 4a, slicer-wasm-host builds with the moved content; slicer-runtime still has stale `pub mod` declarations that 4c resolves.

**Files allowed to read.** Line-range only:
- `crates/slicer-runtime/src/wit_host.rs` (5,259 LOC — section-by-section grep + ≤ 200-line reads).
- `crates/slicer-runtime/src/dispatch.rs` (3,148 LOC — focus on imports, the `WasmRuntimeDispatcher` struct, the `call_*` helpers, and the four runner-trait impls; the impl BODIES become inputs to 4b/4c/4d).
- `crates/slicer-runtime/src/wasm_instance.rs` (≤ 300 LOC OK to read full).
- `crates/slicer-runtime/src/instance_pool.rs` (≤ 200 LOC OK to read full).
- The four trait-source files for trait-def lifts (executor / prepass / postpass / layer_finalization) — just the trait declarations.

**Files allowed to edit.**
1. `crates/slicer-wasm-host/src/host.rs` ← `wit_host.rs` content (verbatim; preserve the four `bindgen!` invocations with the `with:` remap pattern; `pub mod layer` MUST precede the other three).
2. `crates/slicer-wasm-host/src/instance.rs` ← `wasm_instance.rs` content (verbatim).
3. `crates/slicer-wasm-host/src/pool.rs` ← `instance_pool.rs` content (verbatim, with the Step-0.5d-narrowed `build_wasm_instance_pool` signature).
4. `crates/slicer-wasm-host/src/dispatch.rs` ← the `WasmRuntimeDispatcher` struct + `call_*` helpers + the bindgen-Host trait impls + the wasmtime-call inner machinery. `export_name_for_stage` is DELETED here; callers will rewire to `slicer_schema::export_for_stage_id`. The four `impl *StageRunner for WasmRuntimeDispatcher` blocks are STUBBED with a `todo!("4b")` placeholder body (their full implementations land in 4b once the trait sigs + return-type deconstruction are wired).
5. `crates/slicer-wasm-host/src/traits.rs` — declare the four runner traits with their new IR-typed signatures: each `run_stage` (or `run_gcode_postprocess` / `run_text_postprocess`) takes `&CompiledModuleLive<'_>` plus the matching `*StageInput<'_>` borrow struct, and returns `Result<…CommitData / LayerStageOutput, …Error>` per the user's resolution table.
6. `crates/slicer-wasm-host/src/binding.rs` — define `CompiledModuleLive<'a>` (5 fields: `module_id: &'a ModuleId`, `instance_pool: Arc<WasmInstancePool>`, `wasm_component: Option<Arc<WasmComponent>>`, `claims: &'a [String]`, `config_view: Arc<ConfigView>`) plus the four `*StageInput<'a>` borrow structs (precise field lists from the Blackboard/LayerArena access survey).
7. `crates/slicer-wasm-host/src/lib.rs` — public re-exports.
8. Delete the four files from `crates/slicer-runtime/src/`.

**Gate.** `cargo build -p slicer-wasm-host` green (with `todo!()` stubs in the runner impls — the crate compiles; runtime side doesn't yet). `grep -c 'wasmtime::component::bindgen!' crates/slicer-wasm-host/src/host.rs` = 4. `grep -c '"slicer:types/geometry": super::layer::slicer::types::geometry' crates/slicer-wasm-host/src/host.rs` = 3.

### Step 4b — Implement runner trait impls in wasm-host with `HostExecutionContext` → IR deconstruction

**Objective.** Replace the `todo!("4b")` stubs in each `impl *StageRunner for WasmRuntimeDispatcher` block with the wasmtime call + inline deconstruction of `HostExecutionContext` into the IR-typed return type (`LayerStageCommitData` for Layer; analogous narrow shapes for prepass / finalization / postpass, which may not need a CommitData struct if their return is simpler).

**Files allowed to edit.** `crates/slicer-wasm-host/src/dispatch.rs` (the four impl bodies), `crates/slicer-wasm-host/src/binding.rs` if helper deconstruction fns belong there.

**Falsifying check (leak).** Any `arena.*`, `blackboard.*`, or `&LayerArena` / `&Blackboard` reference inside the moved impl bodies is a leak — those have to relocate to 4c/4d (runtime-side orchestration), not stay in wasm-host.

**Gate.** `cargo build -p slicer-wasm-host` green (no more `todo!()`s).

### Step 4c — Move pre-call IR marshal logic from dispatch.rs to layer_executor.rs

**Objective.** The 4 B-pre sites (L438, L1574, L1603, L2434 in the original dispatch.rs) move to `crates/slicer-runtime/src/layer_executor.rs`. The layer executor now: (1) reads `&LayerArena` and `&Blackboard` slots, (2) constructs `LayerStageInput<'_>`, (3) invokes `runner.run_stage(stage_id, layer, &live_module, input) -> LayerStageCommitData`. Equivalent pre-call marshalling for prepass / finalization / postpass moves to their respective executor files.

**Files allowed to edit.** `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs` — replace each file's existing `runner.run_stage(...)` call with the new pre-call marshal + invoke pattern. Each file's body otherwise stays unchanged.

**Gate.** `cargo build --workspace` green (post-4a runtime stale-module errors should be resolved when 4c lands the new invocation pattern that imports the trait + structs from `slicer_wasm_host`).

### Step 4d — Move `commit_layer_outputs` to layer_executor.rs; consume `LayerStageCommitData`

**Objective.** The 14 B-post sites (L2343..L2833 in original dispatch.rs) move into `commit_layer_outputs` which itself relocates to `crates/slicer-runtime/src/layer_executor.rs`. The function signature changes from `(ctx: HostExecutionContext, …)` to `(commit: LayerStageCommitData, …)`. All `ctx.*` accesses become `commit.*` (IR-typed). Equivalent post-call commit logic for prepass / finalization / postpass moves to their respective executor files (likely smaller, since only Layer has the heavy commit surface).

**Files allowed to edit.** `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs`.

**Falsifying check (leak).** `! grep -E 'use slicer_wasm_host::HostExecutionContext' crates/slicer-runtime/src/` — runtime must NOT import wasm-host's `HostExecutionContext`. The runtime-side commit path consumes `slicer_ir::LayerStageCommitData` only.

**Gate.** `cargo build --workspace` green. `cargo clippy --workspace --all-targets -- -D warnings` green.

### Step 4e — Combined gate

- `cargo build --workspace` green.
- `cargo clippy --workspace --all-targets -- -D warnings` green.
- `cargo test -p slicer-runtime --tests` builds (test rewiring per Step 6 may still be needed for runtime).
- `grep -c 'wasmtime::component::bindgen!' crates/slicer-wasm-host/src/host.rs` = 4.
- `! grep -rE 'use slicer_wasm_host::HostExecutionContext' crates/slicer-runtime/src/`.
- `! grep -rE 'fn run.*HostExecutionContext|HostExecutionContext.*->|->.*HostExecutionContext' crates/slicer-wasm-host/src/` (AC-4 symmetric-boundary assertion).

**Context cost (umbrella for 4a–e): L.** Highest of any single step in P83. Implementer monitors context budget — if 60% is hit during 4b–4d, surface and PARTIAL-handoff at the cleanest sub-step boundary (typically between 4b and 4c, since 4a/4b together produce a buildable wasm-host crate independent of runtime rewire).

---

## Step 5 — Update `slicer-runtime`: remove `pub mod`s, rewire trait imports, split `CompiledModule`, swap Cargo.toml deps

**Objective.** Make `slicer-runtime` compile against the new crate. After this step, the workspace builds.

**Precondition.** Step 4 complete; `slicer-wasm-host` builds.

**Postcondition.** `cargo build --workspace` green. `slicer-runtime` no longer declares `wasmtime` directly. `CompiledModule` renamed to `CompiledModuleStatic` with `pub type CompiledModule = CompiledModuleStatic;` alias. The four executor files import their runner trait from `slicer_wasm_host::`.

**Files allowed to read.**
- `crates/slicer-runtime/src/lib.rs` (full).
- `crates/slicer-runtime/src/execution_plan.rs` (L650–730 around `CompiledModule`).
- `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs` — only the trait declaration lines + their callers (already located in Step 1).
- `crates/slicer-runtime/src/dag_cli.rs` — only if Step 1 dispatch #4 surfaced it as a caller of `export_name_for_stage`.
- `crates/slicer-runtime/Cargo.toml`.

**Files allowed to edit.**
1. `crates/slicer-runtime/src/lib.rs` — drop four `pub mod`s + their `pub use`s; add `pub use slicer_wasm_host::{…}` re-exports.
2. `crates/slicer-runtime/src/execution_plan.rs` — rename struct; delete wasmtime fields/accessors; add type alias.
3. `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization}.rs` — delete local trait decl; add `use slicer_wasm_host::*StageRunner;`.
4. `crates/slicer-runtime/Cargo.toml` — remove `wasmtime`; add `slicer-wasm-host`.
5. `crates/slicer-runtime/src/dag_cli.rs` — switch `export_name_for_stage` calls to `slicer_schema::export_for_stage_id` if needed.

**Expected sub-agent dispatches.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail + first failing crate.
- Dispatch: `cargo clippy --workspace --all-targets -- -D warnings`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.** Workspace builds; clippy clean.

**Falsifying check / exit condition.** Build error referencing a missing trait → confirm the runner-trait `use slicer_wasm_host::*StageRunner;` line in each executor file.

---

## Step 6 — Migrate or rewire tests in `slicer-runtime/tests/`

**Objective.** Tests that imported `wit_host::*`, `dispatch::*`, etc., now import from `slicer_wasm_host::`. Tests that constructed `CompiledModule` with `instance_pool: ...` fields now construct `CompiledModuleStatic` + a separate `CompiledModuleLive`.

**Precondition.** Step 5 complete.

**Postcondition.** `cargo test -p slicer-runtime` green (without `--workspace`).

**Files allowed to read.** The test files surfaced in Step 1 dispatch #2.
**Files allowed to edit.** Those same test files; `crates/slicer-runtime/tests/{integration,executor}/main.rs` aggregators if any `mod` declarations need adjusting.

**Expected sub-agent dispatches.**
- Dispatch: `cargo test -p slicer-runtime`. Return FACT pass/fail + count + delta vs pre-packet.

**Context cost: M.**

**Narrow verification.** Test runs pass; count delta near zero.

**Falsifying check / exit condition.** A test that previously passed now fails on type-construction → the `CompiledModuleStatic`/`Live` split needs more work.

---

## Step 7 — Rebuild guest WASMs and confirm `--check` clean

**Objective.** Step 2's schema edit invalidated guests; rebuild them. Confirm `--check` reports zero STALE after rebuild.

**Precondition.** Step 5 complete (otherwise the host build doesn't even succeed).

**Postcondition.** `cargo xtask build-guests --check` reports clean for all guests.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: `cargo xtask build-guests`. Return FACT pass/fail + duration.
- Dispatch: `cargo xtask build-guests --check`. Return FACT clean/STALE-list.

**Context cost: S.**

**Narrow verification.** Both dispatches green; `--check` reports no STALE entries.

**Falsifying check / exit condition.** A guest fails to build → the schema edit was wrong; investigate (likely cause: a typo in `export_for_stage_id` body).

---

## Step 8 — Workspace test gate (checkpoint)

**Objective.** Confirm the full ~1 000-test suite passes. Per the deepening-batch policy, this gate runs at P83 (checkpoint).

**Precondition.** Steps 1–7 green.

**Postcondition.** `cargo test --workspace` passes; count delta vs Step 0 baseline within +1/-1 (any larger delta investigated and explained in the log).

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: `cargo test --workspace 2>&1 | tail -5`. Return SNIPPET (≤ 5 lines: count + final result). Then FACT pass/fail + count delta.

**Context cost: M.**

**Narrow verification.** Pass; count delta within ±1.

**Falsifying check / exit condition.** Any test fails → triage by test name. Likely causes (in order of probability): (1) stale guest from Step 7 missed; (2) `CompiledModule` construction site in a test missed in Step 6; (3) `wasmtime::` direct import in a non-moved file missed in Step 5; (4) bindgen `with:` remap pattern subtly wrong.

---

## Step 9 — Post-packet g-code SHA parity and AC-8 dep-tree assertion

**Objective.** Confirm the byte-identical g-code SHA carries through; confirm `slicer-runtime` has no direct wasmtime dep.

**Precondition.** Step 8 green.

**Postcondition.** Post-packet SHA = Step 0 baseline SHA. `cargo tree -p slicer-runtime --depth 1` does NOT list `wasmtime`.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p83.gcode && sha256sum /tmp/benchy-p83.gcode`. Return FACT `<hex>`.
- Dispatch: `cargo tree -p slicer-runtime --depth 1 --edges normal`. Return SNIPPET (≤ 30 lines).

**Context cost: S.**

**Narrow verification.** SHAs match. Dep-tree depth-1 listing contains `slicer-wasm-host`, does NOT contain `wasmtime`.

**Falsifying check / exit condition.** SHA divergence → bisect via temporary reverts of Step 4 sections to isolate the divergent edit. Dep-tree mentions `wasmtime` directly → check `crates/slicer-runtime/Cargo.toml` for stray `wasmtime = ...` line.

---

## Per-Step Budget Roll-Up

| Step | Cost |
|---|---|
| 0 Verify P81/P82 + baselines | S |
| 0.5 Prework: stage I/O three-group split + narrow runner error + instance_pool narrowing + `LayerStageCommitData` (5 sub-steps a–e) | S |
| 1 Enumerate edit sites | S |
| 2 Schema `export_for_stage_id` + test | S |
| 3 New crate scaffold | S |
| 4 Orchestration split — 5 sub-steps (4a file moves; 4b runner impls w/ HEC → IR deconstruction; 4c pre-call marshal moves to runtime; 4d post-call commit moves to runtime; 4e combined gate) | **L** (largest single step in P83) |
| 5 Runtime rewire + Cargo.toml swap | M |
| 6 Test migration / rewiring | M |
| 7 Guest rebuild + `--check` clean | S |
| 8 Workspace test gate | M |
| 9 g-code SHA + dep-tree check | S |

Aggregate: **L overall; Step 4 is the single L step** after the orchestration-split reframe (the 18/18 Category-B classification of `arena.*` accessors forced run_stage's body to split across the wasm-host/runtime boundary along the WIT seam). Total step count: 11 atomic steps + 9 sub-steps (Step 0.5 a–e and Step 4 a–e).

## Packet Completion Gate

This is a checkpoint packet — workspace tests run at close per the deepening-batch policy.

1. `cargo build --workspace` — green.
2. `cargo clippy --workspace --all-targets -- -D warnings` — green.
3. `cargo xtask build-guests` (rebuild) green, then `cargo xtask build-guests --check` clean.
4. `cargo test --workspace` — green; count delta within ±1.
5. AC-9 post-packet SHA = Step 0 baseline.
6. AC-8 dep-tree depth-1 listing does NOT include `wasmtime`.
7. ADR-0004 and ADR-0005 drafted in `docs/adr/` and reviewed before status flip.

## Acceptance Ceremony

- All 11 ACs (AC-1 .. AC-11) and 3 negative cases (AC-N1, AC-N2, AC-N3) gate green per the inline verification commands in `packet.spec.md`.
- ADR-0004 (`docs/adr/0004-runner-traits-in-slicer-wasm-host.md`) and ADR-0005 (`docs/adr/0005-export-for-stage-id-sole-lookup.md`) written and committed.
- Implementation log records: Step 0 baseline SHA, Step 9 post-packet SHA, pre/post workspace test counts, guest-rebuild duration, list of files moved (count and total LOC), list of `pub trait *StageRunner` declarations lifted (4 expected), list of `export_name_for_stage` call sites collapsed (per Step 1 dispatch #4).
- `status: draft` → `status: implemented` once gate green AND ADRs in place AND user confirms closure. (`superseded` is reserved for packets later replaced by another spec; `implemented` is the terminal state for a closed packet.)

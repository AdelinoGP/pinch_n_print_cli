# Implementation Plan: 28_tree-support-multi-layer-propagation

## Step 1 — Discovery: read PrePass precedent + OrcaSlicer reference

**Task IDs**: TASK-161
**Objective**: Understand the structural precedent (how `PrePass::SeamPlanning` was added in packet `23-rev1`) and the OrcaSlicer algorithmic reference before touching any code.
**Precondition**: None.
**Postcondition**: Known: (a) exact file list edited by packet 23-rev1 to add `run-seam-planning`; (b) the `SeamPlanIR` struct layout and its `SeamPlanEntry` keying pattern; (c) the `ensure_stage_prerequisites` match arm style; (d) OrcaSlicer's `detect_overhangs` and `drop_nodes` control flow summarized in design-note form (no code copy).
**Files**: `.ralph/specs/23-rev1_prepass-seam-planning-orca-parity/implementation-plan.md`, `crates/slicer-ir/src/slice_ir.rs` (SeamPlanIR/SeamPlanEntry), `crates/slicer-host/src/prepass.rs` (PrepassStageOutput + ensure_stage_prerequisites), `crates/slicer-host/src/blackboard.rs` (seam_plan slot pattern), `modules/core-modules/seam-planner-default/` (whole crate layout), `wit/world-prepass.wit` (run-seam-planning block), `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` (lines around `detect_overhangs` and line 2625 `drop_nodes`).
**Verification**: Discovery notes captured; no code changes. `git status` still clean for this packet's file surface.
**Exit**: Engineer can name the five host/IR sites that need new arms and can sketch the simplified `drop_nodes` loop without re-reading Orca source.
**OrcaSlicer refs**: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` lines 2608–2820 (propagation block including MST grouping).

## Step 2 — Add `SupportPlanIR` + `SupportPlanEntry` to `slicer-ir`

**Task IDs**: TASK-161
**Objective**: Introduce the new IR types in `slice_ir.rs` and re-export from the crate root. Include `schema_version: SemVer { major: 1, minor: 0, patch: 0 }`. Precedence: mirror `SeamPlanIR` / `SeamPlanEntry`.
**Precondition**: Step 1 complete.
**Postcondition**: `pub struct SupportPlanIR { schema_version: SemVer, entries: Vec<SupportPlanEntry> }` and `pub struct SupportPlanEntry { global_layer_index: u32, object_id: ObjectId, region_id: RegionId, branch_segments: Vec<ExtrusionPath3D> }` are defined in `slice_ir.rs`; both are re-exported from `crates/slicer-ir/src/lib.rs`; `cargo build -p slicer-ir` succeeds.
**Files**: `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-ir/src/lib.rs`.
**Verification**: `grep -nE 'pub struct SupportPlanIR|pub struct SupportPlanEntry' crates/slicer-ir/src/slice_ir.rs` returns 2 matches; `grep -nE 'SupportPlanIR|SupportPlanEntry' crates/slicer-ir/src/lib.rs` returns ≥1 match; `cargo build -p slicer-ir 2>&1 | tail -5` exits 0.
**Exit**: Build green; both types reachable as `slicer_ir::SupportPlanIR` and `slicer_ir::SupportPlanEntry`.
**OrcaSlicer refs**: None (IR layout is repo-internal).

## Step 3 — Extend `wit/world-prepass.wit` with `run-support-generation`

**Task IDs**: TASK-161
**Objective**: Add the `support-generation-output` resource, `support-plan-entry` record, and `export run-support-generation: func(objects: list<mesh-object-view>, output: support-generation-output, config: config-view) -> result<_, module-error>;` to the canonical prepass world. Mirror the shape of `run-seam-planning`.
**Precondition**: Step 2 complete.
**Postcondition**: `wit/world-prepass.wit` contains the new records and export; all WIT consumers still parse it (wit-parser passes on the file).
**Files**: `wit/world-prepass.wit`.
**Verification**: `grep -nE 'run-support-generation|support-generation-output|support-plan-entry|push-support-plan|branch-segments' wit/world-prepass.wit` returns ≥5 matches; `cargo build --workspace 2>&1 | tail -10` exits 0 (WIT host + macro must consume the new world cleanly).
**Exit**: Build green with the extended WIT.
**OrcaSlicer refs**: None (WIT is repo-internal).

## Step 4 — Wire `PrepassStageOutput::SupportPlan` + blackboard slot + prerequisite chain

**Task IDs**: TASK-161
**Objective**: Extend the host prepass plumbing so a `SupportPlanIR` emitted by a PrePass module is committed to a new blackboard slot and exposes the same audit/error surface as `SeamPlanIR`.
**Precondition**: Steps 2–3 complete.
**Postcondition**:
- `PrepassStageOutput::SupportPlan(Arc<SupportPlanIR>)` variant present.
- `ir_path_for_prepass_output` returns `Some(String::from("SupportPlanIR"))` for it.
- `commit_stage_output` routes the variant into a new `Blackboard::commit_support_plan`.
- `Blackboard::support_plan()` accessor returns `Option<Arc<SupportPlanIR>>`.
- `BlackboardPrepassSlot::SupportPlan` variant added and included in every exhaustive match.
- `ensure_stage_prerequisites` returns `&[BlackboardPrepassSlot::SurfaceClassification, BlackboardPrepassSlot::LayerPlan]` for `"PrePass::SupportGeneration"`.
**Files**: `crates/slicer-host/src/prepass.rs`, `crates/slicer-host/src/blackboard.rs`.
**Verification**: `grep -nE 'SupportPlan\(Arc<SupportPlanIR>\)|"SupportPlanIR"|commit_support_plan|BlackboardPrepassSlot::SupportPlan' crates/slicer-host/src/prepass.rs crates/slicer-host/src/blackboard.rs` returns ≥6 matches; `cargo test -p slicer-host --lib 2>&1 | tail -10` exits 0.
**Exit**: Host library tests still pass; new variants reachable.
**OrcaSlicer refs**: None.

## Step 5 — Implement the host-side WIT dispatcher for `run-support-generation`

**Task IDs**: TASK-161
**Objective**: In the prepass runtime dispatcher (see `crates/slicer-host/src/wit_host.rs` and the prepass entry in `WasmRuntimeDispatcher`), add the glue that calls the `run-support-generation` export of a PrePass module: feed `list<MeshObjectView>` + a `support-generation-output` resource implementation, collect pushed `support-plan-entry` values into a `SupportPlanIR`, return as `PrepassStageOutput::SupportPlan(Arc::new(ir))`.
**Precondition**: Step 4 complete.
**Postcondition**: `WasmRuntimeDispatcher` routes `PrePass::SupportGeneration` to the new glue; a minimal test-guest component that pushes one `support-plan-entry` produces a committed `SupportPlanIR` via `execute_prepass`.
**Files**: `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/dispatch.rs` (or whichever file hosts the prepass branch — follow the `run-seam-planning` precedent).
**Verification**: `cargo test -p slicer-host --lib prepass 2>&1 | tail -20` runs with new variant coverage; `cargo build --workspace 2>&1 | tail -5` exits 0.
**Exit**: Build green; `execute_prepass` path is callable end-to-end for a `PrePass::SupportGeneration` stage.
**OrcaSlicer refs**: None.

## Step 6 — Extend the SDK `PrepassModule` trait with `run_support_generation`

**Task IDs**: TASK-161
**Objective**: Add `fn run_support_generation(&self, ...) -> Result<(), ModuleError>` to the `PrepassModule` trait in `slicer-sdk`, with a default body that returns `Err(ModuleError::unimplemented("run_support_generation"))`. Extend the `#[slicer_module]` macro's stage map so that a module whose manifest declares `stage.id = "PrePass::SupportGeneration"` routes to this method. Provide a matching builder (`SupportPlanOutputBuilder` or equivalent — follow the existing builder pattern in `crates/slicer-sdk/src/builders.rs`).
**Precondition**: Step 5 complete.
**Postcondition**: Existing prepass modules (e.g. `seam-planner-default`) still compile without change. A new core-module crate declaring the new stage routes to the new method.
**Files**: `crates/slicer-sdk/src/traits.rs`, `crates/slicer-sdk/src/builders.rs`, `crates/slicer-macros/src/lib.rs` (if the stage map lives there).
**Verification**: `cargo build --workspace 2>&1 | tail -5` exits 0; `grep -nE 'run_support_generation' crates/slicer-sdk/src/traits.rs crates/slicer-macros/src/lib.rs` returns ≥2 matches.
**Exit**: Build green; SDK exposes the new hook.
**OrcaSlicer refs**: None.

## Step 7 — Create `modules/core-modules/support-planner/` crate

**Task IDs**: TASK-161
**Objective**: Scaffold the new core-module crate with Cargo manifest, module manifest, `wit-guest/` shim, and an `impl PrepassModule for SupportPlanner` that stubs `run_support_generation` with an empty `Ok(())`. Add the entry to `build-core-modules.sh`.
**Precondition**: Step 6 complete.
**Postcondition**: `bash modules/core-modules/build-core-modules.sh` builds `support-planner.wasm` without error; manifest declares `PrePass::SupportGeneration`, `support-planner` claim, reads `MeshIR`+`SurfaceClassificationIR`+`LayerPlanIR`+`PaintRegionIR`, writes `SupportPlanIR`; `bash modules/core-modules/build-core-modules.sh --check` reports the artifact up to date.
**Files**: `modules/core-modules/support-planner/Cargo.toml`, `modules/core-modules/support-planner/support-planner.toml`, `modules/core-modules/support-planner/src/lib.rs`, `modules/core-modules/support-planner/wit-guest/Cargo.toml`, `modules/core-modules/support-planner/wit-guest/src/lib.rs`, `modules/core-modules/build-core-modules.sh`.
**Verification**: `grep -E 'id = "PrePass::SupportGeneration"|holds    = \["support-planner"\]|writes = \["SupportPlanIR"\]|wit-world    = "slicer:world-prepass@1.0.0"' modules/core-modules/support-planner/support-planner.toml` returns 4 matches; `grep -E '"support-planner:support_planner_guest"' modules/core-modules/build-core-modules.sh` returns 1 match; `bash modules/core-modules/build-core-modules.sh 2>&1 | tail -10` exits 0.
**Exit**: Artifact builds; stub returns empty plan.
**OrcaSlicer refs**: None.

## Step 8 — Add host tests (TDD) covering the prepass stage contract

**Task IDs**: TASK-161
**Objective**: Write failing integration tests first. Create `crates/slicer-host/tests/prepass_support_generation_tdd.rs` with the six tests listed in the packet's acceptance criteria (one positive overhang-fixture test + five negatives/determinism).
**Precondition**: Step 7 complete (planner loads cleanly as a stub).
**Postcondition**: Tests compile and fail as expected (positive test fails because stub returns empty plan; negatives already pass against current behavior; determinism fails because no work is done).
**Files**: `crates/slicer-host/tests/prepass_support_generation_tdd.rs`.
**Verification**: `cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -20` runs; `support_planner_produces_branches_for_overhang_fixture` and `support_planner_is_deterministic_across_runs` fail; `support_planner_emits_empty_plan_when_no_overhangs`, `prepass_support_generation_fails_without_layer_plan`, and `support_planner_claim_dedup` pass.
**Exit**: Failing tests exist and target the right module; no implementation yet.
**OrcaSlicer refs**: None.

## Step 9 — Implement simplified `detect_overhangs` in `support-planner`

**Task IDs**: TASK-161
**Objective**: Populate contact points for each layer from `SurfaceClassificationIR` overhang/bridge facets and `PaintRegionIR` `SupportEnforcer` regions; drop points that fall inside `SupportBlocker` polygons. Store `Vec<PlannedSupportNode>` keyed by `(object_id, layer_index)`.
**Precondition**: Step 8 complete.
**Postcondition**: The stub now produces one contact point per overhang facet / enforcer region. `support_planner_emits_empty_plan_when_no_overhangs` still passes. The positive test still fails (contact points exist but propagation and MST still empty).
**Files**: `modules/core-modules/support-planner/src/lib.rs`.
**Verification**: `cargo test -p slicer-host --test prepass_support_generation_tdd support_planner_emits_empty_plan_when_no_overhangs -- --nocapture 2>&1 | tail -10` passes.
**Exit**: Contacts detected deterministically; negative-overhangs case still green.
**OrcaSlicer refs**: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `detect_overhangs` block (structural reference only; simplified port).

## Step 10 — Implement simplified top-down propagation + per-layer MST merging

**Task IDs**: TASK-161
**Objective**: Walk layers top → bottom. Group active nodes by region-part, run Prim MST on each group, execute merge-pass (merge nodes within `support_branch_merge_distance_mm`) and move-pass (move each surviving node by `tan(branch_angle) * layer_height` toward its MST neighbor or toward the nearest contour edge, clamped to the region contour). Emit one `SupportPlanEntry.branch_segments` per group-layer with one `ExtrusionPath3D` per MST edge, using `ExtrusionRole::SupportMaterial`.
**Precondition**: Step 9 complete.
**Postcondition**: `support_planner_produces_branches_for_overhang_fixture` and `support_planner_is_deterministic_across_runs` pass.
**Files**: `modules/core-modules/support-planner/src/lib.rs`.
**Verification**: `cargo test -p slicer-host --test prepass_support_generation_tdd support_planner_produces_branches_for_overhang_fixture support_planner_is_deterministic_across_runs -- --test-threads=1 --nocapture 2>&1 | tail -20` both pass.
**Exit**: All prepass-level ACs green.
**OrcaSlicer refs**: `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `drop_nodes` block (structural reference); `OrcaSlicerDocumented/src/libslic3r/MinimumSpanningTree.cpp` — `prim` (same O(V²) complexity class).

## Step 11 — Update `tree-support` to consume `SupportPlanIR`

**Task IDs**: TASK-161, TASK-120b
**Objective**: Extend `modules/core-modules/tree-support/tree-support.toml` `[ir-access].reads` with `"SupportPlanIR"`. Update `src/lib.rs` `run_support` to look up the committed `SupportPlanIR` for each `(object_id, region_id, layer_index)` via the SDK's layer-view accessor and emit `ExtrusionPath3D` from the plan's `branch_segments`. Preserve the existing grid-MST fallback (packet 26's `fill_expolygon_tree` + `MAX_SAMPLES_PER_EXPOLY` cap) for when no plan entry exists.
**Precondition**: Step 10 complete.
**Postcondition**: Add tests to `crates/slicer-host/tests/live_support_generation_tdd.rs` Section C: `tree_support_consumes_support_plan_ir`, `tree_support_falls_back_to_grid_when_plan_absent`. Both pass.
**Files**: `modules/core-modules/tree-support/tree-support.toml`, `modules/core-modules/tree-support/src/lib.rs`, `crates/slicer-host/tests/live_support_generation_tdd.rs`.
**Verification**: `grep 'SupportPlanIR' modules/core-modules/tree-support/tree-support.toml` returns 1 match; `cargo test -p slicer-host --test live_support_generation_tdd tree_support_consumes_support_plan_ir tree_support_falls_back_to_grid_when_plan_absent -- --test-threads=1 --nocapture 2>&1 | tail -20` both pass; `bash modules/core-modules/build-core-modules.sh 2>&1 | tail -5` exits 0 and rebuilds `tree-support.wasm`.
**Exit**: Tree-support emits plan-driven branches when a plan is present; falls back to grid-MST otherwise.
**OrcaSlicer refs**: None.

## Step 12 — Document traditional-support's per-layer nature + regression assertion

**Task IDs**: TASK-161
**Objective**: Add a module-level `//!` doc comment to `modules/core-modules/traditional-support/src/lib.rs` stating that the module is per-layer-only and that `SupportPlanIR` is intentionally not declared as a read. Add `traditional_support_ignores_support_plan_ir` to `live_support_generation_tdd.rs` asserting byte-identical output with and without a committed `SupportPlanIR`.
**Precondition**: Step 11 complete.
**Postcondition**: Doc comment present; new regression test passes.
**Files**: `modules/core-modules/traditional-support/src/lib.rs`, `crates/slicer-host/tests/live_support_generation_tdd.rs`.
**Verification**: `grep -n 'per-layer scan-line' modules/core-modules/traditional-support/src/lib.rs` returns ≥1 match; `cargo test -p slicer-host --test live_support_generation_tdd traditional_support_ignores_support_plan_ir -- --nocapture 2>&1 | tail -10` passes.
**Exit**: Doc comment landed; regression assertion green.
**OrcaSlicer refs**: None.

## Step 13 — Rebuild all affected WASM artifacts

**Task IDs**: TASK-161
**Objective**: Ensure every downstream `.wasm` binary reflects the new WIT surface.
**Precondition**: Steps 3, 7, 11 complete (WIT change + new planner + tree-support manifest change).
**Postcondition**: All prepass modules still work (WIT change is additive; existing modules continue to compile). `tree-support.wasm` and `support-planner.wasm` rebuilt.
**Files**: all `modules/core-modules/*/wit-guest/` artifacts (indirect). Only the two listed wasm files change.
**Verification**: `bash modules/core-modules/build-core-modules.sh 2>&1 | tail -20` exits 0; `bash modules/core-modules/build-core-modules.sh --check 2>&1 | grep -E 'STALE'` returns 0 matches.
**Exit**: `--check` reports every module up to date.
**OrcaSlicer refs**: None.

## Step 14 — Update `docs/07_implementation_status.md`

**Task IDs**: TASK-161
**Objective**: Add the TASK-161 row (draft line is in `requirements.md`). Do not modify TASK-120 or TASK-120b — this packet neither closes nor supersedes them.
**Precondition**: Steps 1–13 complete.
**Postcondition**: `docs/07_implementation_status.md` contains a `TASK-161` row under Workstream 3 mentioning `PrePass::SupportGeneration` and `SupportPlanIR`.
**Files**: `docs/07_implementation_status.md`.
**Verification**: `grep -n '^- \[.\] TASK-161' docs/07_implementation_status.md` returns exactly 1 match.
**Exit**: Backlog reflects the packet's deliverable.
**OrcaSlicer refs**: None.

## Step 15 — Packet completion gate

**Task IDs**: TASK-161
**Objective**: Run the focused test matrix and workspace checks for Packet 28. Note: the benchy end-to-end suite is NOT in this gate — this packet does not change packet 26's benchy ACs and a full Benchy slice is expensive to run serially.
**Precondition**: Steps 1–14 complete.
**Postcondition**: All focused commands pass.
**Files**: All changed files.
**Verification**:
```
cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10
bash modules/core-modules/build-core-modules.sh --check 2>&1 | tail -10
cargo build --workspace 2>&1 | tail -5
cargo clippy --workspace -- -D warnings 2>&1 | tail -5
```
**Exit**: All five commands exit 0. Packet-close review can proceed.
**OrcaSlicer refs**: None.

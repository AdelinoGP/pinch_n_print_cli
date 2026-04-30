# Packet 31a — Execution Order Bug: Planning Handout

## Context

Packet `31a_support-geometry-prepass-and-layer-height` adds `SupportGeometryIR` and `PrePass::SupportGeometry` to ModularSlicer. The work was partially implemented by swarm workers. The implementation introduced an **execution order bug** that breaks the prepass pipeline.

**Status before this handout:** Implementation is broken. The prepass pipeline fails for end-to-end tests because `PrePass::RegionMapping` and `PrePass::SupportGeneration` are in the wrong order.

---

## What Packet 31a Adds

1. **`SupportGeometryIR`** — new IR type keyed `(global_support_layer_index, object_id, region_id) → Vec<ExPolygon>` committed to blackboard before user prepass stages
2. **`PrePass::SupportGeometry`** — host-built-in prepass that computes coarse support outlines via plane-triangle intersection at support layer boundaries
3. **WIT extension** — `support-geometry-view-entry` + `support-geometry-view` records; `support-geometry` param added to `export run-support-generation`
4. **Config keys** — `support_layer_height_mm` and `support_top_z_distance_mm` on both `support-planner.toml` and `tree-support.toml`
5. **Manifest updates** — `support-planner.toml [ir-access].reads` adds `"SupportGeometryIR"`
6. **Support interpolation** — planner reads `SupportGeometryView` and interpolates to model resolution near column tops

---

## What Was Implemented (by swarm workers)

### Worker A (Steps 1–9): DONE
- Step 1: Discovery — LayerPlanIR schema confirmed ✓
- Step 2: Q resolutions confirmed in design.md ✓
- Step 3: `SupportGeometryIR` + `SupportGeometryKey` added to `crates/slicer-ir/src/slice_ir.rs` ✓
- Step 4: SDK types (`SupportGeometryView`, `SupportGeometryViewEntry`) added to `crates/slicer-sdk/src/prepass_types.rs`; WIT stub records added ✓
- Step 5: `BlackboardPrepassSlot::SupportGeometry` + `commit_support_geometry` + `support_geometry()` added to `crates/slicer-host/src/blackboard.rs` ✓
- Step 6: `PrePass::SupportGeometry` built-in implemented in `crates/slicer-host/src/prepass.rs` (in `execute_prepass_with_builtins`) ✓
- Step 7: WIT `support-geometry-view` records + `support-geometry` param added to `export run-support-generation` in `wit/world-prepass.wit` ✓
- Step 8: SDK trait + macro + projector wiring in `crates/slicer-host/src/wit_host.rs`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-sdk/src/traits.rs` ✓
- Step 9: `required_slots("PrePass::SupportGeneration")` extended to `[SurfaceClassification, LayerPlan, RegionMap, SupportGeometry]` ✓

### Worker B (Steps 10–17): PARTIAL
- Steps 10–14: Some work done (manifests updated, docs/07 updated, support interpolation added to planner lib)
- Step 15 (WASM rebuild): DONE — all `.wasm` rebuilt ✓
- Step 16 (docs/07): DONE ✓
- Step 17 (completion gate): NOT DONE

### Planner fix work (me, after workers):
- Fixed test `prepass_support_generation_fails_without_region_map` → renamed to `prepass_support_generation_succeeds_with_builtin_region_mapping` (updated to new execution semantics)
- Fixed clippy error in helper function
- **Attempted two-phase execution fix** in `execute_prepass_with_builtins` — THIS IS THE PROBLEMATIC CHANGE

---

## The Bug: Execution Order

### Correct order (from old code, pre-31a):
```
1. MeshAnalysis built-in (if SurfaceClassification missing)
2. execute_prepass (all user prepass stages, including PrePass::LayerPlanning)
   → LayerPlanIR committed AT THE START of execute_prepass, before any stages run
3. RegionMapping built-in (if LayerPlanIR present AND RegionMap missing)
```

`RegionMapping` ran **after** `execute_prepass`. Since `LayerPlanIR` is committed at the very start of `execute_prepass` (before any stages), `RegionMapping` always found `LayerPlanIR` present.

### What the 31a implementation did (WRONG):
The 31a implementation added `SupportGeometry` AND moved `RegionMapping` to **before** `execute_prepass`:

```
1. MeshAnalysis built-in
2. SupportGeometry built-in (NEW — requires LayerPlanIR)
3. RegionMapping built-in (MOVED — requires LayerPlanIR)  ← WRONG
4. execute_prepass
```

This broke the pipeline because:
- `RegionMapping` needs `LayerPlanIR` to exist
- `LayerPlanIR` is only committed **during** `execute_prepass` (by `PrePass::LayerPlanning`)
- If the execution plan does NOT include `PrePass::LayerPlanning` (e.g., tree-support pipeline), `LayerPlanIR` is never committed before `RegionMapping` runs
- `RegionMapping` fails with "requires committed LayerPlanIR"

### Additionally: PrePass::SupportGeneration prerequisite is now wrong

The current `required_slots("PrePass::SupportGeneration")` is:
```
[SurfaceClassification, LayerPlan, RegionMap, SupportGeometry]
```

But `SupportGeneration` was historically a **post-RegionMapping** stage. Its `RegionMap` prerequisite can only be satisfied if `RegionMapping` runs AFTER `execute_prepass` (when `LayerPlanIR` is definitely committed).

**The correct order must be:**
```
PrePass::LayerPlanning → commits LayerPlanIR
PrePass::RegionMapping → reads LayerPlanIR, commits RegionMap  
PrePass::SupportGeneration → reads RegionMap (AND SupportGeometry)
```

---

## Files That Need Changes

### Critical Bug Fix Required

**`crates/slicer-host/src/prepass.rs`** — The `execute_prepass_with_builtins` function has the wrong execution order. The changes introduced by Worker A (Steps 6, 9) and the planner's attempted two-phase fix need to be reverted to the old execution order:

Old correct order (restore this):
```rust
pub fn execute_prepass_with_builtins(...) {
    // 1. MeshAnalysis
    if blackboard.surface_classification().is_none() {
        let ir = execute_mesh_analysis(...)?;
        blackboard.commit_surface_classification(...)?;
    }
    // 2. execute_prepass (all user stages, including LayerPlanning which commits LayerPlanIR)
    let audits = execute_prepass(plan, blackboard, runner)?;
    // 3. RegionMapping AFTER execute_prepass (LayerPlanIR now definitely exists)
    if blackboard.layer_plan().is_some() && blackboard.region_map().is_none() {
        commit_region_mapping_builtin(plan, blackboard)?;
    }
    // 4. SupportGeometry AFTER execute_prepass (LayerPlanIR now definitely exists)
    if blackboard.support_geometry().is_none() && blackboard.layer_plan().is_some() {
        commit_support_geometry_builtin(blackboard)?;
    }
    Ok(audits)
}
```

The `PrePass::SupportGeometry` built-in was added BEFORE `execute_prepass`. This is wrong — `SupportGeometry` also needs `LayerPlanIR`, and `LayerPlanIR` is only committed during `execute_prepass`. **Move `SupportGeometry` to AFTER `execute_prepass`, same as `RegionMapping`.**

**Critical:** The `required_slots("PrePass::SupportGeneration")` currently includes `SupportGeometry` and `RegionMap`. If `SupportGeneration` is meant to be the LAST prepass stage, ALL its prerequisites (including `SupportGeometry` and `RegionMap`) must be satisfied before it runs. This means `SupportGeometry` and `RegionMapping` MUST run before `execute_prepass` returns `audits`.

### Alternative (recommended): Keep built-ins BEFORE execute_prepass, but fix the dependency chain

If `SupportGeometry` and `RegionMapping` run before `execute_prepass`, they need `LayerPlanIR`. But `LayerPlanIR` is committed AT THE START of `execute_prepass`. The solution is:

1. Call `execute_prepass` FIRST (with no stages, or with a minimal plan) to commit `LayerPlanIR`
2. Then call `SupportGeometry` built-in
3. Then call `RegionMapping` built-in
4. Then call `execute_prepass` again with all stages (BUT: this causes re-execution of built-in stages, which may not be idempotent)

**This is complex and error-prone.** The simplest fix is to restore the old execution order where `RegionMapping` and `SupportGeometry` run AFTER `execute_prepass`.

### Files with Implementation That Is Likely Correct (preserve)
- `crates/slicer-ir/src/slice_ir.rs` — `SupportGeometryIR` type definition (Step 3)
- `crates/slicer-ir/src/lib.rs` — re-export (Step 3)
- `crates/slicer-host/src/blackboard.rs` — slot + accessor (Step 5)
- `wit/world-prepass.wit` — WIT records + extended `run-support-generation` (Step 7)
- `crates/slicer-sdk/src/prepass_types.rs` — SDK types (Step 4)
- `crates/slicer-sdk/src/prelude.rs` — re-exports (Step 4)
- `crates/slicer-sdk/src/traits.rs` — trait extension (Step 8)
- `crates/slicer-macros/src/lib.rs` — macro threading (Step 8)
- `crates/slicer-host/src/wit_host.rs` — projector + dispatch wiring (Step 8)
- `crates/slicer-host/src/prepass.rs` — `PrepassStageOutput::SupportGeometry` enum variant, `ensure_stage_prerequisites` match arm for `SupportGeometry`, `ir_path_for_prepass_output` match arm
- `crates/slicer-host/src/prepass.rs` — `PrepassExecutionError::SupportGeometry` error variant
- `crates/slicer-host/src/support_geometry.rs` — NEW FILE: `PrePass::SupportGeometry` built-in implementation (Step 6)
- `modules/core-modules/support-planner/support-planner.toml` — config schema + ir-access (Steps 11)
- `modules/core-modules/tree-support/tree-support.toml` — config schema (Step 12)
- `modules/core-modules/support-planner/src/lib.rs` — support interpolation (Step 13)
- `crates/slicer-host/tests/prepass_support_generation_tdd.rs` — modified to add `SupportGeometryIR` to imports
- `docs/07_implementation_status.md` — TASK-163 row updated

### Files That Need Reverted/Fixed Execution Order
- `crates/slicer-host/src/prepass.rs` — **`execute_prepass_with_builtins` function has wrong execution order**. The function currently runs `SupportGeometry` and `RegionMapping` BEFORE `execute_prepass`. This must be changed to run them AFTER `execute_prepass`.
- `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` — Test `prepass_support_generation_succeeds_with_builtin_region_mapping` (renamed from `prepass_support_generation_fails_without_region_map`) was updated for new execution semantics. If execution order is restored to old behavior, this test may need to be reverted or its expectations updated.

---

## The Correct Fix Strategy

### Option A (Simplest): Restore old execution order
Move BOTH `SupportGeometry` and `RegionMapping` to AFTER `execute_prepass`. This is the original behavior that worked before packet 31a.

```rust
pub fn execute_prepass_with_builtins(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
    runner: &dyn PrepassStageRunner,
) -> Result<Vec<ModuleAccessAudit>, PrepassExecutionError> {
    // 1. MeshAnalysis
    if blackboard.surface_classification().is_none() {
        let ir = execute_mesh_analysis(blackboard.mesh().as_ref())
            .map_err(|source| PrepassExecutionError::MeshAnalysis { source })?;
        blackboard
            .commit_surface_classification(std::sync::Arc::new(ir))
            .map_err(|source| PrepassExecutionError::Blackboard {
                stage_id: "PrePass::MeshAnalysis".to_string(),
                module_id: "<host-built-in>".to_string(),
                source,
            })?;
    }
    // 2. All user prepass stages (including PrePass::LayerPlanning which commits LayerPlanIR)
    let audits = execute_prepass(plan, blackboard, runner)?;
    // 3. RegionMapping AFTER execute_prepass (LayerPlanIR now definitely exists)
    if blackboard.layer_plan().is_some() && blackboard.region_map().is_none() {
        commit_region_mapping_builtin(plan, blackboard)
            .map_err(|source| PrepassExecutionError::RegionMapping { source })?;
    }
    // 4. SupportGeometry AFTER execute_prepass (LayerPlanIR now definitely exists)
    if blackboard.support_geometry().is_none() && blackboard.layer_plan().is_some() {
        commit_support_geometry_builtin(blackboard)
            .map_err(|source| PrepassExecutionError::SupportGeometry { source })?;
    }
    Ok(audits)
}
```

**Key insight:** `LayerPlanIR` is committed AT THE START of `execute_prepass` (before any user stages run). By running `RegionMapping` and `SupportGeometry` AFTER `execute_prepass`, both built-ins always find `LayerPlanIR` present.

### Option B (More Complex): Two-phase with idempotent built-ins
Keep built-ins BEFORE `execute_prepass`, but:
1. Add idempotency guard to `commit_layer_plan_builtin` (check `layer_plan().is_some()` before doing work)
2. Ensure all built-ins are idempotent (they mostly are, but `LayerPlanning` might not be)
3. Call `execute_prepass` twice: first with early stages to commit `LayerPlanIR`, then with late stages

**Recommendation:** Use Option A. It is simpler and preserves the original working behavior.

---

## What the Planning Agent Must Address

1. **Revert execution order in `execute_prepass_with_builtins`** to run `RegionMapping` and `SupportGeometry` AFTER `execute_prepass` (Option A above)
2. **Verify** that the `required_slots("PrePass::SupportGeneration")` change (Step 9) is compatible with the restored execution order
3. **Check** whether the `prepass_support_generation_succeeds_with_builtin_region_mapping` test needs its expectations reverted/changed
4. **Run the full Step 17 verification matrix** after the fix:
   - `cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1`
   - `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1`
   - `cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1`
   - `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --test-threads=1`
   - `cargo test -p support-planner --lib`
   - `cargo build --workspace`
   - `cargo clippy --workspace -- -D warnings`
5. **Do NOT revert** the WIT extension, SDK types, blackboard slot, `SupportGeometryIR` type, or manifest config — these are correct and should be preserved
6. **The `stage_requires_region_map` helper** and the two-phase execution code in `prepass.rs` must be removed as part of the revert

---

## Key Technical Facts

- `LayerPlanIR` is committed at line ~1 of `execute_prepass`, **before any user stages run**
- `RegionMapping` needs `LayerPlanIR` → must run AFTER `execute_prepass`
- `SupportGeometry` also needs `LayerPlanIR` → must run AFTER `execute_prepass`
- `PrePass::SupportGeneration` needs `RegionMap` AND `SupportGeometry` → must be the LAST prepass stage, run after both built-ins
- The tree-support pipeline does NOT include `PrePass::LayerPlanning` in its execution plan, so `LayerPlanIR` is only committed if `LayerPlanning` is a built-in that runs inside `execute_prepass`

---

## Execution Order Contract (correct)

```
execute_prepass_with_builtins:
  1. MeshAnalysis built-in (if SurfaceClassification missing)
  2. execute_prepass (all user prepass stages)
     → LayerPlanIR committed HERE (at start of execute_prepass, before any stages)
     → PrePass::LayerPlanning runs here (if in plan), commits LayerPlanIR
  3. RegionMapping built-in (if LayerPlanIR present AND RegionMap missing)
     → ALWAYS finds LayerPlanIR because it was committed at step 2 start
  4. SupportGeometry built-in (if LayerPlanIR present AND SupportGeometry missing)
     → ALWAYS finds LayerPlanIR because it was committed at step 2 start
  5. Return audits
```

Note: In the ORIGINAL code (before 31a), steps 3 and 4 were combined and ran AFTER `execute_prepass`. This MUST be preserved.

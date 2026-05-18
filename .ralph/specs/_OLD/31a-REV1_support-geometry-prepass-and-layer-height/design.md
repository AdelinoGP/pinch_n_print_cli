# Design: 31a-REV1_support-geometry-prepass-and-layer-height

## Controlling Code Paths

- **`crates/slicer-host/src/prepass.rs::execute_prepass_with_builtins`** — single function containing the bug; fix is a direct replacement of its body.

## Implementation Shape

### Current (broken) execution order

```
pub fn execute_prepass_with_builtins(...) {
    // 1. MeshAnalysis (correct)
    if blackboard.surface_classification().is_none() { ... }
    // 2. SupportGeometry built-in — WRONG: runs before execute_prepass
    if blackboard.support_geometry().is_none() && blackboard.layer_plan().is_some() {
        commit_support_geometry_builtin(blackboard)?;
    }
    // 3. RegionMapping built-in — WRONG: runs before execute_prepass
    let layer_plan_existed = blackboard.layer_plan().is_some();
    if layer_plan_existed && blackboard.region_map().is_none() {
        commit_region_mapping_builtin(...)?;
    }
    // Two-phase execute_prepass (phase-1 early stages, phase-2 late stages) — REMOVE
    // ... stage_requires_region_map splitting logic ...
}
```

**Problem:** `LayerPlanIR` is committed at the start of `execute_prepass` (before any user stages run). If the execution plan lacks `PrePass::LayerPlanning`, `LayerPlanIR` is never committed before the built-ins check for it.

### Fixed execution order (Option A — restore original)

```rust
pub fn execute_prepass_with_builtins(...) {
    // 1. MeshAnalysis (if missing)
    if blackboard.surface_classification().is_none() {
        let ir = execute_mesh_analysis(blackboard.mesh().as_ref())
            .map_err(|source| PrepassExecutionError::MeshAnalysis { source })?;
        blackboard
            .commit_surface_classification(std::sync::Arc::new(ir))
            .map_err(|source| PrepassExecutionError::Blackboard { ... })?;
    }
    // 2. execute_prepass (all user stages, including LayerPlanning which commits LayerPlanIR)
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

**Why this works:** `execute_prepass` calls `commit_layer_plan_builtin` at its very start (before any stages). By the time `RegionMapping` and `SupportGeometry` run, `LayerPlanIR` is guaranteed to be committed.

## Files That Need Changes

### Primary edit (1 file)

- **`crates/slicer-host/src/prepass.rs`** — replace the body of `execute_prepass_with_builtins` with the fixed code above. Remove `stage_requires_region_map` helper function and all two-phase splitting code.

### Secondary edit (1 file, optional)

- **`crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs`** — check test `prepass_support_generation_succeeds_with_builtin_region_mapping`. If it was written for the broken two-phase semantics, update expectations. The test was renamed from `prepass_support_generation_fails_without_region_map` for the new execution semantics.

## Files That Are Correct (preserve untouched)

All other 31a work is correct and must not be reverted:

- `crates/slicer-ir/src/slice_ir.rs` — `SupportGeometryIR` type
- `crates/slicer-ir/src/lib.rs` — re-export
- `crates/slicer-host/src/blackboard.rs` — slot + accessor
- `wit/world-prepass.wit` — WIT records + extended `run-support-generation`
- `crates/slicer-sdk/src/prepass_types.rs` — SDK types
- `crates/slicer-sdk/src/prelude.rs` — re-exports
- `crates/slicer-sdk/src/traits.rs` — trait extension
- `crates/slicer-macros/src/lib.rs` — macro threading
- `crates/slicer-host/src/wit_host.rs` — projector + dispatch wiring
- `crates/slicer-host/src/prepass.rs` — `PrepassStageOutput::SupportGeometry` variant, `ensure_stage_prerequisites` match arm, `ir_path_for_prepass_output` match arm, `PrepassExecutionError::SupportGeometry` variant
- `crates/slicer-host/src/support_geometry.rs` — `PrePass::SupportGeometry` built-in implementation
- `modules/core-modules/support-planner/support-planner.toml` — config schema + ir-access
- `modules/core-modules/tree-support/tree-support.toml` — config schema
- `modules/core-modules/support-planner/src/lib.rs` — support interpolation
- `crates/slicer-host/tests/prepass_support_generation_tdd.rs` — modified to add `SupportGeometryIR` imports
- `docs/07_implementation_status.md` — TASK-163 row

## Architecture Constraints

1. `LayerPlanIR` is committed at line ~1 of `execute_prepass`, before any user stages run. This invariant cannot be changed without breaking the fix.
2. `RegionMapping` requires `LayerPlanIR` → must run after `execute_prepass`.
3. `SupportGeometry` requires `LayerPlanIR` → must run after `execute_prepass`.
4. `PrePass::SupportGeneration` requires both `RegionMap` and `SupportGeometry` → must be last prepass stage.
5. All host built-ins must be idempotent (re-check already-committed slots before doing work).

## Data and Contract Notes

- `required_slots("PrePass::SupportGeneration")` = `[SurfaceClassification, LayerPlan, RegionMap, SupportGeometry]` — unchanged, correct.
- `execute_prepass` returns `Vec<ModuleAccessAudit>` — only user-module audits, not built-in audits.
- `RegionMapping` and `SupportGeometry` built-ins do not produce `ModuleAccessAudit` records (they are not guest modules).

## Risks and Tradeoffs

- **Risk:** Removing the two-phase execution may break any test that was written expecting the broken semantics. **Mitigation:** Run full test matrix; update test expectations as needed.
- **Risk:** If any caller relies on `execute_prepass_with_builtins` being called twice in a row for the same plan, the restored single-phase order may change behavior. **Mitigation:** No such usage exists in the codebase (confirmed by grep).
- **Tradeoff:** Option A (simple revert) was chosen over Option B (two-phase with idempotency guards) because Option B is more complex and error-prone.

## Open Questions

None — all questions resolved by the planning handout.

## Locked Assumptions and Invariants

1. `execute_prepass` commits `LayerPlanIR` at its start, before any user stages run.
2. `RegionMapping` and `SupportGeometry` must not run unless `LayerPlanIR` is present.
3. The tree-support pipeline does not include `PrePass::LayerPlanning` in its execution plan.
4. All built-ins are idempotent (re-check slot presence before doing work).

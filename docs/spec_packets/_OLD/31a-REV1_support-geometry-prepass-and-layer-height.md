---
status: superseded
packet: 31a-REV1_support-geometry-prepass-and-layer-height
task_ids:
  - TASK-163
supersedes: 31a_support-geometry-prepass-and-layer-height
---

# 31a-REV1_support-geometry-prepass-and-layer-height

## Goal

Fix the execution order bug introduced by packet `31a_support-geometry-prepass-and-layer-height` in the prepass pipeline. Packet 31a added `PrePass::SupportGeometry` and extended `required_slots("PrePass::SupportGeneration")`, but the swarm workers' implementation moved `RegionMapping` and `SupportGeometry` to run **before** `execute_prepass`. This breaks the pipeline when `PrePass::LayerPlanning` is absent from the execution plan (e.g., tree-support pipeline), because `LayerPlanIR` is only committed **inside** `execute_prepass`.

The fix restores the original correct execution order: `RegionMapping` and `SupportGeometry` run **after** `execute_prepass`, guaranteeing `LayerPlanIR` is always committed before those built-ins check for it.

## Problem Statement

Packet `31a_support-geometry-prepass-and-layer-height` (swarm workers, Steps 1–17) added `SupportGeometryIR` and `PrePass::SupportGeometry` to Pinch 'n Print. The implementation introduced an **execution order bug** in `execute_prepass_with_builtins`:

- The 31a implementation moved `RegionMapping` and `SupportGeometry` to run **before** `execute_prepass`.
- But `LayerPlanIR` is committed **inside** `execute_prepass` (at the very start, before any user stages run).
- If the execution plan does **not** include `PrePass::LayerPlanning` (e.g., tree-support pipeline), `LayerPlanIR` is never committed before those built-ins run.
- `RegionMapping` checks for `LayerPlanIR` presence and fails with "requires committed LayerPlanIR".

This packet (`31a-REV1`) fixes the execution order by restoring the original correct sequence.

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

## Locked Assumptions and Invariants

1. `execute_prepass` commits `LayerPlanIR` at its start, before any user stages run.
2. `RegionMapping` and `SupportGeometry` must not run unless `LayerPlanIR` is present.
3. The tree-support pipeline does not include `PrePass::LayerPlanning` in its execution plan.
4. All built-ins are idempotent (re-check slot presence before doing work).

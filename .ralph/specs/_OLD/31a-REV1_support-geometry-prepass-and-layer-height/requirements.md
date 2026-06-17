# Requirements: 31a-REV1_support-geometry-prepass-and-layer-height

## Problem Statement

Packet `31a_support-geometry-prepass-and-layer-height` (swarm workers, Steps 1–17) added `SupportGeometryIR` and `PrePass::SupportGeometry` to Pinch 'n Print. The implementation introduced an **execution order bug** in `execute_prepass_with_builtins`:

- The 31a implementation moved `RegionMapping` and `SupportGeometry` to run **before** `execute_prepass`.
- But `LayerPlanIR` is committed **inside** `execute_prepass` (at the very start, before any user stages run).
- If the execution plan does **not** include `PrePass::LayerPlanning` (e.g., tree-support pipeline), `LayerPlanIR` is never committed before those built-ins run.
- `RegionMapping` checks for `LayerPlanIR` presence and fails with "requires committed LayerPlanIR".

This packet (`31a-REV1`) fixes the execution order by restoring the original correct sequence.

## Grouped Task IDs

- `TASK-163` (algorithmic foundation) — execution order fix is a prerequisite for the algorithmic work in 31b.

## In Scope

1. Revert `execute_prepass_with_builtins` execution order in `crates/slicer-host/src/prepass.rs` to run `RegionMapping` and `SupportGeometry` **after** `execute_prepass` (Option A).
2. Remove the two-phase execution code (`stage_requires_region_map` helper) introduced by the planner's attempted fix.
3. Verify or update `prepass_support_generation_succeeds_with_builtin_region_mapping` test expectations.
4. Run the full Step 17 verification matrix; all tests must pass.

## Out of Scope

1. Any change to `SupportGeometryIR` type, blackboard slot, WIT records, SDK types, manifest config keys, planner interpolation — these are correct and preserved from 31a.
2. Any change to `required_slots("PrePass::SupportGeneration")` — already correct.
3. Algorithmic features (avoidance/collision, radius tapering, raft, interface) — these are packet 31b.
4. Any change to `support_geometry.rs` built-in implementation — only its execution placement is wrong.

## Authoritative Docs

- `.ralph/specs/31a_support-geometry-prepass-and-layer-height/PLANNING_HANDOUT.md` — complete bug description, correct execution order contract, Option A fix code.
- `docs/01_system_architecture.md` — Tier 1 PrePass architecture.
- `docs/04_host_scheduler.md` — `ensure_stage_prerequisites` and `required_slots` ordering.
- `.ralph/specs/31a_support-geometry-prepass-and-layer-height/packet.spec.md` — original acceptance criteria (not reverted by this packet).

## OrcaSlicer Reference Obligations

None — execution ordering is an internal Pinch 'n Print concern.

## Acceptance Summary

### Measurable Outcomes

- `execute_prepass_with_builtins` runs `execute_prepass` **before** both built-in calls.
- `stage_requires_region_map` does not appear in `prepass.rs` after the fix.
- All 6 verification test suites pass with exit code 0.
- Workspace builds clean (`cargo build --workspace`).
- Clippy clean (`cargo clippy --workspace -- -D warnings`).

### Negative Cases

- Tree-support plan (no `PrePass::LayerPlanning`) does **not** produce "requires committed LayerPlanIR" error.
- `PrePass::SupportGeometry` always finds `LayerPlanIR` present when it runs (after `execute_prepass`).

## Cross-Packet Dependencies

- **Supersedes:** `31a_support-geometry-prepass-and-layer-height` (execution order portion only; type/WIT/SDK/manifold work stands).
- **Unblocks:** `31b_support-planner-algorithmic-parity` — requires correct execution order to function.
- ** prerequisite:** Packet 31a must not be reverted — all non-execution-order work is preserved.

## Verification Commands

```bash
# Execution order checks
grep -n 'execute_prepass(' crates/slicer-host/src/prepass.rs | head -3
grep -n 'stage_requires_region_map' crates/slicer-host/src/prepass.rs | wc -l

# Test suites
cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1
cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1
cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1
cargo test -p slicer-host --test benchy_end_to_end_tdd -- --test-threads=1
cargo test -p support-planner --lib

# Workspace checks
cargo build --workspace
cargo clippy --workspace -- -D warnings
```

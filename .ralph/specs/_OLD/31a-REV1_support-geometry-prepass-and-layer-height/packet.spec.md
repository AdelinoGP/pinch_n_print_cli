---
status: superseded
packet: 31a-REV1_support-geometry-prepass-and-layer-height
task_ids:
  - TASK-163
backlog_source: docs/07_implementation_status.md
supersedes: 31a_support-geometry-prepass-and-layer-height
---

> **Superseded by 31a-REV2.** Substantive ACs absorbed: `execute_prepass()` runs before built-in commitment so `LayerPlanIR` is always present; the `stage_requires_region_map` two-phase helper is removed; tree-support plans without `PrePass::LayerPlanning` succeed without `LayerPlanIR`-missing errors. No substantive work lost — body preserved verbatim for historical record.

# Packet Contract: 31a-REV1_support-geometry-prepass-and-layer-height

## Goal

Fix the execution order bug introduced by packet `31a_support-geometry-prepass-and-layer-height` in the prepass pipeline. Packet 31a added `PrePass::SupportGeometry` and extended `required_slots("PrePass::SupportGeneration")`, but the swarm workers' implementation moved `RegionMapping` and `SupportGeometry` to run **before** `execute_prepass`. This breaks the pipeline when `PrePass::LayerPlanning` is absent from the execution plan (e.g., tree-support pipeline), because `LayerPlanIR` is only committed **inside** `execute_prepass`.

The fix restores the original correct execution order: `RegionMapping` and `SupportGeometry` run **after** `execute_prepass`, guaranteeing `LayerPlanIR` is always committed before those built-ins check for it.

## Scope Boundaries

- **In scope:**
  - **`crates/slicer-host/src/prepass.rs`** — revert `execute_prepass_with_builtins` execution order: `SupportGeometry` and `RegionMapping` run after `execute_prepass` (Option A from planning handout).
  - **`crates/slicer-host/src/prepass.rs`** — remove the two-phase execution code (`stage_requires_region_map` helper, phase-1/phase-2 split) introduced by the planner's attempted fix.
  - **`crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs`** — verify test `prepass_support_generation_succeeds_with_builtin_region_mapping` expectations are compatible with restored execution order; revert or update if needed.
  - Full Step 17 verification matrix (all 31a tests plus regression suite).

- **Out of scope:**
  - All other 31a work (WIT types, SDK types, blackboard slot, `SupportGeometryIR` type, manifests, planner interpolation) — these are correct and preserved.
  - Any change to `required_slots("PrePass::SupportGeneration")` — already correct.
  - Any change to `support_geometry.rs` — the built-in implementation itself is correct; only its placement in the execution sequence is wrong.
  - Algorithmic features (avoidance/collision, radius tapering, raft, interface) — these are packet 31b.

## Prerequisites and Blockers

- **Depends on:** Packet `31a_support-geometry-prepass-and-layer-height` (partial implementation present, must not be reverted).
- **Unblocks:** Packet `31b_support-planner-algorithmic-parity` (depends on correct execution order).
- **Activation blockers:** None — the fix is fully specified by the planning handout.

## Acceptance Criteria

- **Given** `crates/slicer-host/src/prepass.rs::execute_prepass_with_builtins`, **when** the function body is read, **then** `execute_prepass(plan, blackboard, runner)?` appears **before** both the `commit_region_mapping_builtin` call and the `commit_support_geometry_builtin` call. | `grep -n 'execute_prepass(' crates/slicer-host/src/prepass.rs | head -3`
- **Given** `crates/slicer-host/src/prepass.rs::execute_prepass_with_builtins`, **when** the function body is read, **then** it does **not** contain `stage_requires_region_map` (the two-phase split helper must be removed). | `grep -n 'stage_requires_region_map' crates/slicer-host/src/prepass.rs | wc -l` returns `0`
- **Given** the restored execution order, **when** a tree-support execution plan (no `PrePass::LayerPlanning` in `prepass_stages`) runs through `execute_prepass_with_builtins`, **then** `LayerPlanIR` is committed by `PrePass::LayerPlanning` inside `execute_prepass` before `RegionMapping` and `SupportGeometry` run. | `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 2>&1 | tail -20`
- **Given** `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs`, **when** read, **then** the test `prepass_support_generation_succeeds_with_builtin_region_mapping` either (a) passes as-is, or (b) is updated to reflect the restored execution order semantics. | `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd prepass_support_generation_succeeds_with_builtin_region_mapping -- --test-threads=1 2>&1 | tail -5`
- **Given** `cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1`, **when** all tests run, **then** all pass (packet 28 regression). | `cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 2>&1 | tail -5`
- **Given** `cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1`, **when** all tests run, **then** all pass (regression). | `cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 2>&1 | tail -5`
- **Given** `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --test-threads=1`, **when** all tests run, **then** all pass (regression). | `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --test-threads=1 2>&1 | tail -5`
- **Given** `cargo test -p support-planner --lib`, **when** tests run, **then** all pass. | `cargo test -p support-planner --lib 2>&1 | tail -5`
- **Given** `cargo build --workspace`, **when** the workspace builds, **then** it exits 0. | `cargo build --workspace 2>&1 | tail -5`
- **Given** `cargo clippy --workspace -- -D warnings`, **when** clippy runs, **then** it exits 0 with no warnings. | `cargo clippy --workspace -- -D warnings 2>&1 | tail -5`

## Negative Test Cases

- **Given** `execute_prepass_with_builtins` with a plan that lacks `PrePass::LayerPlanning`, **when** the function is called, **then** it **does not** panic or return an error about "requires committed LayerPlanIR" for `RegionMapping`. | `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 2>&1 | tail -10`
- **Given** the restored execution order, **when** `PrePass::SupportGeometry` runs, **then** `blackboard.layer_plan().is_some()` is always true at that point (LayerPlanIR committed by `execute_prepass`). | `grep -nA20 'commit_support_geometry_builtin' crates/slicer-host/src/prepass.rs | head -25`

## Verification

- `cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10`
- `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10`
- `cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --test-threads=1 --nocapture 2>&1 | tail -10`
- `cargo test -p support-planner --lib 2>&1 | tail -10`
- `cargo build --workspace 2>&1 | tail -5`
- `cargo clippy --workspace -- -D warnings 2>&1 | tail -5`

## Authoritative Docs

- `docs/01_system_architecture.md` — Tier 1 PrePass sequential ordering.
- `docs/04_host_scheduler.md` — `ensure_stage_prerequisites`, `required_slots` ordering.
- `.ralph/specs/31a_support-geometry-prepass-and-layer-height/PLANNING_HANDOUT.md` — exact bug description and Option A fix.
- `.ralph/specs/31a_support-geometry-prepass-and-layer-height/` (all files) — prior packet that introduced the bug; all files except `prepass.rs` execution order are correct and preserved.

## OrcaSlicer Reference Obligations

None — this is an internal execution ordering bug fix, not an OrcaSlicer parity issue.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

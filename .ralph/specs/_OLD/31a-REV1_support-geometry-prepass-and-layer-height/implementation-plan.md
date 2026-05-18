# Implementation Plan: 31a-REV1_support-geometry-prepass-and-layer-height

## Execution Rules

- One atomic step at a time.
- All steps map to `TASK-163`.
- TDD not required — this is a targeted bug fix, not new behavior.
- No open questions — all resolved by the planning handout.

## Steps

### Step 1: Read the broken code

- Task IDs: `TASK-163`
- Objective: Confirm the exact lines to change in `execute_prepass_with_builtins`. Read the current function body, identify the `stage_requires_region_map` helper, and confirm the two-phase splitting code.
- Precondition: None.
- Postcondition: Engineer can mark exact lines to replace.
- Files to read: `crates/slicer-host/src/prepass.rs` lines 280–400 (full `execute_prepass_with_builtins` body + `stage_requires_region_map` helper).
- Files to edit: none.
- Authoritative docs: `.ralph/specs/31a_support-geometry-prepass-and-layer-height/PLANNING_HANDOUT.md` (execution order contract).
- Verification: `grep -n 'stage_requires_region_map' crates/slicer-host/src/prepass.rs` returns line numbers of the helper.
- Context cost: S
- Exit condition: Exact lines to change identified.

### Step 2: Revert `execute_prepass_with_builtins` execution order

- Task IDs: `TASK-163`
- Objective: Replace the body of `execute_prepass_with_builtins` with the fixed Option A code. Remove the two-phase splitting logic and `stage_requires_region_map` helper.
- Precondition: Step 1.
- Postcondition: The function runs `execute_prepass` before `RegionMapping` and `SupportGeometry`.
- Files to read: `crates/slicer-host/src/prepass.rs` lines 280–400.
- Files to edit: `crates/slicer-host/src/prepass.rs` (1 file).
- Expected sub-agent dispatches: None.
- Authoritative docs: `.ralph/specs/31a_support-geometry-prepass-and-layer-height/PLANNING_HANDOUT.md` lines 100–120 (exact code to restore).
- Verification: `grep -n 'execute_prepass(' crates/slicer-host/src/prepass.rs | head -3` shows `execute_prepass` called before both built-in calls; `grep -n 'stage_requires_region_map' crates/slicer-host/src/prepass.rs | wc -l` returns `0`.
- Context cost: S
- Exit condition: Build passes; execution order reverted.

### Step 3: Verify build

- Task IDs: `TASK-163`
- Objective: Confirm the code change compiles cleanly.
- Precondition: Step 2.
- Postcondition: `cargo build -p slicer-host 2>&1 | tail -5` exits 0.
- Files to read: none.
- Files to edit: none.
- Verification: `cargo build -p slicer-host 2>&1 | tail -5` exits 0.
- Context cost: S
- Exit condition: Build green.

### Step 4: Check test expectations

- Task IDs: `TASK-163`
- Objective: Run `prepass_support_generation_layer_plan_tdd` and check if `prepass_support_generation_succeeds_with_builtin_region_mapping` passes. If it fails, determine whether the failure is due to the restored execution order or a pre-existing issue.
- Precondition: Step 3.
- Postcondition: Test passes OR test expectations are updated to match restored semantics.
- Files to read: `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` (focus on `prepass_support_generation_succeeds_with_builtin_region_mapping`).
- Files to edit: `crates/slicer-host/tests/prepass_support_generation_layer_plan_tdd.rs` (only if expectations need updating).
- Verification: `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd prepass_support_generation_succeeds_with_builtin_region_mapping -- --test-threads=1 2>&1 | tail -10`
- Context cost: S
- Exit condition: Test passes with restored execution order.

### Step 5: Run full Step 17 verification matrix

- Task IDs: `TASK-163`
- Objective: Confirm all 31a tests and regression suites pass.
- Precondition: Step 4.
- Postcondition: All 7 verification commands exit 0.
- Files to read: none.
- Files to edit: none.
- Authoritative docs: `.ralph/specs/31a_support-geometry-prepass-and-layer-height/PLANNING_HANDOUT.md` lines 218–225 (verification matrix).
- Verification commands (run all):
  1. `cargo test -p slicer-host --test prepass_support_generation_tdd -- --test-threads=1 2>&1 | tail -10`
  2. `cargo test -p slicer-host --test prepass_support_generation_layer_plan_tdd -- --test-threads=1 2>&1 | tail -10`
  3. `cargo test -p slicer-host --test live_support_generation_tdd -- --test-threads=1 2>&1 | tail -10`
  4. `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --test-threads=1 2>&1 | tail -10`
  5. `cargo test -p support-planner --lib 2>&1 | tail -10`
  6. `cargo build --workspace 2>&1 | tail -5`
  7. `cargo clippy --workspace -- -D warnings 2>&1 | tail -5`
- Context cost: S
- Exit condition: All 7 commands exit 0.

### Step 6: Packet completion gate

- Task IDs: `TASK-163`
- Objective: Confirm self-review checklist and mark packet ready for `spec-review`.
- Precondition: Steps 1–5.
- Postcondition: Packet is complete; no unresolved items.
- Files to read: All 5 packet files.
- Files to edit: none.
- Verification: Self-review checklist in `packet.spec.md` all green.
- Context cost: S
- Exit condition: Packet complete.

## Packet Completion Gate

- `execute_prepass` called before both `RegionMapping` and `SupportGeometry` built-ins.
- `stage_requires_region_map` removed from `prepass.rs`.
- All 7 verification commands exit 0.
- No unresolved open questions.
- All 5 packet files written with concrete content.

## Self-Review Checklist (mandatory before reporting)

- [x] Every AC is implementation-grade and names exact assertion content.
- [x] At least one negative case when the slice changes validation, enforcement, or contract behavior.
- [x] `requirements.md` states measurable outcomes, not topical summaries.
- [x] `design.md` selects one approach, lists exact code surfaces, lists out-of-bounds files.
- [x] Each step has precondition, postcondition, falsifying check / exit condition, files-to-read, files-to-edit, expected dispatches, and context cost.
- [x] No step has cost L.
- [x] Verification commands are delegation-friendly.
- [x] Superseding packet explains what the prior packet missed and how this one narrows the gap.
- [x] No open questions (all resolved by planning handout).

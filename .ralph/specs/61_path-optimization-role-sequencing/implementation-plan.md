# Implementation Plan: 61_path-optimization-role-sequencing

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Add TDD unit tests for role ordering (red)

- Task IDs: `TASK-152h`
- Objective: Write failing unit tests that assert role-priority ordering. Tests must fail against current code (which interleaves roles by distance).
- Precondition: `cargo test -p path-optimization-default --lib` passes on current code (all existing tests green).
- Postcondition: 7 new unit tests exist in `lib.rs` `#[cfg(test)] mod tests`, all fail with role-interleaving as the root cause (not compilation errors).
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/path-optimization-default/src/lib.rs` — lines 340-420 (existing `#[cfg(test)]` module) for test pattern reference.
  - `crates/slicer-ir/src/slice_ir.rs` — lines 1318-1347 (`ExtrusionRole` enum variants for test fixture construction).
  - `crates/slicer-sdk/src/views.rs` — lines 419-432 (`OrderedEntityView` struct fields).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/path-optimization-default/src/lib.rs` — append new tests to existing `#[cfg(test)] mod tests`.
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/` — host dispatch; no changes needed.
  - `crates/slicer-sdk/src/traits.rs` — finalization builder; unrelated.
- Expected sub-agent dispatches:
  - "Run `cargo test -p path-optimization-default --lib`; return FACT pass/fail on existing tests + new tests" — confirm new tests FAIL with role-interleaving (not compile errors).
- Context cost: S
- Authoritative docs: `docs/05_module_sdk.md` — load directly for `LayerCollectionBuilder` and `OrderedEntityView` test construction patterns.
- OrcaSlicer refs: None needed for test writing — tests are self-contained unit tests on internal functions.
- Verification:
  - `cargo test -p path-optimization-default --lib -- role_orders_inner_before_outer` — dispatch as FACT; expect FAIL.
  - `cargo test -p path-optimization-default --lib` — dispatch as FACT; existing tests must still pass, new tests must fail.
- Exit condition: All 7 new tests compile and fail with role-interleaving assertion failures (InnerWall after SparseInfill, etc.). Existing tests unchanged.

---

### Step 2: Add WallSequence enum, role_group, and refactor group_then_nearest_neighbor

- Task IDs: `TASK-152h`
- Objective: Implement the core role-priority grouping. Add `WallSequence` enum, `role_group()` method, refactor `group_then_nearest_neighbor` from free function to method with two-level grouping.
- Precondition: Step 1 complete — 7 red tests exist.
- Postcondition: All 7 role-ordering tests pass. `group_then_nearest_neighbor` is a method on `PathOptimizationDefault` that groups entities by tool index, then by role group, then applies `nearest_neighbor_permutation` within each role group. The function signature is `fn group_then_nearest_neighbor(&self, entities: &[OrderedEntityView]) -> (Vec<(u32, bool)>, Vec<ToolChangeRecord>)`.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/path-optimization-default/src/lib.rs` — lines 1-420 (full file, ~100 lines of code).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/path-optimization-default/src/lib.rs` — insert `WallSequence` enum, `role_group` method, refactor `group_then_nearest_neighbor`, update `run_path_optimization` call site.
- Files explicitly out-of-bounds for this step:
  - `path-optimization-default.toml` — config schema added in Step 3.
- Expected sub-agent dispatches:
  - "Run `cargo test -p path-optimization-default --lib`; return FACT pass/fail with failing test count" — all tests must pass after this step.
  - "Run `cargo check --workspace`; return FACT pass/fail" — workspace type-check.
- Context cost: M
- Authoritative docs:
  - `docs/05_module_sdk.md` — `OrderedEntityView` contract (confirm `region_key.region_id` carries tool index).
  - `docs/01_system_architecture.md` — delegate SUMMARY for PathOptimization stage constraints.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:6082-6107` — delegate SUMMARY of `extrude_infill` (confirm all infill types chained together).
- Verification:
  - `cargo test -p path-optimization-default --lib` — all tests pass.
  - `cargo check --workspace` — type-check clean.
- Exit condition: All 7 role-ordering tests pass green. Existing travel-pol```markdown
icy, retract-mode, and seam-consumption tests continue to pass. No compilation errors or warnings.

---

### Step 3: Add wall_sequence config parsing and TOML schema

- Task IDs: `TASK-152h`
- Objective: Parse `wall_sequence` config key in `on_print_start`, add `wall_sequence` field to `PathOptimizationDefault` struct, add config schema to `path-optimization-default.toml`.
- Precondition: Step 2 complete — `role_group` method exists but `wall_sequence` defaults to `InnerOuter` hardcoded.
- Postcondition: `on_print_start` reads `wall_sequence` from config, rejects invalid values with `ModuleError::fatal`, and stores parsed `WallSequence` on the struct. `path-optimization-default.toml` has `[config.schema.wall_sequence]` entry.
- Files allowed to read (with line-range hints when > 300 lines):
  - `modules/core-modules/path-optimization-default/src/lib.rs` — lines 194-233 (`on_print_start` config parsing pattern).
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml` — lines 1-71 (existing config schema pattern).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/path-optimization-default/src/lib.rs` — add `wall_sequence` field to struct, parse in `on_print_start`.
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml` — add `[config.schema.wall_sequence]` table.
- Files explicitly out-of-bounds for this step:
  - None.
- Expected sub-agent dispatches:
  - "Run `cargo test -p path-optimization-default --lib -- role_rejects_invalid_wall_sequence`; return FACT pass/fail" — AC-N1.
  - "Run `cargo test -p path-optimization-default --lib -- role_orders_outer_before_inner`; return FACT pass/fail" — AC-2.
- Context cost: S
- Authoritative docs: None — config pattern is self-documenting in the existing module.
- OrcaSlicer refs: None needed for config schema.
- Verification:
  - `cargo test -p path-optimization-default --lib -- role_rejects_invalid_wall_sequence` — AC-N1.
  - `cargo test -p path-optimization-default --lib -- role_orders_outer_before_inner` — AC-2.
  - `cargo test -p path-optimization-default --lib -- role_orders_inner_before_outer` — AC-1 (still passes with default).
- Exit condition: AC-N1 and AC-2 pass. AC-1 still passes with default config. All other tests pass.

---

### Step 4: Remove bridge tie-break dead code

- Task IDs: `TASK-152h`
- Objective: Remove the BridgeInfill tie-break preference from `nearest_neighbor_permutation` (lines 73-78). After role grouping, all bridges are in the same group, making the tie-break unreachable.
- Precondition: Steps 2 and 3 complete — role grouping is functional, `role_chains_infill_together` test passes.
- Postcondition: `nearest_neighbor_permutation` tie-break falls back to `i < best_idx` for equidistant entities (deterministic, no role bias). No BridgeInfill references remain in the function body.
- Files allowed to read:
  - `modules/core-modules/path-optimization-default/src/lib.rs` — lines 57-84 (current tie-break logic).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/path-optimization-default/src/lib.rs` — simplify tie-break in `nearest_neighbor_permutation`.
- Files explicitly out-of-bounds for this step:
  - None.
- Expected sub-agent dispatches:
  - "Run `cargo test -p path-optimization-default --lib`; return FACT pass/fail" — all tests must still pass.
- Context cost: S
- Authoritative docs: None.
- OrcaSlicer refs: None.
- Verification:
  - `cargo test -p path-optimization-default --lib` — all tests pass.
  - `cargo clippy --workspace -- -D warnings` — lint clean.
- Exit condition: All tests pass. `rg 'BridgeInfill' modules/core-modules/path-optimization-default/src/lib.rs` returns only `role_group` match arm and test fixture constructions (no tie-break code).

---

### Step 5: Packet completion gate

- Task IDs: `TASK-152h`
- Objective: Run all acceptance criteria verification commands and workspace gates. Confirm WASM freshness.
- Precondition: Steps 1-4 complete.
- Postcondition: All ACs green, all verification commands pass, WASM module fresh.
- Files allowed to read: None — all dispatched.
- Files allowed to edit: None.
- Expected sub-agent dispatches (parallel):
  - "Run `cargo test -p path-optimization-default --lib`; return FACT pass/fail + failing test count (expect 0)"
  - "Run `cargo check --workspace`; return FACT pass/fail"
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail"
  - "Run `./modules/core-modules/build-core-modules.sh --check`; return FACT: CLEAN or STALE: <list>"
- Context cost: S
- Verification:
  - `cargo test -p path-optimization-default --lib` — all tests pass.
  - `cargo check --workspace` — type-check clean.
  - `cargo clippy --workspace -- -D warnings` — lint clean.
  - `./modules/core-modules/build-core-modules.sh --check` — WASM fresh.
- Exit condition: All 4 gate commands return clean. Ready to mark `packet.spec.md: status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 (TDD red tests) | S | Write 7 test functions, no implementation |
| Step 2 (core refactor) | M | Largest step: add enum, method, refactor grouping |
| Step 3 (config + schema) | S | Parse one enum value, add TOML table |
| Step 4 (tie-break removal) | S | Delete 8 lines, simplify 2 |
| Step 5 (gate) | S | Dispatch all verifications in parallel |
| **Aggregate** | **M** | |

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for the packet task IDs (via worker dispatch — never edited by loading the full backlog into the implementer's context).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`:
  - `cargo test -p path-optimization-default --lib -- role_orders_inner_before_outer`
  - `cargo test -p path-optimization-default --lib -- role_orders_outer_before_inner`
  - `cargo test -p path-optimization-default --lib -- role_orders_walls_before_infill`
  - `cargo test -p path-optimization-default --lib -- role_chains_infill_together`
  - `cargo test -p path-optimization-default --lib -- role_handles_all_extrusion_roles`
  - `cargo test -p path-optimization-default --lib -- role_preserves_global_sequence`
  - `cargo test -p path-optimization-default --lib -- role_ordering_is_deterministic`
  - `cargo test -p path-optimization-default --lib -- role_rejects_invalid_wall_sequence`
- Confirm packet-level verification commands are green:
  - `cargo check --workspace`
  - `cargo clippy --workspace -- -D warnings`
  - `./modules/core-modules/build-core-modules.sh --check`
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs.

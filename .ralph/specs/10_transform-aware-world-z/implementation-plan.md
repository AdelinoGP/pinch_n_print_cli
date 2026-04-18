# Implementation Plan: transform-aware-world-z

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Discovery — Inventory transform application points and canonical surface decision

- **Task IDs**: `TASK-157`, `TASK-158`
- **Objective**: Complete the open questions in `design.md` before writing code. Specifically:
  1. Inventory every call site of `object_world_z_extent` and confirm all use world-space Z (not local Z).
  2. Inventory every place in LayerPlanning that reads mesh vertex Z directly.
  3. Decide Option A (cached IR field) vs Option B (config-only documentation) for the canonical world-space Z surface.
  4. Determine the print volume floor value for `WORLD_Z_BELOW_FLOOR`.
- **Precondition**: None
- **Postcondition**: Written answers to all four open questions in `design.md`. Inventory of call sites complete. Option A vs Option B decision recorded.
- **Files expected to change**: None (read-only discovery)
- **Authoritative docs**: `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md`, `crates/slicer-host/src/model_loader.rs`, `crates/slicer-host/src/main.rs`, `crates/slicer-host/src/mesh_analysis.rs`
- **Verification**: Read the source files and confirm the inventory is complete. No code changes.
- **Exit condition**: All four open questions answered in `design.md`. Option A vs Option B decision documented.

---

### Step 2: TASK-157 — `translated_object_z_floor_tdd` integration test

- **Task IDs**: `TASK-157`
- **Objective**: Create the first integration test proving that a Z-translated object produces correct world-space `LayerPlanIR.global_layers[*].z` values.
- **Precondition**: Step 1 complete
- **Postcondition**: `crates/slicer-host/tests/translated_object_z_floor_tdd.rs` exists and passes.
- **Files expected to change**:
  - `crates/slicer-host/tests/translated_object_z_floor_tdd.rs` (new file)
- **Authoritative docs**: `docs/02_ir_schemas.md` (`GlobalLayer.z`), `docs/04_host_scheduler.md` (`PrePass::LayerPlanning`), `crates/slicer-host/src/model_loader.rs` (`object_world_z_extent`)
- **Verification**: `cargo test -p slicer-host --test translated_object_z_floor_tdd -- --nocapture`
- **Exit condition**: Test passes. The test asserts that `transform = translate(0, 0, 10mm)` with 0.2mm layer height produces `global_layers[0].z >= 10.0`.

---

### Step 3: TASK-157 — `rotated_object_world_extent_tdd` integration test

- **Task IDs**: `TASK-157`
- **Objective**: Create an integration test proving that a lay-flat rotation (`rotate_x(90deg)`) produces correct world-space Z planes that span the object's projected extent.
- **Precondition**: Step 2 complete
- **Postcondition**: `crates/slicer-host/tests/rotated_object_world_extent_tdd.rs` exists and passes.
- **Files expected to change**:
  - `crates/slicer-host/tests/rotated_object_world_extent_tdd.rs` (new file)
- **Authoritative docs**: `docs/02_ir_schemas.md`, `crates/slicer-host/src/model_loader.rs`, `crates/slicer-host/src/mesh_analysis.rs` (`apply_transform`)
- **Verification**: `cargo test -p slicer-host --test rotated_object_world_extent_tdd -- --nocapture`
- **Exit condition**: Test passes. The test asserts that a `rotate_x(90deg)` transform applied to a vertical object produces Z planes spanning the world-space projection (not the local Z range).

---

### Step 4: TASK-157 — `transformed_model_world_z_tdd` general fixture test

- **Task IDs**: `TASK-157`
- **Objective**: Create the general fixture test for transformed STL/3MF inputs proving world-space Z behavior through the full planning path.
- **Precondition**: Steps 2 and 3 complete
- **Postcondition**: `crates/slicer-host/tests/transformed_model_world_z_tdd.rs` exists and passes.
- **Files expected to change**:
  - `crates/slicer-host/tests/transformed_model_world_z_tdd.rs` (new file)
- **Authoritative docs**: Same as Steps 2 and 3
- **Verification**: `cargo test -p slicer-host --test transformed_model_world_z_tdd -- --nocapture`
- **Exit condition**: Test passes. The test loads or constructs a model with a non-identity transform (rotation + translation) and verifies `LayerPlanIR.global_layers[*].z` is in world-space.

---

### Step 5: TASK-157 — `multi_object_transform_world_z_tdd` integration test

- **Task IDs**: `TASK-157`
- **Objective**: Create the multi-object integration test with different transforms and LCM layer height synchronization.
- **Precondition**: Steps 2–4 complete
- **Postcondition**: `crates/slicer-host/tests/multi_object_transform_world_z_tdd.rs` exists and passes.
- **Files expected to change**:
  - `crates/slicer-host/tests/multi_object_transform_world_z_tdd.rs` (new file)
- **Authoritative docs**: `docs/01_system_architecture.md` (LCM synchronization), `docs/04_host_scheduler.md` (catch-up layer semantics)
- **Verification**: `cargo test -p slicer-host --test multi_object_transform_world_z_tdd -- --nocapture`
- **Exit condition**: Test passes. Two objects with different transforms and layer heights are in the scene. `LayerPlanIR` is built. Global layer indices are from LCM of layer heights. Each object's Z range is correctly projected to world space.

---

### Step 6: TASK-158 — Canonical surface: Option A (IR field) or Option B (config documentation)

- **Task IDs**: `TASK-158`
- **Objective**: Implement the canonical world-space Z surface per the decision in Step 1.
- **Precondition**: Steps 1–5 complete. Step 1 open question #1 answered.
- **Postcondition**: Either:
  - **Option A**: `ObjectMesh.world_z_extent: Option<(f32, f32)>` added to `MeshIR` schema, computed and cached at load time in `model_loader.rs`. `MeshIR.schema_version` bumped. `main.rs:153` updated to use the cached field.
  - **Option B**: `docs/02_ir_schemas.md` updated to document `object_height:{id}` config keys as the canonical world-space Z supply with explicit "do not read local mesh Z" guidance.
- **Files expected to change**:
  - Option A: `crates/slicer-ir/src/` (schema change), `crates/slicer-host/src/model_loader.rs`, `crates/slicer-host/src/main.rs`
  - Option B: `docs/02_ir_schemas.md`
- **Authoritative docs**: `docs/02_ir_schemas.md` (`ObjectMesh`, `MeshIR`)
- **Verification**:
  - Option A: `cargo build --package slicer-ir && cargo build --package slicer-host`
  - Option B: `grep -r "world.space.Z\|object_height" docs/02_ir_schemas.md` finds the canonical surface documentation
- **Exit condition**: Canonical surface is explicitly defined. No ambiguity about whether world-space Z is derived or first-class.

---

### Step 7: TASK-158 — `world_z_canonical_surface_tdd` regression lock test

- **Task IDs**: `TASK-158`
- **Objective**: Create the regression test proving the canonical surface is used consistently and no code path silently reads object-local Z for planning.
- **Precondition**: Step 6 complete
- **Postcondition**: `crates/slicer-host/tests/world_z_canonical_surface_tdd.rs` exists and passes.
- **Files expected to change**:
  - `crates/slicer-host/tests/world_z_canonical_surface_tdd.rs` (new file)
- **Authoritative docs**: `docs/02_ir_schemas.md` (canonical surface decision), `crates/slicer-host/src/model_loader.rs`
- **Verification**: `cargo test -p slicer-host --test world_z_canonical_surface_tdd -- --nocapture`
- **Exit condition**: Test passes. The test verifies that world-space Z is used consistently through the planning path. Any future regression that reintroduces local Z reading will fail this test.

---

### Step 8: Negative test — `non_uniform_scale_tdd`

- **Task IDs**: `TASK-157`, `TASK-158` (negative case)
- **Objective**: Create the negative test proving that non-uniform scale is rejected with `NON_UNIFORM_SCALE_UNSUPPORTED`.
- **Precondition**: Steps 1–5 complete
- **Postcondition**: `crates/slicer-host/tests/non_uniform_scale_tdd.rs` exists and passes.
- **Files expected to change**:
  - `crates/slicer-host/tests/non_uniform_scale_tdd.rs` (new file)
  - `crates/slicer-host/src/model_loader.rs` — may need to add `NON_UNIFORM_SCALE_UNSUPPORTED` error path if not already present
- **Authoritative docs**: `docs/08_coordinate_system.md`
- **Verification**: `cargo test -p slicer-host --test non_uniform_scale_tdd -- --nocapture`
- **Exit condition**: Test passes. An object with `scale_x != scale_y != scale_z` produces a fatal error with code `NON_UNIFORM_SCALE_UNSUPPORTED` at load time.

---

### Step 9: Negative test — `world_z_below_floor_tdd`

- **Task IDs**: `TASK-157`, `TASK-158` (negative case)
- **Objective**: Create the negative test proving that world-space Z < 0 is rejected with `WORLD_Z_BELOW_FLOOR`.
- **Precondition**: Steps 1–5 complete. Step 1 open question #3 answered (print volume floor value).
- **Postcondition**: `crates/slicer-host/tests/world_z_below_floor_tdd.rs` exists and passes.
- **Files expected to change**:
  - `crates/slicer-host/tests/world_z_below_floor_tdd.rs` (new file)
  - `crates/slicer-host/src/model_loader.rs` — may need to add `WORLD_Z_BELOW_FLOOR` check if not already present
- **Authoritative docs**: `docs/08_coordinate_system.md`
- **Verification**: `cargo test -p slicer-host --test world_z_below_floor_tdd -- --nocapture`
- **Exit condition**: Test passes. An object with a transform producing world-space Z < 0 emits a diagnostic and fails with `WORLD_Z_BELOW_FLOOR`.

---

### Step 10: Workspace gate and DEV-027 closure

- **Task IDs**: `TASK-157`, `TASK-158`
- **Objective**: Run the full workspace gate and close DEV-027.
- **Precondition**: Steps 1–9 complete
- **Postcondition**:
  - `cargo build --workspace` succeeds
  - `cargo clippy --workspace -- -D warnings` passes with zero warnings
  - `docs/DEVIATION_LOG.md` updated: DEV-027 status changed from "Partial" or "Open" to "Closed"
  - `docs/07_implementation_status.md` updated: TASK-157 and TASK-158 marked complete
- **Files expected to change**: `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md`
- **Authoritative docs**: `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md`
- **Verification**: Full workspace build and clippy pass
- **Exit condition**: All acceptance criteria green, DEV-027 closed, packet status ready to move to `implemented`.

## Packet Completion Gate

- All 10 steps complete.
- Every step exit condition is met.
- All 7 new test files exist and pass.
- `cargo build --workspace` succeeds.
- `cargo clippy --workspace -- -D warnings` passes with zero warnings.
- `docs/07_implementation_status.md` updated for TASK-157 and TASK-158.
- `docs/DEVIATION_LOG.md` updated: DEV-027 closed.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm all 7 test files pass individually.
- Confirm workspace build is green.
- Confirm clippy is green with zero warnings.
- Verify DEV-027 status in `docs/DEVIATION_LOG.md` is "Closed".
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.

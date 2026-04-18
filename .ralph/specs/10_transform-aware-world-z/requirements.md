# Requirements: transform-aware-world-z

## Packet Metadata

- Grouped task IDs:
  - `TASK-157`
  - `TASK-158`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

AUDIT-21 (DEV-027) confirmed that `object_world_z_extent` exists and is wired into the live pipeline via `object_height:{id}` config keys at `main.rs:153`, but two critical gaps remain:

1. **No integration-level test** loads a real STL/3MF fixture with a non-identity `<transform>` element and verifies world-space Z behavior through `LayerPlanIR` generation. Only unit tests exist in `model_loader_tdd.rs`.

2. **World-space Z is not a first-class derived field** on `ObjectMesh`. It is computed on-demand via `object_world_z_extent(object: &ObjectMesh) -> Option<(f32, f32)>` and not cached. This means the contract surface for world-space Z is implicit, not explicit — any future caller who inadvertently reads local mesh Z (bypassing the transform) would produce incorrect results silently.

TASK-157 closes gap 1. TASK-158 closes gap 2 by promoting world-space Z to a canonical surface.

## In Scope

- Fixture-level integration tests for non-identity transforms through the full planning path:
  - Translated object (positive Z translation)
  - Rotated object (lay-flat 90-degree rotation)
  - Scaled object (uniform scale)
  - Multi-object with different transforms and LCM layer height synchronization
- Canonical world-space Z derived field: either `ObjectMesh.world_z_extent: Option<(f32, f32)>` as a cached first-class IR field, or documented config-only behavior with `object_height:{id}` as the canonical supply path
- Negative test cases:
  - Non-uniform scale rejection with `NON_UNIFORM_SCALE_UNSUPPORTED` error code
  - World-space Z below print volume floor with `WORLD_Z_BELOW_FLOOR` error code
- Regression guard proving no silent use of object-local Z anywhere in the planning path
- `world_z_canonical_surface_tdd` test that verifies the canonical surface is used consistently

## Out of Scope

- Mesh-query host services (`raycast_z_down`, `surface_normal_at`, `object_bounds`) — TASK-147/148
- Path-optimization behavior changes
- Benchy parity — TASK-120 series
- Applying transforms to mesh geometry at load time (transform stays on `ObjectMesh.transform`, applied at query time)

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md` — `MeshIR.ObjectMesh.transform`, `LayerPlanIR.GlobalLayer.z`
- `docs/04_host_scheduler.md` — `PrePass::LayerPlanning`
- `docs/08_coordinate_system.md`
- `docs/05_module_sdk.md`
- `docs/07_implementation_status.md` — DEV-027, TASK-157, TASK-158

## OrcaSlicer Reference Obligations

None. This is an internal architecture contract task. Transform behavior is defined by ModularSlicer's own coordinate system and is not derived from OrcaSlicer comparison.

## Acceptance Summary

### Positive Cases

1. **Translated object**: `transform = translate(0, 0, 10mm)`, layer height 0.2mm → `global_layers[0].z >= 10.0`
2. **Rotated object (lay-flat)**: `transform = rotate_x(90deg)` → world-space Z planes span the object's projected world extent, not local extent
3. **Scaled object**: `transform = scale(1.0, 1.0, 2.0)` → world-space Z extent correctly scaled (z_max = local_z_max * 2.0)
4. **Multi-object**: Two objects with different transforms and layer heights → LCM synchronization with correct world-space projection for each
5. **Canonical surface**: Either `ObjectMesh.world_z_extent` field exists and is used, or `object_height:{id}` config keys are documented as the canonical supply path

### Negative Cases

1. **Non-uniform scale**: `scale_x != scale_y != scale_z` → fatal error `NON_UNIFORM_SCALE_UNSUPPORTED` at load time
2. **World-Z below floor**: `transform` produces world-space Z < 0 → fatal error `WORLD_Z_BELOW_FLOOR` with diagnostic

### Measurable Outcomes

- 7 new test files under `crates/slicer-host/tests/`:
  - `transformed_model_world_z_tdd.rs`
  - `translated_object_z_floor_tdd.rs`
  - `rotated_object_world_extent_tdd.rs`
  - `multi_object_transform_world_z_tdd.rs`
  - `world_z_canonical_surface_tdd.rs`
  - `non_uniform_scale_tdd.rs`
  - `world_z_below_floor_tdd.rs`
- All 7 tests pass
- If `ObjectMesh.world_z_extent` is added: `MeshIR` schema version bump (minor)
- If config-only: `docs/02_ir_schemas.md` updated with canonical supply documentation

### Cross-Packet Impact

- Unblocks TASK-127 (non-planar Z envelope) when world-space Z is a first-class field
- Supports Phase H Benchy acceptance for scenes with transformed objects
- `model_loader.rs` and `mesh_analysis.rs` are the primary code surfaces

## Verification Commands

- `cargo test -p slicer-host --test transformed_model_world_z_tdd -- --nocapture`
- `cargo test -p slicer-host --test translated_object_z_floor_tdd -- --nocapture`
- `cargo test -p slicer-host --test rotated_object_world_extent_tdd -- --nocapture`
- `cargo test -p slicer-host --test multi_object_transform_world_z_tdd -- --nocapture`
- `cargo test -p slicer-host --test world_z_canonical_surface_tdd -- --nocapture`
- `cargo test -p slicer-host --test non_uniform_scale_tdd -- --nocapture`
- `cargo test -p slicer-host --test world_z_below_floor_tdd -- --nocapture`
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

### Step 1 — Read-only discovery
- **Precondition**: None
- **Postcondition**: Inventory of existing `object_world_z_extent` call sites, transform application points in layer planning, and current config key supply path for `object_height:{id}`
- **Falsifying check**: Any call site that reads mesh vertex Z directly without applying the transform would invalidate the discovery

### Step 2 — TASK-157: Integration fixtures
- **Precondition**: Step 1 complete
- **Postcondition**: 5 new integration test files exist and pass, proving world-space Z correctness through `LayerPlanIR` for translated, rotated, scaled, and multi-object cases
- **Falsifying check**: Tests fail if any transform is not correctly applied through the planning path

### Step 3 — TASK-158: Canonical surface
- **Precondition**: Step 2 complete
- **Postcondition**: Either `ObjectMesh.world_z_extent: Option<(f32, f32)>` added as a cached derived field (with schema version bump), OR `docs/02_ir_schemas.md` updated to document `object_height:{id}` config keys as the canonical world-space Z supply with explicit "do not read local mesh Z" guidance
- **Falsifying check**: Regression test `world_z_canonical_surface_tdd` fails if local mesh Z is used instead of world-space Z

### Step 4 — Negative cases
- **Precondition**: Steps 2 and 3 complete
- **Postcondition**: `non_uniform_scale_tdd` and `world_z_below_floor_tdd` exist and pass with correct error codes
- **Falsifying check**: Missing error codes or incorrect diagnostics indicate incomplete negative case coverage

### Step 5 — Regression lock
- **Precondition**: Steps 2–4 complete
- **Postcondition**: Full workspace build and clippy pass with zero warnings
- **Falsifying check**: Any compiler warning or clippy warning related to transform/Z handling indicates incomplete regression lock

# Implementation Plan: live-top-bottom-surface-fill

## Execution Rules

- One atomic step at a time.
- TDD first on the canonical infill module, then host integration.

## Steps

### Step 0 (prerequisite): Add surface classification fields to `SliceRegionView`

- Task IDs:
  - `TASK-120a`
- Objective:
  Add `is_top_surface`, `is_bottom_surface`, and `is_bridge` fields to `SliceRegionView` so surface classification data from `SurfaceClassificationIR` can reach the infill module. Without these fields, Step 1 tests cannot be authored because there is no way to construct a `SliceRegionView` with surface classification.
- Precondition:
  `SliceRegionView` in `crates/slicer-sdk/src/views.rs` has no surface classification fields.
- Postcondition:
  `SliceRegionView` carries `is_top_surface: bool`, `is_bottom_surface: bool`, and `is_bridge: bool`; constructors and setters are updated.
- Files expected to change:
  - `crates/slicer-sdk/src/views.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — `SurfaceClassificationIR` field definitions
  - `crates/slicer-sdk/src/views.rs` — current `SliceRegionView` struct
- Verification:
  - `cargo build -p slicer-sdk`
- Exit condition:
  `SliceRegionView` has the three surface classification fields and the SDK compiles without errors.

### Step 1: Add failing rectilinear top/bottom/bridge role tests

- Task IDs:
  - `TASK-120a`
- Objective:
  Freeze the exact `ExtrusionRole` expectations for top, bottom, bridge, and sparse-only cases on the canonical infill module.
- Precondition:
  `SliceRegionView` has surface classification fields (Step 0 complete). No focused test currently locks top/bottom surface-role generation on the live default infill module.
- Postcondition:
  `top_bottom_fill_tdd.rs` exists with failing role-specific assertions.
- Files expected to change:
  - `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp`
- Verification:
  - `cargo test -p rectilinear-infill --test top_bottom_fill_tdd top_surface_region_emits_top_solid_infill -- --exact --nocapture`
  - `cargo test -p rectilinear-infill --test top_bottom_fill_tdd bottom_surface_region_emits_bottom_solid_infill -- --exact --nocapture`
  - `cargo test -p rectilinear-infill --test top_bottom_fill_tdd bridge_surface_region_emits_bridge_infill_role -- --exact --nocapture`
  - `cargo test -p rectilinear-infill --test top_bottom_fill_tdd sparse_only_region_does_not_fabricate_surface_fill_roles -- --exact --nocapture`
- Exit condition:
  All four tests exist and fail only because the live module/host path has not yet restored the roles.

### Step 2: Restore canonical surface-fill generation on `rectilinear-infill`

- Task IDs:
  - `TASK-120a`
- Objective:
  Implement the exact role generation required by Step 1 on the canonical infill module.
- Precondition:
  Step 1 tests are in place.
- Postcondition:
  The canonical infill module emits non-empty top, bottom, and bridge paths with exact `ExtrusionRole` values.
- Files expected to change:
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
  - `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/02_ir_schemas.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.hpp`
- Verification:
  - `cargo test -p rectilinear-infill --test top_bottom_fill_tdd -- --nocapture`
- Exit condition:
  All module-level role tests pass.

### Step 3: Prove the live host path preserves the restored roles

- Task IDs:
  - `TASK-120a`
- Objective:
  Add one host integration regression that proves the live dispatch and layer assembly path keep the restored roles intact.
- Precondition:
  Module-level role tests are green.
- Postcondition:
  `LayerCollectionIR.ordered_entities` carries `TopSolidInfill` and `BottomSolidInfill` on the real host path.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/tests/live_top_bottom_fill_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp`
- Verification:
  - `cargo test -p slicer-host --test live_top_bottom_fill_tdd layer_execution_preserves_top_and_bottom_fill_roles -- --exact --nocapture`
- Exit condition:
  The host integration test passes and proves the roles survive into `LayerCollectionIR`.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated for `TASK-120a`.

## Acceptance Ceremony

- Re-run all role-specific module tests.
- Re-run the host integration role-preservation test.
- Record any remaining packet-local risk before marking the packet implemented.
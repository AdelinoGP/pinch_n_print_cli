# Implementation Plan: 14-rev1_live-seam-placement-and-consumption

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Documentation update precedes implementation — the doc is the contract, not the code.

## Steps

### Step 0: Documentation clarification — update `docs/02_ir_schemas.md` seam semantics

- Task IDs:
  - `TASK-120c`
- Objective:
  Resolve the documentation ambiguity so the `seam-first` contract is explicit before any code is written. Update `docs/02_ir_schemas.md` § IR 7 to state that after `Layer::PerimetersPostProcess` completes, `WallLoop.path.points[0]` is the seam-first vertex.
- Precondition:
  `docs/02_ir_schemas.md` § IR 7 describes `SeamPosition` as a reference annotation with no mention of path rotation.
- Postcondition:
  The `SeamPosition` section in `docs/02_ir_schemas.md` explicitly documents: "The `resolved_seam` field stores a diagnostic reference. The canonical seam-first geometry is stored in `WallLoop.path.points[0]` — after `Layer::PerimetersPostProcess` completes, `path.points[0]` is the seam vertex and the path sequence starts there."
- Files expected to change:
  - `docs/02_ir_schemas.md` — update § IR 7 `SeamPosition` description and add seam-first invariant note to `WallLoop`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — § IR 7 `PerimeterIR`, `WallLoop`, `SeamPosition`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — confirms seam-first wall loop rotation
- Verification:
  - `grep -n "seam-first\|points\[0\]" docs/02_ir_schemas.md` → must show the new documentation text
- Exit condition:
  The documentation explicitly states that `WallLoop.path.points[0]` is the seam-first vertex. No implementation code is changed in this step.

---

### Step 1: Add `push-reordered-wall-loop` to WIT and update seam-placer manifest

- Task IDs:
  - `TASK-120c`
- Objective:
  Add the new WIT method to `perimeter-output-builder` and update the seam-placer manifest to claim write access to the wall loop geometry fields it will rotate.
- Precondition:
  Step 0 is complete — documentation is updated.
- Postcondition:
  - `wit/deps/ir-types.wit` contains `push-reordered-wall-loop(pos: point3-with-width, wall-index: u32, rotated-wall-loop: wall-loop-view) -> result<_, string>` on `perimeter-output-builder`
  - `modules/core-modules/seam-placer/seam-placer.toml` `writes` includes `PerimeterIR.regions.walls.path` and `PerimeterIR.regions.walls.feature_flags` (the full wall geometry fields)
  - `modules/core-modules/seam-placer/seam-placer.toml` `reads` includes `PerimeterIR` (reads resolved_seam to determine rotation)
- Files expected to change:
  - `wit/deps/ir-types.wit` — add method to `perimeter-output-builder`
  - `modules/core-modules/seam-placer/seam-placer.toml` — update `writes` from `["PerimeterIR.resolved-seam"]` to include wall geometry fields
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — `perimeter-output-builder` WIT resource
  - `docs/02_ir_schemas.md` — wall loop field paths
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`
- Verification:
  - `cargo build -p slicer-host --tests 2>&1 | head -20` → compilation should fail (expected — implementation not yet added)
  - The failure message should mention `push_reordered_wall_loop` (missing from host implementation)
- Exit condition:
  `wit/deps/ir-types.wit` has the new method. Manifest is updated. No implementation code yet — compilation failure is expected.

---

### Step 2: Implement `push-reordered-wall-loop` host side in `wit_host.rs`

- Task IDs:
  - `TASK-120c`
- Objective:
  Implement the host-side WIT handler for `push-reordered-wall-loop` in `HostPerimeterOutputBuilder`. Add `rotated_wall_loops` field to `PerimeterOutputCollected`.
- Precondition:
  Step 1 is complete (WIT method exists, manifest updated, compilation fails with expected missing-symbol message).
- Postcondition:
  - `PerimeterOutputCollected` has `rotated_wall_loops: Vec<RotatedWallLoopEntry>` field
  - `HostPerimeterOutputBuilder::push_reordered_wall_loop` validates Z envelope, validates `feature_flags.len() == rotated_wall_loop.path.points.len()`, and stores the rotated entry
  - `HostPerimeterOutputBuilder::drop` continues to delete the resource
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — `PerimeterOutputCollected`, `HostPerimeterOutputBuilder::push_reordered_wall_loop`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — `perimeter-output-builder` host implementation
  - `docs/02_ir_schemas.md` — cardinality invariant for `feature_flags` and `path.points`
- Verification:
  - `cargo build -p slicer-host --tests 2>&1 | head -20` → should pass (method implemented, no missing symbols)
- Exit condition:
  Host implementation compiles. Unit test: `cargo test -p slicer-host --lib -- perimeter_output 2>&1 | tail -20` → relevant tests pass.

---

### Step 3: Update `perimeter_region_to_data` and `convert_perimeter_output` for rotated wall loops

- Task IDs:
  - `TASK-120c`
- Objective:
  Wire the rotated wall loops from `PerimeterOutputCollected.rotated_wall_loops` into `PerimeterIR` via `convert_perimeter_output`. When `rotated_wall_loops` is non-empty, use it to replace the original wall loop geometry in the committed `PerimeterIR`.
- Precondition:
  Step 2 is complete — `PerimeterOutputCollected` has `rotated_wall_loops` and `push_reordered_wall_loop` stores entries.
- Postcondition:
  - `perimeter_region_to_data` in `wit_host.rs` maps `rotated_wall_loops` entries into the `PerimeterRegionData.wall_loops` vector (replacing original geometry)
  - `convert_perimeter_output` applies rotated geometry to the committed `PerimeterIR.regions`
  - When `rotated_wall_loops` is empty, original wall loop order is preserved (backward compatibility)
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs` — `perimeter_region_to_data`, `PerimeterRegionData`
  - `crates/slicer-host/src/convert.rs` — `convert_perimeter_output` routing logic
- Authoritative docs:
  - `docs/04_host_scheduler.md` — `convert_perimeter_output` commit path
  - `docs/02_ir_schemas.md` — `PerimeterRegion` and `WallLoop` structure
- Verification:
  - `cargo build -p slicer-host --tests 2>&1 | head -20` → should pass
- Exit condition:
  Rotated wall loop geometry flows from `PerimeterOutputCollected` through to `PerimeterIR`. Original geometry preserved when no rotation is emitted.

---

### Step 4: Add SDK `push_reordered_wall_loop` to `PerimeterOutputBuilder` wrapper

- Task IDs:
  - `TASK-120c`
- Objective:
  Add the SDK-level `push_reordered_wall_loop` method to the `PerimeterOutputBuilder` struct so seam-placer can call it.
- Precondition:
  Step 2 is complete — the host WIT implementation exists.
- Postcondition:
  - `crates/slicer-sdk/src/postpass_builders.rs` — `PerimeterOutputBuilder::push_reordered_wall_loop` calls the generated WIT binding
  - `crates/slicer-macros/src/lib.rs` — `build_layer_world_glue` generates the binding for the new method
- Files expected to change:
  - `crates/slicer-sdk/src/postpass_builders.rs`
  - `crates/slicer-macros/src/lib.rs`
- Authoritative docs:
  - `docs/05_module_sdk.md` — SDK builder interface
- Verification:
  - `cargo build -p slicer-sdk 2>&1 | head -20` → should pass
- Exit condition:
  SDK wrapper is in place. Seam-placer can call `push_reordered_wall_loop`.

---

### Step 5: Implement seam rotation in `seam-placer` module

- Task IDs:
  - `TASK-120c`
- Objective:
  Implement the wall loop rotation logic in `seam-placer`. Given a `PerimeterRegionView` with wall loops and a `resolved_seam`, rotate `path.points` so seam is first. Call `push_reordered_wall_loop` for each wall loop that has a resolved seam.
- Precondition:
  Steps 1-4 complete. SDK method `push_reordered_wall_loop` is available to seam-placer.
- Postcondition:
  - `modules/core-modules/seam-placer/src/lib.rs` — rotation logic implemented, calls `push_reordered_wall_loop`
  - `modules/core-modules/seam-placer/src/lib.rs` — revert any `set_resolved_seam` calls (not needed since rotation handles it)
  - When `resolved_seam` is `None`, seam-placer emits no rotated wall loops (preserves original geometry)
  - When `seam_idx >= wall_loops.len()`, original wall loop is preserved (non-fatal, no error)
- Files expected to change:
  - `modules/core-modules/seam-placer/src/lib.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md` — `Layer::PerimetersPostProcess` stage definition
  - `docs/02_ir_schemas.md` — `PerimeterRegion`, `WallLoop`, `SeamPosition`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — rotation algorithm
- Verification:
  - `cargo build -p seam-placer --tests 2>&1 | head -20` → should pass (module compiles)
- Exit condition:
  Seam-placer rotates wall loop geometry and commits via `push_reordered_wall_loop`. Original geometry preserved when no seam is set or seam index is OOB.

---

### Step 6: Revert `path-optimization-default` to comment-only output

- Task IDs:
  - `TASK-151`
- Objective:
  Remove the `push-move` wall loop replay logic from `path-optimization-default`. It now reads `PerimeterIR` with seam-first geometry but does not need to replay anything — `PerimeterIR` is already correctly rotated. Revert to marker-comment-only output.
- Precondition:
  Steps 1-5 complete. `PerimeterIR` contains seam-first wall loops from seam-placer output.
- Postcondition:
  - `modules/core-modules/path-optimization-default/src/lib.rs` — removes all `push_move` calls and wall loop rotation logic
  - `modules/core-modules/path-optimization-default/src/lib.rs` — emits only the per-layer marker comment
  - `modules/core-modules/path-optimization-default/src/lib.rs` — `resolved_seam` read is removed (no longer needed for replay)
- Files expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md` — `Layer::PathOptimization` stage definition
- Verification:
  - `cargo build -p path-optimization-default --tests 2>&1 | head -20` → should pass
- Exit condition:
  PathOptimization emits only marker comments. No `push-move` calls. No wall loop replay logic.

---

### Step 7: Add integration tests for seam rotation and path-opt comment-only behavior

- Task IDs:
  - `TASK-120c`
  - `TASK-151`
- Objective:
  Add tests to `live_seam_path_tdd.rs` proving seam rotation commits correctly, `PerimeterIR` has seam-first geometry, and PathOptimization emits only comments.
- Precondition:
  Steps 1-6 complete. All implementations are in place.
- Postcondition:
  - `crates/slicer-host/tests/live_seam_path_tdd.rs` — new tests:
    - `seam_placer_rotates_wall_loop_points_to_seam_first`
    - `seam_placer_wall_loop_rotate_is_deterministic`
    - `out_of_bounds_seam_wall_index_preserves_original_loop`
    - `rotated_points_cardinality_mismatch_rejected`
    - `seam_z_outside_layer_envelope_rejected`
    - `no_resolved_seam_preserves_original_wall_order`
  - `modules/core-modules/path-optimization-default/tests/seam_consumption_tdd.rs` — update tests for comment-only output
- Files expected to change:
  - `crates/slicer-host/tests/live_seam_path_tdd.rs`
  - `modules/core-modules/path-optimization-default/tests/seam_consumption_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — test integration points
- Verification:
  - `cargo test -p slicer-host --test live_seam_path_tdd -- --nocapture` → all new tests pass
  - `cargo test -p path-optimization-default --test seam_consumption_tdd -- --nocapture` → all tests pass
- Exit condition:
  All acceptance criteria from `packet.spec.md` are verified by running tests.

---

## Packet Completion Gate

- All steps complete (Step 0 through Step 7).
- Every step exit condition is met.
- `docs/02_ir_schemas.md` updated in Step 0 explicitly documents the seam-first contract.
- `14_live-seam-placement-and-consumption/packet.spec.md` status updated to `superseded`.
- `docs/07_implementation_status.md` TASK-120c and TASK-151 task notes reflect this packet's closure.
- `cargo build --workspace` passes.
- `cargo clippy --workspace -- -D warnings` passes.
- All acceptance criteria from `packet.spec.md` pass.
- `packet.spec.md` status updated to `implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm all 7 step exit conditions are satisfied.
- Confirm no remaining CRIT issues from the code review of the original packet `14` implementation.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
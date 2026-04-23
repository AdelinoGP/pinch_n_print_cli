# Implementation Plan: 23-rev1_prepass-seam-planning-orca-parity

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Always run the narrow falsifying command immediately after the first substantive edit.

## Steps

### Step 1: Fix WIT world-prepass.wit — run-seam-planning parameter type

- Task IDs:
  - `TASK-159`
- Objective:
  Change `run-seam-planning` to accept `list<MeshObjectView>` instead of `list<object-id>`.
- Precondition:
  `wit/world-prepass.wit` exports `run-seam-planning` with `objects: list<object-id>`.
- Postcondition:
  `wit/world-prepass.wit` exports `run-seam-planning` with `objects: list<MeshObjectView>`. `MeshObjectView` is defined in the same WIT file.
- Files expected to change:
  - `wit/world-prepass.wit`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/02_ir_schemas.md`
- OrcaSlicer refs:
  - None (WIT interface design, not OrcaSlicer borrowing)
- Verification:
  - `grep -n "run-seam-planning" wit/world-prepass.wit`
  - Confirm parameter is `objects: list<MeshObjectView>`
- Exit condition:
  `wit/world-prepass.wit` source reflects the new signature.

---

### Step 2: Fix dispatch.rs — convert ObjectMesh to MeshObjectView for SeamPlanning

- Task IDs:
  - `TASK-159`
- Objective:
  Mirror the `MeshSegmentation` pattern: call `wit_host::object_mesh_to_wit_mesh_object_view` for each object and pass the resulting `Vec<MeshObjectView>` to `bindings.call_run_seam_planning`.
- Precondition:
  `dispatch.rs:688-691` builds `let object_ids: Vec<String>` and passes it to `call_run_seam_planning`.
- Postcondition:
  `dispatch.rs:688-691` builds `let mesh_object_views: Vec<_>` via `object_mesh_to_wit_mesh_object_view` and passes it to `call_run_seam_planning`. Pattern matches lines 668-673 (`MeshSegmentation`).
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md`
  - `wit/world-prepass.wit`
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo build -p slicer-host 2>&1 | head -30`
- Exit condition:
  `cargo build -p slicer-host` succeeds with no errors.

---

### Step 3: Fix slicer-macros seam_arm — correct sdk_objects type

- Task IDs:
  - `TASK-159`
- Objective:
  Change `sdk_objects` from `Vec<::slicer_ir::ObjectId>` to `Vec<::slicer_sdk::prepass_types::MeshObjectView>` so the macro-produced call is type-compatible with `PrepassModule::run_seam_planning(&self, objects: &[MeshObjectView], ...)`.
- Precondition:
  `slicer-macros/src/lib.rs:1713` constructs `let sdk_objects: ::std::vec::Vec<::slicer_ir::ObjectId>`.
- Postcondition:
  Line 1713 constructs `let sdk_objects: ::std::vec::Vec<::slicer_sdk::prepass_types::MeshObjectView>`. The `_objects` parameter passed from wit-bindgen is already `&[MeshObjectView]` so no other changes needed.
- Files expected to change:
  - `crates/slicer-macros/src/lib.rs`
- Authoritative docs:
  - `docs/05_module_sdk.md`
  - `crates/slicer-sdk/src/traits.rs` (trait signature)
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo build --workspace 2>&1 | head -40`
- Exit condition:
  `cargo build --workspace` succeeds.

---

### Step 4: Lower curvature threshold in seam-planner-default

- Task IDs:
  - `TASK-159`
- Objective:
  Lower the curvature threshold from `0.5` to `0.2` so ordinary cube corners produce seam candidates.
- Precondition:
  `modules/core-modules/seam-planner-default/src/lib.rs:144` contains `if curvature > 0.5`.
- Postcondition:
  Line 144 contains `if curvature > 0.2`.
- Files expected to change:
  - `modules/core-modules/seam-planner-default/src/lib.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — corner detection approach (not borrowed directly; curvature-based detection is industry-standard fallback)
- Verification:
  - `grep -n "curvature > " modules/core-modules/seam-planner-default/src/lib.rs`
- Exit condition:
  Grep confirms threshold is `0.2`.

---

### Step 5: Verify seam-planner-default IR access contract (already done)

- Task IDs:
  - `TASK-159`
- Objective:
  Confirm the `seam_planner_default_declares_prepass_contract_roots` test in `core_module_ir_access_contract_tdd.rs` passes.
- Precondition:
  Test 4 was added to `core_module_ir_access_contract_tdd.rs` in the previous session.
- Postcondition:
  `cargo test -p slicer-host --test core_module_ir_access_contract_tdd seam_planner_default_declares_prepass_contract_roots -- --exact --nocapture` passes.
- Files expected to change:
  - None (verify-only step)
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/03_wit_and_manifest.md`
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-host --test core_module_ir_access_contract_tdd seam_planner_default_declares_prepass_contract_roots -- --exact --nocapture`
- Exit condition:
  Test passes.

---

### Step 6: Rebuild core-modules WASM binaries

- Task IDs:
  - `TASK-159`
- Objective:
  Rebuild `seam-planner-default.wasm` so the new WIT signature is compiled in.
- Precondition:
  `modules/core-modules/seam-planner-default/` contains the old WASM built with the previous WIT interface.
- Postcondition:
  `modules/core-modules/seam-planner-default/seam-planner-default.wasm` is rebuilt.
- Files expected to change:
  - `modules/core-modules/seam-planner-default/seam-planner-default.wasm`
- Authoritative docs:
  - `docs/05_module_sdk.md`
  - `modules/core-modules/build-core-modules.sh`
- OrcaSlicer refs:
  - None
- Verification:
  - `ls -la modules/core-modules/seam-planner-default/seam-planner-default.wasm` (timestamp updated)
- Exit condition:
  WASM binary is rebuilt.

---

### Step 7: Run full acceptance ceremony

- Task IDs:
  - `TASK-159`
- Objective:
  Verify all 9 acceptance criteria pass.
- Precondition:
  All 4 code changes are made and WASM is rebuilt.
- Postcondition:
  All 9 ACs pass. Clippy clean.
- Files expected to change:
  - None (verification-only)
- Authoritative docs:
  - `packet.spec.md`
- OrcaSlicer refs:
  - None
- Verification:
  Each AC has its own pipe-suffixed command; run all in sequence:
  ```
  cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_commits_seam_plan_ir -- --exact --nocapture
  cargo test -p slicer-host --test dispatch_tdd seam_plan_ir_rejects_duplicate_region_keys -- --exact --nocapture
  cargo test -p slicer-host --test dispatch_tdd prepass_seam_planning_requires_layer_plan_slot -- --exact --nocapture
  cargo test -p slicer-host --test execution_plan_tdd prepass_seam_planning_stage_orders_between_layer_planning_and_paint_segmentation -- --exact --nocapture
  cargo test -p slicer-host --test live_seam_path_tdd seam_plan_ir_is_injected_into_wall_postprocess_region_view -- --exact --nocapture
  cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_prepass_seam_plan_matches_live_outer_wall_start -- --exact --nocapture
  cargo test -p slicer-host --test core_module_ir_access_contract_tdd seam_planner_default_declares_prepass_contract_roots -- --exact --nocapture
  cargo clippy --workspace -- -D warnings
  ```
- Exit condition:
  All commands pass. Packet status ready to move to `implemented`.

## Packet Completion Gate

- All 6 steps complete.
- Every step exit condition is met.
- All 9 packet acceptance criteria green.
- `cargo clippy --workspace -- -D warnings` clean.
- `docs/07_implementation_status.md` updated for `TASK-159`.
- Predecessor packet `23_prepass-seam-planning-orca-parity` marked `superseded` in its `packet.spec.md`.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm all 8 verification commands from `packet.spec.md` are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.

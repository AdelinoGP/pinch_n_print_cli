# Implementation Plan: wit-boundary-gaps-postpass

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Fix `dispatch_postpass_gcode_call` to pass real GCode command list

- Task IDs:
  - `TASK-129a`
- Objective:
  - Change the empty `&[]` slice in `dispatch_postpass_gcode_call` (dispatch.rs line 707) to pass the actual `gcode_ir.commands.as_slice()` so the live path carries real command data through the WIT boundary.
- Precondition:
  - `dispatch_postpass_gcode_call` passes `&[]` as the command list argument.
- Postcondition:
  - `dispatch_postpass_gcode_call` passes `gcode_ir.commands.as_slice()` (or equivalent owned copy) as the command list argument.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md` (PostPass execution, execute_postpass function)
  - `crates/slicer-host/src/dispatch.rs` (dispatch_postpass_gcode_call line 626-733)
  - `wit/deps/ir-types.wit` (gcode-output-builder lines 98-116)
- OrcaSlicer refs: None
- Verification:
  - `cargo build -p slicer-host`
  - NOTE: Real command slice verification (vs `&[]`) requires the TDD test in Step 2 to confirm.
- Exit condition:
  - The call to `bindings.call_run_gcode_postprocess` in `dispatch_postpass_gcode_call` receives the real command slice, not `&[]`.

### Step 2: Add `postpass_gcode_boundary_tdd` regression test

- Task IDs:
  - `TASK-129a`
- Objective:
  - Add a TDD test file `postpass_gcode_boundary_tdd.rs` that exercises all 8 `GCodeCommand` variants crossing the WIT boundary and asserts each field is preserved through the round trip.
- Precondition:
  - No test covers all 8 GCodeCommand variants crossing the postpass WIT boundary.
- Postcondition:
  - `postpass_gcode_boundary_tdd.rs` exists and passes, asserting exact field values for Move (all field combos), Retract, Unretract, FanSpeed, Temperature, ToolChange, Comment, Raw.
- Files expected to change:
  - `crates/slicer-host/tests/postpass_gcode_boundary_tdd.rs` (new file)
- Authoritative docs:
  - `docs/02_ir_schemas.md` (GCodeIR lines ~738-770)
  - `wit/deps/ir-types.wit` (gcode-move-cmd lines 112-116)
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-host --test postpass_gcode_boundary_tdd 2>&1`
  - NOTE: On test pass, grep should confirm `Move`, `Retract`, `Unretract`, `FanSpeed`, `Temperature`, `ToolChange`, `Comment`, `Raw` in output.
- Exit condition:
  - All 8 GCodeCommand variants are represented in the test assertions, and the test passes.

### Step 3: Add `postpass_gcode_command_preservation_tdd` regression test

- Task IDs:
  - `TASK-129a`
- Objective:
  - Add a TDD test file `postpass_gcode_command_preservation_tdd.rs` that verifies command order and content are identical after the round trip through `dispatch_postpass_gcode_call`.
- Precondition:
  - No test confirms order/content preservation for postpass GCode commands.
- Postcondition:
  - `postpass_gcode_command_preservation_tdd.rs` exists and passes, asserting commands are in the same order with identical content and no command is silently dropped or mutated.
- Files expected to change:
  - `crates/slicer-host/tests/postpass_gcode_command_preservation_tdd.rs` (new file)
- Authoritative docs:
  - `docs/02_ir_schemas.md` (GCodeIR lines ~738-770)
  - `crates/slicer-host/src/dispatch.rs` (dispatch_postpass_gcode_call line 626)
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-host --test postpass_gcode_command_preservation_tdd 2>&1`
  - NOTE: On test pass, grep should confirm `command.*preserved` or `order.*identical` in output.
- Exit condition:
  - Command order and content assertions pass; no command is dropped or mutated.

### Step 4: Add `layer_world_deep_copy_tdd` regression test

- Task IDs:
  - `TASK-129b`
- Objective:
  - Add a TDD test file `layer_world_deep_copy_tdd.rs` that proves `LayerCollectionIR` fields survive bit-for-bit through the layer-world WIT boundary when a layer-world module writes and the result is read back.
- Precondition:
  - Layer-world deep-copy behavior is only covered by native fallback code, not on the live WASM path.
- Postcondition:
  - `layer_world_deep_copy_tdd.rs` exists and passes, asserting all `LayerCollectionIR` fields (`ordered_entities.path.points`, `role`, `region_key`, `topo_order`, `tool_change.after_entity_index`, `z_hop.hop_height`) are preserved bit-for-bit.
- Files expected to change:
  - `crates/slicer-host/tests/layer_world_deep_copy_tdd.rs` (new file)
- Authoritative docs:
  - `docs/02_ir_schemas.md` (LayerCollectionIR lines ~1235-1252)
  - `crates/slicer-host/src/dispatch.rs` (dispatch_layer_call)
  - `wit/world-layer.wit`
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-host --test layer_world_deep_copy_tdd 2>&1`
  - NOTE: On test pass, grep should confirm `deep.copy.*pass` or `bit.for.bit` in output.
- Exit condition:
  - All LayerCollectionIR entity field assertions pass; bit-for-bit preservation confirmed.

### Step 5: Add `finalization_world_deep_copy_tdd` regression test

- Task IDs:
  - `TASK-129c`
- Objective:
  - Add a TDD test file `finalization_world_deep_copy_tdd.rs` that proves `Vec<LayerCollectionIR>` fields survive bit-for-bit through the finalization-world WIT boundary when a finalization-world module reads the layer collection.
- Precondition:
  - Finalization-world deep-copy behavior is only covered by native fallback code, not on the live WASM path.
- Postcondition:
  - `finalization_world_deep_copy_tdd.rs` exists and passes, asserting all layer indices, z values, `ordered_entities`, `tool_changes`, and `z_hops` are preserved bit-for-bit through the boundary.
- Files expected to change:
  - `crates/slicer-host/tests/finalization_world_deep_copy_tdd.rs` (new file)
- Authoritative docs:
  - `docs/02_ir_schemas.md` (LayerCollectionIR lines ~1235-1252)
  - `crates/slicer-host/src/dispatch.rs` (dispatch_finalization_call)
  - `wit/world-finalization.wit`
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-host --test finalization_world_deep_copy_tdd 2>&1`
  - NOTE: On test pass, grep should confirm `finalization.*deep.copy.*pass` in output.
- Exit condition:
  - All Vec<LayerCollectionIR> field assertions pass; bit-for-bit preservation confirmed across all layers.

### Step 6: Workspace gate

- Task IDs:
  - `TASK-129a`, `TASK-129b`, `TASK-129c`
- Objective:
  - Verify the full workspace builds and passes clippy with no warnings.
- Precondition:
  - All 5 steps above are complete and their exit conditions are met.
- Postcondition:
  - `cargo build --workspace` succeeds and `cargo clippy --workspace -- -D warnings` produces no warnings.
- Files expected to change: None (build verification only)
- Authoritative docs:
  - `CLAUDE.md` (Build & Test Commands section)
- OrcaSlicer refs: None
- Verification:
  - `cargo build --workspace && cargo clippy --workspace -- -D warnings`
- Exit condition:
  - Build succeeds with zero warnings.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Four new TDD test files pass: `postpass_gcode_boundary_tdd`, `postpass_gcode_command_preservation_tdd`, `layer_world_deep_copy_tdd`, `finalization_world_deep_copy_tdd`.
- Packet acceptance criteria green.
- `docs/07_implementation_status.md` updated for the packet task IDs (TASK-129, TASK-129a, TASK-129b, TASK-129c marked complete).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.

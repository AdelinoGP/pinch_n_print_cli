# Implementation Plan: wit-boundary-gaps-postpass

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.

## Steps

### Step 0: Repair packet docs before code changes

- Task IDs:
  - `TASK-129`
- Objective:
  - Rewrite the packet files so the implementation contract matches the selected architecture: widen postpass WIT now, keep TASK-129b on the existing layer-world commit path, and widen finalization WIT now.
- Precondition:
  - `packet.spec.md`, `requirements.md`, `design.md`, and `implementation-plan.md` still describe impossible thin-view or nonexistent read-surface behavior.
- Postcondition:
  - The packet docs are implementation-grade and no longer contain scope-changing open questions.
- Files expected to change:
  - `.ralph/specs/07_wit-boundary-gaps-postpass/packet.spec.md`
  - `.ralph/specs/07_wit-boundary-gaps-postpass/requirements.md`
  - `.ralph/specs/07_wit-boundary-gaps-postpass/design.md`
  - `.ralph/specs/07_wit-boundary-gaps-postpass/implementation-plan.md`
  - `.ralph/specs/07_wit-boundary-gaps-postpass/task-map.md`
- Authoritative docs:
  - `.ralph/specs/README.md`
  - `wit/world-postpass.wit`
  - `wit/world-finalization.wit`
  - `wit/world-layer.wit`
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-host --test wit_drift_detection_tdd 2>&1`
- Exit condition:
  - Packet docs describe only achievable surfaces and the selected approach is locked.

### Step 1: Widen canonical WIT for postpass and finalization

- Task IDs:
  - `TASK-129a`
  - `TASK-129c`
- Objective:
  - Update the canonical WIT files so postpass can express all eight `GCodeCommand` variants plus `push-unretract`, and finalization can expose `ordered_entities` and `z_hops` through `layer-collection-view`.
- Precondition:
  - `wit/deps/ir-types.wit`, `wit/world-postpass.wit`, and `wit/world-finalization.wit` do not yet express the required widened surfaces.
- Postcondition:
  - Canonical WIT defines the widened postpass and finalization boundary contracts.
- Files expected to change:
  - `wit/deps/ir-types.wit`
  - `wit/world-postpass.wit`
  - `wit/world-finalization.wit`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/02_ir_schemas.md`
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-host --test wit_drift_detection_tdd 2>&1`
- Exit condition:
  - Canonical WIT contains `push-unretract`, payload-bearing postpass command input, and finalization `ordered-entities` / `z-hops` read methods.

### Step 2: Mirror widened WIT into host, macro, SDK, and guest surfaces

- Task IDs:
  - `TASK-129a`
  - `TASK-129c`
- Objective:
  - Update every mirrored WIT or type surface so canonical WIT, host bindgen, macro glue, SDK APIs, and hand-written test guests stay aligned.
- Precondition:
  - Canonical WIT is widened, but inline host/macro WIT, SDK APIs, or guest WIT copies still reflect the old thin surfaces.
- Postcondition:
  - Host, macro, SDK, and guest surfaces compile against the widened postpass/finalization contracts without drift.
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-macros/src/lib.rs`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-sdk/src/postpass_builders.rs`
  - `crates/slicer-sdk/src/postpass_types.rs`
  - `test-guests/postpass-guest/src/lib.rs`
  - `crates/slicer-host/tests/wit_drift_detection_tdd.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/05_module_sdk.md`
  - `wit/world-postpass.wit`
  - `wit/world-finalization.wit`
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-host --test wit_drift_detection_tdd 2>&1`
  - `cargo test -p slicer-sdk --test postpass_module_tdd 2>&1`
  - `cargo test -p slicer-sdk --test finalization_module_tdd 2>&1`
- Exit condition:
  - Mirrored surfaces are aligned with canonical WIT and the drift / SDK tests pass.

### Step 3: Thread real postpass commands through the live runtime

- Task IDs:
  - `TASK-129a`
- Objective:
  - Add `commands: &[GCodeCommand]` parameter to `dispatch_postpass_gcode_call`, convert commands into the widened postpass WIT input, and pass real command payloads through the live postpass boundary instead of `&[]`.
- Precondition:
  - `dispatch_postpass_gcode_call` takes no commands parameter or still calls the WIT export with `&[]`.
- Postcondition:
  - `dispatch_postpass_gcode_call` accepts `commands: &[GCodeCommand]`, passes converted postpass input to the bindings call, and `WasmRuntimeDispatcher::run_gcode_postprocess` forwards `&gcode_ir.commands`.
- Files expected to change:
  - `crates/slicer-host/src/dispatch.rs` — `dispatch_postpass_gcode_call` signature and body, `WasmRuntimeDispatcher::run_gcode_postprocess` body
  - `crates/slicer-host/src/wit_host.rs` — postpass command collection and `push-unretract` host handling if not already completed in Step 2
- Authoritative docs:
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/postpass.rs`
  - `wit/world-postpass.wit`
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-host --test postpass_gcode_boundary_tdd 2>&1`
- Exit condition:
  - The live postpass runtime receives real command payloads instead of `&[]`.

### Step 4: Add postpass live-path regressions

- Task IDs:
  - `TASK-129a`
- Objective:
  - Add focused TDD regressions for widened postpass input, preserved output order/content, and the empty-list negative case.
- Precondition:
  - No focused live-path regressions prove payload preservation, `Unretract` output preservation, and empty-list behavior together.
- Postcondition:
  - `postpass_gcode_boundary_tdd`, `postpass_gcode_command_preservation_tdd`, and `postpass_gcode_empty_list_tdd` all pass.
- Files expected to change:
  - `crates/slicer-host/tests/postpass_gcode_boundary_tdd.rs`
  - `crates/slicer-host/tests/postpass_gcode_command_preservation_tdd.rs`
  - `crates/slicer-host/tests/postpass_gcode_empty_list_tdd.rs`
  - `crates/slicer-host/tests/postpass_executor_tdd.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `wit/world-postpass.wit`
  - `crates/slicer-host/src/dispatch.rs`
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-host --test postpass_gcode_boundary_tdd 2>&1`
  - `cargo test -p slicer-host --test postpass_gcode_command_preservation_tdd 2>&1`
  - `cargo test -p slicer-host --test postpass_gcode_empty_list_tdd 2>&1`
- Exit condition:
  - All three postpass regressions pass with exact assertion content.

### Step 5: Add layer-world builder-to-commit regression

- Task IDs:
  - `TASK-129b`
- Objective:
  - Add `layer_world_deep_copy_tdd.rs` proving the live layer-world builder/arena/commit path preserves entity fields, tool changes, and z-hops without introducing a new read surface.
- Precondition:
  - Layer-world field preservation is only proven by adjacent unit coverage, not a focused live-path regression anchored to this packet.
- Postcondition:
  - `layer_world_deep_copy_tdd.rs` exists and passes, asserting exact field values for committed entities, tool changes, and z-hops.
- Files expected to change:
  - `crates/slicer-host/tests/layer_world_deep_copy_tdd.rs`
  - `test-guests/` guest files only if a packet-specific witness guest is required
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/layer_executor.rs`
  - `wit/world-layer.wit`
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-host --test layer_world_deep_copy_tdd 2>&1`
- Exit condition:
  - The live layer-world commit path is covered and the regression passes.

### Step 6: Widen finalization host/runtime plumbing

- Task IDs:
  - `TASK-129c`
- Objective:
  - Implement the widened finalization `layer-collection-view` in host and macro/runtime plumbing so completed layers carry `ordered_entities` and `z_hops` across the live boundary.
- Precondition:
  - Canonical WIT is widened, but host or macro/runtime plumbing still drops `ordered_entities` or `z_hops`.
- Postcondition:
  - Finalization read plumbing carries full completed-layer content into guest-visible input.
- Files expected to change:
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-macros/src/lib.rs`
  - `crates/slicer-sdk/src/traits.rs`
  - `test-guests/sdk-finalization-guest/src/lib.rs`
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `wit/world-finalization.wit`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/wit_host.rs`
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-sdk --test finalization_module_tdd 2>&1`
- Exit condition:
  - Finalization host/runtime surfaces compile and the SDK finalization test passes.

### Step 7: Add finalization deep-copy regression

- Task IDs:
  - `TASK-129c`
- Objective:
  - Add `finalization_world_deep_copy_tdd.rs` proving widened finalization input preserves completed-layer data bit-for-bit through the live boundary.
- Precondition:
  - No focused live-path regression proves full finalization deep copy after WIT widening.
- Postcondition:
  - `finalization_world_deep_copy_tdd.rs` exists and passes, asserting exact preservation for `layer_index`, `z`, `ordered_entities`, `tool_changes`, and `z_hops`.
- Files expected to change:
  - `crates/slicer-host/tests/finalization_world_deep_copy_tdd.rs`
  - `crates/slicer-host/tests/macro_finalization_deep_copy_tdd.rs` if the supporting witness test needs widening
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `wit/world-finalization.wit`
  - `crates/slicer-host/src/dispatch.rs`
- OrcaSlicer refs: None
- Verification:
  - `cargo test -p slicer-host --test finalization_world_deep_copy_tdd 2>&1`
- Exit condition:
  - The focused finalization regression passes with exact assertion content.

### Step 8: Workspace gate

- Task IDs:
  - `TASK-129a`, `TASK-129b`, `TASK-129c`
- Objective:
  - Verify the full workspace builds and passes clippy with no warnings.
- Precondition:
  - All 7 implementation steps above are complete and their exit conditions are met.
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
- Five new TDD test files pass: `postpass_gcode_boundary_tdd`, `postpass_gcode_command_preservation_tdd`, `postpass_gcode_empty_list_tdd`, `layer_world_deep_copy_tdd`, `finalization_world_deep_copy_tdd`.
- `wit_drift_detection_tdd`, `postpass_module_tdd`, and `finalization_module_tdd` pass after the WIT-surface changes.
- Packet acceptance criteria green.
- `docs/07_implementation_status.md` updated for the packet task IDs only after the packet acceptance ceremony is green.
- `packet.spec.md` remains `draft` or moves according to explicit user direction; do not auto-close it mid-run.

## Acceptance Ceremony

- Re-run every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.

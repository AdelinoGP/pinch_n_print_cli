# Requirements: non-planar-z-envelope

## Packet Metadata

- Grouped task IDs:
  - `TASK-127`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

DEV-005 documents that the non-planar Z envelope `[layer.z, layer.z + effective_layer_height]` is not enforced at output-commit boundaries. Per-layer modules that emit paths with Z outside this envelope produce physically impossible output (extrusions at invalid print heights). Violations must be caught as fatal contract errors at the WIT boundary, not silently accepted.

The envelope rule is documented in `docs/01_system_architecture.md` (Non-Planar Z Envelope Rules, lines 260-268) but no runtime check exists. The gap is that any per-layer module can push geometry with arbitrary Z and the host accepts it without error.

This is a coherent slice: it adds one validation gate at every per-layer output-commit path, using the layer metadata already available at dispatch time.

## In Scope

- Z envelope validation at every per-layer `push_*` method in `HostExecutionContext` (`wit_host.rs`) that accepts `ExtrusionPath3d` or `Point3`
- Fatal contract error variant `Z_ENVELOPE_VIOLATION` with descriptive message (floor vs. ceiling, actual vs. bound)
- Catch-up layer envelope adjustment: lower bound becomes `catchup_z_bottom` when `is_catchup_layer = true`
- TDD test file `z_envelope_contract_tdd.rs` with positive and negative cases
- Boundary case tests: exactly at floor and exactly at ceiling are both valid (inclusive bounds)

## Out of Scope

- Postpass Z behavior (separate contract, future packet)
- Non-planar module Z behavior specifically (separate task)
- Mesh-query host services (no Z emission)
- Changes to IR schemas (envelope is a runtime enforcement, not a schema change)

## Authoritative Docs

- `docs/01_system_architecture.md` — Non-Planar Z Envelope Rules (lines 260-268), Per-Layer Error Handling Rules (lines 269-274)
- `docs/02_ir_schemas.md` — `GlobalLayer` struct fields: `z`, `effective_layer_height`, `is_catchup_layer`, `catchup_z_bottom` (lines 259-284)
- `docs/04_host_scheduler.md` — Phase 4 execution, Per-Layer Execution, LayerStageRunner trait
- `crates/slicer-host/src/wit_host.rs` — `HostExecutionContext` struct, per-layer output builder implementations (`push_sparse_path`, `push_solid_path`, `push_ironing_path`, `push_wall_loop`, `push_seam_candidate`, `push_support_path`, `push_interface_path`, `push_raft_path`)
- `crates/slicer-host/src/dispatch.rs` — `dispatch_layer_call`, `LayerParams`

## OrcaSlicer Reference Obligations

None. This is an internal contract enforcement task.

## Acceptance Summary

**Positive cases:**

- Z below `layer.z` → fatal `Z_ENVELOPE_VIOLATION` with message "Z {z} below layer.z floor {floor}"
- Z above `layer.z + effective_layer_height` → fatal `Z_ENVELOPE_VIOLATION` with message "Z {z} above layer.z ceiling {ceiling}"
- Catch-up layer with Z at `catchup_z_bottom + effective_layer_height` → no violation
- PerimeterIR-only module with all Z within envelope → completes without `Z_ENVELOPE_VIOLATION`
- Z exactly at floor `layer.z` → valid (inclusive lower bound)
- Z exactly at ceiling `layer.z + effective_layer_height` → valid (inclusive upper bound)

**Negative cases:**

- Postpass modules emitting invalid Z are OUT OF SCOPE for this packet (separate contract)
- Per-layer modules writing only `PerimeterIR` with valid Z complete cleanly

**Measurable outcomes:**

- All 8 `push_*` methods in `HostExecutionContext` that accept Z-bearing types have envelope validation
- `Z_ENVELOPE_VIOLATION` error code appears in test output for the below-floor and above-ceiling cases
- `z_envelope_contract_tdd.rs` test binary exists and passes all cases

## Verification Commands

- `cargo test -p slicer-host --test z_envelope_contract_tdd -- --nocapture`
- `cargo build --package slicer-host`
- `cargo clippy --package slicer-host -- -D warnings`

## Step Completion Expectations

Step 1 (TDD): Precondition is that `z_envelope_contract_tdd.rs` does not exist. Postcondition is that the file exists with all acceptance criteria as failing tests. Falsifying check: `cargo test -p slicer-host --test z_envelope_contract_tdd` must compile and run.

Step 2 (Context plumbing): Precondition is that `HostExecutionContext::new` does not accept layer parameters. Postcondition is that it accepts and stores `layer_z: f32`, `effective_layer_height: f32`, `catchup_z_bottom: Option<f32>`. Falsifying check: `cargo build --package slicer-host` succeeds.

Step 3 (Z validation in push methods): Precondition is that `push_*` methods do no Z validation. Postcondition is all 6 methods check Z against the envelope and return `Err(Z_ENVELOPE_VIOLATION)` on violation. Falsifying check: the TDD tests from Step 1 pass.

Step 4 (Workspace gate): Precondition is clippy warnings exist. Postcondition is `cargo clippy --package slicer-host -- -D warnings` passes. Falsifying check: the clippy command returns zero warnings.

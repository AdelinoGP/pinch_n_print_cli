# Implementation Plan: anchored-entity-execution

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-330`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract; do not broaden reads.

## Steps

### Step 0: Register the runtime integration target

- Task IDs: `TASK-330`
- Objective: register the runtime integration aggregator as the Cargo test binary named `integration`.
- Precondition: `crates/slicer-runtime/tests/integration/main.rs` exists as the intended integration-binary root.
- Postcondition: Cargo resolves `cargo test -p slicer-runtime --test integration` to `crates/slicer-runtime/tests/integration/main.rs`.
- Implementation: add `[[test]]` with `name = "integration"` and `path = "tests/integration/main.rs"` to `crates/slicer-runtime/Cargo.toml`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/Cargo.toml`
  - `crates/slicer-runtime/tests/integration/main.rs`
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/Cargo.toml`
- Files explicitly out of bounds:
  - all files outside this packet's implementation surface, `docs/07_implementation_status.md`, generated guests, `target/`
- Context cost: `S`
- Verification:
  - `cargo test -p slicer-runtime --test integration -- --list`
  - Confirm the target path in `cargo metadata --no-deps --format-version 1` is `crates/slicer-runtime/tests/integration/main.rs` and its target name is `integration`.
- Exit condition: Cargo discovers the `integration` binary from the subdirectory aggregator, and AC-3 through AC-7 plus AC-N1 can invoke that same target.

### Step 1: Establish anchored IR and event contracts

- Task IDs: `TASK-330`
- Objective: add stable anchored identity, planar/Z-span geometry, capabilities, provenance, and ordered event collection types.
- Precondition: live `GlobalLayer`, `LayerCollectionIR`, and all struct literals are inventoried.
- Postcondition: the new types compile, serialize, and preserve existing flat model-layer behavior.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` lines 1013-1048 and 2260-2366
  - `crates/slicer-ir/tests/ir_validation_tdd.rs` lines 1-120
  - delegated `LOCATIONS` inventory of all struct literals
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-ir/tests/ir_validation_tdd.rs`
  - `crates/slicer-ir/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/slicer-scheduler/**`, generated WIT, `target/`
- Blast-radius discipline: the delegated literal inventory must be appended to this step before activation; all affected literals and old schema assertions belong to the same step or this step must split.
- Expected sub-agent dispatches:
  - Question: list every `LayerCollectionIR` literal and schema assertion; scope: `crates/**/*.rs`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` §§1-2 - direct read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/` scheduling locations - delegate only if needed.
- Verification:
  - `cargo test -p slicer-ir --test ir_validation_tdd -- --exact`
  - `cargo check --workspace --all-targets`
- Exit condition: anchored types and every affected literal compile, and flat model-layer serialization remains unchanged.

### Step 2: Derive anchor closure

- Task IDs: `TASK-330`
- Objective: make the scheduler derive stage closure from capabilities while retaining the `layer-parallel-safe` hint.
- Precondition: Step 1 types exist and literal blast radius is closed.
- Postcondition: capability-derived closure is observable in the scheduler plan.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/execution_plan.rs` lines 15-46 and 265-459
  - `crates/slicer-runtime/src/layer_executor.rs` lines 1160-1260 and 1525-1575
  - `crates/slicer-ir/src/stage_io.rs` lines 257-344
- Files allowed to edit (at most 3):
   - `crates/slicer-scheduler/src/execution_plan.rs`
   - `crates/slicer-scheduler/tests/integration/capability_derived_anchor_closure.rs`
   - `crates/slicer-scheduler/tests/integration/main.rs`
- Test module to create and register in this step: `crates/slicer-scheduler/tests/integration/capability_derived_anchor_closure.rs` in `crates/slicer-scheduler/tests/integration/main.rs`. This is the closure driver named by AC-2.
- Files explicitly out of bounds:
  - support module implementations, packet 213, `docs/07_implementation_status.md`, `target/`
- Expected sub-agent dispatches:
  - Question: locate global-layer worker entry and parallel hint propagation; scope: `crates/slicer-runtime/src`, `crates/slicer-scheduler/src`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
   - `docs/adr/0009-raft-as-layer-infill-role.md` - delegated `SUMMARY`.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Print.cpp` - delegate `LOCATIONS` only.
- Verification:
  - `cargo test -p slicer-scheduler --test scheduler_integration capability_derived_anchor_closure -- --exact`
- Exit condition: capability closure is observable in the plan and the scheduler test passes.

### Step 3: Commit ordered runtime events

- Task IDs: `TASK-330`
- Objective: make runtime workers commit ordered event collections while retaining raft prefix behavior.
- Precondition: scheduler closure exists.
- Postcondition: planar-before-model ordering is observable in the runtime integration harness.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` lines 1878-2030
  - `crates/slicer-wasm-host/src/marshal/out.rs` lines 155-276
  - delegated cooling/time owner locations
- Files allowed to edit (at most 3):
   - `crates/slicer-runtime/src/layer_executor.rs`
   - `crates/slicer-runtime/tests/integration/anchored_event_ordering.rs`
   - `crates/slicer-runtime/tests/integration/main.rs`
- Files explicitly out of bounds:
  - support-family contract files, Orca source, generated guests, packet 213
- Test module to create and register in this step: `crates/slicer-runtime/tests/integration/anchored_event_ordering.rs` in `crates/slicer-runtime/tests/integration/main.rs`. This is the ordering driver named by AC-3.
- Context cost: `M`
- Authoritative docs:
   - `docs/adr/0059-support-families-and-anchored-entities.md` - delegated `SUMMARY`.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Layer.cpp` - delegate `LOCATIONS` only.
- Verification:
   - `cargo test -p slicer-runtime --test integration anchored_event_ordering -- --exact`
- Exit condition: AC-3 passes and event order is deterministic for the covered runtime case.

### Step 4: Add Z-span validation

- Task IDs: `TASK-330`
- Objective: validate Z-spanning paths atomically against each entity's declared range.
- Precondition: ordered event commit exists.
- Postcondition: out-of-range geometry is rejected without clipping or partial retention.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` lines 1878-2030
  - delegated validation owner locations
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/tests/integration/anchored_z_span_validation.rs`
  - `crates/slicer-runtime/tests/integration/main.rs`
- Files explicitly out of bounds:
  - support-family contract files, Orca source, generated guests, packet 213
- Expected sub-agent dispatches:
  - Question: locate Z-span validation seam; scope: `crates/slicer-runtime/src`; return: `LOCATIONS`.
- Test module to create and register in this step: `anchored_z_span_validation` in the runtime integration aggregator. This drives AC-4 and AC-N1.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0059-support-families-and-anchored-entities.md` - delegated `SUMMARY`.
- Verification:
  - `cargo test -p slicer-runtime --test integration anchored_z_span_validation -- --exact`
- Exit condition: AC-4 and AC-N1 pass for valid and rejected Z-spanning paths.

### Step 5: Add planar validation and event accounting

- Task IDs: `TASK-330`
- Objective: reject planar Z mismatches and run optimization/cooling/time accounting per physical event.
- Precondition: ordered event commit and Z-span validation exist.
- Postcondition: planar rejection is atomic and accounting does not reorder event boundaries.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` lines 1878-2030
  - `crates/slicer-wasm-host/src/marshal/out.rs` lines 155-276
  - delegated cooling/time owner locations
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/tests/integration/anchored_z_validation.rs`
  - `crates/slicer-runtime/tests/integration/main.rs`
- Files explicitly out of bounds:
  - support-family contract files, Orca source, generated guests, packet 213
- Expected sub-agent dispatches:
  - Question: identify cooling/time accounting owner; scope: `crates/slicer-runtime/src`; return: `LOCATIONS`.
- Test module to create and register in this step: `anchored_z_validation` in the runtime integration aggregator. This drives AC-5.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` and `docs/04_host_scheduler.md` - delegated bounded summaries for doc updates.
- Verification:
  - `cargo test -p slicer-runtime --test integration anchored_z_validation -- anchored_entity_planar_z_mismatch --exact`
  - `cargo xtask build-guests --check`
- Exit condition: AC-5 passes and malformed planar output leaves no partial event.

### Step 6: Add per-event accounting regression

- Task IDs: `TASK-330`
- Objective: prove optimization and cooling/time accounting remain isolated per physical event.
- Precondition: runtime validation seams exist.
- Postcondition: optimized event collections and accounting are deterministic without crossing event boundaries.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` lines 1878-2030
  - delegated cooling/time owner locations
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/tests/integration/anchored_event_accounting.rs`
  - `crates/slicer-runtime/tests/integration/main.rs`
- Files explicitly out of bounds:
  - support-family contract files, Orca source, generated guests, packet 213
- Expected sub-agent dispatches:
  - Question: identify cooling/time accounting owner; scope: `crates/slicer-runtime/src`; return: `LOCATIONS`.
- Test module to create and register in this step: `anchored_event_accounting` in the runtime integration aggregator. This drives AC-6.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0059-support-families-and-anchored-entities.md` - delegated `SUMMARY`.
- Verification:
  - `cargo test -p slicer-runtime --test integration anchored_event_accounting -- --exact`
- Exit condition: AC-6 passes with independent optimization and accounting.

### Step 7: Add parallel determinism regression

- Task IDs: `TASK-330`
- Objective: prove forced serial and parallel anchored execution produce equal ordered collections.
- Precondition: runtime event commit is tested.
- Postcondition: `layer-parallel-safe` governs concurrency without changing output.
- Files allowed to read, with ranges when over 300 lines:
   - `crates/slicer-runtime/src/layer_executor.rs` lines 1160-1260
   - delegated parallel worker locations
- Files allowed to edit (at most 3):
   - `crates/slicer-runtime/src/layer_executor.rs`
   - `crates/slicer-runtime/tests/integration/anchored_parallel_determinism.rs`
   - `crates/slicer-runtime/tests/integration/main.rs`
- Files explicitly out of bounds:
  - support module manifests and implementations, generated bindings, `target/`
- Expected sub-agent dispatches:
   - Question: locate forced serial/parallel worker seam; scope: `crates/slicer-runtime/src`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
   - `docs/adr/0059-support-families-and-anchored-entities.md` - delegated `SUMMARY`.
- OrcaSlicer refs:
  - none required for the boundary migration.
- Verification:
   - `cargo test -p slicer-runtime --test integration anchored_parallel_determinism -- --exact`
- Exit condition: AC-7 passes under forced serial and parallel execution.

### Step 8: Migrate SDK/WIT producer and optimizer seams

- Task IDs: `TASK-330`
- Objective: expose anchored contracts to guest producers and `Layer::PathOptimization` without preserving a false model-layer Z restriction.
- Precondition: host IR/runtime contract is tested.
- Postcondition: WIT/SDK/macro bindings preserve IDs, capabilities, provenance, and event atomicity.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/ir-types.wit` lines 1-87
  - `crates/slicer-schema/wit/deps/layer-path-optimization/layer-path-optimization.wit` lines 1-80
  - `crates/slicer-macros/src/lib.rs` lines 1120-1325 and 2900-3320
- Files allowed to edit (at most 3):
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-sdk/src/*` selected anchored boundary file
  - `crates/slicer-macros/src/lib.rs`
- Files explicitly out of bounds:
  - support module manifests and implementations, generated bindings, `target/`
- Expected sub-agent dispatches:
  - Question: locate all SDK/macro path conversion and optimizer builder seams; scope: `crates/slicer-sdk/src`, `crates/slicer-macros/src/lib.rs`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated bounded summary.
- Verification:
  - `cargo test -p slicer-macros --test slicer_module_tdd -- --exact`
  - `cargo xtask build-guests --check`
  - `cargo check --workspace --all-targets`
- Exit condition: guest boundary tests pass and no generated guest is stale.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Register runtime integration test target |
| Step 1 | M | IR and literal blast radius |
| Step 2 | M | Scheduler closure and integration driver |
| Step 3 | M | Runtime ordered-event commit and integration driver |
| Step 4 | M | Z-span validation and integration driver |
| Step 5 | M | Planar validation and integration driver |
| Step 6 | M | Per-event accounting and integration driver |
| Step 7 | M | Serial/parallel determinism and integration driver |
| Step 8 | M | WIT/SDK/macro migration |

Split before activation if aggregate cost exceeds M or any step is L; each step is independently testable and remains within the three-file edit cap.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Every AC command using `--test integration` resolves to the Step 0 target; AC-3, AC-4, AC-5, AC-6, AC-7, and AC-N1 use the registered runtime aggregator.
- `docs/07_implementation_status.md` is updated through a worker dispatch, never a full backlog read.
- `packet.spec.md` is ready for `status: implemented` only after the cooling/accounting blocker is resolved.

## Acceptance Ceremony

- Re-dispatch every AC and packet-level gate command.
- Record remaining packet-local risk and generated-guest freshness.
- Confirm context remained within the standard band.

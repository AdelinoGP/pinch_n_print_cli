# Implementation Plan: 205c-native-dispatch-seam

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-329`.
- Use TDD, then implementation, then the narrowest falsifying validation.

## Steps

### Step 1: Inventory and pin the shared seam

- Task IDs: `TASK-329`
- Objective: identify the single view authority, all affected constructors, and every lossy response field before editing.
- Precondition: 205b native transports are implemented.
- Postcondition: the inventory names all struct literals, adapters, commit variants, and focused test binaries.
- Files allowed to read, with ranges: `binding.rs:1-70`; `execution_plan_live.rs:330-375`; `native.rs:1-230, 400-980`; `validation.rs:60-105`; focused test paths.
- Files allowed to edit: none.
- Files explicitly out of bounds: module algorithms, WIT sources, editions, generated code.
- Expected sub-agent dispatches: struct-literal and response-field inventories; return `LOCATIONS`.
- Context cost: `S`.
- Authoritative docs: ADR-0005, ADR-0021, ADR-0056.
- Verification: `rg -n 'CompiledModuleLive|NativeStageEntry|resolve_layer_held_claims_map|commit_native_' crates/slicer-wasm-host/src`.
- Exit condition: no affected construction site remains unidentified.

### Step 2: Unify request marshalling and held claims

- Task IDs: `TASK-329`
- Objective: make native and WASM adapters consume one view authority and call one canonical per-region held-claim resolver.
- Precondition: Step 1 inventory is complete.
- Postcondition: duplicate conversion and resolver logic are deleted; existing native claim and empty-input tests remain meaningful.
- Files allowed to read: Step 1 files plus `slicer-sdk/src/views.rs` and `host.rs:1500-1700`.
- Files allowed to edit: `crates/slicer-sdk/src/views.rs`; `crates/slicer-wasm-host/src/marshal/native.rs`; `crates/slicer-wasm-host/src/dispatch.rs`.
- Files explicitly out of bounds: module crates, WIT, edition files.
- Expected sub-agent dispatches: compile/check after view-shape edits; return `FACT`.
- Context cost: `M`.
- Authoritative docs: ADR-0021 and `CONTEXT.md` marshalling terms.
- Verification: `cargo test -p slicer-runtime --test contract -- native_infill_claim_resolution 2>&1 | rg '^test result'`.
- Exit condition: one resolver and one view authority are used by both dispatch legs, with no completeness-mirror conversion table.

### Step 3: Complete response commits and validate dispatch mode

- Task IDs: `TASK-329`
- Objective: preserve all currently supported native response fields and fail integrated modules without native entries at load time.
- Precondition: Step 2 compiles and existing uncommitted regression intent is represented by tests.
- Postcondition: no supported response silently drops metadata; live bindings have explicit valid mode; external override remains WASM.
- Files allowed to read: `marshal/native.rs:400-980`; `binding.rs`; `execution_plan_live.rs:330-375`; existing integration tests.
- Files allowed to edit: `crates/slicer-wasm-host/src/marshal/native.rs`; `crates/slicer-wasm-host/src/binding.rs`; `crates/slicer-wasm-host/src/execution_plan_live.rs`.
- Files explicitly out of bounds: CLI, dist, module registry, WIT.
- Expected sub-agent dispatches: targeted cargo test/check; return `FACT` or <=20 failure lines.
- Context cost: `M`.
- Authoritative docs: ADR-0005, ADR-0056, ADR-0057.
- Verification: `cargo test -p slicer-scheduler --test integrated_tier_tdd --all-targets` and focused runtime contract tests.
- Exit condition: AC-1 through AC-N1 all have a passing targeted test or static assertion, and no late `MissingComponent` path remains for missing integrated entries.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
|---|---|---|
| 1 | S | inventory only |
| 2 | M | shared view and resolver seam |
| 3 | M | response and loader blast radius |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch.
- `packet.spec.md` is ready for `status: implemented`.

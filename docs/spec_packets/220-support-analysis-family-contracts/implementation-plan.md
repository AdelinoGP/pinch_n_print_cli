# Implementation Plan: support-analysis-family-contracts

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-331`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field is independent; do not write “see Step 1”.

## Steps

### Step 1: Add host support-analysis IR and exact-Z query seam

- Task IDs: `TASK-331`
- Objective: add `SupportAnalysisIR`, `PrePass::SupportAnalysis`, blackboard storage, deterministic candidate assignment, and normalized exact-Z query service.
- Precondition: TASK-330 anchored exports are available in draft form; exact-Z owner location dispatch is answered.
- Postcondition: host analysis produces strategy-neutral inputs and cached immutable exact-Z results without planning family bodies.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` lines 1013-1207
  - `crates/slicer-ir/src/stage_io.rs` lines 257-344
  - delegated exact-Z host locations
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-ir/src/stage_io.rs`
  - selected host analysis/query file identified by dispatch
- Files explicitly out of bounds:
  - WIT generated files, support module algorithms, packet 213, `target/`
- Blast-radius discipline: inventory every `BlackboardPrepassSlot` match and every `SupportGeometryIR`/support-plan literal before editing; all affected matches and assertions belong to this step.
- Expected sub-agent dispatches:
  - Question: locate exact-Z occupancy and blackboard/prepass runner seams; scope: `crates/slicer-wasm-host/src`, `crates/slicer-runtime/src`, `crates/slicer-ir/src`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-families-anchored-entities-plan.md` §§3-4 - direct read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - delegate locations only.
- Verification:
  - `cargo test -p slicer-wasm-host --test contract exact_z_support_query -- --exact`
  - `cargo check --workspace --all-targets`
- Test registration sub-step (at most 3 files): add `crates/slicer-wasm-host/tests/contract/exact_z_support_query.rs` and register `mod exact_z_support_query;` in `crates/slicer-wasm-host/tests/contract/main.rs`; these are the exact module and aggregator driven by AC-2.
- Exit condition: analysis and exact-Z query tests prove no family-specific body propagation occurs in host analysis.

### Step 2: Migrate universal SupportPlanIR and WIT/SDK producer contract

- Task IDs: `TASK-331`
- Objective: replace branch extrusion paths with structural body/role/demand/family metadata and migrate the prepass WIT, SDK, macro, marshal, blackboard, and schema assertions.
- Precondition: Step 1 analysis/query seams compile; schema migration decision is recorded.
- Postcondition: planner output is structural, has no nozzle-width paths, and all current consumers compile against the selected version.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` lines 1141-1207
  - `crates/slicer-schema/wit/deps/prepass-support-geometry/prepass-support-geometry.wit` lines 1-90
  - `crates/slicer-macros/src/lib.rs` lines 2300-2343
  - `crates/slicer-wasm-host/src/marshal/out.rs` lines 155-276
- Files allowed to edit (at most 3 per sub-step):
  - IR schema and its direct tests
  - WIT/SDK/macro boundary files
  - host marshal/blackboard files
- Files explicitly out of bounds:
  - tree/traditional algorithm bodies, generated bindings, `target/`, packet 213
- Blast-radius discipline: every `SupportPlanIR` literal, schema assertion, WIT record consumer, and `SupportPlanEntry.branch_segments` use must be inventoried and edited in the migration step; do not defer compile fallout.
- Expected sub-agent dispatches:
  - Question: enumerate all legacy support plan literals/consumers and test aggregators; scope: `crates/**/*.rs`, `crates/slicer-schema/wit/**`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md` - delegated `SUMMARY`.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegate locations only.
- Verification:
  - `cargo test -p slicer-wasm-host --test contract support_plan_structural_contract -- --exact`
  - `cargo xtask build-guests --check`
- Test registration sub-step (at most 3 files): add `crates/slicer-wasm-host/tests/contract/support_plan_structural_contract.rs` and register `mod support_plan_structural_contract;` in `crates/slicer-wasm-host/tests/contract/main.rs`; these are the exact module and aggregator driven by AC-3.
- Exit condition: structural contract rejects nozzle-width plan paths and all generated guests are fresh.

### Step 3: Migrate structured SupportIR and anchored rendering handoff

- Task IDs: `TASK-331`
- Objective: preserve family/body/demand/role identity from renderer output through TASK-330 ordered anchored events and G-code handoff.
- Precondition: structural plan contract exists and TASK-330 event collections are available.
- Postcondition: `SupportIR` entries are attributed and support paths are emitted only by family renderers.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` lines 2170-2200 and 2260-2349
  - `crates/slicer-wasm-host/src/marshal/out.rs` lines 155-276
  - `crates/slicer-runtime/src/layer_executor.rs` lines 1525-1875
- Files allowed to edit (at most 3):
   - `crates/slicer-ir/src/slice_ir.rs`
   - `crates/slicer-wasm-host/src/marshal/out.rs`
   - `crates/slicer-runtime/src/layer_executor.rs`
- Runtime test-target/registration sub-step (at most 3 files): add `[[test]] name = "integration"` with `path = "tests/integration/main.rs"` to `crates/slicer-runtime/Cargo.toml`, add `crates/slicer-runtime/tests/integration/structured_support_identity.rs`, and register `mod structured_support_identity;` in `crates/slicer-runtime/tests/integration/main.rs`; these are the exact target, module, and aggregator driven by AC-4.
- Files explicitly out of bounds:
  - support algorithm implementations, scheduler claim selection, generated WIT, packet 213
- Blast-radius discipline: every `SupportIR` literal and flat-path assertion is in this step's edit inventory; preserve raft/ironing role handling explicitly.
- Expected sub-agent dispatches:
  - Question: locate all `SupportIR` literals and flattening consumers; scope: `crates/**/*.rs`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated bounded summary.
- OrcaSlicer refs:
  - none required for identity plumbing.
- Verification:
  - `cargo test -p slicer-runtime --test integration structured_support_identity -- --exact`
  - `cargo check --workspace --all-targets`
- Exit condition: body and demand identity survives render, optimization, diagnostics, and G-code handoff.

### Step 4: Implement family claims, region selection, pairing, aggregation, and fallback removal

- Task IDs: `TASK-331`
- Objective: retain family candidates per region, select planner/renderer pairs atomically, aggregate/validate plans, and reject missing pairs before slicing.
- Precondition: Steps 1-3 contracts compile; current manifests and dedup tests are inventoried.
- Postcondition: `support_family` and `support_type` aliases select one matched family per region; malformed pairing is fatal; missing plans never invoke a fallback filler.
- Selector contract: canonical `support_family` is overridden only by the region's compatibility `support_type` when present; `normal*` and `classic*` select the traditional family, while `tree*` and `hybrid*` select the tree family. The AC-5 test must assert all four alias prefixes using the real keys `support_family` and `support_type`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/execution_plan.rs` lines 219-459
  - `modules/core-modules/support-planner/support-planner.toml` lines 1-18
  - `modules/core-modules/tree-support/tree-support.toml` lines 1-18
  - `modules/core-modules/traditional-support/traditional-support.toml` lines 1-18
- Files allowed to edit (at most 3 per sub-step):
  - scheduler selection/validation and its integration tests
  - three support manifests and manifest fixtures
  - host aggregation/dispatch and contract tests
- Files explicitly out of bounds:
  - TASK-332/TASK-333 algorithms, Orca source, `docs/07_implementation_status.md`, packet 213
- Expected sub-agent dispatches:
  - Question: enumerate support-generator dedup and fallback call sites; scope: `crates/slicer-scheduler`, `crates/slicer-runtime`, `crates/slicer-wasm-host`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md`, `docs/03_wit_and_manifest.md` - delegated `SUMMARY`.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - delegate locations only.
- Verification:
  - `cargo test -p slicer-scheduler --test scheduler_integration support_family_selection -- --exact`
  - `cargo test -p slicer-scheduler --test scheduler_integration support_family_pairing_rejected -- --exact`
  - `cargo test -p slicer-runtime --test integration support_disabled_no_output -- --exact`
- Scheduler registration sub-step (at most 3 files): add `crates/slicer-scheduler/tests/integration/support_family_selection.rs` and `crates/slicer-scheduler/tests/integration/support_family_pairing_rejected.rs`, and register both modules in `crates/slicer-scheduler/tests/integration/main.rs`; the `scheduler_integration` target already exists in `crates/slicer-scheduler/Cargo.toml`. AC-5 and AC-N1 drive these exact modules.
- Host contract registration sub-steps (each at most 2 files): add `crates/slicer-wasm-host/tests/contract/support_plan_validation.rs` and register `mod support_plan_validation;` in `crates/slicer-wasm-host/tests/contract/main.rs` for AC-6; add `crates/slicer-wasm-host/tests/contract/support_decline_contract.rs` and register `mod support_decline_contract;` in the same aggregator for AC-7.
- Runtime test registration sub-step (at most 3 files): add `crates/slicer-runtime/tests/integration/support_disabled_no_output.rs`, register `mod support_disabled_no_output;` in `crates/slicer-runtime/tests/integration/main.rs`, and use the `integration` Cargo target added in Step 3; AC-N2 drives this module.
- Exit condition: family selection, fatal mismatch, and no-fallback behavior are each directly exercised by the named integration drivers.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | Analysis and exact-Z host seam |
| Step 2 | M | Breaking structural plan migration |
| Step 3 | M | Structured output identity migration |
| Step 4 | M | Claims, pairing, aggregation, fallback removal |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` is updated through a worker dispatch, never a full backlog read.
- Both `[BLOCK]` design questions are resolved and documented before `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC and packet-level gate command.
- Regenerate guest WASM and inspect model-backed support taps in downstream packets.
- Confirm no fallback filler or legacy branch-path schema remains active.

# Implementation Plan: 205c-native-dispatch-seam

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-329`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Inventory and pin the shared seam

- Task IDs: `TASK-329`
- Objective: identify the single view authority, all affected constructors, and every lossy response field before editing.
- Precondition: 205b native transports are implemented; HEAD includes the native claim, empty-perimeter, and resolved-seam parity fixes.
- Postcondition: the inventory names all struct literals, adapters, commit variants, and focused test binaries.
- Files allowed to read, with ranges:
  - `crates/slicer-wasm-host/src/binding.rs` - lines 1-70
  - `crates/slicer-wasm-host/src/execution_plan_live.rs` - lines 330-375
  - `crates/slicer-wasm-host/src/marshal/native.rs` - lines 1-230, 400-980
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - lines 280-430
  - `crates/slicer-scheduler/src/validation.rs` - lines 60-105
  - `crates/slicer-sdk/src/prepass_types.rs` - lines 225-274
  - focused test paths (file names + test names only)
- Files allowed to edit: none.
- Files explicitly out of bounds: module algorithms, WIT sources, editions, generated code.
- Expected sub-agent dispatches: struct-literal and response-field inventories; return `LOCATIONS`.
- Context cost: `S`.
- Authoritative docs: ADR-0005, ADR-0021, ADR-0056.
- Verification:
  - `rg -n 'CompiledModuleLive|NativeStageEntry|resolve_layer_held_claims_map|commit_native_' crates/slicer-wasm-host/src` - FACT
- Exit condition: no affected construction site remains unidentified.

### Step 2: Unify request marshalling and held claims

- Task IDs: `TASK-329`
- Objective: make native and WASM adapters consume one view authority (constructors on the plain SDK view types) and call one canonical per-region held-claim resolver.
- Precondition: Step 1 inventory is complete.
- Postcondition: duplicate conversion and resolver logic are deleted; existing native claim and empty-input tests remain meaningful.
- Files allowed to read: Step 1 files plus `crates/slicer-sdk/src/views.rs` and `crates/slicer-wasm-host/src/host.rs:1500-1700`.
- Files allowed to edit:
  - `crates/slicer-sdk/src/views.rs`
  - `crates/slicer-wasm-host/src/marshal/native.rs`
  - `crates/slicer-wasm-host/src/marshal/in_.rs`
  - `crates/slicer-wasm-host/src/dispatch.rs`
- Files explicitly out of bounds: module crates, WIT, edition files.
- Expected sub-agent dispatches: compile/check after view-shape edits; return `FACT`.
- Context cost: `M`.
- Authoritative docs: ADR-0021 and `CONTEXT.md` marshalling terms.
- Verification:
  - `cargo test -p slicer-runtime --test contract --all-targets -- native_infill_claim_resolution 2>&1 | rg '^test result'` - FACT pass/fail
  - AC-1 static check: `test -f crates/slicer-wasm-host/src/marshal/native.rs && rg -q 'SliceRegionView|PerimeterRegionView' crates/slicer-wasm-host/src/marshal/native.rs && rg -q 'SliceRegionView|PerimeterRegionView' crates/slicer-wasm-host/src/marshal/in_.rs && ! rg -q 'completeness mirror|Completeness mirror' crates/slicer-wasm-host/src/marshal/native.rs && echo PASS` - FACT pass/fail
- Exit condition: one resolver and one view authority are used by both dispatch legs, with no completeness-mirror conversion table.

### Step 3: Support-origin builder contract

- Task IDs: `TASK-329`
- Objective: add the WIT `set-current-origin` method to `support-output-builder`, SDK `SupportOutputBuilder` origin tracking, the macros drain forwarding, and the host `set_current_origin` implementation.
- Precondition: Step 2 compiles.
- Postcondition: the support builder carries per-push origins; the host routes `set_current_origin` to the correct origin bucket; existing guests remain valid (additive API).
- Files allowed to read, with ranges:
  - `crates/slicer-schema/wit/deps/ir-types.wit` - lines 150-215
  - `crates/slicer-sdk/src/builders.rs` - lines 1-130, 458-529
  - `crates/slicer-wasm-host/src/host.rs` - lines 3500-3635, 3867-3923
  - `crates/slicer-macros/src/lib.rs` - drain section only
- Files allowed to edit:
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-sdk/src/builders.rs`
  - `crates/slicer-macros/src/lib.rs`
  - `crates/slicer-wasm-host/src/host.rs`
  - `crates/slicer-wasm-host/tests/contract/set_current_origin_routes_to_correct_bucket_tdd.rs` (author the support variant)
- Files explicitly out of bounds: module crates, WIT package/version changes, dist, CLI.
- Expected sub-agent dispatches: targeted cargo test/check; return `FACT` or <=20 failure lines.
- Context cost: `M`.
- Authoritative docs: ADR-0021, `CONTEXT.md` marshalling terms.
- Verification:
  - `cargo test -p slicer-wasm-host --all-targets 2>&1 | rg 'test result: ok|0 failed'` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT pass/fail; rebuild without `--check` if `STALE:` is reported
- Exit condition: the host contract test for support `set_current_origin` passes; guests are fresh.

### Step 4: Support-origin IR and marshal

- Task IDs: `TASK-329`
- Objective: give `SupportIR` a per-region shape and make both legs emit it, with the native leg reading the builder's origin accessors.
- Precondition: Step 3 compiles and the builder contract tests pass.
- Postcondition: `SupportIR` carries per-region regions; the native leg reads the builder's origin accessors; the WASM leg emits the same per-region shape; no empty-origin substitution remains.
- Files allowed to read, with ranges:
  - `crates/slicer-wasm-host/src/marshal/native.rs` - lines 742-848
  - `crates/slicer-wasm-host/src/marshal/out.rs` - lines 157-276
  - `crates/slicer-ir/src/slice_ir.rs` - lines 2089-2190
- Files allowed to edit:
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-wasm-host/src/marshal/out.rs`
  - `crates/slicer-wasm-host/src/marshal/native.rs`
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/src/visual_debug_render.rs`
  - `crates/slicer-runtime/tests/contract/parity_invariants_selftest_tdd.rs`
  - `crates/slicer-runtime/tests/contract/dispatch_identity_tdd.rs`
  - `crates/slicer-runtime/tests/contract/dispatch_support_output_tdd.rs`
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` (author `native_support_dispatch_preserves_per_region_origins`)
  - `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs`
  - `crates/slicer-runtime/tests/executor/layer_executor_tdd.rs`
  - `crates/slicer-runtime/tests/common/mod.rs`
  - `crates/slicer-runtime/tests/common/parity_invariants.rs`
  - `crates/slicer-runtime/tests/unit/blackboard_layer_arena_tdd.rs`
  - `crates/slicer-ir/tests/ir_tests.rs`
- Files explicitly out of bounds: module crates, WIT, dist, CLI.
- Blast-radius discipline (mandatory — `SupportIR` gains a new field shape):
  - Struct-literal sites: `marshal/out.rs:197,264`; `layer_executor.rs:2494,2545`; `tests/executor/live_layer_support_tdd.rs:71`; `tests/executor/layer_executor_tdd.rs:834`; `tests/contract/parity_invariants_selftest_tdd.rs:753`; `SupportIR::default()` at `tests/unit/blackboard_layer_arena_tdd.rs:295` and `crates/slicer-ir/tests/ir_tests.rs:751`.
  - Consumers: `layer_executor.rs:1906-1933`; `visual_debug_render.rs:467-471, 805-809`; `tests/common/mod.rs:288-303`; `tests/common/parity_invariants.rs:733-747`; `tests/contract/dispatch_identity_tdd.rs:241-299`; `tests/executor/live_layer_support_tdd.rs:91-184`; `tests/contract/parity_invariants_selftest_tdd.rs:764-799`.
- Expected sub-agent dispatches: targeted cargo test/check; return `FACT` or <=20 failure lines.
- Context cost: `M`.
- Authoritative docs: ADR-0021, `CONTEXT.md` marshalling terms.
- Verification:
  - `cargo test -p slicer-runtime --test contract --all-targets -- native_support_dispatch_preserves_per_region_origins 2>&1 | rg '^test result'` - FACT pass/fail
  - `cargo test -p slicer-runtime --test contract --all-targets 2>&1 | rg 'test result: ok|0 failed'` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT pass/fail; rebuild without `--check` if `STALE:` is reported
- Exit condition: AC-6 passes; both legs emit per-region `SupportIR`; no empty-origin substitution remains.

### Step 5: Complete response commits and validate dispatch mode

- Task IDs: `TASK-329`
- Objective: preserve all currently supported native response fields (prepass metadata, paint segmentation, slice postprocess) and fail integrated modules without native entries at load time.
- Precondition: Step 4 compiles and the support-origin contract tests pass.
- Postcondition: no supported response silently drops metadata; live bindings have explicit valid mode; external override remains WASM.
- Files allowed to read, with ranges:
  - `crates/slicer-wasm-host/src/marshal/native.rs` - lines 400-980
  - `crates/slicer-wasm-host/src/binding.rs` - lines 1-70
  - `crates/slicer-wasm-host/src/execution_plan_live.rs` - lines 330-375
  - `crates/slicer-sdk/src/prepass_types.rs` - lines 225-274
  - `crates/slicer-scheduler/tests/integration/integrated_tier_tdd.rs` - existing tests only
- Files allowed to edit:
  - `crates/slicer-wasm-host/src/marshal/native.rs`
  - `crates/slicer-sdk/src/prepass_types.rs` (additive reason field only if the native commit's candidate type lacks it)
  - `crates/slicer-wasm-host/src/binding.rs`
  - `crates/slicer-wasm-host/src/execution_plan_live.rs`
  - `crates/slicer-scheduler/tests/integration/integrated_tier_tdd.rs` (author `integrated_manifest_without_native_entry_fails_at_load`)
  - `crates/slicer-runtime/tests/contract/native_dispatch_parity_seam_tdd.rs` (author prepass-commit tests)
- Files explicitly out of bounds: CLI, dist, module registry, WIT.
- Expected sub-agent dispatches: targeted cargo test/check; return `FACT` or <=20 failure lines.
- Context cost: `M`.
- Authoritative docs: ADR-0005, ADR-0056, ADR-0057.
- Verification:
  - `cargo test -p slicer-scheduler --test integration --all-targets 2>&1 | rg 'test result: ok|0 failed'` - FACT pass/fail
  - `cargo test -p slicer-runtime --test contract --all-targets 2>&1 | rg 'test result: ok|0 failed'` - FACT pass/fail
- Exit condition: AC-1 through AC-N1 all have a passing targeted test or static assertion, and no late `MissingComponent` path remains for missing integrated entries.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
|---|---|---|
| 1 | S | inventory only |
| 2 | M | shared view and resolver seam |
| 3 | M | support-origin builder contract (WIT + SDK + macros + host) |
| 4 | M | support-origin IR and marshal (SupportIR + conversion + consumers) |
| 5 | M | response commits and loader blast radius |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.

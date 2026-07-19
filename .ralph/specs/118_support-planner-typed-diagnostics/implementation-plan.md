# Implementation Plan: support-planner-typed-diagnostics

## Execution Rules

- Work one atomic step at a time. Every step maps to retained `TASK-163b-diagnostic`; the source-plan `TASK-253` collision is recorded but never used as a support closure ID.
- The WIT edit is first. After any WIT, SDK, macro, or guest-source edit, run the guest freshness check before the next guest-dependent test.
- The public audit field owns its full struct-literal blast radius in one step. Do not let a later check discover missing literals.
- Do not run Cargo commands in this authoring session. The commands below are implementation-worker dispatch contracts.

## Steps

### Step 1: Confirm ownership boundary and current symbols

- Task IDs: `TASK-163b-diagnostic`
- Objective: verify packet 116 explicitly emits no warning, establish that packet 118 owns code `1003`, and inventory `support-geometry-output`, SDK `SupportGeometryOutput`, `HostExecutionContext`, `PrepassStageRunner`, `WasmRuntimeDispatcher`, `dispatch_prepass_call`, `execute_prepass_with_instrumentation`, `ModuleAccessAudit`, and all current support-planner warning/cap paths.
- Precondition: current tree and authority docs are available.
- Postcondition: exact symbols and all audit literals are recorded; packet 116's draft status does not block this packet because no warning is consumed from it. Shared edits may be serialized `116 -> 118`, but no dependency edge is required.
- Files allowed to read, with ranges when over 300 lines:
  - `.ralph/specs/116_support-modules-doc-honesty-cleanup/packet.spec.md` - metadata only.
  - `docs/07_implementation_status.md` - targeted rows for `TASK-163b-diagnostic` and the `TASK-253` collision.
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` - full file.
  - `crates/slicer-sdk/src/prepass_builders.rs` - `SupportGeometryOutput` block.
  - `crates/slicer-wasm-host/src/host.rs` - `HostExecutionContext`, builder, and `pm::HostSupportGeometryOutput for HostExecutionContext` blocks.
  - `crates/slicer-wasm-host/src/traits.rs` - `PrepassStageRunner` block.
  - `crates/slicer-wasm-host/src/dispatch.rs` - prepass dispatch and runner blocks.
  - `crates/slicer-runtime/src/prepass.rs` - audit construction blocks.
  - `crates/slicer-scheduler/src/validation.rs` - `ModuleAccessAudit` block.
  - `modules/core-modules/support-planner/src/lib.rs` - `SupportPlanner::run_support_geometry`, preserved config-key read, cap, and node regions.
- Files allowed to edit: none.
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`, `target/**`, generated bindgen output, guest lockfiles.
  - Full backlog and full support-planner source.
- Expected sub-agent dispatches:
  - Question: does packet 116 explicitly state that it emits no warning, and does packet 118 own code `1003` without a reverse dependency? Scope: packet-116 metadata plus this packet's ownership wording. Return: `FACT` <= 5 lines.
  - Question: enumerate all `ModuleAccessAudit {` literals and `PrepassStageRunner` implementations. Scope: workspace `*.rs`. Return: `LOCATIONS` <= 20 entries.
  - Question: summarize the canonical WIT add-type rule. Scope: relevant `docs/03_wit_and_manifest.md` section. Return: `SUMMARY` <= 200 words.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0010-typed-diagnostic-channel.md` - direct read.
  - `docs/specs/support-modules-orca-port.md` B4/B7/D10/D11 - direct bounded read.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'No warning is emitted here: packet 118 owns the typed D11 diagnostic channel' .ralph/specs/116_support-modules-doc-honesty-cleanup/packet.spec.md` - FACT pass/fail.
  - The symbol inventory includes every audit literal and the invalid `TASK-253` mapping.
- Exit condition: ownership and edit surfaces are grounded; packet 118 remains draft for its own backlog mapping blocker.

### Step 2: Add WIT diagnostic types and the host mirror

- Task IDs: `TASK-163b-diagnostic`
- Objective: add the exact `diagnostic` record, `severity-level` enum, and `push-diagnostic` method to `support-geometry-output`; add `slicer_ir::Diagnostic` and `DiagnosticSeverity` mirrors and re-export them.
- Precondition: Step 1 completed; no packet-116 implementation or warning path is required for activation.
- Postcondition: AC-1 static evidence passes; host mirror fields are `severity`, `code`, `layer: Option<i32>`, `object_id: Option<String>`, and `message`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/adr/0010-typed-diagnostic-channel.md` - full bounded file.
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` - full file.
  - `crates/slicer-ir/src/stage_io.rs` - stage-type style and re-export boundary.
  - `crates/slicer-ir/src/lib.rs` - public re-export block.
- Files allowed to edit:
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`
  - `crates/slicer-ir/src/stage_io.rs`
  - `crates/slicer-ir/src/lib.rs`
- Files explicitly out of bounds:
  - SDK, WASM host, runtime, scheduler, and support-planner source; those belong to later steps.
  - Generated bindgen output and all guest artifacts.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask build-guests`; scope: all guests. Return: `FACT` pass/fail only.
  - Question: run `cargo xtask build-guests --check` after rebuild. Scope: freshness. Return: `FACT` `up to date` or `STALE: <list>`.
  - Question: run `cargo test -p slicer-runtime --all-targets --test contract -- wit_drift_detection_tdd`. Scope: existing contract target. Return: `FACT` pass/fail and bounded failure `SNIPPETS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0010-typed-diagnostic-channel.md` - exact WIT shape.
  - `CLAUDE.md` - WIT/Type Changes Checklist and Guest WASM Staleness.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask build-guests --check` returns `up to date` after any rebuild.
  - `cargo test -p slicer-runtime --all-targets --test contract -- wit_drift_detection_tdd 2>&1 | tee target/test-output.log` passes.
- Exit condition: WIT and host mirror are durable and artifacts are fresh.

### Step 3: Add the SDK ordered diagnostic builder

- Task IDs: `TASK-163b-diagnostic`
- Objective: expose `Diagnostic`, `DiagnosticSeverity`, `SupportGeometryOutput::push_diagnostic`, and an ordered test accessor in the SDK.
- Precondition: Step 2 WIT and host mirror are complete; guest freshness is clean.
- Postcondition: a guest can construct all five fields and push multiple records without reordering.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/prepass_types.rs` - existing prepass record definitions.
  - `crates/slicer-sdk/src/prepass_builders.rs` - `SupportGeometryOutput` only.
  - `crates/slicer-sdk/src/prelude.rs` - prepass re-export style.
  - `docs/05_module_sdk.md` - delegated SUMMARY only.
- Files allowed to edit:
  - `crates/slicer-sdk/src/prepass_types.rs`
  - `crates/slicer-sdk/src/prepass_builders.rs`
  - `crates/slicer-sdk/src/prelude.rs`
- Files explicitly out of bounds:
  - WASM host, runtime, scheduler, and support-planner source.
  - Generated bindgen output and guest lockfiles.
- Expected sub-agent dispatches:
  - Question: run `cargo check -p slicer-sdk --all-targets`. Scope: SDK. Return: `FACT` pass/fail and first-error `SNIPPETS`.
  - Question: run `cargo xtask build-guests --check`. Scope: guest artifacts after SDK edit. Return: `FACT` `up to date` or `STALE: <list>`.
- Context cost: `S`
- Authoritative docs:
  - `docs/05_module_sdk.md` - delegated SUMMARY.
  - `docs/adr/0010-typed-diagnostic-channel.md` - field semantics.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check -p slicer-sdk --all-targets` passes.
  - `cargo xtask build-guests --check` is clean after any required rebuild.
- Exit condition: the ordered SDK API is available to a macro-authored guest and direct planner tests.

### Step 4: Wire the WASM host and prepass runner side channel

- Task IDs: `TASK-163b-diagnostic`
- Objective: collect WIT diagnostics in `HostExecutionContext`, implement `push_diagnostic` in the `pm::HostSupportGeometryOutput for HostExecutionContext` impl, stash them from `dispatch_prepass_call` in `WasmRuntimeDispatcher`, and expose `PrepassStageRunner::last_diagnostics`.
- Precondition: Step 3 SDK API exists and generated bindings are fresh.
- Postcondition: one `run_stage` call can return its diagnostics through the side channel in FIFO order; no `PrepassStageOutput` shape changes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-wasm-host/src/host.rs` - context, builder, and `pm::HostSupportGeometryOutput for HostExecutionContext` impl.
  - `crates/slicer-wasm-host/src/dispatch.rs` - `dispatch_prepass_call`, `WasmRuntimeDispatcher::run_stage`, and thread-local side-channel pattern.
  - `crates/slicer-wasm-host/src/traits.rs` - `PrepassStageRunner` trait.
- Files allowed to edit:
  - `crates/slicer-wasm-host/src/host.rs`
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-wasm-host/src/traits.rs`
- Files explicitly out of bounds:
  - Runtime audit and scheduler definitions; Step 5 owns their coordinated edit.
  - Support-planner source and tests.
  - Other WIT worlds and host service logging.
- Expected sub-agent dispatches:
  - Question: locate generated `Diagnostic` and `SeverityLevel` bindgen names used by the host. Scope: host prepass binding references. Return: `LOCATIONS` <= 10 entries.
  - Question: run `cargo check -p slicer-wasm-host --all-targets`. Scope: host crate. Return: `FACT` pass/fail and first-error `SNIPPETS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY.
  - `docs/adr/0010-typed-diagnostic-channel.md` - conversion semantics.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check -p slicer-wasm-host --all-targets` passes.
  - The diagnostic stash is drained exactly once after the prepass WIT call.
- Exit condition: host context and runner side channel compile and preserve fields/order.

### Step 5: Extend the audit and add the dedicated diagnostic guest

- Task IDs: `TASK-163b-diagnostic`
- Objective: add `diagnostics: Vec<slicer_ir::Diagnostic>` to `ModuleAccessAudit`, update every literal in the symbol inventory, attach the vector in both prepass audit branches, and author the separate macro guest source.
- Precondition: Step 4 side channel compiles; Step 1 inventory is complete.
- Postcondition: all audit constructors initialize diagnostics; the new guest can emit a deterministic diagnostic through `SupportGeometryOutput`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/src/validation.rs` - `ModuleAccessAudit`.
  - Every `ModuleAccessAudit {` location returned by Step 1.
  - `crates/slicer-runtime/src/prepass.rs` - both audit constructors.
  - `crates/slicer-wasm-host/test-guests/sdk-prepass-guest/src/lib.rs` - macro guest pattern only.
  - `crates/slicer-wasm-host/test-guests/sdk-prepass-guest/Cargo.toml` - dependency pattern only.
- Files allowed to edit:
  - `crates/slicer-scheduler/src/validation.rs`
  - `crates/slicer-scheduler/tests/contract/core_module_ir_access_contract_tdd.rs`
  - `crates/slicer-runtime/src/layer_executor.rs`
  - `crates/slicer-runtime/src/prepass.rs`
  - `crates/slicer-runtime/src/postpass.rs`
  - `crates/slicer-runtime/tests/unit/dag_validation_tdd.rs`
  - `crates/slicer-runtime/tests/e2e/acceptance_gate_gaps_tdd.rs`
  - `crates/slicer-wasm-host/test-guests/sdk-support-diagnostic-guest/Cargo.toml`
  - `crates/slicer-wasm-host/test-guests/sdk-support-diagnostic-guest/src/lib.rs`
- Files explicitly out of bounds:
  - No `ModuleAccessAudit` literal from the Step 1 inventory is out of bounds. This step intentionally exceeds the soft three-file edit limit because the public struct-field blast radius must land atomically.
  - Generated component, guest lockfile, and all other guest sources.
  - Support-planner source.
- Expected sub-agent dispatches:
  - Question: enumerate the remaining audit literals after the three primary edits. Scope: workspace `*.rs`. Return: `LOCATIONS` <= 20 entries.
  - Question: run `cargo check --workspace --all-targets`. Scope: workspace compile. Return: `FACT` pass/fail and first-error `SNIPPETS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0010-typed-diagnostic-channel.md` - audit semantics.
  - `CLAUDE.md` - guest staleness.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check --workspace --all-targets` passes after all literal fallout is edited.
  - `cargo xtask build-guests --check` is clean after rebuilding the new guest.
- Exit condition: public audit and diagnostic guest are compile-ready.

### Step 6: Register and run the real host round-trip tests

- Task IDs: `TASK-163b-diagnostic`
- Objective: add `prepass_diagnostic_roundtrip_tdd` to the existing `crates/slicer-runtime/tests/integration/main.rs` aggregate and assert exact fields, FIFO order, and code `99` acceptance through `execute_prepass_with_instrumentation` -> `WasmRuntimeDispatcher::run_stage` -> `dispatch_prepass_call`, with the result drained by `PrepassStageRunner::last_diagnostics`.
- Precondition: Step 5 audit and guest are compile-ready; generated component is fresh.
- Postcondition: AC-3 and AC-N2 pass through the actual `integration` test target; planner-owned AC-6 and AC-N3 are covered by Step 7's direct module test.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/main.rs` - aggregate registration.
  - `crates/slicer-runtime/tests/integration/support_geometry_config_normalization_tdd.rs` - existing support-geometry host-dispatch setup pattern.
  - `crates/slicer-runtime/tests/integration/pipeline_tdd.rs` - runtime test setup pattern.
  - `crates/slicer-runtime/tests/common/wasm_cache.rs` - `compiled_guest` path helper.
  - `crates/slicer-runtime/tests/common/dispatch_fixture.rs` - prepass dispatch pattern.
- Files allowed to edit:
  - `crates/slicer-runtime/tests/integration/main.rs`
  - `crates/slicer-runtime/tests/integration/prepass_diagnostic_roundtrip_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/**` and host source; Step 5 closed the API.
  - Support-planner source and tests; Step 7 owns them.
- Expected sub-agent dispatches:
  - Question: run `cargo test -p slicer-runtime --all-targets --test integration -- prepass_diagnostic_roundtrip_tdd`. Scope: this module. Return: `FACT` pass/fail and bounded failure `SNIPPETS`.
  - Question: run `cargo xtask build-guests --check` immediately before the test. Scope: guest artifacts. Return: `FACT` `up to date` or `STALE: <list>`.
- Context cost: `S`
- Authoritative docs:
  - `docs/01_system_architecture.md` `PrePass::SupportGeometry` section.
  - Canonical `world-prepass.wit`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --all-targets --test integration -- prepass_diagnostic_roundtrip_tdd 2>&1 | tee target/test-output.log` passes.
- Exit condition: real guest-to-host audit round-trip is green through the existing aggregate target.

### Step 7: Migrate support-planner and test cap accounting

- Task IDs: `TASK-163b-diagnostic`
- Objective: replace node-clamped logging with code 1002, count all current cap drops, emit one code 1001 diagnostic per affected layer, and have `SupportPlanner::run_support_geometry` read the preserved config key and emit planner-owned code 1003 exactly once.
- Precondition: Step 6 round-trip is green; current source paths are re-read. Packet 116's implementation status is not a prerequisite because it emits no warning.
- Postcondition: AC-4, AC-5, AC-6, AC-N1, AC-N3, and AC-7 pass; support-planner fatal behavior is unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - `SupportPlanner::run_support_geometry`, node warning, preserved config-key read, and all cap enforcement regions.
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` - node-clamped test.
  - `crates/slicer-sdk/src/prepass_builders.rs` - diagnostic accessor.
- Files allowed to edit:
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs`
  - `modules/core-modules/support-planner/tests/diagnostics_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/support-planner/support-planner.toml` and dead-field cleanup; packet 116 owns them.
  - Other core modules and unrelated log callsites.
- Expected sub-agent dispatches:
  - Question: run the direct diagnostics target. Scope: `cargo test -p support-planner --all-targets --test diagnostics_tdd`. Return: `FACT` pass/fail and <= 30 failure lines.
  - Question: run the existing node-clamped test. Scope: `cargo test -p support-planner --all-targets --test orca_parity_tdd -- node_dropped_when_avoidance_rejects_all_moves`. Return: `FACT` pass/fail.
  - Question: run `cargo xtask build-guests --check` after support-planner source edit. Scope: guest freshness. Return: `FACT` `up to date` or `STALE: <list>`.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` B4/B7/D10/D11.
  - `docs/adr/0010-typed-diagnostic-channel.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p support-planner --all-targets --test diagnostics_tdd 2>&1 | tee target/test-output.log` passes.
  - `cargo test -p support-planner --all-targets --test orca_parity_tdd -- node_dropped_when_avoidance_rejects_all_moves 2>&1 | tee target/test-output.log` passes.
  - `cargo xtask build-guests --check` is clean.
- Exit condition: all current support warning paths, including the planner-owned code `1003` path, are typed, bounded, and covered by actual drivers.

### Step 8: Update contract documentation

- Task IDs: `TASK-163b-diagnostic`
- Objective: document `ModuleAccessAudit.diagnostics`, the support WIT method, the SDK method, FIFO semantics, code convention, and guest-rebuild obligation.
- Precondition: Steps 2-7 are green and final symbol names are stable.
- Postcondition: every Doc Impact grep in `packet.spec.md` passes without claiming a `SupportPlanIR` change or generic all-prepass method.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - audit section location only.
  - `docs/03_wit_and_manifest.md` - world-prepass section location only.
  - `docs/05_module_sdk.md` - prepass builder section location only.
- Files allowed to edit:
  - `docs/02_ir_schemas.md`
  - `docs/03_wit_and_manifest.md`
  - `docs/05_module_sdk.md`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` - closure worker owns status; do not change the colliding `TASK-253` row.
  - All unrelated docs.
- Expected sub-agent dispatches:
  - Question: locate exact insertion sections. Scope: the three named docs. Return: `LOCATIONS` <= 10 entries.
  - Question: run the three Doc Impact greps. Scope: the three named docs. Return: `FACT` pass/fail.
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0010-typed-diagnostic-channel.md`.
  - Canonical `world-prepass.wit`.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'ModuleAccessAudit.*diagnostics' docs/02_ir_schemas.md && rg -q 'push-diagnostic' docs/03_wit_and_manifest.md && rg -q 'SupportGeometryOutput::push_diagnostic' docs/05_module_sdk.md` passes.
- Exit condition: documentation matches the landed names and fields.

### Step 9: Run narrow closure gates

- Task IDs: `TASK-163b-diagnostic`
- Objective: re-dispatch every AC command, freshness check, workspace check, and clippy gate; preserve draft status while either blocker remains unresolved.
- Precondition: Steps 1-8 complete and all current implementation tests pass.
- Postcondition: bounded PASS/FAIL evidence exists for every AC and packet gate; no unsupported status transition is made.
- Files allowed to read, with ranges when over 300 lines:
  - Packet 118 artifacts only.
  - `target/test-output.log` through targeted Grep/Read after each test command.
- Files allowed to edit: none.
- Files explicitly out of bounds:
  - Implementation files, other packet directories, and `target/**` except delegated test output.
- Expected sub-agent dispatches:
  - Question: run all packet AC commands in order. Scope: packet 118 `packet.spec.md`. Return: `FACT` PASS/FAIL list.
  - Question: run `cargo xtask build-guests --check`. Scope: guest artifacts. Return: `FACT` `up to date` or `STALE: <list>`.
  - Question: run `cargo clippy --workspace --all-targets -- -D warnings`. Scope: workspace. Return: `FACT` pass/fail with first-error snippets.
- Context cost: `S`
- Authoritative docs: none additional.
- OrcaSlicer refs: none.
- Verification:
  - Full current AC matrix passes.
  - Freshness, check, and clippy gates pass.
- Exit condition: closure evidence is recorded; `status: implemented` remains prohibited while the packet's backlog-mapping blocker remains unresolved. Packet 116's draft status is not a closure blocker for this packet's typed diagnostic.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Dependency and symbol inventory. |
| Step 2 | M | WIT record/enum and host mirror; guest freshness. |
| Step 3 | S | SDK type and ordered builder. |
| Step 4 | M | WASM host conversion and runner side channel. |
| Step 5 | M | Audit field blast radius and diagnostic guest. |
| Step 6 | S | Existing integration target round-trip. |
| Step 7 | M | Planner warnings, cap accounting, and direct tests. |
| Step 8 | S | Three documentation updates. |
| Step 9 | S | Narrow closure gates. |

Aggregate: `M`. No step is `L`.

## Packet Completion Gate

- All nine steps and exits complete.
- AC-1 through AC-7 and AC-N1 through AC-N3 pass.
- All three Doc Impact greps pass.
- `docs/07_implementation_status.md` closes only `TASK-163b-diagnostic` through a worker dispatch; the colliding `TASK-253` row is not changed by this packet.
- `cargo xtask build-guests --check` returns `up to date`.
- The packet's `[BLOCK]` backlog-mapping question in `design.md` is resolved before status changes to `implemented`; the ownership decision records that packet 116 emits no warning and is not a prerequisite.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm guest freshness immediately before guest-dependent tests.
- Confirm the existing `contract` and `integration` aggregate targets, not nonexistent standalone test binaries, were used.
- Confirm the packet's support-owned backlog mapping is resolved before any implemented transition; confirm packet 116 is referenced only as the no-warning/dead-state boundary, not as a warning provider.

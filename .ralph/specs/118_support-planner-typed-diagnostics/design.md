# Design: support-planner-typed-diagnostics

## Controlling Code Paths

- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit::support-geometry-output` owns the canonical WIT method.
- `crates/slicer-ir/src/stage_io.rs::{Diagnostic, DiagnosticSeverity}` owns the host-side mirror used across runtime, scheduler, and WASM host.
- `crates/slicer-sdk/src/prepass_types.rs::{Diagnostic, DiagnosticSeverity}` and `prepass_builders.rs::SupportGeometryOutput` own the guest API and ordered collection.
- `crates/slicer-wasm-host/src/host.rs::{HostExecutionContext, pm::HostSupportGeometryOutput for HostExecutionContext}` convert and collect WIT values.
- `crates/slicer-wasm-host/src/traits.rs::PrepassStageRunner`, `crates/slicer-wasm-host/src/dispatch.rs::{WasmRuntimeDispatcher::run_stage, dispatch_prepass_call}`, and `crates/slicer-runtime/src/prepass.rs::execute_prepass_with_instrumentation` carry the side channel into audits.
- `crates/slicer-scheduler/src/validation.rs::ModuleAccessAudit` owns `diagnostics: Vec<slicer_ir::Diagnostic>`.
- `modules/core-modules/support-planner/src/lib.rs::SupportPlanner::run_support_geometry` owns the node, cap, and config-driven `support_interface_bottom_layers` diagnostic paths.
- Tests use the existing `contract` and `integration` aggregate targets plus the support-planner test targets: `crates/slicer-runtime/tests/contract/wit_drift_detection_tdd.rs`, `crates/slicer-runtime/tests/integration/main.rs`, `modules/core-modules/support-planner/tests/orca_parity_tdd.rs`, and the new `modules/core-modules/support-planner/tests/diagnostics_tdd.rs`. There is no standalone `support_planner_diagnostic_emission_tdd` binary.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- The WIT addition is additive and scoped to `support-geometry-output`; other prepass export signatures remain unchanged.
- `Diagnostic` is a record with an open `u32` code space, not a WIT variant. The host does not enforce the support-planner range `1000-1999`.
- `layer` is signed `option<s32>` and maps to Rust `Option<i32>`; negative future raft indices must remain representable.
- Diagnostics are recoverable metadata. `ModuleError::fatal` remains the only abort path for unrecoverable module errors.
- `SupportPlanIR` and its schema version do not change. `PrepassStageOutput` does not change; `PrepassStageRunner::last_diagnostics` transports side-channel values.
- Every audit constructor, including layer/postpass and test-only literals, initializes an empty diagnostic vector. Scheduler validation ignores this vector.

## Code Change Surface

- Selected approach:
  - Add `Diagnostic` and `severity-level` to the canonical WIT and `push-diagnostic` only to `support-geometry-output`.
  - Add host and SDK mirrors with explicit field conversion and FIFO vectors.
  - Reuse the existing runner log side-channel shape for `last_diagnostics` instead of changing `PrepassStageOutput`.
  - Keep cap `continue`/`truncate` behavior and add one post-collection diagnostic per affected global layer.
  - Read the preserved `support_interface_bottom_layers` key in `SupportPlanner::run_support_geometry` and emit code `1003` once before its layer loop. This keeps the typed D11 implementation in packet 118 and avoids any packet-116 warning dependency or planner field.
- Exact functions and types:
  - `world-prepass.wit::{diagnostic, severity-level, support-geometry-output.push-diagnostic}`.
  - `slicer_ir::stage_io::{Diagnostic, DiagnosticSeverity}` and `slicer-ir/src/lib.rs` re-exports.
  - `slicer_sdk::prepass_types::{Diagnostic, DiagnosticSeverity}` and `SupportGeometryOutput::{push_diagnostic, diagnostics}`.
  - `HostExecutionContext::{diagnostics, diagnostics_mut}`, the `push_diagnostic` method in the `pm::HostSupportGeometryOutput for HostExecutionContext` impl, `PrepassStageRunner::last_diagnostics`, and the dispatcher stash.
  - `ModuleAccessAudit::diagnostics`, both prepass audit constructors, and all existing `ModuleAccessAudit` literals.
  - `SupportPlanner::run_support_geometry` codes `1001`, `1002`, and planner-owned code `1003` read from the preserved config key.
- Rejected alternatives:
  - A generic diagnostic parameter on all four prepass exports: rejected because current WIT has stage-specific output resources and this slice only needs support geometry.
  - A new `PrepassStageOutput` variant: rejected because it would churn every scripted runner and match site for audit metadata.
  - A shared Rust diagnostic library used by guest modules: rejected across the Rust/WIT module boundary; the SDK type is the guest API.
  - Workspace-wide log migration: rejected by ADR-0010 and outside this packet.

## Files in Scope (read + edit)

- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` - canonical WIT record, enum, and support-output method.
- `crates/slicer-ir/src/stage_io.rs` and `crates/slicer-ir/src/lib.rs` - host mirror and re-export.
- `crates/slicer-sdk/src/prepass_types.rs`, `prepass_builders.rs`, and `prelude.rs` - SDK type and ordered builder API.
- `crates/slicer-wasm-host/src/host.rs`, `dispatch.rs`, and `traits.rs` - WIT conversion, collection, prepass dispatch, and runner drain.
- `crates/slicer-scheduler/src/validation.rs` - public audit field; all existing audit literals are part of its blast radius.
- `crates/slicer-runtime/src/prepass.rs` - attach drained diagnostics to prepass audits.
- `modules/core-modules/support-planner/src/lib.rs` - node warning, cap diagnostics, and config-driven code `1003` emission in `run_support_geometry`.
- `crates/slicer-wasm-host/test-guests/sdk-support-diagnostic-guest/{Cargo.toml,src/lib.rs}` - new macro-authored diagnostic guest.
- `crates/slicer-runtime/tests/integration/{main.rs,prepass_diagnostic_roundtrip_tdd.rs}` - actual runtime WIT round-trip driver.
- `modules/core-modules/support-planner/tests/{orca_parity_tdd.rs,diagnostics_tdd.rs}` - direct module drivers.
- `docs/{02_ir_schemas.md,03_wit_and_manifest.md,05_module_sdk.md}` - same-packet contract documentation.

## Read-Only Context

- `docs/adr/0010-typed-diagnostic-channel.md` - full bounded read for exact fields and code convention.
- `docs/specs/support-modules-orca-port.md` - B4/B7/D10/D11 only.
- `docs/01_system_architecture.md` - `PrePass::SupportGeometry` section only.
- `docs/03_wit_and_manifest.md` - delegated SUMMARY only.
- `docs/05_module_sdk.md` - delegated SUMMARY only.
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` - full 150-line file.
- `crates/slicer-sdk/src/prepass_builders.rs` - `SupportGeometryOutput` block only.
- `crates/slicer-wasm-host/src/host.rs` - `HostExecutionContext`, builder, and `pm::HostSupportGeometryOutput for HostExecutionContext` blocks only.
- `crates/slicer-wasm-host/src/dispatch.rs` - `dispatch_prepass_call` and prepass runner blocks only.
- `crates/slicer-runtime/src/prepass.rs` - audit construction blocks only.
- `crates/slicer-scheduler/src/validation.rs` - `ModuleAccessAudit` block only.
- `modules/core-modules/support-planner/src/lib.rs` - `SupportPlanner::run_support_geometry` and cap/node regions only; do not restore the packet-116 dead field.
- `docs/07_implementation_status.md` - targeted rows only; do not read the backlog end-to-end.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - no Orca behavior is ported.
- `target/`, generated bindgen output, and guest lockfiles - never load directly.
- Guest sources outside `crates/slicer-wasm-host/test-guests/sdk-support-diagnostic-guest/` - do not browse or edit.
- `crates/slicer-ir/src/slice_ir.rs` - no `SupportPlanIR` change.
- `docs/07_implementation_status.md` - no direct edit in this packet; closure status is a worker responsibility.
- Unrelated module log callsites and unrelated scheduler behavior.

## Expected Sub-Agent Dispatches

- Question: does packet 116 explicitly emit no warning, and does packet 118 own code `1003` without a reverse dependency? Scope: packet-116 metadata and this packet's ownership wording. Return: `FACT` <= 5 lines.
- Question: enumerate every `ModuleAccessAudit {` literal and `PrepassStageRunner` implementation. Scope: workspace `*.rs`, exact symbols only. Return: `LOCATIONS` <= 20 entries.
- Question: summarize how a new WIT record/enum is added under canonical world dependencies. Scope: `docs/03_wit_and_manifest.md` relevant section. Return: `SUMMARY` <= 200 words.
- Question: summarize prepass output-builder authoring rules. Scope: `docs/05_module_sdk.md` relevant section. Return: `SUMMARY` <= 200 words.
- Question: run `cargo xtask build-guests --check`. Scope: guest artifacts. Return: `FACT` `up to date` or `STALE: <list>`.
- Question: run the targeted contract, integration, direct planner, workspace check, and clippy commands from `requirements.md`. Scope: each command separately. Return: `FACT` pass/fail and bounded failure `SNIPPETS`.

## Data and Contract Notes

- WIT contract: `diagnostic` has exactly five fields; `severity-level` has exactly five variants; `push-diagnostic` returns `result<_, string>`.
- Host mirror: `DiagnosticSeverity::{Trace, Debug, Info, Warn, Error}` maps one-to-one to WIT variants. `object-id` maps to `object_id`.
- Audit contract: `ModuleAccessAudit.diagnostics` is FIFO and does not participate in scheduler read/write validation.
- Support planner codes: `1001` cap, `1002` node-clamped, `1003` interface-bottom-layers. The host accepts code `99` in the round-trip test.
- No manifest, IR schema version, scheduler stage ordering, or coordinate conversion changes are introduced.

## Locked Assumptions and Invariants

- `Diagnostic` never aborts a run.
- The support cap's configured limit and truncation behavior are unchanged.
- One cap diagnostic is emitted per affected global layer, not once per dropped candidate.
- The host audit preserves guest emission order.
- Existing guests that do not call `push-diagnostic` continue to execute after the guest rebuild.
- Packet 116 remains the owner of dead-field cleanup and emits no warning; packet 118 owns the typed `support_interface_bottom_layers` diagnostic and never adds packet-116 state or a string-warning predecessor.

## Risks and Tradeoffs

- WIT bindgen changes can break every guest artifact. Mitigation: rebuild and freshness-check before every guest-dependent test.
- `ModuleAccessAudit` is shared by scheduler, runtime, and tests. Mitigation: inventory and update every literal in the same implementation step.
- `run_support_geometry` owns the output builder while `on_print_start` does not. Mitigation: read the preserved config key at the start of `run_support_geometry`, emit once before the layer loop, and test both configured and default/absent cases.
- The diagnostic method is limited to `support-geometry-output` rather than generalized to every prepass stage. This minimizes WIT signature churn and matches the current support warning surface.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (host drain plus audit blast radius, or planner cap fixture).
- Highest-risk dispatch and required return format: `cargo xtask build-guests --check` -> `FACT` only, `up to date` or `STALE: <list>`.

## Open Questions

- `[BLOCK]` Source-plan `TASK-253` is a current paint-segmentation task, not a support task. A maintainer must map the B4 cap slice to a support-owned backlog row before closure; this packet proposes no replacement ID.
- `[DECISION]` Packet 116 is not a prerequisite for D11: it explicitly emits no warning, while packet 118 reads the preserved config key in `SupportPlanner::run_support_geometry` and owns the typed code `1003` diagnostic. Shared-file edits may be serialized `116 -> 118`, but no dependency edge is required and no cycle exists.

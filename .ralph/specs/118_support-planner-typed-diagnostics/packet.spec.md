---
status: implemented
packet: 118
task_ids:
  - TASK-163b-diagnostic
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-planner-typed-diagnostics

## Goal

Add the typed prepass diagnostic contract to the existing `SupportGeometryOutput` path, preserve emitted order in `ModuleAccessAudit.diagnostics`, and have support-planner own all three typed warning paths without changing fatal-error behavior. The `support_interface_bottom_layers` record is created here from the preserved config key; packet 116 emits no warning.

## Scope Boundaries

Touches the canonical prepass WIT, shared host diagnostic and audit values, the SDK support-geometry builder, the WASM host drain path, support-planner warning paths, and bounded tests. The support cap remains bounded by its configured limit; only silent drops become one typed record per affected layer. Routine `host-services.log` calls and `ModuleError` fatal errors remain unchanged. Packet 116's dead-state cleanup remains outside this packet; its preserved config key is read by `SupportPlanner::run_support_geometry`, which owns the code `1003` record. Source-plan `TASK-253` is not retained because current `docs/07_implementation_status.md` assigns it to paint-segmentation.

## Prerequisites and Blockers

- Depends on: no packet-116 warning implementation. Packet 116 explicitly emits no warning; packet 118 reads its preserved config key in `SupportPlanner::run_support_geometry` and owns the typed code `1003` record. If both packets edit the shared planner source, workers must serialize the file edits as `116 -> 118`, but that is an ordering note rather than a semantic dependency. Packet 116 does not depend on packet 118, so this creates no cycle. Packet `117_support-planner-geometric-correctness` is adjacent but does not block this channel.
- Unblocks: packet `119_support-validation-wedge-harness` after the typed channel and current support-planner driver are green.
- Activation blockers: the source-plan `TASK-253` mapping remains unresolved; packet 116's draft status is not a diagnostic prerequisite. Do not add a packet-116 warning or revive its removed dead field.

## Acceptance Criteria

- **AC-1. Given** the canonical `world-prepass.wit`, **when** its support output contract is inspected, **then** it contains `record diagnostic { severity: severity-level, code: u32, layer: option<s32>, object-id: option<string>, message: string }`, `enum severity-level { trace, debug, info, warn, error }`, and `push-diagnostic: func(d: diagnostic) -> result<_, string>;` inside `resource support-geometry-output`. | `rg -q 'record diagnostic' crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit && rg -q 'severity: severity-level' crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit && rg -q 'object-id: option<string>' crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit && rg -q 'enum severity-level' crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit && rg -q 'push-diagnostic: func\(d: diagnostic\) -> result<_, string>' crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`
- **AC-2. Given** the typed contract implementation, **when** the workspace is checked, **then** the SDK exposes `SupportGeometryOutput::push_diagnostic`, the runner exposes `PrepassStageRunner::last_diagnostics`, and `ModuleAccessAudit` carries `diagnostics: Vec<Diagnostic>` (fully-qualified as `slicer_ir::Diagnostic` in source) without changing `runtime_reads` or `runtime_writes`. | `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log && rg -q 'fn push_diagnostic' crates/slicer-sdk/src/prepass_builders.rs && rg -q 'fn last_diagnostics' crates/slicer-wasm-host/src/traits.rs && (rg -q 'diagnostics: Vec<Diagnostic>' crates/slicer-scheduler/src/validation.rs || rg -q 'pub diagnostics: Vec<slicer_ir::Diagnostic>' crates/slicer-scheduler/src/validation.rs)`
- **AC-3. Given** the diagnostic round-trip guest emits `Diagnostic { severity: Warn, code: 99, layer: Some(-1), object_id: Some("cube"), message: "round-trip" }`, **when** the real support-geometry WIT call completes, **then** the prepass audit contains exactly one record with those five field values in emission order. | `cargo test -p slicer-runtime --all-targets --test integration -- prepass_diagnostic_roundtrip_tdd::support_geometry_diagnostic_round_trips --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** the existing avoidance fixture whose clamped move target is inside `collision_polys`, **when** `node_dropped_when_avoidance_rejects_all_moves` runs, **then** the output diagnostics contain a warning with `code == 1002` and a message containing `node-clamped-out`, without replacing the recoverable event with a fatal `ModuleError`. | `cargo test -p support-planner --all-targets --test orca_parity_tdd -- node_dropped_when_avoidance_rejects_all_moves --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** `support_max_branches_per_layer = 1024` and more than 1024 candidates on one layer, **when** the planner completes, **then** exactly one diagnostic for that layer has warning severity, `code == 1001`, a message containing `max_branches_per_layer cap exceeded`, `dropped_count` greater than zero, and `kept_count=1024`. | `cargo test -p support-planner --all-targets --test diagnostics_tdd -- cap_exceeded_emits_one_diagnostic_per_layer --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the preserved `support_interface_bottom_layers = 3` config key, **when** `SupportPlanner::run_support_geometry` receives its `SupportGeometryOutput`, **then** it emits exactly one warning with `code == 1003`, `layer == None`, and a message containing `support_interface_bottom_layers is not yet implemented`; no duplicate appears while later support layers are processed. | `cargo test -p support-planner --all-targets --test diagnostics_tdd -- interface_bottom_layers_emits_one_typed_diagnostic --nocapture 2>&1 | tee target/test-output.log`
- **AC-7. Given** the migrated support-planner source, **when** the legacy warning prefixes are searched, **then** none of `support-planner.node-clamped-out:`, `support-planner: max_branches_per_layer cap exceeded`, or `support-planner: support_interface_bottom_layers is not yet implemented` occurs. | `! rg -q 'support-planner\.node-clamped-out:' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'support-planner: max_branches_per_layer' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'support-planner: support_interface_bottom_layers' modules/core-modules/support-planner/src/lib.rs`

## Negative Test Cases

- **AC-N1. Given** every layer stays below `support_max_branches_per_layer = 1024`, **when** the planner completes, **then** zero diagnostics contain `max_branches_per_layer cap exceeded`. | `cargo test -p support-planner --all-targets --test diagnostics_tdd -- below_cap_emits_no_cap_diagnostic --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** the round-trip guest emits `code = 99` outside the support-planner allocation convention, **when** prepass dispatch completes, **then** dispatch succeeds and the audit preserves `code == 99`; the host does not enforce a code range. | `cargo test -p slicer-runtime --all-targets --test integration -- prepass_diagnostic_roundtrip_tdd::out_of_range_code_is_captured --nocapture 2>&1 | tee target/test-output.log`
- **AC-N3. Given** `support_interface_bottom_layers = -1` or no key, **when** `SupportPlanner::run_support_geometry` receives its `SupportGeometryOutput`, **then** zero diagnostics contain `support_interface_bottom_layers is not yet implemented`. | `cargo test -p support-planner --all-targets --test diagnostics_tdd -- interface_bottom_layers_default_emits_no_typed_diagnostic --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo test -p slicer-runtime --all-targets --test integration -- prepass_diagnostic_roundtrip_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p support-planner --all-targets --test diagnostics_tdd 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` sections B4, B7, D10, and D11 - source-plan behavior and codes; direct bounded reads.
- `docs/adr/0010-typed-diagnostic-channel.md` - WIT fields, signed layer, code convention, and recoverable-error rationale; direct read.
- `docs/03_wit_and_manifest.md` - delegated SUMMARY for canonical WIT and bindgen rules.
- `docs/05_module_sdk.md` - delegated SUMMARY for prepass output builders.
- `docs/01_system_architecture.md` - `PrePass::SupportGeometry` contract; direct bounded read.
- `docs/07_implementation_status.md` - targeted lookup of `TASK-163b-diagnostic`, the excluded `TASK-253` collision, and packet-118 status.
- `CLAUDE.md` - WIT/Type Changes Checklist and Guest WASM Staleness guidance.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` section documenting `ModuleAccessAudit.diagnostics` and the host-side diagnostic mirror. Verification: `rg -q 'ModuleAccessAudit.*diagnostics' docs/02_ir_schemas.md && rg -q 'DiagnosticSeverity' docs/02_ir_schemas.md`.
- `docs/03_wit_and_manifest.md` section for `world-prepass` output resources. Verification: `rg -q 'push-diagnostic' docs/03_wit_and_manifest.md && rg -q 'severity-level' docs/03_wit_and_manifest.md`.
- `docs/05_module_sdk.md` prepass output-builder section. Verification: `rg -q 'SupportGeometryOutput::push_diagnostic' docs/05_module_sdk.md`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

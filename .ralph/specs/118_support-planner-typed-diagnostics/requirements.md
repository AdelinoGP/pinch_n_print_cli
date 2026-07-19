# Requirements: support-planner-typed-diagnostics

## Packet Metadata

- Grouped task IDs:
  - `TASK-163b-diagnostic` - typed `Diagnostic` channel on `world-prepass`, including the support-planner node, cap, and not-implemented warning paths named by the current backlog row.
- Removed source-plan ID: `TASK-253` - current `docs/07_implementation_status.md` assigns it to paint-segmentation shell-depth propagation, not support diagnostics.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The current tree has one support-planner diagnostic callsite. `SupportPlanner::run_support_geometry` emits `node-clamped-out` through `slicer_sdk::host::log`, while the cap paths silently discard candidates. Packet 116 explicitly removes the dead `support_interface_bottom_layers` state and emits no warning. This packet therefore reads the preserved key in `SupportPlanner::run_support_geometry` and owns the typed code `1003` record itself; there is no packet-116 warning migration or dependency-gated diagnostic path.

The packet adds a typed diagnostic channel to the existing `support-geometry-output` WIT resource, plumbs it through the SDK builder and `HostExecutionContext`, drains it from the `WasmRuntimeDispatcher` through `PrepassStageRunner::last_diagnostics`, and records it in `ModuleAccessAudit.diagnostics`. The cap keeps its configured limit and data-flow truncation, adding one diagnostic per affected layer. The support planner reads `support_interface_bottom_layers` at the start of `run_support_geometry` and emits one code `1003` record before processing layers when the value is not `-1`; no packet-116 warning is consumed or recreated.

The source-plan `TASK-253` label is removed from this packet because it collides with current paint work. The support cap remains part of the retained `TASK-163b-diagnostic` support slice, and no replacement task ID is invented.

## In Scope

- Add the following exact WIT record and enum to `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`, plus `push-diagnostic: func(d: diagnostic) -> result<_, string>;` inside `resource support-geometry-output`:

  ```wit
  record diagnostic {
      severity: severity-level,
      code: u32,
      layer: option<s32>,
      object-id: option<string>,
      message: string,
  }

  enum severity-level {
      trace,
      debug,
      info,
      warn,
      error,
  }
  ```
- Add `slicer_ir::Diagnostic` and `slicer_ir::DiagnosticSeverity` with `severity`, `code`, `layer: Option<i32>`, `object_id: Option<String>`, and `message` fields.
- Add SDK `Diagnostic`, `DiagnosticSeverity`, `SupportGeometryOutput::push_diagnostic`, and an ordered diagnostic accessor.
- Add `HostExecutionContext.diagnostics`, WIT-to-host conversion in the `pm::HostSupportGeometryOutput for HostExecutionContext` implementation in `crates/slicer-wasm-host/src/host.rs`, `PrepassStageRunner::last_diagnostics` in `crates/slicer-wasm-host/src/traits.rs`, and the `WasmRuntimeDispatcher` stash/drain in `crates/slicer-wasm-host/src/dispatch.rs`.
- Add `diagnostics: Vec<slicer_ir::Diagnostic>` to `slicer_scheduler::validation::ModuleAccessAudit`, update every existing struct literal, and attach the drained vector to both prepass audit constructor branches in `crates/slicer-runtime/src/prepass.rs`.
- Replace the current `node-clamped-out` string log with code `1002`, warning severity, the current layer, object ID, and a message containing `node-clamped-out`; the existing fixture is `modules/core-modules/support-planner/tests/orca_parity_tdd.rs::node_dropped_when_avoidance_rejects_all_moves`.
- Count drops at every current `support-planner` cap enforcement site and emit one code `1001` warning per affected global layer. The message must contain `max_branches_per_layer cap exceeded`, `dropped_count=<n>`, and `kept_count=<configured cap>`.
- In `SupportPlanner::run_support_geometry`, read the preserved `support_interface_bottom_layers` config key and emit one code `1003` warning through the current `SupportGeometryOutput` when the value is not `-1`. Emit it before the layer loop so it is not duplicated per layer; do not add a field, parse-and-store branch, or packet-116 warning.
- Add `crates/slicer-wasm-host/test-guests/sdk-support-diagnostic-guest/{Cargo.toml,src/lib.rs}` as a separate macro-authored prepass guest. Do not add a second stage implementation to `sdk-prepass-guest`, whose current macro fixture is a MeshAnalysis guest.
- Add `crates/slicer-runtime/tests/integration/prepass_diagnostic_roundtrip_tdd.rs` and register it in the existing `crates/slicer-runtime/tests/integration/main.rs` aggregate. It must drive the real `integration` target and assert exact fields, FIFO order, and code `99` acceptance through the WIT host path; support-planner's code `1003` ownership is tested directly in `diagnostics_tdd.rs`.
- Update `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` to inspect typed output for the existing node-clamped fixture rather than `host-services.log` capture.
- Add `modules/core-modules/support-planner/tests/diagnostics_tdd.rs` for the cap positive/negative cases and the planner-owned `support_interface_bottom_layers` warning/default cases.
- Update `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, and `docs/05_module_sdk.md` with the typed contract and audit surface.

## Out of Scope

- Migrating every workspace `log` call. Routine trace/debug logging remains on `host-services.log`.
- A generic diagnostic parameter on every prepass export. This packet uses the current support-geometry output resource and does not change the other three prepass export signatures.
- A new `PrepassStageOutput` variant or a `SupportPlanIR` field. Diagnostics are audit metadata, not support-plan geometry.
- A central code registry. The support-planner range `1000-1999` remains a convention and is not host-enforced.
- Diagnostic emission from `tree-support`, `traditional-support`, raft, or infill modules.
- GUI/report rendering of the new records. This packet exposes per-stage audit data only.
- Changing the support cap, making a new cap key, or changing fatal error handling.
- Reimplementing packet 116's dead-field cleanup. Packet 116 owns removal of the Rust field and parse-and-store branch, while this packet owns the typed D11 read/emission against the preserved config key. Packet 116's explicit no-warning boundary is not a prerequisite.

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` sections B4, B7, D10, and D11 - direct bounded read for behavior and codes.
- `docs/adr/0010-typed-diagnostic-channel.md` - direct read for WIT shape, signed layer, code convention, and recoverable-error rationale.
- `docs/01_system_architecture.md` `PrePass::SupportGeometry` section - direct bounded read for stage ordering.
- `docs/03_wit_and_manifest.md` - delegated SUMMARY for canonical WIT and bindgen rules.
- `docs/05_module_sdk.md` - delegated SUMMARY for prepass output builders.
- `docs/07_implementation_status.md` - targeted lookup for `TASK-163b-diagnostic`, colliding `TASK-253`, and packet status.
- `CLAUDE.md` WIT/Type Changes Checklist and Guest WASM Staleness sections.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-7`.
- Negative: `AC-N1` through `AC-N3`.
- Cross-packet impact: packet 119 may consume the typed diagnostic audit, but it must not assume a diagnostic field on `SupportPlanIR`. Packet 116 owns only the dead-state cleanup; packet 118 owns the typed `support_interface_bottom_layers` diagnostic and packet 116 emits no warning.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the three closure gates.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | Confirm all guest artifacts are current after WIT, SDK, and diagnostic-guest edits. | FACT `up to date` or `STALE: <list>` |
| `cargo check --workspace --all-targets` | Compile the WIT consumers, host mirror, SDK, dispatcher, runtime audit, and tests. | FACT pass/fail; SNIPPETS <= 30 lines on first error |
| `cargo test -p slicer-runtime --all-targets --test contract -- wit_drift_detection_tdd 2>&1 \| tee target/test-output.log` | Verify canonical WIT source and bindgen path assertions in the existing `contract` binary. | FACT pass/fail; SNIPPETS <= 20 lines on failure |
| `cargo test -p slicer-runtime --all-targets --test integration -- prepass_diagnostic_roundtrip_tdd 2>&1 \| tee target/test-output.log` | Verify exact guest-to-host audit round-trip, FIFO order, and code `99` acceptance through the real `integration` aggregate target. | FACT pass/fail; SNIPPETS <= 30 lines on failure |
| `cargo test -p support-planner --all-targets --test orca_parity_tdd -- node_dropped_when_avoidance_rejects_all_moves 2>&1 \| tee target/test-output.log` | Verify the existing node-clamped fixture now inspects typed output. | FACT pass/fail; SNIPPETS <= 20 lines on failure |
| `cargo test -p support-planner --all-targets --test diagnostics_tdd 2>&1 \| tee target/test-output.log` | Verify cap emission, below-cap silence, planner-owned interface warning emission, and default/absent-key silence. | FACT pass/fail; SNIPPETS <= 30 lines on failure |
| `! rg -q 'support-planner\.node-clamped-out:' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'support-planner: max_branches_per_layer' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'support-planner: support_interface_bottom_layers' modules/core-modules/support-planner/src/lib.rs` | Verify all three legacy prefixes are absent. | FACT pass/fail |
| `rg -q 'ModuleAccessAudit.*diagnostics' docs/02_ir_schemas.md && rg -q 'push-diagnostic' docs/03_wit_and_manifest.md && rg -q 'SupportGeometryOutput::push_diagnostic' docs/05_module_sdk.md` | Verify the three Doc Impact sections. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint gate. | FACT pass/fail; SNIPPETS <= 20 lines on first error |

## Step Completion Expectations

- The WIT and SDK edits invalidate guest artifacts. No guest-dependent test runs until `cargo xtask build-guests --check` is clean; rebuild if it reports `STALE:`.
- `ModuleAccessAudit.diagnostics` preserves guest emission order and is empty for modules that emit no diagnostic. Scheduler validation continues to compare only runtime reads and writes.
- The round-trip guest is separate because the current macro-authored `sdk-prepass-guest` is a MeshAnalysis-stage fixture; no second stage implementation is added to it. Its host path is `execute_prepass_with_instrumentation` -> `WasmRuntimeDispatcher::run_stage` -> `dispatch_prepass_call`, with diagnostics returned through `PrepassStageRunner::last_diagnostics`; it is not a standalone dispatcher.
- The cap fixture uses `support_max_branches_per_layer = 1024` and must exercise the current `SupportPlanner::run_support_geometry` cap sites without weakening the cap.
- Packet 116's explicit no-warning boundary is rechecked during authoring, but its status does not block AC-6 or AC-N3. The planner-owned code `1003` path must remain self-contained and must not reintroduce packet-116 state.

## Context Discipline Notes

- Delegate `docs/03_wit_and_manifest.md` and `docs/05_module_sdk.md`; do not read either end-to-end.
- Range-read `crates/slicer-sdk/src/prepass_builders.rs`, `crates/slicer-wasm-host/src/host.rs`, `crates/slicer-wasm-host/src/dispatch.rs`, `crates/slicer-runtime/src/prepass.rs`, and `modules/core-modules/support-planner/src/lib.rs` around named symbols only.
- Do not browse generated bindgen output, every guest manifest, `target/`, or guest lockfiles.
- Do not read `OrcaSlicerDocumented/**`; this packet ports no Orca algorithm.
- Cargo dispatches return `FACT`; failures return only bounded `SNIPPETS` with the first relevant error.

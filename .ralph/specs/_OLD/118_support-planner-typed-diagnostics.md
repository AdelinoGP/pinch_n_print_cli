---
status: implemented
packet: 118_support-planner-typed-diagnostics
task_ids: [TASK-163b-diagnostic]
---

# 118_support-planner-typed-diagnostics

## Goal

Add the typed prepass diagnostic contract to the existing `SupportGeometryOutput` path, preserve emitted order in `ModuleAccessAudit.diagnostics`, and have support-planner own all three typed warning paths without changing fatal-error behavior. The `support_interface_bottom_layers` record is created here from the preserved config key; packet 116 emits no warning.

## Problem Statement

`SupportPlanner::run_support_geometry` emitted warnings via `host-services.log` string prefixes, and the cap paths silently discarded candidates. Packet 116 removed the dead bottom-interface state and emits no warning. Packet 118 reads the preserved `support_interface_bottom_layers` config key itself and owns the typed code-1003 record; the cap keeps its configured limit and data-flow truncation, adding one typed diagnostic per affected layer. No fatal-error behavior changes: diagnostics are recoverable audit metadata; aborting errors remain `ModuleError::fatal(...)`.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path that feeds the guest build (canonical WIT, SDK, macro), the implementer MUST run `cargo xtask build-guests --check` and rebuild if `STALE:`.
- CLAUDE.md §WIT/Type Changes Checklist binding: edit canonical WIT at `crates/slicer-schema/wit/` only; verify type identity across `wit_host.rs` / `dispatch.rs` / `wit_guest`.
- WIT shape is the record+enum design of ADR-0010 (additive `code` field, no variant-per-class — new diagnostic classes ship as module-internal changes without guest rebuilds).

## Data and Contract Notes

- WIT (in `resource support-geometry-output`): `push-diagnostic: func(d: diagnostic) -> result<_, string>;` with `record diagnostic { severity: severity-level, code: u32, layer: option<s32>, object-id: option<string>, message: string }` and `enum severity-level { trace, debug, info, warn, error }`.
- SDK: `SupportGeometryOutput::push_diagnostic` + ordered diagnostic accessor; `slicer_ir::Diagnostic` / `slicer_ir::DiagnosticSeverity` with `layer: Option<i32>`, `object_id: Option<String>`.
- Host drain: `HostExecutionContext.diagnostics` → WIT-to-host conversion in `crates/slicer-wasm-host/src/host.rs` → `PrepassStageRunner::last_diagnostics` (`traits.rs`) → `WasmRuntimeDispatcher` stash/drain (`dispatch.rs`) → `ModuleAccessAudit.diagnostics: Vec<slicer_ir::Diagnostic>` attached to both prepass audit constructor branches. Audit comparison continues to use runtime reads/writes only.
- Three codes: `1001` cap (one warning per affected GLOBAL layer, message contains `max_branches_per_layer cap exceeded`, `dropped_count=<n>`, `kept_count=<configured cap>`); `1002` `node-clamped-out` (replaces the string log; fixture `node_dropped_when_avoidance_rejects_all_moves` now inspects typed output); `1003` `support_interface_bottom_layers` (planner-owned, read from preserved key at the start of `run_support_geometry`, emitted once BEFORE the layer loop when value is not `-1`, `layer == None`).
- Code ranges remain a per-module convention (support-planner 1000-1999), NOT host-enforced — AC-N2 proves out-of-range code 99 is captured.
- MED-1 follow-up (2026-07-19): the cap accumulator was hoisted out of `plan_for_object` into `run_support_geometry` and merged across objects — a multi-object cap hit on the same global layer produces ONE merged code-1001 diagnostic (`object_id=None`, summed `dropped_count`); regression `multi_object_cap_diagnostic_merges_per_layer` in `diagnostics_tdd.rs`.
- MED-2 note: `LAST_PREPASS_DIAGNOSTICS` is a process-global `thread_local!` mirroring `LAST_MODULE_LOG_MESSAGES` — same single-threaded-per-dispatcher constraint.

## Locked Assumptions and Invariants

- FIFO order preserved in the audit vector; empty for modules that emit no diagnostic.
- Legacy prefixes are gone: `! rg 'support-planner\.node-clamped-out:|support-planner: max_branches_per_layer|support-planner: support_interface_bottom_layers'` — three zero-hit greps.
- Packet 116's no-warning boundary preserved (packet 116 emits no warning; packet 118 owns code 1003 end-to-end).
- The round-trip guest is a separate `sdk-support-diagnostic-guest` (the macro-authored `sdk-prepass-guest` is a MeshAnalysis-stage fixture); host path is `execute_prepass_with_instrumentation` → `WasmRuntimeDispatcher::run_stage` → `dispatch_prepass_call`.
- Source-plan `TASK-253` NOT retained — current ledger assigns it to paint segmentation; packet closes against `TASK-163b-diagnostic` only.

## Risks and Tradeoffs

- WIT change triggers full guest rebuild across all guests — standard per ADR-0010's recorded trade-off.
- Routine trace/debug logging stays on `host-services.log`; do NOT migrate all `log(...)` calls (ADR-0010 Future-Reviewer note).

## Implementation Deviations (recorded at close)

None beyond the MED-1 merged-cap behavior (recorded above). Doc Impact: `docs/02_ir_schemas.md` (`ModuleAccessAudit.diagnostics`), `docs/03_wit_and_manifest.md` (`push-diagnostic`), `docs/05_module_sdk.md` (`SupportGeometryOutput::push_diagnostic`) all updated in-packet.

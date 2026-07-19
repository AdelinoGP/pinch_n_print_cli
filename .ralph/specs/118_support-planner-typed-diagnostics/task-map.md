# Task Map: support-planner-typed-diagnostics

The retained frontmatter task ID maps to the live typed-diagnostic row in `docs/07_implementation_status.md`. `TASK-163b-diagnostic` is the only retained canonical task ID. Source-plan `TASK-253` is an excluded historical label: the current ledger assigns it to paint-segmentation work, and no replacement ID is invented.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-163b-diagnostic` | Steps 1-9 | `docs/specs/support-modules-orca-port.md` sections B4, B7, D10, and D11; `docs/adr/0010-typed-diagnostic-channel.md`; `docs/01_system_architecture.md`; `docs/03_wit_and_manifest.md`; `docs/05_module_sdk.md` | Canonical `world-prepass.wit` diagnostic contract; `slicer_ir::Diagnostic` and `DiagnosticSeverity`; `SupportGeometryOutput::push_diagnostic`; `pm::HostSupportGeometryOutput for HostExecutionContext`; `WasmRuntimeDispatcher::run_stage` / `dispatch_prepass_call`; `PrepassStageRunner::last_diagnostics`; `execute_prepass_with_instrumentation`; `ModuleAccessAudit.diagnostics`; planner-owned node, cap, and `support_interface_bottom_layers` paths; round-trip, planner, and documentation tests | none | M | The live row owns the complete typed channel and support-planner migration, including code `1003`. The excluded `TASK-253` source-plan label is not a closure ID. |

Aggregate context cost: `M`. No step is `L`.

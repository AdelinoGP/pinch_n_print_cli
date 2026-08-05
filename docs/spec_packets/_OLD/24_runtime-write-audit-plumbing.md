---
status: implemented
packet: 24_runtime-write-audit-plumbing
task_ids:
  - TASK-123b
  - TASK-124
---

# 24_runtime-write-audit-plumbing

## Goal

Repair the layer runtime-write audit plumbing so modules that write narrow manifest paths (e.g. `seam-placer` writing `PerimeterIR.resolved-seam`) do not trip the undeclared-access validator, which currently treats any write not in `ir_path_for_layer_stage` as missing and any write not covered by the manifest as a failure.

## Problem Statement

The layer runtime-write audit plumbing uses `ir_path_for_layer_stage` as the authoritative source for `ModuleAccessAudit.runtime_writes`, which returns coarse IR roots like `"PerimeterIR"` for `Layer::PerimetersPostProcess`. When `seam-placer` declares a narrow manifest write `PerimeterIR.resolved-seam` and the guest calls `push_resolved_seam`, the audit still records `"PerimeterIR"` — causing the undeclared-access validator to either (a) reject the coarse path against the narrow manifest declaration, or (b) miss that the narrow path was actually exercised. The fix requires instrumenting the output-builder methods directly to record canonical subfield paths, and updating the dispatch/executor chain to carry `runtime_writes` alongside `runtime_reads`.

## Architecture Constraints

- `HostExecutionContext` is per-call and re-created for each WASM call; `runtime_writes` must not persist across calls.
- The write-path vocabulary must match the manifest/docs naming style from `docs/04_host_scheduler.md §Manifest ↔ Runtime Naming Map`: `PerimeterIR.regions.walls` not `PerimeterIR.wall-loops`.
- Negative case: if `seam-placer` manifest says `PerimeterIR.resolved-seam` but runtime collects `PerimeterIR`, `validate_undeclared_access` must fail (coarse write against narrow declared path is an error).

## Data and Contract Notes

- `ModuleAccessAudit.runtime_writes` is `Vec<String>` (same as `runtime_reads`).
- Canonical paths use the manifest naming style (dot-notation, kebab-case field names): `PerimeterIR.regions.walls`, `PerimeterIR.resolved-seam`.
- The `ir_path_for_layer_stage` function is NOT deleted — it remains as a fallback for stages whose builder methods are not yet instrumented.

## Risks and Tradeoffs

- **Risk**: Changing `LayerStageRunner::run_stage` signature is a breaking API change for all implementors. All call sites must be updated.
  - Mitigation: Update all implementations (WasmRuntimeDispatcher + test fakes) in the same atomic change.
- **Risk**: If any perimeter builder method is not instrumented and a module uses it, the audit will be silently incomplete.
  - Mitigation: Add a negative test that fails when the write is missing from the audit.
- **Tradeoff**: Recording coarse `InfillIR`/`SupportIR` instead of subfield paths is a simplification. If a future module writes a subfield of InfillIR, it will need a new instrumented method.
  - Accepted: This is the same coarse pattern used by `ir_path_for_layer_stage` today; the same coarse fallback is preserved for uninstrumented stages.

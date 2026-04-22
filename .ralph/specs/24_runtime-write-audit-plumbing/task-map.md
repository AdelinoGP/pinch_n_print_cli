# Task Map: 24_runtime-write-audit-plumbing → docs/07

This packet maps to Workstream 1 remediation tasks that extend TASK-123 (runtime access audit plumb) and TASK-124 (undeclared access enforcement).

## Task ID Mapping

| Packet Step | docs/07 Task | Notes |
|---|---|---|
| Step 1 | TASK-123b | `runtime_writes` field on `HostExecutionContext` |
| Step 2 | TASK-123b | Instrument perimeter builder methods |
| Step 3 | TASK-123b | Instrument Infill/Support coarse writes |
| Step 4 | TASK-123b | Update `LayerStageRunner::run_stage` signature |
| Step 5 | TASK-123b | Update dispatcher to return writes |
| Step 6 | TASK-123b, TASK-124 | Update `execute_single_layer` audit construction; enforce narrow writes |
| Step 7 | TASK-123b | pipeline_tdd.rs regression tests |
| Step 8 | TASK-124 | `core_module_ir_access_contract_tdd.rs` live seam regression |
| Step 9 | TASK-124 | Negative regression for missing instrumentation |

## Superseding Relationship

- Packet 24 does NOT supersede any prior packet. It extends the runtime-access-audit work from packet 02-rev4 (runtime-access-audit-and-declaration-enforcement) with a specific root-cause fix for the seam narrow-write finding.

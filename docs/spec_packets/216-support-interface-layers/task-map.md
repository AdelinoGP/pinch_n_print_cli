# Task Map: support-interface-layers

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-325` | Steps 1-10 | `docs/specs/support-generation-remediation-plan.md`; `docs/specs/support-generation-defect-verified-findings.md` | `SupportInterfacePlanEntry`, `SupportPlanIR.interface_plan`, WIT/SDK/macro/host transport, planner, both support consumers/manifests, fixture/docs | `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` delegated | M | One coherent planner-plan/module-generates slice; Step 8 explicitly adds the traditional-support `SupportPlanIR` manifest read. |

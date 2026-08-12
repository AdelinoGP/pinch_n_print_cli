# Task Map: support-planner-defect-fix

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-329` | `Step 1`, `Step 2`, `Step 3`, `Step 4` | `docs/specs/support-generation-remediation-plan.md`; `docs/specs/support-generation-defect-verified-findings.md` | `modules/core-modules/support-planner/src/lib.rs`; focused planner tests; `crates/slicer-runtime/src/visual_debug_render.rs`; renderer regression test | `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` | `S` + `S` + `M` + `M` | Closes RC-1 and RC-4 with focused assertions, the required visual-debug gate, and renderer-side degenerate segment visualization. |

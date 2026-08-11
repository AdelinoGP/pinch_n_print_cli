# Task Map: support-type-variants

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-326` | `Step 1` | `support-generation-remediation-plan.md`; `support-generation-defect-verified-findings.md` | `support-planner.toml`; planner tests | `SupportMaterial.cpp` | `S` | Establishes the mode input and red coverage. |
| `TASK-326` | `Step 2` | `support-generation-remediation-plan.md`; `support-generation-defect-verified-findings.md` | `modules/core-modules/support-planner/src/lib.rs` | `TreeSupport.cpp` | `M` | Implements enforcers-only manual mode without changing claim resolution. |
| `TASK-326` | `Step 3` | `docs/19_visual_debug.md` | `pnp-cli` visual-debug integration test; `tmp/visual-debug-tree.json`; `tmp/support-config-manual.json`; `tmp/visual-debug-support-manual.json` | none | `M` | Proves auto/manual geometry through planner PNG entries and exact `Layer::Support` typed-capture paths for every requested layer. |

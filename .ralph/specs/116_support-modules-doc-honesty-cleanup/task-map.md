# Task Map: support-modules-doc-honesty-cleanup

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-250` | Step 2, Step 6 | `docs/specs/support-modules-orca-port.md` §B1 | `modules/core-modules/{tree-support,traditional-support,support-planner}/src/lib.rs` lead `//!` blocks | none | S | Step 2 rewrites; Step 6 gates AC-1, AC-2, AC-3. |
| `TASK-251` | Step 3, Step 4, Step 5 | `docs/specs/support-modules-orca-port.md` §B2, §D8 | `modules/core-modules/support-planner/src/lib.rs` (struct + parse) + `support-planner.toml` (comment) + new `tests/interface_bottom_layers_warning_tdd.rs` | none | S | Step 3 authors RED tests; Step 4 deletes field + adds warning emission (RED→GREEN); Step 5 adds TOML comment. Three ACs gate this (AC-4, AC-5, AC-N1, AC-N2). |
| `TASK-252` | Step 5, Step 6 | `docs/specs/support-modules-orca-port.md` §B3, §D9 | `modules/core-modules/{tree-support,traditional-support,support-planner,rectilinear-infill}/src/lib.rs` `# Speed normalization` sections | none | S | Step 5 adds the section to all four modules; Step 6 gates AC-6. |

Aggregate context cost across rows: `S` (no row exceeds `S`, no row L). Packet ships as a single cohesive Block B / Bucket B housekeeping slice from `docs/specs/support-modules-orca-port.md`.

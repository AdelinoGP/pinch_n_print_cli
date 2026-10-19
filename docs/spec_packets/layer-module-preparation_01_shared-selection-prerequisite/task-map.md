# Task Map: shared-selection-prerequisite

The approved eight-packet queue assigns TASK-572 row 01 both delivery repair and reusable targeting. No prerequisite; accepted exports unblock #04 and subsequently #05. This crosswalk does not alter the backlog or source plan.

| Task | Steps | Primary authority | Surface | Family / context |
| --- | --- | --- | --- | --- |
| TASK-572, row 01 | 1 | docs/02_ir_schemas.md; docs/22_test_quality.md | Registered runner witnesses, corrected painted identity | A / M |
| TASK-572, row 01 | 2 | Source plan selection semantics; docs/02_ir_schemas.md | Public selection module and export | B / M |
| TASK-572, row 01 | 3 | ADR-0056; docs/04_host_scheduler.md | Native projection builder and context-backed commit | A+B / M |
| TASK-572, row 01 | 4–5 | docs/03_wit_and_manifest.md; docs/02_ir_schemas.md | Full-origin host maps, dispatcher wiring, coloring enforcement | A+B / M each |
| TASK-572, row 01 | 6–7 | docs/22_test_quality.md; source plan targeting | Registered accessor/perimeter and shared native/WASM witnesses; AC-B2 carrier selection by global layer, anchor preserved as provenance | A+B / M each |
| TASK-572, row 01 | 8–9 | docs/04_host_scheduler.md; schema STAGES | Registered runtime production driver and shared eligibility consumption | B / M each |
| TASK-572, row 01 | 10 | Existing two-sided coloring contract; docs/22_test_quality.md | Native production/WASM boundary permutation witness | B / M |
| TASK-572, row 01 | 11 | docs/04_host_scheduler.md | Normative routing section and delegated acceptance | A+B / S |

Carrier crosswalk: `carrier_selection_delivery_fixture` at layer 7 retains exactly one eligible carrier, entry 0 with global 7/anchor 3; entry 1 with global 3/anchor 7 and wrong-family/declined entries are excluded. It proves delivery/config, not nonempty support rendering. Independently execute `support_rendering_positive_control` with one global-7/anchor-7 carrier and nonempty support geometry, asserting actual nonempty native/WASM support paths and points; independently execute `raft_positive_control` and assert its nonempty paths on both legs. Controls must not increase the selection fixture's eligible cardinality. Existing exports, predicates, test count and scope are unchanged.

No Orca translation/reference obligations. Status remains draft; implementation/gates/tests are outside this authoring task, and activation requires successful independent preflight and explicit approval. TASK-572 has later preparation obligations; row-01 evidence must not close the entire workstream. Source-plan/backlog files are read-only to this authoring task.

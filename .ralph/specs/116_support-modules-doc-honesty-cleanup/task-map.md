# Task Map: support-modules-doc-honesty-cleanup

The batch anchor names B1/B2/B3 as `TASK-250`/`TASK-251`/`TASK-252`, but those IDs are not current canonical ownership for this support slice: `TASK-250` and `TASK-252` are unrelated current work, and no support row for `TASK-251` exists. The crosswalk therefore uses source-plan labels only and intentionally assigns no replacement ID.

| docs/07 task ID | Source-plan work item | Packet step | Primary docs | Expected code surface | Verification | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `— (unmapped B1)` | B1 | Steps 1-3, 5 | `docs/specs/support-modules-orca-port.md` §B1 | Three support-module contiguous leading `//!` blocks | AC-1, AC-2, and AC-3 use bounded prefix extraction and assert each required opening line inside the block. | none | S | Requires a new or explicitly mapped support backlog row; do not reuse colliding IDs. |
| `— (unmapped B2)` | B2 / D8 | Steps 1, 4-5 | `docs/specs/support-modules-orca-port.md` §B2, §D8 | `SupportPlanner`, `PrepassModule::on_print_start`, `support-planner.toml` | AC-4 rejects all field/struct-literal assignment forms and the parse lookup; AC-7 proves the snake_case schema entry and adjacent deferred-status comment in a bounded excerpt. | none | S | Packet 116 deletes the dead field/parse and preserves the schema signal; packet 118 owns D11 typed warning emission. Its current dependency on a packet-116 string warning is an explicit blocker, not an ownership claim. |
| `— (unmapped B3)` | B3 | Steps 1-3, 5 | `docs/specs/support-modules-orca-port.md` §B3, §D9 | `tree-support`, `traditional-support`, `rectilinear-infill` contiguous leading blocks | AC-6 checks `# Speed normalization`, the formula, and `BASE_SPEED = 50.0` in each current consumer's bounded leading block. | none | S | Current `support-planner` has no `BASE_SPEED`; other consumers remain out of scope. |

Aggregate context cost across rows: `S`; no row exceeds `S` and no row is L. Activation is blocked until the backlog maintainer supplies canonical IDs.

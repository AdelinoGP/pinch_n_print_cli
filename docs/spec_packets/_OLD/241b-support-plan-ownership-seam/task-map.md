# Task Map: 241b-support-plan-ownership-seam

Single umbrella task; this crosswalk exists because the packet absorbs DEV-167 (filed by packet 241) and turns packet 241's AC-N2 green without reopening it.

`TASK-531` is bound to this packet by `docs/07_implementation_status.md`. The backlog source's Packet Queue rows 7a/7b carried an unregistered forward reservation of `TASK-531..TASK-535` for packets 240a/240b; Step 6c/6d repair that collision. Re-derive the free id block at edit time — these are ledger facts.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-531` | `Step 1` | `docs/04_host_scheduler.md` | `crates/slicer-wasm-host/src/support_aggregation.rs` | none | S | declared merge key replaces centroid cell; `ROUTING_CELL_SIZE` renamed `MAX_BODY_EXTENT_UNITS` (it cannot be deleted — `in_routing_cell` reads it) |
| `TASK-531` | `Step 2` | `docs/02_ir_schemas.md` IR 9b, `docs/01_system_architecture.md` | `crates/slicer-wasm-host/src/support_aggregation.rs` | none | M | ownership check, producer identity, arrival order deleted; `SupportAggregationError` stays a struct |
| `TASK-531` | `Step 3` | `docs/21_data_defaults_and_fixtures.md` | `crates/slicer-runtime/src/prepass.rs`, aggregation test fixtures | none | M | blast radius of the new input field; 15 test literals across 3 files plus 2 in `src/` |
| `TASK-531` | `Step 4` | `docs/02_ir_schemas.md` IR 9b | `modules/core-modules/traditional-support-planner` | none | S | DEV-167 fix hardened; packet-239 tests restored against `roles[].regions` areas |
| `TASK-531` | `Step 5` | `docs/specs/support-families-anchored-entities-plan.md` Ruling 1 | `modules/core-modules/tree-support-planner` | none | S | no-row = no owner enforced at the producer too; three edit sites in one long fn |
| `TASK-531` | `Step 6` | `docs/02`, `docs/01`, `docs/04`, ADR-0059, `docs/DEVIATION_LOG.md`, backlog source | `crates/slicer-scheduler/src/validation.rs` (doc comment) | none | M | W7 text defects; ADR-0059 `Ruling 2` amendment + deviation row; DEV-167 closed; paint/tool axis recorded; task-id collision repaired; split 6a/6b/6c/6d |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.

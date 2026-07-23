# Task Map: support-validation-wedge-harness

`TASK-290` is the free replacement for the colliding source-plan `TASK-260` allocation. It covers the absorbed packet-119 planner, IR, WIT, macro, host-marshal, SDK, and seam-guest closure work in Steps 9-12. Steps 1-8 remain packet-local harness work with no separate formal task ID.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `- (packet-local)` | Step 1 | `docs/specs/support-modules-orca-port.md` C1 and Validation Strategy; `docs/02_ir_schemas.md` IR 9b; `docs/07_implementation_status.md` | Integration target registration, fixture helpers, `prepare_prepass_context`, `SupportPlanIR`, and `SupportGeometryIR` shape inventory | none | S | Confirm the current driver and public support surfaces before test authoring. |
| `- (packet-local)` | Step 2 | `docs/01_system_architecture.md` `PrePass::SupportGeometry`; `docs/02_ir_schemas.md` IR 9b | `crates/slicer-runtime/tests/common/support_wedge.rs` and its registration in `common/mod.rs` | none | M | Shared helper must run the real wedge prepass and reject an enabled empty plan. |
| `- (packet-local)` | Step 3 | `docs/02_ir_schemas.md` IR 9b; `docs/08_coordinate_system.md`; `docs/specs/support-modules-orca-port.md` C1 and Validation Strategy | Integration aggregate registration and current public wedge invariants | none | M | Assertions use public committed IR and canonical unit conversion only. |
| `- (packet-local)` | Step 4 | `docs/01_system_architecture.md` `PrePass::SupportGeometry`; `docs/07_implementation_status.md` | Wedge helper config override and disabled-support test | none | S | `support_enabled = false` must produce an explicitly empty plan without weakening the enabled-path check. |
| `- (packet-local)` | Step 5 | `docs/specs/support-modules-orca-port.md` Validation Strategy; `docs/02_ir_schemas.md` IR 9b | Golden parser, endpoint extraction, Hausdorff comparison, and guarded regeneration path | none | M | Normal tests compare committed self-captures without writing them. |
| `- (packet-local)` | Step 6 | `docs/specs/support-modules-orca-port.md` Validation Strategy | The two committed wedge golden resources | none | M | Capture is allowed only after prerequisite packets, freshness, and non-empty enabled output are confirmed. |
| `- (packet-local)` | Step 7 | `docs/specs/support-modules-orca-port.md` Validation Strategy | In-memory branch-count drift test | none | S | The negative case detects drift without mutating either committed golden. |
| `- (packet-local)` | Step 8 | `docs/07_implementation_status.md`; packet 119 verification matrix | Final harness, freshness, check, and clippy evidence | none | S | Packet remains draft until the separate status-flip step. |
| `TASK-290` | Step 9 | `docs/08_coordinate_system.md`; ADR-0048 | `modules/core-modules/support-planner/src/lib.rs` unit-consistency and collision-guard fixes | none | S | Planner fixes absorbed into packet 119 and assigned to the re-numbered task. |
| `TASK-290` | Step 10 | ADR-0048; canonical WIT and IR docs | Canonical WIT, macro mapping, host marshal, SDK prepass types/builders, and `slicer-ir` schema 1.2.0 | none | M | Add `dist_to_top_mm`, `raft_plan`, `push-raft-plan`, and the ABI-safe six-field seam point. |
| `TASK-290` | Step 11 | ADR-0048; support planner manifest/config docs | `support-planner` config/source emission and `seam-planner-default/wit-guest/Cargo.toml` package name | none | S | Emit exact raft configuration and forward per-point distance values. |
| `TASK-290` | Step 12 | ADR-0048; packet 119 verification matrix | Wedge AC-8, AC-9, AC-N3 tests and final gates | none | M | Confirm schema 1.2.0, WIT freshness, check, clippy, and all packet acceptance criteria. |

Aggregate context cost: `M`. No step is `L`; the task mapping is resolved as `TASK-290`.

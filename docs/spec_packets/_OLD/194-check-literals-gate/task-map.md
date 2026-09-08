# Task Map: 194-check-literals-gate

Single-task packet; this crosswalk is emitted because `TASK-316` is a **new** backlog row that does not yet exist in `docs/07_implementation_status.md` — the implementing swarm registers it there at the completion gate (worker dispatch, never a full backlog read). Re-derive the highest existing TASK id at registration time (`rg -o 'TASK-[0-9]{3}' docs/07_implementation_status.md | sort -u | tail -1`); if TASK-316 is already taken by then, renumber the row and this packet's frontmatter together. Suggested row text: "TASK-316 — `cargo xtask check-literals` struct-literal churn gate (report mode, path filter, docs/21) — packet 194".

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-316` | `Step 1` | `docs/specs/struct-literal-churn-gate-plan.md` | `xtask/Cargo.toml`, `xtask/src/check_literals.rs`, `xtask/src/main.rs` | none | S | Watchlist rule provable from in-memory fixtures |
| `TASK-316` | `Step 2` | `docs/specs/struct-literal-churn-gate-plan.md` | `xtask/src/check_literals.rs` | none | M | Every AST violation class + waiver semantics unit-locked |
| `TASK-316` | `Step 3` | `docs/specs/struct-literal-churn-gate-plan.md` | `xtask/src/check_literals.rs` | none | M | Macro token-tree scan + documented blind spot lock |
| `TASK-316` | `Step 4` | `docs/specs/struct-literal-churn-gate-plan.md` | `xtask/src/check_literals.rs`, `xtask/src/main.rs` | none | M | Live-run ACs prove CLI contract consumed by packets 195-199 |
| `TASK-316` | `Step 5` | `docs/specs/struct-literal-churn-gate-plan.md` | `docs/21_data_defaults_and_fixtures.md`, `.claude/doc-index.md`, `docs/00_project_overview.md` | none | S | Rule page is the durable half of the fix |
| `TASK-316` | `Step 6` | `docs/specs/struct-literal-churn-gate-plan.md` | `CLAUDE.md` | none | S | Gate-off marker prevents premature enforcement |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.

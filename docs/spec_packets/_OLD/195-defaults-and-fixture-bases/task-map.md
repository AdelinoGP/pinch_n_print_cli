# Task Map: 195-defaults-and-fixture-bases

Single-task packet; this crosswalk is emitted because `TASK-317` is a **new** backlog row that does not yet exist in `docs/07_implementation_status.md` — the implementing swarm registers it there at the completion gate (worker dispatch, never a full backlog read). Re-derive the highest existing TASK id at registration time (`rg -o 'TASK-[0-9]{3}' docs/07_implementation_status.md | sort -u | tail -1`); if TASK-317 is already taken by then, renumber the row and this packet's frontmatter together (and keep it consistent with packet 194's TASK-316 registration, which precedes this one). Suggested row text: "TASK-317 — safe `Default` impls + `sdk::test_support` fixture bases + per-crate `PipelineConfig` helpers for the struct-literal churn gate — packet 195".

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-317` | `Step 1` | `docs/specs/struct-literal-churn-gate-plan.md` | none (audit via `cargo xtask check-literals --report`) | none | S | Re-derives the class lists the code steps depend on |
| `TASK-317` | `Step 2` | `docs/specs/_OLD/default-builder-migration.md` §5 | `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/tests/unit/` | none | S | Class (a): quiet-baseline `Default` for `SliceRunOptions` |
| `TASK-317` | `Step 3` | `docs/specs/struct-literal-churn-gate-plan.md` decision 3(b) | `crates/slicer-sdk/src/test_support/fixtures.rs`, `crates/slicer-sdk/tests/`, `crates/slicer-sdk/Cargo.toml` | none | M | Class (b): the three `*_base` fns sweeps compose with FRU; guest rebuild owned here |
| `TASK-317` | `Step 4` | `docs/specs/struct-literal-churn-gate-plan.md` decision 3(c) | `crates/slicer-runtime/tests/common/mod.rs`, `crates/slicer-runtime/tests/integration/pipeline_tdd.rs`, `crates/pnp-cli/tests/e2e_integration_tdd.rs` | none | M | Class (c): one waivered literal per crate for the trait-object holder |
| `TASK-317` | `Step 5` | `docs/adr/0054-host-side-test-support-crate.md`, `docs/adr/0004-test-support-lives-in-slicer-sdk.md` | `crates/pnp-cli-locator/src/lib.rs` (header only) | none | S | Policy record: single IR-fixture home |
| `TASK-317` | `Step 6` | `CLAUDE.md` §Test Discipline | none (gate sweep) | none | S | Proves additions are gate-clean before sweeps consume them |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.

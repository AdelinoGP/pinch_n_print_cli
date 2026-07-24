# Task Map: 176-support-preview-verb

Single-task packet; crosswalks to the canonical support-preview row in `docs/07_implementation_status.md`.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-291` | `Step 1` | `docs/15_config_keys_reference.md` (grep) | none (discovery) | — | S | Locks fixture + config for AC-1/AC-2 |
| `TASK-291` | `Step 2` | `docs/08_coordinate_system.md` (range) | `crates/pnp-cli/src/support_preview.rs`, `crates/pnp-cli/src/main.rs` | — | M | Verb + JSON contract structs |
| `TASK-291` | `Step 3` | — | `crates/pnp-cli/tests/support_preview_tdd.rs` | — | M | 6 tests; AC-2 pins mm conversion |
| `TASK-291` | `Step 4` | `docs/19_visual_debug.md` (SUMMARY), `docs/08_coordinate_system.md` (range) | `docs/20_support_preview.md`, `.claude/doc-index.md` | — | S | Fork-facing contract doc |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.

# Task Map — Packet 80

This packet spans **2 task IDs** in `docs/07_implementation_status.md`. Both are added during this packet's generation (TASK-229, TASK-230 reserved at packet 79's docs/07 sync).

## Task → Step crosswalk

| Task ID | Covered by step(s) | One-line scope |
|---|---|---|
| TASK-229 | Steps 1, 2, 3, 4, 5 | Relocate `wipe_tower_bed_bounds.rs` → `wipe-tower/tests/bed_bounds_tdd.rs` (rewrite helpers via packet 79 builders); relocate `prepass_support_generation_orca_parity_tdd.rs` → `support-planner/tests/orca_parity_tdd.rs` (switch `#[test]` → `#[module_test]`, drop manual `install_log_capture`); delete source files; remove `mod` declarations from `executor/main.rs`. |
| TASK-230 | Steps 6, 7, 8, 9, 10 | Add `NOT RELOCATABLE` comments to three legitimately-runtime tests (`slicing_promotion_e2e_regression_tdd`, `gcode_part_cooling_emission_tdd`, `gcode_skirt_brim_emission_tdd`) naming their runtime SUTs; verify packet 77 hook is load-bearing for the relocated support-planner test (AC-N2 manual probe); run final closure ceremony. |

## Authoritative docs per task

| Task ID | Docs |
|---|---|
| TASK-229 | `crates/slicer-runtime/tests/executor/{wipe_tower_bed_bounds, prepass_support_generation_orca_parity_tdd}.rs` (source files; full read for verbatim copy + assertion snapshot). `crates/slicer-runtime/tests/executor/main.rs` (aggregator; recon-confirmed lines 36 and 42). `crates/slicer-sdk/src/test_prelude.rs` (post-packet-78; for import paths). `docs/02_ir_schemas.md` IR-12 LayerCollectionIR + ToolChange (only the field surfaces needed for the wipe-tower helper rewrite). |
| TASK-230 | The three runtime tests that stay (`slicing_promotion_e2e_regression_tdd.rs`, `gcode_part_cooling_emission_tdd.rs`, `gcode_skirt_brim_emission_tdd.rs`) — read their first ~30 lines to confirm structure before adding annotations. `crates/slicer-sdk/src/test_support/mod.rs` (post-packet-77; for the AC-N2 probe of `reset_global_state`). |

## OrcaSlicer references

None. The relocated `prepass_support_generation_orca_parity_tdd.rs` file's content mentions OrcaSlicer parity, but that's the support-planner module's algorithmic concern — not this packet's scope. We relocate verbatim except for the documented helper rewrites and macro switch.

## Predecessor / successor relationships

- **Predecessors**:
  - Packet 79 (TASK-227, TASK-228). Hard requirement — the `LayerCollectionFixtureBuilder` + `tool_change(...)` helpers used by the relocated wipe-tower test were added in packet 79.
  - Packet 78 (TASK-225, TASK-226). Indirectly required — `slicer_sdk::test_prelude` is a packet-78 introduction.
  - Packet 77 (TASK-223, TASK-224). Indirectly required — `#[module_test]` + `reset_global_state` hook used by the relocated support-planner test were created in packet 77.
- **Successors**: None known at packet generation time. The user flagged a future packet (untracked, post-80) may move `GCodeEmitter` to its own crate, which would unlock relocating the two `gcode_*_emission` tests currently annotated NOT RELOCATABLE. That future packet would supersede this packet's annotations.

## Backlog sync status

TASK-229 and TASK-230 are added to `docs/07_implementation_status.md` with status `[ ]` during this packet's generation. They transition to `[x]` with `Closed <date> — packet 80` suffix at the end of this packet's Acceptance Ceremony.

## End-state of the 77-80 sequence

This packet completes the four-packet architectural refactor described in the original plan (`C:\Users\agpen\.claude\plans\hidden-discovering-lollipop.md`). At packet 80's closure:

- One canonical module-testing surface lives at `slicer_sdk::test_support`, behind feature `test`, exposed via `slicer_sdk::test_prelude`.
- `#[module_test]` expands to fully-qualified paths; all four hooks (`reset_global_state`, `install_panic_handler`, `mock_host_setup`, `mock_host_teardown`) do real work.
- `MockHost` is a real `MeshSource` adapter with builder-style chaining.
- `crates/slicer-test/` no longer exists; workspace at 27 members.
- All 20 core-modules with tests use shared builders (the 4 no-test modules are documented in packet 79's task-map.md).
- 2 misplaced runtime tests relocated; 3 legitimately runtime-located tests carry `NOT RELOCATABLE` comments.
- `pnp_cli module new` scaffold emits one feature-flagged `slicer-sdk` dev-dep line.
- `docs/00`, `docs/05`, and `docs/adr/0004-test-support-lives-in-slicer-sdk.md` describe the shipped state.

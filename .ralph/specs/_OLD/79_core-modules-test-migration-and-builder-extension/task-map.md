# Task Map — Packet 79

This packet spans **2 task IDs** in `docs/07_implementation_status.md`. Both are reserved-and-recorded entries waiting for execution.

## Task → Step crosswalk

| Task ID | Covered by step(s) | One-line scope |
|---|---|---|
| TASK-227 | Steps 1, 2, 3, 4, 5, 6, 7 | Half one — extend `slicer_sdk::test_support::fixtures` with 5 new surfaces (`print_entity`, `tool_change`, `seam_candidate`, `LayerCollectionFixtureBuilder`, `PerimeterRegionViewBuilder::add_outer_wall_with_flags`); each lands TDD-first under `crates/slicer-sdk/tests/test_support_*_tdd.rs`. |
| TASK-228 | Steps 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18 | Half two — migrate 13 core-modules to `slicer_sdk::test_prelude::*`: 7 Group-A (existing builders cover); 4 Group-B (require half-one extensions); 3 Group-C (no `make_*` helpers — cosmetic-only decisions). Update `docs/05` §Test Support. Run closure ceremony (cargo test --workspace + wasm-target gate). |

## Authoritative docs per task

| Task ID | Docs |
|---|---|
| TASK-227 | `docs/02_ir_schemas.md` — IR-9 `PrintEntity`, IR-12 `LayerCollectionIR`/`ToolChange`, `SeamCandidate`, IR-6 `WallLoop` subsection. Read only the field surfaces via delegated FACT dispatches. `crates/slicer-sdk/src/layer_collection_builder.rs` (97 lines per recon) — confirmed not the right surface to extend; new `LayerCollectionFixtureBuilder` lives in `test_support/fixtures.rs`. |
| TASK-228 | `docs/05_module_sdk.md` §Test Support (post-packet-78 state) — append the new helper list. `docs/02_ir_schemas.md` — IR field surfaces for the migration fixtures. `CLAUDE.md` §Test Discipline — the workspace-test escape clause this packet activates for AC-11. Per-module `src/lib.rs` — only the `config.get_*` line ranges (delegated FACT). |

## OrcaSlicer references

None. This packet does not borrow or check parity against any OrcaSlicer code.

## Predecessor / successor relationships

- **Predecessors**:
  - Packet 78 (TASK-225, TASK-226). Hard requirement. `slicer_sdk::test_support` mod + `slicer_sdk::test_prelude` + the deleted `slicer-test` crate + the post-78 `pnp_cli` scaffold must all be in place.
  - Indirectly: packet 77's macro wiring + `MockHost` adapter (no direct touch in 79, but the 13 migrations consume `slicer_sdk::test_prelude::*` which is empty without 78, which is empty of macro support without 77).
- **Successors**:
  - Packet 80 — relocate 2 misplaced runtime tests (`wipe_tower_bed_bounds.rs` → `wipe-tower/tests/`, `prepass_support_generation_orca_parity_tdd.rs` → `support-planner/tests/`). The wipe-tower relocation depends on this packet's `LayerCollectionFixtureBuilder` + `tool_change(...)` helper, which is why the relocation lives in packet 80, not packed into 79.

## Backlog sync status

TASK-227 and TASK-228 were added with status `[ ]` during packet 77's generation; they remain `[ ]` until this packet closes. The closure ceremony updates them to `[x]` with the workspace-test count appended.

## Per-group module list (for traceability)

**Group A — 7 modules (clean fit):**
- layer-planner-default, lightning-infill, mesh-segmentation, traditional-support, tree-support, classic-perimeters, gyroid-infill (verify-only from P78)

**Group B — 4 modules (require half-one extensions):**
- path-optimization-default (uses existing `add_outer_wall` after collapse — no extension actually needed despite original plan), seam-placer (uses `seam_candidate` + `add_outer_wall_with_flags`), skirt-brim (uses `print_entity` + `LayerCollectionFixtureBuilder`), wipe-tower (uses `LayerCollectionFixtureBuilder` + `tool_change`)

**Group C — 3 modules (no `make_*` helpers; cosmetic decisions):**
- fuzzy-skin, support-surface-ironing, top-surface-ironing

**Skipped — 4 modules (no tests at all):**
- machine-gcode-emit, part-cooling, seam-planner-default, support-planner
- (support-planner gains its first test in packet 80 via runtime-test relocation)

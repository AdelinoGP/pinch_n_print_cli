# Task Map — Packet 78

This packet spans **2 task IDs** in `docs/07_implementation_status.md`. Both were added during the generation of packet 77; they are reserved-and-recorded entries waiting for this packet's execution to flip from `[ ]` to `[x]`.

## Task → Step crosswalk

| Task ID | Covered by step(s) | One-line scope |
|---|---|---|
| TASK-225 | Steps 1, 2, 3, 4, 5, 6, 7, 8 | Move source/tests from `slicer-test` into `slicer-sdk::test_support`; introduce `slicer_sdk::test_prelude`; update `pnp_cli module new` scaffold; remove workspace member; delete `crates/slicer-test/`; verify the gate is real (AC-5 + AC-N1). |
| TASK-226 | Steps 9, 10, 11, 12, 13, 14, 15 | Migrate `arachne-perimeters` + `rectilinear-infill` as exemplars; verify dev-dep is load-bearing (AC-N2); structurally rewrite `docs/05_module_sdk.md`; clean `docs/00_project_overview.md` crate inventory; scan `CLAUDE.md`; verify production wasm builds do not pull in test_support (AC-4). |

## Authoritative docs per task

| Task ID | Docs |
|---|---|
| TASK-225 | `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (created in packet 77 — the decision this task executes). `crates/slicer-sdk/src/lib.rs` (post-packet-77 state — already has `pub mod test_support;`). |
| TASK-226 | `docs/05_module_sdk.md:445-624` (structural rewrite target). `docs/00_project_overview.md:122-156` (crate-inventory rewrite target). `CLAUDE.md` (project root scrub). `modules/core-modules/arachne-perimeters/src/lib.rs` + `modules/core-modules/rectilinear-infill/src/lib.rs` (config-key string surface — read-only confirmation that migration setter keys match production). |

## OrcaSlicer references

None. This packet does not borrow or check parity against any OrcaSlicer code.

## Predecessor / successor relationships

- **Predecessor**: packet 77 (TASK-223, TASK-224). Hard requirement — `slicer-sdk::test_support` mod with the four hooks AND `MockHost` already a `MeshSource` adapter must exist before this packet's fold can land. Step 1 of `implementation-plan.md` verifies packet 77's `status: implemented`.
- **Successors**:
  - Packet 79 — bulk migration of remaining core-modules + builder extension covering `LayerCollectionIR`, `PrintEntity`, `ToolChange`, `SeamCandidate`, variant `WallLoop` flag combinations. Depends on this packet's `slicer_sdk::test_prelude` shape and the exemplar migration template.
  - Packet 80 — relocate two misplaced runtime tests (`wipe_tower_bed_bounds.rs`, `prepass_support_generation_orca_parity_tdd.rs`) from `crates/slicer-runtime/tests/` to their module crates. Depends on packet 79's builder extensions (the wipe-tower test needs `LayerCollectionBuilder` + `tool_change(...)` from packet 79).

## Backlog sync status

`docs/07_implementation_status.md` rows for TASK-225 and TASK-226 were added with status `[ ]` during packet 77's generation (alongside TASK-223 / TASK-224). They will transition to `[x]` with `Closed <date> — packet 78` suffix at the end of this packet's Acceptance Ceremony (see `implementation-plan.md` §Acceptance Ceremony).

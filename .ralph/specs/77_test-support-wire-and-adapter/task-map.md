# Task Map — Packet 77

This packet spans **2 task IDs** in `docs/07_implementation_status.md`. Both were added during the generation of this packet (no prior backlog entries existed for `slicer-test` / `module_test` / `MockHost` infrastructure work). Task IDs `TASK-223` and `TASK-224` are reserved for this packet's scope.

## Task → Step crosswalk

| Task ID | Covered by step(s) | One-line scope |
|---|---|---|
| TASK-223 | Steps 1, 2, 3 | Add `slicer-sdk` feature `test`; create `slicer-sdk::test_support` module with the four hooks; rewrite `#[module_test]` macro to fully-qualified calls. |
| TASK-224 | Steps 4, 5, 6, 7, 8 | Refactor `MockHost` as a real `MeshSource` adapter; add four regression tests; update `slicer-macros` dev-deps and delete local hook stubs; clean `docs/05` API fictions; record ADR-0004; verify gate-is-real via AC-N1 probe. |

## Authoritative docs per task

| Task ID | Docs |
|---|---|
| TASK-223 | `crates/slicer-sdk/src/host.rs:108-343` (read-only — existing thread-local seam); `CLAUDE.md` §Guest WASM Staleness (because the macro change affects bindgen text). |
| TASK-224 | `docs/05_module_sdk.md:445-624` (subject of cleanup); `docs/adr/0001-*.md` (style template for new ADR-0004); `crates/slicer-sdk/tests/host_wrappers_tdd.rs:67-100` (`StubMesh` precedent that `MockHost`'s new shape mirrors).

## OrcaSlicer references

None. This packet does not borrow or check parity against any OrcaSlicer code. The test-support infrastructure being repaired is an internal SDK concern with no OrcaSlicer analog.

## Predecessor / successor relationships

- **Predecessors**: none (this is the first packet of the 77–80 sequence).
- **Successors**:
  - Packet 78 (TASK-225, TASK-226) — folds `slicer-test` into `slicer-sdk::test_support`, introduces `slicer_sdk::test_prelude`, deletes the `slicer-test` crate, migrates two exemplar core-modules. Hard-depends on this packet's `test_support` module and `MockHost` adapter shape.
  - Packet 79 — bulk migration of remaining core-modules + builder extension. Depends on packet 78.
  - Packet 80 — relocates 2 misplaced runtime tests. Depends on packet 79.

## Backlog sync status

`docs/07_implementation_status.md` rows for TASK-223 and TASK-224 were added with status `[ ]` during this packet's generation. They will transition to `[x]` with `Closed <date> — packet 77` suffix at the end of this packet's Acceptance Ceremony (see `implementation-plan.md` §Acceptance Ceremony).

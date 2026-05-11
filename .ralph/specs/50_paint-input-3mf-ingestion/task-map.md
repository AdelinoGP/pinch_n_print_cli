# Task Map: 50_paint-input-3mf-ingestion

This packet closes DEV-044 (registered 2026-05-10 by the spec-review of Packet 43-rev1) and introduces the new backlog task TASK-180 to `docs/07_implementation_status.md`.

## docs/07 → Packet Steps

| docs/07 Task ID | docs/07 Status (current) | Packet Steps | Closure Trigger |
| --- | --- | --- | --- |
| `TASK-180` ("Wire 3MF `fuzzy_skin_facets` paint metadata through the host loader so PaintSegmentation has a user-reachable input on the live binary path. Covers DEV-044.") | NEW — added at Step 7 with status `[~]` then closed `[x]` at packet completion | Steps 1-8 (all steps touch TASK-180) | All 8 ACs (6 positive + 2 negative — count includes AC-1 unit test, AC-2/3 E2E, AC-4 backward-compat, AC-5 regression battery, AC-6 docs/02, AC-7 clippy, AC-8 DEV-044/TASK-180 closure, NEG-1 malformed metadata, NEG-2 no-paint default) GREEN; DEV-044 row flipped to Closed in `docs/DEVIATION_LOG.md`. |

## Packet Steps → docs/07 Tasks (reverse map)

| Packet Step | Task IDs Touched | Notes |
| --- | --- | --- |
| 1 — Activation grounding | TASK-180 | Resolves Q1-Q4 open questions; flips packet `draft` → `active`. |
| 2 — Fixture authoring | TASK-180 | Commits `resources/benchy_painted.3mf` + README. |
| 3 — Decoder implementation | TASK-180 | Bounded edit to `crates/slicer-host/src/model_loader.rs`. |
| 4 — model_loader_tdd tests | TASK-180 | Three new tests (positive + 2 negative). |
| 5 — Flip E2E tests GREEN | TASK-180 | Pre-existing failing tests at `benchy_painted_e2e_tdd.rs` go GREEN with no test-file edits. |
| 6 — Regression battery | TASK-180 | Five Packet-43-rev1 regression commands all stay GREEN. |
| 7 — Docs + deviation closure | TASK-180 | `docs/02`, `docs/07`, `docs/DEVIATION_LOG.md`, `docs/14` edits. |
| 8 — Acceptance ceremony | TASK-180 | Re-run all AC commands; flip packet to `implemented`. |

## Aggregate

- 1 backlog task id closed (TASK-180 — new at packet start; closed at packet end).
- 1 deviation row closed (DEV-044).
- 1 doc cross-reference updated (`docs/14_deviation_audit_history.md` chronology).
- 0 prior packets superseded (this is a fresh packet, not a revision).

## Why this packet exists

DEV-044 was registered 2026-05-10 by a spec-review of Packet 43-rev1 (`43-rev1_macro-prepass-segmentation-output-drain`, closed 2026-05-08). The review identified that PaintSegmentation is contract-green at the WIT/IR layer but has no user-reachable input surface on the live binary path: `load_3mf` parses geometry only and discards every Bambu/Orca paint metadata namespace. This packet closes that gap for one channel (`fuzzy_skin_facets`); follow-up packets close the other three channels and TriangleSelector subdivision support.

This packet also unblocks DEV-045 closure (Packet 51 `paint-semantic-region-overrides`) by committing the `resources/benchy_painted.3mf` fixture that Packet 51's end-to-end test depends on.

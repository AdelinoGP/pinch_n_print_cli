# t185 gate pre-registration (written before any timing run)

Recorded 2026-11-23, before the t185 arms were built or measured. It adopts
t184's pooled design from the start, after t184's window 1 showed single-window
null bands wide enough to flip a cell's verdict on this box.

## Design

- **Arms.**
  - Baseline: HEAD `5761e533` (t184 landed).
  - Candidate: the t185 tree.
  - Both ordinary and accelerated dists. Guest module trees must be identical
    between arms, since t185 edits host crates only.
- **Output.** Every run fails closed against t182's active set
  (`../t182-spatial-hierarchical-phase3-union/reference.json`), 16/16 output runs
  included.
- **Ordinary windows.** Two complete windows, all 8 cells, 5 interleaved pairs
  each (`ab`, `ab2`), with a 3-pair null control after each (`null`, `null2`).
- **Accelerated corroboration.** 3 pairs (`accel`).
- **Instrumented tail.** 3 pairs (`stage`), attribution only.

## Verdict of record

The **pooled** windows (10 ordinary pairs, 6 null pairs per cell):

- **The four supports-on cells:** standing t137 planner gate. The median paired
  wall saving must be ≥ `0.80 %` and strictly above the pooled null median wall,
  with the process CPU median lower. Accelerated must corroborate (wall
  ≥ `0.80 %`, CPU lower).
- **The four supports-off cells:** structurally inert. They must not regress:
  pooled median wall saving ≥ `−0.80 %`.

Per-window verdicts and the strict any-pair null lens are disclosed, not gated.
No further windows are added to change the verdict. A pooled failure is
recorded and left for a human ruling.

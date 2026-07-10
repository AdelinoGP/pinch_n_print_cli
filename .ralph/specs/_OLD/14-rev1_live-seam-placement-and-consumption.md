---
status: superseded
packet: 14-rev1_live-seam-placement-and-consumption
task_ids:
  - TASK-120c
  - TASK-151
supersedes: 14_live-seam-placement-and-consumption
superseded_by: 22_live-seam-contract-repair
---

# 14-rev1_live-seam-placement-and-consumption

## Goal

Implement seam-started wall loops so that `resolved_seam` is a **directive** — the wall loop's `path.points` sequence must start at the seam vertex. `Layer::PerimetersPostProcess` (seam-placer) emits rotated wall loops via a new `push-reordered-wall-loop` WIT method on `perimeter-output-builder`. `Layer::PathOptimization` reads already-rotated wall loops from `PerimeterIR` and emits only a per-layer marker comment — no replay needed.

**Seam-first contract:** Printing a layer begins at the seam and ends at the seam. The `WallLoop.path.points[0]` is the first vertex emitted, and the last emitted vertex joins cleanly to `points[0]` to close the loop.

## Problem Statement

The original packet `14` attempted to implement seam-started wall loop replay via `push-move` at `Layer::PathOptimization`. This approach has three critical failures:

1. **`push-move` is rejected at `Layer::PathOptimization`** per docs/03 § Path Optimization Output Contract — the host was silently accepting Move commands instead of rejecting them with a fatal diagnostic
2. **E coordinate uses local extrusion width** — `cmd.e` was set to `pt.width` (per-vertex width in mm), not cumulative extrusion amount, producing semantically incorrect GCode
3. **Moves committed as Raw G1 annotations** — not as proper `ordered_entities` in `LayerCollectionIR`

The documentation in `docs/02_ir_schemas.md` is ambiguous on whether `WallLoop.path.points[0]` must be the seam vertex. The current `SeamPosition { point, wall_index }` description in the schema reads as a reference annotation, not a geometry directive.

This packet resolves the ambiguity: **`resolved_seam` is a directive** — the wall loop must be stored with `path.points[0]` as the seam vertex. Printing begins at the seam and ends at the seam.

## Architecture Constraints

- `Layer::PerimetersPostProcess` is the stage that owns wall loop geometry modification. Seam placement writes `resolved_seam` (a reference) AND rotates the wall loop geometry so `path.points[0]` is the seam point.
- `Layer::PathOptimization` must NOT call `push-move` — that method is rejected at that stage per docs/03 contract. The original packet `14` violated this. This revision fixes it.
- `PerimeterIR` is the canonical store of seam-first wall loops. No replay happens at PathOptimization.
- The `feature_flags` and `width_profile` on `WallLoop` must remain parallel to `path.points` after rotation — host validation enforces this at the WIT boundary.

## Data and Contract Notes

**IR contracts touched:**
- `PerimeterIR.regions[*].walls[*].path.points` — rotated so `points[0]` is seam vertex
- `PerimeterIR.regions[*].walls[*].feature_flags` — re-indexed to match rotated points
- `PerimeterIR.regions[*].walls[*].width_profile.widths` — must have same cardinality as rotated `points`
- `PerimeterIR.regions[*].resolved_seam` — still written as `Some(SeamPosition)` for diagnostic reads

**WIT boundary considerations:**
- `push-reordered-wall-loop` receives `rotated-wall-loop: wall-loop-view` — the host validates that `feature-flags.len() == path.points.len()` before accepting
- The seam position (`pos: point3-with-width`) and wall index are passed separately so the host can validate the Z envelope and write the `SeamPosition` reference

**Determinism or scheduler constraints:**
- Wall loop rotation must be deterministic: the same `resolved_seam` applied to the same wall loop geometry must produce byte-identical rotated `path.points` across repeated runs
- The rotation algorithm: given `seam_point` and `wall_loop.points`, find `seam_idx` in points (by coordinate match), then emit `points[seam_idx], points[seam_idx+1], ..., points[end], points[0], ..., points[seam_idx-1]`

## Locked Assumptions and Invariants

- **Seam-first invariant:** After `Layer::PerimetersPostProcess` completes, `PerimeterIR.regions[R].walls[W].path.points[0]` is the first vertex of the seam-started wall loop. Downstream stages (Infill, PathOptimization, GCodeEmit) can assume this without re-checking.
- **Parallel cardinality invariant:** `WallLoop.path.points.len() == WallLoop.feature_flags.len() == WallLoop.width_profile.widths.len()`. This is enforced at the WIT boundary on `push-reordered-wall-loop`.
- **Loop closure invariant:** The last emitted point of a rotated wall loop must join cleanly to `points[0]`. The original wall loop is closed; rotation preserves the closed geometry.

## Risks and Tradeoffs

- **Risk:** Rotating wall loop geometry changes the semantic of what `seam-placer` writes. The manifest currently claims `writes = ["PerimeterIR.resolved-seam"]`. After this packet, it must claim write access to the wall loop fields it rotates.
  - **Mitigation:** Update `seam-placer.toml` `writes` to include `PerimeterIR.regions.walls` (the full wall path and feature_flags).
- **Risk:** If downstream code (GCodeEmit, Infill) reads `PerimeterIR.walls[].path` and assumes original ordering, rotating the wall loop would break that code.
  - **Mitigation:** The seam-first invariant is the documented contract. All downstream stages must respect it. If any downstream consumer is reading wall geometry without understanding the seam-first invariant, that is a bug in the consumer.
- **Risk:** Removing `push-move` support at PathOptimization breaks the original packet `14` tests that verified `Move` emission.
  - **Mitigation:** Those tests are now superseded. This packet provides correct tests for the Option B approach.

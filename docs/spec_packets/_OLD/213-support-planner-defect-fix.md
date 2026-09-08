---
status: superseded
packet: 213-support-planner-defect-fix
task_ids:
  - TASK-329
superseded_by: support-families-and-anchored-entities sequence (packets 219-224); lone-node/radius-floor work retained only where it survives inside the new tree family (221-tree-support-family)
---

# 213-support-planner-defect-fix

## Goal

Make support-planner propagation emit a printable vertical segment for every surviving lone node and keep every tapered branch radius at or above `MIN_BRANCH_RADIUS = 0.4` while retaining the `MAX_BRANCH_RADIUS_MM = 6.0` ceiling.

## Problem Statement

After propagated nodes merge, the surviving lone node has no MST edge and `dist_to_top > 0`, so the planner emits no segment and the support column ends in mid-air. Independently, `tapered_radius` returns zero at the contact tip. This coherent planner slice restores plate-reaching geometry and a printable minimum tip width.

## Architecture Constraints

- A lone propagated node is emitted only when it survives `drop`, has `dist_to_top > 0`, has no surviving MST edge, and passes the existing collision policy; use a degenerate two-point segment at the current layer Z.
- `MAX_BRANCH_RADIUS_MM` remains `6.0`; the new floor is `MIN_BRANCH_RADIUS = 0.4`.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR/manifest contracts: no changes; `branch_segments` remains `Vec<Vec<Point3WithWidth>>`.
- WIT boundary: unchanged; guest module source changes require guest rebuild.
- Determinism/scheduler constraints: preserve active-node iteration order, `drop` decisions, and existing MST ordering.

## Locked Assumptions and Invariants

- A segment's two points are equal in XY and Z for a lone node, with width `tapered_radius(...) * 2.0`.
- Dropped nodes and collision-rejected nodes never emit.
- Radius output is always within `[0.4, 6.0]`.

## Risks and Tradeoffs

- A floor may make contact tips wider than the prior zero-width output; this is intentional for renderability and extrusion validity.
- Visual evidence for `PrePass::SupportGeometry` may have `typed_capture: null`; PNGs and `Layer::Support` typed captures remain the evidence path.

# Tiled collision operand for the planner's phase-1 carve

Type: task / measured candidate (output-identical work-set change + standing
gate)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t173 support-path boolean volume](173-support-geometry-boolean-volume.md)
([`evidence/t173-support-boolean-volume/FINDINGS.md`](../evidence/t173-support-boolean-volume/FINDINGS.md)),
which censused the support path's clipper work and priced the phase-1 bbox/tile
window as **60.8 % / 61.6 %** of the two base supports-on cells' required cuts.

## Question

**Does an exact tiled/windowed collision operand on
`build_roles_batched`'s phase-1 carve clear the standing gate — and what wall
does it actually save against t173's 60.8 % / 61.6 % ceiling?**

t173 measured the shape, not the implementation: phase 1 issues **68,381
calls**, each subtracting the whole layer collision set (**426.9 M clip
points**, mean **6,241/call**, 4.34 paths/call) from a **98.7-point** subject;
only **285,368** clip points (**0.0669 %**) lie inside the subject's bbox. A
polygon-level bbox filter is exact but removes only **28.5 %** of the operand.
The question is whether cutting the collision set into tiles once per layer,
then clipping each region against only the tiles its bbox touches, realizes
enough of that ceiling to matter.

## What is already known (t173)

- **Exactness is by construction.** For a difference, clip geometry whose bbox
  is disjoint from the subject bbox cannot change the result; a tile set whose
  union covers the subject bbox is therefore output-identical, with no parity
  exception. The tiling cost (partition per layer, boundary edges) is the
  unmeasured term.
- **The site is `planner::batched_phase1_carve`** in
  `modules/core-modules/tree-support-planner/src/lib.rs`
  (`build_roles_batched` phase 1 → `run_clip_batches`). 68,381 requests on both
  base supports-on cells; phase 3 (`batched_phase3_union`, 480 calls) is a
  separate shape and is **not** part of this ticket.
- **The ceiling leaves 1.150x / 1.120x.** The phase-1 window alone cannot win
  either base cell; it is a component. The standing gate is the ticket's bar,
  not the 8/8 destination.
- **The benchy supports-on pair is not this program.** Its whole planner clip
  wall is 0.73 / 0.72 s against cuts of 2.08 / 2.15 s; the tile shape covers
  only 23.2 % / 21.8 % of those cuts. Do not expect a benchy win here.

## Scope

1. Implement a per-layer tile index for the collision set used by
   `build_roles_batched` phase 1. Sweep a small number of tile sizes (e.g.
   1 / 2 / 5 / 10 mm) on `base classic-on`, choose by measured wall, and record
   the sweep. The per-region operand is the union of tiles overlapping the
   region bbox; the request order and every other phase stay unchanged.
2. Prove output identity on all eight cells against t172's active reference set
   (the tile union must cover the subject bbox exactly; no tolerance).
3. Run the standing gate: four supports-on cells positive 0.80 % paired
   wall/CPU improvement, eight cells byte-identical, four supports-off cells
   with no median wall/CPU regression (t137's generalized form).
4. Record the measured tile-partition cost and the realized operand reduction
   per call, so the 60.8 % / 61.6 % ceiling is replaced by a measured number.

## Acceptance

- `evidence/t175-tiled-collision-carve/` with the tile-size sweep, the gate
  campaign, byte-identity proof, `FINDINGS.md`, and a checkout-only
  `verify.py --self-test`.
- Map: destination-status block, Decisions-so-far line, and an updated
  **Not yet specified** (the measured phase-1 win, and the phase-3 companion
  still unnamed).
- No production edit until the output-identity check and the gate pass; the
  change is a work-set reduction, not a geometry change, so it does not need a
  new t81 ruling. The t111/t128 prepass exception is unchanged.

## Answer

_(pending)_

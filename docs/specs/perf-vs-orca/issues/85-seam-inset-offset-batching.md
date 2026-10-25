# Seam planner inset-offset batching

Type: task
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Base common-floor attribution](84-base-common-floor-attribution.md)

## Question

Can the seam planner's 495 per-region inset offsets be issued as **one**
`offset_polygons_batch` call — with per-request exactness and entry order
preserved — and how much base-cell wall does the recovered parallelism deliver?

## Measured basis

[ticket 84](84-base-common-floor-attribution.md) decomposed
`PrePass::SeamPlanning` on the two base classic cells. The stage wall is
**92.4 / 93.2 %** one serial native call per region:

| term | classic-off | classic-on |
| --- | ---: | ---: |
| stage wall | 9.14 s (9.84 % of cell) | 8.97 s (4.30 % of cell) |
| scope `t84.offset_project` | **8,195 ms (92.4 %)** | **8,177 ms (93.2 %)** |
| candidate generation + scoring + harvest | 169 ms | 155 ms |
| host view projection | 4 ms | 11 ms |
| host harvest | 58 ms | 45 ms |

495 regions × one `host::offset_polygons` call at
`delta = -0.5 × chosen_width` (Miter, arc tolerance 0.0) in
`run_region_planning_entries`
(`modules/core-modules/seam-planner-default/src/lib.rs`). Each call is
independent: it insets one region's polygons so the chosen boundary vertex can
be projected onto the toolpath centreline
(`project_point_onto_inset_boundary`, `seam-planner-default/src/contours.rs`).

`offset_polygons_batch` already exists (`crates/slicer-wasm-host/src/host.rs`,
`crates/slicer-sdk/src/host_batch.rs`) and routes through the same
input-ordered, work-estimating `map_batch` fan-out t78 used for
`clip_polygons_batch`. No host capability work is required.

Measured ceiling (ticket 84's `summary.json`):

| cell | upper bound (all offset wall removed) | at t78's 2.6x effective |
| --- | ---: | ---: |
| base classic supports-off | 8.83 % of wall | 5.43 % |
| base classic supports-on | 3.92 % of wall | 2.41 % |

Both are far above the map's 0.80 % floor. The ceiling is **not** claimed as the
expected result: the candidate must measure its own batch occupancy, and a
`map_batch` effective speedup below **1.15x** on the seam request set is a
pre-declared DROP.

## Scope

Guest-side restructure only:

1. Pass A: for every admitted region, in the existing sorted/deduped order,
   compute the chosen candidate and record `(region, chosen_position)` plus one
   `OffsetRequest` (exact same polygons, delta, join, arc tolerance, no miter
   limit).
2. One `host::offset_polygons_batch` call for the whole print.
3. Pass B: project each chosen position onto its own returned inset and push the
   entry — same entry fields, same order, same fallback path.

No `SeamPlanIR` shape change, no seam-selection change, no host change. The
candidate must keep the zero-candidate fallback (`fallback_region_position`)
and the paint-enforcement scoring untouched.

## Pre-declared gate

- **Output identity (16/16)**: every cell of {benchy, base} × {classic, arachne}
  × {supports-off, on} byte-identical to its current reference SHA (t81's
  campaign set; the classic base refs are `292df501…` / `22200671…`).
  Determinism: one SHA per arm across repeats.
- **Standing paired A/B** on the two base classic cells, ordinary +
  accelerated, 6 alternating pairs each: every retained pair byte-identical,
  median wall must fall, and process CPU must not rise above **+3.0 %** on any
  cell (work-preserving ceiling, the t78 precedent).
- Disclose the measured batch occupancy (effective speedup) and the residual
  serial terms (candidates, choose, harvest, projection).

Out-of-scope for this ticket: reducing the candidate list, changing the inset
delta or the projection rule, and any geometry change. Those are separate
questions (ticket 84 measured the candidate list at 0.29 % of wall; a
representation-safe projection change is not priced).

## Answer

_(pending)_

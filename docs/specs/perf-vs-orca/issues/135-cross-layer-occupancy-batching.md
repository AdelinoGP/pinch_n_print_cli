# Cross-layer occupancy offset batching: run the emit-check offsets through the planner's clip fan-out

Type: task / BUILD candidate
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: [137](137-planner-scheduling-gate.md) (the four-cell gate form must be confirmed)
Graduated from: [Planner effective parallelism](134-planner-effective-parallelism.md)'s
  2026-11-09 ruling: the per-layer occupancy offsets are the largest serial term left in
  the planner (**14,249 ms wall / 14,169 ms thread** on base classic-on, **0.99x**), while
  the clip core already runs at **4.81x**. The derived ceiling is **11.2 s** at that fan-out.

## Question

[Planner effective parallelism](134-planner-effective-parallelism.md) measured the
planner's remaining serial term: `inflate_model_occupancy_memoized` still issues one
singular `host::offset_polygons` per active layer for the emit check (**433 calls /
14,169 ms thread / 14,249 ms wall** on base classic-on; **186 / 585 ms / 598 ms** on
benchy classic-on). The calls are independent across layers, and the emit pass already
buffers four layers for the clip batch (`EMIT_BATCH_LAYERS`). Batch the per-layer
occupancy payloads across the buffered layers with the same `offset_polygons_batch`
service the role carve uses, then serve both memo sites from the batch result.

The measured ceiling is **11.2 s** at the planner clip core's **4.81x** fan-out
(**8.7 s** at t78's 2.6x clamp, **13.0 s** at 12x), i.e. **11.1x** the 1,010 ms floor
and **12.1 %** of base classic-on's 92.5 s required prepass cut. It is a tranche, not
the route: the planner's whole scheduling potential is below the 50.66 s share t134
priced as the route threshold.

## Why this is not t125

[Ticket 125](125-strong-planner-occupancy-offset-batching.md) batched the **per-origin
role-carve** offsets inside the bounded flush and failed the gate in six of eight cells,
with base classic-on gaining wall but regressing process CPU. This shape batches the
**per-layer emit-check** payloads (the 433 unique offsets t132 did not memoise) across
the layer chunk, and leaves the per-origin carve exactly where t132 left it. t125's
failure is the control: the candidate must report the per-call wall and CPU of both
halves and must not silently move the role-carve half into the batch.

## Pre-declared gate

The standing paired wall/CPU gate in the **t117 four-cell form**, because the four
supports-off cells are structurally inert: the whole `PrePass::SupportGeometry` stage is
**0.64–0.81x** the 0.80 % floor there (t134 measured), so no planner-only change can pass
the literal all-8-cell form.

- Byte-identical G-code in all eight cells on both build arms.
- Positive median wall with CPU corroborating at 0.80 % in all four supports-on cells.
- No median wall or CPU regression in the four supports-off cells.
- A 16-cell output campaign before timing.

## Acceptance

- Candidate build and a fresh 16-cell output campaign; every hash equals ticket 123's
  candidate-ordinary reference set.
- Four supports-on paired cells at or above the floor, CPU corroborating; four
  supports-off cells regression-free.
- A measured conversion of the occupancy thread-time to wall on the candidate arm, so
  the 4.81x proxy is replaced by the batch's own fan-out.

## Answer

_To be recorded on resolution._

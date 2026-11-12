# Union-phase request grouping: batch the planner's union requests across four chunk-batches

Type: task / measured candidate
Label: wayfinder:task
Status: open (unclaimed)
Blocked by: none ([137](137-planner-scheduling-gate.md) resolved 2026-11-12 —
  the standing four-cell gate form is confirmed; [136](136-planner-clip-batch-fanout.md)
  resolved 2026-11-12 with the measured ceiling)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Graduated from: [Planner clip-batch fan-out](136-planner-clip-batch-fanout.md)'s
  2026-11-12 Phase-1: the planner's clip core is **4.99x** end-to-end, and the
  deficit is entirely `union_run` — **10,176.2 ms** of the 13,956.4 ms host clip
  wall on base classic-on at **3.66x**, with **4.4 requests per call** and a
  wall equal to the sum of each call's longest request (10,168.3 of
  10,176.2 ms). `carve_run`, the phase the batch API was built for, is at
  **11.56x** with the 256-request cap saturated. Merging the union phase across
  four chunk-batches takes its measured floor from 10,176.2 ms to
  **3,465.0 ms**; LPT's 4/3 bound puts the achievable saving at **5.6–6.7 s**
  (**7.3 %** of base classic-on's 92.5 s required prepass cut, **6.6x** the
  1,010 ms floor).

## Question

Group the planner's union requests across four `EMIT_BATCH_LAYERS` chunks (16
layers) instead of issuing them one chunk at a time, and measure whether it
lands inside the t136 bracket.

The requests are already independent and already carry the same arguments; only
the call grouping changes, so exactness is by construction. Two forms, in
order:

1. **Raise `EMIT_BATCH_LAYERS` 4 → 16.** One-line shape, but it also quadruples
   the buffered collision sets, and the constant is documented as a memory
   bound. Measure peak working set at 4 / 8 / 16 layers before claiming
   anything.
2. **Union-only accumulator.** Defer just the union requests across four chunk
   batches, holding carved geometry rather than collision sets. More code, no
   memory cliff; price it first if form 1 moves peak RSS materially.

Do not widen scope: `set_run` and `body_cut` carry the same width deficit at
1.5 % of the clip core and are fixed by the same change, but the carve phase is
already at 11.56x and must not be touched.

## Pre-declared gate

The candidate is planner-only (`PrePass::SupportGeometry` /
`com.core.tree-support-planner`) and the supports-off cells return before
`plan_for_object`, so the literal all-8-cell 0.80 % gate is structurally
unreachable. It is judged under the standing
[t137](137-planner-scheduling-gate.md) four-cell form:

- positive 0.80 % paired wall/CPU improvement in the four supports-on cells;
- byte-identical output in all eight against the active post-[t139](139-layer-plane-parity.md)
  reference set;
- no median wall/CPU regression in the four supports-off cells.

The measured ceiling is **6,711 ms** (merge-4 floor, union wall 10,176.2 →
3,465.0 ms) and LPT's 4/3 bound puts the achievable saving at
**5,556–6,711 ms** (union wall 3,465–4,620 ms). A candidate that does not
**save at least 3,500 ms** on base classic-on has not reached the priced band
and is a DROP on price. Disclose peak working set and time-to-first-layer at
every buffering depth tried.

## Acceptance

- A measured candidate with the 8-cell output check and the four-cell paired
  gate, on base classic-on and benchy classic-on at minimum.
- The buffering-depth table (4 / 8 / 16 layers): union wall, clip-core wall,
  peak working set.
- A recorded KEEP / DROP with the achieved saving as a share of base
  classic-on's 92.5 s required cut.
- Probe reverted; production edit only if KEEP.

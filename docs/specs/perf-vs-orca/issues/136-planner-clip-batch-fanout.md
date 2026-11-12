# Planner clip-batch fan-out: why is the clip core at 4.81x and what scheduling change reaches 12x?

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none ([137](137-planner-scheduling-gate.md) resolved 2026-11-12 —
  the standing four-cell gate form is confirmed)
Graduated from: [Planner effective parallelism](134-planner-effective-parallelism.md)'s
  2026-11-09 ruling: the clip core is **85,641 ms thread / 17,811 ms wall = 4.81x** on
  base classic-on, against the 12-thread ideal. Raising it to 12x is a derived **10.7 s**
  wall ceiling, independent of the occupancy tranche.

## Question

The planner's emit-pass clip core (`build_roles_batched` via `run_clip_batches`) fans
out **70,210 clip calls / 270.6 M input points** over **559 parallel batch calls**, yet
delivers only **4.81x** on base classic-on (**4.52x** on benchy classic-on), while the
shell stage reaches **9.26x** and overhang **11.72x** on the same host and run. Which
part of the 4.81x is batch-load imbalance, which is the per-batch serial slice, and
which is the guest-side assembly — and what dependency-preserving scheduling change
recovers the difference without changing the op set or operand size?

Candidate mechanisms to price, in order:

1. **Chunk size.** `EMIT_BATCH_LAYERS = 4` and `CLIP_BATCH_MAX_REQUESTS = 256` bound
   each batch. Measure the per-batch sweep distribution and the tail request share;
   larger or work-balanced chunks may raise the fan-out.
2. **Tail imbalance.** The probe's largest single planner clip sweep is **655 ms**
   and the largest single operand **169,716 points**, against a 3,854-point mean
   on the emit core. Measure how much of each batch's wall is its largest request;
   if the tail dominates, a representation-preserving request partition may help,
   but that is a work-set change and needs its own ruling.
3. **Guest assembly.** `build_roles_batched` does the role union, per-region carve,
   set-wide difference and territory intersect as separate `run_clip_batches` rounds.
   Measure the serial guest time between rounds; if it dominates, the round count is
   the lever, not the fan-out.

## Pre-declared gate

No production edit in this take. Any candidate it graduates must be byte-identical in
all eight cells against the active post-[t139](139-layer-plane-parity.md) reference set
(a scheduling change is output-neutral; if it is not, it is not this shape) and must
clear the standing [t137](137-planner-scheduling-gate.md) four-cell form in the four
supports-on cells with no median wall/CPU regression in the four supports-off cells. The
literal all-8-cell 0.80 % gate is structurally unreachable for a planner-only change.

## Acceptance

- A measured decomposition of the 4.81x into chunk size, tail share and guest assembly
  on base classic-on and benchy classic-on.
- A ranked list of dependency-preserving scheduling variants, each with its measured or
  derived ceiling and floor multiple; no production edit.
- A recorded go/no-go on a candidate build.

## Answer

_To be recorded on resolution._

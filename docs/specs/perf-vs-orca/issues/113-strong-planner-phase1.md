# Strong support planner Phase 1: sub-term decomposition and a representation-safe shape

Type: task
Label: wayfinder:task
Status: open
Assignee: (unclaimed)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 112
Graduated from: [ticket 111](111-prepass-rewrite-scope-ruling.md)'s ruling
(Q3, 2026-11-03: the Strong planner is the second campaign, after
`PrePass::Slice`).

## Question

Where does the **87,824 ms** `com.core.tree-support-planner` module stage go on
base classic-on (**2,943 ms** on benchy classic-on), and is any of it a
representation-safe shape that clears the **0.80 %** floor on all 8 cells?

This is the second campaign under ticket 111's scoped parity exception. The
planner may still turn out representation-safe, in which case it needs no
exception; if it needs canonical parity, the exception already covers it.

## Context

- [Ticket 107](107-support-geometry-producer-decomposition.md): the
  `PrePass::SupportGeometry` stage id is shared by a **9 ms** host producer and
  the planner module. The planner is **95.6 %** of the **91,893 ms** stage and
  **38.4 %** of the base classic-on cell; the producer is priced dead at
  **0.0068x** the floor.
- [Ticket 88](88-supports-on-increment-attribution.md): the supports-on increment
  is **~69 % support geometry / ~31 % host**; the planner is **52 %** of the
  classic increment, and its **55.5 s** attribution refreshes to **64.5 s**
  implied under t104's share arithmetic.
- [Ticket 78](78-tree-support-planner-emit-batching.md) took the batched clip core
  from **79.7 s** to **~30.7 s** (2.6x effective). The residual named by t88 is
  the **serial avoidance ladder, offsets, MST and node moves** — support geometry,
  not a host batch shape. No representation-safe shape was found in t107's take;
  re-enter only with a measured sub-term and a representation argument.
- [Ticket 111](111-prepass-rewrite-scope-ruling.md): candidate acceptance is the
  fresh **16-cell** reference set, arms byte-identical within themselves, the
  candidate-vs-old-reference delta audited before timing, and the standing paired
  wall/CPU gate at the 0.80 % floor in **all 8 cells**.

## Scope

Phase-1 decomposition and shape search only: no production edit, no candidate, no
timing campaign, no reference-set generation.

1. Re-derive the planner's current sub-term split at HEAD from the existing
   profile and evidence: the clip core, the avoidance ladder, offsets, MST, node
   moves, marshalling, and any remaining host work. Separate module time from
   host time.
2. Identify the largest sub-terms and test each against a representation-safety
   argument: does the shape change support geometry, or only how the same geometry
   is computed? A parity shape is allowed under t111, but it must still be named
   and its output delta bounded.
3. For each viable shape, pre-declare the expected wall ceiling on the gated
   cells and the output delta; recommend **build** or **stop**.

## Deliverable

A resolution with the fresh sub-term split, the candidate shapes (representation
argument, ceiling, cells moved, risk), and a recommendation. If stop, the ceiling
against the 0.80 % floor on all 8 cells and which cells fail.

## Stop rule

If no shape clears **0.80 %** on **all 8 cells**, or the output delta cannot be
bounded, record the ceiling and stop: no candidate ticket, no production edit.
The planner's share of the supports-on increment is large enough that a
below-floor ceiling is a real result, not a failure to try.

## Acceptance

The analysis is a resolution with the evidence attached; no production edit and no
timing campaign. If a shape is recommended, the next ticket is a candidate
implementation under ticket 111's acceptance contract.

## Evidence starts

- [`evidence/t107-support-geometry-producer/`](../evidence/t107-support-geometry-producer/FINDINGS.md)
  (stage/producer split and the producer's ceiling).
- [`evidence/t88-supports-increment/`](../evidence/t88-supports-increment/FINDINGS.md)
  (the 55.5 s planner attribution and the sub-term shape).
- [`evidence/t78-planner-emit-batching/`](../evidence/t78-planner-emit-batching/FINDINGS.md)
  (the clip-core batching and its measured limit).
- [`evidence/t104-phase-split/phase-split.json`](../evidence/t104-phase-split/phase-split.json)
  (the `PrePass::SupportGeometry` stage wall and share).
- `modules/core-modules/tree-support-planner/src/lib.rs`
  (`com.core.tree-support-planner`).

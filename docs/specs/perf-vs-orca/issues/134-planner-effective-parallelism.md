# Planner effective parallelism: 2.19x against 8.3–11.8x on the other prepass stages

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Common-stage per-op cost decomposition](131-common-stage-per-op-cost-decomposition.md)'s
  2026-11-09 scheduling census: the planner carries 122,657.6 ms of Clipper2
  sweep thread-time on base classic-on and delivers it in 56,289 ms of wall
  (**2.19x**), against 8.34–11.80x on the other heavy prepass stages.

## Question

Why does `PrePass::SupportGeometry`'s planner convert 122.7 s of sweep
thread-time into 56.3 s of wall when the shell stage converts 118.7 s into
14.3 s, and what dependency-preserving restructure recovers the difference?

[t131](../evidence/t131-per-op-decomposition/FINDINGS.md) measured the
effective parallelism of every prepass stage on the four classic cells:
planner **2.19x** (base-on) and **1.68x** (benchy-on); shell **8.34x / 8.78x**;
overhang **11.80x / 11.74x**; support analysis **11.38x**; slice **6.75x /
6.54x**; `MeshAnalysis` **1.03x** (serial, but only 4.6 s of thread-time).
At shell-level parallelism the planner's 122.7 s would be **14.0 s**; even at
4x it would be **30.7 s**, a **25.6 s** wall cut — **27.7 %** of base-on's
92.5 s required prepass cut, larger than S5 and S6 combined on that cell.

The planner's own phase-1 decomposition ([t113](113-strong-planner-phase1.md))
found the occupancy inflation (56.2 %), clip core (31.0 %), drop loop (6.2 %)
and contacts (2.4 %) on base classic-on; t113's batch candidate
([t125](125-strong-planner-occupancy-offset-batching.md)) failed the gate for
unrelated reasons (per-origin batching inside a bounded flush), so the
dependency structure has not been re-derived since.

## Scope

1. Attribute the planner's wall: which sub-phase is serial (drop loop? node
   smoothing? contact detection? the emit check?), and what fraction of the
   122.7 s sits behind a per-layer barrier.
2. Price the ceiling for a dependency-preserving restructure: layer pipelining,
   per-object parallelism, or batching the per-node ops without changing the
   op set. Keep the op count and operand size fixed so this stays a scheduling
   shape, not a geometry one.
3. Separate the *guest module's* structure (tree-support-planner) from the
   host's `SupportGeometry` producer stage and the `SupportAnalysis` stage —
   t131's stage bracket shares one stage id across two `stage_complete`
   events.
4. No candidate build in this take; a re-measured stage parallelism census on
   the four supports-on cells is the deliverable.

## Pre-declared gate

Any restructure candidate clears the standing paired wall/CPU gate at 0.80 %
in all eight cells with byte-identical output (a scheduling change must be
output-neutral; if it is not, it is not this shape). The measured ceiling this
take graduates is **25,625 ms** on base classic-on at 4x — **25.4x** the
1,010 ms floor, **27.7 %** of the cell's required prepass cut. A restructure
whose measured ceiling is below the planner's own 50,660 ms share of that cut
does not reach the destination on its own; it is a tranche, not the route.

## Acceptance

- A measured sub-phase wall attribution for the planner on base classic-on and
  benchy classic-on, with the serial fraction named.
- A ranked list of dependency-preserving restructures, each with its measured
  or derived ceiling and floor multiple.
- A recorded go/no-go on a planner-scheduling candidate; no production edit.

## Answer

_To be recorded on resolution._

# Planner effective parallelism: 2.19x against 8.3–11.8x on the other prepass stages

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: closed (resolved 2026-11-09, session 37)
Assignee: dev (session 37)
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

**Resolved 2026-11-09: NO-GO as the route, GO (narrowed) for two supports-on
tranches. The planner's serial wall is the per-layer occupancy offset, not the
clip fan-out.**

The stage-level effective parallelism on the four supports-on cells is
**2.64x / 2.66x / 2.09x / 2.11x** (base classic-on / base arachne-on / benchy
classic-on / benchy arachne-on); the planner module alone is **2.76x / 2.79x /
2.31x / 2.32x**. That is up from t131's **2.19x** because t132's exact memo
removed **14.2 s** of *serial* duplicate occupancy work; the clip core's own
fan-out did not move. The other heavy stages on the same runs are shell
**9.26x**, overhang **11.72x**, SupportAnalysis **11.46x**, Slice **6.46x**,
SeamPlanning **7.52x** (base classic-on), so the planner is the least parallel
heavy stage, but the cause is not a broken batch.

**Sub-phase split (base classic-on; benchy classic-on in parentheses):** the
planner module is **37,639 ms wall / 103,972 ms clipper thread = 2.76x**
(1,993 / 4,602 = 2.31x). The emit pass carries **96.3 %** of thread-time
(91.1 %) and **88.6 %** of wall (73.9 %); the recurrence-bound drop loop is
**2.5 %** (5.3 %) of thread and **6.4 %** (12.4 %) of wall; the pre-loop is
**1.2 %** (3.6 %). Inside the emit pass:

- `p6d_occupancy` — **14,249 ms wall / 14,169 ms thread = 0.99x** (598 / 585 =
  0.98x), **37.8 %** of base planner wall: 433 unique per-layer singular
  offsets. t132's memo already serves the byte-equal second ask; these are the
  distinct payloads.
- `p6a_clip_core` — **17,811 ms wall / 85,641 ms thread = 4.81x** (782 /
  3,534 = 4.52x): real batch fan-out, not 12x, and 47.3 % of base planner wall.
- `p6e_node_roles` 989 ms, `p2_contacts` 1,110 ms, `p6f_emit_geometry` 284 ms,
  `p5_smooth` 75 ms, `p0_volumes` 174 ms — guest CPU or small mixed phases.

**The 122.7 s question:** the current stage carries **105.1 s** of clipper
thread-time. **100.1 s (95.3 %)** is in the layer-independent emit pass and
**2.6 s (2.5 %)** is in the recurrence-bound drop loop. The emit pass has no
cross-layer dependency; it is serialised only by the guest issuing one layer's
work at a time. The drop loop is the only true per-layer barrier, and it is
6.4 % of the wall. The barrier is not what holds the planner at 2.6x; the
serial occupancy phase inside a layer-independent loop is.

**Ranked dependency-preserving restructures (base classic-on; floor 1,010 ms;
required prepass cut 92.5 s):**

1. **Cross-layer occupancy batch** — batch the 433 emit-check offsets across
   the buffered layers with `offset_polygons_batch`. Ceiling **11.2 s** at the
   clip core's measured 4.81x (8.7 s at t78's 2.6x clamp, 13.0 s at 12x) =
   **11.1x** floor = **12.1 %** of the required cut. Exact by construction;
   graduated as [t135](135-cross-layer-occupancy-batching.md). t125 is the
   control: batch the emit-check half, not the per-origin carve inside the
   flush.
2. **Clip-core fan-out to 12x** — chunk/load-balance `build_roles_batched`
   without changing the request set. Ceiling **10.7 s** = **10.6x** floor =
   **11.5 %** of the cut. Diagnosis first; graduated as
   [t136](136-planner-clip-batch-fanout.md).
3. **Combined** — **21.9 s = 21.7x** floor = **23.7 %** of the cut. Still a
   tranche.
4. **Drop-loop serial reduction** — 2.4 s, recurrence-bound and not
   pipelineable. Leave.
5. **Guest-CPU phases** — 2.5 s total, not a host-batch shape. Fog.
6. **Per-object parallelism** — one object in every matched cell; no ceiling
   in the 8 cells.

**Gate — the literal all-8-cell form is structurally unreachable.** On the
supports-off cells the whole `PrePass::SupportGeometry` stage is **266 / 211 /
37 / 35 ms** = **0.78x / 0.64x / 0.80x / 0.81x** the 0.80 % floor (base
classic-off / base arachne-off / benchy classic-off / benchy arachne-off), and
the planner module alone is **0.32–0.43x**. Even deleting the entire stage
fails base classic-off and only ties benchy. A planner-only candidate can only
be judged under t117's four-cell form; t117 scoped that amendment to its own
candidate, so the human must confirm it for t135/t136 or pair them with a
supports-off leg.

**Route arithmetic:** the two tranches together are **23.7 %** of base-on's
required prepass cut, below this ticket's **50.66 s** threshold. Even a
**zero-cost planner** leaves **3.25x** on base classic-on, **3.21x** on base
arachne-on, **2.89x** on benchy classic-on and **2.87x** on benchy arachne-on
against t124's Orca medians. The route remains operand volume / geometry
(t129/t131), not scheduling.

Evidence: [`evidence/t134-planner-parallelism/`](../evidence/t134-planner-parallelism/FINDINGS.md)
(94 checks; `--self-test` rejects 12/12 negative controls; all eight probe
G-code hashes equal t123's candidate-ordinary set). No production edit; probe
reverted.

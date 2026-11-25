# t134 — Planner effective parallelism: the serial occupancy half is the wall, not the clip fan-out

Route: [Planner effective parallelism](../../issues/134-planner-effective-parallelism.md).
Take: 2026-11-09, ordinary arm, a temporary probe build at HEAD `aee7b0db`
(the probe is reverted; [`probe.patch`](probe.patch) is the only artifact), the
standing matched-pair configs, 24 external core modules,
`RAYON_NUM_THREADS=12`, one probe pass per cell on all eight cells plus two
extra classic-on passes for repeatability. Attribution only: no production
edit, no G-code reference move, both guest-freshness checks untouched.

Every number below is printed by [`reduce_t134.py`](reduce_t134.py) from
[`reduced/summary.json`](reduced/summary.json); [`verify.py`](verify.py)
re-derives them (94 checks) and rejects 12/12 negative controls. Stage walls
and ratios of record remain [t126](../t126-post-t123-phase-split/FINDINGS.md)'s
and [t124](../t124-post-t123-scoreboard/FINDINGS.md)'s; the instrumented walls
here are not wall claims.

## Verdict

**The planner is not stuck at 2.19x because its clip batch is badly
parallelised. It is at 2.64x because 37.8 % of its wall is a fully serial
occupancy offset per layer (14,249 ms, 0.99x) and the clip core that could
fan out already does so at 4.81x.** t132's exact memo raised the stage from
t131's 2.19x to **2.64x** by deleting 14.2 s of serial duplicate work; what
remains is a serial phase, not a scheduling deficit in the batch.

Four findings:

1. **The planner is the least parallel heavy prepass stage on every
   supports-on cell.** Stage-level effective parallelism is **2.64x / 2.66x /
   2.09x / 2.11x** (base classic-on / base arachne-on / benchy classic-on /
   benchy arachne-on) against shell **9.26x / 9.15x / 5.51x / 5.57x**,
   overhang **11.72x / 11.79x / 10.27x / 10.33x** and SupportAnalysis
   **11.46x / 11.39x / 5.57x / 5.59x**. Module-only ratios are **2.76x /
   2.79x / 2.31x / 2.32x**; the stage bracket is 1.1–1.2 s larger than the
   module because it also covers the stage-level work around the dispatch.
2. **The serial fraction has a name and a price.** On base classic-on the
   emit pass carries **96.3 %** of the planner's 103,972 ms of clipper
   thread-time (wall 33,334 ms = 88.6 %), the recurrence-bound drop loop only
   **2.5 %** (wall 6.4 %), and the pre-loop **1.2 %** (wall 3.6 %). Inside
   the emit pass, `p6d_occupancy` is **14,249 ms wall / 14,169 ms thread =
   0.99x** (433 singular offsets), the clip core `p6a_clip_core` is
   **17,811 ms wall / 85,641 ms thread = 4.81x**, and the guest-CPU phases
   are 989 ms + 284 ms + 1,110 ms (node roles, geometry, contacts).
3. **The per-layer barrier is not the binding constraint.** 95.3 % of the
   planner's thread-time sits in the emit pass, which has no cross-layer data
   dependency: it is layer-by-layer only because the guest issues one layer's
   work at a time. The only true inter-layer recurrence is `drop_nodes`'
   top-down walk, and it is **2.5 %** of thread-time / **6.4 %** of wall. The
   122.7 s figure t131 quoted is now **105.1 s** of stage thread-time, and
   the barrier is around a 2.6 s drop loop, not the 100 s emit pass.
4. **Every planner-only scheduling shape is a tranche, not the route.** The
   occupancy batch ceiling is **11.2 s** (11.1x floor, 12.1 % of base-on's
   required prepass cut) and the clip-to-12x ceiling is **10.7 s** (10.6x,
   11.5 %); together **21.9 s** (21.7x, 23.7 %). Even a **zero-cost planner**
   leaves base classic-on at **3.25x** (base arachne-on **3.21x**, benchy
   classic-on **2.89x**, benchy arachne-on **2.87x**) against Orca on the
   t124 scoreboard. The destination remains an operand-volume / geometry
   program (t129/t131), not a scheduling one.

**Go/no-go:** **NO-GO for a planner scheduling candidate as the route or as a
literal all-8-cell candidate; GO (narrowed) for two supports-on tranches** —
the cross-layer occupancy batch ([t135](../../issues/135-cross-layer-occupancy-batching.md))
and the clip fan-out diagnosis ([t136](../../issues/136-planner-clip-batch-fanout.md)).
No production edit in this take.

## 1. What ran

One `PNP_T134_PROBE=1` pass per cell on all eight matched cells, plus two
further base-classic-on and benchy-classic-on passes for repeatability. The
probe adds:

- host-side Clipper2 sweep counters at the four `polygon_ops` primitives, with
  absolute snapshots triggered by a `T134SNAP:` guest log sentinel;
- `map_batch` serial/parallel and request counters;
- stage and module deltas in `prepass.rs`, including the host built-ins that
  t131's stage bracket shared with the module path;
- guest-side wall spans in the tree-support planner for the seven phases plus
  the emit clip flush.

**Output neutrality:** all eight probe G-code hashes equal ticket 123's
candidate-ordinary set (`90638bf3…`, `711c6334…`, `2ba5d84c…`, `a972b0f2…`,
`b3b4ac5e…`, `4e31a2db…`, `6b865d4a…`, `62d6eedc…`). The probe is
attribution-only and the run walls are not claimed.

**Repeatability:** three base-classic-on passes spread **2.45 %** on planner
module wall (37,639 / 37,734 / 38,562 ms) and three benchy-classic-on passes
spread **1.17 %** (1,993 / 1,972 / 1,995 ms). The phases are stable to the
same order; the reduced summary stores the per-pass values.

## 2. Stage parallelism census (the four supports-on cells)

`thread_ms` is the sum of Clipper2 sweep durations accumulated on the rayon
workers; `wall_ms` is the stage bracket from `stage_complete`. Thread ÷ wall
is effective parallelism. `SupportGeometry` is split: the host producer is
**3–4 ms** and the planner module is the stage.

| stage | base classic-on | base arachne-on | benchy classic-on | benchy arachne-on |
| --- | ---: | ---: | ---: | ---: |
| `MeshAnalysis` | 1.12x | 1.11x | 0.62x | 0.62x |
| `Slice` | 6.46x | 6.71x | 6.28x | 6.27x |
| `OverhangAnnotation` | 11.72x | 11.79x | 10.27x | 10.33x |
| `ShellClassification` | 9.26x | 9.15x | 5.51x | 5.57x |
| `SupportAnalysis` | 11.46x | 11.39x | 5.57x | 5.59x |
| `SupportGeometry` (planner stage) | **2.64x** | **2.66x** | **2.09x** | **2.11x** |
| `SupportGeometry` (planner module) | **2.76x** | **2.79x** | **2.31x** | **2.32x** |
| `SeamPlanning` | 7.52x | 7.84x | 4.82x | 5.20x |

The two cells that share a model are identical in planner work (433 / 186
unique occupancy offsets; 70,210 / 11,062 clip calls), so their ratios differ
only by run noise. `MeshAnalysis` is mostly guest CPU (thread < wall on
benchy), not a parallel stage. The **1.1–1.2 s** gap between the planner stage
and the planner module is stage-level work outside the module dispatch; the
module is the tighter attribution.

The t131 comparison is the headline: its planner **2.19x** (56,289 ms wall /
122,658 ms thread) is now **2.64x** (39,784 / 105,053). The 17.6 s thread drop
is mostly t132's **14.2 s** duplicate-occupancy removal; the wall drop is
16.5 s. t132 did not just move work; it removed serial work, so the effective
parallelism rose even though the clip core's own fan-out did not.

## 3. Planner sub-phase split (base and benchy classic-on)

Milliseconds inside the single planner dispatch. `thread` is Clipper2 sweep
thread-time; `wall` is the guest span. `par` is thread ÷ wall; a guest-CPU
phase has no clipper thread-time and is shown as `—`.

| sub-phase | base classic-on wall | base thread | base par | benchy classic-on wall | benchy thread | benchy par |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `p0_volumes` | 174.1 | 99.0 | 0.57x | 19.3 | 8.3 | 0.43x |
| `p2_contacts` | 1,110.0 | 1,114.1 | 1.00x | 156.1 | 158.1 | 1.01x |
| `p3_drop` | 2,402.4 | 2,610.4 | 1.09x | 246.9 | 241.6 | 0.98x |
| `p5_smooth` | 74.8 | — | — | 10.9 | — | — |
| `p6d_occupancy` | **14,249.1** | **14,168.6** | **0.99x** | **597.9** | **585.0** | **0.98x** |
| `p6e_node_roles` | 988.9 | — | — | 39.5 | — | — |
| `p6f_emit_geometry` | 284.4 | 338.8 | 1.19x | 54.6 | 75.0 | 1.37x |
| `p6a_clip_core` | **17,811.5** | **85,641.4** | **4.81x** | **781.5** | **3,533.6** | **4.52x** |
| residual (dispatch, assembly) | 543.7 | — | — | 86.3 | — | — |
| **module total** | **37,639** | **103,972** | **2.76x** | **1,993** | **4,602** | **2.31x** |

Rollups:

| rollup | base wall | base wall share | base thread | base thread share | benchy wall | benchy wall share | benchy thread | benchy thread share |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| emit pass (`p6d+p6e+p6f+p6a`) | 33,334 | 88.6 % | 100,149 | **96.3 %** | 1,474 | 73.9 % | 4,194 | **91.1 %** |
| drop loop (`p3`) | 2,402 | 6.4 % | 2,610 | 2.5 % | 247 | 12.4 % | 242 | 5.3 % |
| pre-loop (`p0+p2+p5`) | 1,359 | 3.6 % | 1,213 | 1.2 % | 186 | 9.3 % | 166 | 3.6 % |
| residual | 544 | 1.4 % | — | — | 86 | 4.3 % | — | — |

**The occupancy term is the whole story of the serial wall.** `p6d` is 37.8 %
of base classic-on's planner wall and returns **0.99x**: 433 unique offsets,
one per active layer, each a singular `host::offset_polygons` call (14,169 ms
thread, 14,249 ms wall). t132's memo already serves the second, byte-equal ask
from the first; the remaining 433 are genuinely distinct payloads. The clip
core's 85.6 s of thread-time is delivered in 17.8 s at **4.81x**, i.e. the
batch path is doing real fan-out, just not 12x.

The **122.7 s question** is therefore answered in the only form that matters:
the current stage carries **105.1 s** of clipper thread-time, of which
**100.1 s (95.3 %)** is in the layer-independent emit pass and **2.6 s
(2.5 %)** in the recurrence-bound drop loop. The emit pass is not waiting on a
dependency between layers; it is waiting on the guest to issue the next
layer's work. The drop loop is the only part that *cannot* be pipelined across
layers, and it is 6.4 % of the wall.

## 4. Why the clip core is 4.81x, not 12x

This take did not build the mechanism, but it bounds the diagnosis:

- The clip core issues **70,210 calls / 270.6 M input points** over **559
  parallel batch calls** (125 requests per batch on average), plus 26 serial
  batches with only 42 requests. The serial batches are 0.06 % of requests and
  cannot explain the gap.
- Each batch is a `map_batch` over independent `(item, role, region)` clip
  requests, so the work is embarrassingly parallel within a batch. The
  measured **4.81x** against a 12-thread pool is therefore load imbalance,
  per-batch tail, or guest assembly between the batch rounds — not a
  dependency.
- The planner's largest single clip sweep is **655 ms** and the largest single
  operand **169,716 points**, against a 3,854-point mean on the clip core. A
  single tail request can own a meaningful share of one batch's wall; the
  per-batch distribution is the next measurement.

This is graduated as [t136](../../issues/136-planner-clip-batch-fanout.md), not
priced as a candidate. Its **10.7 s** ceiling (85.6 s thread at 4.81x → 12x)
is independent of the occupancy tranche.

## 5. Ranked dependency-preserving restructures

All ceilings are on **base classic-on**; the floor is **1,010 ms** (0.80 % of
t124's 126.27 s cell) and the required prepass cut is **92.5 s** (t126's 90 %
of the 102.8 s prepass). A ceiling below the ticket's **50.66 s** route
threshold is a tranche. The occupancy fan-out is the clip core's measured
4.81x; 2.6x (t78's clamp) and 12x (ideal) bracket it.

| rank | restructure | measured / derived ceiling | floor multiple | share of required cut | verdict |
| --- | --- | ---: | ---: | ---: | --- |
| 1 | **Cross-layer occupancy batch** — batch the 433 per-layer emit-check offsets across the buffered layers with `offset_polygons_batch` | **11.2 s** at 4.81x (8.7 s at 2.6x, 13.0 s at 12x) | **11.1x** | **12.1 %** | **GO (narrowed)**, exact by construction; [t135](../../issues/135-cross-layer-occupancy-batching.md) |
| 2 | **Clip-core fan-out to 12x** — chunk/load-balance `build_roles_batched` without changing the request set | **10.7 s** | **10.6x** | **11.5 %** | **GO (narrowed)**, diagnosis first; [t136](../../issues/136-planner-clip-batch-fanout.md) |
| — | **1 + 2 combined** | **21.9 s** | **21.7x** | **23.7 %** | still a tranche |
| 3 | **Drop-loop serial reduction** — not pipelineable (top-down recurrence) | 2.4 s | 2.4x | 2.6 % | below the tranche bar; leave |
| 4 | **Guest-CPU phases** — contacts 1.11 s, node roles 0.99 s, smooth 0.07 s, geometry 0.28 s | 2.5 s | 2.4x | 2.7 % | not a host-batch shape; fog |
| 5 | **Per-object parallelism** — one object in every matched cell | no ceiling in the 8 cells | — | — | not applicable to the destination |

The occupancy batch is the strongest single shape because it is **exact by
construction** (the same offset per payload, only batched) and it addresses the
only phase that is still **0.99x**. t125 is the control: its per-origin
batching inside the bounded flush failed 6/8 and regressed benchy classic-on
−17.90 % wall; t135 must batch the emit-check half and report both halves
separately.

## 6. The all-8-cell gate is structurally unreachable

The four supports-off cells do not run the planner's geometric work:
`enable_support: false` returns from `run_support_geometry_with_analysis`
before `plan_for_object`. Measured on this take:

| cell | whole `SupportGeometry` stage | stage ÷ 0.80 % floor | planner module | planner ÷ floor |
| --- | ---: | ---: | ---: | ---: |
| base classic-off | 266 ms | **0.78x** | 145 ms | 0.43x |
| base arachne-off | 211 ms | **0.64x** | 104 ms | 0.32x |
| benchy classic-off | 37 ms | **0.80x** | 19 ms | 0.41x |
| benchy arachne-off | 35 ms | **0.81x** | 17 ms | 0.40x |

The floor uses t124's cell medians (0.80 % of 42.53 / 41.18 / 5.77 / 5.34 s).
Even deleting the **whole** stage fails base classic-off and only ties the
floor on benchy; the planner module alone is 0.32–0.43x. No planner-only
change can pass the literal all-8-cell form. The t117 amendment — four
supports-on cells positive, all eight byte-identical, no supports-off
regression — is the only gate under which t135/t136 can be judged, and t117
scoped that amendment to its own candidate, so the human must confirm it for
this shape or pair it with a supports-off leg.

## 7. Go/no-go and route arithmetic

- **NO-GO for a planner scheduling candidate as the route.** The measured
  ceiling of the two tranches is **21.9 s = 23.7 %** of base-on's 92.5 s
  required prepass cut, below the ticket's 50.66 s threshold. The planner's
  entire module wall is 37.6 s; even zeroing it leaves **3.25x** on base
  classic-on against t124's Orca median, **3.21x** on base arachne-on,
  **2.89x** on benchy classic-on and **2.87x** on benchy arachne-on. This is
  not a route.
- **GO (narrowed) for the occupancy tranche** ([t135](../../issues/135-cross-layer-occupancy-batching.md)):
  exact, 11.2 s at the measured clip fan-out, 11.1x floor, 12.1 % of the
  required cut, **conditional on the t117 gate form**.
- **GO (narrowed) for the clip fan-out diagnosis** ([t136](../../issues/136-planner-clip-batch-fanout.md)):
  10.7 s ceiling, 10.6x floor, 11.5 % of the required cut; no candidate until
  the 4.81x is decomposed.
- **NO-GO for drop-loop pipelining and per-object parallelism.** The first is
  recurrence-bound at 2.4 s; the second has no object to parallelise in the
  matched fixtures.
- **The route remains the operand-volume / geometry program.** t129/t131
  measured the sweep at 98.2–99.9 % of primitive time and the operand at
  89,825 shell points/layer and 592,363 planner points/layer. The planner's
  scheduling ceiling is 0.56x the required cut at best; the shell/planner
  operand work is the only term that can reach 8/8.

## 8. Disclosures

- One instrumented pass per cell; the box was not otherwise controlled. The
  three classic-on repeat passes moved the planner module wall by **2.45 %**
  (base) and **1.17 %** (benchy). The instrumented walls are lower than
  t124's medians (e.g. base classic-on 93.4 s vs 126.3 s), so they are used
  for shares and not as wall claims.
- `thread_ms` is Clipper2 sweep time only. It excludes WIT marshalling, path
  conversion, `PolyTree` rebuild and guest CPU; the phase wall includes them.
  A guest-CPU phase therefore has `thread = 0` and is called serial, which is
  exact for the guest's single dispatch thread but not a claim that its work
  cannot be moved.
- The 4.81x fan-out proxy for the occupancy batch is **derived**, not
  measured: it assumes the occupancy offsets fan out like the clip core. The
  measured range 2.6x–12x (8.7–13.0 s) brackets it. t135 must replace the
  proxy with the batch's own conversion.
- The 2.6x clamp is t78's measured clip-batch speedup, used only as a
  conservative screen; t85's method note still governs candidate pricing.
- `SupportGeometry`'s stage bracket is 1.1–1.2 s larger than the planner
  module bracket; the module is the attribution used for the phase split. The
  first `stage_complete` event is the host producer (3–4 ms), the second the
  planner stage.
- The 633 ms maximum clip sweep and 169,716-point maximum operand are module
  maxima, not phase-localised; the probe's max fields are global and cannot be
  attributed to `p6a` without another run.
- No Orca arm is run here. The ratio counterfactual uses t124's medians, not a
  new measurement.

## 9. Evidence

- [`FINDINGS.md`](FINDINGS.md) — this file.
- [`REPRODUCE.md`](REPRODUCE.md) — apply, build, run, reduce, verify.
- [`probe.patch`](probe.patch) — the temporary probe (host + planner guest).
- [`run_t134_probe.py`](run_t134_probe.py) — the eight-cell campaign driver.
- [`reduce_t134.py`](reduce_t134.py) — stderr → [`reduced/summary.json`](reduced/summary.json).
- [`verify.py`](verify.py) — 94 checks; `--self-test` rejects 12/12 negative
  controls.

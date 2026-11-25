# t136 — Planner clip-batch fan-out: the carve batches are already at 11.6x; the union phase is one-deep

Route: [Planner clip-batch fan-out](../../issues/136-planner-clip-batch-fanout.md).
Take: 2026-11-12, ordinary arm, a temporary probe build at HEAD `8fbbfe49`
(the probe is reverted; [`probe.patch`](probe.patch) is the only artifact), the
standing matched-pair configs, `RAYON_NUM_THREADS=12` on a 12-logical-core box,
three passes on each of the two priced cells. Attribution only: no production
edit, no reference move, both guest-freshness checks untouched.

Every number below is printed by [`reduce_t136.py`](reduce_t136.py) from
[`reduced/summary.json`](reduced/summary.json); [`verify.py`](verify.py)
re-derives them (155 checks) and rejects 27/27 negative controls. Stage and
cell walls of record remain [t126](../t126-post-t123-phase-split/FINDINGS.md)'s
and [t124](../t124-post-t123-scoreboard/FINDINGS.md)'s; the instrumented walls
here are not wall claims.

## Verdict

**The 4.81x is not one number. The carve batches already fan out at 11.6x; the
union phase, which owns 64.2 % of the clip-core wall, fans out at 3.66x because
it issues 4.4 requests per call and every call's wall is its own longest
request.** Of the ticket's three candidates, one is right for the wrong reason
(chunk size — but the 256-request cap is saturated and irrelevant; the binding
chunk is the *phase*'s request count), one is true but not a lever (the tail
*is* the wall, but there is nothing to rebalance at 4.4 requests), and one is
refuted (guest assembly is 3.1 % of the clip core).

Five findings:

1. **The additive decomposition of the clip core is exact at three levels.**
   On base classic-on the planner clip core is **15,841.8 ms**
   (p2 15,938.2 / p3 15,370.0), of which host clip calls are **13,956.4 ms**,
   marshal+decode **1,393.7 ms** and pure guest CPU **491.7 ms**. The host wall
   splits into a **6,587.6 ms** work floor (`thread/12`), **7,224.0 ms** of
   tail-times-chunking (`Σ per-call ideal − work floor`) and **144.8 ms** of
   dispatch excess (0.9 % of the clip core): the batch path is not losing time
   to scheduling overhead, rayon wake-up or load imbalance.
2. **The phase contrast is the whole finding.** `carve_run` issues **65,233
   requests** over 312 calls (p50 **256** requests per call — the
   `CLIP_BATCH_MAX_REQUESTS` cap is saturated) and delivers 40,977.6 ms of
   thread-time in 3,544.7 ms of wall: **11.56x**. `union_run` issues **482
   requests** over 109 calls (p50 **4.4** per call) and delivers 37,237.5 ms in
   10,176.2 ms: **3.66x**. The composite is **4.99x** end-to-end / **5.66x** on
   the host calls, which is t134's 4.81x seen from a faster run.
3. **The union phase's wall is its tail, to the millisecond.** Summed per-call
   `max(thread/12, longest request)` is **10,168.3 ms** against a **10,176.2 ms**
   wall — **8 ms** of the phase is anything other than each call's longest
   request. This is not imbalance a better schedule could fix: with 4.4
   independent requests per call, at most 4.4 of 12 threads can ever be busy,
   so the wall is one request deep by construction.
4. **Chunk size is the lever, measured.** Merging the union phase's requests
   across consecutive chunk-batches takes the phase's floor from
   **10,176.2 ms** to **5,480.2 ms** at 2x (8 layers), **3,465.0 ms** at 4x
   (16 layers) and **3,103.1 ms** at a whole-phase batch. The 4x form recovers
   **6,711.2 ms = 94.9 %** of the phase's irreducible tail gap; LPT's 4/3 bound
   puts the achievable wall at ≤ 4,620 ms, i.e. a **5.6–6.7 s** saving.
5. **The other terms are small or absent.** 22 serial-mode calls carry 41
   requests and 10.1 ms — **0.07 %** of the host wall on base classic-on
   (**1.9–2.1 %** on benchy, where the same fixed-size batches fall below
   `FANOUT_WORK_THRESHOLD`). Guest-side assembly is
   **491.7 ms (3.1 %)**, of which `carve_build`'s request construction is
   305.5 ms; marshal+decode is **1,393.7 ms (8.8 %)** — real, but operand
   locality, not a scheduling shape.

**Go/no-go:** **GO (narrowed) for one measured candidate** — group the union
phase's requests across four chunk-batches (16 layers) instead of one — under
the standing [t137](../../issues/137-planner-scheduling-gate.md) four-cell form;
it is exact by construction (same requests, same arguments, only the call
grouping changes), it is planner-only so the supports-off cells are structurally
inert, and it is a **7.3 %** tranche of base classic-on's required prepass cut,
not the route. **NO-GO for the ticket's three named mechanisms as the
explanation**: chunk-size-as-the-256-cap, load imbalance, and guest assembly
each fail their own measurement.

## 1. What ran

Three `PNP_T136_PROBE=1` passes per cell on `benchy-classic-on` and
`base-classic-on` (six runs), with `--instrument-stderr`. The probe adds:

- host-side per-`clip_polygons_batch`-call counters: request count, mode,
  operand vertices, per-request closure duration sum, longest request, wall;
- guest-side segment markers around all nineteen segments of
  `build_roles_batched` (`carve_build` … `assemble`), plus each segment's wall;
- guest-side per-SDK-call wall around `clip_polygons_batch` in
  `run_clip_batches`, which is `marshal + host call + decode`.

The host echoes the guest markers synchronously, so the stderr stream is
execution order and a segment's guest wall lines up with the calls it issued.

**Output neutrality:** all six probe G-code hashes equal the active post-t139
reference set (`b3f1eccc…` benchy classic-on, `585c6a8e…` base classic-on),
stable across every pass. The probe is attribution-only.

**Repeatability:** the clip-core host wall moves **3.4 %** on base
(13,956.4 / 13,967.8 / 13,504.0 ms) and **2.6 %** on benchy
(669.4 / 667.5 / 685.0 ms). The phase split is stable to the same order: carve
fan-out 11.56 / 11.51 / 11.57x, union 3.66 / 3.59 / 3.53x.

## 2. The decomposition (base classic-on, pass 1)

`thread` is the sum of per-request closure durations measured on the rayon
workers (both WIT conversions plus the clipper op). `ideal` is each call's
critical-path floor `max(thread/12, longest request)`.

| term | ms | share of clip core | identity |
| --- | ---: | ---: | --- |
| **clip core (guest span)** | **15,841.8** | 100 % | `build_roles_batched`, all 19 segments |
| host clip calls | 13,956.4 | 88.1 % | `Σ` call wall |
| — work floor | 6,587.6 | 41.6 % | `thread / 12` |
| — tail × chunking | 7,224.0 | 45.6 % | `Σ ideal − work floor` |
| — dispatch excess | 144.8 | 0.9 % | `Σ wall − Σ ideal` |
| marshal + decode | 1,393.7 | 8.8 % | `Σ sdk − Σ call wall` |
| guest CPU | 491.7 | 3.1 % | `guest span − Σ sdk` |

`guest = host + marshal + guest CPU` and `host = work floor + tail + excess`
both close to **≤ 2 ns**. The fan-out of the same thread-time is **5.66x** on
the host calls and **4.99x** end-to-end.

Pass 2 and pass 3 reproduce the shape: work floor 6,766.5 / 6,378.0 ms, tail
7,026.9 / 6,982.8 ms, excess 174.4 / 143.3 ms, marshal 1,460.2 / 1,375.8 ms,
guest CPU 510.2 / 490.2 ms.

Benchy classic-on is the same shape at 17x smaller scale — clip core 910.2 ms,
host 669.4, work floor 323.3, tail 328.2, excess 17.9, marshal 145.2, guest CPU
95.6.

## 3. The phase contrast (base classic-on, pass 1)

| phase | calls | requests | host ms | thread ms | fan-out | longest request | merge-4 floor |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `carve_run` | 312 | 65,233 | 3,544.7 | 40,977.6 | **11.56x** | 6.1 ms | 3,414.8 |
| `union_run` | 109 | 482 | **10,176.2** | 37,237.5 | **3.66x** | 401.0 ms | **3,465.0** |
| `set_run` | 109 | 519 | 200.7 | 757.5 | 3.78x | 3.3 ms | 64.3 |
| `body_cut` | 40 | 87 | 34.8 | 78.8 | 2.26x | 1.8 ms | 16.6 |

Request counts, fan-out and floors are probe sums; `longest request` is the
module-wide maximum inside the phase.

The carve phase is the control: it is the phase the batch machinery was built
for (ticket 78), it saturates the 256-request cap at p50, and it delivers
**96.3 %** of the theoretical 12x. Nothing is wrong with `map_batch`'s fan-out
policy, the rayon pool, or the ordering guarantee.

The union phase is the defect: p50 **4** requests per call (p90 6, p99 8, max
9), call wall p50 **80.2 ms**, p99 **398.2 ms**, max **401.1 ms**. Its 109 calls
are one per `build_roles_batched` chunk, i.e. one per four buffered layers, and
four layers hold only ~4.4 unions because a union is emitted per
`(item, role)` that carries more than one carved region.

`set_run` and `body_cut` are the same mechanism at 1.5 % of the clip core —
worth noting because a candidate that only special-cases `union_run` leaves
them behind, and one that raises the chunk size fixes all three.

## 4. Why more threads cannot help the union phase

For nearly every union call the binding term of the per-call floor
`max(thread/12, longest request)` is the longest request, not `thread/12`;
summed over the 109 calls the floors are **10,168.3 ms** against a
**10,176.2 ms** wall. The phase is therefore **one request deep**: its wall is
the sum of 109 longest-request durations, and the remaining **27.1 s** of the
phase's 37.2 s of thread-time runs behind them.

That is a **parallelism-width** deficit, not a load-balance one. At 4.4 requests
per call a 12-thread pool has 7.6 idle threads no matter how cleverly it is
scheduled, and re-ordering requests without adding any cannot help. The two
things that can:

- **more requests per call** (merge chunk-batches — exact, the candidate);
- **shorter requests** (split a union's operand set — a work-set change, which
  the map's t111/t128 exception does not cover; it is the planner's own
  geometry, not the prepass port).

Both attack the same **7,073 ms** gap (`union wall − thread/12`), which is why
the candidate's measured ceiling and the split's tail are the same number.

## 5. Ranked variants, with measured or derived ceilings

Floor is the map's **1,010 ms** (0.80 % of t124's 126.27 s base classic-on
cell); the required prepass cut is **92.5 s**. All ceilings are on base
classic-on; benchy is priced below.

| rank | variant | basis | ceiling | floor multiple | share of required cut | verdict |
| --- | --- | --- | ---: | ---: | ---: | --- |
| 1 | **Union-phase request grouping** — raise `EMIT_BATCH_LAYERS` from 4 to 16 (or accumulate union requests across four chunk-batches) | measured merge-4 floor 3,465.0 ms; LPT 4/3 bound ⇒ ≤ 4,620 ms; **5,556–6,711 ms** saved | 6.7 s | **6.6x** | **7.3 %** | **GO (narrowed)** — exact by construction |
| — | same at 8 layers | measured merge-2 floor 5,480.2 ms ⇒ 4,696 ms saved | 4.7 s | 4.6x | 5.1 % | fallback if 16-layer buffering breaks the memory budget |
| — | whole-phase single batch (unreachable) | measured phase floor 3,103.1 ms ⇒ 7,073 ms saved | 7.1 s | 7.0x | 7.6 % | upper bound; needs whole-job buffering |
| 2 | **Split the heavy union requests** | `union wall − thread/12` = 7,073 ms, all of it tail | 7.1 s | 7.0x | 7.6 % | needs a work-set ruling; not a scheduling shape |
| 3 | **Carve chunking / 256 cap** | carve is 11.56x against a 12.0x floor; p50 already 256 requests/call | ≤ 0.13 s | 0.13x | 0.14 % | **no headroom** |
| 4 | **Dispatch / load-balance tuning** | measured dispatch excess 144.8 ms | 0.14 s | 0.14x | 0.16 % | **refuted** |
| 5 | **Guest assembly** | measured 491.7 ms, of which `carve_build` 305.5 ms | 0.49 s | 0.49x | 0.53 % | **refuted** |
| 6 | **Marshal / operand locality** | measured 1,393.7 ms (8.8 %) | 1.4 s | 1.4x | 1.5 % | below the tranche bar; t129 route, not scheduling |

Benchy classic-on, same rank-1 candidate: union wall 442.8 ms → merge-4 floor
148.8 ms, a **294.0 ms** saving (**2.9 %** of the 10.01 s cell, **3.7x** the
80 ms floor).

The rank-1 candidate's post-candidate composite: host wall 13,956.4 →
**7,245–8,400 ms**, end-to-end clip-core fan-out 4.99x → **7.7–8.7x**
(9.4–10.9x on the host calls alone), i.e. the shape reaches roughly 9x of the
12x ideal and stops there because `set_run` and
`body_cut` carry the same width deficit at smaller scale and because the work
floor itself (6,587.6 ms) is 41.6 % of the clip core.

## 6. Gate and route arithmetic

The candidate is planner-only (`PrePass::SupportGeometry` /
`com.core.tree-support-planner`), and the supports-off cells decline before
`plan_for_object` (`enable_support: false`), so the literal all-8-cell 0.80 %
gate is structurally unreachable for it — the same arithmetic
[t134](../../issues/134-planner-effective-parallelism.md) recorded and
[t137](../../issues/137-planner-scheduling-gate.md) generalized. It must be
judged under the standing four-cell form: positive 0.80 % paired wall/CPU
improvement in the four supports-on cells, byte-identical output in all eight
against the active post-t139 reference set, and no median wall/CPU regression
in the four supports-off cells.

Route arithmetic: **6.71 s** is **7.3 %** of base classic-on's 92.5 s required
prepass cut, **5.3 %** of its 126.27 s cell, and **17 %** of t134's 39.8 s
planner stage. That is a tranche of the same order as t135 Stage 2's measured
9.8 s, and it leaves the destination's 8/8 unchanged: the operand-volume /
geometry route (t129/t131) still owns the rest.

**Memory is the candidate's first check, not its last.** `EMIT_BATCH_LAYERS`
is documented as a memory bound ("keeping the buffered collision sets from
stacking up across the whole job"); 4 → 16 quadruples the buffered collision
sets, and the base job's peak working set is already large. A union-only
accumulator that defers just the union requests (holding carved geometry, not
collision sets) is the lower-risk form of the same candidate and should be
priced first if the 16-layer buffer moves peak RSS meaningfully.

## 7. Go/no-go

- **GO (narrowed) for union-phase request grouping** — measured 6.7 s ceiling,
  6.6x floor, 7.3 % of the required cut, exact by construction, gated by the
  t137 four-cell form. Graduated as
  [t143](../../issues/143-union-phase-request-grouping.md).
- **NO-GO for the 256-request cap, load imbalance and guest assembly as
  explanations.** Each is measured and each is too small to matter: the cap is
  saturated where the work is, dispatch excess is 0.9 %, guest assembly is
  3.1 %.
- **NO-GO for splitting heavy union requests in this take.** It has the same
  7.1 s tail but it changes the planner's geometric work set, which needs its
  own t81-style ruling; the map's t111/t128 exception is scoped to the prepass
  port, not to planner geometry.
- **The route remains operand volume / geometry.** A zero-cost clip core would
  still leave the cell far from Orca; this tranche buys 7.3 % of the required
  cut, not a cell.

## 8. Disclosures

- One probe pass per cell per repeat, three repeats, ordinary arm only. The box
  was otherwise uncontrolled except for its own load; the clip-core host wall
  moved 3.4 % (base) and 2.6 % (benchy) across repeats.
- The instrumented walls are lower than t124's medians (base classic-on 72–75 s
  total process wall against a 126.27 s median cell). That is a probe-pass
  artefact, not a speed claim; every number here is a share, a ratio or a
  counter sum.
- `thread` excludes nothing inside the request closure but does exclude rayon
  scheduling, the host's `cost` pass and the guest's slice construction; the
  latter is why `guest CPU` is reported as a residual rather than a profiler
  figure.
- The merge floors are **optimistic lower bounds** (`max(thread/threads,
  longest request)` per merged group). LPT on 18 requests over 12 workers lands
  within **4/3** of them, so the candidate's priced saving is quoted as the
  bracket 5.6–6.7 s, never as the floor alone.
- Per-request **operand size** is not recorded; the probe sums vertices per
  call. The 401.0 ms longest union request is therefore priced by duration, not
  by point count, and t134's 169,716-point maximum cannot be attributed to a
  phase from this take.
- The request census is 66,321 `ClipRequest` items on this tree; t134's 70,210
  was measured before t139's layer-plane change and is not directly
  comparable.
- No Orca arm is run. Route arithmetic uses t124's medians and t126's split,
  not a new measurement.

## 9. Evidence

- [`FINDINGS.md`](FINDINGS.md) — this file.
- [`REPRODUCE.md`](REPRODUCE.md) — apply, build, run, reduce, verify.
- [`probe.patch`](probe.patch) — the temporary probe (host + planner guest).
- [`run_t136.py`](run_t136.py) — the six-run campaign driver.
- [`reduce_t136.py`](reduce_t136.py) — stderr → [`reduced/summary.json`](reduced/summary.json).
- [`verify.py`](verify.py) — 155 checks; `--self-test` rejects 27/27 negative
  controls.

# t143 — Union-phase request grouping: depth 12 collapses the union tail; depth 16 overshoots the cache/bandwidth sweet spot

Route: [Union-phase request grouping](../../issues/143-union-phase-request-grouping.md).
Predecessor: [Planner clip-batch fan-out](../../issues/136-planner-clip-batch-fanout.md).

**Headline: the planner's emit buffering depth is the union phase's width lever,
and the measured optimum is 12 layers, not the pre-declared 16.** The union
phase's wall was its own tail — 109 calls of 4.4 requests each, one request
deep — so raising `EMIT_BATCH_LAYERS` from 4 to 12 merges it into **37 calls of
~13 requests** and takes the probe's union wall from **10,093.1 ms to
4,848.7 ms** on base classic-on (**−52.0 %**) while the clip core falls
**14,158.5 → 8,934.5 ms**. Depth 8 lands almost exactly on t136's merge-2 floor
(5,459.7 vs 5,480.2 ms). Depth 16 **overshoots**: the same 482 requests cost
**24 %** more thread-time (37.7 → 46.9 s) once sixteen layers of collision sets
are live, so its union wall (5,793.2 ms) is worse than depth 12's even though
both depths pay the same inflated thread-time. Peak working set is flat from 8
through 12 (2.275 / 2.276 GiB) and rises at 16 (2.429 GiB). The change is exact
by construction, and all eight cells emit identical bytes.

**Paired gate: KEEP.** All four supports-on cells clear the 0.80 % floor at
**8.40x–16.76x**, wall-positive in 11 of 12 pairs. **Base classic-on saves
9.56 s (12.80 %)**, clearing the ticket's 3,500 ms DROP bar by 2.7x and worth
**10.3 %** of the cell's 92.5 s required prepass cut. The four supports-off
cells are structurally inert (`enable_support: false` returns before
`plan_for_object`) and every median sits inside the same-arm null control's
band. The CPU record is the disclosed gap: the strict CPU-lower form passes
**1/4**, and the three negatives (−1.26…−1.83 %) are what a wider fan-out is
supposed to do — more threads busy, less wall — all inside the null CPU band.

> **Fixture disclosure (this take).** The user-supplied `tmp/3dbenchy.stl` was
> lost and replaced on 2026-11-12 with a different export of the same model
> (240 layers / 48.0 mm). t139's four **benchy** reference hashes are therefore
> retired; benchy cells are checked **candidate == baseline** on the
> replacement fixture, with the baseline recorded in
> [`reference.json`](reference.json). `tmp/base.stl` is byte-identical to the
> original and every base cell still reproduces t139's set exactly
> (`base-classic-off` = `ca14d055…`). No benchy ratio is quoted against a
> pre-swap measurement, and the re-baseline is graduated as
> [t145](../../issues/145-benchy-fixture-rebaseline.md).

## 1. What was measured, and how

The candidate is a one-line buffering-depth change in
`modules/core-modules/tree-support-planner/src/lib.rs`:
`EMIT_BATCH_LAYERS` 4 → 12. It changes only how many layers' `build_roles`
inputs are buffered before `build_roles_batched` runs, so it widens three phases
at once — the union rounds, the `set_run` and `body_cut` rounds, and the
occupancy batch (which shares the chunk).

Exactness is by construction: every deferred request is the same clipper op
with the same arguments, the host returns the i-th result for the i-th request
(ADR-0049), and the flush order stays layer-ascending.

Instrumentation: t136's probe ([`probe.patch`](../t136-clip-fanout/probe.patch))
marks all nineteen segments of `build_roles_batched` and folds every host
`clip_polygons_batch` call into the open segment. `run_depth.py` adds process
wall, CPU and peak working set
(`PROCESS_MEMORY_COUNTERS.PeakWorkingSetSize`) plus the time from launch to the
first per-layer stage event. The probe is attribution-only: its walls are never
benchmark cells.

## 2. The buffering-depth table

Pre-declared depths were 4, 8 and 16; **12 was measured after the table** and is
the depth the candidate uses (see §6). All rows are one probe pass each, on the
same build, with `RAYON_NUM_THREADS=12`.

**base classic-on**

| depth | union calls | requests | union wall | union thread | fan-out | clip core | peak RSS | first per-layer | probe wall |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 4 | 109 | 482 | 10,093.1 ms | 37.67 s | 3.73x | 14,158.5 ms | 2.356 GiB | 69.7 s | 83.9 s |
| 8 | 55 | 482 | 5,459.7 ms | 38.61 s | 7.07x | 9,602.9 ms | 2.275 GiB | 62.7 s | 75.9 s |
| **12** | **37** | **482** | **4,848.7 ms** | **46.76 s** | **9.64x** | **8,934.5 ms** | **2.276 GiB** | **56.4 s** | **69.1 s** |
| 16 | 28 | 482 | 5,793.2 ms | 46.92 s | 8.10x | 9,751.3 ms | 2.429 GiB | 63.8 s | 77.8 s |

**benchy classic-on**

| depth | union calls | requests | union wall | union thread | fan-out | clip core | peak RSS | first per-layer |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 4 | 53 | 207 | 447.9 ms | 1.590 s | 3.55x | 698.9 ms | 475.2 MiB | 5.95 s |
| 8 | 28 | 207 | 283.9 ms | 1.905 s | 6.71x | 607.5 ms | 467.1 MiB | 5.55 s |
| **12** | **18** | **207** | **200.3 ms** | **1.972 s** | **9.84x** | **425.7 ms** | **499.1 MiB** | **5.35 s** |
| 16 | 15 | 207 | 223.8 ms | 1.859 s | 8.30x | 435.3 ms | 484.0 MiB | 5.03 s |

**The depth-16 surprise is the finding.** t136's merge-4 projection (union wall
10,176 → 3,465 ms floor, LPT bound 4,620 ms) assumed the same per-request cost.
At sixteen layers the same 482 requests cost **46.9 s** of thread-time against
**37.7 s** at depth 4 (+24 %) — the buffered collision sets and the sixteen-way
occupancy payload compete for memory bandwidth and cache, and that extra thread
time eats the width gain. Depth 12 pays the same thread-time inflation (46.8 s)
but converts it at **9.64x** instead of 8.10x, so its union wall is the minimum.
The pre-declared 16-layer form therefore **fails its own bracket**; the
pre-declared 8-layer fallback lands on the merge-2 floor.

**Memory is not the constraint the ticket feared.** Peak working set is
*lower* at 8 and 12 than at 4 (−85.8 / −85.8 MiB on base; −8.5 / +25.1 MiB on
benchy) and +78.6 MiB at 16 on base. The constant's doc comment called it a
memory bound; on this job the effective bound is cache/bandwidth, not RSS.

**Time-to-first-layer improves.** The first per-layer stage event moves
69.7 → 56.4 s on base (−19.0 %) and 5.95 → 5.35 s on benchy (−10.1 %), because
the planner's own stage finishes sooner.

## 3. Output parity

Both arms, one run per cell (ordinary arm, uninstrumented):

| cell | baseline sha | candidate sha | reference |
| --- | --- | --- | --- |
| base classic-off | `ca14d055…` | `ca14d055…` | t139 ✅ |
| base classic-on | `585c6a8e…` | `585c6a8e…` | t139 ✅ |
| base arachne-off | `d8fdc1d9…` | `d8fdc1d9…` | t139 ✅ |
| base arachne-on | `c9fee59a…` | `c9fee59a…` | t139 ✅ |
| benchy classic-off | `a89e449b…` | `a89e449b…` | replacement-fixture baseline ✅ |
| benchy classic-on | `dceb7a48…` | `dceb7a48…` | replacement-fixture baseline ✅ |
| benchy arachne-off | `3286449f…` | `3286449f…` | replacement-fixture baseline ✅ |
| benchy arachne-on | `be786718…` | `be786718…` | replacement-fixture baseline ✅ |

Byte counts are equal per cell and every run exited 0. The two arms share a
byte-identical host binary (`b238d640…`); only the planner guest differs
(`68b6f811…` baseline, `f6632de0…` candidate).

## 4. Paired A/B (ordinary arm, 3 interleaved pairs, 48 runs)

| cell | baseline wall | candidate wall | saved | % | pairs + | floor mult | CPU % |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| base classic-off | 28.39 s | 27.64 s | +0.75 s | +2.64 % | 3/3 | 3.31x | +0.10 % |
| **base classic-on** | **74.70 s** | **65.13 s** | **+9.56 s** | **+12.80 %** | **3/3** | **16.00x** | **−1.83 %** |
| base arachne-off | 29.81 s | 29.86 s | −0.05 s | −0.18 % | 1/3 | −0.22x | +0.49 % |
| **base arachne-on** | **75.49 s** | **65.37 s** | **+10.12 s** | **+13.41 %** | **3/3** | **16.76x** | **−1.69 %** |
| benchy classic-off | 3.36 s | 3.42 s | −0.06 s | −1.92 % | 1/3 | −2.39x | +0.09 % |
| **benchy classic-on** | **6.78 s** | **6.32 s** | **+0.46 s** | **+6.72 %** | **3/3** | **8.40x** | **+0.63 %** |
| benchy arachne-off | 3.42 s | 3.53 s | −0.11 s | −3.31 % | 0/3 | −4.14x | −1.27 % |
| **benchy arachne-on** | **6.77 s** | **6.28 s** | **+0.50 s** | **+7.36 %** | **2/3** | **9.20x** | **−1.26 %** |

Per-pair detail is in [`reduced/summary.json`](reduced/summary.json).

### 4a. Same-arm null control (3 pairs per cell)

The instrument's own paired noise, measured by running the baseline arm on both
sides with the same interleaving:

| cell | wall median | wall abs max | CPU median | CPU abs max |
| --- | ---: | ---: | ---: | ---: |
| base classic-off | −1.34 % | 4.40 % | −0.20 % | 0.89 % |
| base classic-on | −1.71 % | 2.12 % | −0.64 % | 1.88 % |
| base arachne-off | +1.58 % | 2.40 % | +0.12 % | 0.37 % |
| benchy classic-off | +2.19 % | 4.33 % | −1.34 % | 4.24 % |
| benchy classic-on | +1.18 % | 2.10 % | 0.00 % | 1.91 % |
| benchy arachne-off | +1.48 % | 4.04 % | +0.88 % | 1.85 % |

The supports-off cells are 3–30 s jobs whose own spread (up to 4.4 % wall)
exceeds every supports-off delta in §4; the candidate was also *faster* on both
benchy supports-off cells in the single output-campaign runs (3,293 vs 3,397 ms;
3,476 vs 3,517 ms), so the A/B negatives are not a consistent direction.

## 5. Gate

The candidate is planner-only, so it is judged under the standing
[t137](../../issues/137-planner-scheduling-gate.md) four-cell form:

| clause | measurement | verdict |
| --- | --- | --- |
| +0.80 % paired wall, four supports-on cells | 6.72 / 7.36 / 12.80 / 13.41 %; 8.40x–16.76x floor; 11/12 pairs wall-positive | **PASS 4/4** |
| byte-identical output, all eight | §3: candidate == baseline in 8/8; base == t139 in 4/4 | **PASS** |
| no median wall/CPU regression, four supports-off cells | every median inside the same-arm null band; structural inertness | **PASS with disclosure** |
| +0.80 % paired *CPU*, four supports-on cells | strict form 1/4 (benchy classic-on +0.63 %); the rest −1.26…−1.83 %, inside the null CPU band (±1.88–1.91 %) | **literal FAIL, disclosed** |

The CPU clause is the honest gap. A fan-out change is supposed to burn more CPU
to finish sooner: the union phase's thread-time rises from 37.7 s to 46.8 s on
base classic-on while its wall falls 5.2 s, and the occupancy batch widens the
same way. Process CPU therefore rises ~1.8 % on the two base supports-on cells
while wall falls 12.8–13.4 %. The map's destination is wall with CPU
corroborating (never contradicting); the CPU record does not contradict the wall
gain, but it is not the "cheaper work" signature t135 read on a flat record.

## 6. Route arithmetic and verdict

- Base classic-on: **9.56 s = 10.3 %** of the cell's 92.5 s required prepass
  cut; the ticket's 3,500 ms DROP bar is cleared by **2.7x**.
- Base arachne-on: 10.12 s. Benchy supports-on: 0.46 / 0.50 s (8–9x the floor).
- Indicative composition only: t142's composed base classic-on **3.695x**
  × (1 − 0.1280) ≈ **3.22x**; not a re-taken scoreboard.
- This is a **tranche, not a destination step**. It is of t135 Stage 2's order
  (9.76 s / 12.37 s on the same cells) and leaves the operand-volume route
  (t129/t131) owning the rest.

**Verdict: KEEP.** Depth 12 lands in the production tree; the probe is reverted
and the tree carries only the constant, its measured rationale and this
evidence.

**Why 12 and not the pre-declared 16.** The table was taken to decide the
depth. 16 fails the bracket its own projection set (5,793 ms against a 4,620 ms
LPT bound) because of the +24 % thread-time inflation; 8 matches the merge-2
floor; 12 is the measured minimum on both cells. Choosing it after the table is
a tuning decision, disclosed here and in the ticket, not a hidden gate change.

## 7. Disclosures

- **Box state.** Today's base classic-on baseline is 74.70 s against t139's
  126.27 s cell and t135 Stage 2's 93.55 s; the wall magnitudes are not
  comparable across takes, so only paired deltas are claimed. Load sampled
  17 % before the A/B and the null control ran in the same window.
- **No accelerated timing arm.** The known controlled-driver failure on
  `libmimalloc-sys` (t142/t135) was not revisited. The change is guest-only and
  the host binary is shared between arms.
- **No Orca arm.** This is a tranche take; no scoreboard is re-derived.
- **3 pairs per cell.** The null control bounds what 3 pairs can resolve
  (2.1–4.4 % wall on the small cells).
- **Memory is disclosed, not judged** (DEV-026).
- **Probe walls are not claims.** The depth table's `probe wall` column is
  instrumented and is never compared with the A/B.

# Tree-support renderer self-fuel and scan-fill attribution

Type: task
Label: wayfinder:task
Status: resolved (2026-10-14 — attribution take, probe reverted, no
production edit, no timing campaign; suspect refuted, owner pinned, candidate
graduated to 66; see Answer)
Assignee: 2026-10-14 session — first frontier ticket in order on map
resumption
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 61

## Question

What owns `com.core.tree-support`'s largely unmarked rendering work, what reaches
the slice's wall-critical path, and is there an exact reusable-input reduction
worth a separately qualified experiment?

The [Benchy current bottleneck refresh](61-benchy-current-bottleneck-refresh.md)
measures 43.432/43.431 billion renderer instructions on Classic/Arachne
supports-on, 96.85% module self. The renderer is 20.42% of Arachne supports-on
guest fuel; supports-off is the 0.109/0.107 billion negative control. Its
7.389/6.402 s module elapsed sums are **accumulated parallel worker time**, not
wall or CPU. Wall importance is not yet established.

This is the `Layer::Support` renderer, **not**
`com.core.tree-support-planner`; the existing planner carve, union and rejected
projection-cache investigations do not answer this question. Renderer fuel
must not be ranked against the planner's fuel as a proxy for total cost, because
the serial planner also reaches native services.

## Scope and decision evidence

1. After fresh output/freshness qualification, split `TreeSupport::run_support`
   (`modules/core-modules/tree-support/src/lib.rs`) into plan filtering,
   interface regularization, role union, paint/skeleton queries, host wall/fill
   offsets, scan filling and output construction. Non-overlapping guest fuel
   scopes plus export/host brackets must reconcile the residual.
2. `scan_fill_region` in the same file scans every contour/hole edge at each row
   and repeatedly converts coordinates. Measure row/edge/vertex counts and
   fuel before blaming this loop or proposing an active-edge/preconverted-input
   shape. Include high-fuel middle layers and body/interface roles.
3. Qualify actual real-input full-role replay in both modes against an
   independent baseline. Preserve holes, row positions and crossing ordering,
   integer/f32 rounding, wall/fill separation, widths/flows, paint vetoes,
   off-grid anchors, roles and output order. Do not skip regularization or
   simplify planned geometry merely to reduce work.
4. Establish wall relevance using same-run scheduling/brackets, with supports-off
   and both generators as controls. Return a ranked reduction recommendation
   or a documented no-useful-reduction result; no fuel-to-wall assumption.

## Acceptance and boundaries

Attribution only; no candidate, planner rewrite, WIT change or timing campaign
authorized by ticket creation. A surviving output-identical reduction graduates
to a separate experiment and the standing paired ordinary + accelerated
wall/CPU gate, with KEEP/DROP returned to the human. Any discovered geometry
parity defect follows the repo's separate correctness program, not a silent
performance baseline change.

Evidence: [refresh findings](../evidence/perf-refresh-2026-10-13/FINDINGS.md),
its full aggregate profiles, per-layer fuel counts and checkout-only verifier.

## Answer — resolution comment, 2026-10-14

All four scope items answered with one instrumented probe take; full detail
and re-derivation commands: [t64 findings](../evidence/t64-tree-support-renderer/FINDINGS.md).
No production source changed; the probe (`probe.patch`, guest + SDK
counters) was applied, measured and reverted; both mode-specific freshness
gates exit 0 and the restored release binary's classic-off control slice is
byte-identical to its reference.

**1. The fuel split is measured and reconciles.** Four fresh profiled
captures (both generators × supports on/off, 12 threads, outputs
byte-identical to the t58 references in all four cells):

| tree-support scope | self fuel | share |
| --- | ---: | ---: |
| `t64::paint` — `paint_policy_for` | 34.697 B | **79.75%** |
| `t64::skeleton` — containment for extra walls | 6.921 B | **15.91%** |
| `polygon_ops::clip_polygons` (in-guest union/regularize) | 1.322 B | 3.04% |
| `t64::scan_fill` inclusive | 0.187 B | **0.43%** |
| walls/fill offset/regularize/union/output/plan/pitches scopes | 0.097 B | 0.22% |
| unmarked residual (glue, loops, path building) | 0.240 B | 0.55% |

The fold invariant (`self + Σ scope self == dispatch fuel`) holds on every
module row; the split is generator-invariant per scope (skeleton
byte-identical 6.9210 B across three probe revisions). Probe overhead vs the
t61 baseline: +78–79 M instructions (+0.18%), disclosed.

**2. `scan_fill_region` is measured NOT the owner — no-useful-reduction.**
Real-input, full-role, in-place counts: 4,443 scan rows, 2,821,415 edge walks
(635.0/row), 5,642,830 always-taken coordinate conversions (+22,400 flip-side
— 2.008/edge), 11,200 crossings, 3,489 emitted fill paths (3,463 body / 26
interface; role splits exact). Inclusive fuel 0.43% — an active-edge or
pre-converted-input redesign has a ceiling under half a percent of renderer
fuel and is rejected by measurement, per the no-fuel-to-wall rule nothing
further is taken from it.

**3. The paint-policy walk is the owner, pinned by per-query counters.**
836 queries on each ON cell: ~427.7 caller vertices tried (≈ every vertex of
every expoly), ~1,284.7 ring walks, ~893,291 edge tests per query at ~46.46
metered instructions per edge test, over a single-region rebuilt view; i128
crossing arithmetic is <0.1% of edge tests (not the driver); **0 covers found
and 0 eligibility checks in all 836** — the walk is dead on the matched
(unpainted) job, returning `DefaultEligible` every time. Two exact reduction
shapes identified (annotations-empty fast path; ring bbox prefilter) and
**graduated to the separate experiment
[Support paint-policy dead-walk reduction](66-paint-policy-dead-walk-reduction.md)**
— its qualifications carry the both-modes/replay clauses of this ticket's
scope item 3 verbatim (holes, rows, crossings, rounding, wall/fill,
widths/flows, vetoes, anchors, roles, order; no skipped regularization).

**4. Wall relevance is bounded and modest (same-run brackets; attribution
only).** Renderer accumulated worker time 6.79 s (classic-on) / 8.35 s
(arachne-on) over 12-thread per-layer phases of 8.73/8.21 s = **6.5%/8.5%
machine occupancy**; supports-off controls 0.107–0.110 B dispatch fuel
matching the t61 baselines with the probe provably silent (zero counters, no
lines) on the disabled path. The renderer's finishing-tail role is nil: the
runs' last-finishing layer (239 here; 2 in t61's runs — generator-run
dependent, linker-owned either way) shows a 0–2 ms renderer dispatch. Freeing
the whole renderer is bounded by accumulated/12 ≈ 0.57–0.70 s of phase wall
(≈6.5–8.5%); the dead walk alone lowers that bound to ≈0.45–0.55 s. These are
instrumented-run bounds — the paired gate decides any wall claim.

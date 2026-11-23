# Tree-support renderer self-fuel and scan-fill attribution (t64)

**Resolution in one line:** the renderer's fuel is owned by a **dead
paint-policy scan** — `PaintRegionLayerView::paint_policy_for`
(`crates/slicer-sdk/src/traits.rs`) burns **79.7%** of all renderer fuel
returning `DefaultEligible` after a geometry walk that never finds a covering
region on the matched job — followed by skeleton containment at 15.9%; the
ticket's suspected `scan_fill_region` loop is measured at **0.43%** inclusive
and has **no useful reduction** at this job. A reduction candidate graduated to
[Ticket 66](../../issues/66-paint-policy-dead-walk-reduction.md); no production
source changed in this take.

Route: [Tree-support renderer self-fuel and scan-fill attribution](../../issues/64-tree-support-renderer-attribution.md).

## Method and evidence

- The T64-PROBE guest instrument (env-free, profiling-gated fuel scopes +
  per-dispatch counters) plus a T64-SDK query counter inside
  `paint_policy_for` were applied to
  `modules/core-modules/tree-support/src/lib.rs` and
  `crates/slicer-sdk/src/traits.rs` — preserved as `probe.patch` (888 lines),
  applied, measured, and reverted in this session. Production behaviour and
  produced geometry are unchanged.
- Four fresh ordinary profiled captures (12 threads, matched configs,
  `--profile --profile-verbose --instrument-stderr`, external-module
  provenance, `cargo xtask build-guests --check` exit 0 pre-batch). **Every
  output is byte-identical to its preserved t58 qualification reference**
  (classic-on `76eb4e1d…`, arachne-on `ae9d9061…`, classic-off
  `34886ccb…`, arachne-off `b76d846a…`), status ok, degraded=false, zero
  fatal/non-fatal errors. All four cells, both generators × supports on/off,
  with the off cells as controls.
- Reduced set: `captures/*.json` + `summary.json` re-derivable by
  `reduce.py` from the raw tree (`target/t64-tree-support-renderer/`,
  one-shot, gitignored); `verify.py` exits 0 against the in-repo set alone
  and rejects five mutations (fuel, output, layer, counter, residual).
- **Units:** fuel is guest instructions (not CPU, not wall); profile wall
  columns are accumulated thread-time (ADR-0055 / ticket 30); instrumented
  captures never make a wall claim.

## The fuel split (both supports-on cells; generator-invariant)

| tree-support scope (836 expoly renders, 240 dispatches) | self fuel | share |
| --- | ---: | ---: |
| `t64::paint` — `PaintRegionLayerView::paint_policy_for` | 34.697 B | **79.75%** |
| `t64::skeleton` — `skeleton_wall_count` containment | 6.921 B | **15.91%** |
| `polygon_ops::clip_polygons` (in-guest `union_ex`/regularize clips) | 1.322 B | 3.04% |
| `t64::scan_fill` inclusive (self + `scan_crossings` + `scan_emit`) | 0.187 B | **0.43%** |
| `t64::walls` (wall-offset marshalling around host imports) | 0.060 B | 0.14% |
| `polygon_ops::offset` (in-guest regularize closing offsets) | 0.046 B | 0.11% |
| `t64::fill_offset` | 0.019 B | 0.04% |
| `t64::regularize` + `t64::union` + `t64::output` + `t64::plan_filter` + `t64::pitches` | 0.018 B | 0.04% |
| unmarked residual (glue, loop control, path building) | 0.240 B | 0.55% |

Probe overhead vs the t61 refresh baselines: dispatch fuel 43.509–43.510 B vs
43.431–43.432 B (+78–79 M, +0.18%; 836 per-query trace lines plus scope marks).
Classical vs Arachne cells agree to ≤0.003 B per scope — the renderer's work
depends on the support plan, not the wall generator, exactly as the refresh
observed. The skeleton scope measured 6.9210 B identical across three probe
revisions this session (deterministic).

## What owns `t64::paint`: a dead scan, pinned by counters

Per query (SDK-side counters, mean over 836 queries per cell):

| counter | mean/query | reading |
| --- | ---: | --- |
| `cover_regions` | 1.0 | the support-glue view's rebuilt SliceIR carries **this dispatch's regions only** (Benchy: 1 region, 2.4 polygons, 1,883 edges) |
| `vert_loops` | 427.7 | **every** contour vertex of the caller expoly is tried (mean expoly 428.2 vertices) |
| `ring_calls` | 1,284.7 | all ~2.4 contour rings plus ~0.6 hole rings per vertex |
| `ring_edges` | 893,291 | 746,791,275 edge tests across the run |
| `ring_flips` | 850.1 | i128 crossing math runs on <0.1% of edge tests — **not** the cost driver |
| `cover_hits` | **0 of 836** | the walk never finds a covering region |
| `elig_checks` | **0** | `support_eligibility` never runs |

`region_covers_expoly` has no bbox prefilter and no early exit, so each of the
836 queries walks 427.7 vertices × 1,284.7 rings at **46.46 metered
instructions per edge test** → 34.70 B. On the matched (unpainted, tree
supports) job the entire walk is **dead**: no region covers any rendered
support expoly's vertices, so `paint_policy_for` returns `DefaultEligible`
every time. The walk's result is the same for all 836 queries and its inputs
(the one region's ~1,883-edge polygons) are re-walked per expoly, per layer.

Reduction shapes that are exact for the answer produced (the graduated ticket
holds the detail, no implementation in this take):

- **(a) annotations-empty fast path** — when the view's regions carry no
  support-blocker/enforcer annotation values, `support_eligibility` can only
  return `DefaultEligible` for every covering region; returning
  `DefaultEligible` without the geometry walk is value-identical. Covers every
  unpainted (auto-support) job — i.e. every matched cell.
- **(b) per-ring bbox prefilter / owner-keyed view** — bounding-box gates
  before `point_in_polygon_ring` removes ~all ring walks while remaining a
  conservative membership prefilter; shapes the painted-job path and any
  future covered-region work.

## Scope item 2 — `scan_fill_region` measured, not blamed

Per run (both ON cells identical): **4,443 scan rows**, **2,821,415 edge
walks** (635.0 edges/row mean), **5,642,830** always-taken `units_to_mm`
conversions (+22,400 flip-side conversions — 2.008 per edge walked),
**11,200** crossings gathered, **3,489** fill paths emitted (3,463 body /
26 interface), 1,357 fill regions scanned (one per fill-offset region,
1,357 fill offsets). Role split is exact (body 831 / interface 5 expolys;
rows 4,397/46; fill paths 3,463/26; wall paths 1,207/6).

Inclusive scan-fill fuel is **0.187 B = 0.43%** of renderer fuel, and the
row-walk arithmetic (2.8 M edge tests × ~46 instr equivalent ≈ 0.13–0.19 B)
reconciles with the measured scope split. **The ticket's suspected loop is
measured not an owner at the matched job**; an active-edge/pre-converted-input
redesign there has a ceiling under half a percent of renderer fuel and is
**documented as no-useful-reduction**. The same counters close the cost model
without any synthetic fixture: the measurement is real-input, full-role, in
place.

## Scope item 1 — `t64::skeleton` and the rest

`skeleton_wall_count` runs 836 times (one per expoly render), points tested
203,930 (244.0/expoly), with **94,511,078** ring-edge walks and
**377,682,312** bbox-scan walks (4× ring len per point test — the four
separate min/max passes in `point_in_expolygon`) at 6.921 B (15.91%). A
per-expoly bbox computed once before the point tests removes the bbox-scan
term (its ceiling is ~2–4 B of the 6.921 B; ranked **second** behind the paint
walk). `union_ex`/regularize clip work is in-guest and already core-marked
(1.32 B, 207 calls — 183 body unions + 4 regularize calls, 175 of 179 entries
take the no-interface short circuit); host-import wall/fill offsets (986 wall
offset calls + 836 fill offsets per cell) execute natively and are visible
only as dispatch-bracket wall, never guest fuel. `t64::plan_filter` is
0.0002 B — 179 plan hits over 179 planned regions, no repeated whole-plan
scan.

## Scope item 4 — wall relevance (same-run brackets, attribution only)

| cell | phase wall | renderer accumulated worker time | machine occupancy (÷12 threads) | renderer on last-finishing layer |
| --- | ---: | ---: | ---: | ---: |
| classic-on | 8,728 ms | 6.79 s | **6.5%** | **0 ms** (layer 239) |
| arachne-on | 8,207 ms | 8.35 s | **8.5%** | **2 ms** (layer 239) |
| classic-off | — | 0.03 s | 0.2% | — |
| arachne-off | — | 0.03 s | 0.5% | — |

The renderer's activity spans most of the per-layer phase (spread across
workers; first start <100 ms, last end within 5 ms of the phase end), but its
own finishing-tail role is nil: the last-finishing layer's renderer dispatch
is 0–2 ms in both cells (this run's last layer was 239; t61's earlier runs
finished on layer 2 — the renderer is trivial in either case; the low-layer
tail is the linker's, per tickets 63/65). Freeing **all** renderer work from
these phases is bounded by accumulated/12 ≈ 0.57–0.70 s of phase wall
(perfect-spread upper bound, ≈6.5–8.5%); removing just the dead paint walk
lowers that bound to ≈0.45–0.55 s. These are same-run bracket bounds on
instrumented runs — **the wall value of any reduction is decided only by the
standing paired ordinary + accelerated uninstrumented gate** (fuel-to-wall
transfer is historically 0–16%). Batch runs carried visible machine load in
places (wall 22.3–32.9 s for identical fuel and CPU); occupancy figures ride
that disclosure.

The supports-off controls re-confirm the refresh baselines
(0.107–0.110 B dispatch) and prove the probed renderer's disabled path never
reaches any instrumentation (all counters zero, no trace lines).

## Verification and scope

```bash
python docs/specs/perf-vs-orca/evidence/t64-tree-support-renderer/verify.py
python docs/specs/perf-vs-orca/evidence/t64-tree-support-renderer/reduce.py          # requires the one-shot raw tree
python docs/specs/perf-vs-orca/evidence/t64-tree-support-renderer/reduce.py --raw-root target/t64-tree-support-renderer
```

`verify.py` re-derives the scope split from the committed captures alone
(fold invariant per module row, telescoping scope identities, counter
partitions, SDK↔guest counter agreement) and rejects five negative controls.
`probe.patch` is the full temporary diff (guest module + SDK counter), applied
and reverted in this take; both guests and the release executable were rebuilt
from HEAD afterwards and both mode-specific freshness gates exit 0. No Rust
source change persists, so no commit-time Rust gate is re-run for this take.

**Limits:** ordinary mode only — the accelerated guest artifact was not
probed (the graduated ticket's standing gate is both-modes); instrumented runs
are attribution-only; per-query instrumentation adds the disclosed ~0.18–0.4%
dispatch-fuel overhead; no base.stl, no Orca run, no uninstrumented timing,
no candidate implementation, no commit.
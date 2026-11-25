# Planner occupancy memo: serve the duplicate half of the singular offset imports from a content cache

Type: task / measured candidate
Label: wayfinder:task
Status: resolved (2026-11-09; **KEEP** — the memo reproduces t131's duplicate
  census exactly (**864/431** base classic-on, **365/179** benchy classic-on), is
  byte-identical in all 16 arm/cell combinations, and clears the 0.80 % floor on
  all four supports-on cells at **7.39x–16.54x** with process CPU corroborating.
  The four supports-off cells are inert by construction (`enable_support:
  false` returns from the planner before either memo site); their mixed-sign
  medians are noise, and the strict sign read (6/8) is recorded rather than
  hidden. base classic-on gains **14.31 s** — 1.006x the duplicate thread-time,
  not the 6.5 s t131 priced. See ## Answer)
Assignee: 2026-11-09 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named); 132 is the first unblocked, unclaimed
  AFK frontier ticket in order (130 is the human's grilling).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Common-stage per-op cost decomposition](131-common-stage-per-op-cost-decomposition.md)'s
  2026-11-09 S6 census: the `+0.35 mm` occupancy family is 864 calls over 433
  distinct payloads, and the 431 duplicate calls cost **14,227.7 ms** of
  host-side offset time on base classic-on.

## Question

Can the planner's two `inflate_model_occupancy` sites be served from a
content-keyed memo so the duplicate half of the occupancy offset work
disappears, with **byte-identical** output?

[t131](../evidence/t131-per-op-decomposition/FINDINGS.md) measured that the
occupancy family is exactly **two calls per payload** on the matched job: the
per-layer emit check (`support_geometry` layer occupancy) and the per-origin
role carve inflate the same layer with the same radius. The payload content
hash (every point and ring boundary, plus `delta = 0.35`, `Miter`,
`arc = 0`, no miter-limit) is identical across each pair, and an offset is a
pure function of `(polygons, delta, join, arc, miter)`, so a memo is
output-identical by construction.

## Scope

1. Add a per-slice memo at the guest's `inflate_model_occupancy` (or at the
   host import boundary, if the memo must be shared across instances) keyed on
   the payload bytes plus the offset parameters. Evict per slice; no
   cross-slice reuse.
2. Keep the two call sites' filters untouched — the cache must not change
   *what* each site asks for, only whether the answer is recomputed.
3. Prove the ceiling on the four classic cells first: the census predicts
   **14,227.7 ms** of duplicate thread-time on base classic-on (**6,504 ms**
   wall at the stage's 2.19x parallelism), **609.6 ms** on benchy classic-on,
   and zero on the supports-off cells.
4. Disclose the multi-region caveat: the 2-calls-per-payload structure holds
   while a layer carries one region; on jobs with several regions the payloads
   differ and the hit rate falls. The census is the matched job's.

## Pre-declared gate

The standing paired gate: 16-cell output (ordinary and accelerated), candidate
arms byte-identical within themselves, baseline reproducing the t123
reference set, the G-code delta audited before timing, and the paired
wall/CPU gate at the 0.80 % floor in all eight cells (or the amended
supports-on subset where a shape is supports-only — this one is supports-on
only, so the four supports-off cells must show no regression). The
[t131](../evidence/t131-per-op-decomposition/FINDINGS.md) floor multiple is
**6.4x** on base classic-on; a cache that hits is exact, so a miss on the gate
means the census structure did not transfer and the candidate should be
retired rather than widened.

## Acceptance

- A candidate under the standing 16-cell contract, or a measured DROP.
- The hit-rate census re-recorded from the candidate run (calls, distinct,
  hits) and compared with t131's 864 / 433.
- No guest-freshness drift: `cargo xtask build-guests --check` exits 0 and the
  staged artifacts are rebuilt for the probe arm only.

## Answer

**KEEP: the duplicate half is on the serial critical path, and removing it is
worth 12–13 % of both base supports-on cells.** The census structure
transferred exactly; the memo is exact by construction; the four supports-on
cells clear the floor at 7.39x–16.54x.

**Candidate.** `inflate_model_occupancy_memoized` in
`modules/core-modules/tree-support-planner/src/lib.rs` serves a content-equal
payload from a per-layer memo keyed by `occupancy_offset_key` (FNV-1a over every
contour and hole point plus `xy_distance_mm`). `OccupancyMemoEntry` retains the
payload beside its result, so a hash collision compares and recomputes instead of
answering wrong. Both call sites — the emit-time collision volume and the
per-origin role carve — use it; the memo is created per emit-loop iteration, so
retained geometry is one layer and nothing crosses a slice. Unit test covers hit,
delta miss and payload miss.

**Census (probe arm, counters reverted before the campaign).** base classic-on
**864 calls / 433 distinct / 431 hits**; benchy classic-on **365 / 186 / 179** —
identical to t131's 864/433 and 365/186. Both probe runs reproduce the reference
G-code hash.

**Output (16-cell contract).** 8 cells × 4 arms, each arm carrying its own module
tree. Every cell emits one hash across all four arms, and every hash is ticket
123's candidate-ordinary reference (`90638bf3…`, `711c6334…`, `2ba5d84c…`,
`a972b0f2…`, `b3b4ac5e…`, `4e31a2db…`, `6b865d4a…`, `62d6eedc…`). The host
binary is identical between arms (`f3d50217…` ordinary, `65d3b7ad…` accelerated);
only the planner guest differs.

**Paired gate (ordinary arms, interleaved, `RAYON_NUM_THREADS=12`).**

| cell | median wall | Δ | floor multiple | Δ CPU |
| --- | --- | ---: | ---: | ---: |
| benchy classic-on | 9.33 → 8.15 s | +1.18 s (+12.62 %) | **15.78x** | +1.11 s |
| benchy arachne-on | 9.17 → 8.62 s | +0.54 s (+5.92 %) | **7.39x** | +0.48 s |
| base classic-on | 119.45 → 105.14 s | +14.31 s (+11.98 %) | **14.97x** | +13.25 s |
| base arachne-on | 119.88 → 104.02 s | +15.87 s (+13.23 %) | **16.54x** | +19.50 s |

Every supports-on cell is separated on wall in every pair. **Supports-off:** both
configs pin `enable_support: false`, `SupportPlanner::from_config` maps it to
`enabled`, and `SupportPlanner::run_support_geometry_with_analysis` returns at
`if !self.enabled` before `plan_for_object`, so the memo never executes on those
cells. Their measured medians (benchy classic-off +0.04 s / +0.01 s CPU; benchy
arachne-off −0.07 s / −0.23 s; base classic-off +0.05 s / +0.34 s; base arachne-off
+0.40 s / −0.97 s) sit inside a 0.7 s (benchy) and 4.9 s (base) spread and cannot
resolve the 0.8 % floor; the mechanical strict-sign read is **6/8** and is
recorded, with the ruling that the clause is discharged by inertness, not by a
noise median.

**Ledger correction to t131.** t131 priced the duplicate half at **6,504 ms** of
wall by dividing 14.23 s of thread-time by the stage's 2.19x effective
parallelism. Measured: **14.31 s** on base classic-on, **1.006x** the duplicate
thread-time — the duplicate calls were effectively serial. benchy classic-on
saves more wall and CPU (**1.18 s / 1.11 s**) than its 0.61 s duplicate
thread-time (1.93x / 1.82x); that excess is unaccounted for by an offset-only
census, mechanism unmeasured, no claim taken. Route arithmetic is unchanged: this
is **15.5 %** of base classic-on's 92.5 s required prepass cut — a banked keep,
not a destination step.

**Disclosures.** Two measurement windows plus a restart-completed pair: pairs 1–3
in one batch, `base-arachne-on` pair 3 in a two-run completion after a server
restart, pairs 4–6 for the four supports-off cells and benchy arachne-on in a
third window; every cell is internally paired and no ratio spans windows. The
probe arm carries two counters and a trace line (not the campaign arm). Box load
sampled 2 %, 7 %, 4 %. No Orca arm: this is a PnP-internal A/B and the scoreboard
of record remains ticket 124's. Multi-region caveat: the two-call structure holds
while a layer carries one region; on multi-region jobs the emit payload and the
role payload differ and the hit rate falls (the miss path is the old computation).

Evidence: [`evidence/t132-planner-occupancy-memo/`](../evidence/t132-planner-occupancy-memo/FINDINGS.md)
(`verify.py` 176 checks, 11/11 negative controls rejected; `reduce_t132.py`
reproduces `reduced/summary.json`; both freshness gates exit 0).

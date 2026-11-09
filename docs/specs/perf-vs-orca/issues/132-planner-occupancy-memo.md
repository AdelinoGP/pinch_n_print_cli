# Planner occupancy memo: serve the duplicate half of the singular offset imports from a content cache

Type: task / measured candidate
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
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

_To be recorded on resolution._

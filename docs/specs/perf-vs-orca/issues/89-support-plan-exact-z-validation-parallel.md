# Parallelize support-plan exact-Z validation

Type: task
Label: wayfinder:task
Status: open (graduated 2026-10-25 from the supports-on increment attribution; unclaimed)
Assignee: none
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Supports-on increment attribution at post-resolution-port HEAD](88-supports-on-increment-attribution.md)

## Question

Can the support-plan aggregation's exact-Z validation loop run in parallel
without changing which plan entries are retained, which diagnostics are
emitted, or their order — and does the resulting wall win pass the standing
paired gate?

## Measured premise

The [t88 attribution](../evidence/t88-supports-increment/FINDINGS.md) measured
the `PrePass::SupportGeometry` stage on base supports-on at HEAD, on a quiet
machine (5 % load):

| cell | stage | planner | post-module gap | `validate_entry` |
| --- | ---: | ---: | ---: | ---: |
| base classic on | 71.12 s | 55.52 s | 15.38 s | **15.24 s** |
| base arachne on | 69.63 s | 55.31 s | 14.07 s | **13.94 s** |

The probe attributes **99.3 %** of the post-module gap to the per-entry
`validate_entry` loop in
`aggregate_support_plan_irs_degraded_with_attributed_diagnostics`
(`crates/slicer-wasm-host/src/support_aggregation.rs`); the clone, sort, union
and commit terms together are under 0.1 s. The loop is serial even though every
`validate_entry` call is a pure function of the entry and the immutable
`ExactZQueryService`. A load-affected early capture measured the same term at
27.2 s classic / 106 s arachne; those walls are disclosed as attribution-only
and are not the basis of this ticket.

The validation cost is the `ExactZQueryService::query` mesh cross-section
(`cross_section_at_z`) plus a per-body overlap test. The service is read-only
apart from an internal `Mutex<HashMap>` cache; the expensive cross-section is
computed outside the lock. The cache key currently includes `region_id`, but
the computation is object-wide, so two entries at the same `(object, z)` with
different regions recompute identical geometry. Deduping the **object-level**
occupancy by `(object, z)` is value-identical today; the public per-region key
can remain for the future region-clipping contract.

## Candidate shape

1. Compute every entry's `validate_entry` verdict with an order-preserving
   indexed parallel map (`rayon` `par_iter().map(...).collect()`), then fold the
   verdicts into `retained` / diagnostics in the existing entry order. The
   ownership check, decline path and union step keep their current order.
2. Optionally add the object-level `(object, z)` occupancy cache so multiple
   regions at one Z reuse one cross-section. This is exact by construction
   because the current computation never reads `region_id`.

No geometry, ordering, diagnostic-code, or output change is permitted. This is
a work-preserving host change; the emitted G-code must remain byte-identical to
the post-t81 reference set.

## Pre-declared floor

**0.80 % of the cell wall**, the map's established screening floor
(t83/t84). The t88 quiet instrumented ceiling is **15.24 s of 184.12 s**
(8.28 %) classic and **13.94 s of 187.97 s** (7.42 %) arachne. The candidate's
own paired campaign must measure its actual wall win and occupancy; the floor
screens the shape, it does not predict the win.

## Proof gate

- 16-cell output-identity matrix against the post-t81 references, or the
  standing post-t81 reference set, in both modes.
- Diagnostics identity: the same retained entry set, the same per-family
  diagnostic codes and the same order.
- Standing paired ordinary + accelerated gate on the four base supports-on
  cells (both modes), 6 alternating pairs per cell, 12 threads, zero starvation
  exclusions, every pair byte-identical.
- Both guest freshness gates exit 0; no guest artifact is expected to change
  (host-only candidate), but the gate runs anyway.

## Deliverables

1. The candidate with the order-preserving parallel fold and, if measured
   worthwhile, the `(object, z)` occupancy dedupe.
2. A unit test that pins the parallel fold's entry/diagnostic order against the
   serial reference on a fixture with multiple entries at one Z and a mix of
   accepted/rejected entries.
3. The paired campaign results and a measured recommendation to the human.
   No auto-commit.

# Parallelize support-plan exact-Z validation

Type: task
Label: wayfinder:task
Status: resolved (2026-10-27; human-approved DROP — measured GATE NOT MET on the
CPU clause, candidate reverted, re-appliable patch retained)
Assignee: opencode session (perf-vs-orca map take, 2026-10-27)
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

## Answer

**The loop can run in parallel without changing a single byte of output — and
the win does not pass the standing gate, because it buys wall with CPU.**

Measured at HEAD `d25f1e34` on the user's Windows desktop, 12 rayon threads.
Evidence: [`evidence/t89-exact-z-parallel/`](../evidence/t89-exact-z-parallel/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 8/8 negative controls rejected).

### Exactness: pass

- Order-preserving fold: admission (ownership + decline) stays serial in entry
  order; the exact-Z geometry test runs through an indexed `rayon` map and a
  serial fold replays `retained` / `unmet` / `diagnostics` in entry order.
- New test `support_plan_aggregation_preserves_serial_entry_and_diagnostic_order`
  (`crates/slicer-wasm-host/tests/contract/support_plan_validation.rs`): 64
  entries, two per anchor Z, mixing accepted/rejected/declined/trespassing
  entries; expectations derived three ways and asserted element for element.
- **Qualification 16/16**: baseline ≡ candidate byte-identical in every cell,
  ordinary cells equal to the derived post-t81 reference set, ordinary ≡
  accelerated in 8/8 groups. `cargo test -p slicer-wasm-host --test contract`
  160/160.

### The gate: not met on CPU

The four-cell standing campaign was **not run** — the stop discipline forbids
it once a clause fails by more than a factor of two. Pre-gate,
`ordinary-base-classic-on`, 3 alternating pairs on a 2 %-load machine:

| metric | baseline median | candidate median | paired-median delta |
| --- | ---: | ---: | ---: |
| wall | 188.388 s | 174.309 s | **−5.50 %** |
| process CPU | 1,067.938 s | 1,137.453 s | **+6.78 %** |

Pre-declared work-preserving CPU ceiling: **+3.0 %**. Every retained pair was
byte-identical; no starvation exclusions; median absolute wall win 10.14 s.

### Why

The probe found **no duplicated work** (`entries = admitted =
distinct_object_z = queries = misses = 431` on base supports-on) and split the
tail into accumulated thread-time: requested-Z cross-section 50.96 %, constant
top-Z cross-section 43.39 %, world-mesh build 5.65 %. Those 92.73 s of
thread-time buy only a 7.99 s wall, while t88 measured the same 431-query tail
at **15.24 s serial** — a cross-run comparison, not a paired measurement. The
cross-section kernel is memory/allocator-bound, so the parallel arm spends ≈6x
more thread-time on the same queries. The CPU delta
is contention, not extra queries.

### Decisions this take carries out

1. **DROP approved by the human on 2026-10-27.** The candidate source is
   reverted to HEAD (`crates/slicer-wasm-host/src/support_aggregation.rs` has an
   empty diff) and preserved as `candidate.patch`, which re-applies cleanly.
   No commit carries the candidate. The order contract it had to satisfy is
   kept and renamed: `support_plan_aggregation_preserves_serial_entry_and_diagnostic_order`.
2. The ticket's optional `(object, z)` occupancy cache is **measured
   worthless here**: one entry per anchor Z, so it would remove nothing. Not
   implemented, not silently skipped.
3. The split graduates a strictly better shape:
   [Memoize z-independent exact-Z query terms](91-exact-z-object-geometry-memo.md)
   — the constant top-Z cross-section plus the world-mesh build are 49.04 % of
   the tail and are recomputed on every miss, so removing them is work-reducing
   and CPU-negative.

### Disclosures

- The 16-cell proof matrix ran on a loaded desktop; it is identity evidence
  only and none of its walls are quoted.
- Two pre-existing reds in
  `crates/slicer-wasm-host/tests/contract/slice_region_view_contract_tdd.rs`
  were repaired to let the contract suite run: the hand-built config fixture
  lacked `resolution` (mandatory since ticket 81) and declared no live overhang
  consumer (ticket 20's conditional stamps). Neither touches the candidate
  surface.
- Probe preserved as `probe.patch`, reverted before the pre-gate; both timing
  arms are uninstrumented.

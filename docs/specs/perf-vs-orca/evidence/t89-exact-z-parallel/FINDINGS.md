# t89 — parallel exact-Z validation: exact, but not work-preserving

Route: [ticket 89](../../issues/89-support-plan-exact-z-validation-parallel.md).
Measured at HEAD `d25f1e34` on the user's Windows desktop (12 rayon threads).
This take is **measurement plus a host-side candidate**: the candidate was
reverted on the human's **DROP** ruling (2026-10-27) and is preserved as
[`candidate.patch`](candidate.patch); no commit carries it.

## Verdict in one line

The order-preserving parallel fold is **exact** (16/16 cells byte-identical,
8/8 cross-mode) and **lowers wall by 5.50 %** (paired median, 3 quiet pairs),
but it raises **process CPU by 6.78 %** — more than twice the pre-declared
+3.0 % work-preserving ceiling. The standing gate's CPU-corroboration clause is
therefore **not met**, and the cause is measured: this workload is
bandwidth/allocator-bound, so twelve concurrent exact-Z queries cost ≈6x more
thread-time each while wall improves only 1.9x.

## What was built

`crates/slicer-wasm-host/src/support_aggregation.rs`:

- Admission (ownership + decline classification) stays serial and publishes
  violations and diagnostics in entry order, exactly as before.
- The exact-Z geometry test is deferred into an indexed `rayon` map whose
  `collect` preserves input order; a serial fold replays every verdict
  (retained / unmet / diagnostics) in that same order.
- Refused and declined entries never reach the query, as before. No geometry,
  ordering, diagnostic-code or output change.

A second candidate shape was **priced and not implemented**: the ticket's
optional object-level `(object, z)` occupancy cache. The probe measured
`entries = 431`, `admitted = 431`, `distinct_object_z = 431` — one entry per
anchor Z on this job — so that cache has **zero** headroom here. It is not a
reduction, and its absence is disclosed rather than papered over.

## Exactness

`cargo test -p slicer-wasm-host --test contract` is **160/160**.

`support_plan_aggregation_preserves_serial_entry_and_diagnostic_order`
(`crates/slicer-wasm-host/tests/contract/support_plan_validation.rs`) drives 64
entries, two per anchor Z, mixing accepted bodies, exact-Z occupancy
rejections, max-body-extent rejections, planner declines and ownership
refusals. It derives the expected sequence three independent ways — the
fixture's own verdict table, the public aggregator applied one entry at a time
in documented order, and the multi-entry aggregate — and asserts all three
agree element for element, including the `retained`, `unmet` and `diagnostics`
orders.

Qualification matrix (`qualification.json`): **16/16 cells pass**, baseline and
candidate byte-identical in every cell, ordinary cells equal to the derived
post-t81 reference set (`references.json`, re-derived from the t81 campaign
rows by `derive_references.py`), ordinary ≡ accelerated in **8/8** groups.

## Pre-gate (stop discipline)

The full four-cell standing campaign was **not run**. A three-pair pre-gate on
`ordinary-base-classic-on` already showed one clause failing by a factor of
more than two, so spending the campaign on it is exactly what the stop
discipline forbids.

| pair | baseline wall | candidate wall | wall Δ | baseline CPU | candidate CPU | CPU Δ |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 188.388 s | 172.880 s | −8.23 % | 1,067.938 s | 1,137.453 s | +6.51 % |
| 2 | 188.676 s | 178.925 s | −5.17 % | 1,068.641 s | 1,142.312 s | +6.89 % |
| 3 | 184.444 s | 174.309 s | −5.50 % | 1,062.938 s | 1,134.953 s | +6.78 % |
| **median** | **188.388 s** | **174.309 s** | **−5.50 %** | **1,067.938 s** | **1,137.453 s** | **+6.78 %** |

Machine load at start: 2 %. One warmup pair per arm (excluded), every retained
pair byte-identical, one SHA per arm, no starvation exclusions. The median
absolute wall delta is **10.14 s**, consistent with the measured tail falling
15.24 s → ~8.0 s.

## Why CPU rises: the tail is bandwidth-bound, not thread-starved

The probe (`probe.patch`, env-gated `PNP_T89_PROBE`, reverted after the take)
counted the same query work in both arms — 431 entries, 431 queries, 431
misses — so the extra CPU is **not** duplicated work. It then split the tail
into three accumulated thread-time terms during the parallel arm:

| term | accumulated thread-time | share |
| --- | ---: | ---: |
| requested-Z cross-section | 47.26 s | 50.96 % |
| constant top-Z cross-section | 40.23 s | 43.39 % |
| world-mesh build + bounds | 5.24 s | 5.65 % |
| **total** | **92.73 s** | **100 %** |

That 92.73 s of thread-time buys only a **7.99 s wall** (11.6x concurrency),
while t88 measured the same 431-query tail at **15.24 s serial** on a quiet
machine. So the same work costs ≈6x more thread-time when twelve queries run
concurrently (cross-run comparison: t88's quiet serial capture against this
take's probe run — an order-of-magnitude statement, not a paired measurement).
The cross-section kernel is memory/allocator-bound, so wall improves ~1.9x and
CPU balloons. The +6.78 % process-CPU delta is the
price of that contention, not a bug in the fold.

Disclosure: the split terms are thread-time under 12-way concurrency, so they
are shares, not wall claims; transferring the shares to the serial arm assumes
contention is roughly uniform across the three terms. The serial comparison
figure is t88's capture, not a same-session serial arm.

## The lead this take graduates

The split also prices a **work-reducing** alternative that needs no threads:

- the constant top-Z cross-section (43.39 %) and the world-mesh build (5.65 %)
  are **z-independent** yet recomputed on every miss — **49.04 %** of the tail.
- Memoizing them per object should remove ≈7.5 s of the 15.24 s serial tail
  with CPU **down**, not up: no contention, strictly less work, value-identical
  because the terms never read the requested Z.

That is a different candidate from the one this ticket pre-declared; it is
graduated as a new ticket rather than smuggled into this take.

## Disclosures

- The 16-cell proof matrix ran on a loaded desktop (Opera and other desktop
  processes). It is an identity check only; no proof-run wall or CPU number is
  used for any speed claim.
- The **pre-gate** ran at 2 % load and is the only timing evidence quoted.
- Two pre-existing red tests in
  `crates/slicer-wasm-host/tests/contract/slice_region_view_contract_tdd.rs`
  were repaired to let the contract suite run: the hand-built config fixture
  lacked `resolution`, which ticket 81 made mandatory, and it declared no live
  overhang consumer, so ticket 20's conditional stamps were correctly skipped.
  Neither repair touches the candidate's surface; both are disclosed here and
  on the ticket.
- The candidate is **not committed** — the human approved DROP on 2026-10-27
  and the source is reverted to HEAD. `candidate.patch` re-applies it exactly.
- `probe.patch` is the probe only; both probes were reverted before the
  pre-gate, so the timing arms contain no instrumentation.

## Reproduction

[`REPRODUCE.md`](REPRODUCE.md). `verify.py --self-test` exits 0 against this
directory alone and rejects **8/8** negative controls, including "pre-gate CPU
brought under the ceiling" and "probe claims dedupe headroom" — the two
controls that would let this evidence drift into a false PASS.

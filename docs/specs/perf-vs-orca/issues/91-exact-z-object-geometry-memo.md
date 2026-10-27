# Memoize z-independent exact-Z query terms per object

Type: task
Label: wayfinder:task
Status: open (graduated 2026-10-27 from the parallel-validation take; unclaimed)
Assignee: none
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Parallelize support-plan exact-Z validation](89-support-plan-exact-z-validation-parallel.md)

## Question

Can `ExactZQueryService::query` stop recomputing the terms that do not depend on
the requested Z — the world-space mesh, its bounds, the build-plate rectangle
and the constant top-Z cross-section — and does the resulting wall win pass the
standing paired gate with process CPU **flat or lower**?

## Measured premise

The [t89 take](../evidence/t89-exact-z-parallel/FINDINGS.md) measured the
exact-Z tail on base supports-on at HEAD `d25f1e34`:

- 431 plan entries, **431 distinct `(object, z)` keys, 431 query misses** — one
  entry per anchor Z on the matched job.
- Each miss recomputes `world_mesh(object)` (a full transform of the 326k-facet
  mesh), `world_bounds`, and a cross-section at the **constant** `bbox.5`
  (the object's maximum Z) before producing the requested-Z cross-section.
- Probe split of the tail (accumulated thread-time, 12-way concurrent arm):
  requested-Z cross-section **50.96 %**, constant top-Z cross-section
  **43.39 %**, world-mesh build + bounds **5.65 %**.
- The z-independent terms are therefore **49.04 %** of the tail. t88 measured
  that tail at **15.24 s** (base classic-on) and **13.94 s** (base arachne-on)
  serial, so the ceiling is ≈**7.5 s / 6.8 s** — **4.1 % / 3.6 %** of the
  respective cell walls, against this map's 0.80 % floor.
- The parallel fold over the *same* work was priced and rejected in t89: it
  lowered wall 5.50 % but raised process CPU **6.78 %**, because the
  cross-section kernel is memory/allocator-bound. A memo removes work instead
  of spreading it, so CPU should **fall**.

Disclosure: the split shares come from the 12-way concurrent arm; transferring
them to the serial arm assumes contention is roughly uniform across the three
terms. The ceiling is arithmetic from that assumption, not a measurement of the
memo.

## Candidate shape

1. Add a per-object memo inside `ExactZQueryService` holding the world mesh,
   its bounds, the plate rectangle and the top-Z cross-section, computed once
   per object.
2. Keep the public per-`(object, region, z)` query cache and its contract
   unchanged: two regions at one Z still resolve their own cache entry, and the
   region key stays available for a future region-clipping contract.
3. The requested-Z cross-section is still computed per distinct key and stays
   the only per-Z work.

**Exact by construction**: none of the memoized terms reads the requested Z
(`cross_section_at_z(&world_mesh, bbox.5)` is a function of the object alone),
so every value is identical. Emitted G-code must stay byte-identical to the
post-t81 reference set.

Watch the memory profile: the memo retains one world-space mesh per object for
the life of the service. The map's out-of-scope bullet excludes memory
*instrumentation*, not memory use, so record the retained size in the findings
rather than treating it as a gate.

## Pre-declared floor

**0.80 % of the cell wall**, the map's established screening floor
(t83/t84/t89). The arithmetic ceiling is **4.1 % / 3.6 %**; the candidate's own
paired campaign measures the actual win.

## Proof gate

- 16-cell output-identity matrix against the post-t81 reference set, both modes.
- A test that pins the memo's identity contract: two queries at different Z on
  one object observe the same memoized terms, and two objects never share them.
- Standing paired ordinary + accelerated gate on the four base classic cells
  (both modes), 6 alternating pairs per cell, 12 threads, zero starvation
  exclusions, every pair byte-identical, process CPU **≤ baseline** (this
  candidate removes work, so a CPU rise is a red flag, not a ceiling to spend).
- Both guest freshness gates exit 0 (host-only candidate).

## Deliverables

1. The memo with the unchanged public cache contract.
2. The identity/ownership test for the memoized terms.
3. The paired campaign results and a measured recommendation to the human.
   No auto-commit.

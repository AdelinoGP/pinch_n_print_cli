# Memoize z-independent exact-Z query terms per object

Type: task
Label: wayfinder:task
Status: resolved (2026-10-27; human-approved KEEP — wall −5.52 / −3.84 / −2.68 /
−4.95 % in the four base supports-on cells with CPU −1.22 / −0.66 / −0.32 % and
one +0.08 % sub-noise miss of the strict `≤ 0` clause; exactness 16/16; candidate
committed `f10cef5a`)
Assignee: opencode session (perf-vs-orca map take, 2026-10-27)
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

## Answer

**Resolved (2026-10-27) with a measured GATE NOT MET on one cell's CPU clause
and a human-approved KEEP.** The candidate is committed `f10cef5a`; the evidence
is [`evidence/t91-exact-z-memo/`](../evidence/t91-exact-z-memo/FINDINGS.md).

### The candidate

One host file plus its contract test. `ExactZQueryService`
(`crates/slicer-wasm-host/src/exact_z_query.rs`) holds a per-object memo of the
world mesh, the bounds-derived plate and the cross-section at the object's
constant maximum Z; the public per-`(object, region, z)` cache is unchanged, so
every distinct key still resolves its own entry. `geometry_build_count()` is
the diagnostic surface, and
`exact_z_memoizes_z_independent_terms_per_object`
(`crates/slicer-wasm-host/tests/contract/exact_z_support_query.rs`) pins: two Z
on one object build once and see identical terms; two objects never share them;
a third Z builds nothing; occupancy still tracks the requested Z.

### Exactness

16/16 proof cells byte-identical between arms across
`{benchy, base} x {classic, arachne} x {supports off, on}`; 8/8 ordinary cells
equal the derived post-t81 references; 8/8 cross-mode comparisons equal; 24/24
measured pairs byte-identical with one SHA per arm; every run clean and
non-vacuous; both snapshots carry 24 byte-identical module trees and distinct
host binaries; both guest freshness gates exit 0.

### Paired campaign — the pre-declared four base supports-on cells

| cell | baseline wall | candidate wall | paired wall | paired CPU | pairs |
| --- | ---: | ---: | ---: | ---: | ---: |
| ordinary base classic-on | 175.128 s | 164.803 s | **−5.52 %** | **−1.22 %** | 6/6 |
| ordinary base arachne-on | 172.881 s | 166.931 s | **−3.84 %** | **−0.66 %** | 6/6 |
| accelerated base classic-on | 169.529 s | 163.258 s | **−2.68 %** | **+0.08 %** | 6/6 |
| accelerated base arachne-on | 161.794 s | 154.282 s | **−4.95 %** | **−0.32 %** | 6/6 |

Zero starvation exclusions. The strict clause was **CPU ≤ baseline** (the
ticket's own wording, not the map's +3.0 % work-preserving ceiling):
`accelerated-base-classic-on` misses it by **+0.08 pp** — 0.8 s of ~1,053 s
process CPU, inside that cell's own repeat spread (baseline repeats span
1,040.8–1,061.0 s). Per-pair wall spread is disclosed on the findings
(−8.03…+0.94, −6.57…−2.69, −8.28…+5.11, −8.98…−0.90 %); two single outliers ran
under transient load with both arms above their medians. The human accepted the
sub-noise CPU miss on 2026-10-27 and kept the candidate.

### Attribution

Instrumented `post_planner_gap_ms` (the `PrePass::SupportGeometry` stage minus
the planners' module walls) falls in all four cells: 14.7→9.9, 18.4→7.4,
21.8→9.9, 13.8→12.1 s. The direct probe (both outputs equal to the reference
SHA) is the sharp number: **431 queries, 431 misses, 431 geometry-term builds
→ 1 build**, time inside `ExactZQueryService::query` **22,636.3 → 8,922.3 ms
(−60.6 %)**, `validate_entry` loop **24,335.0 → 10,004.3 ms**. The remaining
8.9 s is the requested-Z cross-section, the only per-Z term left. The probe is
attribution only and is not part of the shipped candidate.

Measured wall wins beat t89's arithmetic ceiling (7.5 / 6.8 s) in three cells
and land at 6.27 s in `accelerated-base-classic-on`; every cell clears the
0.80 % screening floor.

### Memory (recorded, not gated)

The memo retains one world-space `IndexedTriangleSet` per object for the
service's life. Measured on `tmp/base.stl`: 1,230,602 vertices + 7,383,702
indices = **44,302,032 bytes (42.25 MiB)** per object.

### Verification

`verify.py --self-test` exits 0 against the in-repo set alone and rejects 13/13
negative controls; `derive_references.py` re-derives the eight ordinary
references from the t81 campaign rows. Narrow gates: `cargo clippy --workspace
--all-targets -- -D warnings`, `check-literals` 0, `check-test-quality
--report` 0, both freshness gates exit 0.

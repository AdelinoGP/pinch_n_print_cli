# Batch the requested-Z exact cross-sections in the support validation loop

Type: task
Label: wayfinder:task
Status: resolved (2026-10-30; human-approved **KEEP** — the standing four-cell
gate **PASSED**: wall **−3.97 / −3.19 / −2.86 / −2.64 %** with CPU **−0.70 /
−0.70 / −0.90 / −0.15 %**, 24/24 pairs byte-identical, 16/16 proof cells exact;
candidate committed `afe9c361`. See ## Answer)
Assignee: 2026-10-29 OpenCode wayfinder session — the human said "continue the
perf vs orca map" (no ticket named); the frontier re-derivation found **no open
child (94/94 resolved)**, the session offered six measured fog routes, and the
human selected **batch exact-Z cross-sections**
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Memoize z-independent exact-Z query terms per object](91-exact-z-object-geometry-memo.md)

## Question

Can the **8,922.3 ms** of requested-Z cross-section work inside the serial
`validate_entry` loop be removed — without changing a single occupancy polygon —
by serving all **431** distinct anchor Z planes from the existing sorted-Z batch
slicer instead of 431 independent full-mesh walks, and does the resulting wall
win pass the standing four-cell gate?

## Measured premise

The call chain, by symbol:

- `try_aggregate_support_plans_with_policy`
  (`crates/slicer-wasm-host/src/support_aggregation.rs`) validates every
  `SupportPlanEntry` in a **serial** loop, one `validate_entry(&entry,
  input.exact_z)` call per entry.
- `validate_entry` (same file) calls
  `ExactZQueryService::query(&entry.object_id, entry.region_id,
  units_to_mm(entry.anchor_z))`.
- `ExactZQueryService::query`
  (`crates/slicer-wasm-host/src/exact_z_query.rs`) keys its result cache on
  `(object_id, region_id, z_units)`; on a miss it calls
  `cross_section_at_z(&geometry.world_mesh, physical_z_mm)`.
- `cross_section_at_z`
  (`crates/slicer-core/src/algos/mesh_cross_section.rs`) is a thin wrapper over
  `slice_mesh_ex(mesh, &[z])` — a **one-element** Z array.
- `slice_mesh_ex` (`crates/slicer-core/src/triangle_mesh_slicer.rs`) walks
  **every triangle** in `slice_make_lines`, binary-searches the Z array per
  triangle, and computes an intersection line per crossed plane.

t91 measured, on the base supports-on cells: **431 queries**, **431 distinct
`(object, region, z)`**, **431 misses**, and `ExactZQueryService::query`
**22,636.3 → 8,922.3 ms** while the `validate_entry` loop fell to
**10,004.3 ms**. The remaining 8.9 s is therefore one full-mesh walk **per
distinct anchor Z**, all in the serial loop.

`tmp/base.stl` carries **1,230,602 vertices / 7,383,702 indices = 2,461,234
triangles** (t91 measured 42.25 MiB for the retained world mesh). 431 walks ×
2.46M triangles ≈ **1.06 billion triangle visits** per base supports-on slice.

The batch API that would collapse those walks into **one** already exists and is
the same code path: `slice_mesh_ex(mesh, &sorted_zs)`, which is exactly what
`cross_section_at_z` calls with a one-element slice. Per-plane results do not
depend on which other planes are in the array (`slice_make_lines` computes each
crossed plane independently and chains each layer independently), so the batch
is exact **by construction** — a claim this ticket measures rather than asserts.

## Phase 1 — screen (mandatory)

Screening cells: **ordinary base classic supports-on** and **ordinary base
arachne supports-on** — the two cells whose support validation runs the loop —
matched configs (`evidence/matched-pair/configs/pnp-<generator>-supports-on.json`),
`tmp/base.stl`, `RAYON_NUM_THREADS=12`, every run clean and byte-identical to its
post-t81 reference (`22200671…` classic / `0b29e1bf…` arachne). Instrumented
walls are **attribution only**, never a wall claim; the share is wall-equivalent
(accumulated ÷ the run's own cpu/wall) over the current cell wall, the map's
established method (t89 → t91 → t92 → t93 → t94).

Three numbers:

1. **Per-query cost and triangle visits.** Instrument
   `ExactZQueryService::query`, `cross_section_at_z` and the `validate_entry`
   loop: queries, cache misses, distinct `(object, z)`, per-query wall, and the
   world mesh's triangle count. Confirm the 431 full walks and price the
   per-walk cost.
2. **Batch equivalence.** One `slice_mesh_ex(world_mesh, &sorted_all_zs)` per
   object, then compare **each** Z's `Vec<ExPolygon>` against that Z's single-Z
   result — polygon-for-polygon and point-for-point, not a hash. Exactness has to
   be measured on the real job, not inferred from the API shape.
3. **Batch cost.** Wall of the batched walk against the sum of the 431 single-Z
   calls, both measured in the same probe run. This is the ceiling: if the batch
   is not materially cheaper, the take closes measured-dead here.

Floor: **0.80 % of the cell wall** (t83/t84/t92/t93/t94), re-derived at take
time — t91's kept candidate arm is the current production state: **164.803 s**
classic-on and **166.931 s** arachne-on ordinary, so the floor is **1.318 /
1.335 s**.

Honest priors, to be confirmed or refuted by the screen:

- The batch should be far cheaper — one walk instead of 431.
- It is **not** a 431x win: the batched walk pays two binary searches over 431
  sorted planes plus a straddle test per triangle, so per-visit cost rises even
  as visit count collapses. The screen measures the product; it does not assume
  it.
- If anchor Z values never coincide with a previously queried Z (t91: 431
  distinct), a per-`(object, z)` memo alone buys nothing. The prefetch is the
  shape; the memo is only how the prefetched values are served.

## Candidate shape (Phase 2, only if the screen clears the floor)

Additive, exact-by-construction, host-only:

- A per-`(object, z_units)` cross-section memo inside `ObjectGeometry` (the
  occupancy is object-wide today — the `region_id` in the query key is declared
  as reserved for future clipping, so the cross-section itself is
  region-independent).
- A batch prefetch entry point on `ExactZQueryService` that takes one object's
  distinct Z set, calls `slice_mesh_ex` **once** with the sorted set, and fills
  the memo. The aggregation entry point collects the distinct `(object, z)` set
  from the entries **before** the serial validation loop — every entry is in
  hand at that point, so no streaming contract changes.
- `query`'s per-`(object, region, z)` result contract, `ExactZSupportQuery`'s
  fields, and the emitted bytes are unchanged. A miss with no prefetch keeps
  today's single-Z path, so every existing caller (including tests and the
  traditional-support family) is unaffected.

No IR change, no guest change, no geometry function change, no new dependency.
The fallback shape — the memo alone — is **not** a candidate; without the
prefetch it removes no walks, and the screen must not be read as licensing it.

## Pre-declared gate

**Strict wall below baseline in all four pre-declared cells** —
ordinary/accelerated × classic/arachne, all base supports-on — paired median
wall, process CPU **≤ baseline + 3.0 %** (the map's work-preserving ceiling,
t78/t90/t94 precedent), 6 alternating pairs per cell, 12 threads, zero
starvation exclusions, every retained pair byte-identical. Settled before the
campaign; the human rules on the measured result and no commit is automatic.

Note: t93's lesson was that a **1.4 %** work removal can be wall-neutral in an
accelerated cell. This candidate's ceiling is ~6 % of the cell, so a wall-neutral
cell is not an expected outcome; if one appears anyway, the fog's gate-pricing
question (per-mode pricing, CPU-corroborated wall-neutral) is where it gets
settled — before a campaign, not after one. This ticket does not pre-authorize a
wall-neutral pass in any cell.

## Proof gate

- 16-cell output-identity matrix against the current post-t81 references, both
  modes; ordinary ≡ accelerated bytes.
- Per-Z cross-section equality on the real job: every queried Z's
  `Vec<ExPolygon>` from the batch path equals the single-Z path,
  polygon-for-polygon and point-for-point (Phase 1 measures it on the baseline
  tree; Phase 2 re-runs it on the candidate tree).
- `crates/slicer-wasm-host/tests/contract/exact_z_support_query.rs` and
  `support_plan_validation` stay green, plus a new test pinning batch-vs-single-Z
  equality on a fixture with several distinct Z values, including one outside
  the mesh extent (empty) and one exactly on a flat top/bottom plane.
- Both guest freshness gates exit 0 (host-only candidate).

## Deliverables

1. The Phase 1 split and the per-shape verdicts.
2. If the screen clears the floor: the candidate, the proof matrix and the
   four-cell paired campaign, with a measured recommendation to the human.
3. If it does not: a measured-dead resolution with the probe reverted and the
   fog updated — no campaign.

## Answer

**Resolved 2026-10-30 — human-approved KEEP, candidate committed `afe9c361`.**
The premise held exactly as the ticket stated it: the 431 anchor Z values are
431 distinct `(object, region, z)` keys, the cache never hits, and every miss was
one full-mesh walk over **2,461,234 triangles**. Batching the planes into one
`slice_mesh_ex` walk is exact by construction *and measured exact* on the real
job, and it passes the pre-declared gate in all four cells.

### Phase 1 — the split the ticket demanded

Three probe slices, one per round, outputs byte-identical to the post-t81
references (`22200671…` classic, `0b29e1bf…` arachne):

| cell | round | serial `validate_entry` loop | 431 single-Z walks | one batched walk | saving | share | floor margin | planes equal |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| classic-on | 1 | 7.596 s | 6.418 s | 1.053 s | **5.365 s** | **3.26 %** | 4.07x | **431/431** |
| classic-on | 2 | 7.900 s | 6.723 s | 0.991 s | **5.732 s** | **3.48 %** | 4.35x | **431/431** |
| arachne-on | 1 | 8.042 s | 6.844 s | 1.012 s | **5.832 s** | **3.49 %** | 4.37x | **431/431** |

Per-query cost **7.55–29.95 ms** (median 15.3–16.2 ms) against **0.99–1.05 s**
for all 431 planes at once — a **6.4–6.9x** reduction, not 431x: the batched walk
pays two binary searches over 431 sorted planes plus a straddle test per triangle
and is memory-latency-bound on the 1.23 M-vertex array. All **805 polygons**
across the 431 planes are identical between the batch arm and the production arm.

### Candidate

`ObjectGeometry` memoizes cross-sections by canonical Z units;
`ExactZQueryService::prefetch_cross_sections(object_id, &[z_mm])` fills it from
one `slice_mesh_ex` call over the object's sorted distinct Z set;
`try_aggregate_support_plans_with_policy` collects the non-declined entries'
`(object, units_to_mm(anchor_z))` pairs before the serial loop and prefetches once
per object. `query`'s per-`(object, region, z)` contract, the single-Z miss path
and the per-entry `body rejected: exact-Z query unavailable` degradation are all
unchanged. Host-only: no IR change, no guest change, no geometry function change.

### Pre-declared gate — PASS in all four cells

6 alternating pairs per cell, 2 warmups, **6/6 retained, zero starvation
exclusions, 24/24 pairs byte-identical**, one SHA per cell:

| cell | baseline | candidate | wall Δ | CPU Δ |
| --- | ---: | ---: | ---: | ---: |
| ordinary classic-on | 160.42 s | 154.25 s | **−3.97 %** (−6.17 s) | −0.70 % (−7.9 s) |
| ordinary arachne-on | 158.03 s | 152.35 s | **−3.19 %** (−5.68 s) | −0.70 % (−7.5 s) |
| accelerated classic-on | 162.83 s | 157.40 s | **−2.86 %** (−5.43 s) | −0.90 % (−9.2 s) |
| accelerated arachne-on | 165.47 s | 159.98 s | **−2.64 %** (−5.49 s) | −0.15 % (−2.6 s) |

CPU falls in every cell (**−27.2 s** of process CPU across the four units of
work) and the wall transfer brackets the Phase-1 prediction (5.37–5.83 s).
cpu/wall rises in every cell (6.57→6.76, 6.67→6.87, 6.46→6.63, 6.39→6.61) — a
serial work removal concentrating the same parallel work into less wall, the
artefact t93 recorded, not extra work.

### Proof gate — 16/16 PASS

Both arms built from one tree (baseline stashes the two candidate sources, pops
after); **24 byte-identical guest modules per mode**, four distinct host binaries.
Every cell clean and **byte-identical between arms**, **8/8** ordinary cells
equal to the post-t81 references, **8/8** ordinary ≡ accelerated.

### Disclosures

- Machine load **7 %** at campaign start. **One of 24 pairs regressed**:
  accelerated classic-on pair 4 at **1.0235** (+2.35 %), taken while both arms
  ran ~+10 s in the same pair (167.665 s baseline / 171.606 s candidate); that
  cell's other five pairs are 0.9575–0.9779 and its gated paired median is
  −2.86 %. The pair is disclosed, not excluded; no starvation exclusion fired.
- The accelerated classic-on second half (pairs 4–6) is 6–7 % slower in both
  arms — the same load event.
- Proof warmups are single runs and are not ledger input.
- The candidate keeps t91's per-object world mesh (42.25 MiB on `tmp/base.stl`)
  and adds 805 polygons of cross-section memo per 431 planes.

### Pre-commit gates

Workspace clippy `--all-targets -D warnings` clean; `check-literals` 0;
`check-test-quality --report` 0 findings; `slicer-wasm-host` contract suite
163/0; `slicer-runtime` support-family/aggregation integration 22/0; both guest
freshness gates exit 0 (ordinary and accelerated).

### What is left

The remaining per-Z cost is the batched walk itself (0.99–1.05 s), which is
memory-latency-bound on the vertex array; a Z-interval triangle index could in
principle go further but was not measured and is not needed to clear the floor.
The rest of the serial loop (≈1.0–1.2 s of occupancy clones and predicate work)
is now the larger half and was not scoped here.

Evidence: [`evidence/t95-exact-z-batch/`](../evidence/t95-exact-z-batch/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, **7/7** negative controls
rejected).

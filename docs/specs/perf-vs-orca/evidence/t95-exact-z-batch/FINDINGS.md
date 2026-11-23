# t95 findings — batched requested-Z exact-Z cross-sections

Ticket: [Batch the requested-Z exact cross-sections in the support validation
loop](../issues/95-exact-z-cross-section-batching.md).
Take: 2026-10-29, HEAD `8f19a6a9` (production sources unchanged since the t91
keep `f10cef5a`).
Probe: host-only, `PNP_T95_PROBE=1`, applied to
`crates/slicer-wasm-host/src/exact_z_query.rs` and
`crates/slicer-wasm-host/src/support_aggregation.rs`, reverted before the take
closed; the diff is [`probe.patch`](probe.patch) and it applies to the
**pre-candidate tree** (`8f19a6a9`), not to the candidate, which rewrote the same
regions.

**Final ruling: human-approved KEEP — candidate committed `afe9c361`.** Phase 1
cleared the floor **4.07–4.37x** (431/431 planes exact), the 16-cell proof matrix
is **16/16** exact, and the standing four-cell gate **PASSED**: wall
**−3.97 / −3.19 / −2.86 / −2.64 %** with CPU **−0.70 / −0.70 / −0.90 /
−0.15 %**, 24/24 pairs byte-identical.

## Screening cell and method

Three probe slices: **ordinary base classic supports-on** twice and **ordinary
base arachne supports-on** once — the two cells whose support validation runs the
exact-Z loop. Matched configs
`evidence/matched-pair/configs/pnp-<generator>-supports-on.json`, `tmp/base.stl`,
`RAYON_NUM_THREADS=12`. Every run is clean (`degraded=false`, `fatal=0`,
`non_fatal=0`) and its G-code is **byte-identical to the post-t81 reference**
(`22200671…` classic, `0b29e1bf…` arachne, 42,507,912 / 42,330,265 bytes).

Instrumented walls are **not** wall claims: `--profile --profile-verbose` and the
probe's `eprintln` calls inflate them. The probe times the serial `validate_entry`
loop and, inside it, each `cross_section_at_z` call. Because that loop is serial,
its accumulated nanoseconds **are** wall nanoseconds; the share is that saving
over the cell wall, and the cell walls are t91's kept candidate arm
(**164.803 s** classic-on, **166.931 s** arachne-on) — t91 is the last production
change, and t92/t93/t94 changed no source. Floor = **0.80 %** of the cell wall =
**1.318 / 1.335 s**.

The batch arm is a **counterfactual run after the production loop**: it calls
`slice_mesh_ex(world_mesh, &sorted_all_431_zs)` once and compares every plane's
`Vec<ExPolygon>` against the production single-Z result, polygon for polygon. It
adds its own cost to the instrumented wall and changes no emitted byte.

## Phase 1 — the split

Per-object geometry is one `tmp/base.stl` object: **1,230,602 vertices / 2,461,234
triangles**. Every run reports **431 entries, 431 queries, 431 misses, 431
distinct Z**, so the cache never hits and every miss is one full-mesh walk.

| cell | round | loop | 431 single-Z walks | one batched walk | saving | share of cell | floor margin | planes equal |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| classic-on | 1 | 7.596 s | 6.418 s | 1.053 s | **5.365 s** | **3.26 %** | 4.07x | **431/431** |
| classic-on | 2 | 7.900 s | 6.723 s | 0.991 s | **5.732 s** | **3.48 %** | 4.35x | **431/431** |
| arachne-on | 1 | 8.042 s | 6.844 s | 1.012 s | **5.832 s** | **3.49 %** | 4.37x | **431/431** |

Per-query cost is **7.55–29.95 ms** (median **15.3–16.2 ms**); the batched walk
costs **0.99–1.05 s** against **6.42–6.84 s** for the same 431 planes one at a
time — a **6.4–6.9x** reduction in visits-per-walk terms, not 431x, because the
batched walk pays two binary searches over 431 sorted planes and a straddle test
per triangle, and it is memory-latency-bound on the 1.23 M-vertex array.

The 805 polygons produced across the 431 planes are **identical** in the batch
arm and the production arm in every run (`mismatch=0`).

## Shape verdicts

| shape | verdict | measured |
| --- | --- | --- |
| **batched prefetch per object** (one `slice_mesh_ex` over the object's sorted Z set) | **above floor — 4.07–4.37x** | **5.365 / 5.732 / 5.832 s** saved of the serial loop = **3.26–3.49 %** of the cell against a **1.318 / 1.335 s** floor, **431/431** planes equal |
| per-`(object, z)` memo alone | **refuted** | 431 distinct Z and 431 misses: a memo without a prefetch removes nothing (it only serves Z values already computed, and there are none) |
| parallelising the same work | already refuted by t89 | +6.78 % process CPU for −5.50 % wall on the same loop; the batched shape removes ~6.4 s of *work* on the serial thread instead |

## Phase 2 — candidate, proof and campaign

**Candidate** (host-only, `crates/slicer-wasm-host`):
`ObjectGeometry` gains a `cross_sections` memo keyed by canonical Z units, and
`ExactZQueryService::prefetch_cross_sections(object_id, &[z_mm])` fills it from
**one** `slice_mesh_ex` call over the object's sorted distinct Z set.
`try_aggregate_support_plans_with_policy` collects the non-declined entries'
`(object, units_to_mm(anchor_z))` pairs before the serial `validate_entry` loop
and prefetches once per object. `query` keeps its per-`(object, region, z)`
result contract; a miss with no prefetch still takes the single-Z path, and a
prefetch that cannot resolve its object returns silently so the per-entry
`body rejected: exact-Z query unavailable` degradation is untouched.

### Proof matrix — 16/16 PASS

Both arms were built from the same tree: the baseline arm stashed the two
candidate host sources (`build-snapshots.ps1`), the candidate arm kept them.
Both ship **24 byte-identical guest modules per mode** (pinned in
`provenance/*.json`); only `pnp_cli.exe` differs (four distinct SHAs).

- **16/16** cells: baseline and candidate byte-identical, every cell clean
  (`degraded=false`, `fatal=0`, `non_fatal=0`), non-vacuous roles.
- **8/8** ordinary cells equal to the post-t81 references — no reference drift.
- **8/8** ordinary ≡ accelerated (cross-mode byte equality).

### Paired campaign — gate PASS in all four cells

Four pre-declared base supports-on cells, 2 warmups + 6 alternating pairs each,
**6/6 retained in every cell, zero starvation exclusions, 24/24 pairs
byte-identical**, one SHA per cell.

| cell | baseline wall | candidate wall | wall Δ | CPU Δ | pairs |
| --- | ---: | ---: | ---: | ---: | ---: |
| ordinary classic-on | 160.42 s | 154.25 s | **−3.97 %** (−6.17 s) | −0.70 % (−7.9 s) | 6/6 |
| ordinary arachne-on | 158.03 s | 152.35 s | **−3.19 %** (−5.68 s) | −0.70 % (−7.5 s) | 6/6 |
| accelerated classic-on | 162.83 s | 157.40 s | **−2.86 %** (−5.43 s) | −0.90 % (−9.2 s) | 6/6 |
| accelerated arachne-on | 165.47 s | 159.98 s | **−2.64 %** (−5.49 s) | −0.15 % (−2.6 s) | 6/6 |

CPU falls in every cell (sum **−27.2 s** of process CPU across the four), which
is what a work removal must do; the wall transfer is *larger* than the
predicted 5.37–5.83 s in the classic cells and inside it elsewhere. cpu/wall
rises in every cell (6.57→6.76, 6.67→6.87, 6.46→6.63, 6.39→6.61): removing
serial work concentrates the same parallel work into less wall, the same
artefact t93 recorded — not extra work.

23 of 24 pairs are wall-negative. **One pair regressed**: accelerated
classic-on pair 4 at **1.0235** (+2.35 %), taken while both arms ran ~+10 s in
the same pair (baseline 167.665 s, candidate 171.606 s); that cell's other five
pairs are 0.9575–0.9779. The pre-declared clause is the paired median, which is
−2.86 %; the pair is disclosed rather than excluded, and no starvation
exclusion was recorded (the starve test compares each run's own cpu/wall
against its arm's median).

### Disclosures

- Machine load was **7 %** when the campaign started; the accelerated
  classic-on second half (pairs 4–6) runs 6–7 % slower in **both** arms, which
  is the same load event that produced the single non-negative pair.
- The proof warmups (8 base runs) are single runs and are **not** ledger input;
  they are recorded in `proof-rows.json` but only the two-arm identity check
  uses them.
- The candidate keeps the world mesh per object (42.25 MiB on `tmp/base.stl`,
  t91's recorded, not gated, retention) and adds one cross-section per distinct
  Z — **805 polygons across 431 planes**, i.e. negligible next to the mesh.
- Phase 1 and the campaign agree on the mechanism: the campaign's wall fall
  (5.43–6.17 s) brackets Phase 1's batched saving (5.37–5.83 s).

## Ruling

**Human-approved KEEP (2026-10-30); candidate committed `afe9c361`.** Gate PASS
in all four pre-declared cells; proof matrix 16/16.

Evidence: [`evidence/t95-exact-z-batch/`](.) —
`verify.py --self-test` exits 0 checkout-only and rejects **7/7** negative
controls; `reduce.py` re-derives `qualification.json`, `ab-reduction.json`,
`phase1.json` and `summary.json` from the committed rows and probe logs.

# t113 — Strong planner Phase 1: occupancy inflation owns the residual, but the all-8 gate is structural

Route: [ticket 113](../../issues/113-strong-planner-phase1.md).
Phase-1 decomposition and shape search at HEAD `19a5edfe`. No production edit,
no candidate, no timing campaign, no reference-set generation.

## Headline

**The largest remaining planner term is `inflate_model_occupancy`, not the clip
core.** On base classic-on it is **32.71 s = 56.2 %** of the planner
(**1.48 s = 46.0 %** on benchy classic-on; **43.15 s = 51.4 %** and
**1.68 s = 41.4 %** on the two Arachne-on cells). It is one singular
`host::offset_polygons` per call, split almost evenly between the per-layer
emit check (**16.40 s / 433 calls**) and the per-origin role carve
(**16.37 s / 431 calls**). That is a **host offset batch shape**, and
`offset_polygons_batch` runs the same `slicer_core::polygon_ops::offset` per
request with input-order results, so the shape is **representation-safe**: it
changes how the same geometry is computed, not the geometry.

**The shape clears the 0.80 % floor on all four supports-on cells, but no
planner-only — indeed no support-stage — shape can clear it on the four
supports-off cells.** The whole `PrePass::SupportGeometry` stage on base
classic-off is **300 ms = 0.22 %** of the cell (**46 ms = 0.51 %** on benchy
classic-off), below the floor even if deleted; the planner module alone is
**141 / 23 ms = 0.13x / 0.32x** the floor. The stop rule therefore fires:
**no candidate ticket, no production edit**. The carry decision — fold the
batch into ticket 116's `PrePass::Slice` campaign, amend the gate to the four
supports-on cells, or park the shape — is graduated as
[ticket 117](../../issues/117-strong-planner-carry-ruling.md).

## What ran

One instrumented pass per captured cell at HEAD `19a5edfe`, ordinary external
mode, 12 rayon threads, matched-pair configs, the temporary `PERF-T113-PROBE`
guest brackets captured in [`probe.patch`](probe.patch). The probe logs one
`t113-planner-split` line; `reduce_t113.py` parses it into
[`reduced/summary.json`](reduced/summary.json). The raw JSONL captures live in
the gitignored `target/t113-probe/`.

A fresh probe was necessary rather than optional: the only existing sub-term
split is [ticket 22](../t22-planner-substage/FINDINGS.md)'s 2026-09-24 capture,
which predates t78's clip batching and measured a 269.6 s planner (4.5x the
post-t78 module). Its pre-batching shares cannot be rescaled into a current
split, and its occupancy-inflation term was never re-attributed after the clip
batch landed. The probe is attribution-only: no candidate, no paired A/B, no
reference-set generation, and it was reverted before the take closed.

**Probe output-neutrality:** the probe G-code is byte-identical to the
uninstrumented HEAD baseline on the three cells re-run for the check —
base classic-on `4b6d2843…`, benchy classic-on `3b8fa86a…`, benchy arachne-on
`d4644c14…`. The t87/t91 reference set (`22200671…` on base classic-on) is
**pre-merge** and does not match HEAD; that is the post-merge reconciliation
owned by [ticket 115](../../issues/115-post-merge-reconciliation.md), not a
probe effect.

## Table 1 — the planner bracket

Instrumented `slice_complete` walls and `module_complete` brackets. Walls are
**not wall claims**; shares are the usable quantity.

| cell | cell wall ms | planner module ms | planner share | planner ÷ floor |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | 9,085 | 23 | 0.25 % | 0.32x |
| benchy classic-on | 15,333 | 3,279 | 21.39 % | 26.73x |
| benchy arachne-on | 27,122 | 4,133 | 15.24 % | 19.05x |
| base classic-off | 135,030 | 141 | 0.10 % | 0.13x |
| base classic-on | 200,099 | 58,498 | 29.23 % | 36.54x |
| base arachne-on | 416,375 | 84,337 | 20.26 % | 25.32x |

The supports-off cells are the planner **declining**: the module returns before
the emit pass and before the probe guard, so there is no probe line and only the
host bracket exists. Arachne-off was not re-captured; the same early-return path
applies, and the classic-off cells alone already prove the structural stop
below.

The `base arachne-on` wall is **2.19x** t88's **189.9 s** quiet capture; the
`benchy arachne-on` wall is **2.10x** the **12.9 s** cell derived from t103's
4.53x ratio (a ratio derivation, not a direct quiet capture). Those two passes
are disclosed as inflated. Same-run shares are still usable, and the floor
multiples are conservative because the floor is 0.80 % of the same inflated
wall.

## Table 2 — the sub-term split (supports-on cells)

Milliseconds inside the single planner dispatch; shares of the probe total.
The terms are disjoint except the nested ladder/MST brackets, which are shown
inside the drop loop and not added again.

| sub-term | base classic-on | benchy classic-on | base arachne-on | benchy arachne-on |
| --- | ---: | ---: | ---: | ---: |
| `inflate_model_occupancy` (`p6d`) | **32,712 (56.2 %)** | **1,483 (46.0 %)** | **43,147 (51.4 %)** | **1,679 (41.4 %)** |
| clip core (`build_roles_batched`) | 18,023 (31.0 %) | 921 (28.6 %) | 30,100 (35.9 %) | 1,314 (32.4 %) |
| — role union | 11,632 | 416 | 17,098 | 572 |
| — per-region carve | 5,623 | 408 | 11,758 | 603 |
| — set-wide difference | 252 | 45 | 399 | 69 |
| drop loop (`p3`) | 3,612 (6.2 %) | 349 (10.8 %) | 5,022 (6.0 %) | 452 (11.1 %) |
| — collision ladder | 536 | 102 | 970 | 149 |
| — avoidance ladder | 1,943 | 201 | 2,732 | 276 |
| — Prim MST | 411 | 18 | 509 | 19 |
| contacts (`p2`) | 1,397 (2.4 %) | 194 (6.0 %) | 1,830 (2.2 %) | 218 (5.4 %) |
| node roles (`p6e`) | 1,185 (2.0 %) | 83 (2.6 %) | 1,826 (2.2 %) | 133 (3.3 %) |
| node ellipses (`p6f`) | 381 (0.7 %) | 77 (2.4 %) | 612 (0.7 %) | 108 (2.7 %) |
| volumes (`p0`) | 227 (0.4 %) | 22 (0.7 %) | 312 (0.4 %) | 31 (0.8 %) |
| smooth (`p5`) | 119 (0.2 %) | 17 (0.5 %) | 197 (0.2 %) | 16 (0.4 %) |
| stamp (`p7`) | 18 (0.03 %) | 4 (0.1 %) | 29 (0.03 %) | 4 (0.1 %) |
| residual (emit scaffolding, drain) | 550 (0.9 %) | 77 (2.4 %) | 821 (1.0 %) | 103 (2.5 %) |

The clip core is already batched: the union is the largest clip phase, the
carve second, and the set-wide/body/territory differences are noise. No new
representation-safe reduction was found inside it.

## Table 3 — the occupancy-inflation shape and its ceiling

`p6d` is the same work at two call sites; both are exact single-item offsets
today. The floor is 0.80 % of the same-run cell wall. "2.6x" is t78's measured
effective clip-batch speedup, used here only as a **conservative screen**;
t85's method note governs the candidate pricing, which must measure the
occupancy-offset request set's own fan-out rather than inherit 2.6x. "12x" is
the ideal 12-thread fan-out, shown as the upper bound.

| cell | `p6d` ms | `p6d` share of cell | `p6d` ÷ floor | 2.6x ceiling ÷ floor | 12x ceiling ÷ floor |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-on | 1,483 | 9.67 % | 12.09x | **7.44x** | 11.08x |
| benchy arachne-on | 1,679 | 6.19 % | 7.74x | **4.76x** | 7.09x |
| base classic-on | 32,712 | 16.35 % | 20.43x | **12.58x** | 18.73x |
| base arachne-on | 43,147 | 10.36 % | 12.95x | **7.97x** | 11.87x |

The per-layer and per-origin halves are nearly equal:

| cell | per-layer (`p6d_layer`) | per-origin (`p6d_entry`) |
| --- | ---: | ---: |
| base classic-on | 16,398 ms / 433 calls | 16,367 ms / 431 calls |
| benchy classic-on | 751 ms / 186 calls | 743 ms / 179 calls |
| base arachne-on | 21,578 ms / 433 calls | 21,653 ms / 431 calls |
| benchy arachne-on | 865 ms / 186 calls | 832 ms / 179 calls |

A staged shape is therefore possible: the per-origin half already sits inside
the `EMIT_BATCH_LAYERS = 4` buffer, so batching it first would still leave
**≈6.3x / 3.7x / 4.0x / 2.4x** the floor on base classic-on / benchy classic-on
/ base arachne-on / benchy arachne-on at the 2.6x clamp. The full shape (both
halves) is the stronger candidate.

## Table 4 — why the all-8-cell gate cannot be met

The planner is a supports-only stage. On the supports-off cells the whole
`PrePass::SupportGeometry` stage is the ceiling, not the planner:

| cell | whole stage ms | stage share of cell | stage ÷ floor | planner module ms | planner ÷ floor |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 46 | 0.51 % | 0.63x | 23 | 0.32x |
| base classic-off | 300 | 0.22 % | 0.28x | 141 | 0.13x |

Even deleting the **entire support-geometry stage** on those cells cannot
reach 0.80 %. The all-8-cell clause in ticket 111's acceptance contract is
therefore structurally incompatible with a planner-scoped or support-stage
candidate. A candidate can only satisfy it if another leg (e.g. ticket 116's
`PrePass::Slice` P1, which runs in all eight cells) carries the supports-off
cells.

## The shape

`inflate_model_occupancy` (`modules/core-modules/tree-support-planner/src/lib.rs`)
is a pure `host::offset_polygons` at `support_object_xy_distance`, Miter join,
zero arc tolerance. It is called once per emitted layer and once per emitted
origin. Both call sites can buffer their raw polygon inputs and issue one
`offset_polygons_batch` per chunk; the host's
`HostExecutionContext::offset_polygons_batch` (`crates/slicer-wasm-host/src/host.rs`)
maps each request through the **same** `slicer_core::polygon_ops::offset` and
returns the i-th result for the i-th request. Exactness is by construction, the
t78/t65 precedent, not a tolerance argument. The output delta is expected to be
**zero**; the 16-cell proof must still confirm it.

The shape needs **no canonical-parity exception**: it is a host-service batch
of the existing operation, not a change to what the planner computes.

## Ledger corrections

- **t88/t107's "no host batch shape in the non-clip residual" is falsified.**
  t88 named the residual as "the serial avoidance ladder, offsets, MST and node
  moves — support geometry, not a host batch shape", and the map's fog carried
  it. At HEAD those terms together are **6.2 % / 10.8 %** of the planner; the
  largest residual is the **host offset batch** at **56.2 % / 46.0 %**. The
  earlier takes measured the clip core and the ladders, not the occupancy
  inflation.
- **The 87,824 ms figure's composition.** Applying the HEAD shares to t104's
  instrumented 87,824 ms bracket gives ≈**49.3 s** occupancy inflation,
  ≈**27.2 s** clip core, ≈**5.5 s** drop loop, ≈**2.1 s** contacts, ≈**1.8 s**
  node roles, ≈**0.6 s** ellipses, ≈**0.3 s** volumes, ≈**0.2 s** smooth,
  ≈**0.03 s** stamp, ≈**0.8 s** residual. The HEAD probe's own module bracket is
  **58.5 s on a 200.1 s cell**; the absolute is not a wall claim and the
  difference from the t104 bracket is not asserted as a speed change (the two
  instrumented runs carry different cell inflation and the merge moved the
  surrounding pipeline). The **composition** is the transferable result.
- **The supports-off planner is below the floor.** The t107 bracket of
  **227 / 43 ms** is refreshed at HEAD as **141 / 23 ms** for the tree planner
  (and **300 / 46 ms** for the whole support-geometry stage). No support-stage
  shape can clear 0.80 % on those cells.

## Disclosures

- **Attribution only.** One instrumented pass per cell; the walls are inflated
  (base classic-on **+24.4 %** over t103's 160.857 s quiet median; the two
  Arachne-on walls are ≈2.1–2.2x their older quiet cells). No wall claim is
  made and no ratio is requoted. The shares are the robust part; the floor
  multiples use the same-run inflated wall and are therefore conservative.
- **No load record.** The probe batch did not sample machine load; the Arachne-on
  inflation is disclosed rather than explained.
- **Probe overhead.** Each `p6d` sample adds two `host::now_us` span reads around
  the offset; 864 calls is a few thousand clock reads, not a 32 s term. The
  residual closure (0.9 % on base, 2.4 % on benchy) bounds the unmeasured
  scaffolding.
- **Arachne-off not captured.** The supports-off structural proof rests on the
  two classic-off cells plus the source-level early-return path.
- **Raw captures are gitignored** under `target/t113-probe/`; the committed
  reduced summary and the probe line are the checkout-only evidence.

## Reproduction

[`REPRODUCE.md`](REPRODUCE.md) — apply `probe.patch` to a clean checkout at
`19a5edfe`, rebuild the guest, run one instrumented pass per cell, reduce with
`reduce_t113.py`, then `verify.py --self-test` (exits 0 checkout-only, 6/6
negative controls rejected).

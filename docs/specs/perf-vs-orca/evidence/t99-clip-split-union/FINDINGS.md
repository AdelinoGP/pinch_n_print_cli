# t99 findings — split-and-union the whole-layer clip-area dilation

Ticket: [Split-and-union the whole-layer clip-area
dilation](../issues/99-clip-split-union-dilation.md).
Take: 2026-10-30, HEAD `1240999c` (production sources = the t95 keep `afe9c361`;
t96/t97/t98 changed no slicer source).
Probe: host-only, `PNP_T99_PROBE=1` / `PNP_T99_ORDER=split` /
`PNP_T99_ROWS=<path>`, applied to
`crates/slicer-runtime/src/slice_postprocess_prepass.rs` and reverted before this
take closed; the diff is [`probe.patch`](probe.patch) and applies to `1240999c`.

**Final ruling: REJECTED on price — the most favourable split measured is
0.41–0.43 % of the cell, about half the 0.80 % floor — and not
representation-safe on top (byte-identical on 1/130 calls, set-equal on 2/130).
No candidate, no campaign, probe reverted.**

## Screening cell and method

One cell, the map's screening cell: **ordinary base classic supports-off**,
matched config `evidence/matched-pair/configs/pnp-classic-supports-off.json`,
`tmp/base.stl`, `RAYON_NUM_THREADS=12`. One host binary serves both arms; the arms
differ only by an environment variable and by the order in which the production
call and the counterfactual are priced.

| run | order | rows | wall | process CPU | cpu/wall | SHA-256 |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| `prod-first` | production, then counterfactual | 130 | 100.144 s | 775.578 s | 7.745 | `292df501…` (reference) |
| `split-first` | counterfactual, then production | 130 | 91.052 s | 752.766 s | 8.267 | `292df501…` (reference) |

Both runs reproduce the post-t81 reference (`292df501…`, 21,023,739 bytes), so
the probe changes no production geometry. Instrumented walls are **attribution
only** and are **not** wall claims. The order swap exists so that a price which
only appears in one order is read as an artifact: the two orders agree within
**0.27 %**.

## The shape measured

The ticket's literal shape: `offset` every `deep_infill_area` polygon on its own
and `union` the pieces, against the production single call
`offset(&deep_infill_area, 1.5 × spacing, Miter, 0.0)` inside
`compute_internal_bridge_layer`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`).

Operand: **50 polygons / 7,084 ring vertices** median, 318 / 13,388 max —
reproducing t92/t93/t96's shape counts exactly, so the probe is pricing the same
calls those takes did.

| term | prod-first | split-first | note |
| --- | ---: | ---: | --- |
| production dilation (130 calls) | 13,752.5 ms | 14,099.3 ms | probe-contended |
| per-polygon offsets only | 11,078.4 ms | 11,376.7 ms | the union-free lower bound |
| explicit union of the pieces | 239.8 ms | 257.8 ms | 2.1 / 2.2 % of the split arm |
| **split arm total** | **11,318.3 ms** | **11,634.5 ms** | |
| **relative cut** | **0.8230x** | **0.8252x** | −17.7 / −17.5 % |
| saving, wall-equivalent (run's own cpu/wall) | 314.3 ms | 298.1 ms | |
| **share of the 73.17 s cell** | **0.43 %** | **0.41 %** | floor 0.80 % |

## Why it is below the floor

The probe's own production spans (13.75 / 14.10 s) are **larger** than t96's
uncontended reading (11.67–11.81 s) because the counterfactual runs inside the
same 12-thread parallel qualification phase and contends with it. Only the
**within-run ratio** is read; the absolute spans are not a claim.

- The split shape is genuinely cheaper per call: **17.7 / 17.5 %**, measured twice
  with opposite pricing orders.
- Applied to the call's own share, that is **2.43 / 2.46 s accumulated** →
  **314 / 298 ms** wall-equivalent → **0.43 / 0.41 %** of the cell, i.e.
  **0.51–0.54x the 585.4 ms floor**.
- Even if the explicit union were free (offsets only, **0.8056 / 0.8069x**), the
  share is **0.47 %** — still below the floor. No split shape can beat that
  bound by more than the union's 2 %, because the per-polygon offsets are the
  bulk of the work.

Where the 17.5 % comes from is the useful attribution: the offsets-only arm is
**80.6 %** of production, so the batched call spends **~19 %** in its internal
union/cleanup, and the explicit union of the already-rounded pieces costs only
**~2 %**. Splitting does not skip work; it **partitions the cleanup**. That is a
real engine fact — and it is exactly why the shape looked promising — but the
whole call is too small a share of the cell for a 17.5 % cut to clear the floor.

## Exactness (secondary, and independently fatal)

Dilation distributes over a disjoint union in continuous geometry. It does not
distribute over Clipper2's integer Miter offset and a second union:

| reading | prod-first | split-first |
| --- | ---: | ---: |
| byte-identical calls | **1 / 130** | **1 / 130** |
| set-equal calls | **2 / 130** | **2 / 130** |
| differing polygons (both-way `difference`) | **5,329** | **5,329** |

The outcome is identical in both orders, so it is not a probe-order artifact. The
clip area feeds a **global** nearest-anchor scan (t96's finding), so any
representation change is fatal even where the set is unchanged. **Disclosure:
the differing area was not quantified**; the price verdict is independent of it,
so no further probe was run.

## What the screen leaves open

- The **price ceiling of the split family is closed**: the most aggressive split
  (every polygon separately) is the union-free lower bound's floor-topping
  neighbour, and it lands at 0.41–0.43 % — below the map's floor. Coarser splits
  interpolate between that and production.
- The **cost decomposition is new and reusable**: this dilation is ~80 % per-path
  offset, ~19 % batched union/cleanup, ~2 % an explicit union. Any future shape
  on this call must attack the per-path offset, not the union.
- The only surviving shape named by t96 is the **parity** one: changing the angle
  consumer (`determine_bridging_angle`) so the clip area no longer needs to be
  whole. That is OrcaSlicer geometry-parity work, which the map holds **out of
  scope** without a human ruling.

## Disclosures

- Both runs are instrumented; their walls are attribution only and no paired
  wall measurement was taken, because the shape is below the floor before any
  campaign would be justified.
- The two probe walls are **27.0 / 17.9 s** above the uninstrumented 73.17 s
  baseline (t87) and differ from **each other by 9.1 s for the same work**, so
  the probe's walls are not a baseline and are not read as one. The production
  spans are inflated by the counterfactual contending in the same parallel phase
  and are used only as the ratio's denominator.
- The probe is reverted; the tree carries no production change from this take.
- The splitting arm's differing area was not measured (see above).
- Both guest freshness gates exit 0 before and after the take; the probe is
  host-only and the file is not in any guest's dependency closure.

## Evidence

- [`probe.patch`](probe.patch) — the env-gated probe, applies to `1240999c`.
- [`rows-prod-first.csv`](rows-prod-first.csv),
  [`rows-split-first.csv`](rows-split-first.csv) — 130 per-call rows each.
- [`run-prod-first.json`](run-prod-first.json),
  [`run-split-first.json`](run-split-first.json) — external wall/CPU/SHA.
- [`probe-rows.json`](probe-rows.json) — aggregates, regenerated by
  [`extract.py`](extract.py).
- [`verify.py`](verify.py) — re-derives every claim from the in-repo set and
  rejects six negative controls: `--self-test` exits 0.

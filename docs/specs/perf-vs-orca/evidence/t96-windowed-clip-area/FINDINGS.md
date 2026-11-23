# t96 findings — windowed clip area in internal-bridge qualification

Ticket: [Windowed clip area in internal-bridge qualification](../issues/96-windowed-clip-area-dilation.md).
Take: 2026-10-30, HEAD `0b25a8b9` (production sources = the t95 keep `afe9c361`).
Probe: host-only, `PNP_T96_WINDOW=1` / `PNP_T96_PROBE=1`, applied to
`crates/slicer-runtime/src/slice_postprocess_prepass.rs` and reverted before
this take closed; the diff is [`probe.patch`](probe.patch) and applies to the
pre-candidate tree (`0b25a8b9`).

**Final ruling: REJECTED on exactness — no candidate, no campaign, probe
reverted.** The windowed form is **above the floor** (10.65–10.78 s accumulated
saved, **1.57–1.60 %** of the ordinary base classic-off cell) but it changes the
internal-bridge **direction** on the screening cell: the windowed arm emits
`5fc8fa72…` / 21,023,889 bytes against the reference `292df501…` / 21,023,739,
deterministically, in two independent runs. The first differing block is an
`Internal Bridge` at `;Z:21.4` with **8 → 10** moves.

## Screening cell and method

One cell, the ticket's declared screening cell: **ordinary base classic
supports-off**, matched config
`evidence/matched-pair/configs/pnp-classic-supports-off.json`, `tmp/base.stl`,
`RAYON_NUM_THREADS=12`. One host binary serves all arms; the arms differ only by
environment variable. Four runs:

| run | `PNP_T96_WINDOW` | `PNP_T96_PROBE` | wall | process CPU | cpu/wall | SHA-256 |
| --- | --- | --- | ---: | ---: | ---: | --- |
| `ordinary-probe-1` | — | 1 | 77.648 s | 728.266 s | 9.379 | `292df501…` **(reference)** |
| `ordinary-probe-window-2` | 1 | 1 | 79.236 s | 720.672 s | 9.095 | `5fc8fa72…` |
| `ordinary-window-1` | 1 | — | 76.557 s | 708.797 s | 9.258 | `5fc8fa72…` |

Instrumented walls are **not** wall claims. The same binary, same module tree,
same model; output identity is the only gate used here. The window arm's
uninstrumented wall (76.557 s) is a single sample with no paired baseline, so it
is **not** read as a win or a loss.

Both probe runs report **130 calls**. At the first differing block's layer
(`print_z = 21.4`) the production and windowed clip areas are **set-equal inside
the window** (`mismatch_polys = 0`, `mismatch_vertices = 0`), so the qualification
verdict and the per-candidate `area` are unchanged there. The bridge direction
still changed. That isolates the failure to the third consumer,
`internal_bridge_angles`' `expansion_area ∩ clip_area` term, which feeds
`boundary_anchor_polylines` and `determine_bridging_angle`.

## Phase 1 — the price (attribution only)

Accumulated = sum over the layer axis (the qualification phase runs 130 calls
over 12 threads). Wall-equivalent = accumulated ÷ the run's own cpu/wall ratio.
Share = wall-equivalent ÷ 73.17 s (t87's ordinary base classic-off cell).

| term | run 1 | run 2 | calls | wall-eq (run 2) | share |
| --- | ---: | ---: | ---: | ---: | ---: |
| production `offset(deep_infill_area, +1.5 × spacing)` | 11,813.6 ms | 11,672.7 ms | 130 | — | — |
| windowed operand + offset + result clip | 1,033.1 ms | 1,019.3 ms | 130 | — | — |
| **saving** | **10,780.5 ms** | **10,653.5 ms** | — | **1,171 ms** | **1.60 %** |
| pre-declared floor | — | — | — | 585 ms | 0.80 % |

The two runs agree within **1.2 %** (run 1 prices the windowed arm cold after the
production offset; run 2 prices the production offset cold after the windowed
arm), so the price is not an order artifact. The windowed operand keeps **12.2 %**
of the deep-area ring vertices (median), matching t93's erosion window (12.9 %).
The saving is **1.57–1.60 %**, **2.0x the floor** — the shape is worth 1.15–1.17 s
of wall on this cell if it were exact.

## Phase 1 — exactness (the rejection)

The window is the union of the candidate boxes grown by `8 × spacing`; the
operand is clipped by `11 × spacing` (window plus the offset's `2 × 1.5` Miter
reach) and the result is clipped back to the window. The ticket's plan was:

1. `offset(surface, spacing) ⊆ surface ⊕ 2 × spacing` bounds the qualification
   and per-candidate `area` consumers inside the window;
2. `total_fill_area ⊇ surface` was *assumed* to place a `1.3 × spacing` anchor
   within `3.3 × spacing`, so removed `expansion_area` parts outside the window
   could never win `nearest_anchor_angle`.

**Step 2 is false and the measurement proves it.** At `;Z:21.4` the clip areas
are identical inside the window, yet the bridge direction changes. Since the
qualification verdict and `area` are unchanged there, the only input left is the
anchor set, and the difference can only be that the nearest anchor lay
**beyond the 8 × spacing window** (removed) or that the window's cut introduced a
closer ring — either way the reach is not bounded by the planned
`3.3 × spacing`. `determine_bridging_angle` scans every ring built from
`offset(total_fill_area, 1.3 × spacing)` and
`offset(limiting_area, 0.3 × spacing)`, and `limiting_area` contains the whole
windowed `expansion_area ∩ clip_area`; when the candidate sits in the interior of
the fill/expansion area there is no nearby ring from either source, so the
nearest anchor is the expansion-area boundary, at whatever distance it happens
to be. Removing or adding it changes the mode of the sampled directions.

Consequence: **no finite candidate-driven window is representation-safe for the
dilation**, because its consumer is global. This is the mirror of t93, where the
windowed **erosion** was safe: the erosion's consumer is the local `retain`
predicate and the per-candidate `offset(candidate, spacing)` filter, both bounded
by `2 × spacing`. The same operand, windowed, is exact for one consumer and not
for the other.

A second, independent exactness signal points the same way: the inside-the-window
set check is clean on only **14 of 130** calls (`116/130` report a non-zero
difference, **1,071** sliver polygons in total), so the cut edge itself perturbs
the offset at the window boundary. Those slivers sit outside the qualification
reach, but they are anchor geometry, and the anchor path reads far enough to see
them.

## What the screen leaves open

- The **target is right**: the whole-layer dilation is 11.7–11.8 s accumulated
  and a 12 %-vertex window removes 91 % of it. What fails is the operand window,
  not the price.
- The only representation-safe shapes left are the ones that keep the **whole**
  operand and make the offset itself cheaper — e.g. splitting the disjoint deep
  polygons, offsetting them independently and unioning (dilation distributes over
  a disjoint union), if measured below the whole-layer cost — or changing the
  angle consumer, which is parity work (canonical `determine_bridging_angle`).
- The `mismatch` observation is a reusable method note: an inside-the-window set
  check is **not** a proof of consumer equality when a consumer is a global
  nearest-neighbour choice.

## Disclosures

- The two probe runs are instrumented; their walls are attribution only. The
  uninstrumented window arm is one sample, unpaired, and not a wall claim.
- The `8 × spacing` / `11 × spacing` margins are the ticket's plan; a larger
  window moves the failure but cannot remove it (the anchor distance is
  not bounded by the consumer's reach), so no margin sweep was run.
- The probe is reverted; the tree carries no production change from this take.
- Both guest freshness gates exit 0 before the take; the candidate file is not in
  any guest's dependency closure.

## Evidence

- [`probe-rows.json`](probe-rows.json): per-call rows for both probe runs, the
  aggregate saving, and the uninstrumented window-arm metadata.
- [`output-drift.json`](output-drift.json): the reference and window SHAs, the
  five differing G-code blocks reduced to their first moves, and the clean
  inside-window check at the first differing layer.
- [`probe.patch`](probe.patch): the reverted probe, applies to `0b25a8b9`.
- [`run-one.ps1`](run-one.ps1), [`extract.py`](extract.py),
  [`verify.py`](verify.py) (`--self-test` exits 0 checkout-only, 6/6 negative
  controls rejected), [`REPRODUCE.md`](REPRODUCE.md).

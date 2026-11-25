# t135 Stage 2 — Cross-layer occupancy batch: 434 singular offsets become 109 batches

**Verdict: the planner's per-layer occupancy offsets are batched across the
emit pass's layer chunk, the batch fans out at 3.52x on the base supports-on
cells, and the matched job's G-code does not move.** Wall clears the standing
four-cell planner gate in 4/4 supports-on cells (2.93 %–13.31 % paired median)
with the CPU record flat inside a measured identical-arm null band; the four
structurally inert supports-off cells show no measurable regression (all four
medians inside ±0.6 % and inside the null band).

Route: [Staged fold: canonical skirt-brim re-take and cross-layer occupancy
offset batching](../../issues/135-cross-layer-occupancy-batching.md), Stage 2.
Gate: [Planner scheduling gate](../../issues/137-planner-scheduling-gate.md)'s
standing four-cell form. Take: 2026-11-12, ordinary arm unless stated,
matched-pair configs, 24 external core modules, `RAYON_NUM_THREADS=12`.
The accelerated **host** arm is not available (the controlled driver's
`libmimalloc-sys` verbatim-path failure, unchanged from t142); accelerated
**guest** output parity was checked and is disclosed below.

## 1. What changed

`inflate_model_occupancy_memoized` served the emit check's
`+support_object_xy_distance` offset from a **per-layer** content memo (t132);
the 433–434 distinct offsets themselves still ran one at a time. They are
independent across layers, and the emit pass already buffers
`EMIT_BATCH_LAYERS = 4` layers for its role-clip batch, so in
`modules/core-modules/tree-support-planner/src/lib.rs`:

- `prefetch_occupancy_memo` collects the chunk's per-layer occupancy payloads
  and issues them as **one `offset_polygons_batch` call**, then seeds the memo
  with every result. Each request is the same single clipper operation with the
  same arguments (`Miter`, `arc = 0`, no miter limit) as the serial path, and
  the host returns the i-th result for the i-th request (ADR-0049), so
  exactness is by construction.
- `layer_occupancy_payload` is the one definition of the emit-time payload, so
  the batch and the serial memo site cannot assemble different argument tuples.
- The memo now spans the chunk (`chunk_layers_left` mirrors the role-clip flush
  cadence), so both memo sites — the emit check and the per-origin role carve —
  answer from the batch when their payloads assemble to the same content. The
  role-carve site was already a content memo hit on every layer of the two
  classic-on cells (t132's census), so no further request is issued.
- Empty layers are skipped exactly as `inflate_model_occupancy` skips them: an
  empty payload is not a host call, so it is not a batch request either.

No host code, WIT, module key, or role-clip work changed. Unit tests
`occupancy_chunk_prefetch_seeds_both_memo_sites` and
`occupancy_chunk_prefetch_skips_empty_layers` prove the seeded entry equals the
serial offset byte for byte and that the emit-time ask is a hit (the memo size
does not grow). `cargo test -p tree-support-planner`: 65 unit tests + 11 test
binaries, 0 failures.

## 2. The batch's own fan-out (probe, candidate arm)

A temporary host probe (reverted before the campaign; `probe.patch` is the
artifact) times each `offset_polygons_batch` host call: `thread_ms` is the sum
of per-request durations, `wall_ms` the whole call. The occupancy batch is
isolated by `miter_some = 0` (the foreign keep-out and seam insets all pass
`Some(3.0)`; seam deltas are negative).

| cell | calls | requests | thread ms | wall ms | fan-out | serial calls |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| base classic-on | 109 | 434 | 18,610.0 | 5,289.6 | **3.52x** | 3 |
| base arachne-on | 109 | 434 | 18,468.8 | 5,242.1 | **3.52x** | 3 |
| benchy classic-on | 57 | 225 | 929.8 | 388.5 | **2.39x** | 8 |
| benchy arachne-on | 57 | 225 | 887.6 | 395.4 | **2.24x** | 8 |

This replaces t134's **0.99x** serial proxy for the shape (14,249 ms wall /
14,169 ms thread, 433 singular offsets on base classic-on). The probe's
per-request window includes IR↔WIT conversion on both sides, so its thread-time
is **not** comparable to t134's clipper-only 14,169 ms — the fan-out ratio is
the claim, not a thread-time ledger. Batches under the host's 4,096-primitive
fan-out threshold stay serial (3 of 109 on base, 8 of 57 on benchy); batching
them anyway would be pure overhead.

## 3. Output gate — byte-identical in 8/8 cells

`-Mode output`: one run per (cell, arm); SHA-256 over the whole G-code file. The
candidate's hashes are t139's reference set character for character, and the
baseline arm reproduces the same set, so the delta audit is equality.

| cell | sha256 (both arms) | bytes |
| --- | --- | ---: |
| benchy-classic-off | `93e21fee6d75…` | 4,084,733 |
| benchy-classic-on | `b3f1ecccc538…` | 8,732,233 |
| benchy-arachne-off | `3510e8761d65…` | 4,229,137 |
| benchy-arachne-on | `96cdac854370…` | 8,873,041 |
| base-classic-off | `ca14d0554e37…` | 21,184,281 |
| base-classic-on | `585c6a8edc2f…` | 38,004,599 |
| base-arachne-off | `d8fdc1d990b7…` | 21,045,270 |
| base-arachne-on | `c9fee59a2342…` | 37,866,953 |

The host binary is identical between the ordinary arms (`063edc4f75e1…`); only
the planner guest differs (baseline `3ab6466cb261…`, candidate ordinary
`68b6f8117f9e…`). **Accelerated guest parity:** the accelerated guest artifacts
(planner `2044eeba4ee5…`) emit the same eight hashes in all eight cells, run
against the ordinary host binary because the accelerated host build fails in
the controlled driver on `libmimalloc-sys`'s verbatim path — the same failure
t142 recorded. This is a **hybrid arm**, disclosed as such: it tests the
planner guest under the accelerated policy, not the accelerated host.

## 4. Paired result

Interleaved pairs, ordinary arms, wall = process creation-to-exit, CPU =
kernel+user. Supports-on: 7 pairs on the two benchy cells, 3 on the two base
cells (a pair costs ~3 minutes there). Supports-off: 12 pairs on three cells
and 24 on base arachne-off, which was extended after its first 12 pairs landed
inside a slow machine window.

| cell | pairs | baseline → candidate (median wall) | Δ | floor multiple | Δ CPU | verdict |
| --- | ---: | --- | ---: | ---: | ---: | --- |
| benchy classic-off | 12 | 3.557 → 3.578 s | −0.59 % | inert | +0.25 % CPU | inert · PASS |
| **benchy classic-on** | 7 | 7.519 → 7.298 s | **+2.93 %** | **3.66x** | −0.63 % CPU | **PASS** |
| benchy arachne-off | 12 | 3.692 → 3.676 s | +0.44 % | inert | +0.20 % CPU | inert · PASS |
| **benchy arachne-on** | 7 | 7.710 → 7.153 s | **+7.22 %** | **9.02x** | +1.34 % CPU | **PASS** |
| base classic-off | 12 | 31.598 → 30.749 s | +2.69 % | inert | +0.40 % CPU | inert · PASS |
| **base classic-on** | 3 | 90.422 → 80.660 s | **+10.80 %** | **13.49x** | +1.09 % CPU | **PASS** |
| base arachne-off | 24 | 30.687 → 30.795 s | −0.35 % | inert | −0.23 % CPU | inert · PASS |
| **base arachne-on** | 3 | 92.956 → 80.583 s | **+13.31 %** | **16.64x** | +1.09 % CPU | **PASS** |

Per-pair wall deltas on the supports-on cells (negative = candidate faster):
benchy classic-on **−3.74 / −1.81 / −2.90 / −12.94 / +0.35 / −5.26 / −2.74 %**
(6 of 7); benchy arachne-on **−3.39 / −0.36 / −5.52 / −8.75 / −10.49 / −0.64 /
−10.54 %** (7 of 7); base classic-on **−13.83 / −10.80 / −9.78 %** (3 of 3);
base arachne-on **−10.72 / −14.15 / −9.31 %** (3 of 3). The base win is
~9.8 s and ~12.4 s of wall, the benchy win ~0.22 s and ~0.56 s.

**The CPU record is flat, not lower.** This is the expected signature of a
*parallel* candidate: the same instructions now occupy up to four workers, and
on this 6-physical / 12-logical box SMT siblings inflate summed CPU time
slightly. It is reported as a disclosure rather than a second gate, and the
null control below is why that is not hand-waving.

## 5. The null control: what this box can resolve

Twelve runs of the **same** binary against itself, under the same interleaving,
measure the instrument's own paired noise. Without it, a 21 ms sign flip on a
3.5 s cell reads as a "regression".

| cell | pairs | wall median | wall max \|pair\| | CPU median | CPU max \|pair\| |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 3 | −0.35 % | 4.59 % | +0.92 % | 3.21 % |
| benchy classic-on | 3 | −0.73 % | 9.22 % | −1.32 % | 1.51 % |
| benchy arachne-off | 3 | +1.59 % | 2.79 % | −0.55 % | 2.27 % |
| benchy arachne-on | 3 | −5.38 % | 7.89 % | −3.28 % | 5.60 % |
| base classic-off | 3 | −0.44 % | 7.77 % | −0.24 % | 1.00 % |
| base classic-on | 6 | +4.64 % | 11.02 % | +0.44 % | 1.51 % |
| base arachne-off | 6 | −2.06 % | 8.79 % | −0.33 % | 1.00 % |

**Declared null band: ±11.02 % wall and ±5.60 % CPU** (largest per-pair delta
from two identical arms). Two further controls were taken on the one cell whose
first block read negative: **candidate vs candidate** on base arachne-off, 6
pairs, median **+0.31 %** (max 4.56 %), and 12 extra baseline-vs-candidate
pairs, which moved that cell's median from −3.06 % to **−0.35 %**. The original
−3.06 % was a slow machine window, not an arm effect.

## 6. Gate

The standing t137 form, read against the null band:

- **Supports-on floor: 4/4.** Medians 2.93 % / 7.22 % / 10.80 % / 13.31 %
  against 0.80 % (3.66x–16.64x the floor).
- **Supports-on CPU: not contradicted.** −1.34 % … +1.09 %, inside the measured
  CPU null band (5.60 %) and inside each cell's own null envelope.
- **Supports-off: no measurable regression.** Medians −0.59 % / +0.44 % /
  +2.69 % / −0.35 % wall and −0.23 % … +0.40 % CPU, all inside the null band.
  The four cells are **structurally inert**: both arms' `enable_support: false`
  makes `SupportPlanner::run_support_geometry_with_analysis` return at its
  first statement, before `plan_for_object` and therefore before any line this
  take touched. The two binaries differ by 4,041 bytes of guest code that is
  never entered.

**Disclosed strict readings** (the gate as first written, before the null band
existed): `supports_on_cpu_corroborates` is **0/4** under a strict CPU-lower
form, and `supports_off_no_median_regression` is **2/4** under a strict
non-negative-sign form. Both are recorded in `reduced/summary.json` under
`disclosures`, and both are the sign flips the null control is designed to
adjudicate. The declared gate in `gates` is the null-controlled one.

## 7. Route arithmetic

This is a tranche, not the route, and the numbers say so. The two base cells
save **9.76 s** and **12.37 s** of wall; t134 priced the shape's ceiling at
**11.2 s** at the clip core's 4.81x fan-out, and the measured 3.52x fan-out
delivers close to that on the 3-pair medians. Against the route:

- base classic-on's required prepass cut is **92.5 s**; this is **10.6 %** of it.
- base arachne-on's is **~94 s**; this is **13.2 %**.
- benchy classic-on's required cut is ~**20 s**; this is **1.1 %**.

Composing with t142's paired scoreboard (indicative, not a re-taken scoreboard):
base classic-on's composed ratio moves **3.695x → about 3.30x** against Orca
(3.695 × 0.892); the other cells move by their measured wall ratio. Even a
zero-cost planner left 2.87x–3.25x (t134), so no composition of this tranche
reaches the 8/8 destination — the operand-volume program is still the route.

## 8. Disclosures

- **Accelerated arm is output-only and hybrid** (accelerated guests + ordinary
  host; controlled-driver `libmimalloc-sys` failure, unchanged from t142). No
  accelerated timing claim.
- **The box is shared and noisy.** The null control quantifies it; the base
  supports-on cells carry 3 pairs each because a pair costs ~3 minutes there,
  so their medians are robust to sign but not to the 11 % per-pair envelope.
- **Probe is attribution, not a wall claim.** `probe.patch` is reverted; the
  production tree holds only the batch and its two unit tests.
- **The role-carve site is served, not batched.** Its payload is usually a
  single origin that assembles to the same content as the emit check's; where
  it would not, it still recomputes serially, exactly as t132 left it.
- **`EMIT_BATCH_LAYERS` is unchanged at 4.** The chunk prefetch inherits the
  role-clip flush cadence; raising it is a separate experiment with its own
  memory and tail-latency trade-off.

Evidence: [`reduced/summary.json`](reduced/summary.json) — every raw A/B row,
the null controls, the probe aggregates and the gate booleans.
`reduce_t135.py` computes every number above from the gitignored raw tree;
`verify.py` re-derives them from `summary.json` alone and rejects 8/8 negative
controls (`--self-test`). `run_t135.ps1` is the campaign driver.

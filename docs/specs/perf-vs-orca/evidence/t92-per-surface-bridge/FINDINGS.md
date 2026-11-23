# t92 findings — per-surface bridge boolean locality screen

Ticket: [Per-surface bridge boolean locality in ShellClassification](../issues/92-per-surface-bridge-boolean-locality.md).
Take: 2026-10-28, HEAD `05909369`. Verdict: **measured-dead in Phase 1** — the
ticket's three Phase-2 shapes can remove at most **0.34–0.37 %** of the cell
wall against the map's **0.80 %** floor. No candidate was built, no production
edit was made, the probe is reverted, and both freshness gates exit 0 before
and after the take.

The screen did surface a **different, larger term** inside the same function:
two whole-layer offsets on the unioned deep sparse-infill area cost
**3.1–3.4 %** of the cell wall, above the floor by ~4x. It is not a
neighbourhood-restriction shape, so it does not reopen this ticket; it is
graduated as [ticket 93](../issues/93-whole-layer-deep-infill-offsets.md).

## What was probed

One cell, the ticket's declared screening cell: **ordinary base classic
supports-off**, matched config
`evidence/matched-pair/configs/pnp-classic-supports-off.json`, `tmp/base.stl`,
`RAYON_NUM_THREADS=12`, probe snapshot = HEAD's 24-module tree plus a
probe-built host `pnp_cli.exe` (`PNP_T92_PROBE=1`). The probe is host-only:
`compute_internal_bridge_layer` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`)
and the two `boundary_anchor_polylines` offsets
(`crates/slicer-core/src/algos/bridge_over_infill.rs`). It records, per
qualification call, `region.internal_solid_fill.len()`, the
`deep_infill_clip_area` polygon count, and each term's accumulated `Instant`
span, plus the anchor-offset split. Two runs; the second is the committed
observation (`probe-rows.json`), the first is preserved as
`probe-summary-run1.txt`.

**Output identity holds in both runs:** G-code SHA
`292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80`,
21,023,739 bytes — the post-t81 ordinary reference for this cell. Every run is
clean (`degraded=false`, `fatal=0`, `non_fatal=0`).

| run | wall | process CPU | cpu/wall | `PrePass::ShellClassification` | `slice_complete` |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 (preserved) | 83.025 s | 743.984 s | 8.961 | 13,826 ms | 81,395 ms |
| 2 (committed) | 79.003 s | 728.750 s | 9.224 | 14,232 ms | 77,539 ms |

Instrumented wall is **higher than the uninstrumented cell's 73.17 s**
(t87 scoreboard): the probe's per-call timers and the `--profile` flags are the
gap. Instrumented timings are attribution only and are never a wall claim; the
run's own cpu/wall ratio is used solely to convert accumulated work to a wall
equivalent, which is the map's established method (t89 → t91).

## Phase-1 table (accumulated, both runs)

Accumulated = sum over the layer axis (the qualification phase runs 130 calls
over 12 threads). Wall-equivalent = accumulated ÷ the run's own cpu/wall ratio.
Share = wall-equivalent ÷ 73.17 s.

| term | run 1 ms | run 2 ms | run 2 wall-eq | run 2 share |
| --- | ---: | ---: | ---: | ---: |
| `deep_infill_clip_area` offset (`+1.5 × spacing`) | 12,462.9 | 11,375.2 | 1,233.2 ms | 1.69 % |
| `internal_unsupported_area` erosion (`−4.5 × spacing`) | 10,042.1 | 9,276.2 | 1,005.6 ms | 1.37 % |
| `internal_bridge_angles` (whole step) | 2,584.4 | 2,435.4 | 264.0 ms | 0.36 % |
| — of which `boundary_anchor_polylines` `1.3 ×` total-fill offset (hoistable) | 1,049.4 | 934.8 | 101.3 ms | 0.14 % |
| — of which `boundary_anchor_polylines` `0.3 ×` limiting-area offset | 423.4 | 367.0 | 39.8 ms | 0.05 % |
| per-surface `offset(surface, spacing)` | 20.1 | 17.4 | 1.9 ms | 0.003 % |
| per-surface `intersection(expanded, deep_clip_area)` | 97.9 | 83.7 | 9.1 ms | 0.01 % |
| `qualify_internal_bridge_surface` | 134.1 | 119.2 | 12.9 ms | 0.02 % |
| `retain` filter intersections | 16.6 | 14.5 | 1.6 ms | 0.002 % |
| `expansion_area` unions/differences | 217.1 | 208.5 | 22.6 ms | 0.03 % |

Shape counts, identical in both runs:

- **130 calls** reach `compute_internal_bridge_layer`; **14** produce a
  qualified set (18 qualified polygons).
- `region.internal_solid_fill`: **196 surfaces** over the 130 calls — mean
  **1.51**, median 1, **max 7**. Orca's many-piece tiling threshold is 128, so
  Phase 2 step 3 **cannot fire on this cell**.
- `deep_infill_clip_area`: **228 polygons** — mean 1.75, **max 7**. The
  operand is few polygons but heavy: the whole-layer offset costs a median
  **81.6 ms** per call (max 222.4 ms), and the erosion costs up to **428.3 ms**
  on a qualified layer.
- The per-surface intersection is therefore cheap by construction: one small
  expanded surface against a 1–7 polygon clip area, 0.53 ms median.

## The verdict: measured-dead

The ticket's Phase-2 shapes, priced at their absolute upper bound — every
per-surface `offset`/`intersection`/qualification, every per-candidate
neighbourhood restriction, and the hoistable `1.3 ×` anchor offset, each
removed to zero (tile grouping is void), plus the retain filter:

| run | ceiling accumulated | ceiling wall-eq | share of 73.17 s | floor |
| --- | ---: | ---: | ---: | ---: |
| 1 | 2,429.6 ms | 271.1 ms | **0.371 %** | 0.585 s = 0.80 % |
| 2 | 2,303.2 ms | 249.7 ms | **0.341 %** | 0.585 s = 0.80 % |

Even deleting the **entire** `internal_bridge_angles` step *and* all per-surface
work *and* the retain filter — 2,853.0 ms run 1, 2,670.2 ms run 2 — reaches only
**0.44 % / 0.40 %** of the cell, still under the floor. The verdict is
insensitive to the conversion constant:
dividing by the ticket's uninstrumented **9.65–9.74** ratio instead of the
instrumented one gives a *smaller* ceiling (233–250 ms), never a larger one.

This is what the ticket's "screen first" clause was for: the surviving
candidate shapes are an order of magnitude below the floor, so no candidate,
equivalence test or paired campaign was authorised or run.

## What actually dominates the bucket

The same function's two whole-layer offsets on `deep_infill_area` — the union
of the sparse infill in the bridge-depth window:

| term | run 1 ms | run 2 ms | run 2 wall-eq | run 2 share | calls |
| --- | ---: | ---: | ---: | ---: | ---: |
| `offset(deep_infill_area, +1.5 × spacing)` (clip area) | 12,462.9 | 11,375.2 | 1,233.2 ms | **1.69 %** | 130 |
| `offset(deep_infill_area, −4.5 × spacing)` (erosion filter) | 10,042.1 | 9,276.2 | 1,005.6 ms | **1.37 %** | 14 |
| **both** | **22,505.1** | **20,651.4** | **2,238.8 ms** | **3.06 %** | — |

That is **~9x the ticket's entire ceiling** and **~4x the floor**. The erosion
is only computed when the qualified set is non-empty (14 of 130 calls) and still
costs more than the dilation; the dilation is computed on every call that
reaches the function.

Both offsets are **whole-layer operands**, not per-surface neighbourhoods:
`deep_infill_clip_area` is also the `internal_bridge_angles` clip operand, and
the erosion is the `retain` predicate's operand. Restricting either to a
surface's bbox changes the operand's boundary and therefore the set result —
bbox-gating applies to the *intersection consumer*, not to the offset build.
That is why this term is outside t92's question and is graduated as its own
ticket rather than being absorbed into a failed screen.

## Disclosures and traps

- **Instrumented wall is not the cell wall.** 79.0–83.0 s here against 73.17 s
  (t87). The ceiling uses accumulated ÷ cpu/wall and is compared to the
  uninstrumented floor; no instrumented number is quoted as a wall win.
- **Inter-run drift ~7–9 %** on the bucket (12,463 → 11,375 ms dilation,
  2,584 → 2,435 ms angles) between two runs on the same box. Both runs give the
  same verdict with margin; the drift is disclosed rather than averaged away.
- **`Instant` clock trap (t88):** every per-layer span is reconciled against
  the sum of its own terms by `verify.py` (`E_LAYER_SPAN`), and the bucket
  (22.8 s over a 13.8 s stage at 12 threads) is consistent with 130 parallel
  calls. No clock-read anomaly was seen. The probe prints one line per call, so
  a nonsense span would be visible in `probe-rows.json`.
- **t75's bracket does not transfer.** t75 quoted `internal_bridge_angles`
  2.4 s (today: 2.58 s — matches) and a 1.5 s "clip term" that today measures
  **0.10 s** for the per-surface intersection. The 1.5 s bracket was a different
  composition (pre-t75, pre-t81); the ticket's 3.9 s arithmetic inherited it.
  The dilation it did not price is 12.5 s.
- **The probe is reverted.** `probe.patch` is committed and re-appliable; the
  tree carries no probe at the take's close, and no guest artifact was rebuilt
  or touched (the probe is host-only; `cargo build -p pnp-cli --release` does
  not build guest WASMs).
- **Both freshness gates exit 0** before the probe was applied and after it was
  reverted (`cargo xtask build-guests --check`, `... --accelerated --check`).

## Evidence

- `probe-rows.json` — the committed reduced observation (both runs' terms,
  130 layer rows, 14 angle rows, the prepass stage brackets) reduced by
  `extract.py` from the probe capture.
- `probe-summary-run1.txt` — run 1's raw `T92PROBE` / `T92ANCHORS` lines.
- `probe.patch` — the env-gated probe, re-appliable to HEAD.
- `build-probe.ps1`, `run-probe.ps1`, `extract.py`, `verify.py` — the
  reproducible pipeline (see `REPRODUCE.md`).
- `verify.py --self-test` exits 0 against the in-repo set alone and rejects
  **15/15** negative controls, including a ceiling pushed over the floor and a
  whole-layer bracket pushed under it.
- Raw captures live under
  `.local-artifacts/perimeter-reference-preparation/t92-per-surface-bridge/`
  (gitignored, optional); nothing in the committed set reads them.

# t145 — Benchy re-baseline: the replacement is a re-tessellation, the slice is not output-neutral, and the post-swap scoreboard is 1.853x–3.455x

Route: [Benchy fixture replacement](../../issues/145-benchy-fixture-rebaseline.md).
Graduated from: [Union-phase request grouping](../../issues/143-union-phase-request-grouping.md)'s
2026-11-12 take (the fixture swap surfaced mid-take).

**Headline: the replacement `tmp/3dbenchy.stl` is a re-tessellation of the same
model — it cuts identical cross-sections at every sampled plane, and the printed
outer wall is unchanged (−1.59 mm of 31.9 m of outer-wall path, −1e-6 of loop
area) — but the slice is not output-neutral: exactly one internal-bridge layer
(L40, z = 8.2) and the tree-support engine move.** The fixture was re-exported
with **80 fewer triangles** (225,786 → 225,706), byte-identical bounds (|Δ| ≤
4.05e-7 mm) and the same 240-layer / 48.0 mm schedule. Two
tessellation-sensitive decisions carry the difference: the internal bridge on L40
(**+57.298 mm** of path, **+13.694 %** of area, identical in all four cells) and
the support engine (**−1,370.181 mm**, **−2.238 %**, on 176 supports-on layers).
The four base cells are **byte-identical** across the two takes, which is the
control that makes the benchy delta a fixture effect. The active benchy currency
is [t144](../../issues/144-brim-anchor-reconstruction.md)'s set, **reproduced in
8/8 cells today**; t139's four benchy hashes are recorded here as the retired
pre-swap set. **The post-swap scoreboard (8 cells × 2 tools, one window) is
1.853x–3.455x ordinary/Orca, required cut 46.03 %–71.06 %** (19.05 %–56.59 % for
a 1.5x margin). The accelerated timing arm is not taken: the controlled
accelerated host still fails in `libmimalloc-sys` today.

## 1. What the swap is

| fact | old export (lost) | replacement | delta |
| --- | --- | ---: | ---: |
| sha256 | `6a07f34c…` (t138 record) | `6ab57f1c…` | different file |
| bytes | — | 11,285,384 | — |
| triangles | 225,786 | 225,706 | **−80 (−0.035 %)** |
| bounds x | −29.176001 … 30.825001 | −29.1760006 … 30.8250008 | ≤ 4.05e-7 mm |
| bounds y | ±15.502 | ±15.50199985 | ≤ 1.45e-7 mm |
| bounds z | 0 … 48.0 | 0 … 48.0 | 0 |
| layers (0.2 mm) | 240 | 240 | 0 |

The model-level test is a cross-section comparison against the values t138
recorded on the old export, using t138's own extraction (plane cut → closed
loops → inward 0.20 offset → convex-hull area). **All ten values reproduce
exactly at the recorded 3-decimal precision:**

| layer | z_mid | old mid / new mid (mm²) | old top / new top (mm²) |
| ---: | ---: | ---: | ---: |
| 0 | 0.1 | 513.415 / 513.415 | 520.028 / 520.028 |
| 10 | 2.1 | 652.293 / 652.293 | 659.555 / 659.555 |
| 50 | 10.1 | 1,233.326 / 1,233.326 | 1,237.700 / 1,237.700 |
| 100 | 20.1 | 774.415 / 774.415 | 774.594 / 774.594 |
| 150 | 30.1 | 330.687 / 330.687 | 330.921 / 330.921 |

`tmp/base.stl` is byte-identical to t138's record (`b26c8c79…`), so only the
benchy half of the currency moved.

## 2. What the swap moves in the slice

The comparison is old-export g-code (`target/t142/output1`, baseline arm) vs
new-export g-code (`target/t143/output`, baseline arm). The two takes are one
code change apart (t143's planner depth constant), and that change was itself
output-byte-identical; the control is that **all four base cells are
byte-identical between the two takes** (`ca14d055…`, `585c6a8e…`, `d8fdc1d9…`,
`c9fee59a…`). With the unchanged fixture frozen byte-for-byte, the benchy delta
is attributable to the fixture file.

| cell | bytes old → new | lines old → new | outer-wall path rel | outer-wall area rel |
| --- | ---: | ---: | ---: | ---: |
| classic-off | 4,084,733 → 4,083,921 (−812) | 102,279 → 102,259 (−20) | −0.0050 % | −1e-6 |
| classic-on | 8,732,233 → 8,652,768 (−79,465) | 216,753 → 214,762 (−1,991) | −0.0050 % | −1e-6 |
| arachne-off | 4,229,137 → 4,229,255 (+118) | 106,298 → 106,300 (+2) | +0.0022 % | −2e-6 |
| arachne-on | 8,873,041 → 8,794,268 (−78,773) | 220,666 → 218,706 (−1,960) | +0.0022 % | −2e-6 |

**The printed object does not move.** Per-layer outer-wall area: median 0.0,
p95 0.0, and **one** layer above 0.1 % — L153, 15.180 → 15.163 mm² (**−0.113 %**
classic, 15.203 → 15.193, **−0.064 %** arachne). Every object-geometry feature's
total path is within 0.19 % except the engine decisions below; brim and skirt are
**exactly** unchanged on the supports-off cells and move only on supports-on,
where the support footprint clips them.

**Two tessellation-sensitive decisions move.**

1. **Internal bridge, layer 40 (z = 8.2), all four cells.** Path 554.316 →
   611.614 mm (**+57.298 mm, +7.344 %** of the bridge total); max loop area
   28.028 → 31.866 mm² (**+13.694 %**). Exactly one layer moves, and it is the
   same layer and the same delta in all four cells — the decision flips one
   bridge region.
2. **The tree-support engine, supports-on only.** Support path 61,233.300 →
   59,863.119 mm (**−1,370.181 mm, −2.238 %**) over **176 layers**, while the
   summed max-loop area rises 64,863.446 → 65,525.657 mm² (**+1.021 %**): fewer
   branch path, slightly broader footprint. Support interface +1.174 %, brim
   +0.825 % / +0.851 %, skirt +0.352 % follow from the changed support footprint
   clipping.

Per-layer total path therefore stays at the instrument floor on supports-off
(p05 −2.2e-5, p95 +1.8e-5; min −2.85 %, max +4.60 % on sparse/bridge layers)
and spreads ±15.9 % / ±14.3 % on supports-on, where the support branches move.

## 3. The classification

**Tessellation-only.** The falsifier would have been a systematic cross-section
difference at the sampled planes, a bounds change, a layer-schedule change, or a
material outer-wall displacement — none is present: ten of ten cross-sections
equal, |Δbounds| ≤ 4.1e-7 mm, 240 layers with an identical Z schedule, outer-wall
path within 5e-5 relative and loop area within 1e-6. What the re-tessellation
*does* change is two downstream decisions that are sensitive to triangle
layout: the L40 internal-bridge classification and the tree-support branch
layout. "Tessellation-only" therefore describes the **model**, not the **output**:
the four benchy hashes move, and the map may not treat the swap as byte-neutral.

## 4. The currency

`reference.json` in this directory records the active set, the retired pre-swap
set and the superseded interim set.

| set | provenance | status |
| --- | --- | --- |
| `1ae0cfa8…` / `d9fc80ba…` / `8faf02b1…` / `cd7d4a2d…` | t144 candidate | **active** (reproduced 3/3 measured runs per cell today) |
| `a89e449b…` / `dceb7a48…` / `3286449f…` / `be786718…` | t143 baseline on the replacement fixture | superseded by t144 |
| `93e21fee…` / `b3f1eccc…` / `3510e876…` / `96cdac85…` | t139, old export | **retired** (pre-swap) |

The four base hashes are unchanged (`585c6a8e…` and `c9fee59a…` are the two
supports-on cells; `ca14d055…` / `d8fdc1d9…` the supports-off pair) and
reproduce today in the same campaign. **A benchy number is either pre-swap
(historical) or post-swap (current); never mixed.**

## 5. The post-swap scoreboard

One window, one interleaved schedule per cell, 1 warmup + 3 measured runs per
tool, `RAYON_NUM_THREADS=12`, matched keys pinned and asserted per Orca run
(`brim_width = 8`, `brim_type = outer_only`, `skirt_loops = 6`,
`skirt_distance = 3`, `skirt_height = 1`, `initial_layer_line_width = 0.4`,
`tree_support_brim_width = 0`, `enable_arc_fitting = disabled`,
`elefant_foot_compensation = 0`). 64 rows, every run exit 0.

| cell | PnP median | Orca median | ratio | required cut | 1.5x cut | PnP spread | Orca spread |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 3.912 s | 1.915 s | **2.043x** | 51.05 % | 26.57 % | 1.7 % | 4.2 % |
| benchy classic-on | 6.553 s | 2.442 s | **2.683x** | 62.73 % | 44.10 % | 0.7 % | 2.5 % |
| benchy arachne-off | 3.674 s | 1.983 s | **1.853x** | 46.03 % | 19.05 % | 5.9 % | 4.9 % |
| benchy arachne-on | 6.686 s | 2.591 s | **2.580x** | 61.24 % | 41.86 % | 3.8 % | 8.2 % |
| base classic-off | 27.869 s | 13.414 s | **2.078x** | 51.87 % | 27.80 % | 5.3 % | 3.4 % |
| base classic-on | 70.255 s | 23.414 s | **3.001x** | 66.67 % | 50.01 % | 1.2 % | 1.1 % |
| base arachne-off | 29.602 s | 14.920 s | **1.984x** | 49.60 % | 24.40 % | 7.1 % | 8.3 % |
| base arachne-on | 89.224 s | 25.822 s | **3.455x** | 71.06 % | 56.59 % | 21.2 % | 8.6 % |

- Ranges: **1.853x–3.455x** overall, **1.853x–2.683x** on the four benchy cells,
  **1.984x–3.455x** on the four base cells; required cut **46.03 %–71.06 %**,
  **19.05 %–56.59 %** for a 1.5x margin.
- CPU corroborates the direction everywhere (PnP 18.73 / 34.97 / 19.69 / 36.69
  and 207.61 / 492.72 / 212.41 / 497.45 s against Orca 6.53 / 10.39 / 7.67 /
  12.11 and 29.17 / 97.08 / 38.05 / 106.56 s).
- The four benchy cells' PnP hashes are **exactly** the active set (one hash,
  three measured runs each); the four base cells reproduce t144's base hashes.
- The lowest bars in the matrix are now the two benchy supports-off cells
  (**46.03 %** and **51.05 %**); the highest are the two base supports-on cells
  (**66.67 %** and **71.06 %**).

## 6. Retirements

Everything measured on the old export is historical. Specifically retired:

- t139's four benchy reference hashes (recorded above), and every t139 benchy
  cross-take comparison built on them.
- t138's benchy **g-code** measurements — the layer-0 outer-wall hulls
  (519.821 / 513.186 mm²), brim/skirt runs and paths, and the L0/L10 plane
  separations. Its benchy **STL** figures (areas 634.4579 / 627.754 mm², 225,786
  triangles, bounds) survive: the replacement reproduces the cross-sections
  exactly, so the model-level attribution stands.
- t135 Stage 1's benchy oracle numbers (brim work +2.4 / +2.6 %, brim area
  +1.3 %, skirt 12.5 %) and every pre-t144 benchy adhesion comparison.
- t142's benchy A/B (20.00 / 10.56 / 20.23 / 12.66 % wall saved), and every
  pre-swap benchy A/B pair and median.
- t121 / t124 / t139 benchy scoreboard medians and every composite ratio that
  includes a benchy cell.
- t126's "benchy classic-off 1.02x at zero prepass" and t134's benchy planner
  ratios: both are pre-swap arithmetic.
- t143's four benchy hashes are the **interim** set, superseded by t144 (not
  retired for fixture reasons — they are post-swap).

Not retired: base cells and base-only ratios (fixture byte-identical); t143's
planner depth table and A/B benchy rows (all post-swap — the fixture mtime is
2026-11-12T20:48 and the depth runs start 20:55); t144's hashes, oracle and A/B
(post-swap).

## 7. Disclosures

- **Accelerated timing arm absent.** `cargo xtask dist --accelerated --edition
  developer` was attempted today and failed at exit 1 with `c1xx: fatal error
  C1083: Cannot open source file: '\\mimalloc-static.cc'` (the controlled
  driver's verbatim-path / MSVC 14.51 `libmimalloc-sys` failure, t142). A
  manual `--cfg pnp_perimeter_spatial_accelerated` host is **not** a legal
  timing artifact under `docs/23_controlled_perimeter_builds.md` ("an ordinary
  invocation cannot grant that cfg"), so no substitute run was made. The
  accelerated **output** arm is already on record (t144: byte-identical in 8/8).
- **Degraded supports-on cells, pre-existing.** Every PnP supports-on run is
  `degraded: true` with code-1200 `support demand … unmet … body rejected:
  exact-Z occupancy` diagnostics: **101** on each benchy supports-on cell and
  **44,474** on each base supports-on cell; supports-off is clean. The counts
  are identical on the **old** export (t135, t142) and the **new** one
  (t143, t144, t145), so the swap did not cause them, but they mean the
  supports-on prints are support-incomplete on the PnP side and their ratios
  compare a smaller support job. Disclosed for every supports-on number above;
  it is a pre-existing fairness question, not a t145 finding.
- **Machine state.** Load sampled 9 % before the benchy batch and 12 % before
  the base batch (Opera/Discord resident). The t139 scoreboard this supersedes
  was taken on a slower box state (its base classic-on cell was 126.27 s against
  t143's 74.70 s and today's 70.25 s), so **no causal claim is taken from the
  cross-take ratio movement**; each cell here is internally interleaved.
- **Three measured runs per cell.** Per-cell wall spread is ≤ 8.6 % except base
  arachne-on's PnP cell at **21.2 %** (83.5 / 89.2 / 102.4 s), whose median is
  the least certain number in the table.
- **Orca g-code hashes differ per run** (`dc5ea5fd…`, `72ae2fe6…`, `157fa417…`)
  while byte counts are constant (4,077,463): the Orca file carries a per-run
  timestamp. The configuration appendix and section census are validated
  per run; PnP output is byte-constant.
- **No scoreboard re-derivation beyond this window.** The 8-cell composite here
  is today's ordinary-arm measurement only. The accelerated-arm scoreboard has
  no current take.

## 8. Route arithmetic and verdict

The destination (8/8 cells) is unchanged and still not met: **0 of 8 cells are
won**. The required wall cut is now **46.03 %–71.06 %** on the ordinary arm,
**19.05 %–56.59 %** for a 1.5x margin. The benchy supports-off cells have the
lowest bar in the matrix; the base supports-on cells still need ~2/3 of their
wall removed. Because the prepass owns 62–87 % of every cell (t126/t128) and the
planner is a tranche (t134–t143), the operand-volume route is unchanged — this
take re-bases the measurement, not the plan.

**Verdict: KEEP the currency, retire the pre-swap benchy set.** The benchy
reference set is t144's; `reference.json` is the currency artifact; the map
carries the retirement rule and the post-swap scoreboard.

Evidence: `python verify.py` (98/98 checks, 25/25 negative controls rejected)
against `reduced/summary.json` + `reference.json`; `reduce_t145.py` regenerates
both from the gitignored raw tree.

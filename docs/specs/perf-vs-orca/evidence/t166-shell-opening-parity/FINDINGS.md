# t166 take: canonical top/bottom opening in `PrePass::ShellClassification`

Take: 2026-11-19. Baseline arm = HEAD `8be73ea5` (tracker-only commit on top of
`2f6f1635`); candidate arm = the same tree plus the change below.
Host-only change: both arms share one module-tree hash
(`6899f824b831…`); executables `a9a21bd655a7…` (baseline) / `68b11890a9fb…`
(candidate). Box: 12 logical cores, CPU load 16 % before the paired campaign and
8 % after; the null control sampled 2 % before and 8 % after.

**Disposition: the parity fix lands; the speed claim is DEGRADED (5/8 by the
strict wall gate) and the human rules whether it is banked as speed.** The
ticket's own stop rule covers this: a sub-floor cell effect still lands on
correctness grounds.

## Code facts

Canonical read at the `v2.5.0` pin (`40ecc66f78`), not at a nightly.

| | canonical `PrintObject::detect_surfaces_type` (`PrintObject.cpp`) | pre-t166 PnP `apply_opening` | t166 |
| --- | --- | --- | --- |
| radius | `flow(frExternalPerimeter).scaled_width() / 10` | `line_width × 0.5` = 0.2 mm | `external_perimeter_width / 10` = 0.04 mm |
| join | `opening_ex` → `jtMiter`, `DefaultMiterLimit = 3` (`ClipperUtils.hpp`) | `Round`, arc tolerance 0 | `Miter`, limit 3 |
| calls | one `opening_ex` | `offset(-r, Round)` + `offset(+r, Round)` | one `opening_ex` |

Two facts the probe could not settle, both fixed here:

1. **`outer_wall_line_width = 0` is canonical's auto sentinel, not a zero-width
   wall.** The matched job authors `0` (Orca's own resolved appendix prints
   `outer_wall_line_width = 0.42`, i.e. an *auto-resolved* width). Resolving the
   literal zero does not "use canonical's width" — it disables the opening.
   Measured on base classic-off with the literal value: **5,201,451 points in,
   5,201,451 out, 6.8 ms for 974 calls**, every layer carrying a depth-0 seed.
   `resolve_shell_external_width_mm` resolves it the way `Flow` does and the
   way `resolve_contact_params`
   (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) already
   resolves the same key: positive absolute `outer_wall_line_width`, else typed
   `line_width`, else 0.4 mm. On this job that is **0.4 mm → radius 0.04 mm**.
2. **Pass 1's opening radius and Pass 2's shadow width are not the same
   quantity.** The old code derived both from one number
   (`opening_radius = line_width / 2`, `shell_width = 2 × opening_radius`).
   `ShellClassificationParams` now carries them separately, so this change
   cannot move the cascade: `shell_width_mm` stays `line_width` = 0.4 mm.

### Rejected: also porting canonical's `ApplySafetyOffset`

Canonical's statement is `diff_ex(slices, lower/upper, ApplySafetyOffset::Yes)`
followed by the opening, and `ClipperUtils.cpp`'s `c2_clip_paths` inflates the
**clip** by `ClipperSafetyOffset = 10` Orca units = **10 nm**. This workspace is
quantised at 1e-4 mm, so a faithful port lands on one workspace unit
(`SCALED_EPSILON_MM`, the value `classify_unsupported_bridge_areas` already
uses). Measured on base classic-off, one run each, same binary:

| arm | wall | opening thread-time | points out | seeds top/bot | `gate_internal_bridge_sites` |
| --- | ---: | ---: | ---: | ---: | ---: |
| legacy radius (0.2, Round) | 20.9 s | 53.5 s | 361,003 | 361 / 311 | 5.21 s |
| **canonical radius (0.04, Miter 3)** | **19.2 s** | **12.5 s** | 1,996,872 | 481 / 451 | 6.18 s |
| canonical radius + safety offset | 20.2 s | 11.4 s | 2,013,142 | 481 / 451 | 6.85 s |

The safety offset makes the cell **slower** (20.2 vs 19.2 s) and moves output,
so it is not part of this change. The radius-only arm reproduces the original
t166 probe exactly (hash `0a1ee3df…`), which is the cross-check that the probe
and this implementation are the same geometry.

### A separate defect this take evidences, not fixed here

`classify_unsupported_bridge_areas` resolves the **same**
`outer_wall_line_width = 0` through
`resolve_external_perimeter_width_mm` (`crates/slicer-core/src/algos/prepass_slice.rs`),
so canonical's bridge opening (`opening_ex(diff, 0.1 × fw)`) has been a no-op on
the matched job. `bridge_areas` is read by `region_partition.rs` (sparse-infill
area), `perimeter_spatial.rs` (per-vertex `is_bridge`) and
`detect_bridging_direction_deg`, so this is output-affecting, not dead work.
Graduated as [t168](../../issues/168-bridge-opening-auto-sentinel.md); it needs
its own delta audit and is deliberately **not** bundled into this ticket.

## Campaign

One window, ordinary arm only, 8 cells × 5 interleaved pairs (alternating which
arm goes first), `RAYON_NUM_THREADS=12`. 80 paired runs, every exit 0.

| cell | baseline wall (median) | candidate wall (median) | wall saved | floor | CPU saved | pairs faster |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 3.327 s | 2.663 s | **+20.435 %** | 25.54x | +10.938 % | 5/5 |
| benchy classic-on | 6.922 s | 6.385 s | **+7.915 %** | 9.89x | +6.250 % | 5/5 |
| benchy arachne-off | 3.419 s | 2.650 s | **+21.395 %** | 26.74x | +10.736 % | 5/5 |
| benchy arachne-on | 7.070 s | 6.414 s | **+9.277 %** | 11.60x | +5.736 % | 5/5 |
| base classic-off | 21.115 s | 20.494 s | +3.448 % | 4.31x | +7.579 % | 4/5 |
| base classic-on | 73.076 s | 69.374 s | +2.468 % | 3.09x | +2.486 % | 5/5 |
| base arachne-off | 22.318 s | 21.461 s | +0.310 % | 0.39x | +7.081 % | 3/5 |
| base arachne-on | 71.797 s | 70.410 s | +0.734 % | 0.92x | +2.340 % | 4/5 |

**Process CPU is lower in all eight cells** (+2.340 … +10.938 %), and CPU is the
tighter instrument here (see the null control).

### Null control (baseline arm against itself, 3 pairs, 3 cells)

| cell | wall band any-pair | wall band median | CPU band any-pair |
| --- | ---: | ---: | ---: |
| base classic-off | 11.54 % | 3.17 % | 1.54 % |
| base arachne-off | 7.68 % | 0.50 % | 0.74 % |
| benchy classic-off | 6.09 % | 5.35 % | 1.74 % |

### Gate reading (pre-declared: 0.80 % floor and outside the null band on the four base cells; no median regression on the four benchy cells)

- **Four benchy cells: PASS** (25.5x, 9.9x, 26.7x, 11.6x the floor, no regression).
- **base classic-on: PASS** (+2.468 %, 3.09x).
- **base classic-off: DEGRADED** — clears the floor at 4.31x (+3.448 %) but sits
  just inside this window's own null *median* band (3.17 %). Its CPU is +7.579 %
  against a 1.54 % CPU band, and 4/5 pairs are faster.
- **base arachne-off (+0.310 %, 0.39x) and base arachne-on (+0.734 %, 0.92x):
  FAIL the wall floor.** Both are CPU-lower (+7.081 % / +2.340 %) and their null
  wall bands are 7.68 % / not sampled respectively, so the wall channel cannot
  resolve either cell in this window.

Net: **5/8 PASS, 1 DEGRADED, 2 sub-floor**, with CPU lower in 8/8.

## Parity evidence

Per-layer extruded path of `Top surface` / `Bottom surface` / `Internal solid
infill` / `Internal Bridge` from the campaign's own G-code
(`parity_delta.py`; both arms print at the same width, so path length tracks
classifying area).

| cell | top Δ | top layers (+/−/0) | bottom Δ | bottom layers (+/−/0) |
| --- | ---: | :---: | ---: | :---: |
| benchy classic-off | +115.7 mm | 13 / 12 / 215 | +706.3 mm | 5 / 1 / 234 |
| benchy classic-on | +115.7 mm | 13 / 12 / 215 | +706.3 mm | 5 / 1 / 234 |
| benchy arachne-off | +135.3 mm | 18 / 8 / 214 | +783.6 mm | 4 / 1 / 235 |
| benchy arachne-on | +135.3 mm | 18 / 8 / 214 | +783.6 mm | 4 / 1 / 235 |
| base classic-off | +5.8 mm | 37 / 29 / 430 | +4.9 mm | 24 / 14 / 458 |
| base classic-on | +5.8 mm | 37 / 29 / 430 | +4.9 mm | 24 / 14 / 458 |
| base arachne-off | +11.7 mm | 41 / 27 / 428 | +5.0 mm | 28 / 15 / 453 |
| base arachne-on | +11.7 mm | 41 / 27 / 428 | +5.0 mm | 28 / 15 / 453 |

**The ticket's "the expected direction is more top/bottom solid area, never
less" holds at the cell total and is FALSIFIED per layer**: individual layers
lose top or bottom path in every cell (12–29 top layers, 1–15 bottom layers).
That is disclosed, not waived. The mechanism is the neighbour diff: Pass 1
classifies a layer's exposure against its neighbours, so reassigning a strip on
one layer changes what its neighbour sees, and Pass 2's seeds shift with it
(depth-0 seeds measured 361/311 → 481/451 on base classic-off). The classification
is therefore a *relabelling* across layers, not a monotone growth of every layer.

Section-level delta (base classic-off): `Top surface` 381 → 385 lines,
`Bottom surface` 133 → 136, `Internal solid infill` 8 → 0, `Internal Bridge`
30 → 32, `Bridge` 210 → 224, `Sparse infill` 237,523 → 238,389. The internal and
bridge movements follow the changed solid partition; the mechanism behind the
14 `Bridge` lines was not attributed further and is disclosed as unexplained.

## Output and the new reference set

All eight baseline runs reproduce **t151's active reference set exactly**, and
all eight candidate hashes move. `reduced/campaign.json` records every hash;
`reference.json` is the new currency.

| cell | t151 (superseded) | t166 (active) | bytes |
| --- | --- | --- | ---: |
| benchy classic-off | `3d28d1b3…` | `276782a7ce35cfd1` | +8,730 |
| benchy classic-on | `eb202973…` | `44564d29d98937c4` | +8,608 |
| benchy arachne-off | `39ad6289…` | `46111bb0ee0d86a7` | −2,170 |
| benchy arachne-on | `77c3c578…` | `12c5e45c7da449d4` | −2,050 |
| base classic-off | `9bb18f0e…` | `0a1ee3df67e5e971` | +37,138 |
| base classic-on | `eccbd523…` | `bc5cb5e1532248dc` | +37,158 |
| base arachne-off | `2f249cdb…` | `cadebffa6f8c2613` | +12,725 |
| base arachne-on | `1811425f…` | `77f2b6551183889e` | +12,679 |

## Route arithmetic (ESTIMATE, not a ratio of record)

Composing the paired medians with t164's board is a cross-window multiplication
of a board by a paired delta: no fresh Orca arm was run, so no cell's ratio of
record moves here.

| cell | t164 ratio | required cut | post-t166 (estimate) | cut (estimate) |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | 1.978x | 49.45 % | 1.574x | 36.47 % |
| benchy classic-on | 2.884x | 65.33 % | 2.656x | 62.35 % |
| benchy arachne-off | 1.894x | 47.21 % | 1.489x | 32.84 % |
| benchy arachne-on | 2.784x | 64.08 % | 2.526x | 60.40 % |
| base classic-off | 1.501x | 33.38 % | 1.449x | 31.00 % |
| base classic-on | 2.812x | 64.44 % | 2.743x | 63.54 % |
| base arachne-off | 1.379x | 27.47 % | 1.374x | 27.24 % |
| base arachne-on | 2.578x | 61.21 % | 2.559x | 60.92 % |

The change is a **benchy-cluster** lever (−13 points of required cut on both
supports-off benchy cells, −4 and −2 on the supports-on pair) and nearly inert
on `base`, where the four supports-off/aon cells move 0.3–3.4 %. It does not
touch the two benchy supports-on cells' standing constraint (canonical prepass
cost, t164/t165), and **0 of 8 cells are won**.

## Disclosures

- **No accelerated arm.** t163's standing rule says to check it for every
  candidate; the corroboration needs two full `dist --accelerated` builds (the
  baseline arm as well as the candidate) plus its own paired campaign, and was
  not taken in this take. t163 measured the arm's wall effect at −13.2…+4.4 %
  (median −1.1 %), which cannot overturn the benchy cells' 8–21 % but could
  matter for the two sub-floor base Arachne cells. Read those two cells as
  **unresolved by this instrument**, not as proven neutral.
- One box, one window; the null control sampled only three cells (the two base
  supports-off cells and benchy classic-off) and only 3 pairs each.
- Instrumented numbers (opening thread-time, seeds, gate) come from
  `--instrument-stderr`/probe runs on base classic-off only and are **never wall
  claims**; the wall column is the paired campaign.
- The change is host-only; `cargo xtask build-guests --check` exits 0 before and
  after (guests are untouched, and both arms stage one identical module tree).
- The two `Internal solid infill` and `Bridge` section movements are measured,
  not attributed (see above).
- `base classic-on`'s baseline median (73.076 s) is about 3.5x t164's cell wall
  for the same job; only paired deltas are claimed from this window.

## Artifacts

- `reduced/campaign.json` — every campaign claim, re-derived by `verify.py`
  (which also carries a `--self-test` with 7 negative controls).
- `reduced/parity.json` — the per-layer parity table, re-derived by
  `parity_delta.py --self-test` (5 negative controls).
- `reduced/route.json` — the estimate above.
- `reference.json` — the active reference set.
- `run_t166.ps1`, `reduce_t166.py`, `parity_delta.py`, `verify.py`,
  [`REPRODUCE.md`](REPRODUCE.md).

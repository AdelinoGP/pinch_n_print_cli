# t135 Stage 1: canonical skirt/brim re-take

**Status:** resolved 2026-11-10 — the port lands, the matched job authors the same
adhesion values on both arms, the new reference set is recorded, and the parity
oracle passes in all eight cells. Numbers below are re-derived by
`verify.py` from [`reduced/summary.json`](reduced/summary.json); the raw tree
under `target/t135/` is optional.

## What changed

`com.core.skirt-brim` no longer draws bounding-box rectangles. Its geometry is
the canonical family at the pinned OrcaSlicer `v2.5.0` (`40ecc66f78`):

| Feature | Canonical source | Port |
|---|---|---|
| Brim | `make_brim`, `tryExPolygonOffset` (`Brim.cpp`) | annulus between `offset(footprint, brim_object_gap)` and `offset(footprint, brim_object_gap + quantised_width)`; filled with concentric lines at half-a-spacing inside the outer boundary, one spacing per pass, both the outer and the hole boundary of each shrunken band |
| Skirt | `Print::_make_skirt` (`Print.cpp`) | convex hull of the first `skirt_height` layers' points **plus the brim footprint**, offset outward by `skirt_distance + spacing/2 + i × spacing`, `jtRound`, 0.1 mm arc tolerance |
| Quantisation | `make_brim` | `floor(brim_width / spacing / 2) × spacing × 2` (8 mm at 0.4 mm spacing → 8.0 mm, 20 lines) |
| No-brim areas | `no_brim_area_support` | first-layer support footprint (closed contours filled, open infill buffered to bead width) subtracted from the brim **area**, so a support notch is carried inward through every line |

The port carries the standard OrcaSlicer attribution header. Two module keys
were added to the manifest: `brim_object_gap` (default 0.1, the canonical
default) and `min_skirt_length` (default 0; **non-zero fails closed** — the
module has no filament cross-section model, so it cannot honour a filament-length
floor and will not silently print fewer loops than asked for).

## Matched settings (both arms, assert per run)

`brim_width = 8`, `skirt_loops = 6`, `skirt_distance = 3`, `skirt_height = 1`
authored in the four matched configs and asserted from each arm's resolved
appendix:
`Test-OrcaEvidence` for Orca and the new `Test-PnpAdhesionKeys` for PnP.

**Re-derived fact that changes the t130 instruction: `brim_type` must be pinned
too.** The vendor chain resolves `brim_type = auto_brim`, and under `auto_brim`
`make_brim` recomputes the width from its adhesion heuristic and *discards* the
explicit value. Measured on benchy classic-off with `--brim-width=8` alone: a
~10 × 5 mm, 3-loop brim. With `--brim-type=outer_only` the same job prints the
8 mm ring (19 loops, 53.4 mm across). The rig therefore pins
`--brim-type=outer_only` as a fifth matched key, and the ruling's *intent*
(one 8 mm adhesion feature on both arms) is what is implemented.

## Output campaign: the new reference set

8 cells × 2 arms (ordinary, accelerated). The **baseline** arm is the pre-port
`skirt-brim` module with the same host binary; the **candidate** arm is the
ported module.

- Baseline hashes reproduce ticket 123's reference set exactly in all eight
  cells (`90638bf3…`, `711c6334…`, `2ba5d84c…`, `a972b0f2…`, `b3b4ac5e…`,
  `4e31a2db…`, `6b865d4a…`, `62d6eedc…`) — the baseline arm is faithful, so
  every move is attributable to the port.
- Candidate hashes are the **new reference set**:

| Cell | New hash (ordinary = accelerated) |
|---|---|
| benchy-classic-off | `9012df71023c9cb523ce18bb6bd41af3503a19b4a6b3038b58c3d70216bbe25e` |
| benchy-classic-on | `7b199bd1bd13b9fa296773d8b47be1f799351038683cdc455ca60e3956b6a97c` |
| benchy-arachne-off | `dbafeb9d54d8229b5eb250ecb4c69305ec63445f33b6ea4db36ef66fb5797666` |
| benchy-arachne-on | `c86af409f424ca2099321e167eced02547836ff049758574f4a75547a36e2f40` |
| base-classic-off | `27dd2114759bb81c3a39362bc1f843f6753d2762145f742c7477657659b4f7de` |
| base-classic-on | `e2c3c60e81092984ea1f179447a24db149aa2f1a2dd15aeec4e8d1489cebe6cf` |
| base-arachne-off | `32ac8f77e290adb69d99acb52f27951ebc9472730c78b7ab03290d48d83ad40c` |
| base-arachne-on | `e5f5b8f4e6ce3699744b655f5af7b30e0e85964ffe2a54965deefa739b99a688` |

`verify.py` gates: all 32 runs exit 0, both arm classes are internally
byte-identical, every candidate hash differs from its baseline (the port moves
adhesion geometry in every cell), and no output is empty.

## Delta audit (the port's blast radius)

Every motion line outside `Skirt` and `Brim` is **identical after the extrusion
field is stripped** — coordinates, feedrates, Z, travels within a section, and
feature order do not move in any of the eight cells. The extrusion field itself
differs by at most **0.00098 mm** on any line (declared tolerance 1e-3 mm),
because PnP derives relative `E` from a running accumulator and the adhesion
volume changed; that is a rounding artefact, not a geometry change. The
resolved-config block gains the module's two new keys. The raw census is in
`audit.line_census` for each cell.

## Parity oracle (PnP vs Orca, same job, all 8 cells)

Translation-invariant metrics, because the two slicers place the model at
different origins (measured ~3.2 mm in X and ~1.0 mm in Y on benchy). Bands were
declared from the benchy classic-off probe and applied unchanged to all eight
cells.

| Cell | Brim loops PnP / Orca | Brim swept-area delta | Brim growth delta | Skirt outer-area delta | Skirt growth delta |
|---|---|---|---|---|---|
| benchy-classic-off | 20 / 19 | **+9.9 %** | 0.46 mm | **+0.4 %** | 0.08 mm |
| benchy-classic-on | 18 / 26 | +75.3 % | 0.46 mm | +4.5 % | 0.21 mm |
| benchy-arachne-off | 20 / 19 | **+9.7 %** | 0.44 mm | **+0.4 %** | 0.09 mm |
| benchy-arachne-on | 17 / 26 | +75.4 % | 0.44 mm | +4.5 % | 0.21 mm |
| base-classic-off | 20 / 16 | +26.4 % | 0.31 mm | +8.4 % | 0.23 mm |
| base-classic-on | 0 / 0 | **0.0 %** | 0.00 mm | +11.0 % | 1.28 mm |
| base-arachne-off | 20 / 16 | +25.5 % | 0.40 mm | +8.2 % | 0.14 mm |
| base-arachne-on | 0 / 0 | **0.0 %** | 0.00 mm | +11.0 % | 1.18 mm |

**Verdict: the strict 8/8 geometric form is NOT met.** It passes on three axes
and fails on two, and both failures are attributable to differences *outside*
this module. Reporting them rather than widening the band is the point of the
oracle.

1. **Presence parity — 8/8, and it needed the region-level support clip.** Both
   arms now agree on whether a brim exists at all, including the two base
   supports-on cells where canonical clips the *entire* 8 mm ring away because
   the tree-support forest surrounds the part. Reproducing that required
   clipping the brim **area** against the support *region* (a closing of the
   bead union at `2.5 x line_width`), not against individual beads: the
   bead-exact variant left 17 loops where Orca prints none, and clipping
   individual ring lines instead of the area let the brim thread between
   support lines (measured: 470 of 1,401 brim points within 0.1 mm of a support
   bead).
2. **Radial extent — 8/8 within 0.46 mm.** This is the port's own geometry: the
   brim follows the contour at the authored width.
3. **Skirt outer boundary — 0.40 % on the two benchy supports-off cells**, i.e.
   the hull-anchored skirt there is the same shape and size. The residual on the
   other six cells (8.2–8.4 % base supports-off, 4.5 % benchy supports-on,
   11.0 % base supports-on) has two named causes and neither is the skirt's
   anchoring rule: on supports-off the *brim* feeds the hull and PnP's brim is
   8.0 mm wide against Orca's 7.31 mm, which moves the hull the skirt wraps; on
   supports-on the **support hull** dominates, because `_make_skirt` folds the
   support footprint in and PnP's tree-support geometry differs from Orca's
   `tree_strong`.
4. **Brim work — fails the 12 % band in five cells.** The +9.7/+9.9 % on the
   benchy supports-off cells is the flow-spacing quantisation (8.0 mm at PnP's
   0.4 mm spacing vs 7.31 mm at Orca's 0.457 mm). The +26 % on the base
   supports-off cells and +75 % on the benchy supports-on cells are **not** this
   module's geometry:
   - Orca's `; FEATURE: Brim` section also carries **support brims**
     (`tree_support_brim_width = 3`); PnP's tree-support module emits none.
     That is the bulk of the benchy-on delta (Orca 26 brim runs against 19 in
     its own supports-off cell).
   - Orca's per-volume-group brim and its no-brim areas clip the ring further
     on the base part (16 loops against PnP's 20 for the same 8 mm width).

**What the oracle therefore licenses:** the port emits the canonical *shape
family* at the authored width, on the canonical anchor, with canonical support
avoidance and canonical whole-ring clipping — and the two arms' remaining
differences live in the flow model and in the support engine, not in
`com.core.skirt-brim`. It does **not** license a claim that the two arms print
the same brim.

## Old-vs-new paired A/B (attribution only)

3 interleaved pairs per cell, baseline-ordinary vs candidate-ordinary, wall =
process creation-to-exit, CPU = kernel+user from the process handle. Both
ordinary arms carry the **same host binary** (hash-checked). The port is
mandated, so this is a cost record, not a KEEP/DROP gate.

| Cell | baseline wall | candidate wall | ratio | baseline CPU | candidate CPU | CPU ratio | wall + / 3 pairs | CPU + / 3 pairs |
|---|---|---|---|---|---|---|---|---|
| benchy-classic-off | 4.38 s | 4.44 s | 1.013 | 25.1 s | 24.9 s | 0.993 | 3/3 | 1/3 |
| benchy-classic-on | 7.77 s | 7.81 s | 1.004 | 39.8 s | 39.5 s | 0.992 | 2/3 | 1/3 |
| benchy-arachne-off | 4.55 s | 4.55 s | 0.999 | 26.1 s | 26.1 s | 1.001 | 2/3 | 2/3 |
| benchy-arachne-on | 8.10 s | 8.06 s | 0.995 | 40.5 s | 40.7 s | 1.004 | 1/3 | 1/3 |
| base-classic-off | 38.08 s | 37.14 s | 0.975 | 311.3 s | 311.2 s | 1.000 | 0/3 | 1/3 |
| base-classic-on | 92.52 s | 92.38 s | 0.998 | 599.2 s | 595.7 s | 0.994 | 1/3 | 0/3 |
| base-arachne-off | 37.78 s | 37.81 s | 1.001 | 315.4 s | 315.7 s | 1.001 | 1/3 | 2/3 |
| base-arachne-on | 91.91 s | 92.23 s | 1.003 | 601.6 s | 600.0 s | 0.997 | 1/3 | 0/3 |

Read: **the port's cost is below this box's paired noise floor.** Wall ratios
span 0.975–1.013 and CPU ratios 0.992–1.004; no cell has a wall win with CPU
corroborating in 3/3 pairs, and the one cell that looks 2.5 % faster
(`base-classic-off`, 0/3 wall-positive) has flat CPU (ratio 1.000), so it is not
a corroborated effect. The adhesion feature is one layer of a 4–92 s cell; the
port changes its shape, not its order of magnitude.

## Scoreboard re-derivation (8 cells × 3 tools)

Standing matched-pair protocol (1 warmup + 3 measured, interleaved), superseding
ticket 124's ratios of record:

| Cell | Orca | PnP ordinary | ordinary ratio | PnP accelerated | accelerated ratio | required cut (ordinary) |
|---|---|---|---|---|---|---|
| benchy-classic-off | 1.90 s | 4.68 s | **2.468x** | 4.64 s | 2.450x | 59.5 % |
| benchy-classic-on | 2.58 s | 8.34 s | **3.233x** | 8.14 s | 3.156x | 69.1 % |
| benchy-arachne-off | 2.11 s | 5.00 s | **2.364x** | 4.91 s | 2.320x | 57.7 % |
| benchy-arachne-on | 2.57 s | 8.45 s | **3.294x** | 8.28 s | 3.226x | 69.6 % |
| base-classic-off | 13.12 s | 35.92 s | **2.739x** | 35.34 s | 2.695x | 63.5 % |
| base-classic-on | 24.28 s | 92.82 s | **3.824x** | 91.76 s | 3.780x | 73.8 % |
| base-arachne-off | 14.89 s | 38.17 s | **2.563x** | 37.85 s | 2.542x | 61.0 % |
| base-arachne-on | 22.29 s | 91.20 s | **4.091x** | 91.87 s | 4.122x | 75.6 % |

**Ratios of record: 2.364x–4.091x ordinary, 2.320x–4.122x accelerated; required
cut 57.7 %–75.6 %.** 96 rows (8 cells x 3 tools x (1 warmup + 3 measured)),
interleaved; every PnP cell has one hash across both build arms and it equals the
new reference set; 48/48 PnP runs exit 0, `degraded = false`, zero fatal errors;
every Orca run asserts the five matched keys from its resolved appendix.

The spread narrowed against t124's 2.36x–4.65x, but **no causal claim is taken**:
both arms' adhesion work changed, so t124's ratios and these are not comparable.
They are superseded, not improved.

## Disclosures

- The port is a **fairness** change, not a route step: the destination stays at
  0/8 cells won (2.364x–4.091x ordinary / 2.320x–4.122x accelerated, required cut 57.7 %–75.6 %); this ticket removes an asymmetry that made
  every prior ratio of record compare two different adhesion jobs.
- **`brim_type = auto_brim` silently overrides `brim_width`** (measured, above).
  The rig pins `outer_only` on the Orca arm; PnP implements outer-only only, and
  inner/ear brims are not ported.
- **`min_skirt_length > 0` fails closed** (no flow model). The matched job
  authors 0, and Orca's own resolved value is 0.
- **Support clipping is a declared approximation.** Canonical subtracts the
  support *slice contour*; the IR carries bead centerlines only, so the port
  reconstructs the region as `closing(union(beads), 2.5 x line_width)` (1.0 mm
  at the matched job) and fills it. The radius is a parity tolerance, not a
  physical constant, and it is what makes the two base supports-on cells match
  canonical's whole-ring clip.
- **Skirt simplify tolerance**: canonical runs `Geometry::simplify_polygons` at
  0.05 mm; PnP's host `simplify_polygon` only drops collinear vertices, so the
  skirt keeps more vertices. Geometry unaffected, vertex count higher.
- **Orca source vs installed binary** remains the t128 caveat: the checkout is
  at the tag (2026-11-07), the measured binary is dated 2026-11-06 and is not
  byte-proven identical.
- **Arm artifacts** (`reduced/summary.json` -> `manifest`): the two ordinary
  arms share one host binary (`cbae5e3c7417…`) and
  differ only in the `skirt-brim` wasm (baseline
  `fd0da67ec12b…`, candidate
  `a6134a81313e…`); the two
  accelerated arms share one accelerated binary
  (`442f0f2ad275…`). The candidate was
  rebuilt once after a clippy-only source fix and the eight candidate hashes
  were re-derived and matched before the final campaign.
- **Load:** shared dev box, 6 physical / 12 logical cores; idle load 0.9–6.5 %
  before the scoreboard, and the A/B ran on identical host binaries with stable
  windows (base cells 92–94 s across all six runs per cell).

## Route implication

Unchanged from t128/t131/t134: the destination needs a 58–79 % wall cut and the
prepass operand volume is the route. What changes is the **baseline every future
ratio is quoted against** — the new reference set above, with the adhesion job
matched.

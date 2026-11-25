# t135 Stage 1: canonical skirt/brim re-take

**Status:** resolved 2026-11-10 — the port lands, the matched job authors the same
adhesion values **and the same adhesion flow model** on both arms, the new
reference set is recorded, and the parity oracle is re-derived. Every number below
is produced by `reduce_t135.py` from the raw campaign tree and re-checked by
`verify.py` from [`reduced/summary.json`](reduced/summary.json) alone.

## What changed

`com.core.skirt-brim` no longer draws bounding-box rectangles. Its geometry is
the canonical family at the pinned OrcaSlicer `v2.5.0` (`40ecc66f78`):

| Feature | Canonical source | Port |
|---|---|---|
| Brim | `make_brim`, `tryExPolygonOffset` (`Brim.cpp`) | annulus between `offset(outline, brim_object_gap)` and `offset(outline, gap + quantised_width)`; subtracted by the first-layer **support region**; filled with concentric lines, the first half a spacing inside the outer boundary and one spacing per pass, emitting both the outer and the hole boundary of each shrunken band |
| Skirt | `Print::_make_skirt` (`Print.cpp`) | convex hull of the first `skirt_height` layers' points **plus the brim footprint**, offset outward by `skirt_distance + spacing/2 + i × spacing`, `jtRound`, 0.1 mm arc tolerance |
| Flow spacing | `Flow::rounded_rectangle_extrusion_spacing` (`Flow.cpp`) | `line_width − first_layer_height × (1 − π/4)` = **0.35708 mm** at the matched job |
| Quantisation | `make_brim` | even-line quantisation in flow-spacing units: 8 mm at 0.35708 mm → **7.856 mm, 22 lines** |
| No-brim areas | `no_brim_area_support` | first-layer support footprint (closed contours filled, open infill buffered to bead width, merged by a closing at `2.5 × line_width`) subtracted from the brim **area**, so a support notch is carried inward through every line |

The port carries the standard OrcaSlicer attribution header. Three module keys
were added to the manifest: `brim_object_gap` (0.1), `min_skirt_length` (0;
**non-zero fails closed** — no filament cross-section model) and
`first_layer_height` (the flow-spacing input; declared because module config
views expose declared keys rather than typed globals).

## Matched settings (both arms, asserted per run)

**Seven** keys are authored and asserted from each arm's resolved appendix
(`Test-OrcaEvidence` for Orca, `Test-PnpAdhesionKeys` for PnP):
`brim_width = 8`, `brim_type = outer_only`, `skirt_loops = 6`,
`skirt_distance = 3`, `skirt_height = 1`, `initial_layer_line_width = 0.4`,
`tree_support_brim_width = 0`.

Three of those are re-derived corrections to the t130 instruction, each measured:

- **`brim_type = outer_only`**: the vendor chain resolves `auto_brim`, which
  recomputes the width from its adhesion heuristic and silently discards
  `--brim-width=8` (measured: a ~10 × 5 mm, 3-loop brim).
- **`initial_layer_line_width = 0.4`**: the adhesion flow model. Orca resolved
  0.5 mm, PnP prints layer 0 at its 0.4 mm `line_width`, so the same 8 mm
  quantised to 7.31 mm on one arm and 8.0 mm on the other and the skirt bands
  were 2.74 mm and 2.40 mm wide. Pinning Orca to 0.4 makes both spacings
  0.35708 mm. The t82 fairness table records `line_width` as "0.4 on both
  sides" — **falsified by the resolved appendix** (Orca resolves 0.42 for walls
  and 0.5 for layer 0).
- **`tree_support_brim_width = 0`**: Orca resolves 3 mm and prints support brims
  under the same `; FEATURE: Brim` label; PnP's tree-support module emits none.
  Both arms are pinned to none.

## Output campaign: the new reference set

8 cells × 2 build arms. The **baseline** arm is the pre-port `skirt-brim` module
with the same host binary; the **candidate** arm is the ported module.

- The baseline reproduces ticket 123's reference set exactly in all eight cells
  (`90638bf3…`, `711c6334…`, `2ba5d84c…`, `a972b0f2…`, `b3b4ac5e…`, `4e31a2db…`,
  `6b865d4a…`, `62d6eedc…`), so every move is attributable to the port.
- The candidate's **new reference set**, identical on both build arms:

| Cell | New hash (ordinary = accelerated) |
|---|---|
| benchy-classic-off | `7db0641bacec8c205a54147f5b134b2bd5fd19fe2aa038bb42c0ea210b5896b3` |
| benchy-classic-on | `c02aa4e91c6c6a3bf6a6acd27aea4469d1077568ee6dc1546e8ec8eed6b0b6c0` |
| benchy-arachne-off | `a2f7a1fed5f2a5be8529fab8952feeb4786bab568c885a2aee9ba65ee7a61584` |
| benchy-arachne-on | `2326455d306101f367b87f31ee1b42fc3ec24c18b3dd35f67ff7d5b9a9f2139b` |
| base-classic-off | `7ef84447e30e473e230cad18a5592eee386fc34501254fcd85670e2a90fa3a23` |
| base-classic-on | `1ddea2b7026f13f59ba54d81530e8a50c5738626f131e5d1062c8a87773420b3` |
| base-arachne-off | `defae354a073d81344e2797c1e63e8483f319199872a7fee9f1a38697b716eaf` |
| base-arachne-on | `4aa1847c3b9b5b74ad14360e3529082250299be32a44508d5f0cf0b3785b7826` |

**Every later acceptance that cites t123's set must re-point to this one.**

## Delta audit (the port's blast radius)

Every motion line outside `Skirt` and `Brim` is **identical after the extrusion
field is stripped** in all eight cells — coordinates, feedrates, Z, in-section
travels and feature order do not move. The extrusion field itself differs by at
most **0.00098 mm** on any line (declared tolerance 1e-3 mm), because PnP derives
relative `E` from a running accumulator and the adhesion volume changed. The
resolved-config block gains the module's new keys.

## Parity oracle (PnP vs Orca, same job, all 8 cells)

Translation-invariant metrics; the two slicers place the model at different
origins (measured ~3.2 mm in X and ~1.0 mm in Y on benchy), so nothing here
compares absolute coordinates. Bands were declared before the first campaign and
have not been widened.

| Cell | Brim loops PnP / Orca | Brim swept-area delta | Brim growth delta | Skirt outer-area delta | Skirt growth delta | Verdict |
|---|---|---|---|---|---|---|
| benchy-classic-off | 22 / 30 | +2.4 % | 0.20 mm | +2.3 % | 0.38 mm | **PASS** |
| benchy-classic-on | 19 / 39 | +54.5 % | 0.20 mm | +3.4 % | 0.53 mm | fail |
| benchy-arachne-off | 22 / 30 | +2.6 % | 0.22 mm | +2.3 % | 0.40 mm | **PASS** |
| benchy-arachne-on | 19 / 39 | +54.6 % | 0.23 mm | +3.4 % | 0.53 mm | fail |
| base-classic-off | 22 / 22 | +8.1 % | 0.27 mm | +3.2 % | 0.44 mm | fail |
| base-classic-on | 0 / 0 | +0.0 % | 0.00 mm | +12.5 % | 0.88 mm | fail |
| base-arachne-off | 22 / 22 | +7.3 % | 0.32 mm | +3.1 % | 0.49 mm | fail |
| base-arachne-on | 0 / 0 | +0.0 % | 0.00 mm | +12.5 % | 0.92 mm | fail |

**Verdict: the strict 8/8 geometric form is NOT met — 2/8.** The three axes the
port owns are close to parity; the residual classes are named and measured, and
none of them is the module's rule:

1. **The benchy supports-off cells pass outright** (+2.4 % / +2.6 % brim work,
   22 brim lines on PnP against Orca's fragmented 30, +2.3 % skirt area). The
   flow-model fix moved these cells from +9.7 % to +2.4 %.
2. **The base supports-off cells miss by 0.1–0.2 pp on the skirt band**
   (+8.1 % / +7.3 % brim work, +3.2 % / +3.1 % skirt area against a 3 % band).
   Both arms now print **22 brim lines at 0.35708 mm**, so the line set matches;
   the residual is the **anchor convention**: PnP anchors the brim on its
   outer-wall *path*, Orca on the *slice contour*. G-code carries the wall path
   for both arms, not the contour, so the comparison carries a ~0.2 mm anchor
   bias that is ~8 % of a small brim's area.
3. **The supports-on cells are dominated by the support engine.** On base both
   arms agree the brim is *entirely clipped away* (0 / 0 loops — the whole-ring
   clip is reproduced), but the skirt hull differs by 12.5 % because
   `_make_skirt` folds the support footprint in and PnP's tree supports are not
   Orca's `tree_strong`. On benchy Orca's brim fragments into **39 runs** around
   its support pads against PnP's 19, so its brim area is 3x larger; the object
   brim sits inside that label and cannot be separated from support-area brims
   at g-code level.
4. **Radial extent is 8/8 within 0.92 mm** (brim ≤ 0.32 mm on the supports-off
   cells).

**What the oracle licenses:** the port emits the canonical shape family at the
authored width, on the canonical anchor rule, with canonical support avoidance,
canonical whole-ring clipping and a flow model identical to the Orca arm. It
does **not** license a claim that the two arms print the same brim, because the
two slicers' layer-0 geometry and support engines differ upstream of this
module.

## Old-vs-new paired A/B (attribution only)

3 interleaved pairs per cell, baseline-ordinary vs candidate-ordinary, same host
binary in both arms (hash-checked).

| Cell | baseline wall | candidate wall | ratio | baseline CPU | candidate CPU | CPU ratio | wall + / 3 | CPU + / 3 |
|---|---|---|---|---|---|---|---|---|
| benchy-classic-off | 4.45 s | 4.44 s | 0.996 | 25.1 s | 25.4 s | 1.011 | 1/3 | 3/3 |
| benchy-classic-on | 8.28 s | 8.46 s | 1.022 | 41.4 s | 41.2 s | 0.995 | 2/3 | 1/3 |
| benchy-arachne-off | 4.69 s | 4.91 s | 1.046 | 26.4 s | 27.0 s | 1.026 | 2/3 | 2/3 |
| benchy-arachne-on | 8.67 s | 8.94 s | 1.032 | 42.1 s | 42.6 s | 1.013 | 2/3 | 2/3 |
| base-classic-off | 40.09 s | 40.73 s | 1.016 | 317.1 s | 317.0 s | 1.000 | 3/3 | 2/3 |
| base-classic-on | 104.53 s | 100.63 s | 0.963 | 621.8 s | 616.6 s | 0.992 | 0/3 | 0/3 |
| base-arachne-off | 41.99 s | 42.38 s | 1.009 | 322.7 s | 323.8 s | 1.003 | 2/3 | 3/3 |
| base-arachne-on | 105.28 s | 107.50 s | 1.021 | 627.9 s | 626.0 s | 0.997 | 2/3 | 0/3 |

Read: the port's cost is **inside the box's paired noise floor**. Wall ratios
span 0.963–1.046 and CPU 0.992–1.026; no cell wins on both axes in 3/3 pairs.
The one directional cell (`base-classic-on`, 0.963 wall, 0/3 wall-positive,
0/3 CPU-positive) is a 4 s move on a 105 s job with CPU −0.8 %, and the
scoreboard re-takes that cell at 3.71x, so it is not a route claim.

## Scoreboard re-derivation (8 cells × 3 tools)

Standing matched-pair protocol (1 warmup + 3 measured, interleaved), superseding
every earlier ratio of record:

| Cell | Orca | PnP ordinary | ordinary ratio | PnP accelerated | accelerated ratio | required cut (ordinary) |
|---|---|---|---|---|---|---|
| benchy-classic-off | 2.14 s | 5.24 s | **2.445x** | 5.47 s | 2.554x | 59.1 % |
| benchy-classic-on | 2.75 s | 9.35 s | **3.404x** | 9.26 s | 3.373x | 70.6 % |
| benchy-arachne-off | 2.21 s | 5.62 s | **2.538x** | 5.51 s | 2.492x | 60.6 % |
| benchy-arachne-on | 2.84 s | 10.00 s | **3.521x** | 9.55 s | 3.362x | 71.6 % |
| base-classic-off | 16.17 s | 43.46 s | **2.687x** | 42.17 s | 2.608x | 62.8 % |
| base-classic-on | 28.37 s | 105.36 s | **3.713x** | 106.25 s | 3.745x | 73.1 % |
| base-arachne-off | 17.73 s | 44.82 s | **2.529x** | 42.90 s | 2.420x | 60.5 % |
| base-arachne-on | 27.86 s | 105.01 s | **3.769x** | 103.85 s | 3.727x | 73.5 % |

**Ratios of record: 2.445x–3.769x ordinary, 2.420x–3.745x accelerated; required
cut 59.1 %–73.5 %.** 96 measured rows; every PnP cell has one hash across both
build arms equal to the new reference set; PnP runs are `degraded = false` with
zero fatal errors; every Orca run asserts the seven matched keys from its
resolved appendix. t124's 2.36x–4.65x is **superseded, not improved**: the
adhesion job, the first-layer width and the support brim all changed.

One batch was aborted by a **full disk** mid `base-classic-on`; the three
remaining base cells were re-taken in a second interleaved batch and the two
batches are appended in one CSV (the aborted cell's two warmup rows remain and
are excluded from every median).

## Disclosures

- The port is a **fairness** change, not a route step: the destination stays at
  0/8 cells won (2.445x–3.769x ordinary); it removes an asymmetry that made every
  prior ratio compare two different adhesion jobs.
- **`brim_type = auto_brim` overrides `brim_width`**; **`initial_layer_line_width`
  was unmatched** (0.5 vs 0.4) and is now pinned; **`tree_support_brim_width`**
  is pinned off because PnP has no support brim. Porting a support brim, and
  matching Orca's per-feature wall widths (0.42 / 0.45 / 0.5 where PnP prints
  0.4), are separate decisions and are not done here.
- **`min_skirt_length > 0` fails closed** (no flow model).
- **Support clipping is a declared approximation**: canonical subtracts the
  support *slice contour*; the IR carries bead centerlines only, so the port
  reconstructs the region as `closing(union(beads), 2.5 × line_width)`.
- **Skirt simplify** is collinear-only in PnP against canonical's 0.05 mm
  Douglas-Peucker; vertex count higher, geometry unaffected.
- **Orca source vs installed binary** remains the t128 caveat.
- **Arm artifacts** (`reduced/summary.json` → `manifest`): the two ordinary arms
  share one host binary (`04f8dd8a6d1f…`) and differ only in the `skirt-brim`
  wasm (baseline `fd0da67ec12b…`, candidate `8e5354e0c15d…`); the two accelerated
  arms share one accelerated binary (`2958de4466b0…`).
- **Load:** shared dev box, 6 physical / 12 logical cores; the scoreboard ran in
  two windows (disk-full restart); A/B cells are internally paired.

## Route implication

Unchanged from t128/t131/t134: the destination needs a 59–73 % wall cut and the
prepass operand volume is the route. What changes is the **baseline every future
ratio is quoted against** — the new reference set above, with the adhesion job
matched on presence, width, flow spacing and support avoidance.

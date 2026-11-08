# Facet bridge-path retirement: canonical classification over the whole unsupported set

Type: task / candidate campaign
Label: wayfinder:task
Status: resolved (2026-11-07; **BUILD** — 8/8 cells clear the 0.80 % floor
  at 3.4x–27.8x, CPU corroborating; 16-cell output contract met; 0 structural
  geometry lines in any cell — see ## Answer)
Assignee: 2026-11-06/07 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map, pick up the next afk ticket" (no ticket named); 117 is the
  human's grilling, so 123 is the only takeable AFK frontier ticket.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [ticket 122](122-prep-slice-wave-expansion-facet-retirement.md)'s
  split verdict (2026-11-06). The expansion half is STOP (per-layer stage, no
  benchy margin); the classification half is a measured BUILD screen.

## Question

Can PnP retire the facet-derived bridge path, the 1.0 mm `expansion_margin_mm`
offset, the flat-family special case and the redundant unsupported-span gate,
and classify canonical's **whole** unsupported bottom set in `PrePass::Slice`
— with **no** wave expansion — under [ticket 111](111-prepass-rewrite-scope-ruling.md)'s
16-cell contract, such that the output delta is bounded and the paired
wall/CPU gate clears the 0.80 % floor in **all eight** cells?

## What t122 measured

The t122 probe reconstructed both fixtures' production prepass inputs and
priced the two halves separately:

| fixture | current classification | canonical classification | classification-only net | expansion-only cost | combined net |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy | 1,600.8 ms | 477.9 ms | **+1,122.9 ms** | 1,151.9 ms | −29.0 ms |
| base | 98,462.4 ms | 3,610.1 ms | **+94,852.4 ms** | 4,976.6 ms | +89,875.8 ms |

All figures are serial accumulated thread-time from
[`evidence/t122-wave-expansion/`](../evidence/t122-wave-expansion/FINDINGS.md);
they are a screen, not a wall claim. The classification-only shape clears the
0.80 % floor in all eight cells at **12.5x–24.5x** (benchy) and **89.7x–213.9x**
(base). The combined shape is negative in all four benchy cells because the
expansion consumes **102.6 %** of the classification saving there.

The delta is bounded on both fixtures (areas in mm², committed layers only):

| fixture | current bridge | canonical bridge | added | removed | expansion reach bound | combined movement |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy | 706.73 | 1,306.72 | 618.44 | 18.45 | 300.64 | 937.53 |
| base | 4,473.93 | 4,653.28 | 564.17 | 384.82 | 513.85 | 1,462.83 |

The current facet path's polygon sum before the gate is **14,232.05 mm²**
(benchy) and **419,106.22 mm²** (base); at most **5.0 % / 1.1 %** survives the
gate. The benchy bridge set grows **1.85x** under canonical classification; the
base set is **1.04x** (the expensive facet path is already producing almost the
canonical set there).

## Exact shape

In `PrePass::Slice` (`crates/slicer-core/src/algos/prepass_slice.rs`):

```text
unsupported = difference(raw_cur, offset(raw_prev, SCALED_EPSILON_MM, Square))
bridged     = opening_ex(
                  intersection(unsupported, region.infill_areas),
                  0.1 × external_perimeter_width,
                  Miter, 3.0)
region.bridge_areas = bridged
region.is_bridge    = !bridged.is_empty()
```

Delete:

- `classify_region_surfaces` (the boolean is re-derivable and the facet scan is
  18,275.8 ms of base thread-time on its own);
- `assemble_bridge_areas` and the facet `BridgeRegion` consumption in the
  prepass;
- `assemble_flat_bridge_areas` (the whole-set classifier subsumes the flat
  family);
- `gate_bridge_areas_by_unsupported_span` (the classifier already diffs the
  lower layer; the missing-lower-layer guard is the `prev_raw: None` case);
- the prepass use of `expansion_margin_mm`. Audit the remaining
  `SurfaceClassificationIR` consumers (`bridge_regions`, visual debug, tests)
  before retiring the field or its config surface.

Keep:

- `update_external_bridge_orientation` (it already overwrites the facet angle
  and derives from the new set);
- `gate_internal_bridge_sites` and the internal-bridge contract;
- the paint-split and `BridgeSpatialIndex` consumers unchanged.

Do **not** port `detect_bridge_directions` in this ticket. PnP's existing
direction path is the baseline; a direction port is a separate consumer-visible
rewrite.

File a `DEV` row for the knowingly-kept divergence — canonical grows the
classified bridges by wave expansion into the shell/sparse/top zones and trims
those zones; this candidate does not. Re-derive the next `DEV-###` from
`docs/DEVIATION_LOG.md` at the point of use (never quote a frozen id).

## Acceptance (t111's contract)

1. Fresh **16-cell** reference set: baseline ordinary ≡ baseline accelerated ≡
   the post-t116 set, candidate ordinary ≡ candidate accelerated, both arms
   byte-identical within themselves (ordinary and accelerated).
2. Delta audit **before** timing. Expected movement: benchy **+618.44 /
   −18.45 mm²**, base **+564.17 / −384.82 mm²**, confined to the bridge role.
   Every changed structural line must be an `E`-token-only change on unchanged
   `G1` motion, or a bridge-flagged wall/support flow change, exactly as t116;
   any coordinate, feedrate, travel, skirt, brim or raft movement is a red
   flag.
3. Paired wall/CPU gate, interleaved, floor **0.80 %** in all eight cells. The
   screen says 12.5x–24.5x (benchy) and 89.7x–213.9x (base); the actual wall
   delta must still be measured.
4. Fresh output hashes replace the t116 references;
   `evidence/t116-prep-slice-canonical-bridge/` remains the baseline record for
   the old shape.

## Tests that must move

- `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`: the flat-family unit
  tests become whole-set classification tests, including the free-edge band
  and the sub-width sliver.
- `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs`: the
  `non_bridge_region_has_empty_bridge_areas` fixture is re-derived under the
  whole-set rule.
- `crates/slicer-runtime/tests/e2e/slice_end_to_end_tdd.rs`
  (`wedge_multi_layer_top_bottom_evidence`): the bridge/bottom counts move
  again once the sloped family is classified by the same rule.
- Any test that reads `bridge_regions`/`expansion_margin_mm` from the prepass;
  audit with `rg -n "expansion_margin_mm|assemble_bridge_areas|classify_region_surfaces"`
  before editing.

## Stop rule

If the classification-only delta cannot be confined to the bridge role on
either fixture, or the paired wall gate fails in any cell, stop and report the
measured delta — do not fold the wave expansion in to recover the number. The
expansion is a separate stage (see [ticket 122](122-prep-slice-wave-expansion-facet-retirement.md)'s
structural finding) and is not a rescue path for this campaign.

## Evidence starts

- [`evidence/t122-wave-expansion/FINDINGS.md`](../evidence/t122-wave-expansion/FINDINGS.md)
  — the split verdict, the area bounds and the eight-cell screen.
- [`evidence/t112-prep-slice-parity/FINDINGS.md`](../evidence/t112-prep-slice-parity/FINDINGS.md)
  §2 (canonical classification vs the facet/flat split), §5 (canonical
  unsupported areas).
- [`evidence/t116-prep-slice-canonical-bridge/FINDINGS.md`](../evidence/t116-prep-slice-canonical-bridge/FINDINGS.md)
  — the flat-family candidate, the 16-cell proof method and the delta auditor.
- `PrintObject::detect_surfaces_type` (`PrintObject.cpp`) — the canonical
  classifier this shape applies.

## Answer

**BUILD. The classification-only retirement clears the 0.80 % floor in all
eight cells at 3.4x–27.8x, with process CPU corroborating in every cell, and
the delta audit finds no moved geometry in any structural section.**

### 16-cell output (t111's contract)

- Baseline ordinary ≡ baseline accelerated ≡ the **post-t116** set in all eight
  cells (`35600ee4…`, `596f6da5…`, `f21df124…`, `e90f491d…`, `6a6a3d9d…`,
  `8f46fe92…`, `655c99d4…`, `8b5d1de0…`).
- Candidate ordinary ≡ candidate accelerated in all eight cells; the candidate
  moves every cell.
- Bytes: benchy **+873 / +832 / +1,281 / +1,159** (classic-off/on,
  arachne-off/on) and base **−20,294 / −20,255 / −20,767 / −20,745**.

### Delta audit (the stop rule)

**0 structural geometry lines in all eight cells.** Every `Outer wall`,
`Inner wall`, `Support` and `Support interface` difference is an `E`-token-only
change on an unchanged `G1 … X Y Z … F` move, an `M73` progress counter, an
`M106`/`M107` fan command that relocated with the role boundary, or an
appendix comment. No coordinate, feedrate, travel, skirt, brim or raft line
moved. Fill sections re-role **1,864–2,067** benchy lines and **11,465–15,018**
base lines; that is the classification's purpose and is not a stop-rule
failure. The audit buckets fan commands separately (`fan_control`) because
they are role-keyed aux commands, not motion; 60–71 per cell, three of them in
a structural section in each base cell.

### Paired gate

Three interleaved pairs per cell, `RAYON_NUM_THREADS=12`, ordinary arms only.
Median wall (baseline → candidate), floor multiple, and median CPU delta:

| cell | wall | multiple | CPU |
| --- | ---: | ---: | ---: |
| benchy classic-off | 4.91 → 4.53 s (−7.60 %) | 9.5x | −1.0 s |
| benchy classic-on | 8.92 → 8.65 s (−3.04 %) | 3.8x | −1.4 s |
| benchy arachne-off | 5.00 → 4.65 s (−7.02 %) | 8.8x | −1.1 s |
| benchy arachne-on | 8.93 → 8.68 s (−2.73 %) | 3.4x | −1.1 s |
| base classic-off | 49.28 → 38.33 s (−22.23 %) | 27.8x | −109.4 s |
| base classic-on | 116.48 → 108.07 s (−7.23 %) | 9.0x | −108.9 s |
| base arachne-off | 48.92 → 38.68 s (−20.94 %) | 26.2x | −110.2 s |
| base arachne-on | 117.88 → 108.52 s (−7.94 %) | 9.9x | −103.9 s |

Every cell is strictly separated on wall and on CPU (the candidate's slowest
run is faster than the baseline's fastest; its highest CPU is below the
baseline's lowest). Narrowest multiple: benchy arachne-on at **3.4x**; widest:
base classic-off at **27.8x**.

### What was retired

`classify_region_surfaces`, `assemble_bridge_areas` (the facet / 1.0 mm
`expansion_margin_mm` path), `assemble_flat_bridge_areas`, the
`PrePass::ShellClassification` unsupported-span gate, the bottom-surface
footprint cache and helper, and the `surface_class` parameter of the layer-slice
entry points. `classify_unsupported_bridge_areas` now applies canonical
`PrintObject::detect_surfaces_type`'s
`opening_ex(diff(current_raw, safety_offset(previous_raw)), 0.1 x
external_perimeter_width)` to the region's whole unsupported set.
`update_external_bridge_orientation`, the internal-bridge gate, paint splits and
`BridgeSpatialIndex` are unchanged. `SurfaceClassificationIR.bridge_regions` /
`expansion_margin_mm` remain for visual debug and guest observation; no
`PrePass::Slice` consumer remains. `DEV-200` records the unported wave
expansion / zone trim; `DEV-039` (the retired bounding-box fallback) is closed.

### Disclosures

No Orca arm (the post-merge scoreboard remains t121's closing measurement). No
independent load measurement: the slicer saturates its own 12 Rayon threads;
output-mode walls are single runs, only the A/B medians are gated. The t121
per-layer fix is present in both arms. t122's area prediction is not
re-measured from G-code; the line-level audit is the confinement gate. The
removed footprint cost is included in the paired delta but was not part of the
t122 screen. Both guest freshness checks exit 0 (rebuild required because the
fingerprint closure includes `slicer-core`; the WIT world is unchanged).
Baseline binaries `abad3599…` (ordinary) / `40fab992…` (accelerated) at
`270ec239`; candidate `7737a74f…` / `a6ae2adc…`.

Evidence:
[`evidence/t123-facet-retirement/`](../evidence/t123-facet-retirement/FINDINGS.md)
— `verify.py` exits 0 checkout-only (309 checks); `--self-test` rejects 11/11
negative controls.

# t123 — facet bridge-path retirement: canonical classification over the whole unsupported set

Ticket: [Facet bridge-path retirement](../../issues/123-facet-bridge-path-retirement.md)
(wayfinder map [Slice performance vs OrcaSlicer](../../map.md)).
Take: 2026-11-06/07, the only takeable AFK frontier child at HEAD `270ec239`
(t117 is the human's grilling). Baseline = HEAD `270ec239`; candidate = this
take's working tree. Matched job, `RAYON_NUM_THREADS=12`,
`--no-default-module-paths`, in-tree `modules/core-modules` (ordinary) and the
mode-distinct accelerated staging (accelerated).

## Verdict

**BUILD.** The canonical whole-set rule replaces PnP's facet/flat split, and it
is cheaper in every cell:

- **16-cell output contract: met.** Baseline ordinary ≡ baseline accelerated ≡
  the t116 candidate set in all eight cells; candidate ordinary ≡ candidate
  accelerated; the candidate moves the output in every cell.
- **Delta stop rule: not triggered.** The eight-cell audit finds **0 geometry
  lines** in any structural section. The wall/support differences are
  extrusion-token-only (same `G1` coordinates and feedrate, different `E`) on
  segments the new bridge flag covers; fan commands (`M106`/`M107`) relocate
  with the role boundary and progress counters (`M73`) follow the time
  estimate. No coordinate, feedrate, travel, skirt, brim or raft line moved.
- **Paired gate: 8/8 cells clear the 0.80 % floor** at **3.4x–27.8x**, with
  process CPU corroborating in every cell and every cell strictly separated on
  both wall and CPU (table in §4). The narrowest cell is benchy arachne-on
  (3.4x); the widest is base classic-off (27.8x).

The base output shrinks by ~20.3 KB per cell; the benchy output grows by
0.8–1.3 KB. The retired facet path was producing a near-canonical **area** on
base (t122: 1.04x) at 27x the canonical classifier's cost, so the base wall win
is dominated by removing that work; the delta audit still records the role
movement the area screen predicted.

## 1. What changed

`PrePass::Slice` no longer classifies bridges from mesh facets. The new
`classify_unsupported_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`)
applies canonical OrcaSlicer's `PrintObject::detect_surfaces_type` rule to the
region's whole unsupported bottom set:

```text
unsupported = difference(current_raw, offset(previous_raw, SCALED_EPSILON, Square))
bridged     = opening_ex(
                  intersection(unsupported, region.infill_areas),
                  0.1 × external_perimeter_width,
                  Miter, 3.0)
region.bridge_areas = bridged
region.is_bridge    = !bridged.is_empty()
```

- **Deleted:** `classify_region_surfaces` (the facet scan and its
  point-in-polygon test), `assemble_bridge_areas` (the facet/1.0 mm
  `expansion_margin_mm` path), `assemble_flat_bridge_areas` (the flat-family
  special case), `gate_bridge_areas_by_unsupported_span` and its
  `PrePass::ShellClassification` call, `batch_bottom_surface_footprints`, the
  `PrepassSliceCache::bottom_surface_footprint` field, and the now-unused
  `mesh_analysis::bottom_surface_footprint` helper.
- **Removed parameter:** `execute_prepass_slice_single_layer[_with_cache]` no
  longer takes `surface_class`; bridge classification is geometric. The stage
  still waits on `SurfaceClassification` because `PrePass::Slice`'s stage
  ordering requires it (the runtime `required_slots` table is unchanged).
- **Kept:** `update_external_bridge_orientation` (now fed by the classified
  areas), `gate_internal_bridge_sites` and the internal-bridge contract, the
  paint splits, `BridgeSpatialIndex`, and the `SurfaceClassificationIR`
  `bridge_regions` / `expansion_margin_mm` fields. Those fields still feed
  visual debug and the whole-print-input guest observation surface; **no
  `PrePass::Slice` consumer remains**, and the ticket's audit is recorded here
  rather than deleting a still-observed IR field.
- **Deviation log:** `DEV-200` records the knowingly-kept divergence — the
  classification half is canonical, the wave expansion / zone trim is not
  ported (t122 measured it as a per-layer partition change costing 102.6 % of
  the benchy classification saving). `DEV-039` (the old bounding-box fallback
  for `classify_region_surfaces`) is closed as moot: the mechanism is retired.
- **Tests re-derived:** the flat-family unit tests became whole-set
  classification tests; `bridge_false_site_gating_tdd` re-derives the gate's
  claims (supported span → none, unsupported retained, missing lower layer
  clears, existing empty lower layer classifies the whole span) against the
  classifier; `bridge_detector_tdd` retires the facet-only tests
  (`expansion_margin_grows_polygon_observably`,
  `vshape_sharp_anchor_pipeline_produces_simple_polygons`,
  `invalid_bridge_excluded_from_slice_areas`,
  `bridge_footprint_does_not_leak_outside_facet_z_span`) and re-derives
  `non_bridge_region_has_empty_bridge_areas`, the supported-candidate test and
  the two layer-diff tests under the whole-set rule.

## 2. Output: the 16-cell set

Eight cells × two arms per build; hashes from `target/t123/output/rows.csv`
(reduced into `reduced/summary.json`).

| cell | baseline ordinary | candidate ordinary | bytes (base → cand) | Δ bytes |
| --- | --- | --- | ---: | ---: |
| benchy classic-off | `35600ee4…` | `90638bf3…` | 3,926,024 → 3,926,897 | +873 |
| benchy classic-on | `596f6da5…` | `711c6334…` | 8,252,846 → 8,253,678 | +832 |
| benchy arachne-off | `f21df124…` | `2ba5d84c…` | 4,064,091 → 4,065,372 | +1,281 |
| benchy arachne-on | `e90f491d…` | `a972b0f2…` | 8,390,069 → 8,391,228 | +1,159 |
| base classic-off | `6a6a3d9d…` | `b3b4ac5e…` | 21,026,410 → 21,006,116 | −20,294 |
| base classic-on | `8f46fe92…` | `4e31a2db…` | 42,510,534 → 42,490,279 | −20,255 |
| base arachne-off | `655c99d4…` | `6b865d4a…` | 20,847,171 → 20,826,404 | −20,767 |
| base arachne-on | `8b5d1de0…` | `62d6eedc…` | 42,331,252 → 42,310,507 | −20,745 |

- **Baseline reproduction.** Every baseline ordinary hash equals the
  corresponding `output.<cell>.candidate-ordinary.sha256` in
  [`../t116-prep-slice-canonical-bridge/reduced/summary.json`](../t116-prep-slice-canonical-bridge/reduced/summary.json),
  and every baseline accelerated hash equals its ordinary partner.
- **Arm equality.** Candidate ordinary equals candidate accelerated in all
  eight cells, and the candidate moves every cell.
- The byte signs follow the net role movement: benchy gains bridge/fill area
  (t122 predicted +618.44 / −18.45 mm²), while base shrinks by ~20.3 KB. No
  byte count is offered as an area claim; the delta audit is the confinement
  gate.

## 3. Delta audit (the stop rule)

`audit_delta.py` splits both files into per-layer, per-`;TYPE:` sections and
classifies every changed line as `extrusion_only` (identical motion, different
`E`), `appendix` (`;` comments / time estimates), `progress_counter` (`M73`),
`fan_control` (`M106`/`M107`), or `geometry` (a moved toolpath). Geometry lines
in a structural section are the stop-rule failure.

| cell | fill geometry lines | structural geometry lines | structural sections |
| --- | ---: | ---: | --- |
| benchy classic-off | 1,864 | **0** | Inner wall: 2 E-only, 8 M73 |
| benchy classic-on | 1,862 | **0** | Inner wall: 8 E-only; Outer wall: 4 E-only; Support: 3 E-only, 12 M73 |
| benchy arachne-off | 2,067 | **0** | Inner wall: 7 E-only, 4 M73, 2 appendix |
| benchy arachne-on | 2,060 | **0** | Inner wall: 3 E-only, 2 appendix; Outer wall: 1 E-only; Support: 4 E-only, 32 M73 |
| base classic-off | 11,507 | **0** | Inner wall: 2 E-only, 3 fan, 3 appendix; Outer wall: 48 E-only |
| base classic-on | 11,465 | **0** | Inner wall: 3 fan, 3 appendix; Support: 137 E-only, 24 M73; Support interface: 10 M73 |
| base arachne-off | 15,018 | **0** | Inner wall: 70 E-only, 3 fan, 2 appendix; Outer wall: 176 E-only |
| base arachne-on | 14,990 | **0** | Inner wall: 17 E-only, 3 fan, 3 appendix; Outer wall: 94 E-only; Support: 296 E-only, 90 M73; Support interface: 22 M73 |

- **Structural sections carry no moved geometry in any cell.** Every `Outer
  wall`, `Inner wall`, `Support` and `Support interface` line that differs is
  either an `E`-token-only change on an unchanged `G1 … X Y Z … F` move (the
  per-vertex bridge flag feeds wall/support flow) or a non-motion command
  (`M73` progress, `M106`/`M107` fan, or the appendix comment block).
- **Fan control is a role-keyed aux command, not motion.** `M106`/`M107` sits
  at extrusion-role boundaries, so a changed bridge role relocates the command
  between sections. The audit buckets it separately instead of counting it as
  geometry. There are 60–71 fan lines per cell in total; only the three that
  land in a structural section are listed above.
- **Fill geometry is expected.** The bridge/fill family re-roles thousands of
  fill lines (benchy ~1.9–2.1 K, base ~11.5–15 K); that is the classification's
  purpose, and the stop rule only constrains structural sections.
- **Confinement matches the t122 screen.** The changed section types are
  exactly the bridge/fill roles plus the structural flows the per-vertex flag
  feeds; no coordinate, feedrate, travel, skirt, brim or raft line moved.

## 4. Paired gate

Three interleaved pairs per cell, baseline ordinary vs candidate ordinary,
`RAYON_NUM_THREADS=12`, pair order alternating. Wall is process
creation-to-exit; CPU is kernel+user via the process handle.

| cell | baseline median | candidate median | delta | floor (0.80 %) | x floor | CPU delta |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic off | 4.91 s | 4.53 s | 0.37 s (7.60 %) | 39.3 ms | **9.5x** | −1.0 s |
| benchy classic on | 8.92 s | 8.65 s | 0.27 s (3.04 %) | 71.3 ms | **3.8x** | −1.4 s |
| benchy arachne off | 5.00 s | 4.65 s | 0.35 s (7.02 %) | 40.0 ms | **8.8x** | −1.1 s |
| benchy arachne on | 8.93 s | 8.68 s | 0.24 s (2.73 %) | 71.4 ms | **3.4x** | −1.1 s |
| base classic off | 49.28 s | 38.33 s | 10.96 s (22.23 %) | 394.3 ms | **27.8x** | −109.4 s |
| base classic on | 116.48 s | 108.07 s | 8.42 s (7.23 %) | 931.9 ms | **9.0x** | −108.9 s |
| base arachne off | 48.92 s | 38.68 s | 10.24 s (20.94 %) | 391.4 ms | **26.2x** | −110.2 s |
| base arachne on | 117.88 s | 108.52 s | 9.36 s (7.94 %) | 943.1 ms | **9.9x** | −103.9 s |

**All eight cells clear the 0.80 % floor, process CPU corroborating in every
cell.** Every cell is **strictly separated on wall and on CPU**: in each cell
the candidate's slowest run is faster than the baseline's fastest, and the
candidate's highest CPU is below the baseline's lowest. Raw ranges (3 per arm):

- benchy classic-off 4.977/4.802/4.907 vs 4.572/4.461/4.534;
- benchy classic-on 9.054/8.865/8.918 vs 8.757/8.647/8.413;
- benchy arachne-off 4.997/5.034/5.003 vs 4.607/4.734/4.652;
- benchy arachne-on 8.905/9.028/8.926 vs 8.682/8.826/8.610;
- base classic-off 49.285/49.446/48.196 vs 38.329/38.798/38.294;
- base classic-on 116.389/117.449/116.484 vs 106.292/108.533/108.065;
- base arachne-off 48.919/49.701/48.574 vs 38.677/39.240/38.155;
- base arachne-on 117.883/116.275/118.633 vs 106.814/108.609/108.523.

## 5. Disclosures

- **The post-merge per-layer dispatch regression is fixed in both arms.**
  HEAD carries the t121 per-layer fix (`dc945a9a`), so the cell walls here are
  the fixed tree's, not the pre-merge set; both arms pay the same per-layer
  cost and the paired delta isolates the prepass change.
- **No independent load measurement.** The slicer saturates its 12 Rayon
  threads, so a CPU-load percentage cannot separate this workload from
  anything else on the box. The campaign ran sequentially (output first, then
  the A/B); no starvation exclusions were applied and no run was rerun.
  Output-mode wall times are single runs, not timing claims; only the A/B
  medians are gated.
- **No Orca arm.** The post-merge Orca scoreboard remains
  [ticket 121](../../issues/121-post-merge-per-layer-dispatch-regression.md)'s
  closing measurement; no ratio of record is quoted here.
- **The area prediction is not re-measured from G-code.** t122's +618.44 /
  −18.45 mm² (benchy) and +564.17 / −384.82 mm² (base) are the probe's
  classification bounds. The campaign's gate is the line-level delta audit,
  which confirms the movement is confined to the bridge/fill roles; no G-code
  area decomposition is claimed.
- **Fan control moved with the role boundary.** `M106`/`M107` is emitted at
  extrusion-role boundaries, so the new bridge set relocates 60–71 fan lines
  per cell. The audit buckets them as `fan_control` (not geometry); three land
  in a structural section in each base cell. This is a role-keyed aux command,
  not a toolpath move.
- **The removed bottom-surface footprint cost is not separately claimed.**
  The old path computed `batch_bottom_surface_footprints` once per object; the
  new classifier does not need it. That saving is included in the paired delta
  but was not part of t122's per-layer screen.
- **Guest freshness.** Both `cargo xtask build-guests --check` and
  `--accelerated --check` exit 0. Rebuilding was required because the guest
  fingerprint closure includes `slicer-core`; the WIT world is unchanged, and
  baseline output still reproduces the t116 set byte-for-byte.
- **Binaries.** Baseline ordinary `abad3599…` and accelerated `40fab992…` were
  built at HEAD `270ec239` before the candidate edit. Candidate ordinary
  `7737a74f…` and accelerated `a6ae2adc…` were built on the candidate tree.
  Ordinary and accelerated arms use their own staged module trees.
- **IR fields retained.** `SurfaceClassificationIR.bridge_regions` and
  `expansion_margin_mm` remain in the schema for visual debug and the
  whole-print-input guest observation surface; no `PrePass::Slice` consumer
  remains. `DEV-200` records the unported wave expansion, and `DEV-039` (the
  retired bounding-box fallback) is closed.

## Evidence index

- `run_t123.ps1` — campaign driver (`-Mode output`, `-Mode ab`).
- `audit_delta.py` — section-level delta classifier.
- `reduce_t123.py` — raw CSV + audit rows → `reduced/summary.json`.
- `verify.py` — exits 0 from a plain checkout; `--self-test` rejects every
  negative control.
- `reduced/summary.json` — the committed numbers.

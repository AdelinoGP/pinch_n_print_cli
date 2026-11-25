# t142 — Overhang band-partition deletion: distance-derived quartile, output-neutral

**Verdict: the four-band polygon partition is gone from the slice path, the
per-vertex quartile is derived from the signed distance the perimeter
annotation already measures, and the matched job's G-code is byte-identical in
8/8 cells.** The wall drops in every cell, and the per-cell saving is far above
the ticket's 0.80 % paired floor.

The change is a work-set deletion, not a per-op optimisation. t141 priced the
partition at **8.964–10.583 s** on base classic-off (`evidence/t141-overhang-bands/`),
22–27 % of that cell's wall; the deletion removes it outright.

## 1. What changed

### The stage (`PrePass::OverhangAnnotation`)

`annotate_overhangs` (`crates/slicer-core/src/algos/overhang_annotation.rs`) no
longer builds band polygons. It returns
`OverhangAnnotation { overhang_areas, prev_layer_boundaries }`: the unsupported
area `current \ previous` and the previous-layer footprint, keyed by global
layer index and present only on layers that actually overhang. The per-layer
`difference_ex(current, previous)` key test is kept (**Shape A**), so
`overhang_distance_mm` keeps its historical `None` on non-overhanging layers.

Deleted work per overhang layer: **3 round offsets + 6 whole-layer booleans**
(`intersection(current, offset(previous, lw × {0.5, 1.0, 1.5})) - previous`,
then the three band differences). What remains is the single boolean that
decides whether the layer overhangs at all.

### The consumer

`PerimeterSpatialContext::new(boundary, overhang_band_width_mm, bridge_areas)`
takes the producer's band width instead of band polygons, and
`overhang_quartile(x, y)` is
`overhang_quartile_from_distance(signed_distance_to_boundary(x, y), width)`
(`crates/slicer-core/src/perimeter_utils.rs`):

| quartile | raw signed distance |
| --- | --- |
| `None` | `<= 0.0001 mm` (on or inside the previous boundary) |
| 1 | `(ε, 0.5 × lw]` |
| 2 | `(0.5 × lw, 1.0 × lw]` |
| 3 | `(1.0 × lw, 1.5 × lw]` |
| 4 | `> 1.5 × lw` |

The old 4-band union was exactly `current \ previous`, so presence is the sign
of the same signed distance; the epsilon matches Clipper's 1-unit
(0.0001 mm) quantisation, which is the case a point **on** the previous
boundary would otherwise straddle. The quartile **value** still has no
production reader (t141 §2); it is produced correctly for future readers.

The band R-tree (`quartile_y_index`, `QuartileYRecord`,
`indexed_overhang_quartile`, `legacy_overhang_quartile`) is deleted, and
`expolygon_to_path3d_indexed` / arachne's annotation loop now measure the
distance **once** and stamp both `overhang_distance_mm` and
`overhang_quartile`. Before, the same vertex paid one distance query plus a
band point-in-polygon query.

### The IR and the transport

* `SurfaceClassificationIR` gains `overhang_areas` (object-level unsupported
  area by layer) and `overhang_band_width_mm` (the producer's resolved
  `outer_wall_line_width` → `line_width` → `0.4` chain). The production path
  leaves `overhang_quartile_polygons` **empty**; the field is retained for
  diagnostics.
* `PreparedRegionData.overhang_band_width_mm` carries the scalar to the
  layer modules; the WIT `slice-region-view` gains `overhang-band-width-mm`.
  Both guest trees were rebuilt against the new WIT.
* `PreparedRegionData.overhang_areas` (used by
  `extra_perimeters_on_overhangs`) now comes from the annotation's unsupported
  area intersected with the region — one intersection per region instead of
  four band intersections — so that feature keeps working without the bands.
* `region.overhang_areas` in the WIT view is sourced from
  `SurfaceClassificationIR.overhang_areas`, not from the flattened bands.

### The diagnostic tap (fork 2)

The visual-debug renderer reconstructs the bands at render time from
`overhang_areas` + `prev_layer_boundaries` + `overhang_band_width_mm`
(`reconstructed_overhang_bands` in
`crates/slicer-runtime/src/visual_debug_render.rs`), using the retained
`partition_into_bands` helper (now `#[must_use]`, called only from the renderer
and its ramp test). An explicit band set in the IR still wins, so existing
silhouette fixtures and a future diagnostic producer keep working. No slice
path calls the helper.

## 2. Fork decisions (settled by measurement, not assumption)

1. **Layer key — Shape A adopted, Shape B rejected.** Shape B (populate the
   previous boundary on every layer and delete the diff) would change
   `overhang_distance_mm` from `None` to `Some(negative)` on non-overhanging
   layers. That is a real IR-contract change with no measured benefit: the diff
   is one whole-layer boolean per layer, and t141's offset census attributes
   **≥ 85.4 %** of the stage to the offsets/booleans that are deleted either
   way. With A the matched job is byte-identical (below); there was nothing to
   buy by taking the risk.
2. **Debug surface — renderer reconstruction, tap retained.** No production
   cost: the reconstruction runs inside `visual_debug_render`. Retiring the tap
   outright was unnecessary because the annotation still carries everything the
   reconstruction needs.

## 3. Output gate — byte-identical in 8/8 cells

`-Mode output` (baseline-ordinary vs candidate-ordinary), one run per cell per
arm, 16 rows. SHA-256 is over the whole G-code file:

| cell | sha256 (both arms) | bytes | t139 reference |
| --- | --- | ---: | --- |
| base classic-off | `ca14d0554e37…` | 21,184,281 | `ca14d055` ✓ |
| base classic-on | `585c6a8edc2f…` | 38,004,599 | `585c6a8e` ✓ |
| base arachne-off | `d8fdc1d990b7…` | 21,045,270 | `d8fdc1d9` ✓ |
| base arachne-on | `c9fee59a2342…` | 37,866,953 | `c9fee59a` ✓ |
| benchy classic-off | `93e21fee6d75…` | 4,084,733 | `93e21fee` ✓ |
| benchy classic-on | `b3f1ecccc538…` | 8,732,233 | `b3f1eccc` ✓ |
| benchy arachne-off | `3510e8761d65…` | 4,229,137 | `3510e876` ✓ |
| benchy arachne-on | `96cdac854370…` | 8,873,041 | `96cdac85` ✓ |

Both arms are byte-identical **within every cell**, and the candidate's hashes
are t139's reference set character for character. The reference set therefore
does not move, and the disclosure attached to t139's scoreboard stays valid.
The A/B runs (`ab`) re-emit the same eight hashes on every one of their 48
rows, so the equality is not a single-shot artifact.

## 4. Timing — paired A/B, 3 interleaved pairs per cell

`-Mode ab`, baseline-ordinary vs candidate-ordinary, order alternating by pair
parity, 48 rows. Medians are over the three pairs; wall is uninstrumented
process creation-to-exit, CPU is the process's kernel+user total:

| cell | baseline wall (s) | candidate wall (s) | saving | ratio | baseline CPU (s) | candidate CPU (s) | CPU delta |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| base classic off | 39.141 | 28.868 | **26.25 %** | 0.738 | 311.8 | 210.1 | 32.6 % |
| base classic on | 94.848 | 83.699 | **11.76 %** | 0.882 | 587.8 | 485.4 | 17.4 % |
| base arachne off | 39.284 | 29.236 | **25.58 %** | 0.744 | 314.9 | 213.0 | 32.3 % |
| base arachne on | 94.725 | 85.548 | **9.69 %** | 0.903 | 592.8 | 490.6 | 17.2 % |
| benchy classic off | 4.363 | 3.490 | **20.00 %** | 0.800 | 25.0 | 18.5 | 25.9 % |
| benchy classic on | 8.305 | 7.428 | **10.56 %** | 0.894 | 40.9 | 34.9 | 14.6 % |
| benchy arachne off | 4.202 | 3.352 | **20.23 %** | 0.798 | 26.1 | 19.7 | 24.7 % |
| benchy arachne on | 8.235 | 7.192 | **12.66 %** | 0.873 | 42.8 | 35.3 | 17.6 % |

**Every cell clears the 0.80 % floor by an order of magnitude** (9.69 %–26.25 %),
the candidate wins **all 24 individual pairs** (per-pair ratios 0.738–0.940),
and process CPU corroborates the wall in 8/8 (a 14.6 %–32.6 % CPU reduction —
larger than the wall saving, as expected when the deleted work was partly
serial). No cell regresses on either metric.

Reading the shape: the supports-off cells save ~20–26 % of their wall, which
matches t141's 22–27 % price for the stage; the supports-on cells save ~10–13 %
because the support engine dominates their wall, and the deleted stage is a
smaller share of it. The absolute stage cost is roughly constant across the
cells (t141 measured 8.96–11.2 s), so the saving tracks the cell wall.

The output hashes are stable across all 48 A/B rows as well (each cell emits
exactly one sha256), so the byte-equality is not a single-run artifact.

## 5. What is not claimed / gaps

- **Accelerated timing arm not taken; accelerated output parity checked with a
  disclosed manual host build.** `cargo xtask build-guests --accelerated`
  rebuilt the accelerated guest artifacts, but `cargo xtask dist --accelerated
  --edition developer` cannot build the controlled accelerated host binary in
  this environment: the driver's `RUSTC` wrapper hands `libmimalloc-sys`'s C++
  build script a verbatim (`\\?\`) source path and MSVC 14.51 mangles it into
  `\mimalloc-static.cc` (`C1083`). The same build **without** the wrapper
  succeeds, so the failure is in the driver/toolchain interaction, not in this
  change. To keep the output gate honest, the accelerated **output** was still
  checked: a host built with `RUSTFLAGS=--cfg
  pnp_perimeter_spatial_accelerated` (no wrapper) plus the accelerated guest
  artifacts emits the **same eight hashes** in every cell (gate
  `accelerated_output_matches_candidate_8_of_8` in `reduced/campaign.json`).
  That artifact is not the controlled-driver build and is used for parity only,
  never for timing. The 16 output cells in §3 are 8 cells × {baseline,
  candidate} ordinary. See `REPRODUCE.md` §5.
- **The box was not verified idle.** The same-session baseline walls drift
  between campaigns and between modes: the `-Mode output` baseline
  base-classic-on run took 117.4 s, while its A/B median is 94.8 s. That
  variance is why every claim here is paired within the campaign, with the two
  arms interleaved by pair parity, rather than compared across sessions. The
  A/B baseline medians are within ~2 % of t139's scoreboard values for the
  base cells (39.1 vs 37.0–40.6 s; 94.8 vs 93.55 s), so the box was in its
  usual range on the day.
- **`overhang_quartile` value is unread** in production (t141 §2). The gate
  proves presence equivalence on this job; it cannot prove that the bucket
  boundary behaves identically for a future reader. The bucket function is
  unit-tested at every threshold.
- **Pre-existing reds found, not caused by this change:**
  `integrated_parity_skirt_brim_tdd::integrated_parity_skirt_brim` fails at
  HEAD too (`brim_object_gap` missing from its fixture `ConfigView`, a t135
  leftover); `layer_slice_tdd::layer_slice_builtin_produces_real_polygons_from_mesh`
  slices a layer declared at z = 0.1 with height 0.2, i.e. t139's midpoint
  plane is z = 0.0 on the mesh's bottom face, and the test predates that
  change. Neither is in this ticket's scope; both are recorded here so a red
  run is not misattributed.
- **Two latent-vacuous fixtures repaired in passing.** The two
  `overhang_quartile` parity tests (runtime and guest) asserted band-based
  quartiles but never exercised a vertex inside the band — every expectation
  was `None` — and their configs did not enable the classifier predicate, so
  they would have been `None`-only anyway. Both now carry the boundary + band
  width, assert the distance-derived bucket, and fail if no vertex is
  overhanging.

## 6. Provenance

- Driver: [`run_t142.ps1`](run_t142.ps1); commands: [`REPRODUCE.md`](REPRODUCE.md)
  (the accelerated output arm is `-Arms candidate-accelerated`, batch
  `output-accel2`).
- Reducer: [`reduce_t142.py`](reduce_t142.py) → [`reduced/campaign.json`](reduced/campaign.json).
- Verifier: [`verify.py`](verify.py) (re-derives every number from the reduced
  JSON; `--self-test` rejects 4/4 negative controls).
- Prior facts reused: t141 (partition price, the reader census, the two forks),
  t139 (reference set, scoreboard walls), t84 (offset census, 85.4 % share).

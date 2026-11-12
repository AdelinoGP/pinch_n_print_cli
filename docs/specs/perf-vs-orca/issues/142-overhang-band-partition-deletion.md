# Overhang band partition deletion: derive the quartile from the distance the consumer already has

Type: task / measured candidate
Label: wayfinder:task
Status: resolved (2026-11-11; Shape A landed — partitions deleted, distance-
  derived quartile, 8/8 byte-identical, 9.69–26.25 % paired wall saving)
Assignee: 2026-11-11 OpenCode wayfinder session — the human said "continue the
  perf vs orca map, pick an afk ticket"
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Overhang quartile bands: is the 4-band polygon partition read by anything?](141-overhang-quartile-band-deletion.md)'s
  2026-11-11 Phase-1: the quartile **value** has no production reader, the only
  read is a presence bit equal to the sign of the signed distance the consumer
  already computes, and the partition is **9.6–10.6 s** of the base
  classic-off wall (24–26 % of it, 37–40 % of the cell's required cut).

## Question

Delete `partition_into_bands` from `PrePass::OverhangAnnotation` and derive
`overhang_quartile` — presence, and the bucket value if anything ever reads it
— from the `signed_distance_to_boundary` the perimeter annotation already
computes, in the shape that keeps the matched job's G-code byte-identical.

Two forks from the Phase-1, both to be settled by measurement, not assumption:

1. **The layer key.** `annotate_overhangs` returns the previous-layer boundary
   only for layers that have overhang, which is why `overhang_distance_mm` is
   `None` on a layer with none. Shape A keeps the per-layer
   `difference_ex(current, previous)` key test and deletes only the partition
   (expected saving ≥ 85.4 % of the stage wall, t84's offset share). Shape B
   populates the boundary on every layer and deletes the whole stage; it is
   only admissible if `insert_extended_points_with_point_widths` provably
   inserts nothing on a `current ⊆ previous` layer and no other distance reader
   changes behaviour.
2. **The debug/IR surface.** `overhang_quartile_polygons` feeds
   `visual_debug_render`'s band silhouettes and the preparation-input keys. The
   preferred shape leaves the IR field empty on the production path and makes
   the renderer reconstruct bands from `prev_layer_boundary` + the resolved
   line width at render time; retiring the tap is the alternative. Neither may
   cost anything on the slice path.

Where the distance-derived quartile lands matters: the producer resolves the
band width as `outer_wall_line_width` falling back to `line_width` (0.4 in the
matched job), while the consumer knows the per-point extrusion `width`. The
threshold source must be the producer's width, so the context needs the scalar
(`line_width_mm` or the three thresholds), not the polygons.

Gate — the standing contract, unchanged:

- **Output:** byte-identical G-code in the 16 arm/cell cells, or a delta
  audited before timing, confined to `E` and to overhang-speed sections. The
  Phase-1 argument says the only possible disagreement is a point lying on the
  previous boundary; a non-zero delta must be shown to be exactly that set.
- **Timing:** median uninstrumented wall at the **0.80 %** paired floor with
  process CPU corroborating, in **all 8** cells (the shape touches every cell),
  with no cell's wall or CPU regressing.
- **Freshness:** `cargo xtask build-guests --check` exit 0 before attributing
  anything to a guest; `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo xtask check-literals` before committing.
- **Evidence:** a new `evidence/t142-…/` set with `verify.py` and negative
  controls; the reference set is t139's unless the output moves.

## Answer

**Resolved 2026-11-11: the partition is deleted, the quartile is distance-derived,
and the matched job's G-code is byte-identical in 8/8 cells. The wall drops
9.69 %–26.25 % across the eight cells (all 24 individual pairs won, CPU
corroborating 8/8), with no reference-set movement.**

### What landed

- `annotate_overhangs` (`crates/slicer-core/src/algos/overhang_annotation.rs`)
  now returns `OverhangAnnotation { overhang_areas, prev_layer_boundaries }`.
  The 4-band construction — 3 round offsets + 6 whole-layer booleans per
  overhang layer — is gone; only the `difference_ex(current, previous)` key
  test remains (**Shape A**, see below).
- `PerimeterSpatialContext::new(boundary, overhang_band_width_mm, bridge_areas)`;
  `overhang_quartile(x, y)` is
  `overhang_quartile_from_distance(signed_distance_to_boundary(x, y), width)`
  (`crates/slicer-core/src/perimeter_utils.rs`). The band R-tree
  (`quartile_y_index`, `QuartileYRecord`, `indexed_overhang_quartile`,
  `legacy_overhang_quartile`) is deleted. Classic's
  `expolygon_to_path3d_indexed` and arachne's annotation loop measure the
  distance **once** and stamp both `overhang_distance_mm` and
  `overhang_quartile`.
- `SurfaceClassificationIR` gains `overhang_areas` and
  `overhang_band_width_mm`; production leaves `overhang_quartile_polygons`
  empty. The scalar reaches the guests through
  `PreparedRegionData.overhang_band_width_mm` and the new WIT
  `slice-region-view.overhang-band-width-mm`; both guest trees were rebuilt.
- `extra_perimeters_on_overhangs` keeps working from
  `PreparedRegionData.overhang_areas`, now the annotation's unsupported area
  intersected with the region (one intersection per region instead of four
  band intersections).
- The visual-debug renderer reconstructs bands at render time from
  `overhang_areas` + `prev_layer_boundaries` + `overhang_band_width_mm`
  (`reconstructed_overhang_bands` in
  `crates/slicer-runtime/src/visual_debug_render.rs`); explicit bands still win.
  No slice path calls `partition_into_bands`.

### Gates

- **Output — PASS, 8/8 byte-identical.** Baseline and candidate emit the same
  SHA-256 in every cell, and every hash is t139's reference value
  (`ca14d055…`, `585c6a8e…`, `d8fdc1d9…`, `c9fee59a…`, `93e21fee…`,
  `b3f1eccc…`, `3510e876…`, `96cdac85…`). The reference set does not move, so
  no delta audit was needed. The 48 A/B rows re-emit the same eight hashes.
- **Timing — PASS, 8/8 above the 0.80 % floor** (3 interleaved pairs/cell):
  base classic-off **26.25 %**, base classic-on **11.76 %**, base arachne-off
  **25.58 %**, base arachne-on **9.69 %**, benchy classic-off **20.00 %**,
  benchy classic-on **10.56 %**, benchy arachne-off **20.23 %**, benchy
  arachne-on **12.66 %**. Candidate faster in 24/24 individual pairs
  (ratios 0.738–0.940); process CPU lower in 8/8 (14.6 %–32.6 %); no cell
  regresses on wall or CPU.
- **Freshness / hygiene — PASS.** `cargo xtask build-guests --check` exit 0;
  `cargo clippy --workspace --all-targets -- -D warnings` clean;
  `cargo xtask check-literals` 0 violations.
- **Evidence —** [`evidence/t142-overhang-band-deletion/`](../evidence/t142-overhang-band-deletion/FINDINGS.md):
  `run_t142.ps1`, `reduce_t142.py`, `reduced/campaign.json`, and `verify.py`
  (re-derives every number from the reduced JSON; `--self-test` rejects 4/4
  negative controls).

### Forks settled by measurement

1. **Shape A adopted, Shape B rejected.** Shape B (populate the boundary on
   every layer, delete the diff) would flip `overhang_distance_mm` from `None`
   to `Some` on non-overhanging layers — an IR-contract change with no measured
   benefit, because the diff is one boolean per layer and t141 attributes
   ≥ 85.4 % of the stage to the deleted offsets/booleans either way. Shape A is
   byte-identical on the matched job.
2. **Renderer reconstruction, tap retained.** The reconstruction runs only in
   `visual_debug_render`; no slice cost, and the IR field stays available for a
   diagnostic producer.

### Route arithmetic (indicative, not a scoreboard)

Composing t139's paired scoreboard ratios with this ticket's paired
candidate/baseline ratios (the Orca arm is t139's; the t142 A/B baselines are
within ~2 % of t139's base cells): **1.818x–3.697x** ordinary (was
2.279x–4.187x), required cut **45.0 %–73.0 %** (was 56.1 %–76.1 %). Base
classic-off moves **2.899x → 2.138x**, base classic-on **4.187x → 3.695x**.
The ratios of record remain t139's until the closing 8×3 scoreboard; this is
the implied movement only.

### Gaps

- **Accelerated timing arm not taken; accelerated output parity checked with a
  disclosed manual build.** The accelerated guest artifacts were rebuilt, but
  `cargo xtask dist --accelerated --edition developer` cannot build the
  controlled accelerated host binary in this environment: the driver's `RUSTC`
  wrapper makes `libmimalloc-sys`'s C++ build script receive a verbatim
  (`\\?\`) source path that MSVC 14.51 mangles (`C1083`); the same build without
  the wrapper succeeds. The accelerated **output** was still checked with a
  `RUSTFLAGS=--cfg pnp_perimeter_spatial_accelerated` host plus the accelerated
  guest artifacts: **8/8 cells emit the same hashes** as the ordinary candidate
  (gate `accelerated_output_matches_candidate_8_of_8`). That arm is not the
  controlled-driver artifact and is never used for timing. The 16 output cells
  in §3 are 8 cells × {baseline, candidate} ordinary. Disclosed in `FINDINGS.md`
  §5 and `REPRODUCE.md` §5.
- `overhang_quartile`'s **value** is still unread in production; the gate
  proves presence equivalence on this job, and the bucket function is
  unit-tested at every threshold.
- Two pre-existing reds are recorded, not caused by this change:
  `integrated_parity_skirt_brim_tdd` (missing `brim_object_gap` in its fixture,
  a t135 leftover; confirmed red at HEAD via `git stash`) and
  `layer_slice_tdd::layer_slice_builtin_produces_real_polygons_from_mesh` (a
  layer declared at z = 0.1 / height 0.2 now samples z = 0.0 under t139's
  `layer_sample_plane_z`).
- Two latent-vacuous quartile fixtures were repaired in passing (runtime
  `arachne_parity.rs` and guest `arachne_parity_overhang_quartile_tdd.rs`): both
  asserted band-based quartiles that no vertex exercised, and their configs did
  not enable the classifier predicate. They now carry the boundary + band width,
  assert the distance-derived bucket, and fail if no vertex overhangs.

## Comments

_None yet._

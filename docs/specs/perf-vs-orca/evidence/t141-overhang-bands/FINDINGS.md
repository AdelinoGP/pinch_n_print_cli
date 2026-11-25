# t141 — Overhang quartile bands: the value is unread, the presence bit is the distance sign

**Verdict: GO for a measured deletion candidate.** The 4-band polygon partition
is the largest single named shape left in the supports-off shell cluster, and
its *output value* has no production reader: the only production consumer reads
`overhang_quartile.is_some()`, and that bit is the sign of the signed distance
the consumer already computes against the same previous-layer boundary. The
band polygons can therefore go without changing what the pipeline reads,
leaving their value (and the visual-debug tap) as the only surface to re-home.

This ticket is a Phase-1 attribution and price. No production edit; the build
candidate is graduated as
[t142](../issues/142-overhang-band-partition-deletion.md).

## 1. What the partition costs

The probe (`probe.patch`, env `PNP_PROBE_OVERHANG_BANDS=1`) brackets
`annotate_overhangs` and `partition_into_bands` on the matched job:

| run | stage wall | band thread-time | conversion | band layers | offsets | boolean clips |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| base classic-off (p1) | 9,558 ms | 111,974 ms | 11.71x | 486 / 496 | 1,458 | 2,916 |
| base classic-off (p2) | 10,583 ms | 123,499 ms | 11.67x | 486 / 496 | 1,458 | 2,916 |
| base classic-off (p3, fresh guests) | 8,964 ms | 103,511 ms | 11.55x | 486 / 496 | 1,458 | 2,916 |
| base arachne-off | 9,450 ms | 109,555 ms | 11.59x | 486 / 496 | 1,458 | 2,916 |
| base classic-on | 11,195 ms | 130,741 ms | 11.68x | 486 / 496 | 1,458 | 2,916 |
| benchy classic-off | 954 ms | 7,973 ms | 8.35x | 239 / 240 | 717 | 1,434 |
| benchy classic-on | 817 ms | 6,313 ms | 7.73x | 239 / 240 | 717 | 1,434 |

The counts confirm the module's shape: **3 offsets + 6 whole-layer booleans per
band layer in the cumulative stage** (the probe counts those; the three
band-subtraction differences are not counted), matching t84's 1,455 offsets /
4,859 clips census to within the layer-count drift t139 introduced (495 → 486
band layers of 496). The
thread-to-wall conversion (11.6–11.7x on base) reproduces t84's 11.86x; the
benchy cells convert at 7.7–8.4x on 239 layers.

The three base classic-off probes span **8.96–10.58 s (18 %)**; that is the
measurement floor for this shape and is disclosed rather than averaged away.
The p1/p2 rows ran against the guest artifacts that were on disk before this
session's freshness rebuild; p3 ran after `cargo xtask build-guests` restored
`--check` to exit 0. The host-side stage is unaffected by the guest rebuild,
and the paired same-session walls (39.924 s before, 40.627 s after) differ by
1.8 %.

### Share of the cell

Walls are same-session uninstrumented medians for the three cells that were
re-timed, and the t139 scoreboard medians for the two supports-on cells
(`reference_wall_s_t139_scoreboard` in `probe.json`). The required cut is
t139's `required_cut_pct`.

| cell | wall | stage | share of wall | required cut | share of cut | floor multiple |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| base classic-off | 39.924–40.627 s | 8.964–10.583 s | 22.1–26.5 % | 26.15 s | 34.3–40.5 % | 27.6–33.1x |
| base arachne-off | 39.365 s | 9.450 s | 24.0 % | 25.08 s | 37.7 % | 30.0x |
| base classic-on | 92.554 s | 11.195 s | 12.1 % | 70.43 s | 15.9 % | 15.1x |
| benchy classic-off | 4.372 s | 0.954 s | 21.8 % | 2.53 s | 37.8 % | 27.3x |
| benchy classic-on | 7.878 s | 0.817 s | 10.4 % | 5.46 s | 15.0 % | 13.0x |

This is **not** a route ticket by itself: at t139's ratios a 9.5 s deletion
takes base classic-off from 2.899x to about 2.19x. It is the largest single
unpriced shell shape on the board and the first one whose *work set* (not its
per-op cost) can be deleted outright.

## 2. The value has no production reader

`Point3WithWidth.overhang_quartile` (`crates/slicer-ir/src/slice_ir.rs`) is
stamped at three sites:

- `expolygon_to_path3d_indexed` (`crates/slicer-core/src/perimeter_spatial.rs`),
- `expolygon_to_path3d` (`crates/slicer-core/src/perimeter_utils.rs`),
- `arachne-perimeters`' per-vertex annotation loop
  (`modules/core-modules/arachne-perimeters/src/lib.rs`).

Every other production site either **copies** the field
(`seam-placer`'s `interpolate_point`, `overhang-classifier-default`'s
`interpolate`, `arachne/simplify`, `crates/slicer-core/src/lib.rs`, the
WIT/native marshalling) or tests it for **presence**:

- `overhang-classifier-default`'s speed loop gates on
  `point.overhang_quartile.is_some()`, and the next-point filter gates on the
  same bit;
- the speed itself is computed by
  `calculate_speed(distance, &sections, base)` from `overhang_distance_mm`,
  never from the quartile.

No production line compares the quartile's value (`verify.py` enforces the
census). `overhang_areas` — the flattened band polygons the SDK also exposes —
has **no reader in `modules/`** at all.

The debug-only readers are `visual_debug_render` (band silhouettes) and the
preparation-input key enumeration; both are diagnostics, not output.

## 3. Why the presence bit is the distance sign

`partition_into_bands` builds:

- cumulative(k) = `intersection_ex(current, offset(previous, lw x {0.5,1,1.5})) - previous`,
- band 4 = `overhang_area - cumulative(3)`, where `overhang_area = current - previous`.

The union of the four bands is therefore exactly `current \ previous`, so a
path vertex — which lies inside `current` — is in some band **iff it is outside
`previous`**. The bands are empty outside the overhang area, which is why the
query returns `None` there.

In the same per-vertex loop the consumer already computes
`signed_distance_to_boundary(x, y)` against `prev_layer_boundary`, and
`prev_layer_boundary` is the very same object-level previous-layer set the bands
were built from (`prev_layer_boundaries` → `PreparedRegionData.prev_layer_boundary`
→ `PerimeterSpatialContext::new`). So:

- presence = `raw_signed_distance > 0` (the stamp adds `0.5 x width` later and
  must not be used for the test);
- the quartile value, if anything ever wants it, is the same distance bucketed
  at `line_width x {0.5, 1.0, 1.5}`.

Both the polygon test and the distance test reduce to "outside `previous`"
against the same integer geometry. They can disagree only for points **on** the
previous boundary, where f32's signed distance is ~0 while the polygon
difference excludes the point. A one-unit (0.0001 mm) epsilon on the distance
matches Clipper's quantization and closes that case. This is an argument, not a
measurement: the build's gate is byte equality in the 16 arm/cell cells, and a
non-zero delta must be audited as E-only and confined to overhang-speed
sections.

## 4. Two design forks the build must settle

1. **The layer key is load-bearing.** `annotate_overhangs` returns the previous
   boundary **only for layers that have overhang**, so the consumer stamps
   `overhang_distance_mm = None` on a layer with none. Keeping that pattern
   means keeping the per-layer `difference_ex(current, previous)` key test —
   one whole-layer boolean per layer — and deleting only the partition. That
   retains at most the 14.6 % of native work t84 did not attribute to the
   offsets, so the expected saving is **≥ 85.4 %** of the measured stage wall.
   Populating the boundary on every layer instead looks inert (a layer with no
   overhang is `current ⊆ previous`, so `insert_extended_points_with_point_widths`
   finds no boundary crossing to insert), but that is a claim the build must
   prove, not assume.
2. **The visual-debug tap loses its polygons.** `overhang_quartile_polygons`
   feeds `visual_debug_render`'s band silhouettes and the preparation-input
   keys. The cheapest correct shape is to leave the IR field empty on the
   production path and have the renderer reconstruct bands from
   `prev_layer_boundary` + the resolved line width when it draws them, so the
   diagnostic costs nothing in the slice. Retiring the tap outright is the
   alternative; that is a diagnostic-surface decision, not a parity one.

## 5. What is not claimed

- No A/B, no scoreboard, no reference-set movement: this is a Phase-1 price.
- The box was not verified idle, the three base-off probes span 18 %
  (8.96–10.58 s), and the same-session walls are unpaired. No wall claim for a
  landed change.
- t84's 85.4 % offset share is quoted as the floor for the deletion; the
  residual (diff + clone + map rebuild) was not separately instrumented.
- The value/presence equivalence is argued from the geometry, not measured on
  the emitted G-code; the 16-cell byte-equality gate is the build's job.

## 6. Provenance

- Probe patch: [`probe.patch`](probe.patch); rows and walls:
  [`probe.json`](probe.json); commands: [`REPRODUCE.md`](REPRODUCE.md).
- `verify.py` checks the row arithmetic, the shares against pre-declared bands,
  and the source census; `--self-test` rejects 4/4 negative controls (row
  arithmetic, stage share, a value comparison on the quartile, the distance
  consumer).
- Prior facts reused: t84 (offset census, 85.4 % share, 11.86x), t129 (S1
  price 9,689 ms / 28.5x / 34.8 % of the then-cell cut), t139 (current
  reference set, walls and required cuts), t126 (supports-off prepass
  composition).

# t158 — Exact fast-path boolean predicate in `SliceRegionView::derive_needs_support`

Ticket: [`158-needs-support-fast-predicate.md`](../../issues/158-needs-support-fast-predicate.md)

**Final verdict: KEEP by explicit human ruling on 2026-11-17.** The human reviewed
the original mechanical DROP and ruled **“I accept it as a keep”**; see
[`RULING.md`](RULING.md). The candidate and its tests are landed. The candidate is
exact (8/8 byte-identical G-code matching the t151 active reference set, zero
degraded/error runs), removes the redundant Clipper2 `intersection_ex` trees
across all three call sites (process CPU is lower in **8/8 cells** by **−5.88 % to
−21.38 %**, saving **43.6–71.1 CPU s** on `base` and **1.44–2.47 CPU s** on
`benchy`), and cuts `base` wall time by **+5.42 % to +14.87 %** (**6.78x to
18.58x** the `0.80 %` floor, winning **20/20** `base` pairs).

**The original mechanical gate result remains DROP.** It failed the standing
eight-cell wall gate on two `benchy` cells under the separate-arm median rule:
**`benchy-classic-on` saves +0.766 %** (`0.96x` the `0.80 %` floor, even though
the candidate wins **4/5** pairs and its median pairwise saving is **+2.690 %**)
and **`benchy-arachne-off` moves −0.076 %** (`−0.10x` floor, even though the
candidate wins **3/5** pairs and its median pairwise saving is **+3.158 %**).
Because the original stop rule required `wall_saving_pct >= 0.80 %`
(`(1 - median(cand_wall) / median(base_wall)) * 100`) in **every** cell with zero
sub-floor exceptions, the campaign was initially recorded as DROP and reverted.
The later human ruling changes the disposition, not those measurements;
`reduced/campaign.json` and `verify.py` therefore continue to report the original
mechanical verdict.

## 1. What changed

`SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`)
replaces the unconditional
`!slicer_core::polygon_ops::intersection_ex(&self.polygons, &overhang.xy_footprint).is_empty()`
with `expolygons_intersect_area_positive(&self.polygons, &overhang.xy_footprint)`:

1. **Strict bbox prefilter (`poly_bbox`, `bboxes_strictly_overlap`).** Computes
   each contour's axis-aligned bounding box in integer workspace units (`100 nm`)
   and filters `overhang.xy_footprint` against the union bbox of `self.polygons`,
   then collects strictly overlapping `(ai, bi)` pairs (`a.min_x < b.max_x &&
   b.min_x < a.max_x && a.min_y < b.max_y && b.min_y < a.max_y`). Disjoint and
   boundary-touching-only pairs have zero intersection area and return `false` in
   $O(|A| + |B|)$ without invoking Clipper2.
2. **Strict-interior contour vertex witness (`point_in_ring`,
   `point_strictly_inside_expoly`).** Using exact `i128` cross-product and
   ray-crossing arithmetic, checks whether a contour vertex of $B_j$ lies
   strictly inside $A_i$'s outer contour (`point_in_ring == 1`) and strictly
   outside every hole of $A_i$ (`point_in_ring == -1`), or vice versa. Any hit on
   an edge or vertex returns `0` and is never accepted as a positive-area witness.
   Sampled stride (`(pts.len() / 8).max(1)`) runs first, followed by a full scan
   of remaining contour vertices.
3. **Exact Clipper2 fallback on the overlapping subset.** If `overlapping_pairs`
   is non-empty and no strict-interior contour vertex witness is found (for
   example, two bars crossing in a `+` where all 8 vertices lie outside the other
   polygon, or a footprint sitting inside a region's hole), calls
   `!slicer_core::polygon_ops::intersection_ex(&a_sub, &b_sub).is_empty()` on
   only the `ExPolygon`s participating in `overlapping_pairs`.

Four unit tests were added to `crates/slicer-sdk/tests/layer_module_tdd.rs` alongside
the six existing `derive_needs_support` tests:
- `slice_region_view_derive_needs_support_edge_and_vertex_touching_zero_area_is_false`
  (shared vertical edge, shared corner vertex, and concave L-shape reflex edge);
- `slice_region_view_derive_needs_support_footprint_inside_region_hole_is_false`
  (footprint strictly inside an annulus hole and touching the hole wall from
  inside);
- `slice_region_view_derive_needs_support_region_inside_footprint_hole_is_false`
  (region strictly inside a footprint's hole);
- `slice_region_view_derive_needs_support_crossing_bars_without_interior_vertices_is_true`
  (`+`-shaped crossing bars with zero interior vertices, exercising the
  `intersection_ex` fallback).

## 2. Output gate — byte-identical in 8/8 cells

`-Mode output` ran one baseline and one candidate slice per cell. Every run
exited `0` with `degraded=false`, `fatal_error_count=0`, `non_fatal_error_count=0`,
and `diagnostic_lines=0`. Both arms emit the active t151 reference hash in every
cell:

| cell | sha256 (both arms) | bytes | t151 reference |
| --- | --- | ---: | --- |
| benchy classic-off | `3d28d1b32f450af3…` | 4,085,138 | `3d28d1b3…` ✓ |
| benchy classic-on | `eb202973a1ece31c…` | 8,673,267 | `eb202973…` ✓ |
| benchy arachne-off | `39ad6289b681529e…` | 4,227,129 | `39ad6289…` ✓ |
| benchy arachne-on | `77c3c578aa28382c…` | 8,814,878 | `77c3c578…` ✓ |
| base classic-off | `9bb18f0e8173ec9c…` | 21,187,100 | `9bb18f0e…` ✓ |
| base classic-on | `eccbd5238d2dde1b…` | 39,732,305 | `eccbd523…` ✓ |
| base arachne-off | `2f249cdb8fd2c570…` | 21,046,764 | `2f249cdb…` ✓ |
| base arachne-on | `1811425ffccda686…` | 39,594,635 | `1811425f…` ✓ |

## 3. Paired eight-cell gate (5 interleaved pairs per cell)

The initial 3-pair interleaved pass cleared all four `base` cells by
**+7.52 % to +17.31 %** (`9.40x–21.63x` floor) and reduced CPU in **8/8** cells,
but three `benchy` cells (`benchy-classic-off` `+0.36 %`, `benchy-classic-on`
`−1.18 %` despite winning 2/3 pairs, and `benchy-arachne-on` `−0.04 %`) sat
inside `benchy`'s ~235 ms wall noise band. Following the standing protocol, **2
additional interleaved pairs were pre-declared for all 8 cells** before running
pairs 4–5, and all 5 pairs enter the median without discarding any pair.

### Measured table (5 interleaved pairs per cell, 80 runs)

| cell | pairs | baseline wall (s) | candidate wall (s) | wall saving (median of arms) | floor multiple | median of pairwise savings | baseline CPU (s) | candidate CPU (s) | CPU saving | pairs faster (wall / CPU) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 5 | 3.659 | 3.486 | **+4.715 %** | 5.89x | −0.066 % | 19.09 | 17.14 | **+10.23 %** (−1.95 s) | 2/5 / 5/5 |
| benchy classic-on | 5 | 7.628 | 7.570 | **+0.766 %** | **0.96x** | +2.690 % | 40.63 | 38.17 | **+6.04 %** (−2.45 s) | 4/5 / 4/5 |
| benchy arachne-off | 5 | 3.592 | 3.595 | **−0.076 %** | **−0.10x** | +3.158 % | 19.63 | 18.19 | **+7.32 %** (−1.44 s) | 3/5 / 5/5 |
| benchy arachne-on | 5 | 7.926 | 7.799 | **+1.599 %** | 2.00x | +1.599 % | 42.02 | 39.55 | **+5.88 %** (−2.47 s) | 3/5 / 5/5 |
| base classic-off | 5 | 28.583 | 24.334 | **+14.865 %** | 18.58x | +15.246 % | 208.72 | 164.09 | **+21.38 %** (−44.63 s) | 5/5 / 5/5 |
| base classic-on | 5 | 86.278 | 81.602 | **+5.420 %** | 6.78x | +8.667 % | 601.67 | 530.59 | **+11.81 %** (−71.08 s) | 5/5 / 5/5 |
| base arachne-off | 5 | 28.920 | 24.687 | **+14.637 %** | 18.30x | +13.655 % | 211.92 | 168.28 | **+20.59 %** (−43.64 s) | 5/5 / 5/5 |
| base arachne-on | 5 | 84.720 | 78.351 | **+7.518 %** | 9.40x | +6.911 % | 603.45 | 535.61 | **+11.24 %** (−67.84 s) | 5/5 / 5/5 |

### Price vs realised delta and why `benchy` fails the separate-median gate

1. **Realised savings match t157's price:**
   - On `base`, t157 priced the predicate at **4.69 s** wall-equivalent (`base
     classic-off`, ~53 CPU s across 12 logical cores) and **7.41 s** (`base
     classic-on`, ~82 CPU s). The 5-pair gate realises **−4.25 s** wall /
     **−44.63 CPU s** on `base-classic-off`, **−4.23 s** wall / **−43.64 CPU s**
     on `base-arachne-off`, **−4.68 s** wall / **−71.08 CPU s** on
     `base-classic-on`, and **−6.37 s** wall / **−67.84 CPU s** on
     `base-arachne-on`, winning **20 of 20** pairs across the four `base` cells.
   - On `benchy`, t157 priced the predicate at **230.6–237.3 ms** wall-equivalent
     (**2.71–2.81 s** summed across the 12 worker threads). The 5-pair gate
     realises **−1.44 to −2.47 CPU s** across all four `benchy` cells (winning
     **19 of 20** `benchy` pairs on CPU).
2. **Why two `benchy` cells miss the `0.80 %` separate-arm median floor:**
   - Because `benchy` has only 240 simple layers (`a_verts = 655`, `b_verts =
     2,299`), the entire three-site `derive_needs_support` cost on `benchy` is
     only ~235 ms wall-equivalent (~3.0–3.2 % of `benchy` supports-on wall).
   - Across the 5 interleaved pairs (roughly 40 minutes of wall time), drift
     between earlier pairs (pairs 1–3, where `benchy-classic-on` baseline was
     `7.251–7.396 s`) and later pairs (pairs 4–5, where `benchy-classic-on`
     candidate was `7.570–8.075 s` against a `7.628–8.542 s` baseline) shifts the
     **separate** 5-run arm medians (`median(baseline) = 7.628 s` from pair 5 vs
     `median(candidate) = 7.570 s` from pair 5, `+0.766 %`, 3.4 ms shy of `0.80 %`)
     even though within each pair the candidate beat baseline in **4/5 pairs**
     (`−1.18 %`, `+2.69 %`, `+5.57 %`, `+5.47 %`, `+0.77 %`; median pairwise
     saving **+2.690 %**).
   - Similarly, on `benchy-arachne-off`, the candidate beat baseline in **3/5
     pairs** (`+5.78 %`, `−2.84 %`, `+3.16 %`, `+11.15 %`, `−0.08 %`; median
     pairwise saving **+3.158 %**), but the separate arm medians (`3.592 s` vs
     `3.595 s`, `−0.076 %`) pick pair 5 (`3.592 s` vs `3.595 s`).
   - Under the standing gate (`verify.py`), a single-lever candidate cannot pass
     via pairwise medians or CPU corroboration when any cell's separate-arm wall
     median is below `+0.80 %`.

### Null band

Same-arm control (`baseline-ordinary` vs `baseline-ordinary`), three `base`
cells × three pairs (18 runs):

| cell | pairs | wall median delta | CPU median delta | wall max \|delta\| | CPU max \|delta\| |
| --- | ---: | ---: | ---: | ---: | ---: |
| base-classic-off | 3 | −4.410 % | −0.834 % | 7.303 % | 1.117 % |
| base-classic-on | 3 | −3.145 % | −0.934 % | 5.064 % | 0.968 % |
| base-arachne-off | 3 | +0.495 % | −0.301 % | 11.762 % | 0.926 % |
| **band (max across cells)** | **9 pairs** | **4.410 %** | **0.934 %** | **11.762 %** | **1.117 %** |

The CPU null band is tight (**1.12 %** max any-pair, **0.93 %** max median), and
every cell's candidate CPU reduction (**5.88 %–21.38 %**) sits **5.3x to 19.1x**
above the maximum single-pair CPU noise. The wall null band (**11.76 %** max
any-pair, **4.41 %** max median) reflects process-level wall variance; the two
supports-off `base` cells (**+14.87 %** and **+14.64 %**) clear even the
worst-case single-pair wall null band, while `benchy`'s ~235 ms true effect sits
well inside single-process wall jitter.

## 4. Disclosures

- **Why a standalone `derive_needs_support` lever is structurally ceiling-bound on
  `benchy` supports-on:** `derive_needs_support` accounts for **10.7–19.2 %** of
  total slice wall on `base` (`4.69–7.41 s`), where `overhang_regions[0].xy_footprint`
  has 110 islands / 5,921 vertices across 496 layers, but only **~3.0 %**
  (`~231 ms`) on `benchy` supports-on (`7.6–7.9 s` total wall). Any standalone
  optimisation of `derive_needs_support` has a physical ceiling of ~3 % on
  `benchy` supports-on, which is narrow enough that separate-arm medians across a
  40-minute 8-cell batch can land just below `+0.80 %` (`+0.766 %` on
  `benchy-classic-on`). Bundling the exact fast predicate with a complementary
  lever that also moves `benchy` (or eliminating the two dead per-layer
  re-derivations alongside a `benchy`-active prepass/layer lever) is the path to
  capturing the **44–71 CPU s / 4.2–6.4 s wall** `base` win without a `benchy`
  noise-floor drop.
- **Human KEEP, original gate failure disclosed.** The production change and tests
  in `crates/slicer-sdk` are landed under [`RULING.md`](RULING.md). The original
  mechanical DROP and the absence of a `benchy` same-arm null control remain
  disclosed; the active output reference set remains t151's because output is
  byte-identical.
- **No accelerated arm and no fresh Orca scoreboard.** The route ratios below are
  cross-window compositions from t154 and this campaign, not a new scoreboard of
  record. The destination remains 8/8 cells won against Orca.
- **Final landed-tree gates (2026-11-17):** all 72 `slicer-sdk`
  `layer_module_tdd` tests pass; `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo xtask build-guests --check`, `cargo xtask check-literals`,
  and `cargo xtask check-test-quality --report` exit `0`. Fresh release slices of
  `benchy-arachne-off` and `base-classic-off`, on both baseline and candidate arms,
  exit `0` with `degraded=false`, zero fatal/non-fatal errors, and their active
  reference hashes (`39ad6289…` and `9bb18f0e…`). Raw landing-check output is in
  gitignored `target/t158/output/keep-check/`.

## 5. Indicative route composition after KEEP

[`route_arithmetic.py`](route_arithmetic.py) applies this campaign's measured
separate-arm median wall deltas to t154's ledger. Ratios are cross-window
compositions, not a fresh Orca scoreboard. For the canonical-prepass projection,
the script subtracts the whole measured wall saving from t154's projected PnP
wall; it does not add t157's estimate a second time.

| cell | pre-t158 ratio | post-t158 ratio | required cut after | canonical prepass before → after |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | 2.0068x | **1.9122x** | 47.70 % | 0.7395x → **0.6494x** |
| benchy classic-on | 2.5711x | **2.5514x** | 60.81 % | 0.8573x → **0.8334x** |
| benchy arachne-off | 1.7628x | **1.7641x** | 43.31 % | 0.6685x → **0.6699x** |
| benchy arachne-on | 2.6912x | **2.6482x** | 62.24 % | 0.9008x → **0.8519x** |
| base classic-off | 1.8270x | **1.5554x** | 35.71 % | 0.7433x → **0.4266x** |
| base classic-on | 3.2197x | **3.0452x** | 67.16 % | 1.0508x → **0.8343x** |
| base arachne-off | 1.7981x | **1.5349x** | 34.85 % | 0.6975x → **0.4138x** |
| base arachne-on | 3.0603x | **2.8302x** | 64.67 % | 0.9667x → **0.7201x** |

All eight canonical-prepass projections are now below 1.0x. This is route
arithmetic, not a claim that the destination's measured 8/8 gate has closed. The
negative `benchy-arachne-off` wall delta is retained rather than replaced by its
positive pairwise median. Machine-readable results:
[`route_arithmetic.json`](route_arithmetic.json).

## 6. Reproduction

[`REPRODUCE.md`](REPRODUCE.md), [`run_t158.ps1`](run_t158.ps1),
[`reduce_t158.py`](reduce_t158.py), [`verify.py`](verify.py),
[`candidate.patch`](candidate.patch), [`RULING.md`](RULING.md), and
[`route_arithmetic.py`](route_arithmetic.py).

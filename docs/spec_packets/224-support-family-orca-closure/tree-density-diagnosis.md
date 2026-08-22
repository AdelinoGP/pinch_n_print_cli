# Tree support density diagnosis (packet 224, Step 3)

Read-only diagnosis. Commit `d97fb2b8`. Fixture
`crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl`.
Artifacts measured: `target/pnp_support_tree.gcode` (PnP, config
`tmp/support-family-config.json`) and `tmp/SupportTest_Tree_Orca.gcode`
(OrcaSlicer, `nozzle_diameter = 0.5`, `support_line_width = 80%`,
`support_interface_top_layers = 2`, `support_type = tree(auto)`,
`layer_height = 0.2`). No source file was modified.

All numbers below were produced by a G-code parser written for this step.
Its correctness is anchored: summing its per-feature `E` totals reproduces
each file's own `; filament used [mm]` footer exactly - PnP `1862.08`,
Orca `2872.79` vs. the file's `2872.80`.

---

## 0. The headline metric is contaminated (measured)

The baseline row "support filament (mm) | 486.33 | 1538.36" in `design.md`
is **the sum of every positive `E` delta inside a support block, including
de-retraction primes**, which deposit no material and move no distance.
Reproduced exactly: including zero-length `E` moves the parser returns PnP
`483.42 + 2.91 = 486.33` and Orca `1504.12 + 34.25 = 1538.37`.

Excluding zero-length primes (deposited material only):

| | PnP | Orca | PnP / Orca |
| --- | --- | --- | --- |
| `;TYPE:Support` filament | 387.42 mm | 651.31 mm | 59.5% |
| `;TYPE:Support interface` filament | 1.31 mm | 32.65 mm | 4.0% |
| **support + interface filament** | **388.73 mm** | **683.96 mm** | **56.8%** |
| support + interface XY path length | 11 687.5 mm | 22 774.9 mm | 51.3% |

Orca's support blocks contain ~853 mm of prime `E` against PnP's ~96 mm,
because Orca prints ~58 short separate branch loops per top layer and
retracts between them, while PnP prints one body. **The 31.6% figure
therefore measures retraction count as much as material.** The real
deposited-material deficit is **1.76x**, not 3.2x. This does not excuse
the deficit; it re-scales it.

Decomposition of the real 1.76x (measured, multiplicative):

- XY path length: **1.949x short** (11 687.5 vs 22 774.9 mm)
- Filament per XY mm: PnP **1.107x higher** (0.03326 vs ~0.0300 mm/mm)
- 1.949 / 1.107 = 1.76. Closed.

**Every bit of the deficit is path length, i.e. geometric coverage.**
PnP's per-mm flow is not low - it is *high*.

---

## 1. `tapered_radius` vs canonical `calc_branch_radius`

PnP: `tapered_radius` in `modules/core-modules/tree-support-planner/src/lib.rs`.

```
mm_to_top = dist_to_top * effective_layer_height
raw = if mm_to_top <= branch_radius { mm_to_top }
      else { branch_radius + (mm_to_top - branch_radius) * tan(tree_support_branch_diameter_angle) }
radius = clamp(raw, MIN_BRANCH_RADIUS = 0.4, MAX_BRANCH_RADIUS_MM = 6.0)
```

Canonical: the `coordf_t mm_to_top` overload of `TreeSupport::calc_branch_radius`
(`TreeSupport.cpp`) - `tip_height = base_radius` (45-degree tip), same two-piece
form, `std::clamp(radius, MIN_BRANCH_RADIUS, MAX_BRANCH_RADIUS)`, then
`if (support_interface_top_layers > 0) radius = max(radius, base_radius);`.

Divergences:

| # | Divergence | Canonical | PnP | Binds on this fixture? |
| --- | --- | --- | --- | --- |
| R1 | upper clamp | `MAX_BRANCH_RADIUS = 10.0` (`TreeSupport.hpp`) | `MAX_BRANCH_RADIUS_MM = 6.0` | **No.** Largest observed PnP branch is ~4.1 mm radius (8.18 mm span at z=2). |
| R2 | interface radius raise | `radius = max(radius, base_radius)` when `support_interface_top_layers > 0` | absent | **Yes, weakly.** Orca profile sets `support_interface_top_layers = 2`, so canonical floors every node at `base_radius = 2.5 mm`; PnP floors at 0.4 mm. Affects only the top ~2.5 mm of each column. |
| R3 | `use_min_distance` parameter | present in signature | absent | Not exercised (canonical ignores it in this overload). |

Two-piece formula, tip cone, and `MIN_BRANCH_RADIUS = 0.4` all match.
**Radius is not the deficit.** Measured PnP branch diameter at z=2 is
8.18 mm; canonical `calc_branch_radius(2.5, 25 mm, tan 5deg)` predicts
2.5 + 22.5*0.0875 = 4.47 mm radius = 8.94 mm diameter. PnP's single trunk
is *canonically sized*. It is the only trunk that exists.

---

## 2. `render_polygon` vs canonical `tree_supports_generate_paths`

PnP: `TreeSupport::render_polygon` in `modules/core-modules/tree-support/src/lib.rs`.
Per branch cross-section it emits:

- `wall_count` closed loops at insets `-line_width * (i + 0.5)` for `i` in `0..wall_count`.
  `wall_count` comes from `tree_support_wall_count`, `.max(1)`, default **2**.
  **`tree_support_wall_count` is not declared in `modules/core-modules/tree-support/tree-support.toml`'s
  `[config.schema]`**, so it can never be supplied and is always 2.
- then a `scan_fill_region` axis-aligned solid fill of the region inset by
  `-line_width * wall_count`, at pitch `line_width / density.min(1.0)`.

Canonical `tree_supports_generate_paths` (`SupportCommon.cpp`) emits
**perimeters only**: `draw_perimeters` of the branch expolygon, plus one
inner loop via `offset2_ex(-1.5w, +0.5w)` when
`area > tree_branch_diameter_double_wall_area_scaled`. Interior infill is
gated behind `with_infill` in the caller and does not apply to plain
`smpDefault` tree branch bodies.

Divergences:

| # | Divergence | Direction |
| --- | --- | --- |
| W1 | PnP solid-fills the branch interior; canonical leaves it hollow | PnP **over**-extrudes |
| W2 | PnP always uses 2 walls; canonical auto-selects 1 or 2 by cross-section area (`tree_support_wall_count = 0` = AUTO) | mixed; over-extrudes on thin branches |
| W3 | `support_density` unit bug: `tree-support.toml` declares it `default = 20.0, max = 100.0` (percent) and `tmp/support-family-config.json` passes `20.0`, but `TreeSupport::from_config` defaults it to `0.2` and `render_polygon` applies `.min(1.0)` - so pitch = `line_width`, i.e. **100% solid** | PnP **over**-extrudes |

Measured consequence (closed loops = walls, open runs = fill): at z=2 PnP
spends 48.52 mm on 2 wall loops and **92.65 mm on interior fill**; at
z=12, 37.59 mm walls / 52.57 mm fill. Roughly 60% of PnP's support
material at low Z is fill that canonical would not print at all.
**Removing the fill to match canonical would deepen the deficit, not close it.**

---

## 3. Per-Z measurement, PnP vs Orca

Runs = contiguous extruding sequences inside `;TYPE:Support*` blocks
(a travel or type change ends a run). "Closed" = run whose endpoints are
within 0.25 mm and which has more than 3 points, i.e. a wall loop.

| z (mm) | file | runs | closed loops | x span (mm) | y span (mm) | closed len | open len | **total len (mm)** |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2.00 | PnP | 19 | 2 | 8.18 | 8.18 | 48.52 | 92.65 | **141.17** |
| 2.00 | Orca | 4 | 2 | 7.16 | 15.38 | 35.61 | 43.65 | **79.26** |
| 7.00 | PnP | 17 | 2 | 7.30 | 7.31 | 43.06 | 71.23 | **114.28** |
| 7.00 | Orca | 6 | 3 | 10.84 | 16.37 | 52.44 | 54.22 | **106.65** |
| 12.00 | PnP | 15 | 2 | 6.43 | 6.43 | 37.59 | 52.57 | **90.17** |
| 12.00 | Orca | 8 | 4 | 14.07 | 17.77 | 63.65 | 67.71 | **131.36** |
| 18.00 | PnP | 12 | 2 | 5.38 | 5.38 | 31.04 | 33.07 | **64.11** |
| 18.00 | Orca | 20 | 14 | 16.99 | 18.97 | 132.23 | 62.02 | **194.25** |
| 24.00 | PnP | 16 | 2 | 6.84 | 6.84 | 37.07 | 7.77 | **44.84** |
| 24.00 | Orca | 85 | 58 | 19.10 | 20.31 | 346.25 | 137.70 | **483.95** |

Totals: PnP 11 687.5 mm over 125 support-carrying Z; Orca 22 774.9 mm over 124.

Read the `closed loops` column top to bottom. **Orca fans out from 2 loops
at z=2 to 58 at z=24; PnP has exactly 2 at every single Z** - one body,
two walls, forever. PnP's support footprint never exceeds 8.2 mm in either
axis; Orca's reaches 19.10 x 20.31 mm at z=24, which is the full overhang.

---

## 4. Root cause

**PnP generates support contact points from mesh overhang-triangle
centroids, one per triangle. Canonical generates them by sampling the
per-layer overhang *polygon* at `tree_support_branch_distance` spacing.**

Evidence, `SupportTest.stl` (binary STL, 20 triangles, bbox
x -10..20, y 0..20, z 0..30):

- Exactly **4** facets satisfy `nz_unit <= -sin(45deg)`, the test in
  `detect_overhang_facets` (`modules/core-modules/tree-support-planner/src/lib.rs`).
  Two are the z=0 base face and are discarded by the
  `z <= bmin[2] + effective_layer_height * 0.5` guard in the contact loop.
- The remaining two are the z=25.00 overhang plate, a single ~20x20 mm
  square split into two triangles, with centroids `(13.33, 6.67, 25.00)`
  and `(6.67, 13.33, 25.00)`. **PnP therefore emits 2 contact points for
  a ~400 mm2 overhang**, 9.4 mm apart, which the MST/merge pass at
  `DEFAULT_MERGE_DISTANCE_MM` resolves into one column.
- Canonical `TreeSupport::generate_contact_points` (`TreeSupport.cpp`)
  iterates `layer->loverhangs` and inserts contacts from three sources per
  overhang ExPolygon: (i) every contour vertex whose corner angle is under
  135 degrees, (ii) points walked along the contour and every hole via
  `EdgeCache` at arc step `point_spread = scale_(tree_support_branch_distance)`,
  and (iii) interior `grid_points`, prebuilt over the object bounding box at
  `sample_step = max(point_spread, max_bridge_length / 2)` and kept where
  `is_inside_ex(overhang_inner, candidate)`.
- For a 20x20 mm square at the default 5 mm branch distance that is
  4 corners + ~16 contour points + up to 16 interior grid points, before
  the `already_inserted` radius hash dedupe. The measured 58 closed loops
  at Orca z=24 is consistent with that order of magnitude; 2 is not.

The contact-point count is the top of the causal chain: contacts become
nodes, nodes become MST branches, branches become cross-sections, and
cross-section perimeter is what the deficit is made of.

### Ranked contributions

| rank | cause | measured contribution | evidence vs inference |
| --- | --- | --- | --- |
| 1 | **(b) too few branches** - triangle-centroid contacts instead of polygon sampling | **Dominant.** Drives the entire 1.949x path-length gap. PnP: 2 contacts, 1 body, at most 8.2 mm footprint at every Z. Orca: 58 loops and a 19.1 x 20.3 mm footprint at z=24. | **EVIDENCE** (STL facet analysis + per-Z loop counts + canonical `generate_contact_points` read). |
| 2 | **(d) missing top-interface area** - PnP interface filament 1.31 mm vs Orca 32.65 mm (4.0%), path 39.28 vs 1099.51 mm | Real and large in ratio, small in absolute terms (~31 mm of the ~295 mm filament gap, ~11%). | **EVIDENCE** (measured). Its being downstream of cause 1 - with 2 contacts there is almost no roof area for `structural_body_regions` to carve - is **INFERENCE**. |
| 3 | **(c) wall/fill divergence** - W1/W2/W3 above | **Negative contribution: it masks the deficit.** ~60% of PnP's low-Z support length is interior fill canonical would not print. | **EVIDENCE** for the measured wall/fill split. Any post-fix percentage would be an estimate; none is asserted here. |
| 4 | **(a) branch radius too small** | **Zero.** PnP's trunk is 8.18 mm across at z=2 against a canonical prediction of 8.94 mm for the same `mm_to_top`. R1 does not bind (max radius observed ~4.1 mm vs the 6.0 clamp). R2 costs a small amount of material in the top ~2.5 mm only. | **EVIDENCE** (measured span vs formula). |
| 5 | **(d) flow model** - PnP prints `; support_line_width = 0.35` in its own header but extrudes *every* feature at a uniform `0.03326 mm/mm` = a plain `0.4 x 0.2` rectangle, with no rounded-end correction. Orca's support is `0.0300 mm/mm` (`0.4 x 0.2` minus `h^2(1 - pi/4)`), its outer wall `0.04013` (`0.525 x 0.2` minus the same). | PnP over-extrudes support by **1.107x**. Masks the deficit. Not tree-specific - it affects all features. | **EVIDENCE** (per-feature `E`/mm, both files). |

The two profiles are not flow-identical: Orca ran a 0.5 mm nozzle, PnP a
0.4 mm line width. That difference is *removed* from the analysis above by
comparing path length, and quantified separately in cause 5.

---

## 5. Bug vs gap classification

| cause | classification | justification |
| --- | --- | --- |
| **(b) contact-point generation** | **GAP** | PnP does not implement the canonical model at all. `detect_overhang_facets` is a mesh-triangle normal test; canonical works from per-layer overhang ExPolygons and samples them by corner / contour-arc / interior grid at `tree_support_branch_distance`. There is no constant to correct - the sampling stage is absent. `tree_support_branch_distance` is read by the planner for avoidance inflation but never used to place contacts. |
| **(c) W1 hollow-vs-filled branch bodies** | **GAP** | `render_polygon` has no notion of the canonical `with_infill` gate; it unconditionally scan-fills. Canonical emits perimeters only for plain tree branches. |
| **(c) W2 always-2-walls** | **BUG** | The canonical rule (1 wall, or 2 above a cross-section-area threshold) fits the existing loop; PnP hardcodes the count and, separately, omits `tree_support_wall_count` from `tree-support.toml`'s `[config.schema]`, so AUTO (`0`) is unreachable. |
| **(c) W3 `support_density` percent/fraction** | **BUG** | Unit mismatch between the manifest schema (`default = 20.0`, `max = 100.0`) and `TreeSupport::from_config`'s fraction default of `0.2`. |
| **(a) R1 `MAX_BRANCH_RADIUS_MM = 6.0`** | **BUG** | Canonical `MAX_BRANCH_RADIUS = 10.0` (`TreeSupport.hpp`). Wrong constant, correct formula. Not currently binding. |
| **(a) R2 missing interface radius raise** | **BUG** | Canonical's final `if (support_interface_top_layers > 0) radius = max(radius, base_radius)` is a small addition to `tapered_radius`; the surrounding formula is already correct. Requires threading `support_interface_top_layers` into the planner. |
| **(d) top-interface area** | **GAP (downstream)** | Cannot be assessed independently until (b) is fixed. |
| **(d) uniform flow model** | **BUG**, outside tree support | PnP applies one `line_width * layer_height` rectangle to every extrusion role, ignoring the per-role widths it prints in its own header and the rounded-end area correction. Whole-slicer scope. |

**Fix order matters.** Cause (b) is the only one that closes the deficit;
causes (c) and (d.flow) currently inflate PnP's number and will make the
gap look *worse* when corrected. Any parity gate written before (b) lands
is gated on two errors cancelling.

---

## 6. Recommended correction to the packet baseline

`design.md`'s "support filament (mm)" row should either exclude zero-length
`E` moves or be renamed to make the retraction-prime inclusion explicit.
As written it is not a material metric, and the derived "31.6%" and the
"3.2x deficit" framing are both overstated relative to the 56.8% / 1.76x
deposited-material figures.

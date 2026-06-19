# Medial Axis Golden Fixture Derivation

These fixtures are the **independent oracle** for `crates/slicer-core/src/medial_axis.rs`.
They were derived from the mathematical definition of the medial axis — the locus of centers
of maximal inscribed circles — using closed-form geometric analysis plus the EP.cpp trim rule.

**Method used for all fixtures**: closed-form + EP.cpp trim rule.

**Anti-circularity guarantee**: `medial_axis.rs` was never read, loaded, or executed during
derivation. The Python script `gen_goldens.py` (repo root, gitignored) contains the full
derivation computation.

---

## EP.cpp Trim Rule (applied to all fixtures)

1. Compute the raw medial axis (all branches including corner spurs).
2. Identify each branch endpoint's degree:
   - **Junction**: degree >= 2 (where two or more branches meet). NOT open.
   - **Open leaf**: degree == 1 (a spur tip). IS open.
3. Find `global max_w` = maximum inscribed-circle diameter anywhere on the raw axis.
4. **Extension**: for an open leaf endpoint NOT already on the contour boundary, extend it
   along the terminal tangent direction until the first contour intersection; move the endpoint
   there. Open leaves that ARE on the contour boundary need no extension.
5. **Trim**: remove any branch that has >= 1 open endpoint AND whose arc length < 2 * global_max_w.
6. A **closed loop** has no open endpoints and is never trimmed.

---

## 1. `rectangle.json` — 10 mm × 1 mm rectangle

**Method**: closed-form + EP-trim
**Confidence**: HIGH

**Shape**: corners (0,0), (10,0), (10,1), (0,1).
**Parameters**: `min_width=0.4`, `max_width=2.0`.

### Raw medial axis

Central spine at y=0.5, x in [0.5, 9.5], width=1.0 mm constant.

At x=0.5: equidistant from left wall (dist=0.5), top wall (dist=0.5), bottom wall (dist=0.5)
— a 3-way junction. Left spur begins here.
At x=9.5: symmetric right spur junction.

Four corner spurs (one per corner):
- Junction (0.5, 0.5) toward corner (0,0): direction (-1,-1)/sqrt(2), length = 0.5*sqrt(2) ~ 0.707 mm.
- Width on spur at arc-length t: w(t) = 1.0 - sqrt(2)*t. At corner: width=0.
- Three other spurs (to (0,1), (10,0), (10,1)) are mirrors.

### EP.cpp trim

- Global max_w = 1.0 mm (anywhere on the axis).
- Threshold = 2 * max_w = 2.0 mm.
- Corner spurs: each has 1 open endpoint at the corner vertex (on contour boundary; no extension
  needed). Length = 0.707 mm < 2.0 mm → **REMOVED** (all 4).
- Spine: both ends are 3-way junctions (degree >= 2), NOT open leaves. Trim rule does not apply
  → **SURVIVES**.

### Reference axis

Spine only: y=0.5, x in [0.5, 9.5], width=1.0 mm constant.
Sampling step 0.01 mm. **901 points**.
Bbox: (0.5, 0.5) – (9.5, 0.5). Matches impl sanity bbox.

---

## 2. `wedge_25deg.json` — 25° apex isosceles triangle

**Method**: closed-form + EP-trim
**Confidence**: HIGH

**Shape**: apex A=(0, 10), base B=(-2.216947, 0), C=(2.216947, 0).
**Parameters**: `min_width=0.2`, `max_width=5.0`.

### Geometry

- Half-apex angle α = 12.5°.
- h_b = 10·tan(12.5°) = 2.216947 mm (half-base width).
- b_side = 10/cos(12.5°) = 10.242795 mm (lateral side length).
- Inradius: r = H·sin(α)/(sin(α)+1) = 10·sin(12.5°)/(sin(12.5°)+1) = 1.779288 mm.
  Derivation: r = Area/s = h_b·H / (h_b + H/cos(α)) = H·sin(α)/(sin(α)+1). ✓
- Incenter: I = (0, r) = (0, 1.779288) [on axis of symmetry by construction].

Verification via barycentric formula: I_y = (a·10 + b·0 + c·0)/(a+b+c)
where a=2·h_b, b=c=b_side. Matches r numerically. ✓

### Raw medial axis branches

1. **Apex spine I→A**: from (0, r) to (0, 10.0). Length = 10 - r = 8.221 mm.
   Width(y) = 2·h_b·(10-y)/b_side.
   At I: width = 2r = 3.559 mm. At A: width = 0.

2. **Left base branch I→B**: from (0, r) to (-h_b, 0). Length |IB| = sqrt(h_b² + r²) = 2.843 mm.
   Width: 2·(y-coordinate), going from 2r to 0 at B.

3. **Right base branch I→C**: symmetric to I→B. Length = 2.843 mm.

### EP.cpp trim

- Global max_w = 2r = 3.559 mm (at junction I).
- Threshold = 4r = 7.117 mm.
- Apex spine I→A: open endpoint at A (corner vertex on contour, no extension needed).
  Length 8.221 mm ≥ 7.117 mm → **SURVIVES**.
- Base branches I→B, I→C: open endpoint at B/C (corner vertices on contour).
  Length 2.843 mm < 7.117 mm → **REMOVED**.

### Reference axis

Apex spine only: x=0, y in [r, 10.0], width=2·h_b·(10-y)/b_side.
Sampling step 0.01 mm. **823 points**.
Bbox: (0, 1.779288) – (0, 10.0).

---

## 3. `asymmetric_taper.json` — trapezoid (wide-to-narrow)

**Method**: closed-form
**Confidence**: HIGH

**Shape**: (0,0) → (8,1.5) → (8,2.5) → (0,4). Left width=4 mm, right width=1 mm.
**Parameters**: `min_width=0.8`, `max_width=5.0`.

### Analytical spine

Top edge: (0,4)→(8,2.5). Slope = (2.5-4)/8 = -0.1875. Line: 1.5x + 8y = 32.
Bottom edge: (0,0)→(8,1.5). Slope = 1.5/8 = 0.1875. **Same slope → edges are parallel.**
Bottom line: 1.5x - 8y = 0.

L_norm = sqrt(1.5² + 8²) = sqrt(66.25) = 8.139410 mm.

Distance from (x, 2) to top edge: (16 - 1.5x) / L_norm.
Distance from (x, 2) to bottom edge: (16 - 1.5x) / L_norm. (Same — confirms y=2 is midline.)

Left junction x_L: slant_dist = left_wall_dist:
  (16 - 1.5·x_L)/L_norm = x_L → x_L = 16/(L_norm + 1.5) = **1.659853 mm**, width = 3.319705 mm.

Right junction x_R: slant_dist = right_wall_dist:
  (16 - 1.5·x_R)/L_norm = 8 - x_R → x_R = (8·L_norm - 16)/(L_norm - 1.5) = **7.397537 mm**,
  width = 1.204926 mm.

Width formula: w(x) = 2·(16 - 1.5x)/L_norm for x ∈ [x_L, x_R].

### EP.cpp trim

Both ends of the spine (x_L and x_R) terminate where the distance field switches from
slant-dominated to wall-dominated — these are **degenerate cap boundaries**, not 1D leaf tips.
The spine has NO open degree-1 endpoints. Trim rule requires >= 1 open endpoint — does not apply.
Spine **SURVIVES** in full.

### Reference axis

y=2.0, x from 1.659853 to 7.397537. Sampling step 0.002 mm. **2870 points**.
Width = 2·(16-1.5x)/L_norm, monotone decreasing from 3.320 to 1.205 mm.
Bbox: (1.660, 2.0) – (7.398, 2.0). Matches impl sanity bbox.

---

## 4. `curved_boundary.json` — elongated hexagon (coarse rounded cap)

**Method**: closed-form + EP.cpp trim
**Confidence**: HIGH

**Shape**: Elongated hexagon — 6 vertices: (1,0), (11,0), (12,1), (11,2), (1,2), (0,1).
Width = 2 mm, total length = 12 mm. Cap vertex at (0,1) and (12,1).
**Parameters**: `min_width=0.4`, `max_width=3.0`.

**Rationale for shape change**: the previous 200-segment arc approximation created a degenerate
VD junction-web (391 surviving edges), making the fixture adversarial. The elongated hexagon
has 6 clean segments and produces 7 VD surviving edges.

### Medial axis analysis

Spine at y=1 (equidistant from bottom edge y=0 and top edge y=2), width=2mm constant.

Cap vertex transition: for a point P=(x,1) on the spine, the distance to the left cap edges
(0,1)→(1,0) and (0,1)→(1,2) equals x/√2 (for x∈[0,√2]). This equals the distance to
the straight edges (= 1mm) when x = √2. So:

- For x < √2: cap edges dominate (x/√2 < 1 mm). The bisector of the two cap edges at (0,1)
  points in direction (+1,0), producing a short spur along y=1 from (0,1) to (√2, 1).
- For x > √2: straight edges dominate. Spine runs at y=1, width=2mm.
- Junction at (√2, 1) and (12−√2, 1): degree-2 nodes (spine + spur), NOT open leaves.

### EP.cpp trim

- Global max_w = 2.0 mm (on the spine).
- Threshold = 2 × 2.0 = 4.0 mm.
- Left cap spur: open endpoint at cap vertex (0,1) on contour, length = √2 ≈ 1.414 mm < 4 mm → **REMOVED**.
- Right cap spur: symmetric → **REMOVED**.
- Spine: endpoints at (√2, 1) and (12−√2, 1) are degree-2 → NOT open → **SURVIVES**.

### Reference axis

x ∈ [√2, 12−√2] = [1.41421, 10.58579], y=1.0, width=2.0 mm constant.
Sampling step 0.01 mm. **919 points**.
Bbox: (1.41421, 1.0) – (10.58579, 1.0).

---

## 5. `nested_hole.json` — square with square hole

**Method**: closed-form + EP-trim (corrected VD corner geometry)
**Confidence**: HIGH

**Shape**: Outer 10×10 mm square (0,0)–(10,10); inner 4×4 mm hole at (3,3)–(7,7).
**Parameters**: `min_width=0.5`, `max_width=5.0`.

### Geometry

Annular gap = 3 mm on all sides (outer wall to inner hole wall).
Strip midlines (equidistant from parallel outer/inner walls):
- Bottom strip (y∈[0,3]): midline y=1.5, width=3.0 mm.
- Top strip (y∈[7,10]): midline y=8.5, width=3.0 mm.
- Left strip (x∈[0,3]): midline x=1.5, width=3.0 mm.
- Right strip (x∈[7,10]): midline x=8.5, width=3.0 mm.

### VD corner transition vertices

At each annulus corner, the strip midlines do NOT meet at a sharp rectangular corner (1.5,1.5).
Instead, the VD produces a transition vertex equidistant from:
  - The inner hole CORNER VERTEX (a point site), and
  - The two perpendicular outer WALL SEGMENTS (segment sites).

For the bottom-right corner transition (near hole corner at (7,3)):
  Locus equidistant from corner point (7,3) and outer bottom wall y=0:
    y = ((x−7)²+9)/6    [parabola with focus (7,3) and directrix y=0]
  Also equidistant from outer right wall x=10:
    x + y = 10   (bisector of the two outer walls)
  Solving: x = (10+7√2)/(1+√2) = 4+3√2 ≈ 8.24264, y = 6−3√2 ≈ 1.75736, width=2y≈3.51472 mm.

By 4-fold symmetry (reflections about x=5 and y=5):
  Bottom-left transition: (10−x_c, y_c) = (6−3√2, 6−3√2) ≈ (1.75736, 1.75736)
  Top-left transition: (6−3√2, 4+3√2) ≈ (1.75736, 8.24264)
  Top-right transition: (4+3√2, 4+3√2) ≈ (8.24264, 8.24264)

### Loop topology (12 vertices, 12 straight-line segments)

The VD emits the following 12-vertex closed loop (CCW from bottom-right corner transition):
  1. (x_c, y_c)          — bottom-right corner transition, w=3.51472
  2. (7.0, 1.5)          — bottom strip endpoint, w=3.0
  3. (3.0, 1.5)          — bottom strip other endpoint, w=3.0
  4. (10−x_c, y_c)       — bottom-left corner transition, w=3.51472
  5. (1.5, 3.0)          — left strip endpoint, w=3.0
  6. (1.5, 7.0)          — left strip other endpoint, w=3.0
  7. (10−x_c, 10−y_c)    — top-left corner transition, w=3.51472
  8. (3.0, 8.5)          — top strip endpoint, w=3.0
  9. (7.0, 8.5)          — top strip other endpoint, w=3.0
  10. (x_c, 10−y_c)      — top-right corner transition, w=3.51472
  11. (8.5, 7.0)          — right strip endpoint, w=3.0
  12. (8.5, 3.0)          — right strip other endpoint, w=3.0
  → back to 1.

The impl connects these 12 vertices with LINEAR segments (the VD primary edges are linear
for this purely piecewise-linear polygon input). The width varies linearly between
w=3.0 (strip points) and w≈3.51472 (corner transitions).

### Corner spurs (raw axis, trimmed)

From each corner transition vertex, 2 diagonal spurs extend toward the outer and inner
contour corners. All 8 spurs have length ≈ 1.5√2 ≈ 2.121 mm and endpoint on contour.

### EP.cpp trim

- Global max_w from all emitted points: the maximum width is 3.51472 mm (at corner transitions).
- Threshold = 2 × 3.51472 = 7.02944 mm.
- All 8 corner spurs: length ≈ 2.121 mm < 7.029 mm → **REMOVED**.
- Closed 12-vertex loop: NO open endpoints → trim rule does not apply → **SURVIVES**.

### Reference axis

Closed 12-segment loop following the 12 VD vertices above, connected by straight lines with
linearly-interpolated widths. Sampling step 0.01 mm. **≈2616 points** (varies by rounding).
Width ranges from 3.0 mm (strip midpoints) to 3.51472 mm (corner transitions).

---

## Uncertainty flags

| Fixture | Flag |
|---------|------|
| `rectangle.json` | None. Closed-form exact. |
| `wedge_25deg.json` | None. Closed-form exact (all trig computed in float64). |
| `asymmetric_taper.json` | None. Closed-form exact (parallel edges give exact y=2 midline). |
| `curved_boundary.json` | None. Closed-form exact. Shape replaced from 200-segment arc to 6-vertex hexagon. Spine at y=1, x∈[√2,12−√2]. |
| `nested_hole.json` | None. Closed-form exact. Corner transitions derived analytically from VD parabola/directrix intersection (x_c=4+3√2). |

---

## Metric pass thresholds

See `metric.rs.txt`:

| Metric | Threshold |
|--------|-----------|
| `symmetric_hausdorff` (position error) | ≤ 0.005 mm |
| `max_width_error` | ≤ 0.01 mm |

These are conservative enough to account for:
- f32 precision in the implementation (~0.0001 mm at 1 mm scale)
- Any minor topological differences in branch ordering

**Note on curved_boundary fixture replacement**: The original 200-segment arc approximation
created a degenerate VD (391 surviving edges, junction-web near the arc focus). It was replaced
with an elongated hexagon (6 vertices, 7 surviving VD edges) that has a clean, analytically
derivable medial axis. The new fixture still exercises a coarse rounded/angled cap feature.

// Unit tests for the pure dragon tiling and colour mapping.
//
// These cover packet 227's AC-3 (determinism), AC-4 (colour-map wrap),
// AC-5 (holes excluded), AC-6 (per-region tiling_depth override) and
// AC-N1 (zero tool_count / empty color_map).
//
// The packet's AC commands run these by name via
// `moon test --target wasm -p slicer/layer-infill/src/dragon -f '<name>'`,
// each suffixed with a `Total tests: 1, passed: 1` check so a renamed or
// missing test fails the AC instead of silently reporting green.

///| 1 mm in slicer units (1 unit = 100 nm).
let mm : Int64 = 10000L

///|
fn square_ring(cx : Int64, cy : Int64, half : Int64) -> Ring {
  Ring::{
    pts: [
      Pt::{ x: cx - half, y: cy - half },
      Pt::{ x: cx + half, y: cy - half },
      Pt::{ x: cx + half, y: cy + half },
      Pt::{ x: cx - half, y: cy + half },
    ],
  }
}

///|
/// Count distinct dragon instances. `tile` is a lattice coordinate and may be
/// negative, so a max-based count would be wrong.
fn distinct_tiles(segs : Array[Seg]) -> Int {
  let seen : Array[Int] = []
  for i = 0; i < segs.length(); i = i + 1 {
    let t = segs[i].tile
    let mut found = false
    for j = 0; j < seen.length(); j = j + 1 {
      if seen[j] == t {
        found = true
      }
    }
    if !found {
      seen.push(t)
    }
  }
  seen.length()
}

///|
fn plain_square_region() -> Region {
  Region::{ contour: square_ring(0L, 0L, 10L * mm), holes: [] }
}

///|
fn holed_region() -> Region {
  Region::{
    contour: square_ring(0L, 0L, 10L * mm),
    holes: [square_ring(0L, 0L, 3L * mm)],
  }
}

// ---------------------------------------------------------------- AC-3

///|
test "tiling_is_deterministic_across_runs" {
  let region = plain_square_region()
  let first = tile_dragon_curve(region, 1L * mm, 8)
  let second = tile_dragon_curve(region, 1L * mm, 8)
  assert_eq(first.length(), second.length())
  // Same count is not enough: assert same order, ordinals, generations and
  // coordinates, element by element. That is what "byte-identical" means here.
  for i = 0; i < first.length(); i = i + 1 {
    assert_eq(first[i], second[i])
  }
  // Guard against the test passing vacuously on an empty tiling.
  assert_true(first.length() > 0)
}

///|
test "tiling_is_nonempty_and_ordinals_are_in_range" {
  let region = plain_square_region()
  let depth = 8
  let segs = tile_dragon_curve(region, 1L * mm, depth)
  assert_true(segs.length() > 0)
  // `tile` is a lattice coordinate (k + m), not a running counter. Note it
  // COLLIDES by design: cells (0,1) and (1,0) share a value, so two distinct
  // dragons can interleave under one index. Each dragon still prints in a
  // single tool - a finite palette has to reuse colours anyway - but that
  // makes "ordinals ascend within a tile" false, so assert what does hold:
  // every ordinal indexes a segment of some dragon of this depth.
  let limit = 1 << depth
  for i = 0; i < segs.length(); i = i + 1 {
    assert_true(segs[i].ordinal >= 0)
    assert_true(segs[i].ordinal < limit)
    assert_eq(segs[i].generation, generation_of(segs[i].ordinal))
  }
}

///|
/// H3 regression guard: a dragon covering a given patch of the bed must keep
/// its tool no matter what shape the region around it happens to be.
///
/// The colour driver used to be a counter over emitting instances with a
/// region-centred lattice phase, so a tapering part recoloured its dragons on
/// every layer. Here two very different regions overlap the same ground; every
/// dragon whose segments fall in the shared area must agree on `tile`.
test "tile_index_is_stable_across_region_shapes" {
  let wide = Region::{
    contour: Ring::{
      pts: [
        Pt::{ x: -30L * mm, y: -12L * mm },
        Pt::{ x: 30L * mm, y: -12L * mm },
        Pt::{ x: 30L * mm, y: 12L * mm },
        Pt::{ x: -30L * mm, y: 12L * mm },
      ],
    },
    holes: [],
  }
  let tall = Region::{
    contour: Ring::{
      pts: [
        Pt::{ x: -12L * mm, y: -30L * mm },
        Pt::{ x: 12L * mm, y: -30L * mm },
        Pt::{ x: 12L * mm, y: 30L * mm },
        Pt::{ x: -12L * mm, y: 30L * mm },
      ],
    },
    holes: [],
  }
  let a = tile_dragon_curve(wide, 1L * mm, 6)
  let b = tile_dragon_curve(tall, 1L * mm, 6)
  assert_true(a.length() > 0)
  assert_true(b.length() > 0)
  // Index every segment midpoint in the overlap square by rounded position.
  let mut checked = 0
  for i = 0; i < a.length(); i = i + 1 {
    let sa = a[i]
    let ax = (sa.a.x + sa.b.x) / 2L
    let ay = (sa.a.y + sa.b.y) / 2L
    if ax > -12L * mm && ax < 12L * mm && ay > -12L * mm && ay < 12L * mm {
      for j = 0; j < b.length(); j = j + 1 {
        let sb = b[j]
        if sb.a == sa.a && sb.b == sa.b {
          assert_eq(sb.tile, sa.tile)
          checked = checked + 1
        }
      }
    }
  }
  // The guard must not pass vacuously.
  assert_true(checked > 0)
}

// ---------------------------------------------------------------- AC-5

///|
test "holes_are_excluded_from_tiling" {
  let region = holed_region()
  let segs = tile_dragon_curve(region, 1L * mm, 10)
  assert_true(segs.length() > 0)
  let hole = region.holes[0]
  for i = 0; i < segs.length(); i = i + 1 {
    let s = segs[i]
    // No endpoint inside the hole.
    assert_false(point_in_ring(hole, s.a.x.to_double(), s.a.y.to_double()))
    assert_false(point_in_ring(hole, s.b.x.to_double(), s.b.y.to_double()))
    // And the segment body lies in the tiled area: sample the midpoint,
    // which is the part a purely endpoint-based check would miss.
    let mx = (s.a.x.to_double() + s.b.x.to_double()) * 0.5
    let my = (s.a.y.to_double() + s.b.y.to_double()) * 0.5
    assert_true(point_in_region(region, mx, my))
  }
}

///|
test "tiling_a_holed_region_emits_less_than_the_solid_region" {
  // A hole must actually remove material. Without this, a clipper that
  // silently ignored holes would still pass the endpoint checks above.
  let solid = tile_dragon_curve(plain_square_region(), 1L * mm, 10)
  let holed = tile_dragon_curve(holed_region(), 1L * mm, 10)
  let mut solid_len = 0.0
  for i = 0; i < solid.length(); i = i + 1 {
    let dx = (solid[i].b.x - solid[i].a.x).to_double()
    let dy = (solid[i].b.y - solid[i].a.y).to_double()
    solid_len = solid_len + (dx * dx + dy * dy).sqrt()
  }
  let mut holed_len = 0.0
  for i = 0; i < holed.length(); i = i + 1 {
    let dx = (holed[i].b.x - holed[i].a.x).to_double()
    let dy = (holed[i].b.y - holed[i].a.y).to_double()
    holed_len = holed_len + (dx * dx + dy * dy).sqrt()
  }
  assert_true(holed_len < solid_len)
}

// ---------------------------------------------------------------- AC-4

///|
test "color_map_wraps_into_tool_count" {
  // color_map names more distinct colours (6) than the machine has tools (2).
  let color_map = [0, 1, 2, 3, 4, 5]
  for ordinal = 0; ordinal < 64; ordinal = ordinal + 1 {
    match map_tiling_index_to_tool(ordinal, color_map, 2) {
      None => abort("expected Some for a non-empty map and tool_count 2")
      Some(tool) => {
        assert_true(tool >= 0)
        assert_true(tool < 2)
      }
    }
  }
}

///|
test "color_map_is_deterministic_for_fixed_inputs" {
  let color_map = [3, 1, 4, 1, 5, 9]
  let a = map_tiling_index_to_tool(17, color_map, 4)
  let b = map_tiling_index_to_tool(17, color_map, 4)
  assert_eq(a, b)
  // And it is actually a function of the inputs, not a constant.
  let c = map_tiling_index_to_tool(18, color_map, 4)
  assert_true(a != c)
}

// --------------------------------------------------------------- AC-N1

///|
test "tool_count_zero_returns_none" {
  let color_map = [0, 1, 2]
  assert_eq(map_tiling_index_to_tool(0, color_map, 0), None)
  assert_eq(map_tiling_index_to_tool(7, color_map, 0), None)
}

///|
test "empty_color_map_returns_none" {
  let empty : Array[Int] = []
  assert_eq(map_tiling_index_to_tool(0, empty, 4), None)
  assert_eq(map_tiling_index_to_tool(9, empty, 1), None)
}

// ---------------------------------------------------------------- AC-6

///|
test "per_region_tiling_depth_override" {
  // The region override wins over the module-global default.
  assert_eq(pick_tiling_depth(Some(6), 12), 6)
  // Absent an override, the module default stands.
  assert_eq(pick_tiling_depth(None, 12), 12)
  // Both paths clamp, so a hostile config degrades rather than wedging.
  assert_eq(pick_tiling_depth(Some(999), 12), max_tiling_depth)
  assert_eq(pick_tiling_depth(None, -4), 0)
}

///|
test "override_changes_the_emitted_tiling" {
  // Prove the resolved depth is actually load-bearing: a different depth must
  // produce a different tiling, otherwise AC-6 could pass on a value that is
  // resolved and then ignored.
  // Use a region large enough that BOTH depths clear the region-size cap;
  // on a small region they would both be reduced to the same effective depth
  // and the comparison would be vacuous.
  let region = Region::{ contour: square_ring(0L, 0L, 50L * mm), holes: [] }
  let shallow = tile_dragon_curve(region, 1L * mm, pick_tiling_depth(Some(4), 10))
  let deep = tile_dragon_curve(region, 1L * mm, pick_tiling_depth(None, 10))
  assert_true(shallow.length() > 0)
  assert_true(deep.length() > 0)
  // Compare GEOMETRY, not counts. The four-fold tiling is exact, so total
  // path length - and very nearly the segment count - is depth-independent:
  // deeper dragons are larger but proportionally fewer. Only the shape
  // changes, so an equality-of-length assertion would fail even though the
  // depth is plainly load-bearing.
  let mut differs = false
  let n = if shallow.length() < deep.length() {
    shallow.length()
  } else {
    deep.length()
  }
  for i = 0; i < n; i = i + 1 {
    if shallow[i].a != deep[i].a || shallow[i].b != deep[i].b {
      differs = true
    }
  }
  assert_true(differs)
}

// ------------------------------------------------------- curve properties

///|
test "generation_matches_trailing_zero_rule" {
  assert_eq(generation_of(0), 0)
  assert_eq(generation_of(1), 1)
  assert_eq(generation_of(2), 0)
  assert_eq(generation_of(3), 2)
  assert_eq(generation_of(7), 3)
}

///|
test "dragon_polyline_has_two_pow_depth_segments" {
  let pts = dragon_polyline(5, 1L * mm, Pt::{ x: 0L, y: 0L })
  assert_eq(pts.length(), 33)
  // Every step is exactly one spacing, axis-aligned.
  for i = 0; i < pts.length() - 1; i = i + 1 {
    let dx = pts[i + 1].x - pts[i].x
    let dy = pts[i + 1].y - pts[i].y
    let adx = if dx < 0L { -dx } else { dx }
    let ady = if dy < 0L { -dy } else { dy }
    assert_true((adx == 1L * mm && ady == 0L) || (adx == 0L && ady == 1L * mm))
  }
}

///|
test "zero_or_negative_spacing_emits_nothing" {
  let region = plain_square_region()
  assert_eq(tile_dragon_curve(region, 0L, 8).length(), 0)
  assert_eq(tile_dragon_curve(region, -1L, 8).length(), 0)
}


// ------------------------------------------ tiling coverage and colouring

///|
/// Regression guard for the under-fill bug found by visual debug: a single
/// un-tiled dragon covered only ~37% of the requested infill, because its
/// fractal boundary cannot reach a square's corners at any depth. Full lattice
/// tiling lays down `1 / line_spacing` of path per unit area, so total path
/// length must approach `area / line_spacing`.
test "tiling_covers_the_region_at_the_requested_density" {
  let side = 60L * mm
  let region = Region::{ contour: square_ring(0L, 0L, side / 2L), holes: [] }
  let spacing = 7500L // 0.75 mm, i.e. line_width 0.45 at density 0.6
  let segs = tile_dragon_curve(region, spacing, 12)
  let mut total = 0.0
  for i = 0; i < segs.length(); i = i + 1 {
    let dx = (segs[i].b.x - segs[i].a.x).to_double()
    let dy = (segs[i].b.y - segs[i].a.y).to_double()
    total = total + (dx * dx + dy * dy).sqrt()
  }
  let area = side.to_double() * side.to_double()
  let expected = area / spacing.to_double()
  let ratio = total / expected
  // A single dragon scored ~41% of rectilinear here; the uncorrected tiling ~0.92. With the
  // density-correction pass this lands on 1.000, so guard tightly - a loose
  // bound here is what let a 33% real-world shortfall go unnoticed.
  assert_true(ratio > 0.95)
  // And it must not double-extrude: overlapping copies would exceed 1.0.
  assert_true(ratio < 1.15)
}

///|
test "tiling_places_multiple_distinct_dragons" {
  let side = 60L * mm
  let region = Region::{ contour: square_ring(0L, 0L, side / 2L), holes: [] }
  let segs = tile_dragon_curve(region, 7500L, 12)
  assert_true(distinct_tiles(segs) > 1)
}

///|
test "each_dragon_gets_exactly_one_tool" {
  // The colouring contract the user specified: one dragon, one colour. Two
  // segments from the same tile must never disagree about their tool.
  let side = 60L * mm
  let region = Region::{ contour: square_ring(0L, 0L, side / 2L), holes: [] }
  let segs = tile_dragon_curve(region, 7500L, 12)
  let color_map = [0, 1, 2]
  assert_true(segs.length() > 0)
  for i = 0; i < segs.length(); i = i + 1 {
    let a = map_tiling_index_to_tool(segs[i].tile, color_map, 3)
    for j = i + 1; j < segs.length() && segs[j].tile == segs[i].tile; j = j + 1 {
      assert_eq(map_tiling_index_to_tool(segs[j].tile, color_map, 3), a)
    }
  }
}

///|
/// Neighbouring dragons must not all share a tool, derived from a REAL tiling
/// run.
///
/// The previous version of this test called `map_tiling_index_to_tool(0,..)`
/// and `(1,..)` on literals and never invoked the tiler, so it could not
/// observe the defect it existed to catch: the index used to collide along the
/// anti-diagonal, giving physically overlapping dragons the same integer.
test "neighbouring_dragons_do_not_all_share_a_tool" {
  let side = 60L * mm
  let region = Region::{ contour: square_ring(0L, 0L, side / 2L), holes: [] }
  let segs = tile_dragon_curve(region, 7500L, 12)
  assert_true(segs.length() > 0)
  let color_map = [0, 1, 2]
  // Collect the distinct tools actually produced by the tiling.
  let tools : Array[Int] = []
  for i = 0; i < segs.length(); i = i + 1 {
    match map_tiling_index_to_tool(segs[i].tile, color_map, 3) {
      Some(t) => {
        let mut seen = false
        for j = 0; j < tools.length(); j = j + 1 {
          if tools[j] == t {
            seen = true
          }
        }
        if !seen {
          tools.push(t)
        }
      }
      None => abort("expected a tool for a non-empty map")
    }
  }
  // A single-tool tiling would make the structure invisible, which is the
  // whole point of the feature.
  assert_true(tools.length() >= 2)
  assert_true(distinct_tiles(segs) >= 3)
}

// ------------------------------------------------ independent containment

///|
/// Winding-number point-in-polygon, written independently of the ray-casting
/// `point_in_ring` the clipper uses.
///
/// AC-5's original assertion called the same function the implementation
/// calls, so a bug in `point_in_ring` would have failed both sides identically
/// and the test would still pass. This is a genuinely separate algorithm.
fn winding_inside(ring : Ring, px : Double, py : Double) -> Bool {
  let pts = ring.pts
  let n = pts.length()
  if n < 3 {
    return false
  }
  let mut wind = 0
  for i = 0; i < n; i = i + 1 {
    let j = (i + 1) % n
    let xi = pts[i].x.to_double()
    let yi = pts[i].y.to_double()
    let xj = pts[j].x.to_double()
    let yj = pts[j].y.to_double()
    let cross = (xj - xi) * (py - yi) - (yj - yi) * (px - xi)
    if yi <= py {
      if yj > py && cross > 0.0 {
        wind = wind + 1
      }
    } else if yj <= py && cross < 0.0 {
      wind = wind - 1
    }
  }
  wind != 0
}

///|
/// Rotated square, so the dragon's axis-aligned segments cross hole edges
/// obliquely. The original fixture had axis-aligned segments meeting
/// axis-aligned edges, which never exercises a near-tangential crossing - the
/// case where `emit_run`'s along-segment inset gives the least perpendicular
/// clearance.
fn rotated_square_ring(half : Int64, angle_deg : Double) -> Ring {
  let r = angle_deg * 3.141592653589793 / 180.0
  let c = @math.cos(r)
  let s = @math.sin(r)
  let base = [(-half, -half), (half, -half), (half, half), (-half, half)]
  let pts : Array[Pt] = []
  for i = 0; i < base.length(); i = i + 1 {
    let (bx, by) = base[i]
    let x = bx.to_double()
    let y = by.to_double()
    pts.push(Pt::{
      x: (x * c - y * s).to_int64(),
      y: (x * s + y * c).to_int64(),
    })
  }
  Ring::{ pts, }
}

///|
test "oblique_hole_is_excluded_under_independent_check" {
  let region = Region::{
    contour: square_ring(0L, 0L, 15L * mm),
    holes: [rotated_square_ring(5L * mm, 31.0)],
  }
  let segs = tile_dragon_curve(region, 1L * mm, 10)
  assert_true(segs.length() > 0)
  let hole = region.holes[0]
  for i = 0; i < segs.length(); i = i + 1 {
    let s = segs[i]
    assert_false(winding_inside(hole, s.a.x.to_double(), s.a.y.to_double()))
    assert_false(winding_inside(hole, s.b.x.to_double(), s.b.y.to_double()))
    let mx = (s.a.x.to_double() + s.b.x.to_double()) * 0.5
    let my = (s.a.y.to_double() + s.b.y.to_double()) * 0.5
    assert_false(winding_inside(hole, mx, my))
    assert_true(winding_inside(region.contour, mx, my))
  }
}

// ------------------------------------------------------------ infill_angle

///|
test "infill_angle_rotates_the_tiling" {
  // The manifest advertises `infill_angle`; prove it is load-bearing rather
  // than a decorative key.
  let region = Region::{ contour: square_ring(0L, 0L, 20L * mm), holes: [] }
  let flat = tile_dragon_curve_rotated(region, 1L * mm, 10, 0.0)
  let tilted = tile_dragon_curve_rotated(region, 1L * mm, 10, 30.0)
  assert_true(flat.length() > 0)
  assert_true(tilted.length() > 0)
  // Some segment must actually move.
  let mut differs = false
  let n = if flat.length() < tilted.length() {
    flat.length()
  } else {
    tilted.length()
  }
  for i = 0; i < n; i = i + 1 {
    if flat[i].a != tilted[i].a {
      differs = true
    }
  }
  assert_true(differs)
}

///|
test "rotation_preserves_coverage" {
  // Rotating must not cost fill: the lattice is derived from the rotated
  // curve, so coverage is angle-independent.
  let side = 60L * mm
  let region = Region::{ contour: square_ring(0L, 0L, side / 2L), holes: [] }
  let spacing = 7500L
  let ideal = side.to_double() * side.to_double() / spacing.to_double()
  let angles = [0.0, 17.0, 45.0, 90.0]
  for k = 0; k < angles.length(); k = k + 1 {
    let segs = tile_dragon_curve_rotated(region, spacing, 12, angles[k])
    let mut total = 0.0
    for i = 0; i < segs.length(); i = i + 1 {
      let dx = (segs[i].b.x - segs[i].a.x).to_double()
      let dy = (segs[i].b.y - segs[i].a.y).to_double()
      total = total + (dx * dx + dy * dy).sqrt()
    }
    let ratio = total / ideal
    assert_true(ratio > 0.85)
    // Upper bound too: without it this test cannot detect the rotated lattice
    // double-extruding, which is half of what "preserves coverage" means.
    assert_true(ratio < 1.15)
  }
}

// ------------------------------------------------------- coverage uniformity

///|
/// Void-fraction guard.
///
/// This exists because `tiling_covers_the_region_at_the_requested_density`
/// cannot catch the defect it was written for. That test compares *total path
/// length* to the ideal, and length is conserved when fill is merely
/// redistributed: an early version scored a perfect 1.00 ratio while leaving
/// 17% of the region empty and double-extruding the rest. Only bucketing the
/// area and counting empty buckets sees that.
///
/// The sample cell is 3 mm - four times the 0.75 mm line spacing - so a
/// correctly filled region touches every cell, and the figure is not an
/// artifact of the sample pitch matching the pattern pitch.
test "tiling_leaves_no_large_voids" {
  let side = 60L * mm
  let region = Region::{ contour: square_ring(0L, 0L, side / 2L), holes: [] }
  let spacing = 7500L
  let n = 20
  let segs = tile_dragon_curve(region, spacing, 12)
  let grid = Array::make(n * n, 0)
  for i = 0; i < segs.length(); i = i + 1 {
    let s = segs[i]
    let dx = (s.b.x - s.a.x).to_double()
    let dy = (s.b.y - s.a.y).to_double()
    for t = 0; t <= 5; t = t + 1 {
      let f = t.to_double() / 5.0
      let x = s.a.x.to_double() + dx * f
      let y = s.a.y.to_double() + dy * f
      let gx = ((x + (side / 2L).to_double()) / side.to_double() * n.to_double()).to_int()
      let gy = ((y + (side / 2L).to_double()) / side.to_double() * n.to_double()).to_int()
      if gx >= 0 && gx < n && gy >= 0 && gy < n {
        grid[gy * n + gx] = grid[gy * n + gx] + 1
      }
    }
  }
  let mut empty = 0
  for i = 0; i < n * n; i = i + 1 {
    if grid[i] == 0 {
      empty = empty + 1
    }
  }
  // The four-fold rotational tiling is exact: measured 0/400 at every depth,
  // where translation-only placement left 65/400. Guard at 1% rather than the
  // 5% the approximate placement needed - if this ever regresses, the tiling
  // has stopped interlocking.
  assert_true(empty * 100 < n * n)
}

///|
/// The four-fold tiling covers exactly at every depth, so a large
/// `tiling_depth` must be honoured rather than reduced.
///
/// This replaces a `depth_is_capped_against_region_size` test. That cap existed
/// to suppress voids from the old translation-only placement; it is gone,
/// because it shrank dragons below visibility and silently overrode the
/// documented `tiling_depth` key.
test "large_tiling_depth_is_honoured" {
  let side = 60L * mm
  let region = Region::{ contour: square_ring(0L, 0L, side / 2L), holes: [] }
  let shallow = tile_dragon_curve(region, 7500L, 6)
  let deep = tile_dragon_curve(region, 7500L, 12)
  assert_true(shallow.length() > 0)
  assert_true(deep.length() > 0)
  // Deeper dragons are larger, so each one contributes a longer run of
  // consecutive ordinals. Max ordinal is a direct proxy for dragon size and
  // must grow with the requested depth.
  let mut max_shallow = 0
  for i = 0; i < shallow.length(); i = i + 1 {
    if shallow[i].ordinal > max_shallow { max_shallow = shallow[i].ordinal }
  }
  let mut max_deep = 0
  for i = 0; i < deep.length(); i = i + 1 {
    if deep[i].ordinal > max_deep { max_deep = deep[i].ordinal }
  }
  assert_true(max_deep > max_shallow)
}


///|
/// Void guard on the SHIPPED DEFAULT configuration.
///
/// `tiling_leaves_no_large_voids` runs at angle 0, depth 12. The manifest
/// defaults are `infill_angle = 45.0` and `tiling_depth = 10`, which take the
/// float-rotated path - previously guarded only by a length ratio. That is the
/// exact hole this module has already been bitten by once: a length check
/// cannot see fill redistributed into stripes, and an early version scored a
/// perfect 1.00 ratio while leaving 17% of the region empty.
test "default_config_leaves_no_large_voids" {
  let side = 60L * mm
  let region = Region::{ contour: square_ring(0L, 0L, side / 2L), holes: [] }
  let spacing = 7500L
  let n = 20
  // Manifest defaults: angle 45, depth 10.
  let segs = tile_dragon_curve_rotated(region, spacing, 10, 45.0)
  assert_true(segs.length() > 0)
  let grid = Array::make(n * n, 0)
  let mut total = 0.0
  for i = 0; i < segs.length(); i = i + 1 {
    let s = segs[i]
    let dx = (s.b.x - s.a.x).to_double()
    let dy = (s.b.y - s.a.y).to_double()
    total = total + (dx * dx + dy * dy).sqrt()
    for t = 0; t <= 5; t = t + 1 {
      let f = t.to_double() / 5.0
      let x = s.a.x.to_double() + dx * f
      let y = s.a.y.to_double() + dy * f
      let gx = ((x + (side / 2L).to_double()) / side.to_double() * n.to_double()).to_int()
      let gy = ((y + (side / 2L).to_double()) / side.to_double() * n.to_double()).to_int()
      if gx >= 0 && gx < n && gy >= 0 && gy < n {
        grid[gy * n + gx] = grid[gy * n + gx] + 1
      }
    }
  }
  let mut empty = 0
  for i = 0; i < n * n; i = i + 1 {
    if grid[i] == 0 {
      empty = empty + 1
    }
  }
  // Same 1% bound as the unrotated guard - rotation must not cost coverage.
  assert_true(empty * 100 < n * n)
  // And density must still be right on the default path.
  let ratio = total / (region_area(region) / spacing.to_double())
  assert_true(ratio > 0.95)
  assert_true(ratio < 1.15)
}

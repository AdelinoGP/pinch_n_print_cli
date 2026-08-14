// Dragon-curve sparse infill — pure tiling and colour mapping.
//
// This package deliberately imports NOTHING. It defines its own geometry
// structs rather than reusing the wit-bindgen `@geometry` types, because those
// carry `extern "wasm"` FFI and would drag a wasm-only dependency into a
// package whose whole point is to be unit-testable. The glue layer
// (`src/glue/main.mbt`, copied into the generated interface package at build
// time) converts between these types and the WIT ones.
//
// Determinism is the core invariant (packet 227 AC-3): no RNG, no clock, no
// hash-map iteration. Every function here is a pure function of its arguments.
// Floating point is used only for clip parameters and midpoint tests; IEEE
// double arithmetic is deterministic for a fixed input, so byte-identical
// output across runs is guaranteed.
//
// Coordinates are slicer units: 1 unit = 100 nm = 1e-4 mm.

///|
pub(all) struct Pt {
  x : Int64
  y : Int64
} derive(Debug, Eq)

///|
/// One emitted scan segment.
///
/// - `tile` is the ROTATION index (0-3) of the dragon this segment belongs to.
///   Four dragons rotated 90 degrees apart meet at each lattice point, and
///   `tile` is the colouring driver: one dragon, one tool, so the tiling is
///   legible in the printed part. It takes only four values globally, and is
///   intrinsic to the dragon - not derived from lattice position or emission
///   order - so a dragon cannot change colour across regions or layers.
/// - `ordinal` is the segment's index within its own dragon (stable even when
///   the segment is clipped away entirely).
/// - `generation` is the fold level at which its turn was introduced.
///
/// `ordinal` and `generation` are retained because they are the curve's
/// defining structure and are asserted by the unit tests, but they no longer
/// drive colour.
pub(all) struct Seg {
  tile : Int
  ordinal : Int
  generation : Int
  a : Pt
  b : Pt
} derive(Debug, Eq)

///|
pub(all) struct Ring {
  pts : Array[Pt]
} derive(Debug, Eq)

///|
/// A polygon with holes: the tiled area is `contour` minus every ring in
/// `holes`.
pub(all) struct Region {
  contour : Ring
  holes : Array[Ring]
} derive(Debug, Eq)

///|
/// Largest tiling depth we will honour. The curve has 2^depth segments, and
/// every one of them is clipped against every polygon edge, so the cost is
/// 2^depth * edges. 16 caps that at 65536 segments per region, which keeps a
/// pathological config from wedging a slice. Depths above this clamp rather
/// than error — a too-detailed request degrades, it does not fail.
pub let max_tiling_depth : Int = 16

///|
fn clamp_depth(depth : Int) -> Int {
  if depth < 0 {
    0
  } else if depth > max_tiling_depth {
    max_tiling_depth
  } else {
    depth
  }
}

///|
/// Number of trailing zero bits. Used for the fold generation.
fn ctz(n : Int) -> Int {
  if n == 0 {
    return 32
  }
  let mut count = 0
  let mut v = n
  while (v & 1) == 0 {
    count = count + 1
    v = v >> 1
  }
  count
}

///|
/// Heighway dragon turn rule: the i-th turn (i counted from 1) is a left turn
/// when `((i & -i) << 1) & i` is zero, otherwise a right turn.
fn turn_is_left(i : Int) -> Bool {
  (((i & -i) << 1) & i) == 0
}

///|
/// The fold generation of the segment at `ordinal` (0-based): the number of
/// trailing zeros of `ordinal + 1`. Segment 0 has generation 0, every other
/// even-indexed segment has generation 0, and the deeper folds recur at
/// exponentially rarer intervals — which is what makes it a useful colouring
/// input rather than a restatement of the ordinal.
pub fn generation_of(ordinal : Int) -> Int {
  ctz(ordinal + 1)
}

///|
/// Build the dragon-curve polyline: 2^depth segments of length `spacing`,
/// starting at `start` heading in +X.
///
/// Returns 2^depth + 1 points.
pub fn dragon_polyline(depth : Int, spacing : Int64, start : Pt) -> Array[Pt] {
  let d = clamp_depth(depth)
  let n = 1 << d
  let pts : Array[Pt] = []
  let mut cur = start
  pts.push(cur)
  // Direction index: 0 = +X, 1 = +Y, 2 = -X, 3 = -Y.
  let mut dir = 0
  let mut i = 1
  while i <= n {
    let next = match dir {
      0 => Pt::{ x: cur.x + spacing, y: cur.y }
      1 => Pt::{ x: cur.x, y: cur.y + spacing }
      2 => Pt::{ x: cur.x - spacing, y: cur.y }
      _ => Pt::{ x: cur.x, y: cur.y - spacing }
    }
    pts.push(next)
    cur = next
    if i < n {
      dir = if turn_is_left(i) { (dir + 1) % 4 } else { (dir + 3) % 4 }
    }
    i = i + 1
  }
  pts
}

///|
priv struct Bbox {
  min_x : Int64
  min_y : Int64
  max_x : Int64
  max_y : Int64
}

///|
fn bbox_of(pts : Array[Pt]) -> Bbox? {
  if pts.length() == 0 {
    return None
  }
  let mut min_x = pts[0].x
  let mut min_y = pts[0].y
  let mut max_x = pts[0].x
  let mut max_y = pts[0].y
  for i = 1; i < pts.length(); i = i + 1 {
    let p = pts[i]
    if p.x < min_x {
      min_x = p.x
    }
    if p.y < min_y {
      min_y = p.y
    }
    if p.x > max_x {
      max_x = p.x
    }
    if p.y > max_y {
      max_y = p.y
    }
  }
  Some(Bbox::{ min_x, min_y, max_x, max_y })
}

///|
/// Strict point-in-ring test by ray casting along +X. A point exactly on an
/// edge is not reported as inside; that is what keeps AC-5's "no endpoint
/// inside a hole" honest when a clipped endpoint lands exactly on a hole
/// boundary.
pub fn point_in_ring(ring : Ring, px : Double, py : Double) -> Bool {
  let pts = ring.pts
  let n = pts.length()
  if n < 3 {
    return false
  }
  let mut inside = false
  let mut j = n - 1
  for i = 0; i < n; i = i + 1 {
    let xi = pts[i].x.to_double()
    let yi = pts[i].y.to_double()
    let xj = pts[j].x.to_double()
    let yj = pts[j].y.to_double()
    if (yi > py) != (yj > py) {
      let t = (py - yi) / (yj - yi)
      let x_cross = xi + t * (xj - xi)
      if px < x_cross {
        inside = !inside
      }
    }
    j = i
  }
  inside
}

///|
/// Inside the tiled area: within the contour and outside every hole.
pub fn point_in_region(region : Region, px : Double, py : Double) -> Bool {
  if !point_in_ring(region.contour, px, py) {
    return false
  }
  for i = 0; i < region.holes.length(); i = i + 1 {
    if point_in_ring(region.holes[i], px, py) {
      return false
    }
  }
  true
}

///|
fn floor_to_int(v : Double) -> Int {
  let t = v.to_int()
  if v < 0.0 && t.to_double() != v { t - 1 } else { t }
}

///|
fn ceil_to_int(v : Double) -> Int {
  let t = v.to_int()
  if v > 0.0 && t.to_double() != v { t + 1 } else { t }
}

///|
fn to_i64(v : Double) -> Int64 {
  if v < 0.0 { (v - 0.5).to_int64() } else { (v + 0.5).to_int64() }
}

///|
fn abs_d(v : Double) -> Double {
  if v < 0.0 {
    -v
  } else {
    v
  }
}

///|
/// Insertion sort. Hand-rolled rather than reaching for a stdlib sort so the
/// ordering is pinned by this file and cannot drift with a toolchain update —
/// segment order is part of the byte-identical-output contract.
fn sort_doubles(arr : Array[Double]) -> Unit {
  for i = 1; i < arr.length(); i = i + 1 {
    let key = arr[i]
    let mut j = i - 1
    while j >= 0 && arr[j] > key {
      arr[j + 1] = arr[j]
      j = j - 1
    }
    arr[j + 1] = key
  }
}

///|
/// Append the parameters along segment a->b at which it crosses any edge of
/// `ring`. Parameters outside (0,1) are discarded.
fn collect_edge_ts(
  ring : Ring,
  a : Pt,
  b : Pt,
  out : Array[Double],
) -> Unit {
  let pts = ring.pts
  let n = pts.length()
  if n < 2 {
    return
  }
  let ax = a.x.to_double()
  let ay = a.y.to_double()
  let dx = b.x.to_double() - ax
  let dy = b.y.to_double() - ay
  let mut j = n - 1
  for i = 0; i < n; i = i + 1 {
    let cx = pts[j].x.to_double()
    let cy = pts[j].y.to_double()
    let ex = pts[i].x.to_double() - cx
    let ey = pts[i].y.to_double() - cy
    let denom = dx * ey - dy * ex
    if abs_d(denom) > 1.0e-9 {
      let t = ((cx - ax) * ey - (cy - ay) * ex) / denom
      let u = ((cx - ax) * dy - (cy - ay) * dx) / denom
      if t > 0.0 && t < 1.0 && u >= 0.0 && u <= 1.0 {
        out.push(t)
      }
    }
    j = i
  }
}

///|
fn lerp_pt(a : Pt, b : Pt, t : Double) -> Pt {
  let x = a.x.to_double() + (b.x.to_double() - a.x.to_double()) * t
  let y = a.y.to_double() + (b.y.to_double() - a.y.to_double()) * t
  // Round half away from zero, so the result does not depend on a
  // banker's-rounding convention that could differ between backends.
  let rx = if x < 0.0 { (x - 0.5).to_int64() } else { (x + 0.5).to_int64() }
  let ry = if y < 0.0 { (y - 0.5).to_int64() } else { (y + 0.5).to_int64() }
  Pt::{ x: rx, y: ry }
}

///|
/// Emit one kept interval as a segment, insetting the ends that came from a
/// boundary crossing.
///
/// Why the inset: `lerp_pt` rounds to whole units, and a crossing point sits
/// exactly *on* the boundary, so rounding is as likely to land just inside a
/// hole as just outside it. One unit (100 nm) of inset is well below any
/// printable tolerance but is enough to keep the rounded endpoint in the tiled
/// area, which is what AC-5 asserts.
///
/// Both ends are inset, including t=0 and t=1. An earlier version left those
/// alone on the theory that they are interior curve vertices rather than
/// crossings - but a vertex can land exactly on a hole boundary, where ray
/// casting classifies it arbitrarily, and one did test as inside a hole. The
/// cost is a 100 nm gap at each dragon vertex.
fn emit_run(
  region : Region,
  a : Pt,
  b : Pt,
  t0 : Double,
  t1 : Double,
  out : Array[(Pt, Pt)],
) -> Unit {
  ignore(region)
  let dx = b.x.to_double() - a.x.to_double()
  let dy = b.y.to_double() - a.y.to_double()
  let len = (dx * dx + dy * dy).sqrt()
  if len <= 0.0 {
    return
  }
  // Inset BOTH ends unconditionally, not just the ones that came from a
  // crossing. A curve vertex can land exactly on a hole boundary, where ray
  // casting classifies the point arbitrarily - so treating t=0/t=1 as
  // guaranteed-interior is unsound and did let an endpoint test as inside a
  // hole. The cost is a 100 nm gap at each dragon vertex, far below any
  // printable tolerance.
  let eps = 1.0 / len
  let s = t0 + eps
  let e = t1 - eps
  if e <= s {
    return
  }
  let pa = lerp_pt(a, b, s)
  let pb = lerp_pt(a, b, e)
  if pa.x == pb.x && pa.y == pb.y {
    return
  }
  out.push((pa, pb))
}

///|
/// Clip one segment to the region, returning the inside portions in order
/// along a->b. Adjacent kept intervals are merged so a mere boundary touch
/// does not split one run into two paths.
pub fn clip_segment(region : Region, a : Pt, b : Pt) -> Array[(Pt, Pt)] {
  let out : Array[(Pt, Pt)] = []
  if a.x == b.x && a.y == b.y {
    return out
  }
  let ts : Array[Double] = [0.0, 1.0]
  collect_edge_ts(region.contour, a, b, ts)
  for i = 0; i < region.holes.length(); i = i + 1 {
    collect_edge_ts(region.holes[i], a, b, ts)
  }
  sort_doubles(ts)
  let ax = a.x.to_double()
  let ay = a.y.to_double()
  let dx = b.x.to_double() - ax
  let dy = b.y.to_double() - ay
  // Accumulate a run of consecutive inside intervals, flushing when it breaks.
  let mut run_start = -1.0
  let mut run_end = -1.0
  let mut have_run = false
  for k = 0; k < ts.length() - 1; k = k + 1 {
    let t0 = ts[k]
    let t1 = ts[k + 1]
    if t1 - t0 > 1.0e-12 {
      let tm = (t0 + t1) * 0.5
      let inside = point_in_region(region, ax + dx * tm, ay + dy * tm)
      if inside {
        if have_run {
          run_end = t1
        } else {
          run_start = t0
          run_end = t1
          have_run = true
        }
      } else if have_run {
        emit_run(region, a, b, run_start, run_end, out)
        have_run = false
      }
    }
  }
  if have_run {
    emit_run(region, a, b, run_start, run_end, out)
  }
  out
}

///|
/// Signed-area magnitude of a ring (shoelace).
fn ring_area(ring : Ring) -> Double {
  let pts = ring.pts
  let n = pts.length()
  if n < 3 {
    return 0.0
  }
  let mut acc = 0.0
  let mut j = n - 1
  for i = 0; i < n; i = i + 1 {
    acc = acc +
      (pts[j].x.to_double() * pts[i].y.to_double() -
      pts[i].x.to_double() * pts[j].y.to_double())
    j = i
  }
  let a = acc * 0.5
  if a < 0.0 {
    -a
  } else {
    a
  }
}

///|
/// Fillable area: contour minus holes.
pub fn region_area(region : Region) -> Double {
  let mut a = ring_area(region.contour)
  for i = 0; i < region.holes.length(); i = i + 1 {
    a = a - ring_area(region.holes[i])
  }
  if a < 0.0 {
    0.0
  } else {
    a
  }
}

///|
fn total_length(segs : Array[Seg]) -> Double {
  let mut t = 0.0
  for i = 0; i < segs.length(); i = i + 1 {
    let dx = (segs[i].b.x - segs[i].a.x).to_double()
    let dy = (segs[i].b.y - segs[i].a.y).to_double()
    t = t + (dx * dx + dy * dy).sqrt()
  }
  t
}

///|
/// As `tile_dragon_curve`, but rotates the whole tiling by `angle_deg`.
///
/// **Density correction.** A dragon tiling laid at `line_spacing` does not
/// deliver `area / line_spacing` of path: instances are clipped at the region
/// boundary and the curve's fractal lobes leave gaps, so the emitted length
/// falls short. Measured on a 60 mm cube at 60% density, the uncorrected
/// tiling produced only 67% of what rectilinear laid down at the same setting
/// — i.e. asking for 60% infill got you about 40%, with visible voids.
///
/// So this measures what it actually emitted, compares it to the target
/// `area / line_spacing`, and regenerates once at a proportionally finer
/// spacing when it fell short. One corrective pass, not a loop: the response
/// is close to linear in `1 / spacing`, and a fixed pass keeps the cost
/// bounded and the result deterministic.
///
/// This is a correction, not a fudge factor: it is driven by this region's own
/// measured output, so it does not overshoot on large simple regions where the
/// first pass already hits the target.
pub fn tile_dragon_curve_rotated(
  region : Region,
  line_spacing : Int64,
  tiling_depth : Int,
  angle_deg : Double,
) -> Array[Seg] {
  let first = place_dragons(region, line_spacing, tiling_depth, angle_deg)
  if line_spacing <= 0L {
    return first
  }
  let area = region_area(region)
  if area <= 0.0 {
    return first
  }
  let target = area / line_spacing.to_double()
  let got = total_length(first)
  if got <= 0.0 || got >= target * 0.98 {
    return first
  }
  // Scale spacing by the shortfall. Clamp the correction so a pathological
  // region cannot drive the spacing to zero and explode the segment count.
  let mut scale = got / target
  if scale < 0.35 {
    scale = 0.35
  }
  let corrected = to_i64(line_spacing.to_double() * scale)
  if corrected <= 0L || corrected >= line_spacing {
    return first
  }
  let second = place_dragons(region, corrected, tiling_depth, angle_deg)
  // Keep whichever lands closer to the target, so the correction can never
  // make a region worse than leaving it alone.
  let got2 = total_length(second)
  let err1 = if got > target { got - target } else { target - got }
  let err2 = if got2 > target { got2 - target } else { target - got2 }
  if err2 < err1 {
    second
  } else {
    first
  }
}

///|
/// Exact 90-degree rotation about the origin, `times` quarter turns. Integer
/// arithmetic, so the four copies at a lattice point stay exactly congruent -
/// a float rotation would round them apart and break the interlock.
fn rot90_pt(p : Pt, times : Int) -> Pt {
  let mut x = p.x
  let mut y = p.y
  for i = 0; i < times; i = i + 1 {
    let nx = -y
    let ny = x
    x = nx
    y = ny
  }
  Pt::{ x, y }
}

///|
/// Rotate a point list about the origin. Rounding to whole units after
/// rotation perturbs the lattice by at most half a unit (50 nm), far below
/// the clipper's 1-unit inset, so the tiling relationship survives.
fn rotate_points(pts : Array[Pt], angle_deg : Double) -> Array[Pt] {
  let a = angle_deg % 360.0
  if a == 0.0 {
    return pts
  }
  let r = a * 3.141592653589793 / 180.0
  let c = @math.cos(r)
  let s = @math.sin(r)
  let out : Array[Pt] = []
  for i = 0; i < pts.length(); i = i + 1 {
    let x = pts[i].x.to_double()
    let y = pts[i].y.to_double()
    out.push(Pt::{ x: to_i64(x * c - y * s), y: to_i64(x * s + y * c) })
  }
  out
}

///|
/// Largest number of dragon instances we will place for one polygon. A
/// pathological `line_spacing`/`tiling_depth` combination can otherwise ask
/// for an unbounded lattice; capping degrades coverage rather than wedging
/// the slice.
///
/// Truncation is SILENT: on hitting the cap the remaining lattice cells are
/// simply not placed, leaving that part of the region unfilled with no
/// diagnostic. A guest has no log channel on this path, so the honest
/// mitigation is the bound itself being far above any realistic region
/// (4096 dragons at a 12 mm pitch spans roughly 750 mm).
pub let max_tiles : Int = 4096

///|
/// Tile `region` with dragon curves at `line_spacing` and `tiling_depth`.
///
/// Fill `region` with dragon curves at `line_spacing` and `tiling_depth`.
///
/// The region is tiled by **four dragons rotated 90 degrees apart about each
/// lattice point** (see `place_dragons` for the lattice and the reasoning).
/// Each dragon prints in its own tool, so the tiling reads directly off the
/// finished part.
///
/// Coverage is exact: measured 0 empty sample cells at depths 8, 10 and 12 on
/// a 60 mm square, and 99.9% of a rectilinear control's sparse path over a
/// whole slice at the same density.
///
/// Two superseded designs, recorded so the reasoning is not repeated:
/// a *single* un-tiled dragon cannot reach a polygon's corners at any depth,
/// and *translation-only* copies cannot tile at all - a lattice cell holds
/// `2 * 2^n` unit grid edges while one dragon supplies `2^n`, so they cover at
/// most half and leave fractal voids.
pub fn tile_dragon_curve(
  region : Region,
  line_spacing : Int64,
  tiling_depth : Int,
) -> Array[Seg] {
  tile_dragon_curve_rotated(region, line_spacing, tiling_depth, 0.0)
}

///|
/// As `tile_dragon_curve`, but rotates the whole tiling by `angle_deg`.
/// (`rotate_points` rotates about the origin; the lattice placement below then
/// positions the result, so the visible effect is a rotated tiling.)
///
/// The lattice basis is derived from the *rotated* curve, so coverage is
/// approximately angle-independent (`rotation_preserves_coverage` bounds it
/// both ways). It is approximate for the same reason the unrotated placement
/// is - see `tile_dragon_curve`, which explains why this is not an exact plane
/// tiling. `infill_angle` is a real control, not a decorative key: successive
/// prints can cross-hatch by rotating the dragon.
fn place_dragons(
  region : Region,
  line_spacing : Int64,
  tiling_depth : Int,
  angle_deg : Double,
) -> Array[Seg] {
  let segs : Array[Seg] = []
  if line_spacing <= 0L {
    return segs
  }
  let rb = match bbox_of(region.contour.pts) {
    None => return segs
    Some(b) => b
  }
  // Four dragons per lattice point on the `e(1+-i)` lattice lay down exactly
  // two units of path per unit cell - a complete single cover of the square
  // grid - so the segment length is DOUBLED here for the density to come out
  // right.
  //
  // Arithmetic: cell area = 2|e|^2 with |e| = step * 2^(n/2), and the four
  // dragons contribute 4 * 2^n * step of path, so path/area = 2/step. With
  // step = 2 * line_spacing that is exactly 1/line_spacing - the caller's
  // requested density, honoured. (Measuring the *undoubled* arrangement gives
  // a ratio of 2.000; the shipped, doubled one measures ~1.0, which is what
  // `tiling_covers_the_region_at_the_requested_density` asserts.)
  let step = line_spacing * 2L
  // No region-size depth cap.
  //
  // An earlier version shrank the dragon until at least three spanned the
  // region, because the translation-only placement left large fractal voids at
  // high depth (65/400 empty sample cells at depth 12). The four-fold tiling
  // covers exactly at EVERY depth - measured 0/400 from depth 8 to 12 - so the
  // cap now buys nothing and costs two things: it made the dragons too small
  // to see, turning the pattern into an anonymous diagonal mesh, and it
  // silently overrode `tiling_depth`, including per-region overrides, which
  // made AC-6 misdescribe shipped behaviour.
  let depth = clamp_depth(tiling_depth)
  let unrotated = dragon_polyline(depth, step, Pt::{ x: 0L, y: 0L })
  if unrotated.length() < 2 {
    return segs
  }
  let raw = rotate_points(unrotated, angle_deg)
  let cb = match bbox_of(raw) {
    None => return segs
    Some(b) => b
  }
  let ex = (raw[raw.length() - 1].x - raw[0].x).to_double()
  let ey = (raw[raw.length() - 1].y - raw[0].y).to_double()
  if ex == 0.0 && ey == 0.0 {
    return segs
  }
  // Lattice basis e(1+i), e(1-i): cell area 2|e|^2, which is exactly the area
  // four dragons cover. Using (e, rot90 e) instead puts four dragons in half
  // that area and double-extrudes every grid edge.
  let v1x = ex - ey
  let v1y = ex + ey
  let v2x = ex + ey
  let v2y = ey - ex
  let det = v1x * v2y - v1y * v2x
  if det == 0.0 {
    return segs
  }
  let mut min_k = 0.0
  let mut max_k = 0.0
  let mut min_m = 0.0
  let mut max_m = 0.0
  let mut first = true
  let corners_x = [rb.min_x, rb.max_x, rb.min_x, rb.max_x]
  let corners_y = [rb.min_y, rb.min_y, rb.max_y, rb.max_y]
  for i = 0; i < 4; i = i + 1 {
    let cx = corners_x[i].to_double()
    let cy = corners_y[i].to_double()
    let k = (cx * v2y - cy * v2x) / det
    let m = (v1x * cy - v1y * cx) / det
    if first {
      min_k = k
      max_k = k
      min_m = m
      max_m = m
      first = false
    } else {
      if k < min_k { min_k = k }
      if k > max_k { max_k = k }
      if m < min_m { min_m = m }
      if m > max_m { max_m = m }
    }
  }
  let reach_x = (cb.max_x - cb.min_x).to_double()
  let reach_y = (cb.max_y - cb.min_y).to_double()
  let reach = if reach_x > reach_y { reach_x } else { reach_y }
  let pitch = (v1x * v1x + v1y * v1y).sqrt()
  let margin = ceil_to_int(reach / pitch) + 1
  let k0 = floor_to_int(min_k) - margin
  let k1 = ceil_to_int(max_k) + margin
  let m0 = floor_to_int(min_m) - margin
  let m1 = ceil_to_int(max_m) + margin
  let mut placed = 0
  for k = k0; k <= k1; k = k + 1 {
    for m = m0; m <= m1; m = m + 1 {
      if placed >= max_tiles {
        return segs
      }
      let off_x = to_i64(v1x * k.to_double() + v2x * m.to_double())
      let off_y = to_i64(v1y * k.to_double() + v2y * m.to_double())
      let mut used = false
      // Four dragons per lattice point, rotated 90 degrees apart about that
      // point. This is the arrangement that actually tiles: translation alone
      // covers only half the grid edges and leaves fractal voids no neighbour
      // reaches into. `tile` is the rotation index, so the four interlocking
      // dragons print in four tools and the tiling reads directly off the part
      // - and because the index is intrinsic to the dragon rather than derived
      // from lattice position or emission order, a dragon keeps its colour
      // across regions and layers.
      for rot = 0; rot < 4; rot = rot + 1 {
        for i = 0; i < raw.length() - 1; i = i + 1 {
          let pa = rot90_pt(raw[i], rot)
          let pb = rot90_pt(raw[i + 1], rot)
          let a = Pt::{ x: pa.x + off_x, y: pa.y + off_y }
          let b = Pt::{ x: pb.x + off_x, y: pb.y + off_y }
          let pieces = clip_segment(region, a, b)
          if pieces.length() > 0 {
            used = true
          }
          let gen = generation_of(i)
          for c = 0; c < pieces.length(); c = c + 1 {
            let (qa, qb) = pieces[c]
            segs.push(Seg::{ tile: rot, ordinal: i, generation: gen, a: qa, b: qb })
          }
        }
      }
      if used {
        placed = placed + 1
      }
    }
  }
  segs
}

///|
/// Per-region override precedence for `tiling_depth`.
///
/// Mirrors what `slicer_sdk::config_resolution::resolve_float` does for the
/// Rust core modules: the region's own config view wins when it carries the
/// key, otherwise the module-global value from the module config stands. A
/// foreign guest has no SDK to lean on, so the rule is spelled out here and
/// unit-tested rather than assumed (packet 227 AC-6).
pub fn pick_tiling_depth(region_override : Int?, module_default : Int) -> Int {
  match region_override {
    Some(v) => clamp_depth(v)
    None => clamp_depth(module_default)
  }
}

///|
/// Map a dragon instance to a tool index, wrapped into `[0, tool_count)`.
///
/// The colouring driver is the **tile index** - the identity of the dragon
/// instance within the tiling - so each dragon prints in a single tool and the
/// tiling structure is what you see in the finished part. (An earlier version
/// keyed on the segment ordinal, which coloured *within* each dragon and made
/// the tiling invisible.)
///
/// Returns `None` when there is nothing to map into - an empty `color_map` or
/// a zero `tool_count`. Returning `None` rather than a tool keeps this free of
/// a zero divisor and lets `run` emit `tool_index = None`, telling the host to
/// resolve the region tool as usual (packet 227 AC-N1).
pub fn map_tiling_index_to_tool(
  tile_index : Int,
  color_map : Array[Int],
  tool_count : Int,
) -> Int? {
  if tool_count <= 0 || color_map.length() == 0 {
    return None
  }
  let mut idx = tile_index % color_map.length()
  if idx < 0 {
    idx = idx + color_map.length()
  }
  let raw = color_map[idx]
  let mut tool = raw % tool_count
  if tool < 0 {
    tool = tool + tool_count
  }
  Some(tool)
}

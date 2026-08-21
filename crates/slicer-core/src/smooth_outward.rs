// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/MutablePolygon.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Outward-only corner smoothing (`smooth_outward`).
//!
//! Line-by-line port of canonical `smooth_outward` (`MutablePolygon.cpp`,
//! itself adapted from Cura's `ConstPolygonRef::smooth_outward()` by Tim
//! Kuipers), together with its helper `clip_narrow_corner` (Cura's
//! `smooth_corner_complex()`) and the `remove_duplicates` prepass. This is the
//! regularization pass OrcaSlicer applies to support geometry, paired with a
//! `closing` — see canonical `generate_interface_layers`
//! (`Support/SupportCommon.cpp`) and `SupportGridPattern::extract_support`'s
//! `smsSnug` branch (`Support/SupportMaterial.cpp`), and
//! [`crate::polygon_ops::closing_ex`].
//!
//! **What it actually does.** Each vertex is visited exactly once. A vertex is
//! a candidate when `cross2(p0 - p1, p2 - p1) > 0` — under the Slic3r winding
//! convention (contour CCW, holes CW, so material is always left of travel)
//! that is a concave corner, measured through the material. A candidate is
//! only clipped when the angle at the corner is **sharper than 135 degrees**;
//! flatter corners are left alone. The corner vertex is then always removed,
//! and replaced by **zero, one, or two** new vertices forming a straight
//! chamfer whose length is `clip_dist_scaled`. Very narrow cracks are closed
//! iteratively by `clip_narrow_corner`, which can swallow neighbouring
//! vertices and, for a small enough ring, delete it outright.
//!
//! **`clip_dist_scaled` is the chord.** It is the length of the *new clipping
//! edge* inserted across the corner, not the distance travelled back along
//! each arm. For a 90-degree corner each arm is cut back by
//! `clip_dist_scaled / sqrt(2)`.
//!
//! **Not a strict superset.** A ring can be deleted entirely (a narrow hole is
//! filled in — which grows the region — but a small enough *contour* is
//! dropped too, which does not). Callers relying on coverage must keep
//! `clip_dist_scaled` small relative to their feature size, exactly as
//! canonical's call sites do.
//!
//! **Not gated behind `host-algos`:** guest WASM modules call this.
//!
//! # Deviations from canonical
//!
//! - **Units.** `clip_dist_scaled` is a scaled length in *this* crate's units
//!   (1 unit = 100 nm), where canonical's `coord_t` is 1 nm. Every canonical
//!   scaled constant is therefore divided by 100: `SCALED_EPSILON`
//!   (`scale_(1e-4)`) is **1** here versus 100 upstream, and the
//!   `remove_duplicates` prepass epsilon (`scaled<double>(0.01)`) is **100**
//!   here versus 10000 upstream. See `docs/08_coordinate_system.md`.
//! - **Integer width.** Canonical does its exact arithmetic in `int64_t` over
//!   32-bit `coord_t`; [`Point2`] is `i64`, so cross products, dot products
//!   and squared norms are computed in `i128` to keep the same
//!   never-overflows property. No behavioural difference in range.
//! - **Corrected inverted assignment in `clip_narrow_corner`.** In canonical's
//!   "one side is far, the other blocked" branch the final statement reads
//!   `(backward == Far ? *it2 : *it0) += (v.cast<double>() * t).cast<coord_t>();`
//!   after `if (forward == Far) { std::swap(p0, p2); std::swap(p02, p22); }`.
//!   `forward` tracks the `it2` side (it advances via `it2.next()`), `backward`
//!   tracks the `it0` side (`it0.prev()`), and the computed offset `v * t` is
//!   relative to `p0`, i.e. relative to whichever iterator is the *far* one.
//!   The ternary applies it to the *other* one. Ported here as
//!   `if forward == Far { it2 } else { it0 }`, which is what canonical's own
//!   comment ("Find point on (p0, p02) at distance shortcut_length from p2")
//!   and its `assert(dfar2 >= shortcut_length2)` describe. Flagged for
//!   upstream; every other statement is verbatim.
//! - **Debug asserts as guards.** Canonical asserts `u > 0.` before the two
//!   `sqrt` calls and `t > 0 && t < 1` afterwards; in a release build a
//!   violation would produce a NaN vertex. Here those are `if` guards that
//!   fall through to "just remove the corner vertex", which is the same
//!   outcome canonical's neighbouring branches take.
//! - **No arc / `segments` parameter.** Canonical emits a straight chamfer and
//!   has no arc-resolution knob. An earlier revision of this module invented
//!   one (`DEFAULT_SMOOTHING_SEGMENTS = 4`) along with a 0.05 rad flatness
//!   threshold and a "cut at most half of each edge" cap; none of the three
//!   exists upstream and all have been removed.

use slicer_ir::{ExPolygon, Point2, Polygon};

/// Canonical `SCALED_EPSILON` = `scale_(EPSILON)` = `scale_(1e-4)`.
///
/// Upstream that is 100 (1 nm units); here 1 unit = 100 nm, so it is 1.
const SCALED_EPSILON: i128 = 1;

/// Canonical's `remove_duplicates(polygon, scaled<double>(0.01))` prepass
/// epsilon: 0.01 mm expressed in this crate's units.
const REMOVE_DUPLICATES_EPS: f64 = 0.01 * slicer_ir::UNITS_PER_MM;

// ---------------------------------------------------------------------------
// Public API — mirrors canonical's overload set in `MutablePolygon.hpp`.
// ---------------------------------------------------------------------------

/// Smooths a set of [`ExPolygon`]s outward.
///
/// Port of canonical `inline ExPolygons smooth_outward(ExPolygons, coord_t)`:
/// the contour and every hole are smoothed independently, emptied holes are
/// dropped, and an [`ExPolygon`] whose contour was emptied is dropped.
///
/// `clip_dist_scaled` is the length of the clipping edge inserted across each
/// clipped corner, **in scaled units (1 unit = 100 nm)**. Canonical's support
/// call sites pass `scaled(extrusion_width)` (`SupportMaterial.cpp`) and
/// `interface_flow.scaled_spacing() * 1.5` (`SupportCommon.cpp`).
pub fn smooth_outward(subject: &[ExPolygon], clip_dist_scaled: i64) -> Vec<ExPolygon> {
    subject
        .iter()
        .filter_map(|e| {
            let out = smooth_outward_ex(e, clip_dist_scaled);
            (!out.contour.points.is_empty()).then_some(out)
        })
        .collect()
}

/// Smooths a single [`ExPolygon`] outward; emptied holes are dropped.
///
/// The returned contour may be empty — canonical deletes a ring that
/// degenerates while clipping. See [`smooth_outward`] for the parameters.
pub fn smooth_outward_ex(subject: &ExPolygon, clip_dist_scaled: i64) -> ExPolygon {
    ExPolygon {
        contour: smooth_outward_polygon(&subject.contour, clip_dist_scaled),
        holes: subject
            .holes
            .iter()
            .map(|h| smooth_outward_polygon(h, clip_dist_scaled))
            .filter(|h| !h.points.is_empty())
            .collect(),
    }
}

/// Smooths a set of bare [`Polygon`]s outward, dropping emptied rings.
///
/// Port of canonical `inline Polygons smooth_outward(Polygons, coord_t)`.
pub fn smooth_outward_polygons(subject: &[Polygon], clip_dist_scaled: i64) -> Vec<Polygon> {
    subject
        .iter()
        .map(|p| smooth_outward_polygon(p, clip_dist_scaled))
        .filter(|p| !p.points.is_empty())
        .collect()
}

/// Smooths a single ring outward.
///
/// The ring must follow the Slic3r winding convention (CCW for a contour, CW
/// for a hole) so that material lies to the left of travel; canonical selects
/// corners purely by the sign of `cross2(p0 - p1, p2 - p1)` and never inspects
/// winding. Returns an empty [`Polygon`] when the ring degenerates to fewer
/// than three points, matching canonical `MutablePolygon::polygon()`, which
/// emits nothing unless `valid()` (`size() >= 3`).
pub fn smooth_outward_polygon(subject: &Polygon, clip_dist_scaled: i64) -> Polygon {
    let mut ring = Ring::new(&subject.points);
    smooth_outward_ring(&mut ring, clip_dist_scaled as i128);
    Polygon {
        points: ring.to_points(),
    }
}

// ---------------------------------------------------------------------------
// Exact integer vector helpers (canonical's `Vec2i64` / `cross2`).
// ---------------------------------------------------------------------------

/// A 2D vector in exact integer arithmetic (canonical's `Vec2i64`).
type V = (i128, i128);

fn vec_of(p: Point2) -> V {
    (p.x as i128, p.y as i128)
}

fn vsub(a: V, b: V) -> V {
    (a.0 - b.0, a.1 - b.1)
}

fn cross2(a: V, b: V) -> i128 {
    a.0 * b.1 - a.1 * b.0
}

fn dot(a: V, b: V) -> i128 {
    a.0 * b.0 + a.1 * b.1
}

fn norm2(a: V) -> i128 {
    dot(a, a)
}

fn dotf(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

fn norm2f(a: (f64, f64)) -> f64 {
    dotf(a, a)
}

fn tof(a: V) -> (f64, f64) {
    (a.0 as f64, a.1 as f64)
}

/// `p + v`, truncating toward zero exactly like Eigen's `.cast<coord_t>()`.
fn offset_point(p: Point2, v: (f64, f64)) -> Point2 {
    Point2 {
        x: p.x + v.0 as i64,
        y: p.y + v.1 as i64,
    }
}

// ---------------------------------------------------------------------------
// `MutablePolygon` — a circular doubly linked list over a flat arena, with a
// free list, so indices survive insertion. Port of `MutablePolygon.hpp`.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Node {
    pt: Point2,
    prev: i32,
    next: i32,
}

struct Ring {
    data: Vec<Node>,
    size: i32,
    head: i32,
    head_free: i32,
}

impl Ring {
    fn new(pts: &[Point2]) -> Self {
        let n = pts.len() as i32;
        let mut data = Vec::with_capacity(pts.len());
        for (i, &pt) in pts.iter().enumerate() {
            data.push(Node {
                pt,
                prev: i as i32 - 1,
                next: i as i32 + 1,
            });
        }
        if n > 0 {
            data[0].prev = n - 1;
            data[(n - 1) as usize].next = 0;
        }
        Ring {
            data,
            size: n,
            head: if n > 0 { 0 } else { -1 },
            head_free: -1,
        }
    }

    fn size(&self) -> i32 {
        self.size
    }

    fn begin(&self) -> i32 {
        self.head
    }

    /// Canonical `end()` points at the **last** item before roll over, not one
    /// past it.
    fn end(&self) -> i32 {
        if self.size == 0 {
            -1
        } else {
            self.data[self.head as usize].prev
        }
    }

    fn pt(&self, i: i32) -> Point2 {
        self.data[i as usize].pt
    }

    fn vpt(&self, i: i32) -> V {
        vec_of(self.data[i as usize].pt)
    }

    fn set_pt(&mut self, i: i32, pt: Point2) {
        self.data[i as usize].pt = pt;
    }

    fn prev(&self, i: i32) -> i32 {
        self.data[i as usize].prev
    }

    fn next(&self, i: i32) -> i32 {
        self.data[i as usize].next
    }

    fn clear(&mut self) {
        self.data.clear();
        self.size = 0;
        self.head = -1;
        self.head_free = -1;
    }

    /// Removes `i`, returning the following node (or `-1` if the ring emptied).
    fn remove(&mut self, i: i32) -> i32 {
        let (prev, next) = (self.data[i as usize].prev, self.data[i as usize].next);
        self.data[i as usize].next = self.head_free;
        self.head_free = i;
        self.size -= 1;
        if self.size == 0 {
            self.head = -1;
            return -1;
        }
        if self.head == i {
            self.head = next;
        }
        self.data[prev as usize].next = next;
        self.data[next as usize].prev = prev;
        next
    }

    /// Inserts `pt` **before** `i`, returning the new node.
    fn insert(&mut self, i: i32, pt: Point2) -> i32 {
        let j = self.data[i as usize].prev;
        let n = if self.head_free == -1 {
            self.data.push(Node {
                pt,
                prev: j,
                next: i,
            });
            self.data.len() as i32 - 1
        } else {
            let n = self.head_free;
            self.head_free = self.data[n as usize].next;
            self.data[n as usize] = Node {
                pt,
                prev: j,
                next: i,
            };
            n
        };
        self.data[j as usize].next = n;
        self.data[i as usize].prev = n;
        self.size += 1;
        n
    }

    /// Canonical `MutablePolygon::polygon()`: emits nothing unless `valid()`,
    /// i.e. unless at least three points remain.
    fn to_points(&self) -> Vec<Point2> {
        if self.size < 3 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(self.size as usize);
        let mut it = self.head;
        loop {
            out.push(self.pt(it));
            it = self.next(it);
            if it == self.head {
                break;
            }
        }
        out
    }
}

/// Canonical `MutablePolygon::range`: an inclusive `[begin, end]` window of
/// still-unprocessed vertices that shrinks as vertices are consumed or removed.
struct Range {
    begin: i32,
    end: i32,
}

impl Range {
    fn new(ring: &Ring) -> Self {
        Range {
            begin: ring.begin(),
            end: ring.end(),
        }
    }

    fn is_empty(&self) -> bool {
        self.begin < 0
    }

    fn make_empty(&mut self) {
        self.begin = -1;
        self.end = -1;
    }

    fn advance_front(&mut self, ring: &Ring) {
        if self.begin == self.end {
            self.make_empty();
        } else {
            self.begin = ring.next(self.begin);
        }
    }

    fn retract_back(&mut self, ring: &Ring) {
        if self.begin == self.end {
            self.make_empty();
        } else {
            self.end = ring.prev(self.end);
        }
    }

    fn process_next(&mut self, ring: &Ring) -> i32 {
        let out = self.begin;
        self.advance_front(ring);
        out
    }

    fn remove_front(&mut self, ring: &mut Ring, it: i32) -> i32 {
        if !self.is_empty() && self.begin == it {
            self.advance_front(ring);
        }
        ring.remove(it)
    }

    fn remove_back(&mut self, ring: &mut Ring, it: i32) -> i32 {
        if !self.is_empty() && self.end == it {
            self.retract_back(ring);
        }
        ring.remove(it)
    }
}

/// Port of canonical `remove_duplicates(MutablePolygon&, double eps)`.
fn remove_duplicates_eps(ring: &mut Ring, eps: f64) {
    if ring.size == 0 {
        return;
    }
    let eps2 = eps * eps;
    let begin = ring.begin();
    let mut it = ring.next(begin);
    while it != begin {
        let prev = ring.prev(it);
        let d = tof(vsub(ring.vpt(it), ring.vpt(prev)));
        if norm2f(d) < eps2 {
            it = ring.remove(it);
        } else {
            it = ring.next(it);
        }
    }
}

// ---------------------------------------------------------------------------
// The algorithm.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Free,
    Blocked,
    Far,
}

/// Port of canonical `void smooth_outward(MutablePolygon&, coord_t)`.
fn smooth_outward_ring(ring: &mut Ring, clip_dist_scaled: i128) {
    remove_duplicates_eps(ring, REMOVE_DUPLICATES_EPS);

    let clip_dist_scaled2 = clip_dist_scaled * clip_dist_scaled;
    let clip_eps = clip_dist_scaled + SCALED_EPSILON;
    let clip_dist_scaled2eps = clip_eps * clip_eps;
    let foot_dist_min2 = (SCALED_EPSILON * SCALED_EPSILON) as f64;

    // Each source point will be visited exactly once.
    let mut unprocessed = Range::new(ring);
    while !unprocessed.is_empty() && ring.size() > 2 {
        let it1 = unprocessed.process_next(ring);
        let mut it0 = ring.prev(it1);
        let mut it2 = ring.next(it1);
        let p0 = ring.vpt(it0);
        let p1 = ring.vpt(it1);
        let p1_pt = ring.pt(it1);
        let p2 = ring.vpt(it2);
        let v1 = vsub(p0, p1);
        let v2 = vsub(p2, p1);
        if cross2(v1, v2) <= 0 {
            // Convex corner (through the material). Leave it alone.
            continue;
        }
        // Concave corner.
        let dt = dot(v1, v2);
        let mut l2v1 = norm2(v1) as f64;
        let mut l2v2 = norm2(v2) as f64;
        if !(dt > 0 || (dt as f64) * (dt as f64) * 2.0 < l2v1 * l2v2) {
            // The corner is flatter than 135 degrees; not worth cutting.
            continue;
        }
        // Simplify the sharp angle.
        let v02 = vsub(p2, p0);
        let l2v02 = norm2(v02);
        ring.remove(it1);
        if l2v02 < clip_dist_scaled2 {
            // (p0, p2) is short. Clip a sharp concave corner by possibly
            // expanding the trimming region left of it0 and right of it2.
            if clip_narrow_corner(
                p1,
                &mut it0,
                &mut it2,
                ring,
                &mut unprocessed,
                l2v02,
                clip_dist_scaled,
            ) {
                // Trimmed down to an empty polygon or a single CCW triangle.
                return;
            }
        } else if l2v02 > clip_dist_scaled2eps {
            // Clip an obtuse corner.
            let mut v1d = tof(v1);
            let mut v2d = tof(v2);
            // Sort v1d, v2d, shorter first.
            let swap = l2v1 > l2v2;
            if swap {
                std::mem::swap(&mut v1d, &mut v2d);
                std::mem::swap(&mut l2v1, &mut l2v2);
            }
            let lv1 = l2v1.sqrt();
            let lv2 = l2v2.sqrt();
            // Bisector between v1 and v2.
            let bisector = (v1d.0 / lv1 + v2d.0 / lv2, v1d.1 / lv1 + v2d.1 / lv2);
            let l2bisector = norm2f(bisector);
            // Squared distance of the end point of v1 to the bisector.
            let d2 = l2v1 - dotf(v1d, bisector).powi(2) / l2bisector;
            let clip2f = clip_dist_scaled2 as f64;
            if d2 < foot_dist_min2 {
                // Height of the p1, p0, p2 triangle is tiny. Just remove p1.
            } else if d2 < 0.25 * clip2f + SCALED_EPSILON as f64 {
                // The shorter vector is too close to the bisector. Trim the
                // shorter vector fully, trim the longer vector partially.
                // Intersection of a circle at p2 of radius = clip_dist_scaled
                // with a ray (p1, p0), take the intersection after the foot
                // point. The intersection shall always exist because
                // |p2 - p1| > clip_dist_scaled.
                let b = -2.0 * dotf(v1d, v2d);
                let u = b * b - 4.0 * l2v2 * (l2v1 - clip2f);
                if u > 0.0 {
                    // Take the second intersection along v2.
                    let t = (-b + u.sqrt()) / (2.0 * l2v2);
                    if t > 0.0 && t < 1.0 {
                        let pt_new = offset_point(p1_pt, (t * v2d.0, t * v2d.1));
                        ring.insert(it2, pt_new);
                    }
                }
            } else {
                // Cut the corner with a line perpendicular to the bisector.
                let t = (0.25 * clip2f / d2).sqrt();
                let t2 = t * lv1 / lv2;
                let mut pa = offset_point(p1_pt, (v1d.0 * t, v1d.1 * t));
                let mut pb = offset_point(p1_pt, (v2d.0 * t2, v2d.1 * t2));
                if swap {
                    std::mem::swap(&mut pa, &mut pb);
                }
                let inserted = ring.insert(it2, pb);
                ring.insert(inserted, pa);
            }
        } else {
            // |p2 - p0| is within an epsilon of the clipping distance. Just
            // remove p1.
        }
    }

    if ring.size() == 3 {
        // Check whether the last triangle is clockwise oriented (it is a hole)
        // and its height is below clip_dist_scaled. If so, fill in the hole.
        let b = ring.begin();
        let p0 = ring.vpt(ring.prev(b));
        let p1 = ring.vpt(b);
        let p2 = ring.vpt(ring.next(b));
        let mut v1 = vsub(p0, p1);
        let mut v2 = vsub(p2, p1);
        if cross2(v1, v2) > 0 {
            // CW triangle. Measure its height.
            let v3 = vsub(p2, p0);
            let mut l12 = norm2(v1);
            let mut l22 = norm2(v2);
            let l32 = norm2(v3);
            if l22 > l12 && l22 > l32 {
                std::mem::swap(&mut v1, &mut v2);
                std::mem::swap(&mut l12, &mut l22);
            } else if l32 > l12 && l32 > l22 {
                v1 = v3;
                l12 = l32;
            }
            let h2 = l22 as f64 - (dot(v1, v2) as f64).powi(2) / l12 as f64;
            if h2 < clip_dist_scaled2 as f64 {
                // CW triangle with a low height. Close the hole.
                ring.clear();
            }
        }
    } else if ring.size() < 3 {
        ring.clear();
    }
}

/// Port of canonical `static bool clip_narrow_corner(...)` (Cura's
/// `smooth_corner_complex()`).
///
/// A concave corner at `it1` with position `p1` has been removed by the caller
/// between `it0` and `it2`, where `|p2 - p0| < shortcut_length`. Close the
/// concave crack by walking left from `it0` and right from `it2` as long as
/// the new clipping edge stays shorter than `shortcut_length` and remains a
/// diagonal of the polygon. Returns `true` if the ring was completely closed
/// or reduced to a single CCW triangle, which is not to be simplified further.
#[allow(clippy::too_many_arguments)]
fn clip_narrow_corner(
    p1: V,
    it0: &mut i32,
    it2: &mut i32,
    ring: &mut Ring,
    unprocessed: &mut Range,
    mut dist2_current: i128,
    shortcut_length: i128,
) -> bool {
    let shortcut_length2 = shortcut_length * shortcut_length;

    let mut forward = Status::Free;
    let mut backward = Status::Free;

    let mut p0 = ring.vpt(*it0);
    let mut p2 = ring.vpt(*it2);
    let mut p02: V = (0, 0);
    let mut p22: V = (0, 0);
    let mut dist2_next: i128 = 0;

    // As long as there is at least a single triangle left in the polygon.
    while ring.size() >= 3 {
        if forward == Status::Far && backward == Status::Far {
            p02 = ring.vpt(ring.prev(*it0));
            p22 = ring.vpt(ring.next(*it2));
            let d2 = norm2(vsub(p22, p02));
            if d2 <= shortcut_length2 {
                // The region was narrow until now and it is still narrow.
                // Trim at both sides.
                let after = unprocessed.remove_back(ring, *it0);
                *it0 = ring.prev(after);
                *it2 = unprocessed.remove_front(ring, *it2);
                if ring.size() <= 2 {
                    // A hole degenerated to an empty polygon.
                    return true;
                }
                forward = Status::Free;
                backward = Status::Free;
                dist2_current = d2;
                p0 = p02;
                p2 = p22;
            } else {
                // The region is widening. Stop traversal and trim the final
                // trapezoid.
                dist2_next = d2;
                break;
            }
        } else if forward != Status::Free && backward != Status::Free {
            // One of the corners is blocked, the other is blocked or too far.
            break;
        } else if forward == Status::Free
            && (backward != Status::Free || norm2(vsub(p2, p1)) < norm2(vsub(p0, p1)))
        {
            // Try to proceed by flipping a diagonal forward.
            p22 = ring.vpt(ring.next(*it2));
            if cross2(vsub(p2, p0), vsub(p22, p0)) > 0 {
                forward = Status::Blocked;
            } else {
                // New clipping edge length.
                let d2 = norm2(vsub(p22, p0));
                if d2 > shortcut_length2 {
                    forward = Status::Far;
                    dist2_next = d2;
                } else {
                    forward = Status::Free;
                    // Make one step in the forward direction.
                    *it2 = unprocessed.remove_front(ring, *it2);
                    p2 = p22;
                    dist2_current = d2;
                }
            }
        } else {
            // backward == Free
            p02 = ring.vpt(ring.prev(*it0));
            if cross2(vsub(p02, p2), vsub(p0, p2)) > 0 {
                backward = Status::Blocked;
            } else {
                // New clipping edge length.
                let d2 = norm2(vsub(p2, p02));
                if d2 > shortcut_length2 {
                    backward = Status::Far;
                    dist2_next = d2;
                } else {
                    backward = Status::Free;
                    // Make one step in the backward direction.
                    let after = unprocessed.remove_back(ring, *it0);
                    *it0 = ring.prev(after);
                    p0 = p02;
                    dist2_current = d2;
                }
            }
        }
    }

    if ring.size() <= 3 {
        // A hole degenerated to an empty polygon, or a tiny triangle remained.
        if ring.size() < 3 || (forward == Status::Far && backward == Status::Far) {
            ring.clear();
        }
        // Otherwise the remaining triangle is CCW oriented; keep it.
        return true;
    }

    let eps_short = shortcut_length - SCALED_EPSILON;
    let eps_short2 = eps_short * eps_short;
    if (forward == Status::Blocked && backward == Status::Blocked) || dist2_current > eps_short2 {
        // The crack is filled, keep the last clipping edge.
    } else if dist2_next < eps_short2 {
        // To avoid creating tiny edges.
        if forward == Status::Far {
            let after = unprocessed.remove_back(ring, *it0);
            *it0 = ring.prev(after);
        }
        if backward == Status::Far {
            *it2 = unprocessed.remove_front(ring, *it2);
        }
        if ring.size() <= 2 {
            // A hole degenerated to an empty polygon.
            return true;
        }
    } else if forward == Status::Blocked || backward == Status::Blocked {
        // One side is far, the other blocked. Sort, so we will clip the 1st
        // edge: `(pa, pa2)` is the far edge, `pb` the blocked end point, and
        // `target` the iterator sitting on `pa`.
        //
        // NOTE: canonical assigns the offset to `(backward == Far ? *it2 :
        // *it0)`, i.e. to the *opposite* iterator; see the deviation note at
        // the top of the module.
        let (pa, pa2, pb, target) = if forward == Status::Far {
            (p2, p22, p0, *it2)
        } else {
            (p0, p02, p2, *it0)
        };
        // Find the point on (pa, pa2) at distance shortcut_length from pb. The
        // circle intersects the line at two points; because
        // |pb - pa| < shortcut_length only the second intersection is valid,
        // and because |pb - pa2| > shortcut_length it always lies on the
        // segment.
        let v = tof(vsub(pa2, pa));
        let d = tof(vsub(pa, pb));
        let a = norm2f(v);
        let b = 2.0 * dotf(d, v);
        let u = b * b - 4.0 * a * (norm2f(d) - shortcut_length2 as f64);
        if u > 0.0 && a > 0.0 {
            let t = (-b + u.sqrt()) / (2.0 * a);
            if t > 0.0 && t < 1.0 {
                let moved = offset_point(ring.pt(target), (v.0 * t, v.1 * t));
                ring.set_pt(target, moved);
            }
        }
    } else {
        // The trapezoid (it0.prev(), it0, it2, it2.next()) is widening. Trim it.
        let dcurrent = (dist2_current as f64).sqrt();
        let denom = (dist2_next as f64).sqrt() - dcurrent;
        if denom > 0.0 {
            let t = (shortcut_length as f64 - dcurrent) / denom;
            let d0 = tof(vsub(p02, p0));
            let d2v = tof(vsub(p22, p2));
            let n0 = offset_point(ring.pt(*it0), (d0.0 * t, d0.1 * t));
            let n2 = offset_point(ring.pt(*it2), (d2v.0 * t, d2v.1 * t));
            ring.set_pt(*it0, n0);
            ring.set_pt(*it2, n2);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon_ops::difference_ex;

    /// 1 mm in scaled units (1 unit = 100 nm).
    const MM: i64 = 10_000;

    fn poly(pts: &[(i64, i64)]) -> Polygon {
        Polygon {
            points: pts.iter().map(|&(x, y)| Point2 { x, y }).collect(),
        }
    }

    fn ex(contour: &[(i64, i64)], holes: &[&[(i64, i64)]]) -> ExPolygon {
        ExPolygon {
            contour: poly(contour),
            holes: holes.iter().map(|h| poly(h)).collect(),
        }
    }

    fn signed_area(points: &[Point2]) -> f64 {
        if points.len() < 3 {
            return 0.0;
        }
        let mut acc: i128 = 0;
        for i in 0..points.len() {
            let j = (i + 1) % points.len();
            acc += points[i].x as i128 * points[j].y as i128
                - points[j].x as i128 * points[i].y as i128;
        }
        acc as f64 * 0.5
    }

    fn total_area(polys: &[ExPolygon]) -> f64 {
        polys
            .iter()
            .map(|e| {
                signed_area(&e.contour.points).abs()
                    - e.holes
                        .iter()
                        .map(|h| signed_area(&h.points).abs())
                        .sum::<f64>()
            })
            .sum()
    }

    /// 10 mm x 10 mm L-shape, CCW, with exactly one reflex vertex at
    /// (5 mm, 5 mm).
    fn l_shape() -> ExPolygon {
        ex(
            &[
                (0, 0),
                (10 * MM, 0),
                (10 * MM, 5 * MM),
                (5 * MM, 5 * MM),
                (5 * MM, 10 * MM),
                (0, 10 * MM),
            ],
            &[],
        )
    }

    /// The property that makes canonical `smooth_outward` safe as an interface
    /// regularizer: for rings large relative to `clip_dist_scaled`, the
    /// boundary only ever moves into void. `input - output` must be empty.
    ///
    /// Canonical is **not** a superset in general — a ring small enough to
    /// degenerate is deleted outright (see
    /// [`tiny_contour_is_deleted_not_grown`]) — so this sweep stays inside the
    /// regime canonical's own call sites use: a clipping edge well under the
    /// feature size.
    #[test]
    fn output_contains_input_for_features_larger_than_the_clip_distance() {
        // A many-notch star: 4 reflex corners.
        let star = ex(
            &[
                (0, 0),
                (4 * MM, 0),
                (5 * MM, 2 * MM),
                (6 * MM, 0),
                (10 * MM, 0),
                (10 * MM, 10 * MM),
                (6 * MM, 10 * MM),
                (5 * MM, 8 * MM),
                (4 * MM, 10 * MM),
                (0, 10 * MM),
            ],
            &[],
        );
        // 20 mm square with a CW diamond hole: every hole corner is concave
        // through the material, so every one of them is a clipping candidate.
        let holed = ex(
            &[(0, 0), (20 * MM, 0), (20 * MM, 20 * MM), (0, 20 * MM)],
            &[&[
                (10 * MM, 5 * MM),
                (5 * MM, 10 * MM),
                (10 * MM, 15 * MM),
                (15 * MM, 10 * MM),
            ]],
        );
        let square = ex(
            &[(0, 0), (10 * MM, 0), (10 * MM, 10 * MM), (0, 10 * MM)],
            &[],
        );

        for input in [l_shape(), star, holed, square] {
            for clip in [MM / 20, MM / 4, MM, 2 * MM] {
                let out = smooth_outward_ex(&input, clip);
                assert!(
                    !out.contour.points.is_empty(),
                    "contour must survive clip={clip}"
                );
                let lost = difference_ex(std::slice::from_ref(&input), &[out.clone()]);
                assert!(
                    total_area(&lost) == 0.0,
                    "smooth_outward moved the boundary INWARD (clip={clip}): \
                     lost {} units^2 across {} residue polygon(s)",
                    total_area(&lost),
                    lost.len()
                );
                assert!(
                    total_area(&[out]) >= total_area(&[input.clone()]),
                    "region shrank (clip={clip})"
                );
            }
        }
    }

    /// The canonical geometry of the "cut the corner with a line perpendicular
    /// to the bisector" branch, worked through by hand for a 90-degree reflex
    /// corner with equal arms.
    ///
    /// `clip_dist_scaled` is the **chord**: the corner vertex is removed and
    /// replaced by exactly two vertices, each `clip / sqrt(2)` back along its
    /// arm, so that the new clipping edge is exactly `clip` long. An earlier
    /// revision of this module treated `clip_dist_scaled` as the per-arm
    /// setback, which cuts `sqrt(2)` times too much.
    #[test]
    fn ninety_degree_corner_is_chamfered_with_a_chord_of_clip_dist() {
        let clip = 5000_i64; // 0.5 mm
        let corner = Point2 {
            x: 5 * MM,
            y: 5 * MM,
        };
        let out = smooth_outward_ex(&l_shape(), clip);
        let pts = &out.contour.points;

        assert!(
            !pts.contains(&corner),
            "the reflex vertex at (5mm, 5mm) must be removed, got {pts:?}"
        );
        // One corner clipped => 5 original vertices + 2 new ones.
        assert_eq!(pts.len(), 7, "expected a two-point chamfer, got {pts:?}");

        let setback = clip as f64 / std::f64::consts::SQRT_2; // 3535.53 units
        let expect_a = Point2 {
            x: corner.x + setback as i64,
            y: corner.y,
        };
        let expect_b = Point2 {
            x: corner.x,
            y: corner.y + setback as i64,
        };
        assert!(pts.contains(&expect_a), "missing {expect_a:?} in {pts:?}");
        assert!(pts.contains(&expect_b), "missing {expect_b:?} in {pts:?}");

        // The inserted clipping edge is `clip` long (up to lattice rounding).
        let chord = (((expect_a.x - expect_b.x) as f64).powi(2)
            + ((expect_a.y - expect_b.y) as f64).powi(2))
        .sqrt();
        assert!(
            (chord - clip as f64).abs() <= 2.0,
            "clipping edge is {chord} units, expected the chord {clip}"
        );
    }

    /// Canonical only clips a concave corner when the angle between the two
    /// arms is **sharper than 135 degrees** (`dot > 0 || sqr(dot) * 2 <
    /// l2v1 * l2v2`). A gentler notch is left byte-for-byte alone, even though
    /// it is concave and the clip distance is large enough to cut it.
    ///
    /// An earlier revision of this module used a 0.05 rad (~2.9 degree)
    /// threshold, which clips essentially every concave corner.
    #[test]
    fn concave_corner_flatter_than_135_degrees_is_left_alone() {
        // A CCW box whose top edge dips down into the material at mid-span.
        // Arms of height `h` over a half-span `dx` subtend 2*atan(dx/h)
        // between them, so h = dx*tan(10 deg) gives 160 degrees and
        // h = dx*tan(30 deg) gives 120 degrees.
        let dx = 10 * MM;
        let notch = |h: i64| {
            poly(&[
                (0, -(10 * MM)),
                (2 * dx, -(10 * MM)),
                (2 * dx, 0),
                (dx, -h),
                (0, 0),
            ])
        };

        let h_gentle = (dx as f64 * (10.0_f64).to_radians().tan()) as i64;
        let gentle = notch(h_gentle);
        assert_eq!(
            smooth_outward_polygon(&gentle, MM),
            gentle,
            "a 160-degree concave corner is flatter than canonical's 135-degree \
             gate and must survive untouched"
        );

        // The same ring with the notch sharpened past the gate IS clipped.
        let h_sharp = (dx as f64 * (30.0_f64).to_radians().tan()) as i64;
        let sharp = notch(h_sharp);
        let out = smooth_outward_polygon(&sharp, MM);
        assert!(
            !out.points.contains(&Point2 { x: dx, y: -h_sharp }),
            "a 120-degree concave corner is sharper than the gate and must be \
             clipped, got {:?}",
            out.points
        );
    }

    /// A purely convex ring has no candidate corners — `cross2(p0 - p1,
    /// p2 - p1)` is negative everywhere — so it comes back identical.
    #[test]
    fn convex_ring_is_untouched() {
        let square = poly(&[(0, 0), (10 * MM, 0), (10 * MM, 10 * MM), (0, 10 * MM)]);
        assert_eq!(smooth_outward_polygon(&square, MM), square);
    }

    /// Canonical `MutablePolygon::polygon()` emits points only when `valid()`
    /// (`size() >= 3`), so a ring with fewer than three points comes back
    /// **empty**, not passed through. A collinear triple has no concave corner
    /// and survives.
    #[test]
    fn degenerate_rings_follow_canonical_validity() {
        let two_pt = poly(&[(0, 0), (MM, 0)]);
        assert!(
            smooth_outward_polygon(&two_pt, MM).points.is_empty(),
            "a 2-point ring is not `valid()` upstream and must emit nothing"
        );
        let zero_area = poly(&[(0, 0), (MM, 0), (2 * MM, 0)]);
        assert_eq!(smooth_outward_polygon(&zero_area, MM), zero_area);
        // A zero clipping distance is geometrically a no-op: canonical takes
        // the "cut perpendicular to the bisector" branch with `t == 0`, so the
        // removed corner vertex is re-inserted twice at its own position
        // (upstream's `assert(t > 0. && t < 1.)` fires in a debug build; a
        // release build emits the duplicates). The region is unchanged.
        let zero = smooth_outward_polygon(&l_shape().contour, 0);
        assert_eq!(
            signed_area(&zero.points),
            signed_area(&l_shape().contour.points),
            "clip_dist_scaled = 0 must not change the region"
        );
        let mut deduped = zero.points.clone();
        deduped.dedup();
        assert_eq!(deduped, l_shape().contour.points);
    }

    /// Canonical selects corners purely by the sign of
    /// `cross2(p0 - p1, p2 - p1)` and never inspects winding: it assumes the
    /// Slic3r convention (contour CCW, hole CW). Feed it a ring wound the
    /// other way and it clips the corners that are convex through the
    /// material — i.e. it shrinks the region. This test pins that precondition
    /// rather than papering over it; an earlier revision of this module
    /// derived the material side from the signed area, which upstream does
    /// not do.
    #[test]
    fn winding_convention_is_a_precondition_not_an_inference() {
        let mut reversed = l_shape().contour;
        reversed.points.reverse();
        let out = smooth_outward_polygon(&reversed, 5000);
        assert!(
            signed_area(&out.points).abs() < signed_area(&reversed.points).abs(),
            "a CW contour has its convex corners clipped, shrinking it"
        );
        // The (now convex-through-material) reflex vertex survives; the true
        // convex corners do not.
        assert!(out.points.contains(&Point2 {
            x: 5 * MM,
            y: 5 * MM
        }));
        assert!(!out.points.contains(&Point2 { x: 0, y: 0 }));
    }

    /// A narrow crack is closed by `clip_narrow_corner`, which swallows
    /// neighbouring vertices rather than inserting a chamfer. A slot narrower
    /// than the clipping edge disappears entirely.
    #[test]
    fn narrow_slot_is_closed_by_clip_narrow_corner() {
        // 10 mm square with a 0.2 mm wide, 4 mm deep slot cut into the top.
        let slotted = poly(&[
            (0, 0),
            (10 * MM, 0),
            (10 * MM, 10 * MM),
            (51 * MM / 10, 10 * MM),
            (51 * MM / 10, 6 * MM),
            (49 * MM / 10, 6 * MM),
            (49 * MM / 10, 10 * MM),
            (0, 10 * MM),
        ]);
        let out = smooth_outward_polygon(&slotted, MM);
        let slot_bottom = Point2 {
            x: 51 * MM / 10,
            y: 6 * MM,
        };
        assert!(
            !out.points.contains(&slot_bottom),
            "the slot floor must be clipped away, got {:?}",
            out.points
        );
        assert!(
            signed_area(&out.points) > signed_area(&slotted.points),
            "closing a slot grows the region"
        );
    }

    /// Canonical deletes a ring that degenerates while clipping — the
    /// `polygon.clear()` paths in `clip_narrow_corner` and in the trailing
    /// "CW triangle with a low height" check. `smooth_outward` is therefore
    /// **not** a strict superset of its input, and the wrapper drops the
    /// emptied [`ExPolygon`].
    #[test]
    fn tiny_contour_is_deleted_not_grown() {
        // A 1 mm sliver triangle, clipped with a 5 mm edge.
        let sliver = ex(&[(0, 0), (MM, 0), (0, MM)], &[]);
        let out = smooth_outward(std::slice::from_ref(&sliver), 5 * MM);
        // Whatever survives, it must not be a partially-clipped ring: canonical
        // either leaves the triangle alone or clears it.
        for e in &out {
            assert!(e.contour.points.len() >= 3);
        }
        // A CW sliver (a hole shape) is cleared outright: its single candidate
        // corner sends it through `clip_narrow_corner`.
        let cw_sliver = poly(&[(0, 0), (0, MM), (MM, 0)]);
        assert!(
            smooth_outward_polygon(&cw_sliver, 5 * MM).points.is_empty(),
            "a CW sliver smaller than the clipping edge is a hole to be filled"
        );
    }
}

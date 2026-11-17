//! Exact axis-aligned rectangle and rectilinear-region reference helpers.
//!
//! No floating point: rectangles, cells and offsets are integer canonical
//! units, membership is a closed-form comparison, and every area is a checked
//! `i128` double area (twice the area, in squared units) so it compares
//! directly with #2 `signed_area2`.

use slicer_ir::{ExPolygon, Point2, Polygon};

use super::bounds::BoundsError;
use super::envelopes::whole_mm_to_units;

/// A nonempty axis-aligned rectangle `[x0, x1] × [y0, y1]` in canonical units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x0: i64,
    /// Bottom edge.
    pub y0: i64,
    /// Right edge.
    pub x1: i64,
    /// Top edge.
    pub y1: i64,
}

impl Rect {
    /// A rectangle with strictly positive width and height.
    pub fn new(x0: i64, y0: i64, x1: i64, y1: i64) -> Result<Self, String> {
        if x0 < x1 && y0 < y1 {
            Ok(Self { x0, y0, x1, y1 })
        } else {
            Err(format!(
                "rectangle [{x0},{x1}]x[{y0},{y1}] has no positive area"
            ))
        }
    }

    /// A rectangle whose edges are whole millimetres, converted exactly.
    pub fn from_mm(x0: i64, y0: i64, x1: i64, y1: i64) -> Result<Self, BoundsError> {
        let rect = Self::new(
            whole_mm_to_units(x0)?,
            whole_mm_to_units(y0)?,
            whole_mm_to_units(x1)?,
            whole_mm_to_units(y1)?,
        )
        .map_err(|what| BoundsError::Arithmetic { what })?;
        Ok(rect)
    }

    /// Counter-clockwise contour starting at the lower-left corner.
    pub fn ring(&self) -> Polygon {
        Polygon {
            points: vec![
                Point2 {
                    x: self.x0,
                    y: self.y0,
                },
                Point2 {
                    x: self.x1,
                    y: self.y0,
                },
                Point2 {
                    x: self.x1,
                    y: self.y1,
                },
                Point2 {
                    x: self.x0,
                    y: self.y1,
                },
            ],
        }
    }

    /// Clockwise ring, for use as a hole.
    pub fn hole_ring(&self) -> Polygon {
        let mut ring = self.ring();
        ring.points.reverse();
        ring
    }

    /// The rectangle as a hole-free `ExPolygon`.
    pub fn to_expolygon(self) -> ExPolygon {
        ExPolygon {
            contour: self.ring(),
            holes: Vec::new(),
        }
    }

    /// Closed-rectangle containment of an open cell.
    pub fn contains_cell(&self, cell: &Cell) -> bool {
        self.x0 <= cell.x0 && cell.x1 <= self.x1 && self.y0 <= cell.y0 && cell.y1 <= self.y1
    }

    /// Checked double area in squared units.
    pub fn double_area(&self) -> Result<i128, String> {
        double_area(self.x0, self.y0, self.x1, self.y1)
    }

    /// Translate by `(dx, dy)` with checked arithmetic.
    pub fn translated(&self, dx: i64, dy: i64) -> Result<Self, String> {
        let add = |v: i64, d: i64| {
            v.checked_add(d)
                .ok_or_else(|| String::from("rectangle translation overflow"))
        };
        Self::new(
            add(self.x0, dx)?,
            add(self.y0, dy)?,
            add(self.x1, dx)?,
            add(self.y1, dy)?,
        )
    }
}

/// Checked `2 * (x1 - x0) * (y1 - y0)` in `i128`.
pub fn double_area(x0: i64, y0: i64, x1: i64, y1: i64) -> Result<i128, String> {
    let width = i128::from(x1)
        .checked_sub(i128::from(x0))
        .ok_or_else(|| String::from("width overflow"))?;
    let height = i128::from(y1)
        .checked_sub(i128::from(y0))
        .ok_or_else(|| String::from("height overflow"))?;
    width
        .checked_mul(height)
        .and_then(|area| area.checked_mul(2))
        .ok_or_else(|| String::from("double area overflow"))
}

/// One open cell `(x0, x1) × (y0, y1)` of an edge partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Left partition coordinate.
    pub x0: i64,
    /// Right partition coordinate.
    pub x1: i64,
    /// Bottom partition coordinate.
    pub y0: i64,
    /// Top partition coordinate.
    pub y1: i64,
}

impl Cell {
    /// The cell midpoint in doubled coordinates, `(x0 + x1, y0 + y1)`.
    ///
    /// Always an exact integer; it is a grid point at the true midpoint only
    /// when both sums are even (see [`Cell::midpoint`]).
    pub fn doubled_midpoint(&self) -> Result<(i128, i128), String> {
        let x = i128::from(self.x0) + i128::from(self.x1);
        let y = i128::from(self.y0) + i128::from(self.y1);
        Ok((x, y))
    }

    /// The exact grid midpoint, when the doubled midpoint is even on both axes.
    pub fn midpoint(&self) -> Option<Point2> {
        let (x, y) = self.doubled_midpoint().ok()?;
        if x % 2 != 0 || y % 2 != 0 {
            return None;
        }
        Some(Point2 {
            x: i64::try_from(x / 2).ok()?,
            y: i64::try_from(y / 2).ok()?,
        })
    }

    /// Checked double area of the cell.
    pub fn double_area(&self) -> Result<i128, String> {
        double_area(self.x0, self.y0, self.x1, self.y1)
    }
}

/// Sorted, deduplicated coordinates.
pub fn sorted_unique(mut values: Vec<i64>) -> Vec<i64> {
    values.sort_unstable();
    values.dedup();
    values
}

/// Partition the bounding box of the given edges into open cells.
///
/// `xs`/`ys` must be strictly increasing; consecutive pairs bound each cell.
pub fn cell_partition(xs: &[i64], ys: &[i64]) -> Result<Vec<Cell>, String> {
    if xs.windows(2).any(|w| w[0] >= w[1]) || ys.windows(2).any(|w| w[0] >= w[1]) {
        return Err(String::from("partition edges must be strictly increasing"));
    }
    let mut cells = Vec::new();
    for x in xs.windows(2) {
        for y in ys.windows(2) {
            cells.push(Cell {
                x0: x[0],
                x1: x[1],
                y0: y[0],
                y1: y[1],
            });
        }
    }
    Ok(cells)
}

/// Checked sum of cell double areas.
pub fn cells_double_area<'a>(cells: impl IntoIterator<Item = &'a Cell>) -> Result<i128, String> {
    cells.into_iter().try_fold(0_i128, |sum, cell| {
        sum.checked_add(cell.double_area()?)
            .ok_or_else(|| String::from("cell area sum overflow"))
    })
}

/// The four Boolean set operations and their membership truth tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    /// `A ∪ B`.
    Union,
    /// `A ∩ B`.
    Intersection,
    /// `A \ B`.
    Difference,
    /// `A ⊕ B`.
    Xor,
}

impl BoolOp {
    /// All four operations, in a fixed order.
    pub const ALL: [Self; 4] = [Self::Union, Self::Intersection, Self::Difference, Self::Xor];

    /// Membership of a point in the result, given its membership in A and B.
    pub fn truth(self, in_a: bool, in_b: bool) -> bool {
        match self {
            Self::Union => in_a || in_b,
            Self::Intersection => in_a && in_b,
            Self::Difference => in_a && !in_b,
            Self::Xor => in_a != in_b,
        }
    }
}

/// Unit outward-for-material normal of an axis-aligned edge `a -> b`.
///
/// The right-hand normal `(sgn dy, -sgn dx)` points away from the material
/// for both a CCW contour (material on the left) and a CW hole (material on
/// the left, hole interior on the right), so moving every edge by `+d` along
/// it dilates the material by `d` and moving by `-d` erodes it.
fn right_normal(a: Point2, b: Point2) -> Result<(i64, i64), String> {
    match (a.x == b.x, a.y == b.y) {
        (true, false) => Ok(((b.y - a.y).signum(), 0)),
        (false, true) => Ok((0, -(b.x - a.x).signum())),
        _ => Err(format!(
            "edge {a:?}->{b:?} is not a nonzero axis-aligned edge"
        )),
    }
}

/// Offset one rectilinear ring by `d` units along material-outward normals.
///
/// Exact for rectilinear rings with no collinear consecutive vertices: every
/// corner moves by `d * (n_in + n_out)`, the miter (L∞) corner of the two
/// shifted edges. The caller must keep `|d|` below half of every feature width
/// and clearance; the result is re-validated by the caller with #2
/// `validate_ring`.
pub fn offset_rectilinear_ring(ring: &Polygon, d: i64) -> Result<Polygon, String> {
    let n = ring.points.len();
    if n < 4 {
        return Err(String::from(
            "a rectilinear ring needs at least four vertices",
        ));
    }
    let mut points = Vec::with_capacity(n);
    for index in 0..n {
        let prev = ring.points[(index + n - 1) % n];
        let here = ring.points[index];
        let next = ring.points[(index + 1) % n];
        let n_in = right_normal(prev, here)?;
        let n_out = right_normal(here, next)?;
        if n_in == n_out || (n_in.0 == -n_out.0 && n_in.1 == -n_out.1) {
            return Err(format!("vertex {here:?} is not a 90-degree corner"));
        }
        let shift = |coordinate: i64, normal_sum: i64| {
            normal_sum
                .checked_mul(d)
                .and_then(|delta| coordinate.checked_add(delta))
                .ok_or_else(|| String::from("offset coordinate overflow"))
        };
        points.push(Point2 {
            x: shift(here.x, n_in.0 + n_out.0)?,
            y: shift(here.y, n_in.1 + n_out.1)?,
        });
    }
    Ok(Polygon { points })
}

/// Offset a rectilinear polygon set by `d` (dilate for `d > 0`, erode for
/// `d < 0`): contours and holes both move along their material normals.
pub fn offset_rectilinear(polygons: &[ExPolygon], d: i64) -> Result<Vec<ExPolygon>, String> {
    polygons
        .iter()
        .map(|polygon| {
            Ok(ExPolygon {
                contour: offset_rectilinear_ring(&polygon.contour, d)?,
                holes: polygon
                    .holes
                    .iter()
                    .map(|hole| offset_rectilinear_ring(hole, d))
                    .collect::<Result<_, String>>()?,
            })
        })
        .collect()
}

/// Scale every coordinate by `factor` with checked arithmetic.
pub fn scale_polygons(polygons: &[ExPolygon], factor: i64) -> Result<Vec<ExPolygon>, String> {
    let scale_ring = |ring: &Polygon| -> Result<Polygon, String> {
        Ok(Polygon {
            points: ring
                .points
                .iter()
                .map(|p| {
                    Ok(Point2 {
                        x: p.x
                            .checked_mul(factor)
                            .ok_or_else(|| String::from("scale overflow"))?,
                        y: p.y
                            .checked_mul(factor)
                            .ok_or_else(|| String::from("scale overflow"))?,
                    })
                })
                .collect::<Result<_, String>>()?,
        })
    };
    polygons
        .iter()
        .map(|polygon| {
            Ok(ExPolygon {
                contour: scale_ring(&polygon.contour)?,
                holes: polygon
                    .holes
                    .iter()
                    .map(scale_ring)
                    .collect::<Result<_, String>>()?,
            })
        })
        .collect()
}

/// Translate every coordinate by `(dx, dy)` with checked arithmetic.
pub fn translate_polygons(
    polygons: &[ExPolygon],
    dx: i64,
    dy: i64,
) -> Result<Vec<ExPolygon>, String> {
    let move_ring = |ring: &Polygon| -> Result<Polygon, String> {
        Ok(Polygon {
            points: ring
                .points
                .iter()
                .map(|p| {
                    Ok(Point2 {
                        x: p.x
                            .checked_add(dx)
                            .ok_or_else(|| String::from("translation overflow"))?,
                        y: p.y
                            .checked_add(dy)
                            .ok_or_else(|| String::from("translation overflow"))?,
                    })
                })
                .collect::<Result<_, String>>()?,
        })
    };
    polygons
        .iter()
        .map(|polygon| {
            Ok(ExPolygon {
                contour: move_ring(&polygon.contour)?,
                holes: polygon
                    .holes
                    .iter()
                    .map(move_ring)
                    .collect::<Result<_, String>>()?,
            })
        })
        .collect()
}

/// Every ring of a polygon set.
pub fn rings(polygons: &[ExPolygon]) -> impl Iterator<Item = &Polygon> {
    polygons
        .iter()
        .flat_map(|polygon| std::iter::once(&polygon.contour).chain(polygon.holes.iter()))
}

/// Every edge `(a, b)` of every ring of a polygon set.
pub fn edges(polygons: &[ExPolygon]) -> Vec<(Point2, Point2)> {
    let mut out = Vec::new();
    for ring in rings(polygons) {
        let n = ring.points.len();
        for index in 0..n {
            out.push((ring.points[index], ring.points[(index + 1) % n]));
        }
    }
    out
}

/// Whether every edge of every ring is axis-aligned.
pub fn is_rectilinear(polygons: &[ExPolygon]) -> bool {
    edges(polygons)
        .iter()
        .all(|(a, b)| a.x == b.x || a.y == b.y)
}

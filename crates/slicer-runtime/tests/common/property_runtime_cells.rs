//! Independent finite cell-label model for the H-RUNTIME geometry charters.
//!
//! This file is compiled twice on purpose:
//!
//! - as a target-private helper of `tests/property_runtime_contracts.rs`, which
//!   falsifies the comparator with known-wrong outputs, and
//! - through `#[path = "../../tests/common/property_runtime_cells.rs"]` from the
//!   `region_partition::property_tests` inline module, so the modifier and fill
//!   partition properties compare against exactly this comparator.
//!
//! Mounting one file from both homes is deliberate. A second copy would let the
//! AC-N2 comparator control certify code the properties never call.
//!
//! The model never calls `slicer_core::polygon_ops`, never asks production for an
//! expected result, and never samples random interior points. It:
//!
//! 1. collects every orthogonal boundary coordinate from the inputs AND the
//!    observed output, so a cut the output invented is a grid line rather than a
//!    region the sampled cells can miss;
//! 2. classifies a doubled-coordinate midpoint in every elementary open cell with
//!    the independent `point_in_region` predicate, which keeps half-unit
//!    midpoints exact;
//! 3. compares the observed output's cell membership and exact double area
//!    against the independently labelled cell set.
//!
//! The grid product is bounded before allocation; exceeding the bound is an
//! explicit failed diagnostic, never a correctness pass.

#![allow(dead_code)]

use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::test_support::property_testing::oracles::{
    point_in_region, signed_area2, PointLocation,
};

/// Hard ceiling on elementary cells for one comparison (packet domain design).
pub const MAX_CELLS: u64 = 4096;

/// Hard floor on elementary cells, so an empty witness cannot pass silently.
pub const MIN_CELLS: u64 = 16;

/// Hard ceiling on `max_abs_units`: coordinates stay inside 2^20 canonical units.
pub const MAX_ABS_UNITS: u64 = 1 << 20;

/// Hard floor on `max_abs_units`; a smaller bound cannot carry a useful witness.
pub const MIN_ABS_UNITS: u64 = 16;

/// Comparator or domain failure with the concrete witness that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellError {
    /// Stable classification for controls and diagnostics.
    pub kind: &'static str,
    /// Human-readable witness detail naming the offending cell or bound.
    pub message: String,
}

impl CellError {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

/// Axis-aligned rectangle in canonical units, half-open on the maximum edge.
///
/// Both corners must be finite and ordered. Construction performs checked
/// arithmetic only; it never narrows a wider integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rect {
    /// Minimum x coordinate (exclusive minimum edge).
    pub x0: i64,
    /// Minimum y coordinate (exclusive minimum edge).
    pub y0: i64,
    /// Maximum x coordinate (exclusive maximum edge).
    pub x1: i64,
    /// Maximum y coordinate (exclusive maximum edge).
    pub y1: i64,
}

impl Rect {
    /// Construct an ordered, non-degenerate rectangle.
    pub fn new(x0: i64, y0: i64, x1: i64, y1: i64) -> Result<Self, CellError> {
        if x0 >= x1 || y0 >= y1 {
            return Err(CellError::new(
                "degenerate_rect",
                format!("rect ({x0},{y0})-({x1},{y1}) must have positive width and height"),
            ));
        }
        Ok(Self { x0, y0, x1, y1 })
    }

    /// Doubled coordinate pair used for exact midpoint classification.
    ///
    /// The sum is the doubled midpoint of the half-open interval, so a
    /// half-unit boundary pair still yields an integer query coordinate.
    pub fn doubled_midpoint(&self) -> (i64, i64) {
        (
            self.x0.checked_add(self.x1).unwrap_or(i64::MAX),
            self.y0.checked_add(self.y1).unwrap_or(i64::MAX),
        )
    }

    /// Area in squared canonical units as `i128` (no narrowing).
    pub fn area2(&self) -> i128 {
        2 * (i128::from(self.x1) - i128::from(self.x0))
            * (i128::from(self.y1) - i128::from(self.y0))
    }

    /// Convert to a counter-clockwise `ExPolygon` with no holes.
    pub fn to_expolygon(self) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
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
            },
            holes: Vec::new(),
        }
    }
}

/// Build one orthogonal outer rectangle with rectangular holes.
///
/// Holes must lie strictly inside the outer rectangle and must not overlap each
/// other; a violating request is rejected rather than sent to the SUT as
/// invented invalid geometry.
pub fn holed_rect(outer: Rect, holes: &[Rect]) -> Result<ExPolygon, CellError> {
    for (index, hole) in holes.iter().enumerate() {
        if hole.x0 <= outer.x0 || hole.x1 >= outer.x1 || hole.y0 <= outer.y0 || hole.y1 >= outer.y1
        {
            return Err(CellError::new(
                "hole_outside_outer",
                format!("hole {index} {hole:?} is not strictly inside outer {outer:?}"),
            ));
        }
        for other in holes.iter().skip(index + 1) {
            let disjoint = hole.x1 <= other.x0
                || other.x1 <= hole.x0
                || hole.y1 <= other.y0
                || other.y1 <= hole.y0;
            if !disjoint {
                return Err(CellError::new(
                    "overlapping_holes",
                    format!("holes {hole:?} and {other:?} have overlapping interiors"),
                ));
            }
        }
    }
    let mut polygon = outer.to_expolygon();
    polygon.holes = holes
        .iter()
        .map(|hole| Polygon {
            // Clockwise, matching the IR's hole convention.
            points: vec![
                Point2 {
                    x: hole.x0,
                    y: hole.y0,
                },
                Point2 {
                    x: hole.x0,
                    y: hole.y1,
                },
                Point2 {
                    x: hole.x1,
                    y: hole.y1,
                },
                Point2 {
                    x: hole.x1,
                    y: hole.y0,
                },
            ],
        })
        .collect();
    Ok(polygon)
}

/// Convert a disjoint rectangle field into single-rectangle `ExPolygon`s.
///
/// Rectangles may share edges (a concave orthogonal union) but must not share
/// interior area; overlap is rejected because the domain declares disjoint
/// carriers and the independent point predicate would otherwise see an
/// ambiguous union.
pub fn rect_field(rects: &[Rect]) -> Result<Vec<ExPolygon>, CellError> {
    for (index, left) in rects.iter().enumerate() {
        for right in rects.iter().skip(index + 1) {
            if interiors_overlap(left, right) {
                return Err(CellError::new(
                    "overlapping_field_rects",
                    format!("field rects {left:?} and {right:?} share interior area"),
                ));
            }
        }
    }
    Ok(rects.iter().copied().map(Rect::to_expolygon).collect())
}

fn interiors_overlap(a: &Rect, b: &Rect) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

/// Doubled copies of the given polygons.
///
/// `point_in_region` is scale-invariant, so doubling both the boundary polygons
/// and the query point preserves membership while making every half-unit
/// midpoint an integer.
fn doubled(regions: &[ExPolygon]) -> Vec<ExPolygon> {
    regions
        .iter()
        .map(|expolygon| ExPolygon {
            contour: Polygon {
                points: expolygon
                    .contour
                    .points
                    .iter()
                    .map(|point| Point2 {
                        x: point.x.saturating_mul(2),
                        y: point.y.saturating_mul(2),
                    })
                    .collect(),
            },
            holes: expolygon
                .holes
                .iter()
                .map(|hole| Polygon {
                    points: hole
                        .points
                        .iter()
                        .map(|point| Point2 {
                            x: point.x.saturating_mul(2),
                            y: point.y.saturating_mul(2),
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

/// Checked coordinate doubling used by domain controls.
///
/// An input at the `i64` extremes cannot be doubled without overflow, so the
/// domain rejects it before any allocation or comparison.
pub fn checked_double(value: i64) -> Result<i64, CellError> {
    value
        .checked_mul(2)
        .ok_or_else(|| CellError::new("coordinate_overflow", format!("cannot double {value}")))
}

/// Validate a registry `max_abs_units` bound against the written safety ceiling.
pub fn checked_max_abs_units(raw: u64) -> Result<i64, CellError> {
    if !(MIN_ABS_UNITS..=MAX_ABS_UNITS).contains(&raw) {
        return Err(CellError::new(
            "max_abs_units_out_of_ceiling",
            format!("max_abs_units {raw} must be within {MIN_ABS_UNITS}..={MAX_ABS_UNITS}"),
        ));
    }
    i64::try_from(raw)
        .map_err(|_| CellError::new("max_abs_units_narrowing", format!("{raw} does not fit i64")))
}

/// Validate a registry `max_cells` bound against the written safety ceiling.
pub fn checked_max_cells(raw: u64) -> Result<usize, CellError> {
    if !(MIN_CELLS..=MAX_CELLS).contains(&raw) {
        return Err(CellError::new(
            "max_cells_out_of_ceiling",
            format!("max_cells {raw} must be within {MIN_CELLS}..={MAX_CELLS}"),
        ));
    }
    usize::try_from(raw)
        .map_err(|_| CellError::new("max_cells_narrowing", format!("{raw} does not fit usize")))
}

/// Sorted, unique elementary grid boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellGrid {
    xs: Vec<i64>,
    ys: Vec<i64>,
}

impl CellGrid {
    /// Build a grid from all boundary coordinates, rejecting an empty axis or an
    /// oversized product before allocating the cell vector.
    pub fn from_boundaries(
        xs: impl IntoIterator<Item = i64>,
        ys: impl IntoIterator<Item = i64>,
        max_cells: usize,
    ) -> Result<Self, CellError> {
        let mut xs: Vec<i64> = xs.into_iter().collect();
        let mut ys: Vec<i64> = ys.into_iter().collect();
        xs.sort_unstable();
        xs.dedup();
        ys.sort_unstable();
        ys.dedup();
        if xs.len() < 2 || ys.len() < 2 {
            return Err(CellError::new(
                "empty_witness_grid",
                format!(
                    "grid needs at least one cell; got {} x boundaries and {} y boundaries",
                    xs.len(),
                    ys.len()
                ),
            ));
        }
        let cells = (xs.len() - 1)
            .checked_mul(ys.len() - 1)
            .ok_or_else(|| CellError::new("grid_product_overflow", "cell count overflow"))?;
        if cells > max_cells {
            return Err(CellError::new(
                "cell_budget_exceeded",
                format!("grid has {cells} cells, budget {max_cells}"),
            ));
        }
        Ok(Self { xs, ys })
    }

    /// Grid built from every boundary of the given input and output polygons.
    pub fn from_region_boundaries<'a>(
        inputs: impl IntoIterator<Item = &'a ExPolygon>,
        outputs: impl IntoIterator<Item = &'a ExPolygon>,
        max_cells: usize,
    ) -> Result<Self, CellError> {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for expolygon in inputs.into_iter().chain(outputs) {
            for ring in std::iter::once(&expolygon.contour).chain(expolygon.holes.iter()) {
                for point in &ring.points {
                    xs.push(point.x);
                    ys.push(point.y);
                }
            }
        }
        Self::from_boundaries(xs, ys, max_cells)
    }

    /// Number of elementary cells.
    pub fn cell_count(&self) -> usize {
        (self.xs.len() - 1) * (self.ys.len() - 1)
    }

    /// X boundaries.
    pub fn xs(&self) -> &[i64] {
        &self.xs
    }

    /// Y boundaries.
    pub fn ys(&self) -> &[i64] {
        &self.ys
    }

    /// Describe the cell at `(column, row)` for diagnostics.
    pub fn cell_bounds(&self, column: usize, row: usize) -> Rect {
        Rect {
            x0: self.xs[column],
            y0: self.ys[row],
            x1: self.xs[column + 1],
            y1: self.ys[row + 1],
        }
    }

    fn columns(&self) -> usize {
        self.xs.len() - 1
    }
}

/// Bitset of elementary cells in column-major order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellSet {
    cells: Vec<bool>,
}

impl CellSet {
    /// Empty set of the given size.
    pub fn empty(size: usize) -> Self {
        Self {
            cells: vec![false; size],
        }
    }

    /// Full set of the given size.
    pub fn full(size: usize) -> Self {
        Self {
            cells: vec![true; size],
        }
    }

    /// Number of included cells.
    pub fn count(&self) -> usize {
        self.cells.iter().filter(|cell| **cell).count()
    }

    /// Whether no cell is included.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Set size.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Cell inclusion at `index` (column-major).
    pub fn contains(&self, index: usize) -> bool {
        self.cells.get(index).copied().unwrap_or(false)
    }

    /// Mutable cell inclusion for hand-built witness sets.
    ///
    /// Only the deterministic controls use this; the charter properties build
    /// sets from predicates so they cannot mutate a computed expectation.
    pub fn cells_mut(&mut self) -> &mut [bool] {
        &mut self.cells
    }

    /// Intersection.
    pub fn and(&self, other: &Self) -> Self {
        Self {
            cells: self
                .cells
                .iter()
                .zip(&other.cells)
                .map(|(left, right)| *left && *right)
                .collect(),
        }
    }

    /// Union.
    pub fn or(&self, other: &Self) -> Self {
        Self {
            cells: self
                .cells
                .iter()
                .zip(&other.cells)
                .map(|(left, right)| *left || *right)
                .collect(),
        }
    }

    /// Difference.
    pub fn sub(&self, other: &Self) -> Self {
        Self {
            cells: self
                .cells
                .iter()
                .zip(&other.cells)
                .map(|(left, right)| *left && !*right)
                .collect(),
        }
    }

    /// Complement against the grid universe.
    pub fn not(&self, universe: usize) -> Self {
        Self::full(universe).sub(self)
    }

    /// Whether every included cell is included in `other`.
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.cells
            .iter()
            .zip(&other.cells)
            .all(|(left, right)| !*left || *right)
    }

    /// Whether the two sets share no cell.
    pub fn is_disjoint_from(&self, other: &Self) -> bool {
        self.and(other).is_empty()
    }

    /// Exact double area of the included cells under `grid`.
    pub fn area2(&self, grid: &CellGrid) -> i128 {
        let columns = grid.columns();
        let mut total = 0_i128;
        for (index, included) in self.cells.iter().enumerate() {
            if *included {
                total += grid.cell_bounds(index % columns, index / columns).area2();
            }
        }
        total
    }

    /// First cell index where the two sets differ.
    pub fn first_difference(&self, other: &Self) -> Option<usize> {
        self.cells
            .iter()
            .zip(&other.cells)
            .position(|(left, right)| left != right)
    }

    /// Mark the cells classified as one of `regions` (midpoint membership).
    pub fn classify(grid: &CellGrid, regions: &[ExPolygon]) -> Result<Self, CellError> {
        let doubled_regions = doubled(regions);
        let columns = grid.columns();
        let mut cells = Vec::with_capacity(grid.cell_count());
        for row in 0..grid.ys().len() - 1 {
            for column in 0..columns {
                let bounds = grid.cell_bounds(column, row);
                let (x, y) = bounds.doubled_midpoint();
                let location = point_in_region(Point2 { x, y }, &doubled_regions)
                    .map_err(|error| CellError::new("point_predicate_error", error.to_string()))?;
                match location {
                    PointLocation::Inside => cells.push(true),
                    PointLocation::Outside => cells.push(false),
                    PointLocation::Boundary => {
                        return Err(CellError::new(
                            "midpoint_on_boundary",
                            format!(
                                "cell {bounds:?} midpoint lies on a ring; the grid omitted a boundary"
                            ),
                        ));
                    }
                }
            }
        }
        Ok(Self { cells })
    }
}

/// Exact double area of a polygon set, holes subtracted by absolute value.
pub fn region_area2(regions: &[ExPolygon]) -> Result<i128, CellError> {
    let mut total = 0_i128;
    for expolygon in regions {
        let contour = signed_area2(&expolygon.contour)
            .map_err(|error| CellError::new("area_overflow", error.to_string()))?;
        total += contour.abs();
        for hole in &expolygon.holes {
            let area = signed_area2(hole)
                .map_err(|error| CellError::new("area_overflow", error.to_string()))?;
            total -= area.abs();
        }
    }
    Ok(total)
}

/// Assert every output edge is orthogonal and its endpoints lie on `grid`.
///
/// A diagonal edge would cross cell interiors, where midpoint classification
/// cannot see it. Rejecting is the honest outcome; the domain is orthogonal and
/// a non-orthogonal output is a SUT defect, not a reason to sample harder.
pub fn assert_orthogonal_on_grid(regions: &[ExPolygon], grid: &CellGrid) -> Result<(), CellError> {
    let xs = grid.xs();
    let ys = grid.ys();
    for expolygon in regions {
        for (ring_index, ring) in std::iter::once(&expolygon.contour)
            .chain(expolygon.holes.iter())
            .enumerate()
        {
            if ring.points.len() < 3 {
                return Err(CellError::new(
                    "short_ring",
                    format!("ring {ring_index} has {} points", ring.points.len()),
                ));
            }
            for (index, start) in ring.points.iter().enumerate() {
                let end = &ring.points[(index + 1) % ring.points.len()];
                let horizontal = start.y == end.y && xs.binary_search(&start.x).is_ok();
                let vertical = start.x == end.x && ys.binary_search(&start.y).is_ok();
                if !horizontal && !vertical {
                    return Err(CellError::new(
                        "non_orthogonal_edge",
                        format!(
                            "ring {ring_index} edge ({},{})->({},{}) is neither horizontal nor \
                             vertical on the collected grid",
                            start.x, start.y, end.x, end.y
                        ),
                    ));
                }
                if !xs.binary_search(&end.x).is_ok() || !ys.binary_search(&end.y).is_ok() {
                    return Err(CellError::new(
                        "edge_off_grid",
                        format!(
                            "ring {ring_index} endpoint ({},{}) is not on the collected grid",
                            end.x, end.y
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Compare an observed polygon set against an independently labelled cell set.
///
/// Equality is cell membership plus exact double area, not contour order,
/// starting vertex or winding. `label` names the carrier in the diagnostic.
pub fn compare_region_to_cells(
    actual: &[ExPolygon],
    expected: &CellSet,
    grid: &CellGrid,
    label: &str,
) -> Result<(), CellError> {
    assert_orthogonal_on_grid(actual, grid)?;
    let actual_cells = CellSet::classify(grid, actual)?;
    if let Some(index) = actual_cells.first_difference(expected) {
        let columns = grid.columns();
        let bounds = grid.cell_bounds(index % columns, index / columns);
        return Err(CellError::new(
            "cell_membership_mismatch",
            format!(
                "{label}: cell {bounds:?} observed={} expected={} (index {index})",
                actual_cells.contains(index),
                expected.contains(index)
            ),
        ));
    }
    let observed_area = region_area2(actual)?;
    let expected_area = expected.area2(grid);
    if observed_area != expected_area {
        return Err(CellError::new(
            "cell_area_mismatch",
            format!("{label}: observed double area {observed_area}, expected {expected_area}"),
        ));
    }
    Ok(())
}

/// Assert the named parts are pairwise disjoint and their union is `expected`.
pub fn assert_disjoint_union(
    parts: &[(&str, &CellSet)],
    expected: &CellSet,
    universe: usize,
) -> Result<(), CellError> {
    let mut union = CellSet::empty(universe);
    for (index, (left_name, left)) in parts.iter().enumerate() {
        for (right_name, right) in parts.iter().skip(index + 1) {
            if !left.is_disjoint_from(right) {
                let overlap = left.and(right);
                return Err(CellError::new(
                    "roles_overlap",
                    format!(
                        "{left_name} and {right_name} share {} cell(s)",
                        overlap.count()
                    ),
                ));
            }
        }
        union = union.or(left);
    }
    if union != *expected {
        if let Some(index) = union.first_difference(expected) {
            return Err(CellError::new(
                "roles_do_not_cover_domain",
                format!("cell index {index} differs between union and expected domain"),
            ));
        }
    }
    Ok(())
}

/// Domain constructor used by the controls: rejects a bound pair whose product
/// or coordinate span cannot be modelled exactly.
pub fn checked_domain(
    max_abs_units: u64,
    max_cells: u64,
    span_units: u64,
) -> Result<(i64, usize), CellError> {
    let abs = checked_max_abs_units(max_abs_units)?;
    let cells = checked_max_cells(max_cells)?;
    if span_units == 0 {
        return Err(CellError::new(
            "empty_span",
            "a zero coordinate span cannot carry a witness",
        ));
    }
    if span_units > max_abs_units {
        return Err(CellError::new(
            "span_exceeds_bound",
            format!("span {span_units} exceeds max_abs_units {max_abs_units}"),
        ));
    }
    Ok((abs, cells))
}

/// Narrowing check used by the domain control: a `u64` that does not fit `i64`
/// must be rejected instead of wrapping.
pub fn checked_narrow_units(raw: u64) -> Result<i64, CellError> {
    i64::try_from(raw)
        .map_err(|_| CellError::new("units_narrowing", format!("{raw} does not fit i64")))
}

//! Polygon clipping and offset primitives.

use clipper2_rust::Point64;
use slicer_ir::{ExPolygon, Point2, Polygon};

/// Boolean clip operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipOperation {
    /// Union of all subject and clip polygons.
    Union,
    /// Intersection between subject and clip polygons.
    Intersection,
    /// Difference: subject minus clip.
    Difference,
    /// Exclusive-or between subject and clip polygons.
    Xor,
}

/// Join style used for polygon offsetting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetJoinType {
    /// Sharp corners.
    Miter,
    /// Rounded corners.
    Round,
    /// Squared corners.
    Square,
}

/// Converts a Polygon to a Path64 (Vec<Point64>) for clipper2-rust.
fn polygon_to_path(poly: &Polygon) -> Vec<Point64> {
    poly.points
        .iter()
        .map(|p| Point64 { x: p.x, y: p.y })
        .collect()
}

/// Converts an ExPolygon to a vector of paths (contour + holes)
/// for clipper2-rust.
fn expolygon_to_paths(exp: &ExPolygon) -> Vec<Vec<Point64>> {
    let mut paths = Vec::new();
    // Contour
    paths.push(polygon_to_path(&exp.contour));
    // Holes
    for hole in &exp.holes {
        paths.push(polygon_to_path(hole));
    }
    paths
}

/// Executes a boolean clip operation on polygon sets.
pub fn clip_polygons(
    subject: &[ExPolygon],
    clip: &[ExPolygon],
    op: ClipOperation,
) -> Vec<ExPolygon> {
    use clipper2_rust::core::FillRule;
    use clipper2_rust::{difference_64, intersect_64, union_64, xor_64};

    // Flatten all polygons (contours + holes) into separate paths
    // This is necessary because clipper2 operations work on flat path lists
    // Note: This approach loses hole-contour relationships
    let subject_paths: Vec<Vec<Point64>> = subject.iter().flat_map(expolygon_to_paths).collect();
    let clip_paths: Vec<Vec<Point64>> = clip.iter().flat_map(expolygon_to_paths).collect();

    let result_paths = match op {
        ClipOperation::Union => union_64(&subject_paths, &clip_paths, FillRule::NonZero),
        ClipOperation::Intersection => intersect_64(&subject_paths, &clip_paths, FillRule::NonZero),
        ClipOperation::Difference => difference_64(&subject_paths, &clip_paths, FillRule::NonZero),
        ClipOperation::Xor => xor_64(&subject_paths, &clip_paths, FillRule::NonZero),
    };

    // Convert result paths back to ExPolygon
    // Note: This simple conversion treats every path as a separate ExPolygon with no holes.
    // A full implementation would use PolyTree to reconstruct hierarchy.
    result_paths
        .into_iter()
        .map(|path| {
            // Reconstruct Polygon from Point64 vector
            let points: Vec<Point2> = path
                .into_iter()
                .map(|p| Point2 { x: p.x, y: p.y })
                .collect();
            ExPolygon {
                contour: Polygon { points },
                holes: Vec::new(),
            }
        })
        .collect()
}

/// Computes the union of polygon sets.
pub fn union(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    clip_polygons(subject, clip, ClipOperation::Union)
}

/// Computes the intersection of polygon sets.
pub fn intersection(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    clip_polygons(subject, clip, ClipOperation::Intersection)
}

/// Computes the difference of polygon sets.
pub fn difference(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    clip_polygons(subject, clip, ClipOperation::Difference)
}

/// Computes the exclusive-or of polygon sets.
pub fn xor(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    clip_polygons(subject, clip, ClipOperation::Xor)
}

/// Returned by [`validate_polygon_simplicity`] when a polygon fails the simplicity
/// check. `contour_indices` lists the indices of contours (outer = 0; holes = 1..)
/// that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolygonSimplicityError {
    /// Indices of the contours that failed the simplicity check.
    /// Index 0 is the outer contour; indices 1.. are holes in order.
    pub contour_indices: Vec<usize>,
}

/// Verify that every contour of `poly` is simple (no self-intersections, no
/// duplicate-vertex degeneracies that would break clipper2 set-ops). Wraps the
/// clipper2-rust simplicity primitive: a polygon-set passed through clipper2's
/// union with itself produces the same area iff every contour is simple. We use
/// the more direct route — re-running the input through clipper2's union and
/// comparing contour count + signed area per contour.
///
/// Returns `Ok(())` for simple polygons; `Err(PolygonSimplicityError { contour_indices })`
/// listing the failing contour indices when invalid.
pub fn validate_polygon_simplicity(poly: &ExPolygon) -> Result<(), PolygonSimplicityError> {
    use clipper2_rust::core::FillRule;
    use clipper2_rust::{area, union_64};

    // Epsilon for area comparison: 1.0 in workspace units² (1 unit = 100 nm).
    // Self-intersecting contours (e.g. bowties) lose significant area after
    // union cleans them, so 1.0 unit² is a conservative but safe threshold.
    const AREA_EPSILON: f64 = 1.0;

    // Build a flat iterator of (index, path) for outer contour + holes.
    let outer_path: Vec<Point64> = polygon_to_path(&poly.contour);
    let all_contours: Vec<(usize, Vec<Point64>)> = std::iter::once((0usize, outer_path))
        .chain(
            poly.holes
                .iter()
                .enumerate()
                .map(|(i, h)| (i + 1, polygon_to_path(h))),
        )
        .collect();

    let mut failing: Vec<usize> = Vec::new();

    for (idx, path) in &all_contours {
        let original_area = area(path).abs();

        // Self-union a single contour; a simple ring stays as one ring with
        // the same area. A bowtie or self-crossing ring splits into 2+ rings
        // or the area changes significantly.
        let subject: Vec<Vec<Point64>> = vec![path.clone()];
        let clip: Vec<Vec<Point64>> = Vec::new();
        let result = union_64(&subject, &clip, FillRule::NonZero);

        let changed = if result.len() != 1 {
            true
        } else {
            let result_area = area(&result[0]).abs();
            (result_area - original_area).abs() > AREA_EPSILON
        };

        if changed {
            failing.push(*idx);
        }
    }

    if failing.is_empty() {
        Ok(())
    } else {
        Err(PolygonSimplicityError {
            contour_indices: failing,
        })
    }
}

/// Offsets polygons by `delta_mm` millimeters.
pub fn offset(polygons: &[ExPolygon], delta_mm: f32, join: OffsetJoinType) -> Vec<ExPolygon> {
    use clipper2_rust::inflate_paths_64;
    use clipper2_rust::{EndType, JoinType};

    // Convert polygons to paths
    let paths: Vec<Vec<Point64>> = polygons.iter().flat_map(expolygon_to_paths).collect();

    // Convert delta from mm to scaled units (1 unit = 100nm = 10^-4mm)
    // Scaling factor is 10_000
    let delta_units = (delta_mm * 10_000.0) as f64;

    // Map OffsetJoinType to clipper2_rust JoinType
    let join_type = match join {
        OffsetJoinType::Miter => JoinType::Miter,
        OffsetJoinType::Round => JoinType::Round,
        OffsetJoinType::Square => JoinType::Square,
    };

    // Execute offset operation
    // inflate_paths_64 signature: inflate_paths_64(&paths, delta, join_type, end_type, miter_limit, arc_tolerance)
    // We use EndType::Polygon for closed polygon offsetting
    // miter_limit and arc_tolerance can be defaults (2.0 and 0.0)
    let result_paths = inflate_paths_64(&paths, delta_units, join_type, EndType::Polygon, 2.0, 0.0);

    // Convert result paths back to ExPolygon
    // Note: Same limitation as clip_polygons - treats every path as separate ExPolygon with no holes
    result_paths
        .into_iter()
        .map(|path| {
            let points: Vec<Point2> = path
                .into_iter()
                .map(|p| Point2 { x: p.x, y: p.y })
                .collect();
            ExPolygon {
                contour: Polygon { points },
                holes: Vec::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{validate_polygon_simplicity, ClipOperation};
    use slicer_ir::{ExPolygon, Point2, Polygon};

    #[test]
    fn clip_operation_variants_are_distinct() {
        assert_ne!(ClipOperation::Union, ClipOperation::Difference);
    }

    fn make_polygon(pts: &[(i64, i64)]) -> Polygon {
        Polygon {
            points: pts.iter().map(|&(x, y)| Point2 { x, y }).collect(),
        }
    }

    #[test]
    fn validate_polygon_simplicity_accepts_simple_square() {
        // A 10x10 square (in workspace units: 10 units per side).
        let square = ExPolygon {
            contour: make_polygon(&[(0, 0), (10, 0), (10, 10), (0, 10)]),
            holes: Vec::new(),
        };
        assert!(validate_polygon_simplicity(&square).is_ok());
    }

    #[test]
    fn validate_polygon_simplicity_rejects_bowtie() {
        // Bowtie: self-intersecting quad (0,0) → (10,10) → (10,0) → (0,10) → back.
        // The crossing at the centre causes clipper2's union to split it.
        let bowtie = ExPolygon {
            contour: make_polygon(&[(0, 0), (10, 10), (10, 0), (0, 10)]),
            holes: Vec::new(),
        };
        let result = validate_polygon_simplicity(&bowtie);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contour_indices.contains(&0),
            "outer contour (index 0) must be flagged"
        );
    }
}

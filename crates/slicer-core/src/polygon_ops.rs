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
    use super::ClipOperation;

    #[test]
    fn clip_operation_variants_are_distinct() {
        assert_ne!(ClipOperation::Union, ClipOperation::Difference);
    }
}

//! Core geometry algorithms for ModularSlicer.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod aabb_lines_2d;
pub mod aabb_tree;
/// Pure algorithm modules (available only with the `host-algos` feature).
#[cfg(feature = "host-algos")]
pub mod algos;
pub mod paint_region;
pub mod polygon_ops;
pub mod stage_io;
pub mod triangle_mesh_slicer;

use slicer_ir::{Point2, Point3, Point3WithWidth};

pub use aabb_tree::{AabbTree, ClosestPointHit, RayHit};
pub use paint_region::{
    ex_polygon_contains_point, point_in_paint_region, BoundaryInclusion, PaintRegionQueryError,
    PaintRegionRTreeEntry, PaintRegionRTreeIndex,
};
pub use polygon_ops::{
    clip_polygons, difference, intersection, offset, union, xor, ClipOperation, OffsetJoinType,
};
pub use stage_io::{
    FacetAnnotationRecord, FacetClassRecord, MeshAnalysisAuxiliary, PrepassStageOutput,
    SurfaceGroupRecord,
};
pub use triangle_mesh_slicer::slice_mesh_ex;

/// Applies a 4×4 column-major transform `matrix` to point `p`.
///
/// Canonical replacement for the per-module copies that previously lived in the
/// runtime (`mesh_analysis`, `paint_segmentation`, `model_loader`,
/// `prepass_slice`). It carries both behaviours those copies needed:
///
/// - **Zero-matrix guard**: an all-zero `matrix` is treated as identity (`p` is
///   returned unchanged), keeping fixtures that leave `Transform3d::matrix`
///   unset robust rather than collapsing geometry to the origin.
/// - **Perspective divide**: the homogeneous `w` component divides the result.
///   For the affine object transforms used throughout the slicer `w == 1`, so
///   the divide is a no-op; it is retained so the function is correct for any
///   well-formed 4×4 transform.
///
/// Indexing is column-major: element at column `c`, row `r` is `matrix[c * 4 + r]`.
pub fn transform_point3(matrix: &[f64; 16], p: Point3) -> Point3 {
    if matrix.iter().all(|v| *v == 0.0) {
        return p;
    }
    let x = f64::from(p.x);
    let y = f64::from(p.y);
    let z = f64::from(p.z);
    let tx = matrix[0] * x + matrix[4] * y + matrix[8] * z + matrix[12];
    let ty = matrix[1] * x + matrix[5] * y + matrix[9] * z + matrix[13];
    let tz = matrix[2] * x + matrix[6] * y + matrix[10] * z + matrix[14];
    let tw = matrix[3] * x + matrix[7] * y + matrix[11] * z + matrix[15];
    let w = if tw == 0.0 { 1.0 } else { tw };
    Point3 {
        x: (tx / w) as f32,
        y: (ty / w) as f32,
        z: (tz / w) as f32,
    }
}

#[cfg(test)]
mod transform_point3_tests {
    use super::*;

    /// Column-major diagonal scale `s` with translation `t` (affine, `w == 1`).
    fn affine(s: [f64; 3], t: [f64; 3]) -> [f64; 16] {
        [
            s[0], 0.0, 0.0, 0.0, //
            0.0, s[1], 0.0, 0.0, //
            0.0, 0.0, s[2], 0.0, //
            t[0], t[1], t[2], 1.0,
        ]
    }

    #[test]
    fn affine_scale_translate_matches_hand_computation() {
        let m = affine([2.0, 3.0, 4.0], [10.0, -5.0, 1.0]);
        let out = transform_point3(
            &m,
            Point3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        );
        assert_eq!(out.x, (2.0 * 1.0 + 10.0) as f32);
        assert_eq!(out.y, (3.0 * 2.0 - 5.0) as f32);
        assert_eq!(out.z, (4.0 * 3.0 + 1.0) as f32);
    }

    #[test]
    fn zero_matrix_is_treated_as_identity() {
        let p = Point3 {
            x: 1.5,
            y: -2.5,
            z: 7.0,
        };
        let out = transform_point3(&[0.0; 16], p);
        assert_eq!((out.x, out.y, out.z), (p.x, p.y, p.z));
    }

    #[test]
    fn identity_transform_w_divide_is_noop() {
        let mut m = [0.0; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m[15] = 1.0;
        let p = Point3 {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };
        let out = transform_point3(&m, p);
        assert_eq!((out.x, out.y, out.z), (p.x, p.y, p.z));
    }
}

/// Segments a straight 2D path into points whose consecutive spacing does not exceed `max_len_mm`.
pub fn segment_path(start: Point2, end: Point2, max_len_mm: f32) -> Vec<Point2> {
    if start == end {
        return vec![start];
    }

    let (start_x, start_y) = start.to_mm();
    let (end_x, end_y) = end.to_mm();
    let dx = end_x - start_x;
    let dy = end_y - start_y;
    let length = (dx * dx + dy * dy).sqrt();

    if !max_len_mm.is_finite() || max_len_mm <= 0.0 || length <= max_len_mm {
        return vec![start, end];
    }

    let segment_count = (length / max_len_mm).ceil() as usize;
    let mut points = Vec::with_capacity(segment_count + 1);

    for index in 0..=segment_count {
        if index == 0 {
            points.push(start);
            continue;
        }

        if index == segment_count {
            points.push(end);
            continue;
        }

        let t = (index as f32) / (segment_count as f32);
        points.push(Point2::from_mm(start_x + (dx * t), start_y + (dy * t)));
    }

    points
}

/// Computes the total 3D arc length of a point sequence in millimeters.
pub fn path_length(points: &[Point3WithWidth]) -> f32 {
    points
        .windows(2)
        .map(|segment| {
            seg_len_3d(
                segment[1].x - segment[0].x,
                segment[1].y - segment[0].y,
                segment[1].z - segment[0].z,
            )
        })
        .sum()
}

/// Distributes `count` evenly spaced samples along a polyline in millimeters.
pub fn distribute_points(points: &[Point3WithWidth], count: usize) -> Vec<Point3WithWidth> {
    if count == 0 || points.is_empty() {
        return Vec::new();
    }

    if count == 1 {
        return vec![points[0]];
    }

    if points.len() == 1 {
        return vec![points[0]; count];
    }

    let total_length = path_length(points);
    if total_length <= f32::EPSILON {
        return vec![points[0]; count];
    }

    let last_point = points[points.len() - 1];
    let step = total_length / ((count - 1) as f32);
    let mut samples = Vec::with_capacity(count);
    let mut segment_start_index = 0usize;
    let mut traversed = 0.0_f32;

    for sample_index in 0..count {
        if sample_index == 0 {
            samples.push(points[0]);
            continue;
        }

        if sample_index == count - 1 {
            samples.push(last_point);
            continue;
        }

        let target = step * (sample_index as f32);

        loop {
            let start = points[segment_start_index];
            let end = points[segment_start_index + 1];
            let segment_length = seg_len_3d(end.x - start.x, end.y - start.y, end.z - start.z);

            if segment_length <= f32::EPSILON {
                if segment_start_index + 1 >= points.len() - 1 {
                    samples.push(end);
                    break;
                }
                segment_start_index += 1;
                continue;
            }

            let segment_end_distance = traversed + segment_length;
            if target <= segment_end_distance {
                let t = ((target - traversed) / segment_length).clamp(0.0, 1.0);
                samples.push(interpolate_point(start, end, t));
                break;
            }

            traversed = segment_end_distance;
            segment_start_index += 1;

            if segment_start_index + 1 >= points.len() {
                samples.push(last_point);
                break;
            }
        }
    }

    samples
}

/// Computes the Euclidean length of a 3D segment in millimeters.
pub fn seg_len_3d(dx: f32, dy: f32, dz: f32) -> f32 {
    (dx.mul_add(dx, dy.mul_add(dy, dz * dz))).sqrt()
}

/// Computes a finite extrusion-flow correction factor for a non-planar segment.
pub fn flow_correction(dx: f32, dy: f32, dz: f32) -> f32 {
    let planar_length = (dx.mul_add(dx, dy * dy)).sqrt();
    if planar_length <= f32::EPSILON {
        return 1.0;
    }

    let corrected = seg_len_3d(dx, dy, dz) / planar_length;
    if corrected.is_finite() && corrected > 0.0 {
        corrected
    } else {
        1.0
    }
}

fn interpolate_point(start: Point3WithWidth, end: Point3WithWidth, t: f32) -> Point3WithWidth {
    Point3WithWidth {
        x: start.x + ((end.x - start.x) * t),
        y: start.y + ((end.y - start.y) * t),
        z: start.z + ((end.z - start.z) * t),
        width: start.width + ((end.width - start.width) * t),
        flow_factor: start.flow_factor + ((end.flow_factor - start.flow_factor) * t),
        overhang_quartile: start.overhang_quartile,
    }
}

#[cfg(test)]
mod tests {
    use super::{flow_correction, segment_path};
    use slicer_ir::Point2;

    #[test]
    fn segment_path_preserves_requested_endpoints() {
        let points = segment_path(Point2::from_mm(0.0, 0.0), Point2::from_mm(2.0, 0.0), 0.75);

        assert_eq!(points.first(), Some(&Point2::from_mm(0.0, 0.0)));
        assert_eq!(points.last(), Some(&Point2::from_mm(2.0, 0.0)));
    }

    #[test]
    fn flow_correction_stays_positive_for_vertical_input() {
        assert!(flow_correction(0.0, 0.0, 1.0).is_sign_positive());
    }
}

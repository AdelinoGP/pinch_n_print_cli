//! Core geometry algorithms for ModularSlicer.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod polygon_ops;
pub mod triangle_mesh_slicer;

use slicer_ir::{Point2, Point3WithWidth};

pub use polygon_ops::{
    clip_polygons,
    difference,
    intersection,
    offset,
    union,
    xor,
    ClipOperation,
    OffsetJoinType,
};
pub use triangle_mesh_slicer::slice_mesh_ex;

/// Segments a straight 2D path into points whose consecutive spacing does not exceed `max_len_mm`.
pub fn segment_path(_start: Point2, _end: Point2, _max_len_mm: f32) -> Vec<Point2> {
    todo!("TASK-013 geometry helpers not implemented")
}

/// Computes the total 3D arc length of a point sequence in millimeters.
pub fn path_length(_points: &[Point3WithWidth]) -> f32 {
    todo!("TASK-013 geometry helpers not implemented")
}

/// Distributes `count` evenly spaced samples along a polyline in millimeters.
pub fn distribute_points(_points: &[Point3WithWidth], _count: usize) -> Vec<Point3WithWidth> {
    todo!("TASK-013 geometry helpers not implemented")
}

/// Computes the Euclidean length of a 3D segment in millimeters.
pub fn seg_len_3d(_dx: f32, _dy: f32, _dz: f32) -> f32 {
    todo!("TASK-013 geometry helpers not implemented")
}

/// Computes a finite extrusion-flow correction factor for a non-planar segment.
pub fn flow_correction(_dx: f32, _dy: f32, _dz: f32) -> f32 {
    todo!("TASK-013 geometry helpers not implemented")
}

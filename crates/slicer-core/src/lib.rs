//! Core geometry algorithms for ModularSlicer.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod polygon_ops;

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

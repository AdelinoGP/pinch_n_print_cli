//! Common imports for module authoring.

pub use crate::coords::{mm_to_units, units_to_mm, SCALING_FACTOR};
pub use crate::host;
pub use crate::host::{ClipOperation, LogLevel, OffsetJoinType};

pub use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, InfillIR, InfillRegion,
    PaintSemantic, PaintValue, PerimeterIR, PerimeterRegion, Point2, Point3, Point3WithWidth,
    Polygon, RegionId, SliceIR, SlicedRegion, WallFeatureFlags, WallLoop,
};

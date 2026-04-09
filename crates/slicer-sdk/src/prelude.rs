//! Common imports for module authoring.

pub use crate::coords::{mm_to_units, units_to_mm, SCALING_FACTOR};
pub use crate::host;
pub use crate::host::{ClipOperation, LogLevel, OffsetJoinType};

// Module traits and types
pub use crate::builders::{
    InfillOutputBuilder, PerimeterOutputBuilder, SlicePostprocessBuilder, SupportOutputBuilder,
};
pub use crate::error::ModuleError;
pub use crate::traits::{LayerModule, PaintRegionLayerView, PostpassModule, PrepassModule};
pub use crate::views::{PerimeterRegionView, SliceRegionView};

// Postpass types and builders
pub use crate::postpass_builders::{GcodeMoveCmd, GcodeOutputBuilder};
pub use crate::postpass_types::{GcodeCommandKind, GcodeCommandView};

// Prepass types and builders
pub use crate::prepass_builders::{LayerPlanOutput, MeshAnalysisOutput};
pub use crate::prepass_types::{
    FacetAnnotation, FacetClass, LayerProposal, ObjectId, RegionId as PrepassRegionId,
    RegionLayerProposal, SurfaceGroupProposal,
};

// IR re-exports
pub use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, InfillIR, InfillRegion,
    PaintSemantic, PaintValue, PerimeterIR, PerimeterRegion, Point2, Point3, Point3WithWidth,
    Polygon, RegionId, RegionKey, SliceIR, SlicedRegion, WallFeatureFlags, WallLoop,
};

//! Common imports for module authoring.

// Authoring macros re-exported so `use slicer_sdk::prelude::*;` brings
// `#[slicer_module]` / `#[module_test]` into scope (docs/05 §module SDK).
pub use slicer_macros::{module_test, slicer_module};

pub use crate::coords::{mm_to_units, units_to_mm, SCALING_FACTOR};
pub use crate::host;
pub use crate::host::{ClipOperation, LogLevel, OffsetJoinType};

// Module traits and types
pub use crate::builders::{
    InfillOutputBuilder, PerimeterOutputBuilder, SlicePostprocessBuilder, SupportOutputBuilder,
};
pub use crate::error::ModuleError;
pub use crate::layer_collection_builder::LayerCollectionBuilder;
pub use crate::traits::{
    FinalizationModule, FinalizationOutputBuilder, LayerCollectionView, LayerModule,
    PaintRegionLayerView, PostpassModule, PrepassModule,
};
pub use crate::views::{PerimeterRegionView, SliceRegionView};

// Postpass types and builders
pub use crate::postpass_builders::{GcodeMoveCmd, GcodeOutputBuilder};
pub use crate::postpass_types::{GcodeCommand, GcodeOutputCommand};

// Prepass types and builders
pub use crate::prepass_builders::{
    LayerPlanOutput, MeshAnalysisOutput, MeshSegmentationOutput, ObjectMeshModification,
    PaintRegionEntry, PaintSegmentationOutput, SeamPlanningOutput, SupportGenerationOutput,
    TrianglePaintMark,
};
pub use crate::prepass_types::{
    FacetAnnotation, FacetClass, LayerProposal, MeshObjectView, ObjectId, PaintLayerView,
    PaintSegmentationObjectView, PaintStrokeView, PaintValueView, RegionId as PrepassRegionId,
    RegionLayerProposal, ScoredSeamCandidate, SeamPlanEntry, SeamReason, SupportPlanEntry,
    SurfaceGroupProposal,
};

// IR re-exports
pub use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole, InfillIR, InfillRegion,
    PaintSemantic, PaintValue, PerimeterIR, PerimeterRegion, Point2, Point3, Point3WithWidth,
    Polygon, RegionId, RegionKey, SliceIR, SlicedRegion, WallFeatureFlags, WallLoop,
};

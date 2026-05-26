//! Intermediate Representation (IR) Schemas for ModularSlicer
//!
//! All IR types are shared contracts between the host and modules.
//! Every IR struct carries a `schema_version: SemVer`.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod entity_id;
pub mod polygon_predicate;
pub mod resolved_config;
pub mod slice_ir;
pub mod validation;

pub use entity_id::LayerEntityIdGen;
pub use polygon_predicate::{point_in_contour_winding, point_in_polygon_winding};
pub use resolved_config::{ConfigResolutionError, ResolvedConfig};
pub use validation::validate_travel_anchors;

pub use slice_ir::{
    ActiveRegion,
    BoundingBox3,
    BridgeRegion,
    BridgeRegionId,
    ConfigDelta,

    // Config types
    ConfigKey,
    ConfigValue,
    ConfigView,

    ExPolygon,
    ExtrusionPath3D,
    ExtrusionRole,

    FacetClass,
    // Paint types
    FacetPaintData,
    FacetPaintMark,
    GCodeCommand,
    // GCode types
    GCodeIR,
    GlobalLayer,
    IndexedTriangleSet,
    // Infill types
    InfillIR,
    InfillRegion,

    // Infill type
    InfillType,

    // Layer collection types
    LayerAnnotation,
    LayerAnnotationKind,
    LayerCollectionIR,
    LayerPaintMap,
    // Layer planning types
    LayerPlanIR,
    LoopType,
    MeshIR,
    MeshSegmentationIR,

    ModifierId,
    ModifierScope,
    // Modifier types
    ModifierVolume,
    ModuleId,
    ModuleInvocation,

    NonPlanarShellRef,
    ObjectConfig,
    // ID types
    ObjectId,
    ObjectLayerRef,

    ObjectMesh,
    ObjectSurfaceData,
    OverhangRegion,

    OverhangRegionId,
    PaintLayer,
    // Paint region types
    PaintRegionIR,
    PaintSemantic,
    PaintStroke,

    PaintValue,
    // Perimeter types
    PerimeterIR,
    PerimeterRegion,
    // Basic types
    Point2,
    Point3,
    Point3WithWidth,
    Polygon,

    PrintEntity,
    PrintMetadata,

    RegionId,
    RegionKey,
    // Region map types
    RegionMapIR,
    RegionPlan,
    RetractMode,
    ScoredSeamCandidate,
    SeamCandidate,
    SeamPlanEntry,
    SeamPlanIR,
    SeamPosition,

    SeamReason,
    SemVer,
    SemanticRegion,

    // Slice types
    SliceIR,
    SlicedRegion,
    StageId,
    // Support types
    SupportGeometryIR,
    SupportGeometryKey,
    SupportIR,
    SupportPlanEntry,
    SupportPlanIR,
    SupportType,

    // Surface classification types
    SurfaceClassificationIR,
    SurfaceGroup,
    SurfaceGroupId,
    ToolChange,
    Transform3d,
    TravelMove,
    TravelRetract,
    WallBoundaryType,
    WallFeatureFlags,
    // Wall generator
    WallGenerator,

    WallLoop,
    WidthProfile,
    ZHop,
};

pub use slice_ir::{
    // Helper functions for Point2 coordinate conversion
    mm_to_units,
    units_to_mm,
    // Schema-version constants
    CURRENT_GCODE_IR_SCHEMA_VERSION,
    CURRENT_INFILL_IR_SCHEMA_VERSION,
    CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION,
    CURRENT_LAYER_PLAN_IR_SCHEMA_VERSION,
    CURRENT_MESH_IR_SCHEMA_VERSION,
    CURRENT_MESH_SEGMENTATION_IR_SCHEMA_VERSION,
    CURRENT_PAINT_REGION_IR_SCHEMA_VERSION,
    CURRENT_PERIMETER_IR_SCHEMA_VERSION,
    CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
    CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION,
    CURRENT_SLICE_IR_SCHEMA_VERSION,
    CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION,
    CURRENT_SUPPORT_IR_SCHEMA_VERSION,
    CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
    CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION,
};

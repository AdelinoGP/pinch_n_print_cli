//! Intermediate Representation (IR) Schemas for Pinch 'n Print
//!
//! All IR types are shared contracts between the host and modules.
//! Every IR struct carries a `schema_version: SemVer`.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod entity_id;
/// Feedrate computation and configuration.
pub mod feedrate;
pub mod polygon_predicate;
pub mod region_split_registry;
pub mod resolved_config;
pub mod slice_ir;
pub mod stage_io;
pub mod validation;

pub use entity_id::LayerEntityIdGen;
pub use feedrate::FeedrateConfig;
pub use polygon_predicate::{point_in_contour_winding, point_in_polygon_winding};
pub use resolved_config::{ConfigResolutionError, ResolvedConfig};
pub use stage_io::{
    BlackboardError, BlackboardPrepassSlot, Diagnostic, DiagnosticSeverity, FinalizationError,
    FinalizationOutput, LayerArenaError, LayerArenaSlot, LayerStageCommit, LayerStageError,
    LayerStageOutput, PathOptimizationCommit, PostpassError, PostpassOutput, PrepassRunnerError,
    RetractSpec, TravelMoveDest,
};
pub use validation::validate_travel_anchors;

pub use slice_ir::{
    ActiveRegion,
    BoundingBox3,
    BridgeRegion,
    BridgeRegionId,
    ConfigDelta,

    // Config types
    ConfigId,
    ConfigKey,
    ConfigValue,
    ConfigView,

    ExPolygon,
    ExtrusionJunction,
    ExtrusionLine,
    ExtrusionPath3D,
    ExtrusionRole,

    FacetClass,
    // Paint types
    FacetPaintData,
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
    // Layer planning types
    LayerPlanIR,
    LoopType,
    MaterialBoundarySegment,
    MeshIR,
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
    PaintSemantic,
    PaintStroke,

    PaintValue,
    // Perimeter types
    PerimeterIR,
    PerimeterRegion,
    // Basic types
    Point2,
    Point2WithWidth,
    Point3,
    Point3WithWidth,
    Polygon,
    PrintEntity,
    PrintMetadata,

    RegionId,
    RegionKey,
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
    ThickPolyline,

    ToolChange,
    Transform3d,
    TravelMove,
    TravelRetract,
    VariableWidthLines,
    WallBoundaryType,
    WallFeatureFlags,
    // Wall generator
    WallGenerator,

    WallLoop,
    WidthProfile,
    ZHop,
    // Region map types
    DEFAULT_REGION_MAP_CAP,
};

pub use slice_ir::{
    // ExtrusionLine -> ExtrusionPath3D conversion helper
    extrusion_line_to_extrusion_path3d,
    // Helper functions for Point2 coordinate conversion
    mm_to_units,
    units_to_mm,
    // Variable-width centerline helper
    variable_width,
    // Schema-version constants
    CURRENT_GCODE_IR_SCHEMA_VERSION,
    CURRENT_INFILL_IR_SCHEMA_VERSION,
    CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION,
    CURRENT_LAYER_PLAN_IR_SCHEMA_VERSION,
    CURRENT_MESH_IR_SCHEMA_VERSION,
    CURRENT_PERIMETER_IR_SCHEMA_VERSION,
    CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
    CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION,
    CURRENT_SLICE_IR_SCHEMA_VERSION,
    CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION,
    CURRENT_SUPPORT_IR_SCHEMA_VERSION,
    CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
    CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION,
    // Canonical mm→unit scaling factor (f64); use instead of raw 10_000.0 literals
    UNITS_PER_MM,
};

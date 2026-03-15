//! Intermediate Representation (IR) Schemas for ModularSlicer
//!
//! All IR types are shared contracts between the host and modules.
//! Every IR struct carries a `schema_version: SemVer`.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod slice_ir;

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
    LayerCollectionIR,
    LayerPaintMap,
    // Layer planning types
    LayerPlanIR,
    LoopType,
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
    ResolvedConfig,
    SeamCandidate,
    SeamPosition,

    SeamReason,
    SemVer,
    SemanticRegion,

    // Slice types
    SliceIR,
    SlicedRegion,
    StageId,
    // Support types
    SupportIR,
    SupportType,

    // Surface classification types
    SurfaceClassificationIR,
    SurfaceGroup,
    SurfaceGroupId,
    ToolChange,
    Transform3d,
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
};

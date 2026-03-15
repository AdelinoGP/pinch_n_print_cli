//! Intermediate Representation (IR) Schemas for ModularSlicer
//!
//! All IR types are shared contracts between the host and modules.
//! Every IR struct carries a `schema_version: SemVer`.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod slice_ir;

pub use slice_ir::{
    // Basic types
    Point2, Point3, BoundingBox3, Transform3d, SemVer,
    IndexedTriangleSet, ObjectMesh, MeshIR,
    
    // Paint types
    FacetPaintData, PaintLayer, PaintSemantic, PaintValue, PaintStroke,
    
    // Modifier types
    ModifierVolume, ModifierScope, ConfigDelta,
    
    // Config types
    ConfigKey, ConfigValue, ResolvedConfig, ObjectConfig, ConfigView,
    
    // Surface classification types
    SurfaceClassificationIR, ObjectSurfaceData, FacetClass, SurfaceGroup,
    BridgeRegion, OverhangRegion,
    
    // Layer planning types
    LayerPlanIR, GlobalLayer, ActiveRegion, NonPlanarShellRef, ObjectLayerRef,
    
    // Paint region types
    PaintRegionIR, LayerPaintMap, SemanticRegion,
    
    // Region map types
    RegionMapIR, RegionKey, RegionPlan, ModuleInvocation,
    
    // Slice types
    SliceIR, SlicedRegion, ExPolygon, Polygon,
    
    // Perimeter types
    PerimeterIR, PerimeterRegion, WallLoop, WallFeatureFlags, WallBoundaryType,
    LoopType, WidthProfile, ExtrusionPath3D, Point3WithWidth, SeamCandidate,
    SeamReason, SeamPosition,
    
    // Infill types
    InfillIR, InfillRegion,
    
    // Support types
    SupportIR, SupportType,
    
    // Layer collection types
    LayerCollectionIR, PrintEntity, ToolChange, ZHop, ExtrusionRole,
    
    // GCode types
    GCodeIR, GCodeCommand, PrintMetadata,
    
    // Wall generator
    WallGenerator,
    
    // Infill type
    InfillType,
    
    // ID types
    ObjectId, ModifierId, ModuleId, SurfaceGroupId, BridgeRegionId,
    OverhangRegionId, RegionId, StageId,
};

pub use slice_ir::{
    // Helper functions for Point2 coordinate conversion
    mm_to_units, units_to_mm,
};
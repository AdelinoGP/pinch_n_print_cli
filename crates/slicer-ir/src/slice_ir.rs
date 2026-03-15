//! Core IR type definitions
//!
//! All coordinate conversions follow the canonical rules:
//! - mm → units: `units = round(mm * 10_000.0)` (round half away from zero).
//! - units → mm: `mm = units / 10_000.0`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// ID Types
// ============================================================================

/// UUID string for objects
pub type ObjectId = String;

/// UUID string for modifiers
pub type ModifierId = String;

/// Reverse-domain module identifier (e.g., "com.example.module")
pub type ModuleId = String;

/// Surface group identifier
pub type SurfaceGroupId = u64;

/// Bridge region identifier
pub type BridgeRegionId = u64;

/// Overhang region identifier
pub type OverhangRegionId = u64;

/// Region identifier
pub type RegionId = u64;

/// Stage identifier
pub type StageId = String;

// ============================================================================
// Coordinate System and Basic Types
// ============================================================================

/// Convert millimeters to scaled integer units
/// 
/// # Arguments
/// * `mm` - Value in millimeters
/// 
/// # Returns
/// Scaled integer units (1 unit = 100 nm = 10^-4 mm)
#[inline]
pub fn mm_to_units(mm: f32) -> i64 {
    (mm * 10_000.0).round() as i64
}

/// Convert scaled integer units to millimeters
/// 
/// # Arguments
/// * `units` - Scaled integer units
/// 
/// # Returns
/// Value in millimeters
#[inline]
pub fn units_to_mm(units: i64) -> f32 {
    (units as f32) / 10_000.0
}

/// 2D point using scaled integer coordinates
/// 
/// Coordinate system: 1 unit = 100 nm = 10^-4 mm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point2 {
    pub x: i64,
    pub y: i64,
}

impl Point2 {
    /// Create from millimeter values
    pub fn from_mm(x: f32, y: f32) -> Self {
        Self {
            x: mm_to_units(x),
            y: mm_to_units(y),
        }
    }
    
    /// Convert to millimeter values
    pub fn to_mm(&self) -> (f32, f32) {
        (units_to_mm(self.x), units_to_mm(self.y))
    }
}

/// 3D point using floating-point millimeter coordinates
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// 3D bounding box
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox3 {
    pub min: Point3,
    pub max: Point3,
}

/// 3D transformation (column-major 4x4 matrix)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform3d {
    pub matrix: [f64; 16],
}

/// Indexed triangle set (vertices + indices)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexedTriangleSet {
    pub vertices: Vec<Point3>,
    pub indices: Vec<u32>,
}

/// Semantic versioning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ============================================================================
// Mesh IR Types
// ============================================================================

/// Raw user config (not yet resolved)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectConfig {
    // This is a placeholder - actual config fields would be populated
    // during config resolution
    pub data: HashMap<String, ConfigValue>,
}

/// Paint semantic types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaintSemantic {
    /// Which tool/filament to use for this surface region
    Material,
    /// Apply fuzzy skin texture to this surface region
    FuzzySkin,
    /// Force support generation in this region regardless of overhang angle
    SupportEnforcer,
    /// Block support generation in this region regardless of overhang angle
    SupportBlocker,
    /// Community-defined semantic
    Custom(String),
}

/// Paint value types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PaintValue {
    /// Boolean flag
    Flag(bool),
    /// Scalar value
    Scalar(f32),
    /// Tool/material index
    ToolIndex(u32),
}

/// Paint stroke (3D triangles defining painted region)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintStroke {
    pub triangles: Vec<[Point3; 3]>,
    pub semantic: PaintSemantic,
    pub value: PaintValue,
}

/// Paint layer (all paint with same semantic on one object)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintLayer {
    pub semantic: PaintSemantic,
    /// One entry per mesh triangle, parallel to mesh.triangles
    pub facet_values: Vec<Option<PaintValue>>,
    /// Sub-facet strokes that cross triangle boundaries
    pub strokes: Vec<PaintStroke>,
}

/// All paint layers on one object
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FacetPaintData {
    pub layers: Vec<PaintLayer>,
}

/// Config delta (only explicitly set fields)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigDelta {
    pub fields: HashMap<ConfigKey, ConfigValue>,
}

/// Modifier scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierScope {
    AllFeatures,
    Infill,
    Perimeters,
    Support,
    LayerHeight,
}

/// Modifier volume
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModifierVolume {
    pub id: ModifierId,
    pub mesh: IndexedTriangleSet,
    pub config_delta: ConfigDelta,
    pub priority: u32,
    pub applies_to: ModifierScope,
}

/// Object mesh
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectMesh {
    pub id: ObjectId,
    pub mesh: IndexedTriangleSet,
    pub transform: Transform3d,
    pub config: ObjectConfig,
    pub modifier_volumes: Vec<ModifierVolume>,
    /// All user-painted data for this object
    pub paint_data: Option<FacetPaintData>,
}

/// Mesh IR (produced by host mesh loader)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshIR {
    pub schema_version: SemVer,
    pub objects: Vec<ObjectMesh>,
    pub build_volume: BoundingBox3,
}

// ============================================================================
// Surface Classification IR Types
// ============================================================================

/// Facet classification
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FacetClass {
    Normal,
    NearHorizontal { slope_angle_deg: f32 },
    Overhang { angle_deg: f32 },
    Bridge,
    TopSurface,
    BottomSurface,
}

/// Surface group
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceGroup {
    pub id: SurfaceGroupId,
    pub facet_indices: Vec<u32>,
    pub z_min: f32,
    pub z_max: f32,
    pub area_mm2: f32,
    pub printable: bool,
    pub shell_count: u32,
}

/// Bridge region
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeRegion {
    pub id: BridgeRegionId,
    pub facet_indices: Vec<u32>,
    pub bridge_direction_deg: f32,
}

/// Overhang region
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverhangRegion {
    pub id: OverhangRegionId,
    pub facet_indices: Vec<u32>,
    pub max_angle_deg: f32,
    pub needs_support: bool,
}

/// Object surface data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectSurfaceData {
    pub facet_classes: Vec<FacetClass>,
    pub surface_groups: Vec<SurfaceGroup>,
    pub bridge_regions: Vec<BridgeRegion>,
    pub overhang_regions: Vec<OverhangRegion>,
}

/// Surface classification IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceClassificationIR {
    pub schema_version: SemVer,
    pub per_object: HashMap<ObjectId, ObjectSurfaceData>,
}

// ============================================================================
// Layer Plan IR Types
// ============================================================================

/// Config key type
pub type ConfigKey = String;

/// Config value type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<ConfigValue>),
}

/// Config view (pre-filtered for specific module)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigView {
    pub fields: HashMap<ConfigKey, ConfigValue>,
}

/// Non-planar shell reference
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NonPlanarShellRef {
    pub surface_group_id: SurfaceGroupId,
    pub shell_index: u32,
}

/// Resolved config (fully merged)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedConfig {
    // Geometry
    pub layer_height: f32,
    pub line_width: f32,
    pub first_layer_height: f32,
    pub first_layer_line_width: f32,
    
    // Walls
    pub wall_count: u32,
    pub outer_wall_speed: f32,
    pub inner_wall_speed: f32,
    pub wall_generator: WallGenerator,
    pub arachne_min_feature_size: Option<f32>,
    
    // Infill
    pub infill_type: InfillType,
    pub infill_density: f32,
    pub infill_angle: f32,
    pub infill_speed: f32,
    pub solid_infill_speed: f32,
    pub top_shell_layers: u32,
    pub bottom_shell_layers: u32,
    
    // Support
    pub support_enabled: bool,
    pub support_type: SupportType,
    pub support_overhang_angle: f32,
    
    // Non-planar (module-contributed)
    pub nonplanar_max_angle_deg: Option<f32>,
    pub nonplanar_shell_count: Option<u32>,
    pub nonplanar_amplitude: Option<f32>,
    
    // Smoothificator (module-contributed)
    pub smoothificator_target_height: Option<f32>,
    pub smoothificator_adaptive: Option<bool>,
    
    /// Overflow bucket for unknown module configs
    pub extensions: HashMap<String, ConfigValue>,
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        Self {
            layer_height: 0.2,
            line_width: 0.4,
            first_layer_height: 0.2,
            first_layer_line_width: 0.4,
            wall_count: 2,
            outer_wall_speed: 50.0,
            inner_wall_speed: 50.0,
            wall_generator: WallGenerator::Classic,
            arachne_min_feature_size: None,
            infill_type: InfillType::Grid,
            infill_density: 0.2,
            infill_angle: 45.0,
            infill_speed: 50.0,
            solid_infill_speed: 50.0,
            top_shell_layers: 3,
            bottom_shell_layers: 3,
            support_enabled: false,
            support_type: SupportType::Traditional,
            support_overhang_angle: 45.0,
            nonplanar_max_angle_deg: None,
            nonplanar_shell_count: None,
            nonplanar_amplitude: None,
            smoothificator_target_height: None,
            smoothificator_adaptive: None,
            extensions: HashMap::new(),
        }
    }
}

/// Active region in a global layer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveRegion {
    pub object_id: ObjectId,
    pub region_id: RegionId,
    pub resolved_config: ResolvedConfig,
    pub effective_layer_height: f32,
    pub nonplanar_shell: Option<NonPlanarShellRef>,
    pub is_catchup_layer: bool,
    pub catchup_z_bottom: f32,
    pub tool_index: u32,
}

/// Global layer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalLayer {
    pub index: u32,
    pub z: f32,
    pub active_regions: Vec<ActiveRegion>,
    pub has_nonplanar: bool,
    pub is_sync_layer: bool,
}

/// Object layer reference
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectLayerRef {
    pub local_layer_index: u32,
    pub global_layer_index: u32,
    pub effective_layer_height: f32,
}

/// Layer plan IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerPlanIR {
    pub schema_version: SemVer,
    pub global_layers: Vec<GlobalLayer>,
    pub object_participation: HashMap<ObjectId, Vec<ObjectLayerRef>>,
}

// ============================================================================
// Paint Region IR Types
// ============================================================================

/// Semantic region
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticRegion {
    pub object_id: ObjectId,
    pub polygons: Vec<ExPolygon>,
    pub value: PaintValue,
    pub paint_order: u64,
}

/// Layer paint map
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerPaintMap {
    pub global_layer_index: u32,
    pub semantic_regions: HashMap<PaintSemantic, Vec<SemanticRegion>>,
}

/// Paint region IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintRegionIR {
    pub schema_version: SemVer,
    pub per_layer: HashMap<u32, LayerPaintMap>,
}

impl PaintRegionIR {
    /// Convenience accessor for per-layer stage modules
    pub fn get(&self, layer_index: u32, semantic: &PaintSemantic) -> &[SemanticRegion] {
        self.per_layer
            .get(&layer_index)
            .and_then(|l| l.semantic_regions.get(semantic))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

// ============================================================================
// Region Map IR Types
// ============================================================================

/// Region key (unique identifier for a region in a layer)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegionKey {
    pub global_layer_index: u32,
    pub object_id: ObjectId,
    pub region_id: RegionId,
}

/// Module invocation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleInvocation {
    pub module_id: ModuleId,
    pub config_view: ConfigView,
}

/// Region plan
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionPlan {
    pub config: ResolvedConfig,
    pub stage_modules: HashMap<StageId, Vec<ModuleInvocation>>,
}

/// Region map IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionMapIR {
    pub schema_version: SemVer,
    pub entries: HashMap<RegionKey, RegionPlan>,
}

// ============================================================================
// Slice IR Types
// ============================================================================

/// Polygon
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polygon {
    pub points: Vec<Point2>,
}

/// Polygon with holes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExPolygon {
    pub contour: Polygon,
    pub holes: Vec<Polygon>,
}

/// Sliced region
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlicedRegion {
    pub object_id: ObjectId,
    pub region_id: RegionId,
    pub polygons: Vec<ExPolygon>,
    pub infill_areas: Vec<ExPolygon>,
    pub nonplanar_surface: Option<SurfaceGroupId>,
    pub effective_layer_height: f32,
    pub boundary_paint: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
}

/// Slice IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SliceIR {
    pub schema_version: SemVer,
    pub global_layer_index: u32,
    pub z: f32,
    pub regions: Vec<SlicedRegion>,
}

// ============================================================================
// Perimeter IR Types
// ============================================================================

/// Wall generator type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallGenerator {
    Classic,
    Arachne,
}

/// Infill type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InfillType {
    Grid,
    Triangles,
    Honeycomb,
    Gyroid,
    Lightning,
    Line,
    Concentric,
}

/// Support type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportType {
    Traditional,
    Tree,
}

/// Wall boundary type
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WallBoundaryType {
    ExteriorSurface,
    MaterialBoundary { adjacent_tool: u32 },
    Interior,
}

/// Loop type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopType {
    Outer,
    Inner,
    ThinWall,
    NonPlanarShell,
}

/// Wall feature flags
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallFeatureFlags {
    pub tool_index: Option<u32>,
    pub fuzzy_skin: bool,
    pub is_bridge: bool,
    pub is_thin_wall: bool,
    pub skip_ironing: bool,
    pub custom: HashMap<String, PaintValue>,
}

/// Width profile (for variable-width extrusion)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidthProfile {
    pub widths: Vec<f32>,
}

/// Point with width (for extrusion paths)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point3WithWidth {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub width: f32,
    pub flow_factor: f32,
}

/// Extrusion role
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtrusionRole {
    OuterWall,
    InnerWall,
    ThinWall,
    TopSolidInfill,
    BottomSolidInfill,
    SparseInfill,
    SupportMaterial,
    SupportInterface,
    WipeTower,
    PrimeTower,
    Ironing,
    BridgeInfill,
    Custom(String),
}

/// 3D extrusion path
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtrusionPath3D {
    pub points: Vec<Point3WithWidth>,
    pub role: ExtrusionRole,
    pub speed_factor: f32,
}

/// Wall loop
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallLoop {
    pub perimeter_index: u32,
    pub loop_type: LoopType,
    pub path: ExtrusionPath3D,
    pub width_profile: WidthProfile,
    pub feature_flags: Vec<WallFeatureFlags>,
    pub boundary_type: WallBoundaryType,
}

/// Seam reason
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeamReason {
    Concave,
    Aligned,
    UserForced,
    Sharp,
}

/// Seam candidate
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeamCandidate {
    pub position: Point3WithWidth,
    pub score: f32,
    pub reason: SeamReason,
}

/// Seam position
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeamPosition {
    pub point: Point3WithWidth,
    pub wall_index: u32,
}

/// Perimeter region
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerimeterRegion {
    pub object_id: ObjectId,
    pub region_id: RegionId,
    pub walls: Vec<WallLoop>,
    pub infill_areas: Vec<ExPolygon>,
    pub seam_candidates: Vec<SeamCandidate>,
    pub resolved_seam: Option<SeamPosition>,
}

/// Perimeter IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerimeterIR {
    pub schema_version: SemVer,
    pub global_layer_index: u32,
    pub regions: Vec<PerimeterRegion>,
}

// ============================================================================
// Infill IR Types
// ============================================================================

/// Infill region
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfillRegion {
    pub object_id: ObjectId,
    pub region_id: RegionId,
    pub sparse_infill: Vec<ExtrusionPath3D>,
    pub solid_infill: Vec<ExtrusionPath3D>,
    pub ironing: Vec<ExtrusionPath3D>,
}

/// Infill IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfillIR {
    pub schema_version: SemVer,
    pub global_layer_index: u32,
    pub regions: Vec<InfillRegion>,
}

// ============================================================================
// Support IR Types
// ============================================================================

/// Support IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportIR {
    pub schema_version: SemVer,
    pub global_layer_index: u32,
    pub support_paths: Vec<ExtrusionPath3D>,
    pub interface_paths: Vec<ExtrusionPath3D>,
    pub raft_paths: Vec<ExtrusionPath3D>,
    pub ironing_paths: Vec<ExtrusionPath3D>,
}

// ============================================================================
// Layer Collection IR Types
// ============================================================================

/// Tool change
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolChange {
    pub after_entity_index: u32,
    pub from_tool: u32,
    pub to_tool: u32,
}

/// Z hop
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZHop {
    pub after_entity_index: u32,
    pub hop_height: f32,
}

/// Print entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrintEntity {
    pub path: ExtrusionPath3D,
    pub role: ExtrusionRole,
    pub region_key: RegionKey,
    pub topo_order: u32,
}

/// Layer collection IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerCollectionIR {
    pub schema_version: SemVer,
    pub global_layer_index: u32,
    pub z: f32,
    pub ordered_entities: Vec<PrintEntity>,
    pub tool_changes: Vec<ToolChange>,
    pub z_hops: Vec<ZHop>,
}

// ============================================================================
// GCode IR Types
// ============================================================================

/// GCode command
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GCodeCommand {
    Move {
        x: Option<f32>,
        y: Option<f32>,
        z: Option<f32>,
        e: Option<f32>,
        f: Option<f32>,
        role: ExtrusionRole,
    },
    Retract {
        length: f32,
        speed: f32,
    },
    Unretract {
        length: f32,
        speed: f32,
    },
    FanSpeed {
        value: u8,
    },
    Temperature {
        tool: u32,
        celsius: f32,
        wait: bool,
    },
    ToolChange {
        from: u32,
        to: u32,
    },
    Comment {
        text: String,
    },
    Raw {
        text: String,
    },
}

/// Print metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrintMetadata {
    pub estimated_print_time_s: u32,
    pub filament_used_mm: Vec<f32>,
    pub layer_count: u32,
    pub slicer_version: String,
}

/// GCode IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GCodeIR {
    pub schema_version: SemVer,
    pub commands: Vec<GCodeCommand>,
    pub metadata: PrintMetadata,
}
//! Core IR type definitions
//!
//! All coordinate conversions follow the canonical rules:
//! - mm → units: `units = round(mm * 10_000.0)` (round half away from zero).
//! - units → mm: `mm = units / 10_000.0`.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Point2 {
    /// X coordinate in scaled integer units (1 unit = 100 nm)
    pub x: i64,
    /// Y coordinate in scaled integer units (1 unit = 100 nm)
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    /// X coordinate in millimeters
    pub x: f32,
    /// Y coordinate in millimeters
    pub y: f32,
    /// Z coordinate in millimeters
    pub z: f32,
}

/// 3D bounding box
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox3 {
    /// Minimum corner of the bounding box
    pub min: Point3,
    /// Maximum corner of the bounding box
    pub max: Point3,
}

/// 3D transformation (column-major 4x4 matrix)
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Transform3d {
    /// 4x4 transformation matrix in column-major order
    pub matrix: [f64; 16],
}

/// Indexed triangle set (vertices + indices)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IndexedTriangleSet {
    /// List of 3D vertices
    pub vertices: Vec<Point3>,
    /// List of indices into vertices, 3 per triangle
    pub indices: Vec<u32>,
}

/// Semantic versioning
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemVer {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Schema version for `SurfaceClassificationIR`. Single source of truth — production
/// constructors must use this constant, not literal `SemVer { ... }` values.
pub const CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 1,
    patch: 0,
};

/// Schema version for `SliceIR`. Single source of truth — production constructors
/// must use this constant, not literal `SemVer { ... }` values.
pub const CURRENT_SLICE_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 2,
    minor: 1,
    patch: 0,
};

/// Schema version for `MeshIR`. Bumped to 1.1.0 by packet 56b — populated
/// `modifier_volumes` from `Metadata/model_settings.config`.
pub const CURRENT_MESH_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 1,
    patch: 0,
};

/// Schema version for `LayerPlanIR`.
pub const CURRENT_LAYER_PLAN_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Schema version for `SeamPlanIR`.
pub const CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Schema version for `SupportPlanIR`.
pub const CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Schema version for `SupportGeometryIR`.
pub const CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Schema version for `PaintRegionIR`.
pub const CURRENT_PAINT_REGION_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Schema version for `MeshSegmentationIR`.
pub const CURRENT_MESH_SEGMENTATION_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Schema version for `RegionMapIR`. Bumped to 1.1.0 by packet 51 — additive
/// `paint_overrides` field on `RegionPlan`.
pub const CURRENT_REGION_MAP_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 1,
    patch: 0,
};

/// Schema version for `PerimeterIR`.
pub const CURRENT_PERIMETER_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Schema version for `InfillIR`.
pub const CURRENT_INFILL_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Schema version for `SupportIR`.
pub const CURRENT_SUPPORT_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Schema version for `LayerCollectionIR`.
pub const CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 0,
    patch: 0,
};

/// Schema version for `GCodeIR`.
pub const CURRENT_GCODE_IR_SCHEMA_VERSION: SemVer = SemVer {
    major: 1,
    minor: 0,
    patch: 0,
};

// ============================================================================
// Mesh IR Types
// ============================================================================

/// Raw user config (not yet resolved)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ObjectConfig {
    /// Configuration data key-value map
    pub data: HashMap<String, ConfigValue>,
}

/// Paint semantic types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PaintValue {
    /// Boolean flag
    Flag(bool),
    /// Scalar value
    Scalar(f32),
    /// Tool/material index
    ToolIndex(u32),
    /// Community-defined or module-defined string value.
    /// Used when none of the typed variants is appropriate.
    Custom(String),
}

/// Paint stroke (3D triangles defining painted region)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintStroke {
    /// 3D triangles on the mesh surface defining the painted region
    pub triangles: Vec<[Point3; 3]>,
    /// The semantic type of this paint stroke
    pub semantic: PaintSemantic,
    /// The value associated with this paint stroke
    pub value: PaintValue,
}

/// Paint layer (all paint with same semantic on one object)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintLayer {
    /// The semantic type of this paint layer
    pub semantic: PaintSemantic,
    /// One entry per mesh triangle, parallel to mesh.triangles
    pub facet_values: Vec<Option<PaintValue>>,
    /// Sub-facet strokes that cross triangle boundaries
    pub strokes: Vec<PaintStroke>,
}

/// All paint layers on one object
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FacetPaintData {
    /// List of paint layers on this object
    pub layers: Vec<PaintLayer>,
}

/// Config delta (only explicitly set fields)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfigDelta {
    /// Configuration fields that are explicitly set
    pub fields: HashMap<ConfigKey, ConfigValue>,
}

/// Modifier scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifierScope {
    /// Applies to all features
    AllFeatures,
    /// Applies only to infill
    Infill,
    /// Applies only to perimeters
    Perimeters,
    /// Applies only to support
    Support,
    /// Applies to layer height
    LayerHeight,
}

/// Modifier volume
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModifierVolume {
    /// Unique identifier for the modifier
    pub id: ModifierId,
    /// Geometry of the modifier
    pub mesh: IndexedTriangleSet,
    /// Configuration changes applied by this modifier
    pub config_delta: ConfigDelta,
    /// Priority of the modifier (higher wins)
    pub priority: u32,
    /// Scope of the modifier application
    pub applies_to: ModifierScope,
}

/// Object mesh
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ObjectMesh {
    /// Unique identifier for the object
    pub id: ObjectId,
    /// Geometry of the object
    pub mesh: IndexedTriangleSet,
    /// World-space placement of the object
    pub transform: Transform3d,
    /// Raw user config for the object
    pub config: ObjectConfig,
    /// Modifier volumes affecting this object
    pub modifier_volumes: Vec<ModifierVolume>,
    /// All user-painted data for this object
    pub paint_data: Option<FacetPaintData>,
    /// Cached world-space Z extent `(z_min, z_max)` in millimeters.
    ///
    /// Computed once at `ObjectMesh` construction time by applying the
    /// object transform to all mesh vertices and extracting the Z range.
    /// `None` when the mesh is empty or the extent is degenerate
    /// (`z_max <= z_min`, e.g. a lay-flat rotation collapses vertical extent).
    ///
    /// Not serialized — recomputed on every load so the field always reflects
    /// the current `transform` value. No schema version bump needed (v1.0.0
    /// not released). This makes world-space Z a first-class IR contract
    /// surface, closing DEV-027.
    #[serde(skip_deserializing, default)]
    pub world_z_extent: Option<(f32, f32)>,
}

/// Mesh IR (produced by host mesh loader)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshIR {
    /// Schema version of this IR
    pub schema_version: SemVer,
    /// List of objects in the mesh
    pub objects: Vec<ObjectMesh>,
    /// Build volume bounding box
    pub build_volume: BoundingBox3,
}

impl Default for MeshIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_MESH_IR_SCHEMA_VERSION,
            objects: Vec::new(),
            build_volume: BoundingBox3::default(),
        }
    }
}

// ============================================================================
// Surface Classification IR Types
// ============================================================================

/// Facet classification
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FacetClass {
    /// Normal facet
    Normal,
    /// Near horizontal facet
    NearHorizontal {
        /// Slope angle in degrees
        slope_angle_deg: f32,
    },
    /// Overhang facet
    Overhang {
        /// Angle in degrees
        angle_deg: f32,
    },
    /// Bridge facet
    Bridge,
    /// Top surface facet
    TopSurface,
    /// Bottom surface facet
    BottomSurface,
}

/// Surface group
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SurfaceGroup {
    /// Unique identifier for the surface group
    pub id: SurfaceGroupId,
    /// Indices of facets in this group
    pub facet_indices: Vec<u32>,
    /// Minimum Z height of the group
    pub z_min: f32,
    /// Maximum Z height of the group
    pub z_max: f32,
    /// Area of the group in mm^2
    pub area_mm2: f32,
    /// Whether the group is printable
    pub printable: bool,
    /// Shell count for the group
    pub shell_count: u32,
}

/// Bridge region
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BridgeRegion {
    /// Unique identifier for the bridge region
    pub id: BridgeRegionId,
    /// Indices of facets in this region
    pub facet_indices: Vec<u32>,
    /// Optimal bridge angle in degrees
    pub bridge_direction_deg: f32,
    /// Shortest perpendicular run of contiguous anchor edges (mm)
    #[serde(default)]
    pub anchor_width_mm: f32,
    /// Longest unsupported span across the cluster (mm)
    #[serde(default)]
    pub bridge_length_mm: f32,
    /// Frozen at PrePass from MeshAnalysisConfig (mm)
    #[serde(default)]
    pub expansion_margin_mm: f32,
    /// Pass/fail of min-length + anchor-width filters
    #[serde(default)]
    pub is_valid: bool,
    /// Facet-cluster XY projection in 100 nm units
    #[serde(default)]
    pub xy_footprint: Vec<ExPolygon>,
}

/// Overhang region
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OverhangRegion {
    /// Unique identifier for the overhang region
    pub id: OverhangRegionId,
    /// Indices of facets in this region
    pub facet_indices: Vec<u32>,
    /// Maximum overhang angle in degrees
    pub max_angle_deg: f32,
    /// Whether this region needs support
    pub needs_support: bool,
}

/// Object surface data
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ObjectSurfaceData {
    /// Facet classifications for each triangle
    pub facet_classes: Vec<FacetClass>,
    /// Surface groups in the object
    pub surface_groups: Vec<SurfaceGroup>,
    /// Bridge regions in the object
    pub bridge_regions: Vec<BridgeRegion>,
    /// Overhang regions in the object
    pub overhang_regions: Vec<OverhangRegion>,
}

/// Surface classification IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceClassificationIR {
    /// Schema version of this IR
    pub schema_version: SemVer,
    /// Per-object surface data
    pub per_object: HashMap<ObjectId, ObjectSurfaceData>,
}

impl Default for SurfaceClassificationIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION,
            per_object: HashMap::new(),
        }
    }
}

// ============================================================================
// Layer Plan IR Types
// ============================================================================

/// Config key type
pub type ConfigKey = String;

/// Config value type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigValue {
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Floating-point value
    Float(f64),
    /// String value
    String(String),
    /// List of config values
    List(Vec<ConfigValue>),
}

/// Config view (pre-filtered for specific module).
///
/// # Contract
///
/// Mirrors the `config-view` WIT resource (`wit/deps/config.wit`):
/// read-only and pre-filtered to a module's declared reads only. The
/// Rust side enforces this by keeping the backing `HashMap` private — all
/// consumers (host built-ins, guest modules, and tests) go through the
/// typed accessors (`get`, `get_bool`, `get_int`, `get_float`,
/// `get_string`, `get_float_list`, `get_string_list`, `keys`,
/// `iter_entries`) rather than touching the map directly (docs/03
/// §host-boundary access enforcement; docs/05 §module SDK).
///
/// # Construction
///
/// The only binding path allowed by the docs is pre-filtering to the
/// module's declared config keys. Use [`ConfigView::from_declared`] when
/// the host wires a config view for a compiled module; use
/// [`ConfigView::new`] or [`ConfigView::from_map`] for test fixtures that
/// simulate an already-filtered view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigView {
    // Private map. External construction goes through `from_map` /
    // `from_declared`; reads go through the typed accessors below. This
    // enforces the WIT `resource config-view { ... }` read-only
    // contract at the Rust boundary, so consumers cannot mutate a
    // view after the host synthesises it.
    fields: HashMap<ConfigKey, ConfigValue>,
}

impl ConfigView {
    /// Create an empty, already-frozen config view.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Wrap an existing `HashMap` as a config view without filtering.
    /// Intended for test fixtures only; live-path host code must use
    /// [`ConfigView::from_declared`].
    #[must_use]
    pub fn from_map(fields: HashMap<ConfigKey, ConfigValue>) -> Self {
        Self { fields }
    }

    /// Build a config view pre-filtered to `declared` keys — the only
    /// contract-compliant constructor for the live host/runtime path
    /// (docs/03 §host-boundary access enforcement; docs/02 §Pre-filtered
    /// config).
    ///
    /// Keys in `source` that are not in `declared` are dropped. Keys in
    /// `declared` that are not in `source` produce no entry (the typed
    /// accessors will return `None`, matching the undeclared/missing
    /// semantics modules already observe).
    #[must_use]
    pub fn from_declared<'a, I>(source: &HashMap<ConfigKey, ConfigValue>, declared: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut fields = HashMap::new();
        for key in declared {
            if let Some(value) = source.get(key) {
                fields.insert(key.to_string(), value.clone());
            }
        }
        Self { fields }
    }

    /// Typed read: return the raw `ConfigValue` for `key`, if present.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        self.fields.get(key)
    }

    /// Typed read: `bool` value, or `None` if missing/other type.
    #[must_use]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.fields.get(key)? {
            ConfigValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Typed read: `i64` value, or `None` if missing/other type.
    #[must_use]
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.fields.get(key)? {
            ConfigValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Typed read: `f64` value with subnormal normalization
    /// (subnormals are coerced to `0.0`), matching the WIT boundary
    /// behavior in `slicer-host::wit_host::normalize_subnormal_boundary`
    /// and the schema parser's `normalize_subnormal`.
    /// Returns `None` if missing/other type.
    #[must_use]
    pub fn get_float(&self, key: &str) -> Option<f64> {
        match self.fields.get(key)? {
            ConfigValue::Float(f) => Some(if f.is_subnormal() { 0.0 } else { *f }),
            _ => None,
        }
    }

    /// Typed read: `String` value, or `None` if missing/other type.
    #[must_use]
    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.fields.get(key)? {
            ConfigValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Return all keys visible to the module, sorted for deterministic
    /// iteration (mirrors the WIT `keys()` contract).
    #[must_use]
    pub fn keys(&self) -> Vec<String> {
        let mut out: Vec<String> = self.fields.keys().cloned().collect();
        out.sort();
        out
    }

    /// True if `key` is visible to the module.
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    /// Number of declared keys visible to the module.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// True when no keys are visible.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Deterministically iterate all `(key, value)` pairs visible to the
    /// module, ordered by key. Used by non-typed consumers (e.g. the
    /// Python postpass bridge) that need to ship every declared value
    /// to a foreign runtime without additional round-trip host calls.
    pub fn iter_entries(&self) -> impl Iterator<Item = (&str, &ConfigValue)> {
        let mut keys: Vec<&str> = self.fields.keys().map(String::as_str).collect();
        keys.sort();
        keys.into_iter()
            .map(move |k| (k, self.fields.get(k).expect("iter_entries: key vanished")))
    }
}

impl Default for ConfigView {
    fn default() -> Self {
        Self::new()
    }
}

/// Non-planar shell reference
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct NonPlanarShellRef {
    /// ID of the surface group this shell belongs to
    pub surface_group_id: SurfaceGroupId,
    /// Index of the shell (0 = top surface, 1..N = internal shells)
    pub shell_index: u32,
}

/// Resolved config (fully merged)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedConfig {
    // Geometry
    /// Layer height in millimeters
    pub layer_height: f32,
    /// Line width in millimeters
    pub line_width: f32,
    /// First layer height in millimeters
    pub first_layer_height: f32,
    /// First layer line width in millimeters
    pub first_layer_line_width: f32,

    // Walls
    /// Number of walls (perimeters)
    pub wall_count: u32,
    /// Outer wall speed in mm/s
    pub outer_wall_speed: f32,
    /// Inner wall speed in mm/s
    pub inner_wall_speed: f32,
    /// Wall generator algorithm
    pub wall_generator: WallGenerator,
    /// Minimum feature size for Arachne (optional)
    pub arachne_min_feature_size: Option<f32>,

    // Infill
    /// Infill type
    pub infill_type: InfillType,
    /// Infill density (0.0 to 1.0)
    pub infill_density: f32,
    /// Infill angle in degrees
    pub infill_angle: f32,
    /// Infill speed in mm/s
    pub infill_speed: f32,
    /// Solid infill speed in mm/s
    pub solid_infill_speed: f32,
    /// Number of top shell layers
    pub top_shell_layers: u32,
    /// Number of bottom shell layers
    pub bottom_shell_layers: u32,

    // Fill-role holders (packet 37). Each names the module ID that holds the
    // corresponding `claim:*-fill`. Default is "rectilinear-infill" for all
    // four (matches today's behavior). Per-region overrides flow through
    // `RegionMapIR.entries[*].config`.
    /// Module ID holding `claim:top-fill`.
    pub top_fill_holder: String,
    /// Module ID holding `claim:bottom-fill`.
    pub bottom_fill_holder: String,
    /// Module ID holding `claim:bridge-fill`.
    pub bridge_fill_holder: String,
    /// Module ID holding `claim:sparse-fill`.
    pub sparse_fill_holder: String,

    // Support
    /// Whether support is enabled
    pub support_enabled: bool,
    /// Support generation type
    pub support_type: SupportType,
    /// Support overhang angle threshold in degrees
    pub support_overhang_angle: f32,

    // Non-planar (module-contributed)
    /// Maximum non-planar angle in degrees (optional)
    pub nonplanar_max_angle_deg: Option<f32>,
    /// Number of non-planar shells (optional)
    pub nonplanar_shell_count: Option<u32>,
    /// Non-planar amplitude in millimeters (optional)
    pub nonplanar_amplitude: Option<f32>,

    // Smoothificator (module-contributed)
    /// Smoothificator target height in millimeters (optional)
    pub smoothificator_target_height: Option<f32>,
    /// Smoothificator adaptive mode (optional)
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
            top_fill_holder: String::from("rectilinear-infill"),
            bottom_fill_holder: String::from("rectilinear-infill"),
            bridge_fill_holder: String::from("rectilinear-infill"),
            sparse_fill_holder: String::from("rectilinear-infill"),
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActiveRegion {
    /// Object ID this region belongs to
    pub object_id: ObjectId,
    /// Region ID
    pub region_id: RegionId,
    /// Fully resolved config for this region
    pub resolved_config: ResolvedConfig,
    /// Effective layer height for this region
    pub effective_layer_height: f32,
    /// Non-planar shell reference (optional)
    pub nonplanar_shell: Option<NonPlanarShellRef>,
    /// True if this region skipped the previous global Z and is catching up
    pub is_catchup_layer: bool,
    /// Bottom Z of the catchup layer
    pub catchup_z_bottom: f32,
    /// Tool/filament index for this region
    pub tool_index: u32,
}

/// Global layer
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GlobalLayer {
    /// Index of the global layer
    pub index: u32,
    /// Z height of the layer
    pub z: f32,
    /// Active regions in this layer
    pub active_regions: Vec<ActiveRegion>,
    /// True if the layer contains non-planar features
    pub has_nonplanar: bool,
    /// True if multiple objects with different layer heights align at this Z
    pub is_sync_layer: bool,
}

/// Object layer reference
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ObjectLayerRef {
    /// Local layer index within the object
    pub local_layer_index: u32,
    /// Global layer index in the scene
    pub global_layer_index: u32,
    /// Effective layer height for this layer
    pub effective_layer_height: f32,
}

/// Layer plan IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerPlanIR {
    /// Schema version of this IR
    pub schema_version: SemVer,
    /// Global layers in the plan
    pub global_layers: Vec<GlobalLayer>,
    /// Per-object layer participation mapping
    pub object_participation: HashMap<ObjectId, Vec<ObjectLayerRef>>,
}

impl Default for LayerPlanIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_LAYER_PLAN_IR_SCHEMA_VERSION,
            global_layers: Vec::new(),
            object_participation: HashMap::new(),
        }
    }
}

// ============================================================================
// Seam Plan IR Types
// ============================================================================

/// One scored seam candidate from the prepass planner.
///
/// Used both inside `SeamPlanEntry.scored_candidates` and as the
/// `SeamPlanEntry.chosen_candidate` selected by the planning algorithm.
/// The `score` field is the primary sort key; lower is better.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredSeamCandidate {
    /// Candidate position with extrusion width.
    pub position: Point3WithWidth,
    /// Normalized seam score (lower = preferred).
    pub score: f32,
    /// Enum tag explaining why this candidate was scored this way.
    pub reason: SeamReason,
}

/// One entry in the global seam plan.
///
/// Produced once per `(global_layer_index, object_id, region_id)` triple
/// by `PrePass::SeamPlanning` and stored immutably on the blackboard.
/// Consumed at dispatch time by `Layer::PerimetersPostProcess` to seed
/// `PerimeterRegionView.resolved_seam` so the apply-stage module
/// (seam-placer) operates on a pre-resolved seam without rescoring.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SeamPlanEntry {
    /// Stable region key for lookup during layer dispatch.
    pub region_key: RegionKey,
    /// The seam position selected by the planner.
    pub chosen_candidate: SeamPosition,
    /// Full scored candidate list for evidence and regression checks.
    pub scored_candidates: Vec<ScoredSeamCandidate>,
}

/// Seam plan IR — committed once to the blackboard by `PrePass::SeamPlanning`.
///
/// Stored as write-once on the blackboard alongside the other prepass
/// artifacts. Consumed by the layer dispatch path to inject resolved
/// seams into `PerimeterRegionView` before `seam-placer` runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeamPlanIR {
    /// Schema version of this IR.
    pub schema_version: SemVer,
    /// One entry per active `(layer, object, region)` triple.
    pub entries: Vec<SeamPlanEntry>,
}

impl Default for SeamPlanIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

// ============================================================================
// Support Plan IR Types
// ============================================================================

/// One entry in the global support plan.
///
/// Produced once per `(global_layer_index, object_id, region_id)` triple
/// by `PrePass::SupportGeometry` and stored immutably on the blackboard.
/// Consumed at dispatch time by `Layer::Support` modules (notably
/// `tree-support`) that emit pre-planned organic branch geometry instead
/// of running a per-layer filler.
///
/// `global_layer_index` uses a signed integer to support raft prefix layers:
/// raft entries carry negative indices (`-1, -2, ..., -raft_layers`) so raft
/// always sorts before model layers (which use `0, 1, 2, ...`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportPlanEntry {
    /// Global (inter-object) layer index this entry applies to.
    /// Negative values (`-1`, `-2`, ...) are reserved for raft prefix layers.
    /// Non-negative values (`0`, `1`, ...) refer to model layers.
    pub global_layer_index: i32,
    /// Object the branches belong to.
    pub object_id: ObjectId,
    /// Region inside the object the branches belong to.
    pub region_id: RegionId,
    /// Planned branch geometry for this `(layer, object, region)` triple.
    /// Each `ExtrusionPath3D` is typically a two-point segment (a single
    /// MST edge) but may be multi-point for long merged branches.
    pub branch_segments: Vec<ExtrusionPath3D>,
}

/// Support plan IR — committed once to the blackboard by
/// `PrePass::SupportGeometry`.
///
/// Carries per-layer organic branch geometry produced by a simplified
/// OrcaSlicer-style top-down propagation (see the `support-planner`
/// core module). The per-layer `Layer::Support` tree-support module
/// consumes the plan when it is committed and emits branch segments
/// directly; modules whose algorithm is inherently per-layer (e.g.
/// `traditional-support`) do not read this IR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportPlanIR {
    /// Schema version of this IR.
    pub schema_version: SemVer,
    /// One entry per active `(layer, object, region)` triple that received
    /// planned branches. Multiple entries may share `(layer, object)` when
    /// a single object has multiple regions on the same layer.
    pub entries: Vec<SupportPlanEntry>,
}

impl Default for SupportPlanIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

// ============================================================================
// Support Geometry IR Types
// ============================================================================

/// Key uniquely identifying one support geometry entry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SupportGeometryKey {
    /// u32::MAX sentinel = intermediate model-resolution layer.
    pub global_support_layer_index: u32,
    /// Object this entry belongs to.
    pub object_id: ObjectId,
    /// Region identifier within the object.
    pub region_id: RegionId,
}

/// Support geometry IR — coarse outline prepass results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportGeometryIR {
    /// Schema version of this IR.
    pub schema_version: SemVer,
    /// 0.0 = use model layer height (config schema enforces min > 0).
    pub support_layer_height_mm: f32,
    /// Distance in mm from column tops to add intermediate model layers.
    pub support_top_z_distance_mm: f32,
    /// Per-(layer, object, region) coarse outline polygons.
    pub entries: HashMap<SupportGeometryKey, Vec<ExPolygon>>,
}

impl Default for SupportGeometryIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION,
            support_layer_height_mm: 0.0,
            support_top_z_distance_mm: 0.0,
            entries: HashMap::new(),
        }
    }
}

// ============================================================================
// Paint Region IR Types
// ============================================================================

/// Semantic region
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticRegion {
    /// Object ID this region belongs to
    pub object_id: ObjectId,
    /// Polygons defining this region
    pub polygons: Vec<ExPolygon>,
    /// Paint value for this region
    pub value: PaintValue,
    /// Paint order (higher means painted later)
    pub paint_order: u64,
}

/// Layer paint map
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerPaintMap {
    /// Global layer index
    pub global_layer_index: u32,
    /// Paint regions keyed by semantic
    pub semantic_regions: HashMap<PaintSemantic, Vec<SemanticRegion>>,
}

/// Paint region IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintRegionIR {
    /// Schema version of this IR
    pub schema_version: SemVer,
    /// Per-layer paint maps
    pub per_layer: HashMap<u32, LayerPaintMap>,
}

impl Default for PaintRegionIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_PAINT_REGION_IR_SCHEMA_VERSION,
            per_layer: HashMap::new(),
        }
    }
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
// Mesh Segmentation IR Types
// ============================================================================

/// One whole-triangle paint mark emitted by a `PrePass::MeshSegmentation`
/// module via `mark-triangle-paint`.
///
/// Matches `world-prepass.wit::mesh-segmentation-output::mark-triangle-paint`:
/// the guest normalizes sub-facet strokes into per-triangle paint values and
/// reports `(semantic, value)` for a specific `(object_id, facet_index)`.
/// Semantic and value are free-form strings at this layer — the consumer
/// decides how to parse `value` (tool index, boolean flag, named material).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FacetPaintMark {
    /// Object the mark applies to.
    pub object_id: ObjectId,
    /// Zero-based facet index into the object's triangle list.
    pub facet_index: u32,
    /// Paint semantic (e.g. `"material"`, `"fuzzy_skin"`, `"support_enforcer"`).
    pub semantic: String,
    /// Paint value associated with the semantic.
    pub value: String,
}

/// Mesh segmentation IR produced by `PrePass::MeshSegmentation`.
///
/// Commits a deterministic, ordered list of per-facet paint marks covering
/// every object. Downstream stages (today: none in the routed topology)
/// can read this to understand which facets carry which paint semantic at
/// whole-triangle granularity after stroke normalization. The invariant
/// is "exactly one value per `(object_id, facet_index, semantic)` triple".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshSegmentationIR {
    /// Schema version of this IR.
    pub schema_version: SemVer,
    /// Deterministic, insertion-ordered list of paint marks.
    pub marks: Vec<FacetPaintMark>,
}

impl Default for MeshSegmentationIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_MESH_SEGMENTATION_IR_SCHEMA_VERSION,
            marks: Vec::new(),
        }
    }
}

// ============================================================================
// Region Map IR Types
// ============================================================================

/// Region key (unique identifier for a region in a layer)
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegionKey {
    /// Global layer index
    pub global_layer_index: u32,
    /// Object ID
    pub object_id: ObjectId,
    /// Region ID
    pub region_id: RegionId,
}

/// Module invocation
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModuleInvocation {
    /// ID of the module to invoke
    pub module_id: ModuleId,
    /// Configuration view for the module
    pub config_view: ConfigView,
}

/// Region plan
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RegionPlan {
    /// Resolved config for the region
    pub config: ResolvedConfig,
    /// Module invocations per stage
    pub stage_modules: HashMap<StageId, Vec<ModuleInvocation>>,
    /// Per-paint-semantic config overrides (empty when no paint overrides apply)
    pub paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>,
}

/// Region map IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionMapIR {
    /// Schema version of this IR
    pub schema_version: SemVer,
    /// Region plans keyed by region key
    pub entries: HashMap<RegionKey, RegionPlan>,
}

impl Default for RegionMapIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
            entries: HashMap::new(),
        }
    }
}

// ============================================================================
// Slice IR Types
// ============================================================================

/// Polygon
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Polygon {
    /// List of points defining the polygon
    pub points: Vec<Point2>,
}

/// Polygon with holes
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExPolygon {
    /// Outer contour (CCW)
    pub contour: Polygon,
    /// Inner holes (CW)
    pub holes: Vec<Polygon>,
}

/// Sliced region
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SlicedRegion {
    /// Object ID this region belongs to
    pub object_id: ObjectId,
    /// Region ID
    pub region_id: RegionId,
    /// Closed polygon islands
    pub polygons: Vec<ExPolygon>,
    /// Inset polygons available for infill
    pub infill_areas: Vec<ExPolygon>,
    /// Non-planar surface group (optional)
    pub nonplanar_surface: Option<SurfaceGroupId>,
    /// Effective layer height
    pub effective_layer_height: f32,
    /// Paint region membership for points on polygon contour boundaries
    pub boundary_paint: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
    /// True if this region is an exposed top surface
    #[serde(default)]
    pub is_top_surface: bool,
    /// True if this region is an exposed bottom surface
    #[serde(default)]
    pub is_bottom_surface: bool,
    /// True if this region spans an unsupported gap (bridge)
    #[serde(default)]
    pub is_bridge: bool,
    /// Per-layer expanded bridge polygons
    #[serde(default)]
    pub bridge_areas: Vec<ExPolygon>,
    /// Best bridge direction across all valid bridge regions (degrees)
    #[serde(default)]
    pub bridge_orientation_deg: f32,
}

/// Slice IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SliceIR {
    /// Schema version of this IR
    pub schema_version: SemVer,
    /// Global layer index
    pub global_layer_index: u32,
    /// Z height of the layer
    pub z: f32,
    /// Sliced regions in this layer
    pub regions: Vec<SlicedRegion>,
}

impl Default for SliceIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.0,
            regions: Vec::new(),
        }
    }
}

// ============================================================================
// Perimeter IR Types
// ============================================================================

/// Wall generator type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallGenerator {
    /// Classic wall generator
    #[default]
    Classic,
    /// Arachne variable-width wall generator
    Arachne,
}

/// Infill type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InfillType {
    /// Grid infill
    #[default]
    Grid,
    /// Triangles infill
    Triangles,
    /// Honeycomb infill
    Honeycomb,
    /// Gyroid infill
    Gyroid,
    /// Lightning infill
    Lightning,
    /// Line infill
    Line,
    /// Concentric infill
    Concentric,
}

/// Support type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportType {
    /// Traditional support generation
    #[default]
    Traditional,
    /// Tree support generation
    Tree,
}

/// Wall boundary type
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum WallBoundaryType {
    /// Outer wall facing air or a gap
    ExteriorSurface,
    /// Wall adjacent to a different material region
    MaterialBoundary {
        /// Tool index of the neighboring region
        adjacent_tool: u32,
    },
    /// Inner wall — no special boundary handling
    Interior,
}

/// Loop type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopType {
    /// Outer loop
    Outer,
    /// Inner loop
    Inner,
    /// Thin wall loop
    ThinWall,
    /// Non-planar shell loop
    NonPlanarShell,
}

/// Wall feature flags
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WallFeatureFlags {
    /// Tool override for this segment
    pub tool_index: Option<u32>,
    /// Enables fuzzy skin modulation
    pub fuzzy_skin: bool,
    /// Segment is bridge-like
    pub is_bridge: bool,
    /// Segment belongs to thin-wall logic
    pub is_thin_wall: bool,
    /// If true, force skip of ironing behavior
    pub skip_ironing: bool,
    /// Custom paint values keyed by PaintSemantic::Custom module ID
    pub custom: HashMap<String, PaintValue>,
}

/// Width profile (for variable-width extrusion)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WidthProfile {
    /// One width per vertex in path.points
    pub widths: Vec<f32>,
}

/// Point with width (for extrusion paths)
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Point3WithWidth {
    /// X coordinate in millimeters
    pub x: f32,
    /// Y coordinate in millimeters
    pub y: f32,
    /// Z coordinate in millimeters
    pub z: f32,
    /// Local extrusion width in millimeters
    pub width: f32,
    /// Multiplier on base extrusion volume
    pub flow_factor: f32,
    /// Overhang severity quartile (0-3), None if not classified
    #[serde(default)]
    pub overhang_quartile: Option<u8>,
}

/// Extrusion role
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtrusionRole {
    /// Outer wall
    OuterWall,
    /// Inner wall
    InnerWall,
    /// Thin wall
    ThinWall,
    /// Top solid infill
    TopSolidInfill,
    /// Bottom solid infill
    BottomSolidInfill,
    /// Sparse infill
    SparseInfill,
    /// Support material
    SupportMaterial,
    /// Support interface
    SupportInterface,
    /// Wipe tower
    WipeTower,
    /// Prime tower
    PrimeTower,
    /// Ironing
    Ironing,
    /// Bridge infill
    BridgeInfill,
    /// Skirt / brim
    Skirt,
    /// Custom role
    Custom(String),
}

impl ExtrusionRole {
    /// Returns the default print priority for this role.
    /// Values mirror the canonical producer-emit order used by Packet 40's
    /// stable-sort merge: lower = printed first, gaps ≥ 100 guarantee
    /// unambiguous slot insertion for finalization-pushed entities.
    pub const fn default_priority(&self) -> u32 {
        match self {
            Self::Skirt => 0,
            Self::OuterWall => 1000,
            Self::InnerWall => 1500,
            Self::ThinWall => 1700,
            Self::SparseInfill => 3000,
            Self::BridgeInfill => 3500,
            Self::BottomSolidInfill => 4000,
            Self::TopSolidInfill => 4500,
            Self::SupportMaterial => 5000,
            Self::SupportInterface => 5500,
            Self::Ironing => 6000,
            Self::WipeTower => 8000,
            Self::PrimeTower => 8500,
            Self::Custom(_) => 9000,
        }
    }
}

/// 3D extrusion path
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtrusionPath3D {
    /// List of 3D points with width
    pub points: Vec<Point3WithWidth>,
    /// Role of this extrusion path
    pub role: ExtrusionRole,
    /// Speed factor multiplier
    pub speed_factor: f32,
}

/// Wall loop
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallLoop {
    /// 0 = outermost
    pub perimeter_index: u32,
    /// Type of loop
    pub loop_type: LoopType,
    /// 3D extrusion path
    pub path: ExtrusionPath3D,
    /// Variable-width profile
    pub width_profile: WidthProfile,
    /// Per-vertex feature flags
    pub feature_flags: Vec<WallFeatureFlags>,
    /// Boundary type
    pub boundary_type: WallBoundaryType,
}

/// Seam reason
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeamReason {
    /// Concave corner
    Concave,
    /// Aligned edge
    Aligned,
    /// User forced
    UserForced,
    /// Sharp corner
    Sharp,
}

/// Seam candidate
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeamCandidate {
    /// Position of the candidate
    pub position: Point3WithWidth,
    /// Score of the candidate
    pub score: f32,
    /// Reason for the candidate
    pub reason: SeamReason,
}

/// Seam position
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SeamPosition {
    /// Position of the seam
    pub point: Point3WithWidth,
    /// Index of the wall
    pub wall_index: u32,
}

/// Perimeter region
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PerimeterRegion {
    /// Object ID this region belongs to
    pub object_id: ObjectId,
    /// Region ID
    pub region_id: RegionId,
    /// Wall loops in this region
    pub walls: Vec<WallLoop>,
    /// Remaining area after wall insets
    pub infill_areas: Vec<ExPolygon>,
    /// Seam candidates
    pub seam_candidates: Vec<SeamCandidate>,
    /// Resolved seam position
    pub resolved_seam: Option<SeamPosition>,
}

/// Perimeter IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerimeterIR {
    /// Schema version of this IR
    pub schema_version: SemVer,
    /// Global layer index
    pub global_layer_index: u32,
    /// Perimeter regions in this layer
    pub regions: Vec<PerimeterRegion>,
}

impl Default for PerimeterIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_PERIMETER_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            regions: Vec::new(),
        }
    }
}

// ============================================================================
// Infill IR Types
// ============================================================================

/// Infill region
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InfillRegion {
    /// Object ID this region belongs to
    pub object_id: ObjectId,
    /// Region ID
    pub region_id: RegionId,
    /// Sparse infill paths
    pub sparse_infill: Vec<ExtrusionPath3D>,
    /// Solid infill paths
    pub solid_infill: Vec<ExtrusionPath3D>,
    /// Ironing paths
    pub ironing: Vec<ExtrusionPath3D>,
}

/// Infill IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InfillIR {
    /// Schema version of this IR
    pub schema_version: SemVer,
    /// Global layer index
    pub global_layer_index: u32,
    /// Infill regions in this layer
    pub regions: Vec<InfillRegion>,
}

impl Default for InfillIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_INFILL_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            regions: Vec::new(),
        }
    }
}

// ============================================================================
// Support IR Types
// ============================================================================

/// Support IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportIR {
    /// Schema version of this IR
    pub schema_version: SemVer,
    /// Global layer index
    pub global_layer_index: u32,
    /// Support paths
    pub support_paths: Vec<ExtrusionPath3D>,
    /// Interface paths
    pub interface_paths: Vec<ExtrusionPath3D>,
    /// Raft paths
    pub raft_paths: Vec<ExtrusionPath3D>,
    /// Ironing paths
    pub ironing_paths: Vec<ExtrusionPath3D>,
}

impl Default for SupportIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SUPPORT_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            support_paths: Vec::new(),
            interface_paths: Vec::new(),
            raft_paths: Vec::new(),
            ironing_paths: Vec::new(),
        }
    }
}

// ============================================================================
// Layer Collection IR Types
// ============================================================================

/// Tool change
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolChange {
    /// Index of the entity after which the tool change occurs
    pub after_entity_index: u32,
    /// Tool index to change from
    pub from_tool: u32,
    /// Tool index to change to
    pub to_tool: u32,
}

/// Z hop
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ZHop {
    /// Index of the entity after which the Z hop occurs
    pub after_entity_index: u32,
    /// Height of the Z hop in millimeters
    pub hop_height: f32,
}

/// Retract or unretract decision from `Layer::PathOptimization`, keyed by entity anchor.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TravelRetract {
    /// Index of the entity after which this retract/unretract is anchored.
    pub after_entity_index: u32,
    /// Retraction length in mm.
    pub length: f32,
    /// Retraction speed in mm/s.
    pub speed: f32,
    /// `true` = Unretract; `false` = Retract.
    pub is_unretract: bool,
    /// Emit-mode: `Gcode` materializes as inline-E `G1` moves; `Firmware`
    /// materializes as bare `G10`/`G11` opcodes. Defaults to `Gcode` for
    /// callers that haven't been migrated to thread the mode field.
    #[serde(default)]
    pub mode: RetractMode,
}

/// Travel move destination from `Layer::PathOptimization`, keyed by entity anchor.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TravelMove {
    /// Stable identifier of the entity in `LayerCollectionIR.ordered_entities` after which this
    /// travel is emitted. Resolved at emit time via a per-layer `HashMap<u64, usize>` lookup.
    pub entity_id: u64,
    /// X destination (module coordinate units, 100 nm).
    pub x: Option<f32>,
    /// Y destination (module coordinate units, 100 nm).
    pub y: Option<f32>,
    /// Z destination (module coordinate units, 100 nm).
    pub z: Option<f32>,
    /// Feed-rate override in mm/s (`None` = keep current speed).
    pub f: Option<f32>,
}

/// Print entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrintEntity {
    /// Per-layer-monotonic stable identifier issued by `LayerEntityIdGen` at construction.
    /// Reserved value `0` MAY be used as uninitialized sentinel; valid IDs start at 1.
    pub entity_id: u64,
    /// Extrusion path
    pub path: ExtrusionPath3D,
    /// Role of the entity
    pub role: ExtrusionRole,
    /// Region key
    pub region_key: RegionKey,
    /// Topological order
    pub topo_order: u32,
}

/// Kind of a guest-emitted layer annotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerAnnotationKind {
    /// A GCode comment (prefixed with `;`).
    Comment(String),
    /// A raw GCode line to emit verbatim.
    Raw(String),
}

/// Guest-emitted annotation (comment / raw line) to splice into the emitted
/// GCode at `after_entity_index`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerAnnotation {
    /// Insert the annotation after the entity with this 0-based `topo_order`.
    pub after_entity_index: u32,
    /// The annotation body.
    pub kind: LayerAnnotationKind,
}

/// Layer collection IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerCollectionIR {
    /// Schema version of this IR
    pub schema_version: SemVer,
    /// Global layer index
    pub global_layer_index: u32,
    /// Z height of the layer
    pub z: f32,
    /// Ordered, ready-to-emit extrusion entities
    pub ordered_entities: Vec<PrintEntity>,
    /// Tool changes in this layer
    pub tool_changes: Vec<ToolChange>,
    /// Z hops in this layer
    pub z_hops: Vec<ZHop>,
    /// Guest-emitted per-layer annotations (comments / raw lines).
    pub annotations: Vec<LayerAnnotation>,
    /// Retract/unretract decisions from `Layer::PathOptimization`.
    pub retracts: Vec<TravelRetract>,
    /// Travel move destinations from `Layer::PathOptimization`.
    pub travel_moves: Vec<TravelMove>,
}

impl Default for LayerCollectionIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION,
            global_layer_index: 0,
            z: 0.0,
            ordered_entities: Vec::new(),
            tool_changes: Vec::new(),
            z_hops: Vec::new(),
            annotations: Vec::new(),
            retracts: Vec::new(),
            travel_moves: Vec::new(),
        }
    }
}

// ============================================================================
// GCode IR Types
// ============================================================================

/// Selects whether retract/unretract commands are emitted as explicit G-code
/// extruder moves (`Gcode`) or delegated to the printer firmware via `G10`/`G11`
/// (`Firmware`). Default callers pass `Gcode` to preserve packet-15 behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RetractMode {
    /// Slicer emits retract/unretract as explicit `G1 E...` moves.
    #[default]
    Gcode,
    /// Slicer emits firmware retract/unretract (`G10`/`G11`).
    Firmware,
}

/// GCode command
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GCodeCommand {
    /// Move command
    Move {
        /// X coordinate
        x: Option<f32>,
        /// Y coordinate
        y: Option<f32>,
        /// Z coordinate
        z: Option<f32>,
        /// Extrusion amount
        e: Option<f32>,
        /// Feed rate
        f: Option<f32>,
        /// Role of the move
        role: ExtrusionRole,
    },
    /// Retract command
    Retract {
        /// Retraction length
        length: f32,
        /// Retraction speed
        speed: f32,
        /// Retract emission mode (G-code vs. firmware)
        mode: RetractMode,
    },
    /// Unretract command
    Unretract {
        /// Unretraction length
        length: f32,
        /// Unretraction speed
        speed: f32,
        /// Retract emission mode (G-code vs. firmware)
        mode: RetractMode,
    },
    /// Fan speed command
    FanSpeed {
        /// Fan speed value (0-255)
        value: u8,
    },
    /// Temperature command
    Temperature {
        /// Tool index
        tool: u32,
        /// Temperature in Celsius
        celsius: f32,
        /// Whether to wait for temperature
        wait: bool,
    },
    /// Tool change command
    ToolChange {
        /// Entity index after which this tool change occurs
        after_entity_index: u32,
        /// Tool index to change from
        from: u32,
        /// Tool index to change to
        to: u32,
    },
    /// Comment command
    Comment {
        /// Comment text
        text: String,
    },
    /// Raw GCode command
    Raw {
        /// Raw text
        text: String,
    },
}

/// Print metadata
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PrintMetadata {
    /// Estimated print time in seconds
    pub estimated_print_time_s: u32,
    /// Filament used per tool in millimeters
    pub filament_used_mm: Vec<f32>,
    /// Total layer count
    pub layer_count: u32,
    /// Slicer version string
    pub slicer_version: String,
}

/// GCode IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GCodeIR {
    /// Schema version of this IR
    pub schema_version: SemVer,
    /// List of GCode commands
    pub commands: Vec<GCodeCommand>,
    /// Print metadata
    pub metadata: PrintMetadata,
}

impl Default for GCodeIR {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_GCODE_IR_SCHEMA_VERSION,
            commands: Vec::new(),
            metadata: PrintMetadata::default(),
        }
    }
}

# ModularSlicer — Intermediate Representation (IR) Schemas

All IRs are defined in `crates/slicer-ir/src/`. They are the shared contract between the host and all modules. IR types are re-exported by the SDK crate for module authors.

Every IR struct carries `schema_version: SemVer`. The host enforces compatibility at module load time.

## ⚠️ **Coordinate System**

All `Point2` integer coordinates use **1 scaled integer unit = 100 nm (10⁻⁴ mm)**. The scaling factor is **10_000** (multiply mm by 10_000 to get units). `f32` fields are in millimeters unless annotated otherwise.
Never construct `Point2` with raw integer literals. Use `Point2::from_mm(x, y)` or `mm_to_units()`.
**This is NOT the same as OrcaSlicer**, which uses 1 unit = 1 nm factor 1_000_000). When porting any OrcaSlicer coordinate constant, divide it by 100. See `./docs/08_coordinate_system.md` for the full reference including a conversion table and porting checklist.

## Coordinate Precision & Determinism (Normative)

Canonical conversion rules:

- mm → units: `units = round(mm * 10_000.0)` (round half away from zero).
- units → mm: `mm = units / 10_000.0`.

Determinism bounds:

- One conversion round-trip (`units -> mm -> units`) must be identity.
- One float round-trip (`mm -> units -> mm`) has bounded error `<= 0.00005 mm`.
- Any pipeline step that accumulates more than `0.001 mm` absolute error in one axis across one layer is a contract violation.

Invalid numeric values:

- `NaN` and `±Inf` in any config or IR numeric field are fatal validation errors.
- Denormal/subnormal values must be normalized to zero at parse time.

## Canonical ID Types (Normative)

All IDs below are stable for one slice command and must be deterministic across repeated runs with identical inputs/config.

```rust
pub type ObjectId = String;        // UUID string
pub type ModifierId = String;      // UUID string
pub type ModuleId = String;        // reverse-domain id (e.g. com.example.module)

pub type SurfaceGroupId = u64;
pub type BridgeRegionId = u64;
pub type OverhangRegionId = u64;
pub type RegionId = u64;
```

WIT bridge rule:

- WIT represents `object-id`/`region-id` as strings.
- Canonical host mapping is:
  - `ObjectId`: UUID string, passed through unchanged.
  - `RegionId`: decimal string serialization of `u64` with no leading zeros.
- Any non-canonical region-id string from a module is rejected as fatal contract error.

Bounds and overflow policy:

- `GlobalLayer.index` must be `< 100_000`; host rejects plans above this budget.
- Region/group IDs are allocated from monotonic counters and must never be reused for different geometry within a single slice.
- ID collisions are fatal contract errors.

---

## IR 0 — MeshIR

**Produced by:** Host mesh loader  
**Consumed by:** PrePass stages (read-only via host-services API; never passed directly to modules)

```rust
pub struct MeshIR {
    pub schema_version: SemVer,
    pub objects: Vec<ObjectMesh>,
    pub build_volume: BoundingBox3,
}

pub struct ObjectMesh {
    pub id: ObjectId,                       // stable UUID string
    pub mesh: IndexedTriangleSet,           // host-owned; serialized to WASM only for single-pass modules
                                               // (PaintSegmentation, MeshSegmentation, SeamPlanning,
                                               // SupportGeneration); not serialized for multi-pass
                                               // per-layer modules
    pub transform: Transform3d,             // world-space placement (column-major f64)
    pub config: ObjectConfig,               // raw user config, not yet override-resolved
    pub modifier_volumes: Vec<ModifierVolume>,
    pub paint_data: Option<FacetPaintData>, /// All user-painted data for this object. None if the user has not applied any paint to this object.
}

/// All paint layers on one object. Each layer carries one semantic
/// (material, fuzzy skin, support enforcer/blocker, or custom).
/// Multiple semantics may be painted simultaneously on the same mesh.
pub struct FacetPaintData {
    pub layers: Vec<PaintLayer>,
}

pub struct PaintLayer {
    pub semantic: PaintSemantic,
    /// One entry per mesh triangle, parallel to mesh.triangles.
    /// None = unpainted — inherits default behavior for this semantic.
    pub facet_values: Vec<Option<PaintValue>>,
    /// Sub-facet strokes that cross triangle boundaries.
    /// Resolved into whole-triangle values by PrePass::MeshSegmentation.
    pub strokes: Vec<PaintStroke>,
}

pub enum PaintSemantic {
    /// Which tool/filament to use for this surface region.
    Material,
    /// Apply fuzzy skin texture to this surface region.
    FuzzySkin,
    /// Force support generation in this region regardless of overhang angle.
    SupportEnforcer,
    /// Block support generation in this region regardless of overhang angle.
    SupportBlocker,
    /// Community-defined semantic. String format is mandatory:
    /// `<module-id>/<semantic-name>@<major>`
    /// Example: `com.example.texture/roughness@1`
    ///
    /// Compatibility rule: identical string => identical semantic contract.
    /// Changing value domain or meaning requires incrementing `@<major>`.
    /// Host validates format at startup.
    /// Unknown semantics are preserved in the IR and ignored by stages
    /// that do not recognize them.
    Custom(String),
}

pub enum PaintValue {
    /// Boolean flag (FuzzySkin: on/off, SupportEnforcer/Blocker: on/off)
    Flag(bool),
    /// Scalar (painted variable infill density, etc.)
    Scalar(f32),
    /// Tool/material index (Material semantic only)
    ToolIndex(u32),
}

pub struct PaintStroke {
    /// 3D triangles on the mesh surface defining the painted region.
    /// Produced by the frontend's brush projection onto the mesh.
    pub triangles: Vec<[Point3; 3]>,
    pub semantic: PaintSemantic,
    pub value: PaintValue,
}

pub struct ModifierVolume {
    pub id: ModifierId,
    pub mesh: IndexedTriangleSet,
    pub config_delta: ConfigDelta,          // only explicitly set fields
    pub priority: u32,                      // higher wins when modifiers overlap
    pub applies_to: ModifierScope,
}

pub enum ModifierScope {
    AllFeatures,
    Infill,
    Perimeters,
    Support,
    LayerHeight,
}

/// Sparse config delta. Only explicitly set fields. Never has defaults baked in.
pub struct ConfigDelta {
    pub fields: HashMap<ConfigKey, ConfigValue>,
}
```

### Modifier Resolution Contract

Modifier deltas are merged deterministically during planning:

1. Start with global defaults.
2. Apply object config.
3. Apply matching modifiers sorted by `(priority desc, load_order asc)`.
4. Apply explicit layer-range overrides.

For the same key, the last applied value wins. If a later layer-range override omits a key,
the previously resolved value remains unchanged (no implicit reset).

Worked example (deterministic):

- Global `infill_density = 0.20`
- Object config `infill_density = 0.25`
- Modifier A (`priority=20`) sets `infill_density = 0.30`
- Modifier B (`priority=10`) sets `infill_density = 0.15`
- Layer override for layers `40..60` sets `infill_density = 0.35`
- Effective result: layers outside `40..60` use `0.30`; layers inside `40..60` use `0.35`.

Worked example (invalid):

- Two overlapping modifiers with equal `priority` and equal `load_order` assign different values to the same key.
- Result: planning rejects configuration as non-deterministic modifier precedence.

`load_order` assignment (normative):

- `load_order` is assigned by stable sort on manifest `module.id` (ascending UTF-8 byte order).
- Filesystem traversal order must never influence `load_order`.
- Ties on identical `module.id` are fatal manifest-identity errors.

## IR 2 — SurfaceClassificationIR

**Stage:** Output of `PrePass::MeshAnalysis`  
**Lifetime:** Blackboard (immutable after PrePass)

```rust
pub struct SurfaceClassificationIR {
    pub schema_version: SemVer,
    pub per_object: HashMap<ObjectId, ObjectSurfaceData>,
}

pub struct ObjectSurfaceData {
    /// Indexed parallel to ObjectMesh.mesh.triangles
    pub facet_classes: Vec<FacetClass>,
    pub surface_groups: Vec<SurfaceGroup>,
    pub bridge_regions: Vec<BridgeRegion>,
    pub overhang_regions: Vec<OverhangRegion>,
}

pub enum FacetClass {
    Normal,
    NearHorizontal { slope_angle_deg: f32 },
    Overhang       { angle_deg: f32 },
    Bridge,
    TopSurface,
    BottomSurface,
}

pub struct SurfaceGroup {
    pub id: SurfaceGroupId,
    pub facet_indices: Vec<u32>,
    pub z_min: f32,
    pub z_max: f32,
    pub area_mm2: f32,
    pub printable: bool,
    pub shell_count: u32,
}

pub struct BridgeRegion {
    pub id: BridgeRegionId,
    pub facet_indices: Vec<u32>,
    pub bridge_direction_deg: f32,        // optimal bridge angle
}

pub struct OverhangRegion {
    pub id: OverhangRegionId,
    pub facet_indices: Vec<u32>,
    pub max_angle_deg: f32,
    pub needs_support: bool,
}
```

---

## IR 3 — LayerPlanIR

**Stage:** Output of `PrePass::LayerPlanning`  
**Lifetime:** Blackboard (immutable after PrePass)  
**Critical:** This is the authoritative Z-plane sequence. Every downstream stage derives its Z from here.

```rust
pub struct LayerPlanIR {
    pub schema_version: SemVer,
    pub global_layers: Vec<GlobalLayer>,
    pub object_participation: HashMap<ObjectId, Vec<ObjectLayerRef>>,
}

pub struct GlobalLayer {
    pub index: u32,
    pub z: f32,
    pub active_regions: Vec<ActiveRegion>,
    pub has_nonplanar: bool,
    /// True if multiple objects with different layer heights align at this Z (LCM point)
    pub is_sync_layer: bool,
}

pub struct ActiveRegion {
    pub object_id: ObjectId,
    pub region_id: RegionId,
    /// Fully resolved, fully defaulted config for this region at this layer.
    /// Single source of truth — no runtime fallback chain.
    pub resolved_config: ResolvedConfig,
    pub effective_layer_height: f32,
    pub nonplanar_shell: Option<NonPlanarShellRef>,
    /// True if this region skipped the previous global Z and is catching up
    pub is_catchup_layer: bool,
    pub catchup_z_bottom: f32,
    /// Tool/filament index for this region.
    /// 0 = default tool. Set by PrePass::PaintSegmentation when
    /// Material paint is present; otherwise always 0.
    pub tool_index: u32,
}

pub struct NonPlanarShellRef {
    pub surface_group_id: SurfaceGroupId,
    pub shell_index: u32,          // 0 = top surface, 1..N = internal shells
}

pub struct ObjectLayerRef {
    pub local_layer_index: u32,
    pub global_layer_index: u32,
    pub effective_layer_height: f32,
}

/// The fully resolved, typed config for one region at one layer.
/// Generated by merging: global config → object config → modifier config → layer-range override.
/// Merge is ordered and deterministic. Last writer wins per key.
/// Layer-range overrides only affect explicitly provided keys.
/// Option<T> fields are contributed by optional modules; None if the module is disabled.
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

    // Non-planar (module-contributed; None if non-planar module disabled)
    pub nonplanar_max_angle_deg: Option<f32>,
    pub nonplanar_shell_count: Option<u32>,
    pub nonplanar_amplitude: Option<f32>,

    // Smoothificator (module-contributed)
    pub smoothificator_target_height: Option<f32>,
    pub smoothificator_adaptive: Option<bool>,

    /// Overflow bucket: keys contributed by modules not in the current schema snapshot.
    /// Round-trips safely without corrupting config.
    pub extensions: HashMap<String, ConfigValue>,
}

pub enum WallGenerator { Classic, Arachne }
pub enum SupportType   { Traditional, Tree }
```

### Config Precedence Rules

When two sources assign the same key:

- `layer-range override` > `modifier` > `object config` > `global default`
- Between overlapping modifiers, higher `priority` wins
- On equal modifier priority, first-loaded modifier wins

These rules are the single source of truth for runtime-free config resolution in `LayerPlanIR`.

### Config Float Handling (Normative)

Numeric config requirements:

- Host parses and stores config floats as finite `f64`.
- When consumed by `ResolvedConfig` `f32` fields, conversion must be explicit and clamped only by declared schema bounds.
- If a value cannot be represented without exceeding declared min/max after conversion, startup validation fails.

Reproducibility requirements:

- Config serialization/deserialization must preserve deterministic value selection for all keys affecting geometry.
- Equality for deterministic checks is done on the quantized scaled-int form where applicable, not raw JSON textual formatting.

---

## IR 4 — `PaintRegionIR`

**Stage:** Output of `PrePass::PaintSegmentation` (runs after `PrePass::LayerPlanning`)
**Lifetime:** Blackboard (immutable after PrePass)

```rust
/// Per-layer, per-semantic 2D polygon regions derived from 3D facet paint.
/// All downstream stages query this IR by semantic rather than receiving
/// separate material/fuzzy/support IRs.
pub struct PaintRegionIR {
    pub schema_version: SemVer,
    pub per_layer: HashMap<u32, LayerPaintMap>,
}

pub struct LayerPaintMap {
    pub global_layer_index: u32,
    /// Keyed by semantic. A semantic is absent from the map if no
    /// paint of that type was applied to any object in the scene.
    pub semantic_regions: HashMap<PaintSemantic, Vec<SemanticRegion>>,
}

pub struct SemanticRegion {
    pub object_id: ObjectId,
    pub polygons: Vec<ExPolygon>,
    pub value: PaintValue,
    /// Increasing ordinal used to resolve overlaps for the same semantic.
    /// Higher value means "painted later" and therefore higher precedence.
    pub paint_order: u64,
}

impl PaintRegionIR {
    /// Convenience accessor used by per-layer stage modules.
    /// Returns empty slice if no regions exist for this layer/semantic pair.
    pub fn get(
        &self,
        layer_index: u32,
        semantic: &PaintSemantic,
    ) -> &[SemanticRegion] {
        self.per_layer
            .get(&layer_index)
            .and_then(|l| l.semantic_regions.get(semantic))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}
```

### Paint Region Resolution Contract

- `PaintRegionIR` is the canonical source for all paint semantics in downstream stages.
- If no paint exists for `(layer, semantic)`, `get(...)` returns an empty slice.
- For overlapping regions with the same semantic, highest `paint_order` wins.
- Different semantic families do not override each other (for example `Material` and `FuzzySkin` can both apply at one point).
- For support logic conflicts at the same point: `SupportBlocker` takes precedence over `SupportEnforcer`.
- Material paint resolution is consumed via `ActiveRegion.tool_index` and `WallLoop.boundary_type`.
- For overlapping `PaintSemantic::Custom` values at the same point, highest `paint_order` wins; equal `paint_order` is a fatal deterministic conflict.

Worked example (support overlap):

- Region R has both `SupportEnforcer=true` and `SupportBlocker=true` at one point.
- Effective result: no support generated at that point (`SupportBlocker` precedence).

Worked example (custom overlap):

- `Custom(com.example.texture/roughness@1)` has two overlapping polygons.
- If `paint_order` is `12` and `19`, value from `19` is used.
- If both are `19` with different values, host raises a fatal deterministic conflict.

---

## IR 5 — RegionMapIR

**Stage:** Output of `PrePass::RegionMapping` (host-built-in)  
**Lifetime:** Blackboard (immutable after PrePass)

```rust
pub struct RegionMapIR {
    pub schema_version: SemVer,
    pub entries: HashMap<RegionKey, RegionPlan>,
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct RegionKey {
    pub global_layer_index: u32,
    pub object_id: ObjectId,
    pub region_id: RegionId,
}

pub struct RegionPlan {
    pub config: ResolvedConfig,
    /// Ordered module invocations per stage, pre-sorted by DAG topo sort.
    pub stage_modules: HashMap<StageId, Vec<ModuleInvocation>>,
}

pub struct ModuleInvocation {
    pub module_id: ModuleId,
    /// Pre-filtered config: only keys this module declared it reads.
    pub config_view: ConfigView,
}
```

---

## IR 6 — SliceIR

**Stage:** Output of `Layer::Slice`, mutated by `Layer::SlicePostProcess`

**Current schema_version: 1.1.0** (additive-minor bump from 1.0.0 in packet `12-rev1_external-surface-classification-at-slice`; new fields default `false` when classification data is absent or the region falls outside the Z window).

```rust
pub struct SliceIR {
    pub schema_version: SemVer,
    pub global_layer_index: u32,
    pub z: f32,
    pub regions: Vec<SlicedRegion>,
}

pub struct SlicedRegion {
    pub object_id: ObjectId,
    pub region_id: RegionId,
    /// Closed polygon islands. Coordinates in scaled integers (nanometers).
    pub polygons: Vec<ExPolygon>,
    /// Inset polygons available for infill (updated by Perimeters stage).
    pub infill_areas: Vec<ExPolygon>,
    pub nonplanar_surface: Option<SurfaceGroupId>,
    pub effective_layer_height: f32,
    /// Paint region membership for points on polygon contour boundaries.
    /// Written by the PaintRegionAnnotator module in Layer::SlicePostProcess.
    /// Outer Vec: one entry per polygon in `polygons`.
    /// Middle Vec: one entry per contour point in that polygon's contour.
    /// Inner value: the paint value at that point for this semantic, or None.
    /// Empty map if no paint data applies to this region at this layer.
    pub boundary_paint: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
    /// True when this region lies on the topmost exposed surface of the object
    /// at this layer (i.e. at least one mesh facet classed TopSurface has a
    /// vertex inside the region polygon and the layer falls within the
    /// top-surface Z window).  Written by `classify_region_surfaces` in
    /// `crates/slicer-host/src/layer_slice.rs`.  Defaults `false` when
    /// `SurfaceClassificationIR` is absent or the region is out of window.
    pub is_top_surface: bool,
    /// True when this region lies on the bottommost exposed surface of the
    /// object at this layer (same vertex-in-polygon test against BottomSurface
    /// facets and the bottom-surface Z window).  Defaults `false`.
    pub is_bottom_surface: bool,
    /// True when this region spans a bridge gap at this layer (at least one
    /// `BridgeRegion` Z span covers the layer and a region vertex falls inside
    /// the bridge polygon).  Defaults `false`.  Note: `bridge_regions` is
    /// currently initialized empty in
    /// `crates/slicer-host/src/mesh_analysis.rs:213`; production runs always
    /// see `false` until packet 36 populates bridge detection (see DEV-035).
    pub is_bridge: bool,
}

/// Polygon with holes. Contour is CCW; holes are CW.
pub struct ExPolygon {
    pub contour: Polygon,
    pub holes: Vec<Polygon>,
}

pub struct Polygon {
    pub points: Vec<Point2>,
}

pub struct Point2 { pub x: i64, pub y: i64 }   // scaled integer nanometers
```

---

## IR 7 — PerimeterIR

**Stage:** Output of `Layer::Perimeters`, mutated by `Layer::PerimetersPostProcess`

```rust
pub struct PerimeterIR {
    pub schema_version: SemVer,
    pub global_layer_index: u32,
    pub regions: Vec<PerimeterRegion>,
}

pub struct PerimeterRegion {
    pub object_id: ObjectId,
    pub region_id: RegionId,
    pub walls: Vec<WallLoop>,
    /// Remaining area after wall insets — consumed by Infill stage.
    pub infill_areas: Vec<ExPolygon>,
    pub seam_candidates: Vec<SeamCandidate>,
    pub resolved_seam: Option<SeamPosition>,
}

/// **Seam-first contract (normative):**
/// After `Layer::PerimetersPostProcess` completes, `walls[i].path.points[0]` is the
/// first vertex of the seam-started wall loop. Printing begins at the seam and ends
/// at the seam — the path sequence starts at `points[0]` and the last emitted vertex
/// joins cleanly to `points[0]` to close the loop.
///
/// The `resolved_seam` field stores a diagnostic reference to the seam position.
/// The canonical seam-first geometry is stored in `WallLoop.path.points[0]`.
///
/// Rotation invariant: when `Layer::PerimetersPostProcess` rotates a wall loop so
/// the seam becomes the first vertex, `path.points`, `feature_flags`, and
/// `width_profile.widths` are all rotated together and must maintain equal cardinality.
/// `wall_index` in `SeamPosition` identifies which wall loop carries the seam.

pub struct WallLoop {
    pub perimeter_index: u32,     // 0 = outermost
    pub loop_type: LoopType,
    pub path: ExtrusionPath3D,
    pub width_profile: WidthProfile,
    /// Per-vertex feature flags, parallel to path.points.
    /// A segment from vertex i to vertex i+1 uses flags[i].
    /// Empty vec means no feature paint applies to this loop —
    /// all segments use default behavior.
    /// Populated by the perimeter generator from SlicedRegion.boundary_paint.
    pub feature_flags: Vec<WallFeatureFlags>,
    /// Identifies whether this wall is adjacent to another material region.
    /// Set by the perimeter generator when PaintRegionIR contains Material
    /// regions at this layer.
    pub boundary_type: WallBoundaryType,
}

pub struct WallFeatureFlags {
    /// Tool override for this segment. None means use region default tool_index.
    pub tool_index: Option<u32>,
    /// Enables fuzzy skin modulation on this segment.
    pub fuzzy_skin: bool,
    /// Segment is bridge-like and may require bridge handling.
    pub is_bridge: bool,
    /// Segment belongs to thin-wall logic.
    pub is_thin_wall: bool,
    /// If true, force skip of ironing behavior on this segment.
    pub skip_ironing: bool,
    /// Custom paint values keyed by PaintSemantic::Custom module ID.
    pub custom: HashMap<String, PaintValue>,
}

pub enum WallBoundaryType {
    /// Outer wall facing air or a gap.
    ExteriorSurface,
    /// Wall adjacent to a different material region.
    /// `adjacent_tool` is the tool index of the neighboring region.
    MaterialBoundary { adjacent_tool: u32 },
    /// Inner wall — no special boundary handling.
    Interior,
}

pub enum LoopType { Outer, Inner, ThinWall, NonPlanarShell }

/// Variable-width profile (Arachne). Constant-width = all values equal.
pub struct WidthProfile {
    pub widths: Vec<f32>,         // one per vertex in path.points
}

/// 3D extrusion path. For purely planar layers all z values equal layer z.
/// Non-planar and smoothificator modules write non-uniform z values.
pub struct ExtrusionPath3D {
    pub points: Vec<Point3WithWidth>,
    pub role: ExtrusionRole,
    pub speed_factor: f32,
}

pub struct Point3WithWidth {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub width: f32,           // local extrusion width in mm
    pub flow_factor: f32,     // multiplier on base extrusion volume
}

pub struct SeamCandidate {
    pub position: Point3WithWidth,
    pub score: f32,
    pub reason: SeamReason,
}

pub enum SeamReason { Concave, Aligned, UserForced, Sharp }

pub struct SeamPosition {
    /// Diagnostic reference: the position of the resolved seam on the wall loop.
    /// The canonical seam-first geometry is stored in `WallLoop.path.points[0]`
    /// — `point` here is provided for diagnostics and validation only.
    pub point: Point3WithWidth,
    /// Which wall loop carries this seam (index into `PerimeterRegion.walls`).
    pub wall_index: u32,
}
```

---

## IR 8 — InfillIR

**Stage:** Output of `Layer::Infill`, mutated by `Layer::InfillPostProcess`

```rust
pub struct InfillIR {
    pub schema_version: SemVer,
    pub global_layer_index: u32,
    pub regions: Vec<InfillRegion>,
}

pub struct InfillRegion {
    pub object_id: ObjectId,
    pub region_id: RegionId,
    pub sparse_infill: Vec<ExtrusionPath3D>,
    pub solid_infill: Vec<ExtrusionPath3D>,
    pub ironing: Vec<ExtrusionPath3D>,
}
```

---

## IR 9 — SupportIR

**Stage:** Output of `Layer::Support`, mutated by `Layer::SupportPostProcess`

```rust
pub struct SupportIR {
    pub schema_version: SemVer,
    pub global_layer_index: u32,
    pub support_paths:    Vec<ExtrusionPath3D>,
    pub interface_paths:  Vec<ExtrusionPath3D>,
    pub raft_paths:       Vec<ExtrusionPath3D>,
    pub ironing_paths:    Vec<ExtrusionPath3D>, 
}
```

---

## IR 9b — SupportPlanIR

**Stage:** Output of `PrePass::SupportGeometry` (optional; only present when a
`support-planner` module is loaded)

**Producer:** A module holding the `support-planner` claim on `PrePass::SupportGeometry`;
guests of `PrePass::SupportGeometry` emit `SupportPlanIR` via `run-support-geometry`;
the host built-in commits `SupportGeometryIR` first within the same stage
(e.g. the bundled `support-planner` core module — a simplified port of OrcaSlicer's
`TreeSupport::detect_overhangs` + `TreeSupport::drop_nodes`).

**Consumers:** `Layer::Support` modules that declare `SupportPlanIR` as a read in
their manifest (notably `tree-support`). Modules whose algorithm is inherently
per-layer (e.g. `traditional-support`'s scan-line filler) intentionally do not
read this IR.

```rust
pub struct SupportPlanIR {
    pub schema_version: SemVer,
    /// One entry per active `(global_layer_index, object_id, region_id)` triple
    /// that received planned branches. Multiple entries may share a `(layer,
    /// object)` when an object has multiple regions on the same layer.
    pub entries: Vec<SupportPlanEntry>,
}

pub struct SupportPlanEntry {
    pub global_layer_index: u32,
    pub object_id: ObjectId,
    pub region_id: RegionId,
    /// Pre-planned organic branch geometry. Each `ExtrusionPath3D` is typically
    /// a two-point segment (one MST edge between propagated contact points)
    /// but may be multi-point for long merged branches. Points carry mm-valued
    /// `Point3WithWidth` data and are emitted with `ExtrusionRole::SupportMaterial`.
    pub branch_segments: Vec<ExtrusionPath3D>,
}
```

**Consumption pattern — tree-support precedence:**

For each `(layer, object, region)` reached during `Layer::Support` dispatch, a
plan-aware module must:

1. Look up `SupportPlanIR.entries` matching `(global_layer_index, object_id,
   region_id)` (e.g. via the SDK's `PaintRegionLayerView::support_plan_segments_for(...)`
   accessor).
2. If at least one entry's `branch_segments` is non-empty: emit those segments
   directly with `ExtrusionRole::SupportMaterial` and skip the per-layer filler
   for that region.
3. Otherwise: fall back to the module's own per-layer filler (e.g. tree-support's
   grid-MST sample-and-merge path).

This ordering preserves byte-for-byte fallback behavior when no `support-planner`
module is installed, while enabling organic multi-layer branch geometry when
one is loaded.

**Determinism:** Identical PrePass inputs must produce byte-identical
`SupportPlanIR` (`entries.len()`, every entry's `branch_segments.len()`, and
every endpoint coordinate). The host-side prepass ceremony round-trips this via
the `support_planner_is_deterministic_across_runs` test.

---

## IR 10 — LayerCollectionIR

**Stage:** Output of `Layer::PathOptimization`

**Ownership lifecycle — three phases:**

1. **During parallel per-layer execution:**
   `Layer::PathOptimization` produces a **complete, self-consistent**
   `LayerCollectionIR` for its layer. All entities are ordered, all z-hops
   and intra-layer tool changes are resolved. Nothing is left partially
   populated for a later stage to finish. The completed struct is written
   into a `SlotVec<LayerCollectionIR>` slot (one slot per global layer index)
   inside the Blackboard. Each slot is written exactly once by the thread
   that processed that layer — no mutex required.

2. **After the rayon join — `PostPass::LayerFinalization`:**
   Ownership of all `LayerCollectionIR` values is **moved out of the
   Blackboard** into a plain `Vec<LayerCollectionIR>` owned by the
   finalization executor. The `Arc<SlotVec>` in the Blackboard becomes
   unreachable at this point. The finalization executor holds exclusive
   mutable ownership of the `Vec` and is single-threaded — no `RwLock`
   or `Mutex` is needed. Finalization modules may append entities to
   existing layers or insert new synthetic layers (e.g. wipe tower slices).

3. **After finalization — `PostPass::GCodeEmit` onward:**
   The `Vec<LayerCollectionIR>` is passed as `&[LayerCollectionIR]`
   (immutable slice) to `execute_postpass`. It is never re-entered into
   the Blackboard. GCodeEmit reads it sequentially and the Vec is dropped
   after emission completes.

**Consequence for module authors:**
A module in `PostPass::LayerFinalization` receives a mutable view of the
full layer sequence. A module in any per-layer stage receives only the
current layer and cannot see or modify any other layer's `LayerCollectionIR`.

```rust
pub struct LayerCollectionIR {
    pub schema_version: SemVer,
    /// Signed to support raft prefix layers. Raft entries use negative indices                                                                       
    /// (`-1, -2, ..., -raft_layers`) so raft always sorts before model layers.                                                                       
    pub global_layer_index: i32, 
    pub z: f32,
    /// Ordered, ready-to-emit extrusion entities.
    /// Produced by travel minimization + DAG topo sort.
    pub ordered_entities: Vec<PrintEntity>,
    pub tool_changes: Vec<ToolChange>,
    pub z_hops: Vec<ZHop>,
}

pub struct PrintEntity {
    pub path: ExtrusionPath3D,
    pub role: ExtrusionRole,
    pub region_key: RegionKey,
    pub topo_order: u32,       // guaranteed predecessors appear earlier in ordered_entities
}

pub struct ToolChange {
    pub after_entity_index: u32,
    pub from_tool: u32,
    pub to_tool: u32,
}

pub struct ZHop {
    pub after_entity_index: u32,
    pub hop_height: f32,
}

pub enum ExtrusionRole {
    OuterWall, InnerWall, ThinWall,
    TopSolidInfill, BottomSolidInfill, SparseInfill,
    SupportMaterial, SupportInterface,
    WipeTower, PrimeTower,
    Ironing, BridgeInfill,
    Custom(String),    // community modules may register new roles
}
```

---

## IR 11 — GCodeIR

**Stage:** Output of `PostPass::GCodeEmit`, mutated by `PostPass::GCodePostProcess`

```rust
pub struct GCodeIR {
    pub schema_version: SemVer,
    pub commands: Vec<GCodeCommand>,
    pub metadata: PrintMetadata,
}

pub enum GCodeCommand {
    Move {
        x: Option<f32>, y: Option<f32>, z: Option<f32>,
        e: Option<f32>, f: Option<f32>,
        role: ExtrusionRole,
    },
    Retract    { length: f32, speed: f32 },
    Unretract  { length: f32, speed: f32 },
    FanSpeed   { value: u8 },
    Temperature { tool: u32, celsius: f32, wait: bool },
    ToolChange  { from: u32, to: u32 },
    Comment     { text: String },
    Raw         { text: String },       // escape hatch for printer-specific codes
}

pub struct PrintMetadata {
    pub estimated_print_time_s: u32,
    pub filament_used_mm: Vec<f32>,     // one per tool
    pub layer_count: u32,
    pub slicer_version: String,
}
```

---

## IR Versioning Contract

| Change Type              | Version Bump      | Backward Compatible                |
|--------------------------|-------------------|------------------------------------|
| New optional field added | Minor (1.0 → 1.1) | Yes — old modules ignore it        |
| Field renamed            | Major (1.x → 2.0) | No — requires compatibility shim   |
| Field type changed       | Major (1.x → 2.0) | No — requires compatibility shim   |
| Field removed            | Major (1.x → 2.0) | No — requires compatibility shim   |
| New enum variant         | Minor (1.0 → 1.1) | Yes — old modules treat as unknown |

The `extensions: HashMap<String, ConfigValue>` field on `ResolvedConfig` is the soft landing zone for config keys contributed by modules not present in the host's schema snapshot. Keys always round-trip safely.

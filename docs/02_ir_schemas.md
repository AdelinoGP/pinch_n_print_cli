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
**Current schema_version: 1.1.0** (Bumped to 1.1.0 by packet 56b — populated `modifier_volumes` from `Metadata/model_settings.config`.)

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

```

### 3MF paint-metadata extraction

The host 3MF loader (`model_loader.rs::parse_3mf_model_xml`) recognizes four
paint attributes on `<triangle>` elements in 3MF model XML. Each attribute
maps to one or more `PaintSemantic` layers via the TriangleSelector hex-encoded
state values described below.

#### TriangleSelector hex-encoded state values

The attribute string is decoded as a whole-facet state value:

- Empty string (attribute present but empty) → state 0 (unpainted; treated as `None`).
- Single hex character:
  - `"4"` → state 1
  - `"8"` → state 2
- Two hex characters (encoded as `byte = nibble_high << 4 | nibble_low`):
  - `"0C"` → state 3, `"1C"` → state 4, `"2C"` → state 5, … up to `"DC"` → state 16.
- Strings longer than two characters represent subdivision: a hex-encoded
  recursive tree of sub-triangle states. Packet 50a added the decoder.
  The dominant state across the sub-tree is stamped onto `facet_values[i]`;
  per-leaf 3D triangle geometry for subdivided facets is captured in
  `PaintLayer.strokes` (see "Stroke geometry" below).

#### Channel decode contracts

| 3MF attribute | Valid states | `PaintSemantic` mapping |
|---|---|---|
| `paint_fuzzy_skin` | 1 only | state 1 → `PaintValue::Flag(true)` (`PaintSemantic::FuzzySkin`) |
| `paint_supports` | 1, 2 | state 1 → `PaintSemantic::SupportEnforcer`; state 2 → `PaintSemantic::SupportBlocker` |
| `paint_seam` | 1, 2 | state 1 → `PaintSemantic::Custom("seam_enforcer")`; state 2 → `PaintSemantic::Custom("seam_blocker")` |
| `paint_color` | 1–16 | state N → `PaintValue::ToolIndex(N)` (`PaintSemantic::Material`) |

Channel-specific constraints:

- `paint_fuzzy_skin`: only state 1 is valid; any other state is rejected with `ModelLoadError::PaintMetadata`.
- `paint_supports`: only states 1 and 2 are valid; any other state is rejected.
- `paint_seam`: only states 1 and 2 are valid; any other state is rejected.
- `paint_color`: states 1–16 are valid (extruder indices). States greater than 16 and subdivision strings are rejected.

#### Multiple layers

`paint_supports` can produce up to two `PaintLayer` entries
(`SupportEnforcer` + `SupportBlocker`).
`paint_seam` can produce up to two `PaintLayer` entries
(`Custom("seam_enforcer")` + `Custom("seam_blocker")`).
All other channels produce at most one layer.

#### Stroke geometry (packet 50a)

`PaintLayer.strokes` is populated **only for subdivided facets**. Whole-facet
attributes (single-character or two-character state strings) produce no stroke
geometry — only a `facet_values[i]` entry — because the entire triangle carries
one paint value and 3D stroke geometry would be redundant.

`PaintStroke.triangles` carries world-space sub-triangle geometry in slicer
units (1 unit = 100 nm); the 3MF document supplies coordinates in millimetres
and the loader applies `mm_to_units()` before commit. The dominant state for a
subdivided facet is determined by leaf-area majority across the decoded
sub-tree and written into `facet_values[i]`. Downstream stages may consume
either source: `Layer::Slice` reads `facet_values` for whole-triangle paint
decisions; `Layer::SlicePostProcess` may consult `strokes` when sub-facet
boundary accuracy matters.

```rust
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
**Current schema_version: 1.1.0** (Bumped to 1.1.0 by packet 36 — new struct `BridgeRegion` and field `bridge_regions: Vec<BridgeRegion>` on `SurfaceClassificationIR`.)

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
    pub anchor_width_mm: f32,             // shortest perpendicular run of contiguous anchor edges (mm)
    pub bridge_length_mm: f32,            // longest unsupported span across the cluster (mm)
    pub expansion_margin_mm: f32,         // frozen at PrePass from MeshAnalysisConfig (mm)
    pub is_valid: bool,                   // pass/fail of min-length + anchor-width filters
    pub xy_footprint: Vec<ExPolygon>,     // facet-cluster XY projection in 100 nm units
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
    /// Multi-layer top-surface window. Default 3. Set per region by
    /// `PrePass::RegionMapping` from `top_shell_layers` config; can be
    /// overridden per object or per paint semantic. **Default deviates
    /// from OrcaSlicer's 4** (packet 35).
    pub top_shell_layers: u32,
    /// Multi-layer bottom-surface window. Default 3 (packet 35).
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
**Current schema_version: 1.1.0** (Minor bump per Packet 51 — additive `paint_overrides` field on `RegionPlan`; prior version was 1.0.0.)

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
    /// Audit trail of paint-semantic config overlays applied to `config`
    /// during `PrePass::RegionMapping`. Each entry records the `ResolvedConfig`
    /// snapshot that was merged in for that semantic. Added in Packet 51
    /// (RegionMapIR schema 1.0.0 → 1.1.0, additive field).
    pub paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>,
}

pub struct ModuleInvocation {
    pub module_id: ModuleId,
    /// Pre-filtered config: only keys this module declared it reads.
    pub config_view: ConfigView,
}
```

### Config Key Namespaces

Config keys follow a structured namespace convention used in `ResolvedConfig` and print-profile JSON:

- `object_config:<id>:<key>` — per-object override for the object whose `ObjectId` matches `<id>`. Recognised since DEV-040 (Packet 35a).
- `paint_config:<semantic>:<key>` — per-paint-semantic override. Applied during `PrePass::RegionMapping` when the region's polygons overlap a `SemanticRegion` in `PaintRegionIR` for the corresponding `PaintSemantic`. Built-in `PaintSemantic` variants serialize as: `material`, `fuzzy_skin`, `support_enforcer`, `support_blocker`. `PaintSemantic::Custom(s)` serializes the inner string `s` verbatim (e.g. `paint_config:ironing:line_width`). Added in Packet 51.

**Override precedence** (lowest → highest):

```
global < per_object (object_config:<id>:<key>) < per_paint_semantic (paint_config:<semantic>:<key>)
```

When multiple paint semantics overlap a single region during `RegionMapping`, the host sorts the contributing semantics by the lexicographic order of `paint_semantic_namespace_key(&PaintSemantic)` ascending and overlays them in that order. The lexicographically-last semantic in sort order overlays last and therefore wins. This RegionMap-stage rule (determines which semantic's config wins in `RegionPlan.config`) is distinct from the `paint_order`-based rule documented in the [Paint Region Resolution Contract](#paint-region-resolution-contract) above, which governs intra-semantic polygon overlap resolution during `PrePass::PaintSegmentation`.

---

## IR 6 — SliceIR

**Stage:** Output of `Layer::Slice`, mutated by `Layer::SlicePostProcess`

**Current schema_version: 1.2.0** (additive-minor bump from 1.0.0 in packet `12-rev1_external-surface-classification-at-slice`; new fields default `false` when classification data is absent or the region falls outside the Z window. Bumped to 1.2.0 by packet 36 — new fields on `SlicedRegion`: `bridge_areas`, `bridge_orientation_deg`.)

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
    /// True when this region spans a bridge gap at this layer.  Defaults `false`.
    /// Populated by mesh analysis (packet 36 / 36-rev1).
    pub is_bridge: bool,
    /// Per-layer expanded bridge polygons (in 100 nm units).  Added in packet 36.
    pub bridge_areas: Vec<ExPolygon>,
    /// Best bridge direction across all valid bridge regions (degrees).  Added in packet 36.
    pub bridge_orientation_deg: f32,
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
    /// **Origin-scoped commit (packet 22):** `resolved_seam` is committed only
    /// on the `PerimeterRegion` whose `(object_id, region_id)` matches the
    /// origin region the seam was placed for. The value is *not* broadcast to
    /// every region sharing the same `region_id` across objects. `seam-placer`
    /// modules must select from `seam_candidates` (a pre-populated
    /// `resolved_seam` is never guaranteed at the time `seam-placer` runs).
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
    /// Overhang quartile classification for wall-family roles
    /// (`OuterWall`/`InnerWall`/`ThinWall`). Populated by the
    /// overhang classifier prepass inside `DefaultGCodeEmitter::emit_gcode`
    /// immediately before per-layer emission; `None` for non-wall roles
    /// and for any path that has not been classified yet. Values are
    /// `1..=4` corresponding to the four overhang speed buckets
    /// (`overhang_1_4_speed` … `overhang_4_4_speed`). Added in packet 57.
    pub overhang_quartile: Option<u8>,
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

## IR 9c — SeamPlanIR

**Stage:** Output of `PrePass::SeamPlanning` (optional; only present when a
`seam-planner` module is loaded — packet 23-rev1).

**Producer:** A module holding the `seam-planner` claim on
`PrePass::SeamPlanning`. The stage is ordered after `PrePass::LayerPlanning`
and before `PrePass::PaintSegmentation`; its prerequisites are
`MeshIR` (via `MeshObjectView` parameters) and `LayerPlanIR`.

**Consumers:** `Layer::PerimetersPostProcess` modules that hold the
`seam-placer` claim. The plan is advisory — `seam-placer` may use it as a
strong prior or fall back to per-layer scoring over `SeamCandidate`s.

```rust
pub struct SeamPlanIR {
    pub schema_version: SemVer,
    /// One entry per planned `(global_layer_index, object_id, region_id)` triple.
    /// **Duplicate key contract:** two entries with identical
    /// `(global_layer_index, object_id, region_id)` are a fatal IR validation
    /// error; the host rejects the plan at commit time.
    pub entries: Vec<SeamPlanEntry>,
}

pub struct SeamPlanEntry {
    pub global_layer_index: u32,
    pub object_id: ObjectId,
    pub region_id: RegionId,
    /// Pre-planned seam vertex on the outermost wall loop, in `Point2` units.
    pub seam_xy: Point2,
    /// Optional rationale tag for diagnostics; e.g. "vertex-cluster", "concave-fit".
    pub reason: Option<String>,
}
```

---

## IR 10 — LayerCollectionIR

**Stage:** Output of `Layer::PathOptimization`
**Current schema_version: 2.0.0** (Major bump by packet 39 — `TravelMove.entity_idx: u32` renamed to `entity_id: u64`; new `entity_id: u64` field on `PrintEntity`. Travel anchors are now decoupled from positional indices, so finalization-stage entity insertion no longer invalidates anchors.)

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
    /// Stable per-layer entity identifier. Assigned at construction by
    /// `LayerEntityIdGen::next()`; never reused within a single
    /// `LayerCollectionIR`. Reserved value `0` means "uninitialized" /
    /// sentinel; the generator starts at `1`. Travel anchors and
    /// finalization mutations reference entities by this id, not by
    /// positional index, so inserting or sorting entities cannot
    /// invalidate anchors. Added in packet 39.
    pub entity_id: u64,
}

pub struct TravelMove {
    /// Travel anchor: the entity this travel was emitted before.
    /// Replaces the previous `entity_idx: u32` positional anchor;
    /// the emitter resolves it via an `entity_id -> index` map
    /// built per-layer. Added in packet 39.
    pub entity_id: u64,
    pub from: Point3WithWidth,
    pub to: Point3WithWidth,
    pub speed: f32,
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

### Extrusion-role default priority (Normative)

`ExtrusionRole::default_priority()` returns a `u32` used by
`PostPass::LayerFinalization::push_entity_with_priority` to order entities
inserted into a layer when the inserting module does not supply an explicit
priority. Lower numbers print earlier. Added in packet 40.

| Role                  | `default_priority()` |
|-----------------------|----------------------|
| `Skirt` (`Custom("slicer.builtin/skirt@1")`)        | 100 |
| `Brim`  (`Custom("slicer.builtin/brim@1")`)         | 110 |
| `PrimeTower`          | 200 |
| `WipeTower`           | 210 |
| `OuterWall`           | 300 |
| `InnerWall`           | 310 |
| `ThinWall`            | 320 |
| `BridgeInfill`        | 400 |
| `TopSolidInfill`      | 410 |
| `BottomSolidInfill`   | 420 |
| `SparseInfill`        | 430 |
| `SupportMaterial`     | 500 |
| `SupportInterface`    | 510 |
| `Ironing`             | 900 |
| `Custom(_)` (unknown) | 1000 |

When two entities share a `default_priority` (or two callers pass equal
explicit priorities), insertion order is preserved (stable sort).

### Stable entity IDs (Normative — packet 39)

- `PrintEntity.entity_id: u64` and `TravelMove.entity_id: u64` are populated
  by a single `LayerEntityIdGen` per `LayerCollectionIR`. The generator is
  per-layer and never reused across layers.
- ID `0` is the reserved "uninitialized" sentinel; valid IDs start at `1`.
- Producers in `Layer::Perimeters`, `Layer::Infill`, and `Layer::Support`
  stamp every entity at construction. Finalization (`PostPass::LayerFinalization`)
  stamps fresh IDs on entities it inserts; sorts and inserts never rewrite
  existing IDs.
- `GCodeEmit` resolves travels by building an `entity_id -> index` map per
  layer; lookup is `O(1)` per travel.
- `validate_travel_anchors(layer: &LayerCollectionIR) -> Result<(), ValidateError>`
  short-circuits on the first dangling travel anchor; finalization invokes
  it before the layer is handed off to `PostPass::GCodeEmit`.

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
    /// Retract.
    /// `mode` selects whether the emitter writes a parameterised
    /// `G1 E-<length> F<speed>` (Gcode mode) or a parameterless `G10`
    /// (Firmware mode). Length / speed are still carried in firmware
    /// mode for diagnostics but are not serialized. Added in packet 34.
    Retract    { length: f32, speed: f32, mode: RetractMode },
    /// Unretract — symmetric inverse of Retract.
    /// `mode = Gcode` emits `G1 E<length> F<speed>`; `mode = Firmware`
    /// emits `G11`. M207/M208 are intentionally never emitted —
    /// firmware-side retract tuning is the printer's start G-code's job
    /// (OrcaSlicer parity).
    Unretract  { length: f32, speed: f32, mode: RetractMode },
    FanSpeed   { value: u8 },
    Temperature { tool: u32, celsius: f32, wait: bool },
    ToolChange  { from: u32, to: u32 },
    Comment     { text: String },
    Raw         { text: String },       // escape hatch for printer-specific codes
}

/// Per-command retract / unretract emission mode. Added in packet 34.
/// Default is `Gcode` (preserves packet-15 emission bit-for-bit).
/// Every `Retract` / `Unretract` in a single print carries the same
/// value — the field is per-command for matcher-exhaustiveness rather
/// than for per-command variation.
pub enum RetractMode {
    /// `G1 E-<length> F<speed>` / `G1 E<length> F<speed>`.
    Gcode,
    /// Parameterless `G10` / `G11`. Length / speed in the IR are
    /// carried but not serialized.
    Firmware,
}

pub struct PrintMetadata {
    pub estimated_print_time_s: u32,
    pub filament_used_mm: Vec<f32>,     // one per tool
    pub layer_count: u32,
    pub slicer_version: String,
}
```

### G-code envelope blocks (Normative — packet 55)

`PostPass::GCodeEmit` wraps the per-layer command stream in four canonical
envelope blocks. Block sentinels and ordering are part of the wire-format
contract — frontends and post-processors parse these tokens.

**Envelope sequence (top to bottom of the `.gcode` output):**

```
; HEADER_BLOCK_START
;   <semicolon-prefixed metadata lines: model name, layer count, filament
;    used, max Z, slicer version, etc.>
; HEADER_BLOCK_END
; THUMBNAIL_BLOCK_START                          (only when --thumbnail set)
;   <Base64-encoded PNG, 76 chars per line, each prefixed with "; ">
; THUMBNAIL_BLOCK_END
; ; <per-role width comments, e.g. "; outer_wall_width = 0.42">
<machine_start_gcode expanded — packet 59>
M83  (or M82 — packet 54)
<per-layer ;TYPE: blocks with G1/G0 moves>
<machine_end_gcode expanded — packet 59>
; CONFIG_BLOCK_START
;   <serialized ResolvedConfig as `; key = value` per line>
; CONFIG_BLOCK_END
```

**Block-ordering rules (normative):**

1. `HEADER_BLOCK_*` and `THUMBNAIL_BLOCK_*` precede the first `;TYPE:` block.
2. `CONFIG_BLOCK_*` follows the last `;TYPE:` block and is the final
   semicolon-prefixed content in the file.
3. The machine start / end G-code wraps the layer stream but sits *inside*
   the envelope — header/thumbnail come first, config-dump comes last
   (OrcaSlicer parity).

**Thumbnail format:**

- Triggered by `--thumbnail <path>` CLI flag pointing to a PNG file.
- Bytes are validated against the PNG magic header (`\x89PNG\r\n\x1a\n`);
  non-PNG inputs are a fatal error.
- Base64-encoded with 76 characters per line, each line prefixed by `"; "`,
  matching OrcaSlicer's wire format exactly so downstream tools (printer
  UIs, gcode preview viewers) parse it identically.

**Configurable header fields (config keys, packet 55):**

| Key | Type | Default | Purpose |
|---|---|---|---|
| `filament_diameter` | f32 (mm) | `1.75` | Header `; filament_diameter` line; consumed by some post-processors. |
| `filament_density` | f32 (g/cm³) | `1.24` | Header `; filament_density` line. |
| `max_z_height` | f32 (mm) | `0.0` (auto) | Hard cap reported in header; `0.0` means "use per-print z_max". |
| `thumbnail_path` | string | `""` | Alternative to the `--thumbnail` CLI flag; CLI wins when both set. |

### Stream-level extrusion mode (Normative — packet 54)

`GCodeCommand::Move.e` is a signed delta in **relative** extrusion mode
(M83) and an absolute position in **absolute** mode (M82). Mode is a
stream-level invariant — the emitter writes the appropriate `M82` / `M83`
preamble once per print and resets the E-accumulator with `G92 E0` on
mode change or layer reset. Mode is selected by the config key
`use_relative_e_distances` (boolean; default `true` → M83). Carrier
helper: `DefaultGCodeSerializer::with_extrusion_mode(mode)`.

### Polyline simplification and precision (Normative — packet 60)

Seven `ResolvedConfig` keys control simplification of polyline geometry
at G-code emit and slice-layer finalization. All units are millimetres
unless stated.

| Key                       | Type | Default        | Consumer                                                          |
|---------------------------|------|----------------|-------------------------------------------------------------------|
| `gcode_resolution`        | f32  | `0.0125 mm`    | Per-role Douglas-Peucker tolerance for wall-family / brim roles.  |
| `infill_resolution`       | f32  | `0.0125 mm`    | Per-role tolerance for infill / solid-infill / bridge / top / bottom. |
| `support_resolution`      | f32  | `0.05 mm`      | Per-role tolerance for support material / interface.              |
| `min_segment_length`      | f32  | `0.025 mm`     | Drop adjacent segments shorter than this after D-P.               |
| `gcode_xy_decimals`       | u32  | `3`            | Decimal places for X / Y / Z token formatting (via `format_xyz`). |
| `perimeter_arc_tolerance` | f32  | `0.0025 mm`    | Clipper2 arc-tolerance for `slicer_core::polygon_ops::offset(...)` — read per-module by `classic-perimeters` and `arachne-perimeters`. |
| `slice_closing_radius`    | f32  | `0.0 mm` (off) | Per-layer Clipper2 `inflate(+r) → inflate(-r)` round-trip after `simplify_polygon_points` in `triangle_mesh_slicer`. |

Per-role tolerance dispatch (consumed by `tolerance_for_role` in
`gcode_emit.rs`):

| `ExtrusionRole`                                                   | Tolerance source     |
|-------------------------------------------------------------------|----------------------|
| `OuterWall`, `InnerWall`, `ThinWall`, `Custom("…/brim@1")`        | `gcode_resolution`   |
| `TopSolidInfill`, `BottomSolidInfill`, `SparseInfill`, `BridgeInfill` | `infill_resolution`  |
| `SupportMaterial`, `SupportInterface`                             | `support_resolution` |
| Travel (synthetic — no `ExtrusionRole`), `Custom(_)` (unknown)    | `0.0` (no D-P)       |

Legacy-equivalent mode is `gcode_resolution = infill_resolution = support_resolution = min_segment_length = 0.0`, `gcode_xy_decimals = 4`, `perimeter_arc_tolerance = 0.0`, `slice_closing_radius = 0.0`. Setting all seven to those values produces byte-identical G-code to the pre-packet-60 output.

The `format_xyz(value: f32, decimals: u32) -> String` helper formats the
X / Y / Z tokens; F (feedrate), E (extrusion), and temperature continue
to use the previous `format_coord` (which is byte-identical to its
pre-packet-60 behavior at `{:.4}`).

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

# Pinch 'n Print — Intermediate Representation (IR) Schemas

**What this covers:** every IR struct that crosses the host/module boundary —
its fields, its `schema_version`, and the normative contracts governing how it
is produced and consumed.

**Who it's for:** module authors reading or writing IR, and anyone changing an
IR type (the versioning contract at the end of this file binds you).

**Prerequisites:** `00_project_overview.md` for crate layout;
`01_system_architecture.md` for which stage produces which IR. The coordinate
rules in `08_coordinate_system.md` are assumed throughout — every integer
coordinate below obeys them.

All IRs are defined in `crates/slicer-ir/src/`. They are the shared contract between the host and all modules. IR types are re-exported by the SDK crate for module authors.

The struct definitions in this document are the **normative spec**; the Rust types under `crates/slicer-ir/src/` implement them. If the code and this document disagree, treat the discrepancy as a bug to be filed against whichever side drifted. Canonical source files:

- `crates/slicer-ir/src/slice_ir.rs` — most IR structs (Slice/Perimeter/Infill/Support/LayerCollection/GCode and their nested types).
- `crates/slicer-ir/src/resolved_config.rs` — `ResolvedConfig` and config-merge helpers.
- `crates/slicer-ir/src/entity_id.rs` — `LayerEntityIdGen` and stable-id contract.
- `crates/slicer-ir/src/validation.rs` — `validate_travel_anchors`.

Every IR struct carries `schema_version: SemVer`. The host enforces compatibility at module load time.

## ⚠️ **Coordinate System**

All `Point2` integer coordinates use **1 scaled integer unit = 100 nm (10⁻⁴ mm)**. The scaling factor is **10_000** (multiply mm by 10_000 to get units). `f32` fields are in millimeters unless annotated otherwise.
Never construct `Point2` with raw integer literals. Use `Point2::from_mm(x, y)` or `mm_to_units()`.
**This is NOT the same as OrcaSlicer**, which uses 1 unit = 1 nm (scaling factor 1_000_000). When porting any OrcaSlicer coordinate constant, divide it by 100. See `08_coordinate_system.md` for the full reference including a conversion table and porting checklist.

## Coordinate Precision & Determinism (Normative)

The canonical mm↔units conversion rules and determinism bounds are defined in
`docs/08_coordinate_system.md` § "Conversion & Determinism (Normative)", the
single source of truth for coordinate conventions. They are not restated here.

Invalid numeric values (apply to any config or IR numeric field, not just
coordinates):

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
    pub id: ObjectId,                       // stable UUID string; see "ObjectId derivation" below
    pub mesh: IndexedTriangleSet,           // host-owned; serialized to WASM only for single-pass modules
                                               // (PaintSegmentation, SeamPlanning,
                                               // SupportGeometry); not serialized for multi-pass
                                               // per-layer modules
    pub transform: Transform3d,             // world-space placement (column-major f64)
    pub config: ObjectConfig,               // raw user config + sidecar overlay (see contract below)
    pub modifier_volumes: Vec<ModifierVolume>,
    pub paint_data: Option<FacetPaintData>, /// All user-painted data for this object. None if the user has not applied any paint to this object.
    /// Cached world-space Z extent `(z_min, z_max)` in millimeters, computed at
    /// construction time from the transformed mesh vertices. `None` when the
    /// mesh is empty or the extent is degenerate (`z_max <= z_min`).
    /// Not serialized (`#[serde(skip_deserializing, default)]`) — recomputed on
    /// every load so it always reflects the current `transform`. This makes
    /// world-space Z a first-class IR contract surface.
    pub world_z_extent: Option<(f32, f32)>,
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
    /// Resolved into whole-triangle values by the host loader (split_triangle_strokes).
    pub strokes: Vec<PaintStroke>,
}

```

### ObjectId derivation (reproducibility contract)

`ObjectId` is minted **only** by `path_object_id` in
`crates/slicer-model-io/src/loader.rs`, as:

```text
uuid5(NAMESPACE_OID, "<file basename>#<per-file object index>")
```

e.g. `uuid5(NS_OID, "cube.stl#0")`. STL and OBJ always yield index `0`; a 3MF
yields one id per build item, indexed in document order.

**The key is the basename, never the absolute path.** This is a hard requirement,
not an implementation detail: the id is emitted into shipped G-code as the
`; object_height:<id> = <mm>` config-dump comment, so keying on the absolute path
made G-code **byte-different on every machine and every checkout location**, and
committed goldens only reproduced on the checkout that recorded them. Basename
keying makes ids — and therefore G-code — reproducible across machines, across
checkouts, and across a user moving the model file.

Consequences to respect when changing this:

- **Renaming a model file changes its object ids.** That is intended and
  documented; ids are a function of the name the user gave the file.
- **Two *distinct* files sharing a basename in one job collide.** That is refused
  at load, not silently merged — `slicer_model_io::check_basename_collisions`
  returns `ModelLoadError::DuplicateInputBasename` naming both full paths. There
  is no multi-input job today (`pnp_cli slice` takes exactly one `--model`), so
  this guard currently has no production call site; wire it into whatever future
  code collects several model inputs into one `MeshIR`.
- **Nothing persists an `ObjectId` across runs.** No cache, database, or sidecar
  is keyed on it, so the derivation can be changed without invalidating on-disk
  artifacts. `write_3mf` / `write_obj` embed it only as a cosmetic display name
  and never read it back — `load_model` always re-derives.

Regression coverage: `object_id_is_identical_across_different_absolute_directories`
and `object_id_is_the_documented_basename_uuid5` in
`crates/slicer-model-io/tests/model_loader_tdd.rs`.

### Shared geometry types

```rust
/// 2D axis-aligned bounding box used as spatial pre-filter for paint region
/// point queries. Uses Point2 in native 100 nm units.
/// Computed at harvest time; never serialized.
pub struct BoundingBox2 {
    pub min: Point2,
    pub max: Point2,
}

impl BoundingBox2 {
    /// Returns true if point is within the box (inclusive bounds).
    pub fn contains_point(&self, point: Point2) -> bool { /* ... */ }
}
```

### 3MF paint-metadata extraction

The host 3MF loader (`parse_3mf_model_xml` in
`crates/slicer-model-io/src/loader.rs`) recognizes four
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
  `PaintLayer.strokes` (see "Stroke geometry" below). The tree walker
  enforces a depth guard of **64 recursion levels**; malformed trees
  exceeding this depth are rejected with `ModelLoadError::PaintMetadata`
  containing `"exceeds maximum depth"` (prevents stack overflow on
  pathological input).

#### Channel decode contracts

| 3MF attribute | Valid states | `PaintSemantic` mapping |
|---|---|---|
| `paint_fuzzy_skin` | 1 only | state 1 → `PaintValue::Flag(true)` (`PaintSemantic::FuzzySkin`) |
| `paint_supports` | 1, 2 | state 1 → `PaintSemantic::SupportEnforcer`; state 2 → `PaintSemantic::SupportBlocker` |
| `paint_seam` | 1, 2 | state 1 → `PaintSemantic::Custom("seam_enforcer")`; state 2 → `PaintSemantic::Custom("seam_blocker")` |
| `paint_color` | 1–16 | state N → `PaintValue::ToolIndex(N-1)` (`PaintSemantic::Material`) |

Channel-specific constraints:

- `paint_fuzzy_skin`: only state 1 is valid; any other state is rejected with `ModelLoadError::PaintMetadata`.
- `paint_supports`: only states 1 and 2 are valid; any other state is rejected.
- `paint_seam`: only states 1 and 2 are valid; any other state is rejected.
- `paint_color`: states 1–16 are valid (extruder indices). States greater than 16 and subdivision strings are rejected. **ToolIndex encoding (Packet 50b):** OrcaSlicer encodes 1-based nibble states in 3MF; the loader adjusts to 0-based on commit, so the IR is uniformly 0-indexed (`ToolIndex(0..=15)`).

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
    /// Tool/material index (Material semantic only). 0-based in the IR:
    /// the 3MF `paint_color` state N (1..=16) is decoded to
    /// `ToolIndex(N-1)` by the loader so the IR is uniformly 0-indexed.
    /// The previous `HashablePaintValue` wrapper used at paint
    /// segmentation was removed in Packet 91 — this enum is directly
    /// hashable, per the "`PaintValue` Eq+Hash invariant" section of this
    /// document. The `Custom` variant is intentionally reserved for
    /// per-module user-defined paint values.
    ToolIndex(u32),
    /// Community-defined paint value (string-keyed). Added for parity
    /// with `PaintSemantic::Custom`.
    Custom(String),
}

pub struct PaintStroke {
    /// 3D triangles on the mesh surface defining the painted region.
    /// Produced by the frontend's brush projection onto the mesh.
    pub triangles: Vec<[Point3; 3]>,
    pub semantic: PaintSemantic,
    pub value: PaintValue,
}
```

#### `PaintValue` Eq+Hash invariant (Normative — Packet 91)

`PaintValue` derives `Eq` + `Hash` so it can be used as a `HashMap` key
and as a `RegionKey.variant_chain` element. `Scalar(f32)` is hashed via
`to_bits()`; `Custom(String)` via its String contents; `Flag` and
`ToolIndex` use discriminant + value hashing. This makes the previous
`HashablePaintValue` wrapper (formerly in `paint_segmentation.rs`)
obsolete — code keying `HashMap<PaintValue, _>` directly is the
canonical pattern post-Packet 91. The same `to_bits()` portability
caveat as `ResolvedConfig` applies.

```rust

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

### `ObjectMesh` Assembly Contract (Normative — Packet 75)

All `ObjectMesh` instances are constructed via
`slicer_model_io::loader::assemble_object(mesh, id, paint_data, modifiers, config)`.
Five wrap sites use this single entry point: the STL, OBJ, and 3MF
loader paths in `load_model`, plus the `mesh convert` split
re-assembly. `assemble_object` computes `world_z_extent` from the mesh
and applies the object's transform; for single-component models that
reuse a parent extent during convert's split re-assembly the recompute
is identical under identity transform (locked by AC-4.3 regression in
packet 75). Z-extent logic is centralised here; the convert path's separate
`compute_z_extent_for_component` was deleted in Packet 75 rather than left as a
second implementation. The `assemble_object` symbol (`crates/slicer-model-io/src/loader.rs`)
was promoted from `pub(crate)` to `pub` in Packet 81 to support the CLI's
`helpers_cmd.rs` move into `pnp-cli`.

### `ObjectConfig.data` Population (Normative — Packet 67)

`ObjectConfig.data: HashMap<String, ConfigValue>` is populated during
3MF model loading from object-scoped sidecar metadata. The loader uses a
hand-written, non-data-driven allowlist from each `<object>`'s `<metadata>`
block and seeds admitted keys into the host's `config_source` via the
`object_config:<id>:<key>` pattern documented in §"Config Key Namespaces" of
this document. This is what makes user-specified per-object metadata from 3MF
files reach `RegionMapping` and downstream consumers.

The complete admitted object-level key list is:

- Existing keys: `extruder`, `enable_support`, `support_type`.
- Integer keys: `wall_loops`, `top_shell_layers`, `bottom_shell_layers`,
  `raft_layers`, `support_interface_top_layers`,
  `support_interface_bottom_layers`.
- Rebasing integer keys: `support_filament`,
  `support_interface_filament`.
- Float keys: `layer_height`, `brim_width`, `support_threshold_angle`,
  `support_top_z_distance`.
- String keys: `seam_position`, `sparse_infill_density`,
  `sparse_infill_pattern`, `brim_type`, `fuzzy_skin`,
  `support_base_pattern`.

The 18 Packet 172 additions are the six ordinary integer keys, two rebasing
integer keys, four float keys, and six string keys above. Orca's
`support_filament` and `support_interface_filament` values are 1-indexed and
are rebased to 0-indexed values at load time; raw `0` remains `0`. The existing
`extruder` selector follows the same rebase convention. For
`sparse_infill_density`, a percentage such as `20%` remains a raw
`ConfigValue::String`, while a numeric non-percentage value is a
`ConfigValue::Float`. Unknown object keys are dropped after one debug log per
key; they are not silently accepted or inserted untyped.

### Host-Local Sidecar Types (Normative — Packet 56)

The 3MF sidecar parser at
`crates/slicer-model-io/src/sidecar.rs::parse_3mf_sidecar` produces
host-local types that are NEVER exposed at the WIT boundary or in any
IR contract. They exist to thread per-part metadata from
`Metadata/model_settings.config` through `resolve_object` to
downstream consumers (packets 56b/56c/67).

```rust
/// Five-variant enum carrying the `<part subtype="…">` attribute.
/// Unknown subtype values downgrade to `NormalPart` with `log::warn!`.
pub enum PartSubtype {
    NormalPart,
    ModifierPart,
    NegativePart,
    SupportEnforcer,
    SupportBlocker,
}

pub struct PartSidecarInfo {
    pub subtype: PartSubtype,
    pub metadata: BTreeMap<String, String>,
}

pub struct ObjectSidecarInfo {
    /// Keyed by `<part id>` (u32).
    pub parts: HashMap<u32, PartSidecarInfo>,
    /// Object-scoped `<metadata>` entries that are not nested in a
    /// `<part>` element. Routed into `ObjectConfig.data` and into
    /// every modifier_volume's `config_delta` for that object.
    pub object_metadata: BTreeMap<String, String>,
}

/// Return type of `parse_3mf_sidecar`. Outer key = `<object id>` (u32);
/// inner key = `<part id>` (u32). Mirrors Bambu's documented sidecar
/// XML nesting `<object id="N"><part id="M" subtype="…">`.
pub type SidecarMap = HashMap<u32, ObjectSidecarInfo>;
```

Behaviour:

- **Missing sidecar:** `Metadata/model_settings.config` absent from the
  archive ⇒ empty `SidecarMap` returned silently (no warning).
- **Malformed XML:** parser returns an empty `SidecarMap` and emits
  `log::warn!` on the `slicer_model_io::sidecar` target containing the
  substring `"treating all parts as normal_part"`. `load_model` still
  returns `Ok(MeshIR)`; failure is non-fatal.
- **Unknown subtype attribute:** downgraded to `PartSubtype::NormalPart`
  with `log::warn!`.

Parser plumbing: `parse_3mf_sidecar(&mut zip)` is invoked inside
`load_3mf` after `parse_3mf_model_xml` and before the `ZipArchive` is
dropped. The resulting map is threaded through `parse_3mf_model_xml`
to `resolve_object` for routing (packets 56b/56c).

### `ModifierVolume.config_delta` Sources (Normative — Packets 56b, 67, 68)

`ModifierVolume.config_delta.fields` can be populated from two
distinct sidecar sources in a single 3MF file:

1. **Part-level `<metadata>`** inside a `<part>` element (Packet 56)
   — follows its separate part-level metadata rules and subtype routing; Packet
   172 does not widen that allowlist.
2. **Object-level `<metadata>`** at the `<object>` scope (Packet 67) —
   routed to every `modifier_volume` belonging to that object and governed by
   the complete object-level allowlist above. This object-level list is
   separate from the part-level allowlist and does not widen it.

Subtype-key exclusion (Packet 68): the literal key `subtype` is
routing metadata and is excluded from stamping into
`RegionPlan.config.extensions`; only non-`subtype` keys flow through.
Additionally, modifier volumes whose subtype value is
`"support_enforcer"` or `"support_blocker"` are entirely SKIPPED
during config stamping for OrcaSlicer parity — canonical
`PrintApply.cpp` skips these volume subtypes when applying per-volume
config overrides. Their semantics are exercised via
`PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`
instead, never via `PaintValue::ToolIndex` — see also the
"Support semantics use Flag, never ToolIndex" constraint in IR 4.

`ConfigDelta` semantics:

- Sparse — only explicitly set fields. No baked-in defaults.
- `priority` (deterministic ordering hint): `ModifierPart = 0`,
  `NegativePart = 100`, `SupportEnforcer = 200`, `SupportBlocker = 300`.
  Consumers may ignore and apply their own ordering.
- `applies_to`: for 3MF-sourced volumes, `ModifierScope::AllFeatures`
  scoped to the parent `ObjectId` (the volume applies only to features
  of its parent object, not the whole plate).

### Canonical region-id parser (host-only — Packet 75)

The decimal-`u64` parser `parse_canonical_region_id` lives in
`crates/slicer-wasm-host/src/host.rs` and is the SOLE host validator for
the canonical region-id string format (decimal `u64` with no leading
zeros, no other whitespace or punctuation). It is not part of the
public SDK and must NOT be called by modules. Packet 75 deduplicated
the prior copies and made it `pub(crate)` in one place — any new
caller must call this symbol rather than re-implementing the parse.

## IR 2 — SurfaceClassificationIR

**Stage:** Output of `PrePass::MeshAnalysis`  
**Lifetime:** Blackboard (immutable after PrePass)  
**Current schema_version: 1.2.0** (Bumped to 1.2.0 by packet 106 — `OverhangRegion` gains `xy_footprint`, new type `QuartileBand`, and new field `overhang_quartile_polygons` on `SurfaceClassificationIR`. Previously bumped to 1.1.0 by packet 36 — new struct `BridgeRegion` and field `bridge_regions: Vec<BridgeRegion>` on `SurfaceClassificationIR`.)

```rust
pub struct SurfaceClassificationIR {
    pub schema_version: SemVer,
    pub per_object: HashMap<ObjectId, ObjectSurfaceData>,
    /// Populated by `PrePass::OverhangAnnotation` (packet 106), which atomically
    /// replaces this IR via `replace_surface_classification()`. Key = global layer
    /// index. A layer with no overhang has its key ABSENT (not an empty Vec); layer 0
    /// is always absent. `#[serde(default)]`.
    pub overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>,
}
```

**Consumer note (packet 107):** `overhang_quartile_polygons` is consumed by `SliceRegionView::overhang_areas()` and `SliceRegionView::overhang_quartile_polygons()` (both populated by the host marshaller, keyed by `global_layer_index`; see `docs/05_module_sdk.md` "SliceRegionView accessors (packet 107)"). Per-vertex propagation onto `Point3WithWidth.overhang_quartile` (perimeter-generation side) is now wired on **both** perimeter paths — classic-perimeters (packets 104/107, closing T-024/T-077) and arachne-perimeters (packet 148); `D-104-OVERHANG-QUARTILE-NONE` closed 2026-07-03.

```rust

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
    pub xy_footprint: Vec<ExPolygon>,     // packet 106: per-region 2D projection of the underlying facets, populated at MeshAnalysis, mirrors BridgeRegion.xy_footprint
}

// packet 106: one quartile band of unsupported overhang area for a single layer.
// `quartile` ranges 1..=4 (band 1 nearest support, band 4 most overhanging), computed by
// `annotate_overhangs()` from thresholds at line_width_mm × {0.5, 1.0, 1.5, 2.0}.
pub struct QuartileBand {
    pub quartile: u8,
    pub polygons: Vec<ExPolygon>,
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

pub enum WallGenerator { Classic, Arachne }
pub enum SupportType   { Traditional, Tree }
```

### `ResolvedConfig`

The fully resolved, typed config for one region at one layer. Generated by
merging: global config → object config → modifier config → layer-range override.
The merge is ordered and deterministic, last writer wins per key, and
layer-range overrides only affect explicitly provided keys. `Option<T>` fields
are contributed by optional modules and are `None` when the module is disabled.

**The field list is not reproduced here.** `ResolvedConfig` is generated by the
`declare_resolved_config!` macro invocation in
`crates/slicer-ir/src/resolved_config.rs`, which is the authoritative list of
fields, types, defaults, and per-field extractors. For the config **keys** those
fields bind to — including host-only keys and namespaced module keys — see
`15_config_keys_reference.md`.

Field-shape rules that the macro invocation does not state, and that a reader
must know:

- **`layer_height` and `first_layer_height` are `f64`, deliberately — not
  `f32`.** They feed the layer-Z formula (`z = n * layer_height`). An `f32`
  round-trip re-taints the value and drifts onto an adjacent float at roughly
  every 10th layer, which misses STL vertices stored as `f32(mm_value)` and
  breaks `classify_vertex`'s exact `f32 ==` plane test. Other millimeter fields
  are `f32`.
- **`top_shell_layers` / `bottom_shell_layers` default to 3, which deviates from
  OrcaSlicer's 4** (packet 35). `PrePass::RegionMapping` sets them per region;
  they can be overridden per object or per paint semantic.
- **The four `*_fill_holder` fields select claim holders, not values.** Each
  names the module holding the corresponding fill-role claim for the region
  (default `"rectilinear-infill"`). The claim↔key mapping is in
  `03_wit_and_manifest.md` § "Known claim IDs"; resolution is in
  `04_host_scheduler.md` § "Claim Resolution".
- **`extensions: BTreeMap<String, ConfigValue>` is the overflow bucket** for
  keys contributed by modules outside the current schema snapshot. It
  round-trips without corrupting config. It was migrated from `HashMap` to
  `BTreeMap` in Packet 91 so `ResolvedConfig` can derive `Hash`; deterministic
  iteration order is the upside. The `Hash` impl hashes `f32` fields via
  `to_bits()`, which is consistent within one process.

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

### ResolvedConfig Hash invariant (Normative — Packet 91)

`ResolvedConfig` derives `PartialEq` + `Eq` + `Hash`. All `f32`/`f64`
fields are hashed via `to_bits()` so that `a == b ⇒ hash(a) == hash(b)`
holds (both equality and hashing use bit-pattern comparison, not float
equality). This is required for the Packet 91 interner that dedupes
configs into `RegionMapIR.configs` via linear scan keyed by `==`.

Portability caveat: hash output is consistent within one process but is
NOT portable across architectures with differing NaN bit patterns. Two
configs differing only in NaN payload bit pattern would compare unequal
and intern as distinct entries. NaN is already a fatal validation error
(see top of this doc), so this is theoretical for real prints.

---

## IR 4 — RegionMapIR

**Stage:** Output of `PrePass::RegionMapping` (host-built-in)  
**Lifetime:** Blackboard (immutable after PrePass)  
**Current schema_version: 2.0.0** (Major bump by Packet 91 — `RegionPlan.config` is now a `ConfigId` interner index, `RegionMapIR.configs` Vec added, `RegionKey.variant_chain` added. Prior versions: 1.0.0 initial; 1.1.0 (Packet 51 — additive `paint_overrides` field on `RegionPlan`). RegionMapIR schema remains at 2.0.0 post-roadmap.)

```rust
pub struct RegionMapIR {
    pub schema_version: SemVer,
    pub entries: HashMap<RegionKey, RegionPlan>,
    /// Interned `ResolvedConfig` pool. Each `RegionPlan.config` is an
    /// index into this Vec. Pre-seeded with `ResolvedConfig::default()`
    /// at index 0 so `ConfigId::default()` is always a valid interner
    /// index. Use `intern_config` to add and `config_for` to resolve.
    /// Added in Packet 91.
    pub configs: Vec<ResolvedConfig>,
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct RegionKey {
    pub global_layer_index: u32,
    pub object_id: ObjectId,
    pub region_id: RegionId,
    /// Ordered `(paint_semantic_name, value)` pairs identifying this
    /// region's paint variant. Empty for the legacy single-variant flow;
    /// populated by Packet 93 when RegionMapping cross-product expands
    /// each `(layer, ActiveRegion)` into one `RegionPlan` per canonical
    /// variant chain. Added in Packet 91 (scaffolding).
    pub variant_chain: Vec<(String, PaintValue)>,
}

pub struct RegionPlan {
    /// Interned index into `RegionMapIR.configs`. Resolve via
    /// `RegionMapIR::config_for(&key)`. Bumped from inline
    /// `ResolvedConfig` to `ConfigId` by Packet 91 so that duplicated
    /// configs across painted-variant `RegionPlan`s intern to a single
    /// instance.
    pub config: ConfigId,
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

/// Stable index into `RegionMapIR.configs`. Stable within one
/// `RegionMapIR` only — not portable across IRs. Introduced in Packet 91
/// so duplicated `ResolvedConfig` payloads across painted-variant
/// `RegionPlan`s can intern to a single instance.
#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq)]
pub struct ConfigId(pub u32);

impl RegionMapIR {
    /// Resolves a `RegionKey` to its `ResolvedConfig` via the interner.
    /// Panics if the key is unknown or the `ConfigId` is out of bounds
    /// — both are construction invariants the interner upholds.
    pub fn config_for(&self, key: &RegionKey) -> &ResolvedConfig { /* ... */ }

    /// Interns a `ResolvedConfig`, returning the existing `ConfigId` if
    /// an equal config is already present (linear scan — bounded by
    /// distinct configs per print job).
    pub fn intern_config(&mut self, rc: ResolvedConfig) -> ConfigId { /* ... */ }
}
```

### Config Interner Contract (Normative — Packet 91)

- All production code reads a region's config via `region_map.config_for(&key)`
  rather than direct field access. The interner model is the only supported
  read path post-Packet 91.
- `configs` is non-empty by construction: every `RegionMapIR` is seeded with
  `ResolvedConfig::default()` at index 0, so `ConfigId::default()` (zero) is
  always a valid index. Legacy single-config flows produce a one-entry Vec
  and a single `ConfigId(0)`.
- `intern_config` uses linear-scan deduplication; equivalent configs reuse
  the same `ConfigId`. This prevents duplication in cross-product expansion
  (Packet 93) where many `(variant_chain)` entries share base config payload.
- `RegionMapIR.entries` cardinality is bounded by `DEFAULT_REGION_MAP_CAP`
  (currently `750_000`). Overflow surfaces `RegionMappingError::CapExceeded`
  naming the top-contributing `ObjectId`.

### `RegionMappingPlanProjection` (Internal Decoupling Type — Packet 87)

- **Scope:** internal to `slicer-core` and the runtime wrapper only. Not
  serialized, not transmitted at any IR or WIT boundary.
- **Purpose:** projection of the subset of `ExecutionPlan` (a scheduler-crate
  type) that `execute_region_mapping` reads — specifically
  `stage_invocations: &[(StageId, Vec<ModuleInvocation>)]`. Defined in
  `slicer-core/src/algos/region_mapping.rs`. Allows the kernel to remain
  IR-in/IR-out without importing scheduler types into `slicer-core`.

### Config Key Namespaces

Config keys follow a structured namespace convention used in `ResolvedConfig` and print-profile JSON:

- `object_config:<id>:<key>` — per-object override for the object whose `ObjectId` matches `<id>`. Recognised since DEV-040 (Packet 35a).
- `paint_config:<semantic>:<key>` — per-paint-semantic override. Applied during `PrePass::RegionMapping` when the region's polygons overlap a painted region for the corresponding `PaintSemantic`. Built-in `PaintSemantic` variants serialize as: `material`, `fuzzy_skin`, `support_enforcer`, `support_blocker`. `PaintSemantic::Custom(s)` serializes the inner string `s` verbatim (e.g. `paint_config:ironing:line_width`). Added in Packet 51.
- `tool_config:<tool_index>:<key>` — per-tool/extruder override keyed by the integer `tool_index`, resolved by `resolve_per_tool_configs`. A clean additive axis enabled by the region_id↔tool split (`PrintEntity.tool_index` is now a first-class selector). Consumed in **two** places, because a tool can be known at two different points:
  1. **Painted/material tools — at `RegionMapping`** (`region_mapping.rs`): the variant-chain cross-product splits a painted region into one `RegionPlan` per `("material", ToolIndex(n))` chain, and the `tool_config:<n>:<key>` overlay is applied to that chain at **highest precedence** (see below). This delivers per-tool **geometry** (`line_width`, etc.) for painted/MMU tools **without any pipeline reordering** — the tool is already known from the paint. (Verified end-to-end: `algo_region_mapping_tdd::region_mapping_applies_per_tool_config_overlay_to_painted_tool` → `classic_perimeters_tdd::per_region_line_width_sets_emitted_wall_width`.)
  2. **Every tool — at G-code emit** (`emit.rs`): emit-time settings (e.g. `retract_length`) are overlaid by the entity's resolved `tool_index`, the one place *every* entity's tool is known.

  **Still out of scope:** per-tool *geometry* for **non-painted** tools (spatial / modifier-extruder / `DEFAULT_TOOL` fallback), whose tool is resolved *after* perimeter generation in `assemble_ordered_entities` (`layer_executor.rs:597,747-751`); that would require moving tool resolution before the perimeter stages (a pipeline-ordering change). OrcaSlicer itself has no per-filament line-width — its per-tool *width* variation comes from the per-extruder `nozzle_diameter` vector (a base, selected by the region's extruder index) when width is a percentage; our explicit `tool_config:<n>:line_width` is a superset.

**Override precedence** (lowest → highest):

```text
global < per_object (object_config:<id>:<key>) < per_paint_semantic (paint_config:<semantic>:<key>) < per_tool (tool_config:<idx>:<key>)
```

Per-tool config is applied **last (highest)**, mirroring OrcaSlicer's filament-override-last model (`PrintApply.cpp` applies the filament preset's overrides on top of print/object/modifier/material). At `RegionMapping` the per-tool overlay runs after the paint overlays for a painted tool's chain; at emit it overlays the global config.

When multiple paint semantics overlap a single region during `RegionMapping`, the host sorts the contributing semantics by the lexicographic order of `paint_semantic_namespace_key(&PaintSemantic)` ascending and overlays them in that order. The lexicographically-last semantic in sort order overlays last and therefore wins. This RegionMap-stage rule determines which semantic's config wins in `RegionPlan.config`. It is distinct from the `paint_order`-based rule, which governs intra-semantic polygon overlap during `PrePass::PaintSegmentation`: the highest `paint_order` wins, and equal-order conflicting values are a fatal error. The `paint_order` field is defined in `crates/slicer-sdk/src/prepass_builders.rs`; its resolution rule is documented in `04_host_scheduler.md` § "Layer::PaintRegionAnnotation Stage" and traced in `10_scenario_traces.md`.

**Overlap determination (Normative — Packet 51):** A region's polygons
are considered to overlap a `PaintSemantic` when
`slicer_core::intersection(region_polygons, semantic_region_polygons)`
returns ANY non-empty result (a single shared point or line segment
counts). The first such overlap found by the per-region traversal wins
the precedence vote for its semantic; all overlapping semantics
contribute their `ResolvedConfig` snapshot to `RegionPlan.paint_overrides`
for audit visibility.

---

## IR 6 — SliceIR

**Stage:** Output of `PrePass::Slice`, refined by `PrePass::ShellClassification`
and `PrePass::PaintSegmentation`, then mutated by `Layer::SlicePostProcess`

**Current schema_version: 4.7.0** (`CURRENT_SLICE_IR_SCHEMA_VERSION` in
`crates/slicer-ir/src/slice_ir.rs`). Minor bump to 4.7.0 by P112 — additive
`ExtrusionJunction` / `ExtrusionLine` types for Arachne variable-width walls.
The full version history is in the "IR Versioning Contract" table at the end of
this document; that table is authoritative for this IR's history.

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
    /// Closed polygon islands. Coordinates in scaled integer units (1 unit = 100 nm = 10⁻⁴ mm).
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
    /// Renamed from `boundary_paint` in packet 91; carries paint semantics
    /// that are NOT declared `[[region_split]]` in any module manifest
    /// (split semantics surface through `variant_chain` instead).
    pub segment_annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
    /// Minimum depth (in layers, 0 = exposed) of this region within the top
    /// shell zone.  `None` outside any top shell.  Written by
    /// `PrePass::ShellClassification` (host built-in, port of OrcaSlicer's
    /// `discover_horizontal_shells`).  Saturates at `u8::MAX` (255) for
    /// pathological shell configurations.  Bumped to `Option<u8>` in
    /// `CURRENT_SLICE_IR_SCHEMA_VERSION = 3.0.0` (replaces the prior
    /// `is_top_surface: bool`).
    pub top_shell_index: Option<u8>,
    /// Minimum depth of this region within the bottom shell zone (same shape
    /// as `top_shell_index`).  `None` outside any bottom shell.
    pub bottom_shell_index: Option<u8>,
    /// Polygon-precise solid-fill area produced by the top shell's
    /// shrinking-shadow projection.  Empty when `top_shell_index` is `None`.
    pub top_solid_fill: Vec<ExPolygon>,
    /// Polygon-precise solid-fill area produced by the bottom shell's
    /// shrinking-shadow projection.  Empty when `bottom_shell_index` is `None`.
    pub bottom_solid_fill: Vec<ExPolygon>,
    /// True when this region spans a bridge gap at this layer.  Defaults `false`.
    /// Populated by mesh analysis (packet 36 / 36-rev1).
    pub is_bridge: bool,
    /// Per-layer expanded bridge polygons (in 100 nm units).  Added in packet 36.
    /// After `Layer::Perimeters` commit, host-clipped to `perimeter.infill_areas`
    /// and deduped by precedence `bridge > bottom > top > sparse`.
    pub bridge_areas: Vec<ExPolygon>,
    /// Best bridge direction across all valid bridge regions (degrees).  Added in packet 36.
    pub bridge_orientation_deg: f32,
    /// Sparse-only infill polygon after host-side fill partition.  Empty
    /// before `Layer::Perimeters` commit; afterwards equals
    /// `perimeter.infill_areas − union(bridge_areas, bottom_solid_fill, top_solid_fill)`.
    /// Pairwise disjoint with the other three canonical fill polygons after the
    /// host hook in `crates/slicer-runtime/src/region_partition.rs` runs.
    /// Added in `docs/specs/_OLD/infill-fill-partition-plan.md` (superseded
    /// spec, retained for the partition write-up).
    pub sparse_infill_area: Vec<ExPolygon>,
    /// Per-variant paint semantic chain. Empty in packet 91 (scaffold);
    /// populated by packet 93 (region splitting cross-product) for
    /// regions that match a `[[region_split]]` semantic in some module
    /// manifest. Ordering matches `RegionKey.variant_chain`.
    pub variant_chain: Vec<(String, PaintValue)>,
    // `external_contour: Option<Vec<ExPolygon>>` was removed in schema 4.6.0 (P109, T-P96-D):
    // dead field per ADR-0013 (Model-A per-color fragmentation traces each region's own outer
    // wall; the P96 union-trace boundary lost its last consumer when P105 removed it from both
    // perimeter modules).
}

/// ### Post-`Layer::Perimeters` invariant: four canonical fill polygons
///
/// After the host runs `sync_perimeter_infill_areas_into_slice` at
/// `Layer::Perimeters` commit (see
/// `crates/slicer-runtime/src/region_partition.rs`):
///
/// 1. **`bridge_areas`**, **`bottom_solid_fill`**, **`top_solid_fill`**, and
///    **`sparse_infill_area`** are pairwise disjoint subsets of the
///    corresponding `PerimeterIR.regions[i].infill_areas` (the wall-inset
///    polygon).
/// 2. Precedence on overlap is strict: `bridge > bottom > top > sparse`
///    (OrcaSlicer `PrintObject::prepare_infill` parity).
/// 3. The pre-perimeter values of `top_solid_fill` / `bottom_solid_fill` /
///    `bridge_areas` (committed by `PrePass::ShellClassification` and
///    `PrePass::MeshAnalysis`) live unchanged on the **Blackboard**'s
///    `Arc<Vec<SliceIR>>`; the per-layer arena copy is the one that gets
///    clipped + deduped. This preserves the read-only Blackboard contract.
/// 4. A `SliceIR` region with no matching `PerimeterIR.regions` entry is
///    skipped silently (used by the region_split work in packets 92–95 where
///    variant regions share wall geometry with their base region).
///
/// Each fill claim holder (`claim:sparse-fill`, `claim:top-fill`,
/// `claim:bottom-fill`, `claim:bridge-fill`; see `docs/03_wit_and_manifest.md`)
/// emits over exactly one of these polygons with zero polygon math.

### Modifier sub-regions

A *modifier sub-region* is a wall-less region spawned by a modifier volume. A
modifier volume (e.g. a density / speed override) does **not** carve its own
walls. Packet 132 (`132_modifier-region-split`, binding per ADR-0030 —
*Modifier splits fill, not perimeters*) instead spawns **wall-less sub-regions**
that share the base region's walls: a sub-region carries
`wall_source_region_id = Some(base)` so the perimeter stage traces walls once on
the base region and the sub-region's infill is emitted against that shared wall
geometry (no duplicate outer wall). ADR-0030 is the governing decision; the
binding implementation and tests live in packet 132.

**Per-sub-region config binding.** Each sub-region is bound to its own resolved
config via the `stamp_modifier_sub_region_configs` map keyed by `region_id`
(see `crates/slicer-core/src/algos/region_mapping.rs:335`: it overlays the
modifier volumes' config deltas onto the base `ResolvedConfig`, skipping
`support_enforcer` / `support_blocker` subtypes, and returns a
`BTreeMap<region_id, ResolvedConfig>` stamped per sub-region).

**Sub-region `region_id` namespace.** Sub-region IDs are derived from the base
region ID with a dedicated coprime stride so they never collide with paint's
`1_000_000`-stride namespace:

```
sub_region_id = base_region_id * MODIFIER_VARIANT_REGION_ID_STRIDE + modifier_hash(footprint_geo)
```

where `MODIFIER_VARIANT_REGION_ID_STRIDE = 1_000_003` (the next prime above
paint's `1_000_000`, hence coprime — see
`crates/slicer-runtime/src/region_partition.rs:71`). `modifier_hash` folds the
footprint geometry into a non-zero value `< stride` (so the low-order band is
reserved for `base_region_id * stride` itself), giving a stable, collision-free
sub-region id that round-trips through `RegionMapIR` and dispatch.

/// Polygon with holes. Contour is CCW; holes are CW.
pub struct ExPolygon {
    pub contour: Polygon,
    pub holes: Vec<Polygon>,
}

pub struct Polygon {
    pub points: Vec<Point2>,
}

pub struct Point2 { pub x: i64, pub y: i64 }   // 1 unit = 100 nm = 10⁻⁴ mm
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
    /// Populated by the perimeter generator from SlicedRegion.segment_annotations.
    pub feature_flags: Vec<WallFeatureFlags>,
    /// Identifies whether this wall is adjacent to another material region.
     /// Set by the perimeter generator when material paint regions are
     /// present at this layer.
     pub boundary_type: WallBoundaryType,
}

pub struct WallFeatureFlags {
     /// Tool override for this segment. `None` means use region default
     /// `tool_index`. Populated during `Layer::PaintRegionAnnotation`
     /// when the perimeter wall loop overlaps a `PaintSemantic::Material`
     /// paint region; the annotation propagates the
    /// dominant `ToolIndex(n)` value to all vertices in the overlapping
    /// wall loop. Vertices in unpainted regions have
    /// `tool_index = None`. Consumed downstream by
    /// `dominant_tool_index()` (assemble_ordered_entities, packet 50b)
    /// to stamp `RegionKey.region_id` so path-optimization can group
    /// by tool and `GCode` can emit `T{n}` tool changes.
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
    /// `segments` lists each contiguous boundary transition along the wall
    /// polygon, recording the point range and the tool indices on each side.
    MaterialBoundary { segments: Vec<MaterialBoundarySegment> },
    /// Inner wall — no special boundary handling.
    Interior,
}

/// A segment of a material boundary transition on a wall polygon.
///
/// Each segment records the point range (half-open `[start, end)`) on the
/// polygon contour where two different tool indices are adjacent. `near_tool`
/// is the tool on the inside of the wall; `far_tool` is the tool on the
/// outside. Either may be `None` when the polygon edge is adjacent to air or
/// a gap.
pub struct MaterialBoundarySegment {
    /// Half-open range `[start, end)` of point indices on the polygon contour
    /// where this boundary transition occurs.
    pub point_range: std::ops::Range<u32>,
    /// Tool index on the near side of the boundary (inside the wall).
    pub near_tool: Option<u32>,
    /// Tool index on the far side of the boundary (outside the wall).
    pub far_tool: Option<u32>,
}

pub enum LoopType { Outer, Inner, ThinWall, NonPlanarShell, GapFill }

/// Variable-width profile (Arachne). Constant-width = all values equal.
pub struct WidthProfile {
    pub widths: Vec<f32>,         // one per vertex in path.points
}
```

#### Variable-width geometry (Packet 103 — additive, schema 4.3.0)

`ThickPolyline` and `Point2WithWidth` are the 2-D input types consumed by Arachne
perimeter generation before conversion to `ExtrusionPath3D`.

```rust
/// 2-D point with an associated extrusion width (Arachne input).
pub struct Point2WithWidth { pub x: f32, pub y: f32, pub width: f32 }

/// Ordered sequence of variable-width 2-D points (Arachne polyline).
pub struct ThickPolyline { pub points: Vec<Point2WithWidth> }
```

`variable_width(thick: &ThickPolyline, role: ExtrusionRole) -> ExtrusionPath3D`
maps each `Point2WithWidth` to a `Point3WithWidth` with `z = 0.0`,
`flow_factor = 1.0`, `overhang_quartile = None`, `dist_to_top_mm = 0.0`,
`speed_factor = 1.0`, and the
supplied `role` passed through unchanged.

```rust
/// 3D extrusion path. For purely planar layers all z values equal layer z.
/// Non-planar and smoothificator modules write non-uniform z values.
pub struct ExtrusionPath3D {
    pub points: Vec<Point3WithWidth>,
    pub role: ExtrusionRole,
    /// Per-move speed multiplier consumed by `resolve_feedrate`
    /// (packet 52). Clamped to `[0.05, 5.0]` at emission time to
    /// reject pathological values (OrcaSlicer parity confirmed). The
    /// emitter multiplies the role-resolved base speed by `speed_factor`
    /// before converting to the F-token.
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
    /// Distance from this point to the top of its support column in mm.
    /// Support-planner emits this per point; non-support geometry uses `0.0`.
    /// Added in packet 119.
    pub dist_to_top_mm: f32,
}
```

#### Arachne extrusion-line geometry (Packet 112 — additive, schema 4.7.0)

`ExtrusionLine` and `ExtrusionJunction` are the variable-width polyline types
produced by the real Arachne beading-strategy pipeline (packets 110–112:
Voronoi → `SkeletalTrapezoidation` → centrality → per-edge bead-count →
propagation → `generate_toolpaths` → stitch → simplify → remove-small),
mirroring OrcaSlicer's Arachne `ExtrusionLine`/`ExtrusionJunction`
(`libslic3r/PerimeterGenerator.hpp`). They sit upstream of the existing
`ExtrusionPath3D`/`Point3WithWidth` pair above — `ExtrusionLine` is the
Arachne-native shape (ordered junctions + per-line topology flags);
`extrusion_line_to_extrusion_path3d(line, role) -> ExtrusionPath3D` converts
one into the other for assignment to `WallLoop.path`, the same way
`variable_width()` converts a `ThickPolyline`.

```rust
/// A single junction (vertex) of an `ExtrusionLine`.
pub struct ExtrusionJunction {
    /// 3D position, local width, flow factor, and overhang classification.
    pub p: Point3WithWidth,
    /// Perimeter index this junction was generated for (0 = outermost).
    #[serde(default)]
    pub perimeter_index: u32,
}

/// A variable-width extrusion line produced by the Arachne beading-strategy
/// stack, prior to conversion into `ExtrusionPath3D`.
pub struct ExtrusionLine {
    /// Ordered junctions (vertices) making up this line.
    pub junctions: Vec<ExtrusionJunction>,
    /// Inset index this line belongs to (0 = outermost wall).
    pub inset_idx: u32,
    /// True if this line was generated as part of an odd-width transition
    /// region rather than a uniform-bead perimeter.
    #[serde(default)]
    pub is_odd: bool,
    /// True when this line forms a closed loop (first and last junction
    /// coincide in XY).
    #[serde(default)]
    pub is_closed: bool,
}
```

Both new fields (`perimeter_index`, `is_odd`, `is_closed`) carry
`#[serde(default)]`, making the addition backward-compatible: a pre-bump
JSON fixture with neither field present still deserializes (`is_odd` and
`is_closed` default to `false`), which is what the schema-version table
below classifies as an **additive** (minor) bump rather than a breaking one.
`CURRENT_SLICE_IR_SCHEMA_VERSION` moves **4.6.0 → 4.7.0** for this addition
— see the Reservation Table entry below.

**WIT boundary.** `ExtrusionLine`/`ExtrusionJunction` mirror onto WIT
`extrusion-line`/`extrusion-junction` records in
`crates/slicer-schema/wit/deps/ir-types.wit`. Unlike most IR additions,
these do NOT round-trip through a `SliceRegionView` accessor read by an
arbitrary guest module — the only production consumer is the new
`host-services::generate-arachne-walls` WIT function
(`crates/slicer-schema/wit/deps/common.wit`), which returns
`result<list<extrusion-line>, string>` from a host-side call to
`slicer_core::arachne::pipeline::run_arachne_pipeline`. `arachne-perimeters`
(the WASM guest) calls this host service because it cannot link the
`host-algos`-gated Voronoi/SkeletalTrapezoidation/beading code itself
(`rayon` + `boostvoronoi` are native-only) — see `D-112-HOSTSVC-BRIDGE` in
`docs/DEVIATION_LOG.md` for the full architecture rationale.

#### Overhang quartile bucketization (Normative — Packet 57)

The four overhang speed bands map to signed-distance thresholds from
the previous-layer support polygons (negative = unsupported):

| Quartile | Signed distance `d` (multiples of width `w`) | Speed key            |
|----------|----------------------------------------------|----------------------|
| 1 (least supported) | `d < -0.5 w`                       | `overhang_1_4_speed` |
| 2                   | `-0.5 w ≤ d < -0.25 w`             | `overhang_2_4_speed` |
| 3                   | `-0.25 w ≤ d < 0`                  | `overhang_3_4_speed` |
| 4 (fully supported) | `d ≥ 0`                            | `overhang_4_4_speed` |

`w` is the per-point extrusion width from `Point3WithWidth.width`.
OrcaSlicer uses `<` for interval boundaries; this implementation
mirrors that exactly.

Invariants:

- `overhang_quartile` is populated ONLY for roles in
  `{OuterWall, InnerWall, ThinWall}`. All other roles remain `None`
  even when overhanging — bridge-family, infill, supports, and ironing
  use their own role base speeds; overhang modulation applies to walls
  only.
- Layer 0 (no previous layer to classify against) leaves all
  `overhang_quartile` values `None` regardless of config.
- An all-zero `overhang_*_4_speed` config (all four keys 0) short-circuits
  the classifier (no work performed, output byte-identical to pre-packet
  legacy path).

```rust

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

Packet 172 routing: support paths and raft paths are emitted on the support
tool; interface paths and ironing paths are emitted on the interface tool.
Selection is global because this flat `SupportIR` has no per-object identity;
a future packet can lift `SupportIR` to per-object selection when needed.

---

## IR 9a — SupportGeometryIR

**Stage:** Output of `PrePass::SupportGeometry` — coarse outline prepass results,
committed before `SupportPlanIR` within the same stage.

**Producer:** The host built-in commits `SupportGeometryIR` first within
`PrePass::SupportGeometry`, ahead of any `support-planner` module's
`SupportPlanIR` (see IR 9b below).

**Consumers:** `Layer::Support` modules that need coarse per-`(layer, object,
region)` outline polygons independent of organic branch planning.

```rust
pub struct SupportGeometryIR {
    pub schema_version: SemVer,
    /// 0.0 = use model layer height (config schema enforces min > 0).
    pub support_layer_height_mm: f32,
    /// Distance in mm from column tops to add intermediate model layers.
    pub support_top_z_distance_mm: f32,
    /// Per-(layer, object, region) coarse outline polygons.
    pub entries: HashMap<SupportGeometryKey, Vec<ExPolygon>>,
}

pub struct SupportGeometryKey {
    /// Model layer index that this support geometry entry applies to.
    /// `u32::MAX` sentinel = intermediate model-resolution layer.
    pub global_support_layer_index: u32,
    /// Object this entry belongs to.
    pub object_id: ObjectId,
    /// Region identifier within the object.
    pub region_id: RegionId,
}
```

---

## IR 9b — SupportPlanIR

**Stage:** Output of `PrePass::SupportGeometry` (optional; only present when a
`support-planner` module is loaded)

**Current schema_version: 1.2.0** (`CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` in
`crates/slicer-ir/src/slice_ir.rs`). Packet 119 added the per-point
`Point3WithWidth.dist_to_top_mm` field and the optional raft configuration seam.

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
    /// Optional configuration-only raft seam. `None` means no raft was
    /// requested. The planner does not put raft geometry in this field.
    pub raft_plan: Option<RaftPlan>,
}

pub struct RaftPlan {
    /// Number of raft layers below the model.
    pub raft_layers: u32,
    /// Density of the first raft layer.
    pub raft_first_layer_density: f32,
    /// Number of base raft layers.
    pub base_raft_layers: u32,
    /// Number of interface raft layers.
    pub interface_raft_layers: u32,
}

pub struct SupportPlanEntry {
    /// Signed: negative values (`-1`, `-2`, ...) are reserved for raft prefix
    /// layers; non-negative values refer to model layers.
    pub global_layer_index: i32,
    pub object_id: ObjectId,
    pub region_id: RegionId,
    /// Pre-planned organic branch geometry. Each `ExtrusionPath3D` is typically
    /// a two-point segment (one MST edge between propagated contact points)
    /// but may be multi-point for long merged branches. Points carry mm-valued
    /// `Point3WithWidth` data and are emitted with `ExtrusionRole::SupportMaterial`.
    pub branch_segments: Vec<ExtrusionPath3D>,
}
```

`raft_plan` is emitted as `Some(RaftPlan)` when the support planner receives a
positive `support_raft_layers` value. It mirrors the raft configuration only;
raft polygons, layer geometry, and raft infill remain deferred to packet 124.
The current support planner emits no negative raft-prefix entries.

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
`SupportPlanIR` (`entries.len()`, every entry's `branch_segments.len()`, every
endpoint coordinate, and the optional raft configuration). The host-side
prepass ceremony round-trips this via
the `support_planner_is_deterministic_across_runs` test.

### `ModuleAccessAudit.diagnostics` (Normative — Packet 118)

`ModuleAccessAudit` (`crates/slicer-scheduler/src/validation.rs`) records the
runtime read/write paths a prepass module exercised during its most recent
invocation, plus the typed diagnostics it emitted. The diagnostic field was
added in Packet 118 to carry the prepass diagnostic channel defined in
`docs/adr/0010-typed-diagnostic-channel.md` from the host into the scheduler
audit surface.

```rust
pub struct ModuleAccessAudit {
    pub module_id: ModuleId,
    pub runtime_reads: Vec<String>,
    pub runtime_writes: Vec<String>,
    /// Typed diagnostics emitted by the module during prepass execution.
    /// FIFO order, preserved from guest emission. Not compared by scheduler
    /// validation — only runtime_reads and runtime_writes participate.
    pub diagnostics: Vec<slicer_ir::Diagnostic>,
}
```

The `diagnostics` field has the following contract:

- **FIFO ordering.** The host (`WasmRuntimeDispatcher::dispatch_prepass_call`
  in `crates/slicer-wasm-host/src/host.rs`) drains the per-call thread-local
  diagnostic stash once and pushes the entries onto `ModuleAccessAudit.diagnostics`
  in the order the guest emitted them. The order is preserved end-to-end:
  guest `push-diagnostic` → `HostExecutionContext.diagnostics` →
  `PrepassStageRunner::last_diagnostics` → `ModuleAccessAudit.diagnostics`.
- **Not used by scheduler validation.** Pass 11 (`ModuleAccessAuditValidation`)
  compares only `runtime_reads` and `runtime_writes`. `diagnostics` is
  surfaced for the host's own log/metrics pipeline; it does not influence
  startup validation outcomes.
- **Empty when the module emits no diagnostic.** A module that does not call
  `push-diagnostic` produces a `Vec::new()`. The field is not optional.
- **Type mirror.** Entries are `slicer_ir::Diagnostic` (see
  `crates/slicer-ir/src/stage_io.rs`); the host converts from the WIT
  `diagnostic` record to `slicer_ir::Diagnostic` at the
  `pm::HostSupportGeometryOutput::push_diagnostic` boundary in
  `crates/slicer-wasm-host/src/host.rs` so the audit never sees WIT types.
  The severity field is `slicer_ir::DiagnosticSeverity`
  (`{Trace, Debug, Info, Warn, Error}`), the rust-mirrored 1:1 mapping of
  the WIT `severity-level` enum (see
  `03_wit_and_manifest.md` § "`support-geometry-output.push-diagnostic`").

This adds a typed `Vec` field; the existing `runtime_reads` and `runtime_writes`
shape and the pass-11 comparator are unchanged. Packet 118 does not introduce a
generic all-prepass method, does not add a `SupportPlanIR` field, and does not
change fatal-error behaviour.

---

## IR 9c — SeamPlanIR

**Stage:** Output of `PrePass::SeamPlanning` (optional; only present when a
`seam-planner` module is loaded — packet 23-rev1).

**Producer:** A module holding the `seam-planner` claim. Ordered after
`PrePass::LayerPlanning`, before `PrePass::PaintSegmentation`.

**Consumers:** `Layer::PerimetersPostProcess` modules holding the
`seam-placer` claim. Advisory — may fall back to per-layer scoring.

**schema_version: 1.1.0** (`CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION`; packet 178
added additive `variant_chain` propagation through harvest (the field already
exists on `RegionKey`) and bumped the minor version per the schema versioning
policy in `docs/11`.)

```rust
pub struct SeamPlanIR {
    pub schema_version: SemVer,
    /// One entry per planned `(layer, object, region)` triple, keyed by
    /// `RegionKey`. **Duplicate key contract:** two entries with identical
    /// `RegionKey` are a fatal IR validation error; rejected at commit time.
    pub entries: Vec<SeamPlanEntry>,

/// Each `SeamPlanEntry` carries the full `variant_chain` as part of `SeamPlanIR` `RegionKey` identity, enabling per-variant seam planning. `PerimeterRegion` also carries `variant_chain` for injection lookup during per-layer seam placement.
/// `variant_chain` for injection lookup during per-layer seam placement.
}

pub struct SeamPlanEntry {
    /// Stable region key for lookup during layer dispatch.
    pub region_key: RegionKey,
    /// The seam position selected by the planner.
    pub chosen_candidate: SeamPosition,
    /// Full scored candidate list for evidence and regression checks.
    pub scored_candidates: Vec<ScoredSeamCandidate>,
}

pub struct SeamPosition {
    /// Seam point on the outermost wall loop, in **millimeters**
    /// (`Point3WithWidth`, f32 mm — not the IR-internal 100 nm integer units;
    /// see the packet-161 correction on coordinate scale in this doc).
    pub point: Point3WithWidth,
    /// Index of the wall this seam was placed on.
    pub wall_index: u32,
}

/// One scored seam candidate from the prepass planner. `score` is the
/// primary sort key; lower is better.
pub struct ScoredSeamCandidate {
    /// Candidate position with extrusion width, in millimeters.
    pub position: Point3WithWidth,
    pub score: f32,
    /// Enum tag explaining why this candidate was scored this way.
    pub reason: SeamReason,
}
```

`SeamPlanEntry.chosen_candidate` is consumed via
`PerimeterRegionView.resolved_seam` so the apply-stage module (seam-placer)
operates on a pre-resolved seam without rescoring.

---

## IR 9d — LightningTreeIR

**Stage:** Output of `PrePass::LightningTreeGen` (optional; only committed when
the print's `sparse_fill_holder` resolves to `lightning-infill` per ADR-0029).
Positioned after `PrePass::SupportGeometry`, before `Layer::PaintRegionAnnotation`.

**Current schema_version: 1.0.0** (authoritative source:
`CURRENT_LIGHTNING_TREE_IR_SCHEMA_VERSION` in `crates/slicer-ir/src/slice_ir.rs`).
Packet 137 lands the contract; packets 138/139 fill the producer skeleton with
the real cross-layer distance-field + tree-node generator.

**Producer:** A host built-in committed via
`crates/slicer-runtime/src/builtins/lightning_tree_producer.rs`. The producer is
**skipped** (no commit, slot stays `None`) when no region's
`sparse_fill_holder` is `lightning-infill` — the zero-cost skip promise from
ADR-0029. When committed, the IR carries per-object, per-region, per-layer 2-point
tree-edge segments in integer coordinate units (compact storage per ADR-0029's
memory note; no full topology).

**Consumers:** `Layer::Infill` modules that declare `LightningTreeIR` as a read
in their manifest. The packet 140 `lightning-infill` module consumes this view
and emits one raw path per committed tree segment.

```rust
pub struct LightningTreeIR {
    pub schema_version: SemVer,
    /// One entry per active `(global_layer_index, object_id, region_id)` triple
    /// that received tree-edge segments. Multiple entries may share an
    /// `(object_id, global_layer_index)` when an object has multiple regions.
    pub entries: Vec<LightningTreeEntry>,
}

pub struct LightningTreeEntry {
    pub object_id: ObjectId,
    /// Region inside the object; this follows the per-region precedent of
    /// `SupportPlanEntry.region_id: RegionId`.
    pub region_id: RegionId,
    /// Signed: negative values (`-1`, `-2`, ...) are reserved for raft prefix
    /// layers; non-negative values refer to model layers.
    pub global_layer_index: i32,
    /// 2-point tree-edge segments in integer coordinate units. Each pair
    /// `[a, b]` is rendered directly as one raw path.
    pub tree_edge_segments: Vec<[Point2; 2]>,
}
```

**Consumption pattern — read-view via `lightning-tree-segments`:**

The host exposes the IR to a `Layer::Infill` guest via the
`lightning-tree-segments` method on the `paint-region-layer-view` WIT resource
(`crates/slicer-schema/wit/deps/ir-types.wit:206`; canonical package
`slicer:world-layer@2.3.0`). The `run-infill` export receives a
`paint: paint-region-layer-view` argument, and the guest looks up the per-layer
`tree_edge_segments` matching `(object_id, region_id, layer_index)` via the SDK's
`PaintRegionLayerView::lightning_tree_segments_for(object_id, region_id)`
accessor (`crates/slicer-sdk/src/traits.rs:196-212`). When no `LightningTreeIR` is
committed (skip-when-no-lightning-holder), the accessor returns an empty
`Vec` and the module emits no paths for that layer; there is no non-lightning
fallback.

**Determinism:** Identical PrePass inputs must produce byte-identical
`LightningTreeIR`. The `entries` Vec order is producer-defined and must be
stable (no hash containers); per-layer segment ordering is the producer's
responsibility. 138/139 inherit this contract.

---

## IR 10 — LayerCollectionIR

**Stage:** Output of `Layer::PathOptimization`
**Current schema_version: 1.1.0** (authoritative source: `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` in `crates/slicer-ir/src/slice_ir.rs`). Introduced at 1.0.0; packet 125 added the additive `PrintEntity.tool_index: u32` field (region_id↔tool split), bumping 1.0.0→1.1.0. Packet 39 earlier renamed `TravelMove.entity_idx: u32` → `entity_id: u64` and added `entity_id: u64` on `PrintEntity`, decoupling travel anchors from positional indices so finalization-stage entity insertion no longer invalidates anchors (these landed without bumping the constant beyond 1.x).

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

<!-- VERIFY: LayerCollectionIR.global_layer_index is `u32` in
     crates/slicer-ir/src/slice_ir.rs, so negative raft prefix indices are NOT
     representable here. SupportPlanEntry.global_layer_index IS `i32` and does
     reserve negatives for raft prefix layers. This doc previously specified
     `i32` for both; the LayerCollectionIR half does not match the code. Either
     the raft design was never carried into LayerCollectionIR (doc was
     aspirational) or the u32 is a defect that makes raft layers unrepresentable
     in the layer collection. Resolve before relying on either type here. -->

```rust
pub struct LayerCollectionIR {
    pub schema_version: SemVer,
    /// Unsigned in code today; see the VERIFY note preceding this block
    /// regarding raft prefix layers and negative indices.
    pub global_layer_index: u32,
    pub z: f32,
    /// Ordered, ready-to-emit extrusion entities.
    /// Produced by travel minimization + DAG topo sort.
    pub ordered_entities: Vec<PrintEntity>,
    pub tool_changes: Vec<ToolChange>,
    pub z_hops: Vec<ZHop>,
    /// Guest-emitted per-layer annotations (comments / raw G-code lines).
    pub annotations: Vec<LayerAnnotation>,
    /// Retract/unretract decisions from `Layer::PathOptimization`.
    pub retracts: Vec<TravelRetract>,
    /// Travels between entities. Anchors are by `entity_id`, not positional index
    /// (packet 39), so finalization mutations cannot dangle anchors.
    pub travel_moves: Vec<TravelMove>,
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
    /// Resolved tool/extruder index — a pure SELECTOR (which extruder/filament
    /// prints this entity). Separated from `region_key.region_id` (a pure region
    /// IDENTITY) by the region_id↔tool split so a painted-variant identity hash
    /// can never leak into the tool slot (the packet-125 9.9 GiB OOM). Set at
    /// assembly from `dominant_tool_index`/spatial/variant/modifier resolution,
    /// falling back to `0` (T0). Read by the emitter and `path-optimization`.
    /// `#[serde(default)]`; no `Default` on the struct (construction sites are
    /// compiler-forced to set it). Added with LayerCollectionIR schema 1.0.0 →
    /// 1.1.0 (additive).
    pub tool_index: u32,
}

pub struct TravelMove {
    /// Travel anchor: the entity in `ordered_entities` after which this travel
    /// is emitted. Replaces the previous `entity_idx: u32` positional anchor;
    /// the emitter resolves it via an `entity_id -> index` map
    /// built per-layer. Added in packet 39.
    pub entity_id: u64,
    /// Destination, in millimeters. Each axis is independently optional; `None`
    /// leaves that axis unchanged. The travel carries a destination only — the
    /// start point is wherever the previous move ended.
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
    /// Feed-rate override in mm/s. `None` keeps the current speed.
    pub f: Option<f32>,
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
    InternalSolidInfill,
    SupportMaterial, SupportInterface,
    WipeTower, PrimeTower,
    Skirt, Brim,
    Ironing, BridgeInfill,
    GapFill,           // thin-gap fill paths (added P105, schema 4.4.0)
    Custom(String),    // community modules may register new roles
}
```

### Extrusion-role default priority (Normative)

`ExtrusionRole::default_priority()` returns a `u32` used by
`PostPass::LayerFinalization::push_entity_with_priority` to order entities
inserted into a layer when the inserting module does not supply an explicit
priority. Lower numbers print earlier. Added in packet 40.

Values below are ordered as they print (lowest first) and mirror
`ExtrusionRole::default_priority()` in `crates/slicer-ir/src/slice_ir.rs`.

| Role                  | `default_priority()` |
|-----------------------|----------------------|
| `Skirt`               | 0    |
| `Brim`                | 110  |
| `OuterWall`           | 1000 |
| `InnerWall`           | 1500 |
| `ThinWall`            | 1700 |
| `GapFill`             | 2000 |
| `SparseInfill`        | 3000 |
| `BridgeInfill`        | 3500 |
| `InternalSolidInfill` | 3800 |
| `BottomSolidInfill`   | 4000 |
| `TopSolidInfill`      | 4500 |
| `SupportMaterial`     | 5000 |
| `SupportInterface`    | 5500 |
| `Ironing`             | 6000 |
| `WipeTower`           | 8000 |
| `PrimeTower`          | 8500 |
| `Custom(_)` (unknown) | 9000 |

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
- `validate_travel_anchors(layer: &LayerCollectionIR) -> Result<(), String>`
  (`crates/slicer-ir/src/validation.rs`) short-circuits on the first dangling
  travel anchor; the error string names the offending `entity_id`. Finalization
  invokes it before the layer is handed off to `PostPass::GCodeEmit`.

### `LayerCollectionIR::default()` contract (Normative — Packet 79 fixture support)

`LayerCollectionIR` derives `Default`. The default-field values are
load-bearing because the test-support
`LayerCollectionFixtureBuilder` (in `slicer-sdk::test_support::fixtures`)
only sets four fields explicitly (`global_layer_index`, `z`,
`ordered_entities`, `tool_changes`) and lets `Default` populate the
rest: `z_hops = vec![]`, `annotations = vec![]`, `travel_moves =
vec![]`, and `schema_version = CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`.
Tests that assemble synthetic layers via the fixture builder rely on
these defaults; changing the field set or the defaulted values is a
breaking change for the fixture surface, not just the production IR.

### `SlicedRegion` builder setter semantics (Normative — Packet 79 fixture support)

`SliceRegionViewBuilder`'s shell / bridge setters (`top_shell_index`,
`top_solid_fill`, `bottom_shell_index`, `bottom_solid_fill`,
`is_bridge`, `bridge_areas`, `bridge_orientation_deg`) implement
idempotent, last-write-wins semantics. Calling a setter with the same
value twice is a no-op; calling it twice with different values yields
the final value. Unset setters preserve `SliceRegionViewBuilder::new()`
field defaults (typically `None` / `vec![]` / `false`). The contract
exists so test migrations from hand-rolled fixture helpers can be
mechanical — one builder chain call per original field assignment, no
order-dependence.

### `rect_polygon` fixture helper (Normative — Packet 79 fixture support)

`rect_polygon(cx_mm: f32, cy_mm: f32, width_mm: f32, height_mm: f32) -> ExPolygon`
(in `slicer-sdk::test_support::fixtures`) constructs an axis-aligned
rectangular `ExPolygon` with vertices at
`(cx ± width/2, cy ± height/2)` in millimetres, converted to slicer
units via `mm_to_units()` (1 unit = 100 nm). Winding is
counter-clockwise (signed area > 0); `holes` is `vec![]`. This is the
canonical test fixture for rectangular shapes; production code MUST NOT
import it (it is `#[cfg(any(test, feature = "test"))]`-gated under
`slicer-sdk`).

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
        e: Option<f32>,
        /// Feedrate (mm/min). When `Some(_)`, the emitter writes this
        /// value verbatim and DOES NOT substitute the role-default
        /// (Packet 52 contract). When `None`, `resolve_feedrate(&role,
        /// speed_factor)` is applied at the print-move and z-hop
        /// builders to dispatch one of 26 per-role `*_speed` config
        /// keys; travel moves fall back to `travel_speed` when `f` is
        /// `None`. This `Some` override is how upstream modules (e.g.
        /// retract speed) keep their feedrates intact end-to-end.
        f: Option<f32>,
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
    ToolChange  { after_entity_index: u32, from: u32, to: u32 },
    Comment     { text: String },
    Raw         { text: String },       // escape hatch for printer-specific codes
    /// Extrusion mode selector (M82 = absolute, M83 = relative).
    /// Pushed by `DefaultGCodeEmitter::emit_gcode` as the first command
    /// so that `PostPass::GCodePostProcess` modules can prepend
    /// `machine_start_gcode` before it. Added in packet 59.
    ExtrusionMode { absolute: bool },
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

#### CONFIG_BLOCK viewer-key contract

The `CONFIG_BLOCK` is consumed by OrcaSlicer's `ConfigBase::load_from_gcode_file`
and `GCodeProcessor::apply_config` for both the configuration-panel display and
the time/motion estimator. The fork therefore supplies the following keys:

- `printer_model` — required so OrcaSlicer's `s_IsBBLPrinter` heuristic does not default to Bambu behavior.
- `filament_density` — supplies the filament table shown in the configuration panel.
- `filament_cost` — supplies the cost estimate.
- `printable_area` — supplies the bed shape displayed by the viewer.
- `nozzle_diameter` — supplies the extruder panel data.
- `machine_max_*` — supplies the time estimator and machine-limit display; the family includes `machine_max_acceleration_extruding`, `machine_max_acceleration_retracting`, `machine_max_acceleration_travel`, `machine_max_jerk_x`, `machine_max_jerk_y`, `machine_max_jerk_z`, `machine_max_jerk_e`, and …

PNP's `ORCA_CONFIG_PADDING` table must never emit keys whose names match
`*speed*`, `*acceleration*`, `*jerk*`, or `machine_max_*`. These keys are always
fork-supplied and are never synthesized as padding.

When `raw_config` lacks `printer_model`, PNP emits
`; printer_model = Generic PNP Printer`. This synthesis uses the same
deduplication path, `emit_config_kv` plus `BTreeSet<String>`, so a fork-supplied
value always wins.

`gcode_flavor` is a real honored key, not cosmetic padding. It supports five
values: `marlin` (default), `marlin2`, `klipper`, `reprapfirmware`, and
`repetier`; the dialect is implemented in `crates/slicer-gcode/src/flavor.rs`
(packet 171) and echoed in the `CONFIG_BLOCK` between `; CONFIG_BLOCK_START`
and `; CONFIG_BLOCK_END`. Unknown values fall back to `marlin` with a
`log::warn!`.

Packet 169-time-estimator-slice-stats depends on this contract when constructing
fork-realistic machine-limit fixtures.

`PostPass::GCodeEmit` wraps the per-layer command stream in four canonical
envelope blocks. Block sentinels and ordering are part of the wire-format
contract — frontends and post-processors parse these tokens.

**Envelope sequence (top to bottom of the `.gcode` output):**

```text
; HEADER_BLOCK_START
;   <semicolon-prefixed metadata lines: model name, layer count, filament
;    used, max Z, slicer version, etc.>
; HEADER_BLOCK_END
; THUMBNAIL_BLOCK_START                          (only when --thumbnail set)
;   <inner-framed entries: `; <tag> begin <W>x<H> <len>` / `; <tag> end`,
;    Base64 bodies wrapped at 78 chars/line each prefixed with "; ">
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
- Base64-encoded with 78 characters per line, each line prefixed by `"; "`,
  matching OrcaSlicer's wire format exactly so downstream tools (printer
  UIs, gcode preview viewers) parse it identically.

The block is **OrcaSlicer-parseable**: printer firmware and Orca-family parsers
key off the inner `; <tag> begin <W>x<H> <len>` / `; <tag> end` framing that
canonical `export_thumbnails_to_file` (`OrcaSlicerDocumented/src/libslic3r/GCode/Thumbnails.hpp`)
emits. Per entry:

  ; <tag> begin <W>x<H> <len>
  ; <base64 chunk, ≤ 78 chars per line>
  ; <tag> end

  e.g. (PNG):  ; thumbnail begin 300x300 123456
                ; <base64 ...>
                ; thumbnail end

`<tag>` is one of five values returned by the per-format `tag()` overrides:
  - PNG  → `thumbnail`
  - JPG  → `thumbnail_JPG`
  - QOI  → `thumbnail_QOI`
  - BTT_TFT (Biqu/BIQU RGB565 hex, `;<WWWW><HHHH>\r\n` header, per-row `;` prefix + `\r\n`)  → `thumbnail_BIQU` (Raw body, spliced verbatim)
  - ColPic (QIDI `;gimage:`/`;simage:` chunked, 512px aspect-preserved cap)  → `thumbnail_QIDI` (Raw body, spliced verbatim)

`<len>` is the total base64 character count for the entry (Base64 bodies only;
Raw bodies have no `<len>` header). Base64 lines are wrapped at 78 characters
(canonical `max_row_length`). ColPic and BTT_TFT payloads are self-framed
and spliced verbatim between the outer sentinels.

The block contains one entry per spec in the `thumbnails` config key
(`"WxH/EXT,WxH/EXT"`, e.g. `"48x48/PNG,300x300/PNG"`), in spec order. When
the key is absent, the block contains a single `thumbnail` entry at the
source PNG's dimensions, with the source bytes passed through un-re-encoded.

**Fork-facing single-source-PNG contract (deviation from fork ticket 011).**
The fork renders ONE high-res top-down PNG and passes it via `--thumbnail`;
requested sizes/formats travel in the `thumbnails` config key; PnP owns
the decode/resize/encode fan-out. See `D-173-THUMBNAIL-SINGLE-PNG` in
`docs/DEVIATION_LOG.md`.

**Configurable header fields (config keys, packet 55):**

| Key | Type | Default | Purpose |
|---|---|---|---|
| `filament_diameter` | f32 (mm) | `1.75` | Header `; filament_diameter` line; consumed by some post-processors. |
| `filament_density` | f32 (g/cm³) | `1.24` | Header `; filament_density` line. |
| `max_z_height` | f32 (mm) | `0.0` (auto) | Hard cap reported in header; `0.0` means "use per-print z_max". |
| `thumbnail_path` | string | `""` | Alternative to the `--thumbnail` CLI flag; CLI wins when both set. |

### Per-role feedrate emission (Normative — Packet 52)

`DefaultGCodeEmitter` carries a `FeedrateConfig` struct (bound at
construction from `ConfigView` at the postpass dispatch site) that
holds all 26 per-role speed keys (mm/s). `resolve_feedrate(role,
speed_factor) -> f32` is invoked at the print-move and z-hop builders
when `Move.f` is `None`; the resulting F-token is computed as
`round(speed_mm_per_s * 60.0 * speed_factor * 1000.0) / 1000.0`
(mm/min, three decimal places). `speed_factor` is clamped to
`[0.05, 5.0]` before multiplication (OrcaSlicer parity).

First-layer detection: `resolve_feedrate` selects the `initial_layer_*`
override variants (`initial_layer_speed`, `initial_layer_infill_speed`,
`initial_layer_travel_speed`) when the move's layer is layer 0.
First-layer membership is determined by comparing `Move.z` against the
committed `layer_height` with an epsilon tolerance; explicit
`is_first_layer` flags on `GlobalLayer` are not present in the IR
post-Packet 52.

### Stream-level extrusion mode (Normative — packet 54, 59)

`GCodeCommand::Move.e` is a signed delta in **relative** extrusion mode
(M83) and an absolute position in **absolute** mode (M82). Mode is a
stream-level invariant — the emitter pushes `GCodeCommand::ExtrusionMode { absolute }`
as the first command (packet 59) and resets the E-accumulator with `G92 E0` on
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
| `infill_resolution`       | f32  | `0.04 mm`      | Per-role tolerance for infill / solid-infill / bridge / top / bottom. |
| `support_resolution`      | f32  | `0.0375 mm`    | Per-role tolerance for support material / interface.              |
| `min_segment_length`      | f32  | `0.05 mm`      | Drop adjacent segments shorter than this after D-P.               |
| `gcode_xy_decimals`       | u32  | `3`            | Decimal places for X / Y / Z token formatting (via `format_xyz`). |
| `perimeter_arc_tolerance` | f32  | `0.0125 mm`    | Clipper2 arc-tolerance for `slicer_core::polygon_ops::offset(...)` — declared and read per-module by `classic-perimeters`. (P108 deleted an earlier stub `arachne-perimeters`; the module of that name today is a real Arachne generator that does not declare this key.) |
| `slice_closing_radius`    | f32  | `0.049 mm`     | Per-layer Clipper2 `inflate(+r) → inflate(-r)` round-trip after `simplify_polygon_points` in `triangle_mesh_slicer`. |

Per-role tolerance dispatch (consumed by `tolerance_for_role` in
`crates/slicer-gcode/src/serialize.rs`):

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

### Reservation Table — perimeter parity roadmap (P102–P112)

| Version | Packet | Rationale |
|---------|--------|-----------|
| 4.1.0 | P102 | (prior) — `SlicedRegion.sparse_infill_area` additive field |
| 4.2.0 | P102 | `WallBoundaryType::MaterialBoundary` widening to `Vec<MaterialBoundarySegment>` (T-013). The old single-`adjacent_tool` wire format is deserialized via `WallBoundaryTypeWire` migration adapter; new code writes `segments`. |
| 4.3.0 | P103 | `ThickPolyline` + `Point2WithWidth` additive types (T-042) |
| 4.4.0 | P105 | `LoopType::GapFill` + `ExtrusionRole::GapFill` additive variants (T-062b) |
| ~~4.5.0~~ | ~~P106~~ | STRUCK — reservation was speculative; P106 shipped `overhang_quartile_polygons` on `SurfaceClassificationIR` (bumped 1.1.0 → 1.2.0, see IR 2 above), not on `SliceIR`. `CURRENT_SLICE_IR_SCHEMA_VERSION` was left unchanged at 4.4.0 by P106. |
| 4.6.0 | P109 | `SlicedRegion.external_contour` field + WIT `external-contour` accessor removed (T-P96-D). Field removal is *major* by default, but this ships as a backward-**compatible** minor bump — a deliberate, documented exception; see the Contract note below. |
| 4.7.0 | P112 | `ExtrusionLine` + `ExtrusionJunction` additive types (T-224) |

Reservations apply only to the perimeter parity roadmap (P102..P112 + P106/P107). Other concurrent packets must coordinate with the active roadmap maintainer before bumping. Multiple bumps within a single packet are not permitted.

> **Contract note (P109) — compatible-removal exception.** The contract table above classifies "Field removed" as *major* by default. The `SlicedRegion.external_contour` removal is a deliberate, documented EXCEPTION shipped as a **minor** (4.6.0) because ALL THREE conditions hold: (1) the field had **no live consumer** (superseded by ADR-0013 Model-A per-color fragmentation; consumption was already removed in P105, D-105-AC22-PARITY-RESHAPE); (2) `SlicedRegion` does not use `deny_unknown_fields`, so **serde ignores the now-absent field** and serialized 4.x fixtures still parse; and (3) **every loaded module declares `max_ir_schema = 5.0.0`**, so a 5.0.0 host would fail the scheduler's `validate_ir_versions` gate (`min_ir_schema ≤ host < max_ir_schema`, `crates/slicer-scheduler/src/validation.rs`) for EVERY module — a major bump would break the entire module ecosystem for a removal that changes no behaviour. The original filing's "additive removal" phrasing was imprecise; the accurate term is a *compatible removal*. A removal that does NOT meet all three conditions (there is a live consumer, the shape is not serde-tolerant, or the target host version crosses any module's `max_ir_schema`) MUST take the major bump and the coordinated `max_ir_schema` widening across all modules.

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

The Rust types under `crates/slicer-ir/src/` are the normative implementation of this contract. This document records the boundary semantics, invariants, and source references; if prose here disagrees with code, treat the discrepancy as a documentation bug to be filed against this document.

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

Invalid numeric values:

- `NaN` and `±Inf` in config numeric fields are rejected at config
  resolution: `check_scalar` in
  `crates/slicer-scheduler/src/config_resolution.rs` requires `is_finite()`
  and raises `ConfigResolutionError::OutOfRange` otherwise. No blanket
  validator covers every IR numeric field; IR-side enforcement is per-field.
- Denormal/subnormal values are normalized to zero at the parse and
  boundary points: `normalize_subnormal_boundary` in
  `crates/slicer-wasm-host/src/host.rs`, the typed `ConfigValue` reads in
  `crates/slicer-ir/src/slice_ir.rs`, the planner's config-map builder in
  `crates/slicer-scheduler/src/execution_plan.rs`, and the model loader
  (`crates/slicer-model-io/src/loader.rs`).

## Canonical ID Types (Normative)

All IDs below are stable for one slice command and must be deterministic across repeated runs with identical inputs/config.

Canonical ID aliases and their serialization are defined in
`crates/slicer-ir/src/entity_id.rs` and the shared WIT types in
`crates/slicer-schema/wit/deps/ir-types.wit`. The aliases are string IDs for
objects, modifiers, and modules, and integer IDs for surface, bridge,
overhang, and generic regions.

WIT bridge rule:

- WIT represents `object-id`/`region-id` as strings.
- Canonical host mapping is:
  - `ObjectId`: UUID string, passed through unchanged.
  - `RegionId`: decimal string serialization of `u64` with no leading zeros.
- Any non-canonical region-id string from a module is rejected as fatal contract error.

Bounds and overflow policy:

- `GlobalLayer.index` must be `< 100_000`; host rejects plans above this budget.
- Region IDs are minted deterministically from a base ID plus a
  per-variant-chain hash (`paint_variant_region_id` in
  `crates/slicer-core/src/algos/paint_segmentation/mod.rs`) or a
  stride/hash combination for modifier sub-regions (`region_partition.rs`
  in `crates/slicer-runtime/src/`); IDs must not be reused for different
  geometry within a single slice.
- ID collisions are fatal contract errors.

---

## IR 0 — MeshIR

**Produced by:** Host mesh loader  
**Consumed by:** PrePass stages (read-only via host-services API; never passed directly to modules)
**Current schema_version: 1.1.0** (Bumped to 1.1.0 by packet 56b — populated `modifier_volumes` from `Metadata/model_settings.config`.)

The `MeshIR`, `ObjectMesh`, `FacetPaintData`, and `PaintLayer` definitions are
in `crates/slicer-ir/src/slice_ir.rs`. `MeshIR` carries the object list and
build volume; `ObjectMesh` carries mesh, transform, object config, modifier
volumes, optional paint data, and the cached world-space Z extent. Paint layers
remain parallel to the mesh triangles, with `None` meaning unpainted.

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

`BoundingBox2` is defined in `crates/slicer-ir/src/slice_ir.rs` and is a
2D axis-aligned, inclusive spatial pre-filter in native 100 nm units. It is
computed at harvest time and is not serialized.

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
- `paint_color`: states 1–16 are valid (extruder indices). States greater than 16 are rejected. Subdivision strings (longer than two characters) are **parsed, not rejected**: `decode_paint_hex_state` walks the TriangleSelector tree and returns the dominant leaf state, which is then subject to the same 1–16 range check. **ToolIndex encoding (Packet 50b):** OrcaSlicer encodes 1-based nibble states in 3MF; the loader adjusts to 0-based on commit, so the IR is uniformly 0-indexed (`ToolIndex(0..=15)`).

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

`PaintStroke.triangles` carries world-space sub-triangle geometry as
millimeter `Point3` values (the loader reads the 3MF document's millimetre
coordinates directly into `Point3 { x, y, z }` — see
`decode_strokes_for_channel` in `crates/slicer-model-io/src/loader.rs`).
The dominant state for a subdivided facet is determined by leaf-area
majority across the decoded sub-tree and written into `facet_values[i]`.
Downstream stages may consume either source: `Layer::Slice` reads
`facet_values` for whole-triangle paint decisions; `Layer::SlicePostProcess`
may consult `strokes` when sub-facet boundary accuracy matters.

`PaintSemantic`, `PaintValue`, and `PaintStroke` are defined in
`crates/slicer-ir/src/slice_ir.rs`. `PaintSemantic::Custom` is preserved as an
opaque string; the IR does not impose a module-id/name format. `PaintValue`
supports flag, scalar, tool-index, and custom string values. A `PaintStroke`
carries world-space sub-triangle geometry plus its semantic and value.

#### `PaintValue` Eq+Hash invariant (Normative — Packet 91)

`PaintValue` derives `Eq` + `Hash` so it can be used as a `HashMap` key
and as a `RegionKey.variant_chain` element. `Scalar(f32)` is hashed via
`to_bits()`; `Custom(String)` via its String contents; `Flag` and
`ToolIndex` use discriminant + value hashing. This makes the previous
`HashablePaintValue` wrapper (formerly in `paint_segmentation.rs`)
obsolete — code keying `HashMap<PaintValue, _>` directly is the
canonical pattern post-Packet 91. The same `to_bits()` portability
caveat as `ResolvedConfig` applies.

`ModifierVolume`, `ModifierScope`, and `ConfigDelta` are defined in
`crates/slicer-ir/src/slice_ir.rs`. A modifier volume carries its ID, mesh,
sparse config delta, priority, and scope. `ConfigDelta` contains only explicit
fields and never includes baked-in defaults.

### Modifier Resolution Contract

Modifier deltas are merged deterministically during planning:

1. Start with global defaults.
2. Apply object config.
3. Apply matching modifiers in priority-ascending order, last writer wins
   (`stamp_modifier_config_deltas` in
   `crates/slicer-core/src/algos/region_mapping.rs`; a stable sort on
   `ModifierVolume.priority`). Source order breaks equal-priority ties;
   there is no separate `load_order` concept.
4. Apply paint-semantic overlays (`paint_config:`) on top.

For the same key, the last applied value wins. If a later overlay omits a key,
the previously resolved value remains unchanged (no implicit reset).

Worked example (deterministic):

- Global `infill_density = 0.20`
- Object config `infill_density = 0.25`
- Modifier A (`priority=20`) sets `infill_density = 0.30`
- Modifier B (`priority=10`) sets `infill_density = 0.15`
- Effective result: B (priority 10) applies first, then A (priority 20)
  wins: `infill_density = 0.30`.

For equal modifier priorities, the resolver's stable input order breaks the
tie; equal priorities are not themselves an error.

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

`PartSubtype`, `PartSidecarInfo`, `ObjectSidecarInfo`, and `ParsedSidecar` are
host-local types defined in `crates/slicer-model-io/src/sidecar.rs`. They are
not IR or WIT types. `ParsedSidecar.objects` is keyed by object ID; each object
contains part metadata keyed by part ID plus object-scoped metadata.

Behaviour:

- **Missing sidecar:** `Metadata/model_settings.config` absent from the
  archive ⇒ empty `ParsedSidecar` returned silently (no warning).
- **Malformed XML:** parser returns an empty `ParsedSidecar` and emits
  `log::warn!` on the `slicer_model_io::sidecar` target containing the
  substring `"treating all parts as normal_part"`. `load_model` still
  returns `Ok(MeshIR)`; failure is non-fatal.
- **Unknown subtype attribute:** downgraded to `PartSubtype::NormalPart`
  with `log::warn!`.

Parser plumbing: `parse_3mf_sidecar(&mut zip)` is invoked inside
`load_3mf` before the `ZipArchive` is dropped. `ParsedSidecar.objects` is
threaded through `parse_3mf_model_xml` to `resolve_object` for part routing;
`plate_metadata` is parsed into the host-local result but is not currently
stamped into `MeshIR` by that loader path.

### `ModifierVolume.config_delta` Sources (Normative — Packets 56b, 67, 68, 185)

`ModifierVolume.config_delta.fields` can be populated from two
distinct sidecar sources in a single 3MF file:

1. **Part-level `<metadata>`** inside a `<part>` element — every part metadata
   key is copied through the loader's string-to-`ConfigValue` coercion, with
   `extruder` rebased to zero-based indexing and `sparse_infill_density`
   parsed as a density when valid. Invalid special values are warned about and
   skipped.
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
**Current schema_version: 1.3.0** (Bumped from 1.2.0 to 1.3.0 by packet 193 — additive `prev_layer_boundaries` map, keyed by GLOBAL layer index exactly as `overhang_quartile_polygons`. Previously bumped to 1.2.0 by packet 106 — `OverhangRegion` gains `xy_footprint`, new type `QuartileBand`, and new field `overhang_quartile_polygons` on `SurfaceClassificationIR`. Previously bumped to 1.1.0 by packet 36 — new struct `BridgeRegion` and field `bridge_regions: Vec<BridgeRegion>` on `SurfaceClassificationIR`.)

`SurfaceClassificationIR` is defined in `crates/slicer-ir/src/slice_ir.rs`.
It contains per-object surface data plus the host-only
`overhang_quartile_polygons` and `prev_layer_boundaries` maps, both keyed by
global layer index and defaulting to empty maps.

Both `overhang_quartile_polygons` and `prev_layer_boundaries` are keyed by GLOBAL layer index.

**Consumer note (packet 107):** `overhang_quartile_polygons` is consumed by `SliceRegionView::overhang_areas()` and `SliceRegionView::overhang_quartile_polygons()` (both populated by the host marshaller, keyed by `global_layer_index`; see `docs/05_module_sdk.md` "SliceRegionView accessors (packet 107)"). Per-vertex propagation onto `Point3WithWidth.overhang_quartile` (perimeter-generation side) is now wired on **both** perimeter paths — classic-perimeters (packets 104/107, closing T-024/T-077) and arachne-perimeters (packet 148); the former propagation gap closed 2026-07-03.

`ObjectSurfaceData`, `FacetClass`, `SurfaceGroup`, `BridgeRegion`,
`OverhangRegion`, and `QuartileBand` are defined in
`crates/slicer-ir/src/slice_ir.rs`. Facet classifications remain parallel to
the object mesh; bridge and overhang regions carry their facet IDs and derived
geometry, while each quartile band carries a quartile number and polygons.

---

## IR 3 — LayerPlanIR

**Stage:** Output of `PrePass::LayerPlanning`  
**Lifetime:** Blackboard (immutable after PrePass)  
**Critical:** This is the authoritative Z-plane sequence. Every downstream stage derives its Z from here.

`LayerPlanIR`, `GlobalLayer`, `ActiveRegion`, `NonPlanarShellRef`,
`ObjectLayerRef`, `WallGenerator`, and `SupportType` are defined in
`crates/slicer-ir/src/slice_ir.rs`. `LayerPlanIR.global_layers` is the
authoritative global Z sequence; each active region carries its resolved
config, effective height, optional non-planar shell, catch-up state, and tool
index.

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

`RegionMapIR`, `RegionKey`, `RegionPlan`, `ModuleInvocation`, and `ConfigId` are
defined in `crates/slicer-ir/src/slice_ir.rs`. `RegionMapIR.configs` is the
interned `ResolvedConfig` pool, `RegionPlan.config` is its per-plan index, and
`RegionKey.variant_chain` carries ordered paint variants. Use
`RegionMapIR::config_for` and `RegionMapIR::intern_config` rather than relying
on the internal pool layout.

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

`SliceIR` and `SlicedRegion` are defined in
`crates/slicer-ir/src/slice_ir.rs`. A slice is identified by global layer index
and Z; each region carries object/region identity, polygons, infill areas,
optional non-planar surface, effective height, segment paint annotations, shell
depths, shell/bridge fill polygons, and its paint `variant_chain`. The removed
`external_contour` field is not part of the current schema.

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
(see `stamp_modifier_sub_region_configs` in
`crates/slicer-core/src/algos/region_mapping.rs`: it overlays the
modifier volumes' config deltas onto the base `ResolvedConfig`, skipping
`support_enforcer` / `support_blocker` subtypes, and returns a
`BTreeMap<region_id, ResolvedConfig>` stamped per sub-region).

**Sub-region `region_id` namespace.** Sub-region IDs are derived from the base
region ID with a dedicated coprime stride so they never collide with paint's
`1_000_000`-stride namespace:

```
sub_region_id = base_region_id * MODIFIER_VARIANT_REGION_ID_STRIDE + modifier_hash(mi)
```

where `MODIFIER_VARIANT_REGION_ID_STRIDE = 1_000_003` (the next prime above
paint's `1_000_000`, hence coprime — see
`crates/slicer-runtime/src/region_partition.rs`'s `modifier_hash` symbol).
`modifier_hash(mi)` is a
**stable hash of the modifier's identity** — `(object_id, modifier_index,
priority)` in document order within `object.modifier_volumes` — folded into a
non-zero value `< stride` (so the low-order band is reserved for
`base_region_id * stride` itself). The hash is derived from identity, never
from HashMap iteration or footprint geometry, giving a stable sub-region id
that round-trips through `RegionMapIR` and dispatch. The sub-region carries an
**empty `variant_chain`** and is identified by its modifier-namespace
`region_id` alone; the `wall_source_region_id` predicate inverts
`sub_region_id / MODIFIER_VARIANT_REGION_ID_STRIDE` → `Some(base)`. The infill
linker reads only `wall_source_region_id` + `tool_index` + the four fill
polygons (packet 132/133). Modifier meshes are sliced once per layer during
prepass (`slice_modifier_volumes`, extended to material/config-delta
subtypes), and the cached cross-sections are consumed at partition-time
splitting. For overlapping non-support modifier volumes, priority is applied
first and document order breaks ties: the first winning modifier owns the
footprint, and subsequent modifiers intersect only the remaining base area.

`ExPolygon`, `Polygon`, and `Point2` are shared geometry types defined in
`crates/slicer-ir/src/slice_ir.rs`; polygon contours are counter-clockwise,
holes are clockwise, and `Point2` uses native 100 nm integer units.

---

## IR 7 — PerimeterIR

**Stage:** Output of `Layer::Perimeters`, mutated by `Layer::PerimetersPostProcess`
**Current schema_version: 1.1.0** (Bumped additively from 1.0.0 to 1.1.0 by packet 193 for the optional `Point3WithWidth.overhang_distance_mm` field.)

`PerimeterIR`, `PerimeterRegion`, `WallLoop`, `WallFeatureFlags`,
`WallBoundaryType`, `MaterialBoundarySegment`, `LoopType`, and `WidthProfile`
are defined in `crates/slicer-ir/src/slice_ir.rs`. A perimeter region contains
walls, remaining infill areas, seam candidates, and an origin-scoped optional
resolved seam. Wall feature flags are parallel to path points; material-boundary
segments record half-open contour ranges and the tools on each side.

#### Variable-width geometry (Packet 103 — additive, schema 4.3.0)

`ThickPolyline` and `Point2WithWidth` are the 2-D input types consumed by Arachne
perimeter generation before conversion to `ExtrusionPath3D`.

`Point2WithWidth` and `ThickPolyline` are defined in
`crates/slicer-ir/src/slice_ir.rs` as the variable-width 2D Arachne input
types. `variable_width` converts them to an `ExtrusionPath3D` with the stated
default factors.

`variable_width(thick: &ThickPolyline, role: ExtrusionRole) -> ExtrusionPath3D`
maps each `Point2WithWidth` to a `Point3WithWidth` with `z = 0.0`,
`flow_factor = 1.0`, `overhang_quartile = None`, `dist_to_top_mm = 0.0`,
`speed_factor = 1.0`, and the
supplied `role` passed through unchanged.

`ExtrusionPath3D` and `Point3WithWidth` are defined in
`crates/slicer-ir/src/slice_ir.rs`. Paths carry points, role, and a uniform
speed factor. Point records carry position, width, flow, support distance,
optional overhang quartile, and optional signed distance to the previous-layer
boundary; `None` means no boundary measurement.

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

`ExtrusionJunction` and `ExtrusionLine` are defined in
`crates/slicer-ir/src/slice_ir.rs`. A line contains ordered junctions, inset
index, and `is_odd`/`is_closed` topology flags. The new fields use serde
defaults for compatibility with older serialized SliceIR values.

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
  (`rayon` + `boostvoronoi` are native-only) — see the host-service bridge
  record in `docs/DEVIATION_LOG.md` for the full architecture rationale.

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

`SeamCandidate`, `SeamReason`, and `SeamPosition` are defined in
`crates/slicer-ir/src/slice_ir.rs`; the resolved seam position is diagnostic,
while seam-first geometry is represented by the first point of the wall path.

---

## IR 8 — InfillIR

**Stage:** Output of `Layer::Infill`, mutated by `Layer::InfillPostProcess`

`InfillIR` and `InfillRegion` are defined in
`crates/slicer-ir/src/slice_ir.rs`. Each layer carries region-scoped sparse,
solid, and ironing extrusion paths.

---

## IR 9 — SupportIR

**Stage:** Output of `Layer::Support`, mutated by `Layer::SupportPostProcess`

`SupportIR` and `SupportRegion` are defined in
`crates/slicer-ir/src/slice_ir.rs`. Each layer carries a
`regions: Vec<SupportRegion>` collection. Each `SupportRegion` contains
`object_id`, `region_id`, and the support, interface, raft, and ironing
extrusion-path lists for that region.

Packet 172 routing: support paths and raft paths are emitted on the support
tool; interface paths and ironing paths are emitted on the interface tool.
Selection is region-scoped through `object_id` and `region_id`.

---

## IR 9a — SupportGeometryIR

**Stage:** Output of `PrePass::SupportGeometry` — coarse outline prepass results,
committed before `SupportPlanIR` within the same stage.

**Producer:** The host built-in commits `SupportGeometryIR` first within
`PrePass::SupportGeometry`, ahead of any `support-planner` module's
`SupportPlanIR` (see IR 9b below).

**Consumers:** `Layer::Support` modules that need coarse per-`(layer, object,
region)` outline polygons independent of organic branch planning.

`SupportGeometryIR` and `SupportGeometryKey` are defined in
`crates/slicer-ir/src/slice_ir.rs`. The IR carries support layer-height
settings and coarse outline polygons keyed by support-layer index, object ID,
and region ID; `u32::MAX` denotes an intermediate model-resolution layer.

---

## IR 9b — SupportPlanIR

**Stage:** Output of `PrePass::SupportGeometry` (optional; only present when a
`support-planner` module is loaded)

**Current schema_version: 1.3.0** (`CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION` in
`crates/slicer-ir/src/slice_ir.rs`). Packet 119 added the per-point
`Point3WithWidth.dist_to_top_mm` field and the optional raft configuration seam
(1.1.0 → 1.2.0); packet 124 bumped 1.2.0 → 1.3.0 (semver-minor) for the additive
`ExtrusionRole::RaftInfill` variant and its `claim:raft-fill` mapping.

**Producer:** A module holding the `support-planner` claim on `PrePass::SupportGeometry`;
guests of `PrePass::SupportGeometry` emit `SupportPlanIR` via `run-support-geometry`;
the host built-in commits `SupportGeometryIR` first within the same stage
(e.g. the bundled `support-planner` core module — a simplified port of OrcaSlicer's
`TreeSupport::detect_overhangs` + `TreeSupport::drop_nodes`).

**Consumers:** `Layer::Support` modules that declare `SupportPlanIR` as a read in
their manifest (notably `tree-support`). Modules whose algorithm is inherently
per-layer (e.g. `traditional-support`'s scan-line filler) intentionally do not
read this IR.

`SupportPlanIR`, `RaftPlan`, and `SupportPlanEntry` are defined in
`crates/slicer-ir/src/slice_ir.rs`. Support plans contain region-scoped branch
segments and an optional configuration-only raft plan. Entry layer indices are
signed so negative values can reserve raft-prefix layers.

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
invocation, plus the batched host-service calls and typed diagnostics it
emitted. The diagnostic field was added in Packet 118 to carry the prepass
diagnostic channel defined in `docs/adr/0010-typed-diagnostic-channel.md`
from the host into the scheduler audit surface.

`ModuleAccessAudit` is defined in
`crates/slicer-scheduler/src/validation.rs`. It records the module ID, runtime
read/write paths, batched host-service calls (`batch_calls: Vec<(String, u32)>`
— one entry per batch, added with ADR-0049's batched host services), and FIFO
typed diagnostics. Scheduler validation compares
only the runtime read/write paths; the other fields remain observability
channels.

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

`SeamPlanIR`, `SeamPlanEntry`, `SeamPosition`, and `ScoredSeamCandidate` are
defined in `crates/slicer-ir/src/slice_ir.rs`. Entries are keyed by the full
`RegionKey`, including `variant_chain`; duplicate full keys are rejected at
commit. Seam positions and scored candidates use millimeter-valued
`Point3WithWidth` coordinates.

`SeamPlanEntry.chosen_candidate` is consumed via
`PerimeterRegionView.resolved_seam` so the apply-stage module (seam-placer)
operates on a pre-resolved seam without rescoring.

**Identity validation (Normative — packet 178):** seam-plan harvest validates
every `RegionKey` component — `global_layer_index`, `object_id`, `region_id`,
and `variant_chain`. Unsupported or malformed variant-chain values (including
Custom paint values not representable at the WIT boundary) produce a fatal
structured contract error and prevent `SeamPlanIR` commit; identity failure is
a contract error, never a best-effort drop.

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

`LightningTreeIR` and `LightningTreeEntry` are defined in
`crates/slicer-ir/src/slice_ir.rs`. Entries are scoped by object, region, and
signed layer index and carry two-point integer-coordinate tree edges.

**Supersession note (packet 137, C-class):** the packet 137 contract's
`LightningTreeEntry` shape (which omitted `region_id`) is superseded by the
implemented per-region contract shown in IR 9d above. The
`(object_id, region_id, layer_index)` keying is authoritative — the entry
carries `region_id` exactly as `SupportPlanEntry` and `SeamPlanEntry` do, and
the view lookup below is region-scoped, not object-scoped.

**Consumption pattern — read-view via `lightning-tree-segments`:**

The host exposes the IR to a `Layer::Infill` guest via the
`lightning-tree-segments` method on the `paint-region-layer-view` WIT resource
(`crates/slicer-schema/wit/deps/ir-types.wit:206`; the guest exports the
`slicer:layer-infill@1.0.0` package's `infill` interface, whose `run` function
receives the `paint: paint-region-layer-view` argument). The guest looks up the per-layer
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
**Current schema_version: 1.2.0** (authoritative source: `CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION` in `crates/slicer-ir/src/slice_ir.rs`). Introduced at 1.0.0; packet 125 added the additive `PrintEntity.tool_index: u32` field (region_id↔tool split), bumping 1.0.0→1.1.0; packet 189 added the additive `LayerCollectionIR.speed_profiles: Vec<EntitySpeedProfile>` side table (per-point speed-factor carrier), bumping 1.1.0→1.2.0. Packet 39 earlier renamed `TravelMove.entity_idx: u32` → `entity_id: u64` and added `entity_id: u64` on `PrintEntity`, decoupling travel anchors from positional indices so finalization-stage entity insertion no longer invalidates anchors (these landed without bumping the constant beyond 1.x).

**Ownership lifecycle — three phases:**

1. **During parallel per-layer execution:**
   `Layer::PathOptimization` produces a **complete, self-consistent**
   `LayerCollectionIR` for its layer. All entities are ordered, all z-hops
   and intra-layer tool changes are resolved. Nothing is left partially
   populated for a later stage to finish. The completed struct is written
   into the Blackboard's `layer_outputs: Vec<Option<LayerCollectionIR>>`
   slot (one slot per global layer index; see `Blackboard` in
   `crates/slicer-runtime/src/blackboard.rs`). Each slot is written exactly
   once by the thread that processed that layer — no mutex required.

2. **After the rayon join — `PostPass::LayerFinalization`:**
   Ownership of all `LayerCollectionIR` values is **moved out of the
   Blackboard** into a plain `Vec<LayerCollectionIR>` owned by the
   finalization executor. The `layer_outputs` vector in the Blackboard
   becomes unreachable at this point. The finalization executor holds
   exclusive mutable ownership of the `Vec` and is single-threaded — no
   `RwLock` or `Mutex` is needed. Finalization modules may append entities
   to existing layers or insert new synthetic layers (e.g. wipe tower
   slices).

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

The canonical definitions of `LayerCollectionIR`, `PrintEntity`,
`TravelMove`, `EntitySpeedProfile`, `ToolChange`, `ZHop`, and
`ExtrusionRole` live in `crates/slicer-ir/src/slice_ir.rs`. The structs'
doc comments there carry the packet-39 /
packet-125 / packet-189 contracts summarized in the ownership lifecycle
above; read them from source rather than from a copy here.

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
| `RaftInfill`          | 50   |
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
  travel anchor; the error string names the offending `entity_id`.
  <!-- VERIFY: no production call site for validate_travel_anchors was found in
       crates/slicer-runtime/ or modules/ (grep 2026-08-06); it is exported from
       slicer-ir and exercised only by crates/slicer-ir/tests/ir_validation_tdd.rs.
       The prior claim that finalization invokes it before handing the layer to
       GCodeEmit is unverified. -->

### `LayerCollectionIR::default()` contract (Normative — Packet 79 fixture support)

`LayerCollectionIR` implements `Default` (an explicit impl, not a
derive). The default-field values are
load-bearing because the test-support
`LayerCollectionFixtureBuilder` (in `slicer-sdk::test_support::fixtures`)
only sets four fields explicitly (`global_layer_index`, `z`,
`ordered_entities`, `tool_changes`) and lets `Default` populate the
rest: `z_hops = vec![]`, `annotations = vec![]`, `travel_moves =
vec![]`, `speed_profiles = vec![]` (packet 189), and `schema_version =
CURRENT_LAYER_COLLECTION_IR_SCHEMA_VERSION`. Because the impl is explicit,
`speed_profiles = vec![]` is written there literally.
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

The canonical definitions of `GCodeIR`, `GCodeCommand`, `RetractMode`,
and `PrintMetadata` live in `crates/slicer-ir/src/slice_ir.rs`. The
variant doc comments there carry the packet-34 / packet-54 /
packet-59 contracts summarized in the subsections below; read them from
source rather than from a copy here.

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

**Minimum-key gate (normative — packet 167):** PNP pads `CONFIG_BLOCK` with
`; key = value` entries until the block holds 96 entries even when `raw_config`
is minimal (`serialize_config_block` in `crates/slicer-gcode/src/serialize.rs`
stops once `emitted.len() >= 96`), keeping OrcaSlicer's viewer minimum-key gate
(`ConfigBase::load_from_gcode_file` rejects blocks under ~80 pairs) satisfied.
Cosmetic padding must not fabricate motion or machine-limit values — a padding
key may only be a key the viewer's `GCodeProcessor` does not feed into
motion/time computation (pattern/enum/toggle/count/geometry-cosmetic keys such
as `wall_loops`, `top_shell_layers`, `infill_direction`, `wall_generator`,
`support_type`), and any retained cosmetic padding key uses the corresponding
OrcaSlicer upstream default value per `docs/ORCA_CONFIG_REFERENCE.md`.

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

##### G-code dialect variants (Normative — packet 171)

The five `gcode_flavor` values select per-flavor command spellings in
`crates/slicer-gcode/src/flavor.rs`, ported from canonical `GCodeWriter.cpp`
(`set_temperature`, `set_acceleration_internal`, `set_jerk_xy`,
`set_junction_deviation`, `set_pressure_advance`, and
`supports_separate_travel_acceleration`). Divergent spellings:

- **Temperature (`GCodeCommand::Temperature`):** RRF (`reprapfirmware`)
  emits `G10 P<tool> S<celsius>`, appending `M116` on `wait: true` —
  never `M104`/`M109`. The other four flavors emit `M104`/`M109` with
  `T<tool> S<temp>`.
- **Acceleration:** Marlin emits `M204 S<accel>`; Marlin2 and RRF emit
  `M204 P<accel>`; Repetier emits `M201 X<accel> Y<accel>`. Separate
  travel acceleration (`supports_separate_travel_acceleration`) is
  supported by exactly Repetier, Marlin2, and RRF; Marlin and Klipper
  have no separate form.
- **Jerk and junction deviation:** jerk is `M205 X.. Y..` for Marlin,
  Marlin2, and RRF, `M207 X..` for Repetier, and Klipper's
  `SET_VELOCITY_LIMIT SQUARE_CORNER_VELOCITY=..`; junction deviation
  (`M205 J..`) is Marlin2-only.
- **Pressure advance:** Marlin/Marlin2 `M900 K..`, RRF `M572 D0 S..`,
  Repetier `M233 X.. Y..`, Klipper `SET_PRESSURE_ADVANCE ADVANCE=..`.

Uniform across all five flavors: fan (`M106 S`), tool-change (`T<n>`),
extrusion-mode (`M82`/`M83`), firmware-retract (`G10`/`G11`), and
bed-temperature (`M140`/`M190`) commands — divergences exist only in flavors
outside the supported five.

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
<!-- VERIFY: resolve_feedrate (crates/slicer-gcode/src/emit.rs) does not
     reference the initial_layer_* fields — FeedrateConfig declares them
     (crates/slicer-ir/src/feedrate.rs) but the resolution match never reads
     them, and no first-layer detection (z-compare or flag) exists in the
     function body as of 2026-08-06. The claims above are unverified. -->

### Stream-level extrusion mode (Normative — packet 54, 59)

`GCodeCommand::Move.e` is an absolute E position in the IR (the
E-accumulator); serialization converts consecutive moves into signed
deltas in **relative** extrusion mode (M83) or emits absolute positions in
**absolute** mode (M82). Mode is a stream-level invariant — the emitter
opens the stream with `GCodeCommand::ExtrusionMode { absolute }` (packet
59) and resets the E-accumulator with `G92 E0` on mode change or layer
reset. Mode is selected by the config key `use_relative_e_distances`
(boolean; default `true` → M83). Carrier helper:
`DefaultGCodeSerializer::with_extrusion_mode(mode)`.

### M73 progress emission (Normative — packet 175)

`inject_m73` (`crates/slicer-gcode/src/m73.rs`) runs inside
`DefaultGCodeEmitter::emit_gcode` after the estimator fills
`metadata.estimated_print_time_s`, operating on `GCodeIR.commands` as
`GCodeCommand::Raw` entries so post-process modules and the serializer see the
injected lines. Emission contract:

- The M73 pair is **prepended to the head** of the command list, so it
  precedes the `ExtrusionMode` command (which is no longer index 0 when
  M73 is enabled). `machine-gcode-emit` at `PostPass::GCodePostProcess`
  rebuilds the stream rather than splicing, so the resolved start template
  still precedes both the M73 pair and `ExtrusionMode`; the ordering is
  pinned by `machine_start_gcode_precedes_m73_and_extrusion_mode` in
  `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs`.
- The emitter injects `M73 P<pct> R<remaining_min>` followed immediately by
  an identical `M73 Q<pct> S<remaining_min>` line (same estimate for both
  masks; OrcaSlicer `M73 P%s R%s` / `M73 Q%s S%s` reference masks) at three
  points: **stream start** (`M73 P0 R<total_min>`), **changed-value layer
  boundaries** (after each `;LAYER_CHANGE` `Raw` marker whose `(pct, min)`
  differs from the last emitted pair — Orca's `process_line_move` dedup; `P`
  values are monotonically non-decreasing), and **stream end**
  (`M73 P100 R0`). Empty/zero-total streams are a no-op.
- Filament-used and estimated-printing-time comments are appended as
  `GCodeCommand::Raw` entries: `; filament used [mm] = …`,
  `; filament used [cm3] = …`, the `; filament used [g] = …` line (only when
  a filament density is configured — never `0.00`), and
  `; estimated printing time (normal mode) = …` (`format_time_dhms`
  formatting, zero-leading units omitted, seconds always present). These
  comment lines are unconditional.
- `disable_m73` (bool, default `false`) suppresses **only** the M73 lines —
  neither the `P`/`R` nor the `Q`/`S` pairs — while the filament/time comment
  block remains present.

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

| `ExtrusionRole`                                                                  | Tolerance source     |
|----------------------------------------------------------------------------------|----------------------|
| `OuterWall`, `InnerWall`, `ThinWall`, `Skirt`, `Brim`, `GapFill`, `RaftInfill`    | `gcode_resolution`   |
| `SparseInfill`, `TopSolidInfill`, `BottomSolidInfill`, `InternalSolidInfill`, `BridgeInfill`, `Ironing`, `WipeTower`, `PrimeTower` | `infill_resolution`  |
| `SupportMaterial`, `SupportInterface`                                            | `support_resolution` |
| Travel (synthetic — no `ExtrusionRole`), `Custom(_)` (unknown)                   | `0.0` (no D-P)       |

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

The `extensions: BTreeMap<String, ConfigValue>` field on `ResolvedConfig` is the soft landing zone for config keys contributed by modules not present in the host's schema snapshot. Keys always round-trip safely.

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

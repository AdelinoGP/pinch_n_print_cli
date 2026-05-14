# Design: 56b_threemf-modifier-part-ir-routing

## Controlling Code Paths

### State after Packet 56 (precondition for this packet)

- `crates/slicer-host/src/model_loader.rs` carries:
  - Host-local `PartSubtype` enum and `ObjectSidecarInfo` / `PartSidecarInfo` carrier structs.
  - `parse_3mf_sidecar(&mut zip) -> HashMap<u32, ObjectSidecarInfo>` helper.
  - `load_3mf` calls `parse_3mf_sidecar` before the `ZipArchive` is dropped.
  - `parse_3mf_model_xml` signature widened to accept the sidecar map.
  - `resolve_object` signature widened to accept `_sidecar: &HashMap<u32, ObjectSidecarInfo>` (underscore prefix; body unchanged from pre-Packet-56 behavior — every part still merged into one solid mesh).
- `MeshIR.schema_version` is still `SemVer { 1, 0, 0 }` (Packet 56 did not bump it).
- `ObjectMesh.modifier_volumes` is still `Vec::new()` for every 3MF load (Packet 56 did not populate it).
- `crates/slicer-host/src/region_mapping.rs::execute_region_mapping` signature is unchanged from pre-Packet-56 state (no modifier-volumes parameter).
- `crates/slicer-host/src/pipeline.rs` does not thread `modifier_volumes` into region-mapping.

### After this packet

- `resolve_object` branches on `sidecar` (underscore removed):
  - For each `<component>` resolved during recursion, look up its `<objectid>` in the sidecar map. If the entry has a matching `<part id>` (per Bambu's convention: the child component's `<objectid>` matches the sidecar's `<part id>` inside the parent's `<object id>`), use that part's `PartSubtype`.
  - If `PartSubtype == NormalPart` (or no sidecar entry): existing behavior — merge triangles into `merged_vertices`/`merged_indices`, concatenate paint data.
  - Otherwise: skip the merge. Construct a `ModifierVolume`:
    - `id: ModifierId` — deterministic, derived from `(parent_object_uuid, part_id, subtype_str)` via stable hash (NOT `uuid::Uuid::new_v4()`). Step 2 FACT dispatch returns the Packet 39 `stable-entity-ids` recipe.
    - `mesh: IndexedTriangleSet` — the part's world-space transformed mesh (apply `<component>` transform composition as today, but populate a separate mesh rather than merging).
    - `config_delta: ConfigDelta` — typed keys derived from `PartSidecarInfo.metadata`:
      - `ConfigKey::from("subtype") -> ConfigValue::String("modifier_part" | "negative_part" | "support_enforcer" | "support_blocker")` (always present).
      - `ConfigKey::from("fuzzy_skin") -> ConfigValue::String(...)` (when sidecar metadata contains `fuzzy_skin`).
      - `ConfigKey::from("extruder") -> ConfigValue::Int(parsed_i64)` (when sidecar metadata contains `extruder` and parses as i64; emit `log::warn!` and skip on parse failure).
      - `ConfigKey::from("matrix") -> ConfigValue::String(verbatim_string)` (when sidecar metadata contains `matrix`; telemetry only).
    - `priority: i32` — 0 for `ModifierPart`, 100 for `NegativePart`, 200 for `SupportEnforcer`, 300 for `SupportBlocker` (deterministic ordering hint; consumers may ignore).
    - `applies_to: Option<ObjectId>` — set to the parent object's `ObjectId` (so the volume is scoped to its parent object, not the global plate).
  - If a part is classified non-`NormalPart` AND has `paint_data` attached: drop the paint data and emit `log::warn!` with target chosen at Step 2 (FACT dispatch returns the existing log target convention in `model_loader.rs`), message containing the substring `"paint data on non-normal part dropped"` and the part id.
- `MeshIR.schema_version` bumps to `SemVer { 1, 1, 0 }` at `model_loader.rs:194-199`.
- `crates/slicer-host/src/region_mapping.rs::execute_region_mapping` widens its signature to accept per-object `modifier_volumes: &[ModifierVolume]` (or pulls them from `ExecutionPlan` — Step 5 picks based on the existing signature's ownership model). For each `(layer, region)`:
  - For each `ModifierVolume` in scope whose `config_delta.fields[ConfigKey::from("subtype")] == ConfigValue::String("modifier_part")`:
    - Project the modifier's mesh to the layer's Z plane via the slicer entry-point identified by Step 5's FACT dispatch (likely candidate: existing slicing code already used for `paint_data` stroke projection).
    - Convert the projected polygons to `Point2` in scaled integer units (1 unit = 100 nm; `Point2::from_mm(x, y)`).
    - Run `slicer_core::polygon_ops::intersection(region.polygon, modifier_projection)`.
    - If the intersection is non-empty, stamp `RegionPlan.config["fuzzy_skin.apply-to-all"] = ConfigValue::Bool(true)`. (Insert into `ConfigDelta.fields` map.)
  - The stamping runs AFTER Packet 51's paint-semantic overlay; it is additive (it only sets a key Packet 51 does not set).
  - Preserve the no-modifier fast path: if `modifier_volumes.is_empty()`, skip the entire overlap loop. Bit-identical output for fixtures without sidecars.
- `crates/slicer-host/src/pipeline.rs` threads per-object `modifier_volumes` into the `execute_region_mapping` call. No new stages.
- `modules/core-modules/fuzzy-skin/manifest.toml`'s `[config.schema]` block is verified to declare `apply-to-all`. If absent, register it additively (entry name: `apply-to-all`, type: `bool`, default: `false`).

## Neighboring Tests / Fixtures

- `crates/slicer-host/tests/threemf_transform_tdd.rs` — must stay green; this packet preserves `<build>/<item>` composition unchanged.
- `crates/slicer-host/tests/gcode_emit_tdd.rs` — must stay green.
- `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` — no-sidecar regression baseline. Slice output must be byte-identical to pre-Packet-56b output (no sidecar → empty `modifier_volumes` → fast path).
- `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs` — Packet 51 paint-config overlay regression. Reuses `count_perimeter_markers_in_z_band` helper which this packet's E2E test imports.
- `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — Packet 56's parser suite. Must stay green; this packet's `resolve_object` branching does not change parser behavior.
- `crates/slicer-host/tests/core_module_ir_access_contract_tdd.rs` — manifest schema contract. Must stay green after the (possibly no-op) `fuzzy-skin/manifest.toml` edit.
- `resources/benchy_4color.3mf` — primary fixture for the modifier-part E2E.

## Architecture Constraints

- Scaled integer units: `slicer_core::polygon_ops` operates in `Point2` scaled integer units (1 unit = 100 nm; `docs/08_coordinate_system.md`). Modifier projections MUST convert to `Point2::from_mm(x, y)` before any `polygon_ops` call.
- Per-layer projection: use the existing slicer entry-point that already projects paint-data strokes to layer Z. Step 5 FACT dispatch identifies the function (likely candidate: a function in `slicer-core` taking `(triangle_set, z) -> Vec<Polygon>`).
- Determinism: `ModifierId` derivation MUST be stable across runs. Hash recipe locked at Step 2 (FACT dispatch returns Packet 39's `stable-entity-ids` precedent).
- WIT boundary: clean — re-confirmed at Step 0.
- IR versioning: additive minor bump (1.0.0 → 1.1.0). No new struct, no removed field, no changed enum variant. The producer contract widens; the consumer contract is unchanged.
- Loop ordering inside `execute_region_mapping`: Packet 51's paint-semantic overlay runs first; this packet's modifier-overlap stamp runs second. The latter only sets the `fuzzy_skin.apply-to-all` key, which Packet 51 does not set — no clobbering.

## Selected Approach (Locked Decisions)

| Decision | Locked choice | Justification |
|---|---|---|
| Geometry routing | `modifier_part`, `negative_part`, `support_enforcer`, `support_blocker` → `ObjectMesh.modifier_volumes` (each as its own `ModifierVolume`). `NormalPart` → merged into solid mesh (existing behavior). | Reuses existing IR field. Packets 56c branches on `config_delta.fields[ConfigKey::from("subtype")]` for its consumer wiring. |
| Per-modifier IR fields | Reuse `ModifierVolume.config_delta` with typed keys (`subtype`, `fuzzy_skin`, optional `extruder`, optional `matrix`). NO new IR fields. NO new IR struct. | Minimum-surface change. Mirrors Packet 51's `paint_overrides` overlay pattern. |
| `subtype` config value | `ConfigValue::String("modifier_part" | "negative_part" | "support_enforcer" | "support_blocker")`. Lowercase, underscore. | Matches the sidecar's `subtype=` attribute verbatim. Easy to grep and assert. |
| Paint drop on non-`NormalPart` | Drop with `log::warn!` per dropped part. The warning contains the substring `"paint data on non-normal part dropped"` and the part id. | DEV-048 contract. |
| Schema bump | `MeshIR.schema_version`: 1.0.0 → 1.1.0 (additive minor). Annotation in `docs/02_ir_schemas.md` IR 0 section per IR 2 / IR 5 precedent. | Mirrors Packet 36 / Packet 51 precedent. |
| `modifier_part` fuzzy region wiring | **Region-mapping direct stamp** (user-selected Option 1 in original packet). In `execute_region_mapping`, for each `(layer, region)`, run `slicer_core::polygon_ops::intersection` against per-layer modifier projection. On non-empty overlap, stamp `RegionPlan.config["fuzzy_skin.apply-to-all"] = ConfigValue::Bool(true)`. | User-approved at original-packet-author time. Does not piggyback on PaintSegmentation, so the `paint_fuzzy_skin` triangle path and the modifier-overlap path are independent producers; both stamp the same key; final fuzz region = union. |
| Modifier projection ownership | `ModifierVolume.mesh` is in world space at IR construction time (transform already applied during `resolve_object`). Region-mapping projects the world-space mesh to layer Z. | Avoids re-applying the transform at every layer. Consistent with `paint_data` projection. |
| `ModifierId` derivation | Stable hash of `(parent_object_uuid, part_id, subtype_str)` per Packet 39's `stable-entity-ids` precedent. Step 2 FACT dispatch returns the one-line recipe. | Deterministic across runs. |
| `applies_to` field | Set to the parent object's `ObjectId`. | Volume is scoped to its parent object, not the plate. |
| `priority` field | 0 (modifier), 100 (negative), 200 (support_enforcer), 300 (support_blocker). | Deterministic ordering hint. Consumers may ignore. |
| Fuzzy-skin manifest gate | Step 3 FACT dispatch checks the `[config.schema]` block. If `apply-to-all` is missing, additive edit. If present, no-op. | Activation Q4 from the original packet. |
| Negative-case behavior on parse failures | `extruder` parse failure → `log::warn!` + skip the key (other keys preserved). `matrix` always captured verbatim (no parsing). | Defensive; matches Packet 56's "downgrade with warning" tone. |
| `resolve_object` signature | Rename `_sidecar` → `sidecar`; widen the return tuple to include `Vec<ModifierVolume>`. The accumulator is built up through the component recursion alongside `merged_vertices`/`merged_indices`. | Single recursion pass; no second walk. |

## Rejected Alternatives

| Alternative | Reason rejected |
|---|---|
| New `ModifierVolumeIR` schema struct with fully typed `fuzzy_skin: Option<FuzzySkinMode>` etc. | YAGNI. `ConfigDelta` already carries typed values via `ConfigValue` enum. Doubles maintenance surface. |
| PaintSegmentation piggyback for `modifier_part` fuzzy_skin | User selected Option 1 (region-mapping direct stamp). Recorded for awareness only. |
| Inline `negative_part` subtract inside `execute_region_mapping` | Mixes geometry mutation with config overlay. Activation Q3 = Option 1 picked a separate stage. Out of scope for this packet — owned by Packet 56c. |
| Consume sidecar `<part>/<metadata key="matrix">` as geometry source | Sidecar matrix duplicates model XML's `<component>` `ST_Matrix3D`. Model XML path is already exercised. Sidecar matrix is telemetry only. |
| Apply paint-data accumulation to non-`NormalPart` parts (preserve rather than drop) | No consumer needs paint on a modifier. Preserving wastes IR memory and risks double-counting if a future consumer accidentally reads it. |
| `uuid::Uuid::new_v4()` for `ModifierId` | Non-deterministic; breaks reproducible-build comparison. Packet 39 precedent dictates stable hash. |
| Add `WallFeatureFlags.fuzzy_skin` propagation through perimeter generation | The `fuzzy-skin` module's existing `apply-to-all` config flag covers the entire region. No per-vertex flag path needed. |
| Run modifier-overlap stamp BEFORE Packet 51's paint-semantic overlay | Both producers stamp the same `fuzzy_skin.apply-to-all` key only on overlap, so ordering doesn't matter for the bool — but stamping after preserves Packet 51's invariants more clearly. |

## Code Change Surface (≤ 3 primary files per step)

Primary files this packet edits:

1. `crates/slicer-host/src/model_loader.rs` — `resolve_object` branching + `ModifierVolume` construction + paint-drop + schema bump. Up to ~200 added lines on top of Packet 56's additions. Watch the 800-line ceiling.
2. `crates/slicer-host/src/region_mapping.rs` — modifier-overlap config stamp loop. ~80 added lines.
3. `crates/slicer-host/src/pipeline.rs` — thread `modifier_volumes` into the region-mapping call. ~10 added lines.
4. `modules/core-modules/fuzzy-skin/manifest.toml` — additive edit only if `apply-to-all` is missing.
5. Two new test files under `crates/slicer-host/tests/`.
6. Documentation files (`docs/02_ir_schemas.md`, `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md`).

Each step picks at most three of these and a worker dispatch covers each step in isolation.

## Read-only Context the Implementer Needs

| Path | Lines | Purpose |
|---|---|---|
| `crates/slicer-host/src/model_loader.rs` | 130-203, 430-550 | `load_model`, `resolve_object` body. Post-Packet-56 state. |
| `crates/slicer-ir/src/slice_ir.rs` | 192-295 | `ConfigDelta`, `ModifierVolume`, `ObjectMesh`. |
| `crates/slicer-host/src/region_mapping.rs` | 1-260 | `execute_region_mapping` signature + Packet 51 overlay pattern. |
| `crates/slicer-host/src/config_resolution.rs` | 80-220 | `ConfigKey`/`ConfigValue` helpers. |
| `modules/core-modules/fuzzy-skin/src/lib.rs` | 1-120 | `apply-to-all` consumer branch (read-only verification). |
| `modules/core-modules/fuzzy-skin/manifest.toml` | full (small) | `[config.schema]` block. |
| `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs` | search for `count_perimeter_markers_in_z_band` | Helper reused by the E2E test. |
| `docs/02_ir_schemas.md` | 5, 62-244, 250, 506 | Versioning rule + IR 0 + IR 2/5 schema_version annotation precedents. |
| `docs/08_coordinate_system.md` | full | Scaled integer units. |
| `crates/slicer-host/src/pipeline.rs` | search for `execute_region_mapping(` | Insertion point for `modifier_volumes` threading. |

## Out-of-Bounds Files (must not be loaded directly)

- `crates/slicer-macros/src/lib.rs` (>2300 lines).
- `crates/slicer-sdk/` — all files.
- `crates/slicer-ir/src/lib.rs`, `crates/slicer-ir/src/mesh_ir.rs`, etc. — read only the narrow sections in `slice_ir.rs`; do not load full files.
- All `wit/**` and `crates/slicer-host/src/wit_host.rs`, `dispatch.rs` — re-confirmed clean by Step 0 FACT; do not load otherwise.
- `OrcaSlicerDocumented/**` — always delegate via Explore agent with LOCATIONS. Two dispatches total in this packet (Steps 2 and 5).
- `target/`, `Cargo.lock`, generated code.
- `crates/slicer-host/src/paint_segmentation.rs`, `prepass.rs`, `negative_part_subtract.rs` (which will be new in Packet 56c) — owned by Packet 56c. Do not edit here.

## Data and Contract Notes

- `ConfigDelta.fields` is `HashMap<ConfigKey, ConfigValue>` (per `crates/slicer-ir/src/slice_ir.rs:230-235`).
- `ConfigKey::from(&str)` — Step 2 FACT dispatch confirms constructor signature and whether it accepts arbitrary strings.
- `ConfigValue` enum variants — Step 2 FACT dispatch returns the exact variant list.
- `RegionPlan.config` is a `ResolvedConfig` (per Packet 51's design). This packet mutates it in place after Packet 51's overlay has run. Ordering: (1) Packet 51 paint-semantic overlay, (2) this packet's modifier-overlap stamp. Additive on the `fuzzy_skin.apply-to-all` key only.
- `MeshIR.schema_version`: change the `SemVer { 1, 0, 0 }` literal at `model_loader.rs:194-199` to `SemVer { 1, 1, 0 }`. No host-side enforcement update required; additive minor preserves backward compat.
- Deterministic `ModifierId`: Step 2 FACT dispatch returns the one-line recipe used by Packet 39's `stable-entity-ids`. Apply the same recipe with input `format!("{parent_uuid}:{part_id}:{subtype_str}")` or whatever the recipe dictates.

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|
| `resolve_object` recursion has subtle paint-data alignment logic (lines 488-527). Adding modifier accumulation may break paint alignment for partially-modifier objects. | Step 2 TDD covers two cases: (a) component-with-mixed-subtypes paint alignment preserved on `NormalPart` rows; (b) paint dropped on non-`NormalPart` rows. Both must be GREEN before Step 5. |
| Per-layer modifier projection is O(layers × volumes × triangles). On a 200-layer benchy with a 12-triangle cube modifier, trivial. On future high-triangle modifiers it could be a hotspot. | Out of scope for this packet. Log a TODO in `crates/slicer-host/src/region_mapping.rs` near the overlap loop. |
| Support enforcer / blocker subtypes ALSO route into `modifier_volumes` here (their geometry storage is identical to `modifier_part`'s), but their downstream consumer is not wired until Packet 56c. Until Packet 56c lands, fixtures with `support_*` parts will have populated `modifier_volumes` entries with no downstream effect. | This is intentional. The IR plumbing is uniform; consumer wiring is staggered across packets. No tests in this packet exercise `support_*` end-to-end. Packet 56c's tests cover the consumer side. |
| Existing call sites of `execute_region_mapping` break when the signature widens. | Step 5 FACT dispatch enumerates every call site. Update each one to pass `modifier_volumes` (or an empty slice for unit tests). |
| `apply-to-all` already present in fuzzy-skin manifest but with a different name (`apply_to_all`, `applyToAll`, etc.). | Step 3 FACT dispatch returns the verbatim key. If a different name is in use, this packet stamps the verbatim key Packet 51 / fuzzy-skin module actually reads. The AC's `"fuzzy_skin.apply-to-all"` literal is updated if the dispatch returns a different key. |
| `ModifierId` collisions if two parts share `(parent_object_uuid, part_id, subtype)`. | Bambu's sidecar requires `<part id>` uniqueness inside its parent `<object>`. Step 2 TDD asserts the assumption against a synthetic two-part fixture. |
| The original packet's Activation Q3 (negative-part subtract stage placement) is "answered" only by Packet 56c. This packet's negative-part routing populates `modifier_volumes` but does not subtract. If a fixture with `negative_part` is sliced AFTER this packet lands but BEFORE Packet 56c, the negative volume sits in IR with no effect. | Acceptable. Fixtures with `negative_part` are exercised only by Packet 56c's synthetic tests. No real fixture in `resources/` carries `negative_part`. |

## Open Questions Blocking Activation

- **Q1 (Packet 56 status).** Confirm `56_threemf-sidecar-parser` is `status: implemented`. Verify by grep on `.ralph/specs/56_threemf-sidecar-parser/packet.spec.md`.
- **Q2 (deviation numbering).** Confirm DEV-048 is the next free deviation slot. Verify against `docs/DEVIATION_LOG.md` at packet-open time.

## Locked Assumptions and Invariants

1. WIT scope is clean (re-confirmed at Step 0).
2. `ModifierVolume.config_delta` is the IR carrier for all per-part metadata. No new IR struct introduced.
3. The `fuzzy-skin` module is unchanged. The region-stamped `apply-to-all` config key is the consumer.
4. The model XML `<component>` transform is the geometry source. Sidecar `<part>/<metadata key="matrix">` is telemetry only.
5. `MeshIR.schema_version` bumps additively (1.0.0 → 1.1.0).
6. `resources/benchy_painted.3mf` (no sidecar) slices byte-identical to pre-Packet-56b output.
7. Determinism: `ModifierId` strings stable across repeated runs with identical inputs.
8. Paint data on non-`NormalPart` rows is dropped at load time with a structured warning. There is no consumer that needs it.
9. `support_enforcer` and `support_blocker` parts ALSO populate `modifier_volumes` here, but their downstream consumer is not wired until Packet 56c. This is intentional.
10. `negative_part` parts ALSO populate `modifier_volumes` here, but the host stage that consumes them (`apply_negative_part_subtract`) is not introduced until Packet 56c. This is intentional.

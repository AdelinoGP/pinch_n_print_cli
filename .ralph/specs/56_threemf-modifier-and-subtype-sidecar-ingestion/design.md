# Design: 56_threemf-modifier-and-subtype-sidecar-ingestion

## Controlling Code Paths

### Today's behavior (current state of master, verified by direct narrow reads)

- `crates/slicer-host/src/model_loader.rs:130-203` — `load_model` for 3MF builds one `ObjectMesh` per item returned by `load_3mf`, hardcodes `SemVer { 1, 0, 0 }`, and initializes `modifier_volumes: Vec::new()` unconditionally for every format including 3MF (lines 179).
- `crates/slicer-host/src/model_loader.rs:430-552` — `resolve_object` recursively walks `<component>` children of an `<object>` and merges every leaf mesh into one `IndexedTriangleSet` (`merged_vertices` / `merged_indices` at lines 463-480). Paint data is aligned per-component by `semantic` and concatenated (lines 488-527). There is no branch on part identity beyond `objectid`.
- `crates/slicer-host/src/model_loader.rs:555-587` — `load_3mf` opens the ZIP archive and calls `find_model_path` which only ever returns `3D/3dmodel.model` (line 579-581). The ZIP archive handle is dropped after this call.
- `crates/slicer-host/src/model_loader.rs:599-` — `parse_3mf_model_xml` consumes the model XML bytes. It does not see the ZIP archive and cannot reach `Metadata/model_settings.config`.
- `crates/slicer-ir/src/slice_ir.rs:252-265` — `ModifierVolume { id, mesh, config_delta, priority, applies_to }` already exists. `config_delta: ConfigDelta { fields: HashMap<ConfigKey, ConfigValue> }` (lines 230-235). **No new IR struct is needed.**
- `crates/slicer-ir/src/slice_ir.rs:267-295` — `ObjectMesh.modifier_volumes: Vec<ModifierVolume>` already exists. The 3MF path simply never populates it.
- `crates/slicer-host/src/region_mapping.rs:200-258` — `execute_region_mapping` accepts `paint_regions: Option<&PaintRegionIR>` and `paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>` (Packet 51). The signature must widen to accept per-object `modifier_volumes` (or it pulls them off `ExecutionPlan`, whichever is cleaner — Step 6 picks).
- `modules/core-modules/fuzzy-skin/src/lib.rs:51-54, 80-81` — module reads `apply-to-all` from `ConfigView`; when `true`, all outer walls in the region are fuzzed regardless of per-vertex `WallFeatureFlags`. This is the consumer the region-direct-stamp path exploits.
- `docs/02_ir_schemas.md:5, 62-244` — schema-version compatibility rule; IR 0 `MeshIR` documentation (no per-IR "Current schema_version" line today, in contrast to IR 2 at line 250 and IR 5 at line 506).

### After this packet

- `model_loader.rs::load_3mf` reads `3D/3dmodel.model` AND `Metadata/model_settings.config` (when present) from the same `zip::ZipArchive` BEFORE the archive is dropped. The sidecar parser returns `HashMap<u32, ObjectSidecarInfo>`.
- `parse_3mf_model_xml` is extended to thread the sidecar info into the eventual `resolve_object` call, OR a parallel call path is added: `parse_3mf_model_xml_with_sidecar` returning richer per-item data. **Locked choice (Step 2):** parse the model XML first to its current shape, then post-process by walking each `Parsed3mfObject` and applying sidecar classification at the resolution step. This minimizes the diff to the existing XML state machine.
- `resolve_object` gains a fifth parameter `sidecar: &HashMap<u32, ObjectSidecarInfo>` and a sixth-and-final return value `Vec<ModifierVolume>` accumulated through the component recursion. Each non-`normal_part` part contributes (a) NO triangles to `merged_vertices/merged_indices`, (b) a fresh `ModifierVolume` with the world-space transformed mesh and a `config_delta` carrying typed metadata (`subtype`, `fuzzy_skin`, optionally `extruder`).
- `MeshIR.schema_version` bumps to `1.1.0`.
- A new host stage `negative_part_subtract` runs between prepass and region-mapping (Activation Q3 = Option 1) and mutates per-layer slice polygons.
- `execute_region_mapping` learns the `modifier_part` overlap-stamp loop.
- Either `paint_segmentation.rs` or a sibling helper emits synthetic `PaintRegionIR` for `support_enforcer` / `support_blocker` volumes; flow through Packet 51's overlay to stamp `RegionPlan.paint_overrides`.

## Neighboring Tests / Fixtures

- `crates/slicer-host/tests/threemf_transform_tdd.rs` — current 3MF transform tests (10 passing). Must remain green; this packet preserves the `<build>/<item>` composition path unchanged.
- `crates/slicer-host/tests/gcode_emit_tdd.rs` — G-code emission tests (27 passing). Must remain green.
- `crates/slicer-host/tests/benchy_painted_e2e_tdd.rs` — no-sidecar regression baseline. The `resources/benchy_painted.3mf` fixture has no `Metadata/model_settings.config` (verified by Step 0 sub-agent).
- `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs` — Packet 51 paint-config overlay regression. Contains the helper `count_perimeter_markers_in_z_band` this packet's E2E test reuses.
- `resources/benchy_4color.3mf` — primary fixture. Sidecar shape verified by the planning handout §9 quote.
- `resources/benchy_painted.3mf` — secondary fixture (no sidecar).

## Architecture Constraints

- Scaled integer units: `slicer_core::polygon_ops` operates on `Point2` in scaled integer units (1 unit = 100 nm; `docs/08_coordinate_system.md`). The modifier projection MUST convert to `Point2::from_mm(x, y)` before any `polygon_ops` call.
- Per-layer projection of a 3D mesh: project the mesh's vertices to the slicing plane's Z = `layer.z`. Reuse the existing slicer entry-point (delegate FACT dispatch at Step 7: "Which function in `slicer-core` slices an `IndexedTriangleSet` at a given Z?"). Likely candidate: existing slicing code already used for `paint_data` stroke projection; do not re-implement.
- Determinism: every `ModifierVolume` produced from a sidecar carries a deterministic `id: ModifierId` (UUID string per `docs/02_ir_schemas.md:37`). Derive deterministically from `(parent_objectid, part_id)` via a stable hash, NOT `uuid::Uuid::new_v4()` (which is non-deterministic). This matches the Packet 39 `stable-entity-ids` precedent.
- WIT boundary: clean — confirmed.
- IR versioning: additive minor bump (1.0.0 → 1.1.0). No new struct, no removed field, no changed enum variant. The producer contract widens (3MF path now populates `modifier_volumes`); the consumer contract is unchanged (consumers already had to handle non-empty `modifier_volumes`).

## Selected Approach (Locked Decisions)

| Decision | Locked choice | Justification |
|---|---|---|
| Sidecar parser placement | In-file helper inside `model_loader.rs`, OR a new sibling `model_loader_sidecar.rs` if `model_loader.rs` exceeds 800 lines post-addition. | Keeps the call site close; the sidecar is a 3MF concern only. |
| Sidecar return shape | `HashMap<u32 /* objectid */, ObjectSidecarInfo { parts: HashMap<u32 /* part_id */, PartSidecarInfo { subtype: PartSubtype, metadata: BTreeMap<String, String> }> }>`. `PartSubtype` is a fresh enum local to `model_loader.rs` (or its sibling); NOT an IR type. | The IR carries typed config keys (`config_delta.fields`), not subtype enums. Keeping `PartSubtype` host-local avoids an IR ripple. |
| Geometry routing | `modifier_part`, `negative_part`, `support_enforcer`, `support_blocker` → `ObjectMesh.modifier_volumes` (each as its own `ModifierVolume`). `normal_part` → merged into solid mesh (existing behavior). | One IR field already exists. Downstream consumers branch on `config_delta.fields[ConfigKey::from("subtype")]`. |
| Per-modifier IR fields | Reuse `ModifierVolume.config_delta` with typed keys (`subtype`, `fuzzy_skin`, optionally `extruder`, optionally `matrix` for telemetry). NO new IR fields. NO new IR struct. | Minimum-surface change. The producer/consumer contract is the existing `ConfigDelta`. |
| Schema bump | `MeshIR.schema_version`: 1.0.0 → 1.1.0 (additive minor). | Mirrors Packet 36 / Packet 51 precedent. Additive widening of producer contract. |
| `modifier_part` fuzzy region | **Region-mapping direct stamp** (user-selected Option 1). In `commit_region_mapping_builtin`, for each `(layer, region)`, run `slicer_core::polygon_ops::intersection` between `RegionPlan.polygon` and the modifier's per-layer projection. On non-empty overlap, stamp `RegionPlan.config["fuzzy_skin.apply-to-all"] = ConfigValue::Bool(true)`. | User-approved. Does not piggyback on PaintSegmentation, so the `paint_fuzzy_skin` triangle path and the modifier-overlap path are independent producers of fuzz regions; both stamp the same config key. Final fuzz region = union of the two. |
| `negative_part` subtract | **New host stage `apply_negative_part_subtract`** (Activation Q3 = Option 1). Runs between prepass and region-mapping. For each layer, computes `slicer_core::polygon_ops::difference(slice_polygons, projected_negative_per_layer)` and mutates the per-layer slice polygons in place. | Keeps `region_mapping.rs` focused on config overlays. Negative-part geometry is independently testable. |
| `support_enforcer` / `support_blocker` | **Paint-segmentation piggyback.** Project per layer; emit synthetic `PaintRegionIR` entries with `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`. Flow through Packet 51's `paint_overrides` overlay. | Reuses existing wiring proven by Packet 50b (TASK-180b). Zero new region-mapping code beyond what `modifier_part` requires. |
| WIT scope | Not touched. Step 0 gate confirmed host-only. | Avoids the DEV-043-class deviation. |
| Negative-case behavior | Malformed sidecar / unknown subtype → `log::warn!` + fallback to all-`normal_part`. Loader returns `Ok(MeshIR)`. | User-selected. Preserves backward compat for fixtures without sidecars. |
| Subtypes covered v1 | All five (`normal/modifier/negative/support_enforcer/support_blocker`). | User-selected. |
| Paint data on non-`normal_part` rows | Dropped at load time with `log::warn!`. | User-implicit (DEV-NN-B registered). |

## Rejected Alternatives

| Alternative | Reason rejected |
|---|---|
| New `ModifierVolumeIR` schema struct with full typed `fuzzy_skin: Option<FuzzySkinMode>` etc. | YAGNI. `ConfigDelta` already carries typed values via `ConfigValue` enum (`String`, `Bool`, `Float`, …). Adding a parallel typed enum doubles the maintenance surface. |
| PaintSegmentation piggyback for `modifier_part` fuzzy_skin | User selected Option 1 (region-mapping direct stamp). Recorded for the implementer's awareness only. |
| Inline `negative_part` subtract inside `commit_region_mapping_builtin` | Mixes geometry mutation with config overlay. Activation Q3 = Option 1 picked a separate stage. |
| Consume sidecar `<part>/<metadata key="matrix">` as geometry source | The 16-float row-major matrix in the sidecar duplicates the model XML's 12-float `ST_Matrix3D`. The model XML path is already exercised by `parse_3mf_transform`. The sidecar matrix is captured into `config_delta` as a sanity-check key only. Avoids two competing transform sources. |
| Hard error on malformed sidecar | User-selected fallback-with-warning. Hard error would break fixtures with hand-edited sidecars. |
| Add `WallFeatureFlags.fuzzy_skin` propagation through perimeter generation | Step-0 read of `modules/core-modules/fuzzy-skin/src/lib.rs:51-54, 80-81` confirms the module's `apply-to-all` config flag covers the entire region. No per-vertex flag path is needed for this packet's scope. |

## Code Change Surface (≤ 3 primary files per step)

Primary files this packet edits (the implementer's authoritative files-in-scope list):

1. `crates/slicer-host/src/model_loader.rs` — sidecar parser + `resolve_object` branching + schema bump. (Heaviest single-file edit; up to ~250 added lines. Implementer must watch the 800-line ceiling and split into `model_loader_sidecar.rs` if it crosses.)
2. `crates/slicer-host/src/region_mapping.rs` — modifier-overlap config stamp loop (`fuzzy_skin.apply-to-all`).
3. `crates/slicer-host/src/negative_part_subtract.rs` — NEW file; per-layer 2D subtract host stage.
4. `crates/slicer-host/src/paint_segmentation.rs` (or a sibling helper) — synthetic `PaintRegionIR` emission for `support_enforcer`/`blocker`.
5. `crates/slicer-host/src/pipeline.rs` — insert `apply_negative_part_subtract` call; thread `modifier_volumes` into `execute_region_mapping` and the support-paint emission helper.
6. Three new test files under `crates/slicer-host/tests/`.
7. Three documentation files (`docs/02_ir_schemas.md`, `docs/07_implementation_status.md`, `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md`).

Per the L-aggregate context cost, each implementation step picks at most three of these and a worker dispatch covers each step in isolation. See `implementation-plan.md` for the per-step file allocation.

## Read-only Context the Implementer Needs

| Path | Lines | Purpose |
|---|---|---|
| `crates/slicer-host/src/model_loader.rs` | 130-203, 285-360, 430-587 | `load_model`, parsed 3MF data shapes, `resolve_object`, `load_3mf`, `find_model_path`. |
| `crates/slicer-ir/src/slice_ir.rs` | 230-295 | `ConfigDelta`, `ModifierVolume`, `ObjectMesh`. |
| `crates/slicer-host/src/region_mapping.rs` | 1-260 | `execute_region_mapping` signature and Packet 51 overlay; the place this packet extends. |
| `crates/slicer-host/src/config_resolution.rs` | 80-220 | `paint_semantic_namespace_key`, `resolve_per_paint_semantic_configs`. Reused unchanged by support-paint piggyback. |
| `modules/core-modules/fuzzy-skin/src/lib.rs` | 1-120 | Module's `apply-to-all` branch; consumer of the region-stamped config. |
| `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs` | search for `count_perimeter_markers_in_z_band` | Z-band marker counter — reuse pattern. |
| `docs/02_ir_schemas.md` | 5, 62-244 | Versioning rule + IR 0 `MeshIR` section. |
| `docs/08_coordinate_system.md` | full | Scaled integer units. Small file; safe to read directly. |
| `resources/benchy_4color.3mf` | sidecar XML (≤ 60 lines after `unzip -p`) | Confirm sidecar shape matches the planning handout §9 quote. |

## Out-of-Bounds Files (must not be loaded directly)

- `crates/slicer-macros/src/lib.rs` (>2300 lines).
- `crates/slicer-sdk/` — all files.
- All `wit/**` and `crates/slicer-host/src/wit_host.rs` — unless Step 0 sub-agent dispatch finds a `ModifierVolume` mirror (it did not; gate is clean).
- `OrcaSlicerDocumented/**` — always delegate via Explore agent with the LOCATIONS return-format.
- `target/`, `Cargo.lock`, generated code.

## Data and Contract Notes

- `ConfigDelta.fields` is `HashMap<ConfigKey, ConfigValue>`. `ConfigKey` is the existing typed key wrapper used by `config_resolution.rs`. Use the same key-namespacing convention as the rest of the system (`subtype`, `fuzzy_skin`, `extruder`). Confirm at Step 1 via FACT dispatch: "What is `ConfigKey`'s constructor signature; can it accept arbitrary strings?".
- `ConfigValue` enum: `String(String)`, `Bool(bool)`, `Int(i64)`, `Float(f64)`, possibly more. Step 1 confirms exact variants via FACT dispatch.
- `RegionPlan.config` is a `ResolvedConfig`. The stamping path mutates the resolved config in place after Packet 51's overlay has already run. Order of operations within `commit_region_mapping_builtin`: (1) Packet 51 paint-semantic overlay, (2) this packet's modifier-overlap stamp. The latter wins only on the specific key it sets — additive.
- `MeshIR.schema_version`: change the `SemVer { 1, 0, 0 }` literal at `model_loader.rs:194-199` to `SemVer { 1, 1, 0 }`. No host-side enforcement update is required; the host validates compatibility at module load (per `docs/02_ir_schemas.md:5`), and an additive minor bump preserves backward compat.
- Deterministic `ModifierId`: derive from `format!("{}:{}-{}-{}", parent_object_uuid, part_id, subtype_str, hash_of_metadata)` or similar. The exact recipe is locked at Step 1 via FACT dispatch: "What deterministic id derivation does Packet 39 use for stable entity ids? Return a one-line example."

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|
| `resolve_object` recursion has subtle paint-data alignment logic (lines 488-527). Adding modifier accumulation may break paint alignment for partially-modifier objects. | Step 2's TDD adds two unit tests: (a) component-with-mixed-subtypes paint alignment preserved on `normal_part` rows; (b) paint dropped on non-`normal_part` rows. Run these BEFORE Step 4. |
| Per-layer modifier projection is O(layers × volumes × triangles). On a 200-layer benchy with a 12-triangle cube modifier, this is trivial. On future fixtures with high-triangle modifiers it could be a hotspot. | Out of scope for this packet. Log via DEV-NN-A if a perf concern arises. |
| Negative-part subtract changes per-layer slice polygons. Downstream stages (`paint_data` projection, fuzzy-skin region stamping) must see the post-subtract polygons. | Insert the stage BEFORE both paint segmentation and region-mapping in `pipeline.rs`. Step 7's TDD asserts a paint-on-negative-part fixture sees the reduced polygon area. |
| Support enforcer / blocker piggyback re-uses Packet 50b's paint-supports semantic. If Packet 50b's test surfaces drift, this packet's tests may regress. | Step 8 explicitly re-runs Packet 50b's regression test (`cargo test -p slicer-host --test benchy_painted_e2e_tdd painted_benchy_3mf_reaches_paint_segmentation`) as part of its exit criteria. |
| Sidecar `<part id>` and 3MF `<object id>` collisions: in Bambu's sidecar, `<part id="1">` refers to the `<object id="1">` from the model XML, but the parent `<object id="3">` in the sidecar is the wrapper. The handout calls out this mapping. | Step 1's TDD asserts the mapping explicitly: `<object id="X">` in sidecar maps to `<object id="X">` in model XML; `<part id="Y">` inside that sidecar object maps to `<object id="Y">` in model XML (the leaf component). Step 1 also adds a `LOCATIONS` dispatch to OrcaSlicer's `bbs_3mf.cpp` to confirm this mapping. |
| Aggregate L context cost. | Activation Q1 must be resolved by user before activation. Implementer dispatches each step to a fresh worker. |

## Open Questions Blocking Activation

See Activation Blockers Q1-Q4 in `packet.spec.md`. Of these, Q1 is the most material — the user must explicitly authorize the L-aggregate or direct a split.

## Locked Assumptions and Invariants

1. WIT scope is clean. If a worker discovers `ObjectMesh.modifier_volumes` or `ModifierVolume` in `wit/**` or any `wit_host.rs`, the packet halts and registers a DEV-043-style escalation. Step 0 of the implementation plan re-runs this gate as a FACT dispatch.
2. `ModifierVolume.config_delta` is the IR carrier for all per-part metadata. No new IR struct is introduced by this packet.
3. The `fuzzy-skin` module is unchanged. The region-stamped `apply-to-all` config key is the consumer.
4. The model XML `<component>` transform is the geometry source. Sidecar `<part>/<metadata key="matrix">` is telemetry only.
5. `MeshIR.schema_version` bumps additively (1.0.0 → 1.1.0). No new struct, no removed field, no changed variant.
6. `resources/benchy_painted.3mf` (no sidecar) slices byte-identical to pre-packet output.
7. The negative-part subtract runs BEFORE paint segmentation and region mapping.
8. Determinism: all derived `ModifierId` strings are stable across repeated runs with identical inputs.
9. Paint data on non-`normal_part` rows is dropped at load time with a structured warning. There is no consumer that needs it.
10. Unknown subtype values silently downgrade to `normal_part`. The loader does not fail.

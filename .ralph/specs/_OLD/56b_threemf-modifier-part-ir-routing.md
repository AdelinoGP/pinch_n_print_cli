---
status: implemented
packet: 56b_threemf-modifier-part-ir-routing
task_ids:
  - TASK-191
  - TASK-192a
---

# 56b_threemf-modifier-part-ir-routing

## Goal

Branch `crates/slicer-host/src/model_loader.rs::resolve_object` on the sidecar classification produced by Packet 56. Route every part whose `PartSubtype != NormalPart` into `ObjectMesh.modifier_volumes` instead of merging into the solid mesh. Translate the host-local `PartSidecarInfo` (raw string metadata + enum) into a typed `ModifierVolume { id, mesh, config_delta, priority, applies_to }` entry whose `config_delta.fields` carries:

- `ConfigKey::from("subtype") -> ConfigValue::String("modifier_part" | "negative_part" | "support_enforcer" | "support_blocker")`
- `ConfigKey::from("fuzzy_skin") -> ConfigValue::String(...)` (when sidecar metadata contains the key)
- `ConfigKey::from("extruder") -> ConfigValue::Int(...)` (when sidecar metadata contains the key; not consumed by any downstream packet yet)
- `ConfigKey::from("matrix") -> ConfigValue::String(...)` (telemetry only)

Drop `paint_data` carried on any non-`NormalPart` row with a single `log::warn!` per dropped part (DEV-052). Bump `MeshIR.schema_version` from `SemVer { 1, 0, 0 }` to `SemVer { 1, 1, 0 }` at `crates/slicer-host/src/model_loader.rs:194-199` (additive minor — producer contract widens to populate `modifier_volumes` from 3MF). Document the bump in `docs/02_ir_schemas.md`'s IR 0 section per the IR 2 / IR 5 precedent.

Then wire the `modifier_part` consumer: extend `crates/slicer-host/src/region_mapping.rs::execute_region_mapping` to accept per-object modifier volumes (read from `ExecutionPlan` or threaded through the call site). For each `(layer, region)`, project each `modifier_part` volume to the layer's Z plane and run `slicer_core::polygon_ops::intersection` against the region polygon. On non-empty overlap, stamp `RegionPlan.config["fuzzy_skin.apply_to_all"] = ConfigValue::Bool(true)`. Preserve the no-modifier fast path (bit-identical output when `modifier_volumes.is_empty()`).

Confirm `apply_to_all` is declared in `modules/core-modules/fuzzy-skin/fuzzy-skin.toml`'s `[config.schema]` block. If absent, register it additively.

This packet closes the `modifier_part` half of the original packet's scope. The `negative_part` host stage and `support_enforcer`/`support_blocker` paint-segmentation piggyback are owned by Packet 56c.

**Activation Q3 (negative-part subtract stage placement)** from the original packet is NOT a blocker here — it is deferred to Packet 56c. **Activation Q4 (fuzzy-skin manifest schema)** is resolved by Step 3 of this packet (the manifest gate).

## Problem Statement

Packet 56 (`56_threemf-sidecar-parser`) adds a sidecar parser that classifies each 3MF `<part>` by `subtype=` and surfaces typed per-part metadata as `HashMap<u32, ObjectSidecarInfo>`. The parser is plumbed into `load_3mf` and threaded through `parse_3mf_model_xml` → `resolve_object`, but `resolve_object` does not yet branch on the classification — its parameter is `_sidecar` (underscore-prefixed, unused).

This packet (56b) is where `resolve_object` actually branches. It routes every part whose `PartSubtype != NormalPart` into `ObjectMesh.modifier_volumes` instead of merging triangles into the solid mesh. It bumps `MeshIR.schema_version` 1.0.0 → 1.1.0 (additive minor) to reflect the producer contract widening. It drops `paint_data` carried on non-`NormalPart` rows with a structured warning (DEV-052). It confirms the `fuzzy-skin` module manifest declares `apply_to_all` in its `[config.schema]` block. And it wires the `modifier_part` consumer: per-layer 2D overlap testing in `execute_region_mapping`, stamping `RegionPlan.config["fuzzy_skin.apply_to_all"] = ConfigValue::Bool(true)` on overlapping regions.

This packet does NOT introduce the `negative_part` host stage and does NOT wire `support_enforcer`/`support_blocker` paint-segmentation piggyback — those are owned by Packet 56c. The IR routing in this packet does, however, populate `ObjectMesh.modifier_volumes` for ALL four non-`NormalPart` subtypes (because the routing is uniform — each non-`NormalPart` part becomes a `ModifierVolume` regardless of whether its downstream consumer is wired). The consumer wiring for `negative_part` and `support_*` lands in Packet 56c.

The original `56_threemf-modifier-and-subtype-sidecar-ingestion` packet identified Bug B (fuzzy applied globally because the modifier marker never reached the IR) as the visible symptom. This packet closes Bug B for `modifier_part` specifically. The closing E2E test loads `resources/benchy_4color.3mf` and asserts:

- The merged solid mesh has 225,240 triangles (down from 225,252) — the 12 cube triangles are excluded.
- `modifier_volumes.len() == 1` carrying typed `subtype = "modifier_part"` + `fuzzy_skin = "external"`.
- The world-space AABB centroid of the modifier matches the predicted composition position within ±0.01 mm.
- Region overlap stamps appear on regions intersecting the cube's Z-band but NOT above it.
- Fuzzy G-code markers appear in the cube's projection band + on `paint_fuzzy_skin` triangles, but NOT on other regions.

WIT scope is **clean** — confirmed at the original packet's Step 0. This packet re-confirms via its own Step 0 because the IR producer contract is widening here (`ObjectMesh.modifier_volumes` becomes contractually populated from 3MF for the first time).

One deviation is registered and closed by this packet:

- **DEV-052** — Paint data on non-`NormalPart` rows (modifier, negative, support enforcer, support blocker) is dropped at load time with `log::warn!`. No consumer needs paint on a modifier; preserving it would waste IR memory and risk double-counting.

This packet does not modify Packet 56's directory. Packet 56's `status: implemented` is a precondition (verified at Step 0).

## Architecture Constraints

- Scaled integer units: `slicer_core::polygon_ops` operates in `Point2` scaled integer units (1 unit = 100 nm; `docs/08_coordinate_system.md`). Modifier projections MUST convert to `Point2::from_mm(x, y)` before any `polygon_ops` call.
- Per-layer projection: use the existing slicer entry-point that already projects paint-data strokes to layer Z. Step 5 FACT dispatch identifies the function (likely candidate: a function in `slicer-core` taking `(triangle_set, z) -> Vec<Polygon>`).
- Determinism: `ModifierId` derivation MUST be stable across runs. Hash recipe locked at Step 2 (FACT dispatch returns Packet 39's `stable-entity-ids` precedent).
- WIT boundary: clean — re-confirmed at Step 0.
- IR versioning: additive minor bump (1.0.0 → 1.1.0). No new struct, no removed field, no changed enum variant. The producer contract widens; the consumer contract is unchanged.
- Loop ordering inside `execute_region_mapping`: Packet 51's paint-semantic overlay runs first; this packet's modifier-overlap stamp runs second. The latter only sets the `fuzzy_skin.apply_to_all` key, which Packet 51 does not set — no clobbering.
- ActiveRegion polygon constraint: region polygons do not exist at RegionMapping time (ActiveRegion carries config only — confirmed at region_mapping.rs:225). This is why modifier overlap detection must run during the paint annotation step (post-slicing), where SlicedRegion.polygons and per-contour paint annotation are available.

## Data and Contract Notes

- `ConfigDelta.fields` is `HashMap<ConfigKey, ConfigValue>` (per `crates/slicer-ir/src/slice_ir.rs:230-235`).
- `ConfigKey::from(&str)` — Step 2 FACT dispatch confirms constructor signature and whether it accepts arbitrary strings.
- `ConfigValue` enum variants — Step 2 FACT dispatch returns the exact variant list.
- `RegionPlan.config` is a `ResolvedConfig` (per Packet 51's design). This packet mutates it in place after Packet 51's overlay has run. Ordering: (1) Packet 51 paint-semantic overlay, (2) this packet's modifier-overlap stamp. Additive on the `fuzzy_skin.apply_to_all` key only.
- `MeshIR.schema_version`: change the `SemVer { 1, 0, 0 }` literal at `model_loader.rs:194-199` to `SemVer { 1, 1, 0 }`. No host-side enforcement update required; additive minor preserves backward compat.
- Deterministic `ModifierId`: Step 2 FACT dispatch returns the one-line recipe used by Packet 39's `stable-entity-ids`. Apply the same recipe with input `format!("{parent_uuid}:{part_id}:{subtype_str}")` or whatever the recipe dictates.

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|
| `resolve_object` recursion has subtle paint-data alignment logic (lines 488-527). Adding modifier accumulation may break paint alignment for partially-modifier objects. | Step 2 TDD covers two cases: (a) component-with-mixed-subtypes paint alignment preserved on `NormalPart` rows; (b) paint dropped on non-`NormalPart` rows. Both must be GREEN before Step 5. |
| Per-layer modifier projection is O(layers × volumes × triangles). On a 200-layer benchy with a 12-triangle cube modifier, trivial. On future high-triangle modifiers it could be a hotspot. | Out of scope for this packet. Log a TODO in `crates/slicer-host/src/region_mapping.rs` near the overlap loop. |
| Support enforcer / blocker subtypes ALSO route into `modifier_volumes` here (their geometry storage is identical to `modifier_part`'s), but their downstream consumer is not wired until Packet 56c. Until Packet 56c lands, fixtures with `support_*` parts will have populated `modifier_volumes` entries with no downstream effect. | This is intentional. The IR plumbing is uniform; consumer wiring is staggered across packets. No tests in this packet exercise `support_*` end-to-end. Packet 56c's tests cover the consumer side. |
| Existing call sites of `execute_region_mapping` break when the signature widens. | Step 5 FACT dispatch enumerates every call site. Update each one to pass `modifier_volumes` (or an empty slice for unit tests). |
| `apply_to_all` already present in fuzzy-skin manifest but with a different name (`apply_to_all`, `applyToAll`, etc.). | Step 3 FACT dispatch returns the verbatim key. If a different name is in use, this packet stamps the verbatim key Packet 51 / fuzzy-skin module actually reads. The AC's `"fuzzy_skin.apply_to_all"` literal is updated if the dispatch returns a different key. |
| `ModifierId` collisions if two parts share `(parent_object_uuid, part_id, subtype)`. | Bambu's sidecar requires `<part id>` uniqueness inside its parent `<object>`. Step 2 TDD asserts the assumption against a synthetic two-part fixture. |
| The original packet's Activation Q3 (negative-part subtract stage placement) is "answered" only by Packet 56c. This packet's negative-part routing populates `modifier_volumes` but does not subtract. If a fixture with `negative_part` is sliced AFTER this packet lands but BEFORE Packet 56c, the negative volume sits in IR with no effect. | Acceptable. Fixtures with `negative_part` are exercised only by Packet 56c's synthetic tests. No real fixture in `resources/` carries `negative_part`. |

## Locked Assumptions and Invariants

1. WIT scope is clean (re-confirmed at Step 0).
2. `ModifierVolume.config_delta` is the IR carrier for all per-part metadata. No new IR struct introduced.
3. The `fuzzy-skin` module is unchanged. The region-stamped `apply_to_all` config key is the consumer.
4. The model XML `<component>` transform is the geometry source. Sidecar `<part>/<metadata key="matrix">` is telemetry only.
5. `MeshIR.schema_version` bumps additively (1.0.0 → 1.1.0).
6. `resources/benchy_painted.3mf` (no sidecar) slices byte-identical to pre-Packet-56b output.
7. Determinism: `ModifierId` strings stable across repeated runs with identical inputs.
8. Paint data on non-`NormalPart` rows is dropped at load time with a structured warning. There is no consumer that needs it.
9. `support_enforcer` and `support_blocker` parts ALSO populate `modifier_volumes` here, but their downstream consumer is not wired until Packet 56c. This is intentional.
10. `negative_part` parts ALSO populate `modifier_volumes` here, but the host stage that consumes them (`apply_negative_part_subtract`) is not introduced until Packet 56c. This is intentional.

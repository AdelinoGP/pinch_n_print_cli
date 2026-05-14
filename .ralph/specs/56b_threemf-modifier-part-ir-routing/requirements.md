# Requirements: 56b_threemf-modifier-part-ir-routing

## Problem Statement

Packet 56 (`56_threemf-sidecar-parser`) adds a sidecar parser that classifies each 3MF `<part>` by `subtype=` and surfaces typed per-part metadata as `HashMap<u32, ObjectSidecarInfo>`. The parser is plumbed into `load_3mf` and threaded through `parse_3mf_model_xml` → `resolve_object`, but `resolve_object` does not yet branch on the classification — its parameter is `_sidecar` (underscore-prefixed, unused).

This packet (56b) is where `resolve_object` actually branches. It routes every part whose `PartSubtype != NormalPart` into `ObjectMesh.modifier_volumes` instead of merging triangles into the solid mesh. It bumps `MeshIR.schema_version` 1.0.0 → 1.1.0 (additive minor) to reflect the producer contract widening. It drops `paint_data` carried on non-`NormalPart` rows with a structured warning (DEV-048). It confirms the `fuzzy-skin` module manifest declares `apply-to-all` in its `[config.schema]` block. And it wires the `modifier_part` consumer: per-layer 2D overlap testing in `execute_region_mapping`, stamping `RegionPlan.config["fuzzy_skin.apply-to-all"] = ConfigValue::Bool(true)` on overlapping regions.

This packet does NOT introduce the `negative_part` host stage and does NOT wire `support_enforcer`/`support_blocker` paint-segmentation piggyback — those are owned by Packet 56c. The IR routing in this packet does, however, populate `ObjectMesh.modifier_volumes` for ALL four non-`NormalPart` subtypes (because the routing is uniform — each non-`NormalPart` part becomes a `ModifierVolume` regardless of whether its downstream consumer is wired). The consumer wiring for `negative_part` and `support_*` lands in Packet 56c.

The original `56_threemf-modifier-and-subtype-sidecar-ingestion` packet identified Bug B (fuzzy applied globally because the modifier marker never reached the IR) as the visible symptom. This packet closes Bug B for `modifier_part` specifically. The closing E2E test loads `resources/benchy_4color.3mf` and asserts:

- The merged solid mesh has 225,240 triangles (down from 225,252) — the 12 cube triangles are excluded.
- `modifier_volumes.len() == 1` carrying typed `subtype = "modifier_part"` + `fuzzy_skin = "external"`.
- The world-space AABB centroid of the modifier matches the predicted composition position within ±0.01 mm.
- Region overlap stamps appear on regions intersecting the cube's Z-band but NOT above it.
- Fuzzy G-code markers appear in the cube's projection band + on `paint_fuzzy_skin` triangles, but NOT on other regions.

WIT scope is **clean** — confirmed at the original packet's Step 0. This packet re-confirms via its own Step 0 because the IR producer contract is widening here (`ObjectMesh.modifier_volumes` becomes contractually populated from 3MF for the first time).

One deviation is registered and closed by this packet:

- **DEV-048** — Paint data on non-`NormalPart` rows (modifier, negative, support enforcer, support blocker) is dropped at load time with `log::warn!`. No consumer needs paint on a modifier; preserving it would waste IR memory and risk double-counting.

This packet does not modify Packet 56's directory. Packet 56's `status: implemented` is a precondition (verified at Step 0).

## Task IDs (registered by this packet)

- **TASK-191** — Branch `resolve_object` to route `modifier_part`, `negative_part`, `support_enforcer`, and `support_blocker` geometry into `ObjectMesh.modifier_volumes` instead of merging into the solid mesh. Drop paint data carried on non-`NormalPart` rows. Bump `MeshIR.schema_version` 1.0.0 → 1.1.0 additively.
- **TASK-192a** — Wire the `modifier_part` downstream consumer: region-mapping direct stamp (user-selected Option 1 in original packet) — `slicer_core::polygon_ops::intersection` between `RegionPlan` polygons and per-layer modifier projection, stamping `RegionPlan.config["fuzzy_skin.apply-to-all"] = ConfigValue::Bool(true)` on overlapping regions only. Includes the fuzzy-skin manifest schema confirmation gate. (Packet 56c registers TASK-192b for the `negative_part` stage and TASK-192c for the support piggyback.)

(TASK-193 is reserved for Packet 56c, which performs the synthetic-fixture E2E for `negative_part` and `support_*`.)

## In Scope

- Files-in-scope (write):
  - `crates/slicer-host/src/model_loader.rs` — `resolve_object` branching, `ModifierVolume` construction with typed `config_delta`, paint-data drop on non-`NormalPart`, `SemVer { 1, 0, 0 }` → `SemVer { 1, 1, 0 }` bump.
  - `crates/slicer-host/src/region_mapping.rs` — region-overlap config stamp loop for `modifier_part`.
  - `crates/slicer-host/src/pipeline.rs` — thread per-object `modifier_volumes` into the `execute_region_mapping` call.
  - `modules/core-modules/fuzzy-skin/manifest.toml` — read-only check; additive edit only if `apply-to-all` is missing from `[config.schema]`.
  - `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` — NEW; fixture-backed E2E suite.
  - `crates/slicer-host/tests/threemf_paint_drop_on_modifier_tdd.rs` — NEW (or fold into `threemf_sidecar_classification_tdd.rs` at Step 2's discretion); DEV-048 negative case.
  - `docs/02_ir_schemas.md` — schema_version header bump annotation under IR 0.
  - `docs/07_implementation_status.md` — append TASK-191 and TASK-192a rows.
  - `docs/DEVIATION_LOG.md` — register DEV-048 as `Closed — Packet 56b, 2026-MM-DD`.
  - `docs/14_deviation_audit_history.md` — chronology entry.

## Out of Scope

- `apply_negative_part_subtract` host stage — owned by Packet 56c.
- Support enforcer/blocker paint-segmentation piggyback — owned by Packet 56c.
- Any change to `crates/slicer-ir/`. `ModifierVolume`, `ConfigDelta`, `ObjectMesh.modifier_volumes` already exist; only producer contract widens.
- `wit/**`, `crates/slicer-host/src/wit_host.rs`, `dispatch.rs` — confirmed clean by Packet 56's predecessor Step 0 + re-confirmed here at Step 0.
- `crates/slicer-macros/`, `crates/slicer-sdk/`.
- `modules/core-modules/fuzzy-skin/src/lib.rs` — read-only. The region-stamped `apply-to-all` config key is the consumer; no module code change.
- Bambu Studio printer-config (`Metadata/project_settings.config`), STL+sidecar JSON ingestion, sidecar `<part>/<metadata key="matrix">` as geometry source, `<assemble>`/`<plate>` sections, `extruder` per-modifier consumer.

## Authoritative Docs

- `docs/02_ir_schemas.md` — IR 0 `MeshIR` (lines 62-244), versioning rule at line 5, `ConfigDelta`/`ModifierVolume` shape (lines 192-211). Read directly.
- `docs/01_system_architecture.md` :107-114 — RegionMapping responsibility. Delegate SUMMARY.
- `docs/03_wit_and_manifest.md` — module manifest TOML `[config.schema]` block. Delegate SUMMARY.
- `docs/08_coordinate_system.md` — scaled integer units. Read directly.
- `docs/07_implementation_status.md` — append TASK-191, TASK-192a.
- `docs/DEVIATION_LOG.md` — register DEV-048.
- `docs/14_deviation_audit_history.md` — chronology entry.

## OrcaSlicer Reference Obligations

Host implementation MUST be project-internal Rust.

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — `<part subtype>`-branching production function. Delegate at Step 2 with LOCATIONS contract; ≤ 5 entries.
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (or sibling) — fuzzy-skin overlap production routine. Delegate at Step 5 with LOCATIONS contract; ≤ 5 entries.

## Acceptance Summary (measurable outcomes)

- `resolve_object` branches on the `_sidecar` parameter (renamed to `sidecar`). Non-`NormalPart` parts contribute `ModifierVolume` entries; their triangles are NOT in `merged_vertices`/`merged_indices`.
- `MeshIR.objects[0].mesh.indices.len() / 3 == 225_240` for `resources/benchy_4color.3mf`.
- `MeshIR.objects[0].modifier_volumes.len() == 1` for the same fixture, carrying typed `subtype = "modifier_part"` and `fuzzy_skin = "external"` in `config_delta.fields`.
- `MeshIR.schema_version == SemVer { major: 1, minor: 1, patch: 0 }`.
- Fuzzy-skin G-code markers appear inside the cube's XY+Z projection band AND on `paint_fuzzy_skin`-painted facets; markers do NOT appear on other regions of the body.
- `resources/benchy_painted.3mf` (no sidecar) slices byte-identical to pre-Packet-56 output (Packet 50 / 51 regression tests stay green).
- Paint data on `modifier_part` row → dropped + `log::warn!` naming part id.
- `cargo clippy --workspace -- -D warnings` clean.

## Negative Cases (explicit)

- Paint data on a `modifier_part` row → dropped at load time + structured warning (DEV-048).
- Empty modifier volume (zero triangles after sidecar parsing) → `modifier_volumes[0].mesh.indices.is_empty()`; no region stamps emitted.
- Region above the modifier's Z-extent → no overlap, no `fuzzy_skin.apply-to-all` stamp.

## Cross-Packet Dependencies / Unblockers

- Depends on **Packet 56** (`56_threemf-sidecar-parser`) being `status: implemented`. Without Packet 56's `parse_3mf_sidecar` and `_sidecar` parameter on `resolve_object`, this packet cannot start.
- Depends on `slicer_core::polygon_ops::intersection` (Clipper2-backed). Public export.
- Depends on Packet 51's `RegionPlan.config` overlay path remaining intact (verified by regression).
- Unblocks Packet 56c (which consumes `ObjectMesh.modifier_volumes` populated by this packet for `negative_part` subtract and support piggyback).

## Verification Commands

```powershell
cargo check --workspace
cargo clippy -p slicer-host --tests -- -D warnings
cargo clippy --workspace -- -D warnings
cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd
cargo test -p slicer-host --test threemf_paint_drop_on_modifier_tdd
cargo test -p slicer-host --test threemf_sidecar_classification_tdd
cargo test -p slicer-host --test threemf_transform_tdd
cargo test -p slicer-host --test gcode_emit_tdd
cargo test -p slicer-host --test benchy_painted_e2e_tdd
cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd
cargo test -p slicer-host --test core_module_ir_access_contract_tdd
```

Per CLAUDE.md Test Discipline: `cargo test --workspace` is NOT a per-criterion or per-step verification command. This packet's closure verification does not include a workspace-wide run; the targeted commands above cover the producer + `modifier_part` consumer surface.

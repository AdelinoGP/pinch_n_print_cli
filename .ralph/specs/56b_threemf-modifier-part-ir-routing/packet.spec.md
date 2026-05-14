---
status: draft
packet: 56b_threemf-modifier-part-ir-routing
task_ids:
  - TASK-191
  - TASK-192a
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
depends_on:
  - 56_threemf-sidecar-parser (must be status: implemented)
unblocks:
  - 56c_threemf-negative-and-support-subtype-routing
---

# Packet Contract: 56b_threemf-modifier-part-ir-routing

> This packet is the second of a three-way split of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` packet. Packet 56 owns the sidecar parser; this packet (56b) owns `resolve_object` branching, `MeshIR.schema_version` 1.0.0 → 1.1.0 bump, fuzzy-skin manifest gate, and `modifier_part` region-mapping overlap stamp. Packet 56c owns the `negative_part` host stage and `support_enforcer`/`support_blocker` paint-segmentation piggyback. Aggregate cost is **M**.

## Goal

Branch `crates/slicer-host/src/model_loader.rs::resolve_object` on the sidecar classification produced by Packet 56. Route every part whose `PartSubtype != NormalPart` into `ObjectMesh.modifier_volumes` instead of merging into the solid mesh. Translate the host-local `PartSidecarInfo` (raw string metadata + enum) into a typed `ModifierVolume { id, mesh, config_delta, priority, applies_to }` entry whose `config_delta.fields` carries:

- `ConfigKey::from("subtype") -> ConfigValue::String("modifier_part" | "negative_part" | "support_enforcer" | "support_blocker")`
- `ConfigKey::from("fuzzy_skin") -> ConfigValue::String(...)` (when sidecar metadata contains the key)
- `ConfigKey::from("extruder") -> ConfigValue::Int(...)` (when sidecar metadata contains the key; not consumed by any downstream packet yet)
- `ConfigKey::from("matrix") -> ConfigValue::String(...)` (telemetry only)

Drop `paint_data` carried on any non-`NormalPart` row with a single `log::warn!` per dropped part (DEV-048). Bump `MeshIR.schema_version` from `SemVer { 1, 0, 0 }` to `SemVer { 1, 1, 0 }` at `crates/slicer-host/src/model_loader.rs:194-199` (additive minor — producer contract widens to populate `modifier_volumes` from 3MF). Document the bump in `docs/02_ir_schemas.md`'s IR 0 section per the IR 2 / IR 5 precedent.

Then wire the `modifier_part` consumer: extend `crates/slicer-host/src/region_mapping.rs::execute_region_mapping` to accept per-object modifier volumes (read from `ExecutionPlan` or threaded through the call site). For each `(layer, region)`, project each `modifier_part` volume to the layer's Z plane and run `slicer_core::polygon_ops::intersection` against the region polygon. On non-empty overlap, stamp `RegionPlan.config["fuzzy_skin.apply-to-all"] = ConfigValue::Bool(true)`. Preserve the no-modifier fast path (bit-identical output when `modifier_volumes.is_empty()`).

Confirm `apply-to-all` is declared in `modules/core-modules/fuzzy-skin/manifest.toml`'s `[config.schema]` block. If absent, register it additively.

This packet closes the `modifier_part` half of the original packet's scope. The `negative_part` host stage and `support_enforcer`/`support_blocker` paint-segmentation piggyback are owned by Packet 56c.

**Activation Q3 (negative-part subtract stage placement)** from the original packet is NOT a blocker here — it is deferred to Packet 56c. **Activation Q4 (fuzzy-skin manifest schema)** is resolved by Step 3 of this packet (the manifest gate).

## Scope Boundaries

- In scope:
  - `crates/slicer-host/src/model_loader.rs` — branch `resolve_object` on `_sidecar` parameter (introduced unused by Packet 56). Non-`NormalPart` parts contribute a `ModifierVolume` to a new return accumulator instead of merging triangles into `merged_vertices/merged_indices`. Drop `paint_data` on non-`NormalPart` rows with `log::warn!`. Bump `SemVer { 1, 0, 0 }` → `SemVer { 1, 1, 0 }` at lines 194-199.
  - `crates/slicer-host/src/region_mapping.rs` — extend `execute_region_mapping` (lines 200-258 onward) to accept `&[ModifierVolume]` for each object. Project each `modifier_part` per layer (reuse the existing slicer entry-point identified by Step 5's FACT dispatch). Run `slicer_core::polygon_ops::intersection`; on non-empty overlap stamp `RegionPlan.config["fuzzy_skin.apply-to-all"] = ConfigValue::Bool(true)`.
  - `crates/slicer-host/src/pipeline.rs` — thread per-object `modifier_volumes` into the `execute_region_mapping` call. No new pipeline stages (those belong to Packet 56c).
  - `modules/core-modules/fuzzy-skin/manifest.toml` — confirm `apply-to-all` is declared in `[config.schema]`. If absent, register it (additive; no SemVer ripple).
  - `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` — NEW; fixture-backed E2E covering triangle counts, `modifier_volumes` IR shape, world-space AABB centroid, region overlap Z-band, fuzzy G-code restriction.
  - `crates/slicer-host/tests/threemf_paint_drop_on_modifier_tdd.rs` — NEW; standalone test covering DEV-048 (paint dropped on non-`NormalPart` rows with structured warning). May be folded into the existing `threemf_sidecar_classification_tdd.rs` if scope keeps tight; Step 2 picks based on file size.
  - `docs/02_ir_schemas.md` — IR 0 `MeshIR` schema_version header bump to 1.1.0 with annotation `**Current schema_version: 1.1.0** (Bumped to 1.1.0 by packet 56b — populated `modifier_volumes` from `Metadata/model_settings.config`.)` modelled on the IR 2 / IR 5 precedents.
  - `docs/07_implementation_status.md` — append TASK-191 and TASK-192a rows naming this packet.
  - `docs/DEVIATION_LOG.md` — register DEV-048 as `Closed — Packet 56b, 2026-MM-DD`.
  - `docs/14_deviation_audit_history.md` — chronology entry for DEV-048.

- Out of scope:
  - `apply_negative_part_subtract` host stage. → Packet 56c.
  - `support_enforcer` / `support_blocker` paint-segmentation piggyback. → Packet 56c.
  - Any change to `crates/slicer-ir/src/slice_ir.rs` — `ModifierVolume`, `ConfigDelta`, and `ObjectMesh.modifier_volumes` already exist at the IR layer (no struct change; only producer contract widens).
  - Any change to `wit/**`, `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/dispatch.rs`. Confirmed clean by Packet 56's predecessor Step 0; this packet re-confirms via Step 0 because the IR producer contract is widening here.
  - Any change to `crates/slicer-macros/src/lib.rs` (>2300 lines; explicit ban).
  - Any change to `crates/slicer-sdk/` — no SDK trait, `ConfigView`, or builder change.
  - Any change to `modules/core-modules/fuzzy-skin/src/lib.rs`. The region-stamped `apply-to-all` config key is sufficient per the module's existing branching at lines 80-81.
  - Consuming `<part>/<metadata key="matrix">` as a geometry source. Captured into `config_delta` as telemetry only.
  - The `extruder="N"` per-modifier override consumer (config_delta carries the value; no consumer wires it yet — future packet).

## Prerequisites and Blockers

- Depends on:
  - **Packet 56 (`56_threemf-sidecar-parser`) must be status: implemented.** This packet consumes Packet 56's `parse_3mf_sidecar` output and `_sidecar` parameter on `resolve_object`.
  - `slicer_core::polygon_ops::intersection` (Clipper2-backed). Public export.
  - Packet 51's `RegionPlan.config` overlay path remains in place; this packet stamps additional keys without replacing Packet 51's behavior.
- Unblocks:
  - Packet 56c (consumes `ObjectMesh.modifier_volumes` populated by this packet for `negative_part` and `support_*` routing).
- Activation blockers (must be resolved before flipping `status: draft` → `active`):
  - **Q1 (Packet 56 status).** Confirm `56_threemf-sidecar-parser` is `status: implemented`. If not, this packet cannot activate. Verify by grep on `.ralph/specs/56_threemf-sidecar-parser/packet.spec.md`.
  - **Q2 (deviation numbering).** Confirm DEV-048 is the next free deviation slot (verify against `docs/DEVIATION_LOG.md` at packet-open time).

## Acceptance Criteria

- **Given** `resources/benchy_4color.3mf` is loaded after this packet lands, **when** `slicer_host::model_loader::load_model("resources/benchy_4color.3mf")` is invoked, **then** the returned `MeshIR.objects[0].mesh.indices.len() / 3 == 225_240` (exactly; the 12 cube triangles are NOT in the solid mesh) AND `MeshIR.schema_version == SemVer { major: 1, minor: 1, patch: 0 }`. | `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_part_excluded_from_solid_mesh -- --exact --nocapture`
- **Given** the same fixture load, **when** `MeshIR.objects[0].modifier_volumes` is inspected, **then** `len() == 1`, `config_delta.fields.get(&ConfigKey::from("fuzzy_skin")) == Some(&ConfigValue::String("external".into()))`, AND `config_delta.fields.get(&ConfigKey::from("subtype")) == Some(&ConfigValue::String("modifier_part".into()))`. | `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_volume_carries_typed_metadata -- --exact --nocapture`
- **Given** the build/item transform `1 0 0 0 1 0 0 0 1 120.164588 105 35.2312426` and the `<component objectid="2">` row-major transform from `3dmodel.model`, **when** the modifier volume's world-space AABB is computed, **then** its centroid in X/Y/Z lies within ±0.01 mm of the cube's predicted projected position (the expected centroid is computed in the test from the model XML transform composition; sidecar `matrix` is consulted only as a sanity-check log line, not as the geometry source). | `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_world_aabb_matches_composition -- --exact --nocapture`
- **Given** `resources/benchy_painted.3mf` (no sidecar) is loaded after this packet lands, **when** the loader runs, **then** `MeshIR.objects[0].modifier_volumes.is_empty() == true` AND the slice output is byte-identical to the pre-Packet-56 G-code for the same config (no regression on the no-sidecar path). | `cargo test -p slicer-host --test benchy_painted_e2e_tdd painted_benchy_3mf_reaches_paint_segmentation -- --exact --nocapture && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd paint_config_override_visibly_differs_gcode -- --exact --nocapture`
- **Given** the modifier-vs-region overlap test runs in `execute_region_mapping`, **when** a region polygon at layer Z = (cube Z-min + 0.5 mm) intersects the projected modifier volume, **then** that region's `RegionPlan.config.fields.get(&ConfigKey::from("fuzzy_skin.apply-to-all")) == Some(&ConfigValue::Bool(true))` AND a region at layer Z = (cube Z-max + 1.0 mm) does NOT carry that key (i.e., overlap is geometric, not whole-object). | `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd region_overlap_stamps_only_in_cube_zband -- --exact --nocapture`
- **Given** the full sliced G-code from `resources/benchy_4color.3mf`, **when** fuzzy-skin marker lines are counted in a Z-band intersecting the cube vs a Z-band well above the cube (using the existing helper `count_perimeter_markers_in_z_band` in `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs`), **then** the intersecting band has > 5 fuzzy markers AND the above-cube band has 0 fuzzy markers on regions that are NOT in the `paint_fuzzy_skin` triangle set. | `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd fuzzy_region_restricted_to_cube_and_painted_facets -- --exact --nocapture`
- **Given** a synthetic 3MF archive where part 2 is classified `modifier_part` AND carries `paint_color="4"` triangle attributes in `3dmodel.model`, **when** the loader runs, **then** `MeshIR.objects[0].paint_data` does NOT contain any `PaintLayer` entries sourced from part 2 (DEV-048: paint dropped on non-`NormalPart`), AND a `log::warn!` is emitted with target containing the substring `"paint data on non-normal part dropped"` and naming part id 2. | `cargo test -p slicer-host --test threemf_paint_drop_on_modifier_tdd paint_on_modifier_part_dropped_with_warning -- --exact --nocapture`
- **Given** `MeshIR.schema_version` bumps additively to 1.1.0, **when** Step 2 lands, **then** `crates/slicer-host/src/model_loader.rs` constructs `SemVer { major: 1, minor: 1, patch: 0 }` AND `docs/02_ir_schemas.md`'s IR 0 section carries a header line containing `Current schema_version: 1.1.0` AND `packet 56b`. | `rg -q 'SemVer\s*\{[^}]*major:\s*1[^}]*minor:\s*1[^}]*patch:\s*0' crates/slicer-host/src/model_loader.rs && rg -q 'schema_version: 1\.1\.0.*packet 56b' docs/02_ir_schemas.md`
- **Given** TASK-191 and TASK-192a are registered by this packet, **when** Step 7 runs, **then** `docs/07_implementation_status.md` contains rows matching `[x] TASK-191` AND `[x] TASK-192a` AND each row names this packet (`56b_threemf-modifier-part-ir-routing`). | `rg -q '\[x\] TASK-191.*56b_threemf-modifier-part-ir-routing' docs/07_implementation_status.md && rg -q '\[x\] TASK-192a.*56b_threemf-modifier-part-ir-routing' docs/07_implementation_status.md`
- **Given** DEV-048 is registered and closed by this packet, **when** Step 7 runs, **then** `docs/DEVIATION_LOG.md` contains exactly one row whose ID column matches `DEV-048` AND whose status column reads `Closed — Packet 56b, 2026-MM-DD`. | `rg -c '^\| DEV-048.*Closed.*Packet 56b' docs/DEVIATION_LOG.md` (expected: 1)
- **Given** the existing regression-defense surfaces must stay GREEN, **when** Step 6 runs, **then** `cargo test -p slicer-host --test threemf_transform_tdd` reports all-pass AND `cargo test -p slicer-host --test gcode_emit_tdd` reports all-pass AND `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` reports all-pass. | `cargo test -p slicer-host --test threemf_transform_tdd && cargo test -p slicer-host --test gcode_emit_tdd && cargo test -p slicer-host --test threemf_sidecar_classification_tdd`
- **Given** clippy is the lint gate, **when** Step 6 runs, **then** `cargo clippy -p slicer-host --tests -- -D warnings` is green AND `cargo clippy --workspace -- -D warnings` is green. | `cargo clippy -p slicer-host --tests -- -D warnings && cargo clippy --workspace -- -D warnings`

## Negative Test Cases

- Paint data on a `modifier_part` row → dropped at load time + structured warning (AC-7 above; DEV-048).
- Modifier volume Z-extent above the region's Z plane → no overlap, no stamp (AC-5 above; verifies geometric — not whole-object — overlap).
- Modifier volume present but empty triangle set (degenerate fixture) → `slicer_core::polygon_ops::intersection` returns empty; no stamp. Verified by:
  - **Given** a synthetic 3MF whose part 2 is classified `modifier_part` but has zero triangles after sidecar parsing (no-op modifier), **when** the loader runs, **then** `MeshIR.objects[0].modifier_volumes[0].mesh.indices.is_empty() == true` AND `execute_region_mapping` emits no `fuzzy_skin.apply-to-all` stamps. | `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd empty_modifier_volume_stamps_no_regions -- --exact --nocapture`

## Verification

- `cargo check --workspace` — compile health.
- `cargo clippy -p slicer-host --tests -- -D warnings` — lint gate (per-crate).
- `cargo clippy --workspace -- -D warnings` — lint gate (workspace).
- `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` — fixture-backed E2E.
- `cargo test -p slicer-host --test threemf_paint_drop_on_modifier_tdd` — DEV-048 negative case.
- `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` — Packet 56's parser suite (must stay green).
- `cargo test -p slicer-host --test threemf_transform_tdd` — transform regression.
- `cargo test -p slicer-host --test gcode_emit_tdd` — G-code emission regression.
- `cargo test -p slicer-host --test benchy_painted_e2e_tdd` — no-sidecar E2E regression.
- `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` — paint-semantic regression.
- `cargo test -p slicer-host --test core_module_ir_access_contract_tdd` — manifest contract (after fuzzy-skin manifest edit).

## Authoritative Docs

- `docs/02_ir_schemas.md` — IR 0 `MeshIR` (lines 62-244) + versioning rule at line 5 + `ConfigDelta`/`ModifierVolume` shape (lines 192-211). Read directly. Edited at Step 7 to add the schema_version header annotation.
- `docs/01_system_architecture.md` :107-114 — RegionMapping responsibility and the pipeline ordering. Delegate SUMMARY if > 300 lines on a fresh read.
- `docs/03_wit_and_manifest.md` — module manifest TOML schema. Delegate SUMMARY for the `[config.schema]` block format.
- `docs/08_coordinate_system.md` — coordinate hazards; `slicer_core::polygon_ops` operates in scaled integer units. Read directly (small).
- `docs/07_implementation_status.md` — append TASK-191 and TASK-192a.
- `docs/DEVIATION_LOG.md` — register DEV-048.
- `docs/14_deviation_audit_history.md` — chronology entry.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — `<part subtype>`-branching production function. Delegate ONE Explore agent dispatch at Step 2 with the LOCATIONS contract:
  - Question: "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` that branch on `<part subtype>` and route geometry into the modifier-volume container. Return LOCATIONS with one-line role each; ≤ 5 entries. No source pasted."
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (or wherever the fuzzy-skin overlay is applied) — production fuzzy-skin overlap routine. Delegate ONE Explore agent dispatch at Step 5 with the LOCATIONS contract.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

Aggregate cost is **M** (4M + 4S step distribution). Downstream agents:

- treat `design.md`'s code change surface as the authoritative files-in-scope list;
- honor the out-of-bounds list — `crates/slicer-macros/`, `wit/**`, `crates/slicer-host/src/wit_host.rs`, OrcaSlicer source, `crates/slicer-sdk/`, `crates/slicer-ir/` — they must not be loaded directly;
- delegate every `cargo` run via a sub-agent FACT contract;
- delegate every authoritative-doc fact-check that exceeds 200 lines;
- stop reading at 60% context and hand off at 85%.

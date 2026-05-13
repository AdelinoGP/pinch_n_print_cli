---
status: draft
packet: 56_threemf-modifier-and-subtype-sidecar-ingestion
task_ids:
  - TASK-190
  - TASK-191
  - TASK-192
  - TASK-193
backlog_source: docs/07_implementation_status.md
context_cost_estimate: L
---

# Packet Contract: 56_threemf-modifier-and-subtype-sidecar-ingestion

> Aggregate context cost is **L** because the user-approved scope covers all five OrcaSlicer/Bambu `<part subtype>` values (`normal_part`, `modifier_part`, `negative_part`, `support_enforcer`, `support_blocker`) in a single vertical slice. Per the spec-packet-generator context-discipline contract, an L aggregate is an explicit activation-blocker — the packet stays `draft` until the user either authorizes the L scope or directs a split into Packet 57/58/59 along subtype-handler boundaries (see Activation Blocker Q1 below).

## Goal

Close the root cause behind two correlated bugs surfaced during DEV-046 validation against `resources/benchy_4color.3mf`:

1. **Bug A (root cause):** the 3MF loader in `crates/slicer-host/src/model_loader.rs::resolve_object` (lines 430-552) merges every `<component>` of a parent object into a single solid `ObjectMesh.mesh` (lines 463-530), ignoring the OrcaSlicer sidecar `Metadata/model_settings.config` which classifies each `<part>` by `subtype=`. Modifier and helper-geometry parts are currently extruded as solid plastic. `find_model_path` (lines 575-587) only ever opens `3D/3dmodel.model`; the sidecar is never read.
2. **Bug B (visible symptom):** the modifier cube in `benchy_4color.3mf` carries `<metadata key="fuzzy_skin" value="external"/>` on its `<part>`. Because Bug A drops the modifier marker, the modifier's `fuzzy_skin` configuration silently defaults to "applied to whole object", producing fuzzy paths on the entire benchy body instead of the cube's projected volume + explicitly painted `paint_fuzzy_skin` triangles.

This packet introduces a new parser for `Metadata/model_settings.config`, branches `resolve_object` to honor the subtype classification, plumbs typed per-part metadata into `ObjectMesh.modifier_volumes` (existing IR field; reuses the existing `ModifierVolume.config_delta: ConfigDelta` shape at `crates/slicer-ir/src/slice_ir.rs:254-265`), and wires four subtype-specific downstream consumers:

- `normal_part` — pass-through (existing behavior; reaffirmed by regression).
- `modifier_part` — geometry routed to `ObjectMesh.modifier_volumes`; per-modifier `fuzzy_skin` value typed into `config_delta`; `commit_region_mapping_builtin` learns to perform a 2D polygon overlap test between each `RegionPlan` polygon and the per-layer projection of each modifier volume, stamping `RegionPlan.config["fuzzy_skin.apply-to-all"]=true` on overlapping regions (user-approved Option 1 — region-mapping direct stamp, not paint-segmentation piggyback).
- `negative_part` — geometry collected and per-layer 2D-subtracted from the parent object's slice polygons via `slicer_core::polygon_ops::difference` (Clipper2) before regions enter `RegionMap`.
- `support_enforcer` / `support_blocker` — projected per layer and emitted as synthetic `PaintRegionIR` entries with `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`. This piggybacks on the paint-supports plumbing already proven by Packet 50b (TASK-180b) and the paint-semantic config-overlay proven by Packet 51 (TASK-181) — no new region-mapping code beyond what `modifier_part` adds.

Additive minor schema bump `MeshIR.schema_version` 1.0.0 → 1.1.0 to reflect that `ObjectMesh.modifier_volumes` is now contractually populated from 3MF sidecars (the field always existed but was always empty for the 3MF format path; the producer contract widens).

WIT scope is **clean** — neither `ObjectMesh.modifier_volumes` nor `ModifierVolume` is exposed in `wit/**` or any `wit_host.rs`/`wit_guest.rs` module (Step 0 gate confirmed by sub-agent search). No DEV-043-style scope-escalation deviation needed.

The closing E2E test is a TDD-RED test added by Step 12 that loads `resources/benchy_4color.3mf` and asserts (a) merged solid mesh has 225,240 triangles (down from 225,252), (b) `modifier_volumes.len() == 1` with typed `fuzzy_skin` config, (c) sliced G-code shows fuzzy-skin markers only in the cube's XY+Z projection band plus explicitly painted `paint_fuzzy_skin` facets.

## Scope Boundaries

- In scope:
  - `crates/slicer-host/src/model_loader.rs` — add `parse_3mf_sidecar` helper (or sibling file `model_loader_sidecar.rs`) that reads `Metadata/model_settings.config` from the same `zip::ZipArchive` already opened at line 558-559; branch `resolve_object` (lines 430-552) to classify each component by `subtype` and route geometry accordingly; bump `SemVer { major:1, minor:0, patch:0 }` at lines 195-199 to `SemVer { major:1, minor:1, patch:0 }`.
  - `crates/slicer-ir/src/slice_ir.rs` — **no struct shape changes.** Reuse existing `ModifierVolume.config_delta: ConfigDelta` (lines 254-265) to carry typed per-part metadata. Document the additive widening of the `MeshIR` producer contract in the IR doc.
  - `crates/slicer-host/src/region_mapping.rs` — extend `execute_region_mapping` (lines 249-258 onward) to take an additional `&[ModifierVolume]` projection slice (or the per-object volumes from `ExecutionPlan`) and stamp `RegionPlan.config["fuzzy_skin.apply-to-all"]=true` on overlapping regions via `slicer_core::polygon_ops::intersection`. Preserve the no-modifier fast path (bit-identical output when modifier_volumes is empty).
  - New host stage / helper in `crates/slicer-host/src/` (likely a new prepass-like function called from `pipeline.rs` between slicing and region-mapping) for `negative_part` per-layer 2D subtract. Concretely: when `ObjectMesh.modifier_volumes[i].config_delta.fields` contains the sentinel `subtype = negative` (typed `ConfigValue::String("negative_part")`), iterate the sliced polygons per layer and replace each with `slicer_core::polygon_ops::difference(slice_polygons, projected_negative_per_layer)`.
  - `crates/slicer-host/src/paint_segmentation.rs` or a sibling helper — extend the synthetic-`PaintRegionIR`-emission path to also emit per-layer `PaintRegion` polygons for each `support_enforcer` / `support_blocker` modifier volume (semantics `PaintSemantic::SupportEnforcer` / `PaintSemantic::SupportBlocker`).
  - `crates/slicer-host/src/model_loader.rs` — drop `paint_data` carried on a part classified as `modifier_part`, `negative_part`, `support_enforcer`, or `support_blocker`, with a single `log::warn!` per dropped layer; preserve `paint_data` on `normal_part`.
  - `modules/core-modules/fuzzy-skin/manifest.toml` — confirm (read-only; do not modify unless the schema is missing) that `apply-to-all` is declared in the `[config.schema]` block. If absent, register it (additive; no SemVer ripple).
  - `crates/slicer-host/tests/` — three new TDD test files:
    1. `threemf_sidecar_classification_tdd.rs` — sidecar parser unit tests (subtype enumeration, malformed XML fallback, unknown subtype downgrade).
    2. `benchy_4color_modifier_part_e2e_tdd.rs` — fixture-backed E2E: triangle counts, `modifier_volumes` shape, fuzzy region Z-band.
    3. `threemf_subtypes_synthetic_e2e_tdd.rs` — host-built synthetic 3MF archives (in-memory) exercising `negative_part`, `support_enforcer`, `support_blocker` paths against minimal benchy-like fixtures.
  - `docs/02_ir_schemas.md` — under IR 0 `MeshIR`, document that `ObjectMesh.modifier_volumes` is now populated by the 3MF loader when a sidecar classifies parts as non-`normal_part`; document the new `config_delta` typed keys (`subtype`, `fuzzy_skin`, optionally `extruder`); record the `MeshIR.schema_version` minor bump to 1.1.0 with a "Current schema_version: 1.1.0 (Packet 56)" annotation matching the Packet 51 / Packet 36 precedents.
  - `docs/07_implementation_status.md` — append TASK-190, TASK-191, TASK-192, TASK-193 in a new section appended after TASK-181 (highest current entry).
  - `docs/DEVIATION_LOG.md` — register three new DEV entries (numbering taken at Step 1; recommend DEV-047, DEV-048, DEV-049 — see Negative Test Cases below).
  - `docs/14_deviation_audit_history.md` — chronology entries for the three new DEV rows.

- Out of scope:
  - Bambu Studio printer-config block parsing in `Metadata/project_settings.config` (different file, different schema, separate concern). Treat its absence/presence as irrelevant to load success.
  - STL+sidecar JSON ingestion (rejected as YAGNI in DEV-044's predecessor scope; unchanged here).
  - Any change to `wit/**`. Confirmed by Step-0 sub-agent gate that neither `ObjectMesh.modifier_volumes` nor `ModifierVolume` crosses the WIT boundary.
  - Any change to `crates/slicer-host/src/wit_host.rs` and `crates/slicer-host/src/dispatch.rs` beyond the no-op consequence of an additional `modifier_volumes` round-trip (which is host-internal).
  - Any change to `crates/slicer-macros/src/lib.rs` (>2300 lines; explicit ban — delegated only if Step 0's gate flips).
  - Any change to `crates/slicer-sdk/` — no SDK trait, `ConfigView`, or builder change. The `fuzzy-skin` module reads `apply-to-all` from its existing `ConfigView` API (`modules/core-modules/fuzzy-skin/src/lib.rs:51-54`).
  - Any change to `crates/slicer-host/src/pipeline.rs` beyond inserting the new negative-part-subtract host stage call between `prepass::execute_prepass_with_builtins_configured` and `region_mapping::execute_region_mapping`, and forwarding `modifier_volumes` into the region-mapping call.
  - Paint data preservation on non-`normal_part` parts (dropped with warning per DEV-NN-B). Revisit only if a fixture demands it.
  - Consuming the sidecar `<part>/<metadata key="matrix">` for geometry placement — the model XML's `<component>` transform path remains the source of truth (recommendation in §9 of the planning handout). Sidecar `matrix` is captured into `config_delta` as a typed-string sanity-check field but does not affect mesh transforms.
  - `<assemble>` and `<plate>` sections of the sidecar — informational only; not consumed.
  - Any change to the fuzzy-skin algorithm itself (`modules/core-modules/fuzzy-skin/src/lib.rs:120+`). The region-stamped `apply-to-all` config flag is sufficient per the module's existing branching at lines 80-81.

## Prerequisites and Blockers

- Depends on:
  - DEV-044 closure (Packet 50; complete). Provides `FacetPaintData` ingestion that this packet's paint-drop-on-modifier path coexists with.
  - DEV-045 closure (Packet 51; complete). Provides `RegionPlan.config` paint-semantic overlay that the `support_enforcer`/`blocker` piggyback path reuses. Also provides the per-`PaintSemantic` `ResolvedConfig` resolution this packet borrows for support paint stamping.
  - DEV-046 closure (Packet 51 step 4 retroactive; complete). Provides `<build>/<item>` and `<component>` transform composition that this packet must preserve for the modifier volume placement.
  - `slicer_core::polygon_ops::intersection` and `slicer_core::polygon_ops::difference` (Clipper2-backed). Confirmed available; reused as proven by Packet 51's geometric-overlap path.
- Unblocks:
  - All future OrcaSlicer/Bambu-fixture round-trips that depend on per-part metadata respect (every published Bambu fixture above one part uses the sidecar).
  - A follow-up packet for sidecar-`extruder` per-modifier tool-index assignment (out of scope here; `config_delta` carries the value but no consumer wires it yet).
- Activation blockers (must be resolved before flipping `status: draft` → `active`):
  - **Q1 (scope split decision).** Aggregate cost is **L** because of the five-subtype scope. Skill rule mandates a split before activation. Resolution options:
    1. Split into Packet 56 (`normal_part` + `modifier_part` + sidecar parser), Packet 57 (`negative_part`), Packet 58 (`support_enforcer` + `support_blocker`).
    2. Override the L-aggregate skill rule and activate this packet as a single vertical slice (user already approved the five-subtype scope at packet-author time).
    3. Reduce scope to `normal_part` + `modifier_part` only and defer the other three to follow-up packets.
    Decision text must land in this packet before activation.
  - **Q2 (deviation numbering).** Confirm DEV-047, DEV-048, DEV-049 are the next free deviation slots (verify against `docs/DEVIATION_LOG.md` at packet-open time; numbering may need bumping if other packets register deviations first).
  - **Q3 (negative-part subtract stage placement).** Two reasonable insertion points exist:
    1. New host stage called between `execute_prepass_with_builtins_configured` and `commit_region_mapping_builtin` in `pipeline.rs`, operating on `SliceIR` per-layer polygons.
    2. Inline subtract inside `commit_region_mapping_builtin` before the region polygon is materialized into `RegionPlan`.
    Decision lock at packet-author time: **Option 1.** Justification: keeps `region_mapping.rs` focused on the config-overlay rule; isolates negative-part geometry from the per-paint-semantic overlay path; produces a separately-testable host stage. Recorded here so Step 7 has no design ambiguity.
  - **Q4 (fuzzy-skin manifest schema gate).** Confirm that `apply-to-all` is declared in `modules/core-modules/fuzzy-skin/manifest.toml`'s `[config.schema]` block. If missing, Step 5 adds it; if present, Step 5 is skipped. Sub-agent FACT dispatch at Step 5 start.

## Acceptance Criteria

- **Given** `resources/benchy_4color.3mf` exists and `Metadata/model_settings.config` classifies `<part id="2">` as `subtype="modifier_part"`, **when** `slicer_host::model_loader::load_model("resources/benchy_4color.3mf")` is invoked, **then** the returned `MeshIR.objects[0].mesh.indices.len() / 3 == 225_240` (exactly; the 12 cube triangles are NOT in the solid mesh) AND `MeshIR.schema_version == SemVer { major: 1, minor: 1, patch: 0 }`. | `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_part_excluded_from_solid_mesh -- --exact --nocapture`
- **Given** the same fixture load, **when** `MeshIR.objects[0].modifier_volumes` is inspected, **then** `len() == 1`, the entry's `config_delta.fields.get(&ConfigKey::from("fuzzy_skin"))` returns `Some(ConfigValue::String("external".into()))`, AND `config_delta.fields.get(&ConfigKey::from("subtype"))` returns `Some(ConfigValue::String("modifier_part".into()))`. | `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_volume_carries_typed_metadata -- --exact --nocapture`
- **Given** `resources/benchy_painted.3mf` (no sidecar) is loaded, **when** the loader runs, **then** `MeshIR.objects[0].modifier_volumes.is_empty()` AND the slice output is byte-identical to the pre-packet G-code for the same config (no regression on the no-sidecar path). | `cargo test -p slicer-host --test benchy_painted_e2e_tdd painted_benchy_3mf_reaches_paint_segmentation -- --exact --nocapture && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd paint_config_override_visibly_differs_gcode -- --exact --nocapture`
- **Given** the build/item transform `1 0 0 0 1 0 0 0 1 120.164588 105 35.2312426` and the `<component objectid="2">` row-major transform from `3dmodel.model`, **when** the modifier volume's world-space AABB is computed, **then** its centroid in X/Y/Z lies within ±0.01 mm of the cube's predicted projected position (sub-agent computes the expected centroid from the model XML transform composition; sidecar `matrix` is consulted only as a sanity-check log line, not as the geometry source per Out-of-Scope §5). | `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_world_aabb_matches_composition -- --exact --nocapture`
- **Given** the modifier-vs-region overlap test runs in `commit_region_mapping_builtin`, **when** a region polygon at layer Z = (cube Z-min + 0.5 mm) intersects the projected modifier volume, **then** that region's `RegionPlan.config` carries the key `fuzzy_skin.apply-to-all = ConfigValue::Bool(true)` and a region at layer Z = (cube Z-max + 1.0 mm) does NOT carry that key (i.e., overlap is geometric, not whole-object). | `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd region_overlap_stamps_only_in_cube_zband -- --exact --nocapture`
- **Given** the full sliced G-code from `resources/benchy_4color.3mf`, **when** fuzzy-skin marker lines (`; FEATURE: fuzzy_skin` or the existing per-segment marker convention; the test reads the marker convention from `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs::count_perimeter_markers_in_z_band`) are counted in a Z-band intersecting the cube vs a Z-band well above the cube, **then** the intersecting band has > 5 fuzzy markers AND the above-cube band has 0 fuzzy markers on regions that are NOT in the `paint_fuzzy_skin` triangle set. | `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd fuzzy_region_restricted_to_cube_and_painted_facets -- --exact --nocapture`
- **Given** a synthetic 3MF fixture (built in test code with an in-memory `zip` archive) where part 2 carries `subtype="negative_part"`, **when** the model is loaded and sliced through a host stage call to `apply_negative_part_subtract` (Step 7 introduces this stage), **then** the sliced layer polygons at every Z intersecting the negative volume have area strictly less than the same layer's polygons would have without the negative subtract (compare areas via `slicer_core::polygon_ops::area`). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_removes_layer_polygon_area -- --exact --nocapture`
- **Given** a synthetic 3MF fixture where part 2 carries `subtype="support_enforcer"`, **when** the model is sliced, **then** `PaintRegionIR.per_layer[N].semantic_regions.get(&PaintSemantic::SupportEnforcer)` returns `Some(_)` for every layer N intersecting the enforcer volume, AND the polygons in that semantic match the modifier's per-layer projection within ±0.005 mm². The same property holds for `support_blocker` mapped to `PaintSemantic::SupportBlocker`. | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_enforcer_emits_paint_region && cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_blocker_emits_paint_region`
- **Given** clippy is the lint gate, **when** Step 12 runs, **then** `cargo clippy -p slicer-host --tests -- -D warnings` is green AND `cargo clippy --workspace -- -D warnings` is green. | `cargo clippy -p slicer-host --tests -- -D warnings && cargo clippy --workspace -- -D warnings`
- **Given** the existing regression-defense surfaces must stay GREEN, **when** Step 12 runs, **then** `cargo test -p slicer-host --test threemf_transform_tdd` reports 10/10 passes AND `cargo test -p slicer-host --test gcode_emit_tdd` reports 27/27 passes. | `cargo test -p slicer-host --test threemf_transform_tdd && cargo test -p slicer-host --test gcode_emit_tdd`
- **Given** `MeshIR.schema_version` bumps additively to 1.1.0, **when** Step 1 lands, **then** `crates/slicer-host/src/model_loader.rs:194-199` constructs `SemVer { major: 1, minor: 1, patch: 0 }` AND `docs/02_ir_schemas.md`'s IR 0 section carries a header line `**Current schema_version: 1.1.0** (Bumped to 1.1.0 by packet 56 — populated `modifier_volumes` from `Metadata/model_settings.config`.)` modelled on the IR 2 / IR 5 precedents. | `rg -q 'SemVer \{[^}]*minor: 1[^}]*patch: 0' crates/slicer-host/src/model_loader.rs && rg -q 'schema_version: 1\.1\.0.*packet 56' docs/02_ir_schemas.md`
- **Given** the four new TASK ids are appended to `docs/07_implementation_status.md`, **when** Step 12 runs, **then** the file contains `[x] TASK-190`, `[x] TASK-191`, `[x] TASK-192`, `[x] TASK-193` rows AND each row names this packet (`56_threemf-modifier-and-subtype-sidecar-ingestion`). | `rg -q '\[x\] TASK-190.*56_threemf' docs/07_implementation_status.md && rg -q '\[x\] TASK-191.*56_threemf' docs/07_implementation_status.md && rg -q '\[x\] TASK-192.*56_threemf' docs/07_implementation_status.md && rg -q '\[x\] TASK-193.*56_threemf' docs/07_implementation_status.md`
- **Given** the three new deviations are registered and closed by this packet (DEV-NN-A "partial subtype coverage", DEV-NN-B "paint dropped on non-normal parts", DEV-NN-C "missing/malformed sidecar fallback"), **when** Step 12 runs, **then** `docs/DEVIATION_LOG.md` shows three new rows (recommended IDs DEV-047, DEV-048, DEV-049) all in status `Closed — Packet 56, 2026-MM-DD`. | `rg -c '^\| DEV-04[789].*Closed.*Packet 56' docs/DEVIATION_LOG.md` (expected: 3)

## Negative Test Cases

- **Given** a 3MF archive whose `Metadata/model_settings.config` is malformed (truncated XML cut mid-`<part>`), **when** `slicer_host::model_loader::load_model` is invoked, **then** the loader emits a structured `log::warn!` with target `slicer_host::model_loader::sidecar` containing the substring "treating all parts as normal_part" AND returns `Ok(MeshIR)` with `modifier_volumes.is_empty()` for every object (DEV-NN-C: missing/malformed sidecar is non-fatal). The load does NOT return `Err(ModelLoadError::ThreeMfParse)`. | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd malformed_sidecar_falls_back_to_normal_part -- --exact --nocapture`
- **Given** a 3MF archive whose `Metadata/model_settings.config` contains a part with `subtype="unrecognized_subtype_value"`, **when** the loader runs, **then** it logs a `log::warn!` naming the unknown subtype, treats that part as `normal_part`, and returns `Ok(MeshIR)` (DEV-NN-A: partial subtype coverage; unknown subtypes silently downgrade). | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd unknown_subtype_downgrades_to_normal_part -- --exact --nocapture`
- **Given** a 3MF archive whose part 2 is classified `modifier_part` AND carries `paint_color="4"` triangle attributes in `3dmodel.model`, **when** the loader runs, **then** `MeshIR.objects[0].paint_data` does NOT contain any `PaintLayer` entries sourced from part 2 (paint dropped on modifier; DEV-NN-B), AND the loader emits a `log::warn!` containing "paint data on non-normal part dropped" with part id 2 in the message. | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd paint_on_modifier_part_dropped_with_warning -- --exact --nocapture`
- **Given** a 3MF archive with `Metadata/model_settings.config` absent entirely (e.g., `resources/benchy_painted.3mf`), **when** the loader runs, **then** every part is treated as `normal_part`, `modifier_volumes.is_empty()` for every object, no `log::warn!` is emitted (absence is NOT an error and NOT a warning — it is the default), and existing Packet 50 / Packet 51 regression tests stay green. | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd missing_sidecar_is_silent_default -- --exact --nocapture && cargo test -p slicer-host --test benchy_painted_e2e_tdd && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd`

## Verification

- `cargo check --workspace` — compile health.
- `cargo clippy --workspace -- -D warnings` — lint gate.
- `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` — sidecar parser unit suite.
- `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` — fixture-backed E2E.
- `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd` — synthetic-fixture negative/support paths.
- `cargo test -p slicer-host --test threemf_transform_tdd` — transform regression.
- `cargo test -p slicer-host --test gcode_emit_tdd` — G-code emission regression.
- `cargo test -p slicer-host --test benchy_painted_e2e_tdd` — no-sidecar regression.
- `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` — paint-semantic regression.

## Authoritative Docs

- `docs/02_ir_schemas.md` — IR 0 (`MeshIR`, lines 62-244) and the versioning rule at line 5. Read directly; relevant sections only.
- `docs/01_system_architecture.md` — RegionMapping responsibility (`:107-114`) and the pipeline ordering (slice → prepass → region-map). Delegate SUMMARY if > 300 lines on a fresh read.
- `docs/04_host_scheduler.md` — prepass ordering and the slot where the new negative-part-subtract stage lands. Delegate the section read.
- `docs/08_coordinate_system.md` — coordinate hazards; `slicer_core::polygon_ops` operates in scaled integer units. Read directly (small).
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — append the three new DEV rows.
- `docs/07_implementation_status.md` — append TASK-190..193 rows after TASK-181.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — the production sidecar parser. Delegate via Explore agent; never load. Question to delegate: "Name the function(s) in `bbs_3mf.cpp` that parse `Metadata/model_settings.config` and the function(s) that branch on `<part subtype>`. Return LOCATIONS with one-line role; do not paste source." Cite the returned names in `design.md` and `requirements.md`. The host implementation MUST be project-internal Rust.
- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — same delegate dispatch for `negative_part` per-layer subtract entry-points (function names only).
- `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — same delegate dispatch for support enforcer/blocker geometry paths (function names only).

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md` (required: this packet maps four new TASK IDs onto distinct implementation steps and registers three new deviations)

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list;
- honor `design.md`'s out-of-bounds list — `crates/slicer-macros/`, `wit/**`, `crates/slicer-host/src/wit_host.rs`, OrcaSlicer source — they must not be loaded directly;
- delegate every `cargo` run via a sub-agent FACT contract;
- delegate every authoritative-doc fact-check that exceeds 200 lines (`docs/01`, `docs/04`, OrcaSlicer source);
- stop reading at 60% context and hand off at 85%.

Aggregate context cost is **L** because of the five-subtype scope (see Activation Blocker Q1). If activation Q1 is resolved by splitting, this annotation drops; if it is resolved by override, the implementer MUST budget worker dispatch heavily — each subtype-handler step is its own worker run with a fresh context.

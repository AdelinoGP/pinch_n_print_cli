---
status: draft
packet: 57_3mf-fixture-e2e-hardening
task_ids:
  - TASK-205
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
depends_on:
  - 56c_threemf-negative-and-support-subtype-routing (must be status: implemented; provides negative_part_subtract + support paint emission)
  - resources/cube_positive_n_negative.3mf (must exist on disk; negative_part fixture)
  - resources/bridge_support_enforcers.3mf (must exist on disk; support_enforcer + support_blocker fixture)
  - resources/benchy_4color.3mf (must exist on disk; modifier_part regression fixture)
unblocks:
  - 68_extruder-per-modifier-gcode (turns RED tests GREEN)
---

# Packet Contract: 57_3mf-fixture-e2e-hardening

> Hardening packet — loads real on-disk 3MF files through `load_model()` → full pipeline, verifying all five 3MF subtype consumers (Packets 56/56b/56c) work end-to-end. Includes RED tests that document pending extruder behavior for Packet 58.

## Goal

Add integration tests in `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` that load real on-disk 3MF files (`cube_positive_n_negative.3mf`, `bridge_support_enforcers.3mf`, `benchy_4color.3mf`) through `load_model()` → full pipeline, asserting that: negative_part reduces per-layer polygon area, support_enforcer and support_blocker emit `PaintRegionIR` entries, modifier_part fuzzy-skin is intact (regression), modifier_volumes carry correct subtype/extruder metadata, duplicate part IDs don't panic, and models without negative parts skip subtract. Two RED tests document extruder metadata routing (for Packet 58).

## Scope Boundaries

In scope: `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` (NEW) — integration tests loading 3MF files from `resources/` through the full host pipeline. Tests assert IR-level outcomes: polygon area reduction, `PaintRegionIR` semantic entries, modifier_volume metadata, regression coverage for Packets 56/56b/56c. `docs/07_implementation_status.md` — register TASK-205.

Out of scope: creating or modifying 3MF fixtures (all three fixtures exist on disk), implementing extruder GCode consumption (Packet 58), changes to any production source file, `<assemble>`/`<plate>` parsing, new IR types, WIT changes.

## Prerequisites and Blockers

- **Packet 56c** (`56c_threemf-negative-and-support-subtype-routing`) must be `status: implemented`. Provides `apply_negative_part_subtract` and support paint-segmentation piggyback.
- Fixtures `resources/cube_positive_n_negative.3mf`, `resources/bridge_support_enforcers.3mf`, `resources/benchy_4color.3mf` must exist on disk.
- `load_model()` (public API at `model_loader.rs:145`) must be callable from integration tests.

## Acceptance Criteria

- **Given** `cube_positive_n_negative.3mf` is loaded via `load_model()` and processed through the full pipeline (prepass + per-layer slice), **when** `apply_negative_part_subtract` runs on a layer Z inside the negative cube's extent, **then** the sum of `slice_ir.regions[].polygons` area is strictly less than the sum from a baseline run where the negative modifier_volume is removed before slicing. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd negative_part_subtracts_via_full_pipeline -- --exact --nocapture`
- **Given** `cube_positive_n_negative.3mf` is loaded, **when** modifier_volumes for the negative_part are inspected, **then** the modifier_volume mesh vertices are in world space reflecting the component transform baked by `resolve_object` (X offset ~-11.1 mm, Y offset ~-11.9 mm — the mesh is NOT at the origin). | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd negative_part_transform_baked_correctly -- --exact --nocapture`
- **Given** `cube_positive_n_negative.3mf` is loaded, **when** modifier_volumes are inspected, **then** `modifier_volumes[0].config_delta.fields` contains `"subtype" => ConfigValue::String("negative_part")` AND `"extruder" => ConfigValue::Int(0)`. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd modifier_volumes_populated_with_correct_metadata -- --exact --nocapture`
- **Given** `bridge_support_enforcers.3mf` is loaded and `execute_paint_segmentation` runs, **when** `paint_region_ir.per_layer` is queried at layers intersecting object 4's support enforcer volumes, **then** at least one layer contains `semantic_regions.get(&PaintSemantic::SupportEnforcer)` returning a non-empty `Vec<SemanticRegion>`. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd support_enforcer_emits_paint_regions_from_disk -- --exact --nocapture`
- **Given** `bridge_support_enforcers.3mf` is loaded and `execute_paint_segmentation` runs, **when** `paint_region_ir.per_layer` is queried at layers intersecting object 5's support blocker volumes, **then** at least one layer contains `semantic_regions.get(&PaintSemantic::SupportBlocker)` returning a non-empty `Vec<SemanticRegion>`. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd support_blocker_emits_paint_regions_from_disk -- --exact --nocapture`
- **Given** `benchy_4color.3mf` is loaded, **when** the model is processed through prepass, **then** modifier_part volumes exist on `mesh_ir.objects[].modifier_volumes` AND `execute_paint_segmentation` completes without error (Packet 56b regression). | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd modifier_part_benchy_regression -- --exact --nocapture`
- **Given** `benchy_4color.3mf` is loaded (contains no negative_part), **when** `apply_negative_part_subtract` runs on any layer's `SliceIR`, **then** the per-layer polygon area is bit-identical to a baseline run where the subtract stage is skipped entirely. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd model_without_negative_skips_subtract -- --exact --nocapture`
- **Given** `bridge_support_enforcers.3mf` is loaded, **when** `mesh_ir.objects` is inspected, **then** `objects.len() == 2` (object 4 and object 5) AND each object carries its own distinct `modifier_volumes` — object 4 has `support_enforcer` volumes, object 5 has `support_blocker` volumes. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd two_objects_produce_separate_modifier_volumes -- --exact --nocapture`
- **Given** `bridge_support_enforcers.3mf` is loaded where part id=3 appears twice for each object (two support enforcer instances on object 4, two support blocker instances on object 5), **when** the model is loaded, **then** the loader does not panic AND both modifier_volume entries for id=3 are present (the second entry supersedes or accumulates deterministically). | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd duplicate_part_id_handled_gracefully -- --exact --nocapture`
- **Given** `bridge_support_enforcers.3mf` is loaded and processed through paint_segmentation, **when** `SemanticRegion.value` is inspected for the support_enforcer entries, **then** `value` is `PaintValue::ToolIndex(0)` (the support part's extruder=0) rather than `PaintValue::Flag(true)`. This is a RED test — it documents the expected behavior for Packet 58's extruder routing. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd extruder_metadata_reaches_tool_index -- --exact --nocapture` (RED until Packet 58)
- **Given** `bridge_support_enforcers.3mf` is loaded where the parent object has extruder=1 and the support parts have extruder=0, **when** the GCode output is inspected, **then** the GCode contains both `T0` (support regions) and `T1` (normal part regions) tool-change commands. This is a RED test — it documents the expected behavior for Packet 58's GCode integration. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd extruder_per_object_vs_support_extruder -- --exact --nocapture` (RED until Packet 58)

## Negative Test Cases

- **Given** `benchy_4color.3mf` (no negative_part), **when** `apply_negative_part_subtract` runs, **then** no modifier_volume has subtype `negative_part`, the function is a no-op, and per-layer polygons are unchanged. Verified by AC-7 above.
- **Given** a fixture path that does not exist on disk, **when** `load_model()` is called, **then** it returns `Err(ModelLoadError::...)` without panicking. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd missing_fixture_returns_error -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test threemf_fixture_e2e_tdd` — all GREEN tests pass; all RED tests fail with the expected assertion (not a panic or unrelated error).
- `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd && cargo test -p slicer-host --test threemf_sidecar_classification_tdd && cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` — regression sweep for Packets 56/56b/56c.
- `cargo clippy --workspace -- -D warnings` — lint gate.

## Doc Impact Statement

- `docs/07_implementation_status.md` — append TASK-205 row after TASK-193 (line 147). Verify: `rg -c 'TASK-205' docs/07_implementation_status.md` → 1.

## Deviations

None at authoring time. The two RED tests (AC-R1, AC-R2) are intentional — they document expected behavior for Packet 58 and are marked RED in this packet's scope.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `ModifierVolume`, `ConfigDelta`, `PaintRegionIR`, `SemanticRegion`, `PaintSemantic` shapes (delegate narrow search).
- `docs/04_host_scheduler.md` — prepass stage ordering (delegate SUMMARY).
- `docs/08_coordinate_system.md` — scaled integer units for area assertions.

## OrcaSlicer Reference Obligations

None. This packet is test-only and exercises host-native Rust pipeline paths. No OrcaSlicer parity is required for fixture-based integration tests.

## Context Discipline Note

This packet is test-only — zero production code changes. The implementation reads 3 existing 3MF fixtures from `resources/` and exercises the public `load_model()` API plus existing host functions (`execute_paint_segmentation`, `apply_negative_part_subtract`). No sub-agent needs to read OrcaSlicer source, WIT files, or generated code.

Aggregate cost is **M**. Downstream agents:
- treat the test file as the sole edit surface;
- delegate every `cargo` run via a sub-agent FACT contract;
- stop reading at 60% context and hand off at 85%.

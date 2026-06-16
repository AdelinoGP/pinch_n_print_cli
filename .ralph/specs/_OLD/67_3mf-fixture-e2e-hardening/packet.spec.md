---
status: implemented
packet: 67_3mf-fixture-e2e-hardening
task_ids:
  - TASK-208
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

# Packet Contract: 67_3mf-fixture-e2e-hardening

> Hardening packet — loads real on-disk 3MF files through `load_model()` → full pipeline, verifying all five 3MF subtype consumers (Packets 56/56b/56c) work end-to-end. Adds five region-mapping tests (AC-Mod-1..6) that verify Packet 68's modifier `config_delta` stamping contract: `negative_part` and `modifier_part` stamp into `RegionPlan.config.extensions`; `support_enforcer` and `support_blocker` do not (OrcaSlicer parity per `PrintApply.cpp:590-594`). Includes a loader production fix that surfaces object-scoped sidecar metadata into `ObjectConfig.data`.

## Goal

Add integration tests in `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` that load real on-disk 3MF files (`cube_positive_n_negative.3mf`, `bridge_support_enforcers.3mf`, `benchy_4color.3mf`) through `load_model()` → full pipeline, asserting that: negative_part reduces per-layer polygon area, support_enforcer and support_blocker emit `PaintRegionIR` entries, modifier_part fuzzy-skin is intact (regression), modifier_volumes carry correct subtype/extruder metadata, duplicate part IDs don't panic, and models without negative parts skip subtract. **Adds 8 new tests** covering (a) the loader fix that populates `ObjectConfig.data` from sidecar object-scoped metadata (AC-Loader-1, AC-Loader-2) and (b) the OrcaSlicer-parity modifier propagation contract for Packet 68 (AC-Mod-1..6: 3 RED tests gate Packet 68's `stamp_modifier_config_deltas`, 3 GREEN parity guards catch the failure modes where Packet 68 over-stamps enforcer/blocker or where someone re-wires the divergent paint-segmentation extruder routing). **Withdraws AC-R1 and AC-R2** — both were premised on the OrcaSlicer-divergent claim that `support_enforcer` `extruder` field propagates to a tool change. See Deviations (D6).

## Scope Boundaries

In scope: `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` and `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — integration tests loading 3MF files from `resources/` through `load_model()` and through `execute_paint_segmentation` + `execute_region_mapping_with_cap`. Tests assert IR-level outcomes: polygon area reduction, `PaintRegionIR` semantic entries, modifier_volume metadata, regression coverage for Packets 56/56b/56c, modifier `config_delta` propagation into `RegionPlan.config.extensions`. **Loader production fix** in `crates/slicer-host/src/model_loader_sidecar.rs`, `crates/slicer-host/src/model_loader.rs`, and `crates/slicer-host/src/main.rs` — surface object-scoped sidecar metadata (`extruder`, `enable_support`, `support_type` allowlist) into `ObjectMesh.config.data` and seed it into `config_source` via the `object_config:<id>:<key>` prefix for `resolve_per_object_configs` consumption. `docs/07_implementation_status.md` — register TASK-208 and three follow-up TASK rows for downstream gaps. Packet 68 design and packet.spec amendments — add ENFORCER/BLOCKER subtype filter to `stamp_modifier_config_deltas`, retarget AC-2 to synthetic-IR test, add `AC-Filter` reusing AC-Mod-4 as cross-packet regression guard.

Out of scope: creating or modifying 3MF fixtures (all three fixtures exist on disk), implementing extruder GCode consumption (Packet 68), `<assemble>`/`<plate>` parsing, new IR types, WIT changes, `support_filament` GCode routing, additional object-level allowlist keys beyond `extruder`/`enable_support`/`support_type`.

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
- **AC-Loader-1.** **Given** any of the three Packet 67 fixtures is read by `parse_3mf_sidecar`, **when** `ObjectSidecarInfo.object_metadata` is inspected for the parent object id, **then** it contains `extruder = "1"`; for `bridge_support_enforcers.3mf` object 5 additionally `enable_support = "1"` and `support_type = "tree(auto)"`. | `cargo test -p slicer-host --test threemf_sidecar_classification_tdd sidecar_parser_extracts_object_metadata -- --exact --nocapture`
- **AC-Loader-2.** **Given** any of the three Packet 67 fixtures is loaded by `load_model`, **when** `mesh_ir.objects[i].config.data` is inspected, **then** every object has `config.data["extruder"] == ConfigValue::Int(1)`; for `bridge_support_enforcers.3mf` object 5 additionally `enable_support = Bool(true)` and `support_type = String("tree(auto)")`. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd load_model_populates_object_config_data -- --exact --nocapture`
- **AC-Mod-1.** **Given** `cube_positive_n_negative.3mf` is loaded, paint segmentation runs, and `execute_region_mapping_with_cap` produces a `RegionMapIR`, **when** the entries are walked, **then** at least one `entries[*].plan.config.extensions["extruder"]` equals `ConfigValue::Int(0)` — stamped from the negative_part modifier's `config_delta`. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd negative_part_stamps_extruder_into_extensions -- --exact --nocapture` (RED until Packet 68)
- **AC-Mod-2.** **Given** `benchy_4color.3mf` is processed through `execute_region_mapping_with_cap`, **when** the entries are walked, **then** at least one `entries[*].plan.config.extensions["fuzzy_skin"]` equals `ConfigValue::String("external")` — stamped from the modifier_part. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd modifier_part_stamps_fuzzy_skin_into_extensions -- --exact --nocapture` (RED until Packet 68)
- **AC-Mod-3.** **Given** `benchy_4color.3mf` is processed through `execute_region_mapping_with_cap`, **when** the entries are walked, **then** at least one `entries[*].plan.config.extensions["extruder"]` equals `ConfigValue::Int(0)` — stamped from the modifier_part. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd modifier_part_stamps_extruder_into_extensions -- --exact --nocapture` (RED until Packet 68)
- **AC-Mod-4.** **Given** `bridge_support_enforcers.3mf` is processed through `execute_region_mapping_with_cap`, **when** the entries are walked, **then** NO `entries[*].plan.config.extensions` contains `extruder` — `support_enforcer` is excluded from stamping per OrcaSlicer parity (`PrintApply.cpp:590-594`). | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd support_enforcer_config_delta_not_stamped -- --exact --nocapture`
- **AC-Mod-5.** **Given** the same fixture and pipeline as AC-Mod-4, **when** entries are walked, **then** the same `extruder`-key-absent assertion holds for the `support_blocker`-only object (kept as a separate test so each subtype's parity contract is findable in test output). | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd support_blocker_config_delta_not_stamped -- --exact --nocapture`
- **AC-Mod-6.** **Given** `bridge_support_enforcers.3mf` is processed through `execute_paint_segmentation`, **when** every `SemanticRegion` in `LayerPaintMap.semantic_regions.get(&PaintSemantic::SupportEnforcer)` is inspected, **then** every `region.value` is `PaintValue::Flag(_)` — never `PaintValue::ToolIndex(_)`. Paint-segmentation parity guard: catches anyone re-wiring the divergent extruder→ToolIndex path that AC-R1 was testing for. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd support_enforcer_paint_value_is_flag_not_tool_index -- --exact --nocapture`

## Negative Test Cases

- **Given** `benchy_4color.3mf` (no negative_part), **when** `apply_negative_part_subtract` runs, **then** no modifier_volume has subtype `negative_part`, the function is a no-op, and per-layer polygons are unchanged. Verified by AC-7 above.
- **Given** a fixture path that does not exist on disk, **when** `load_model()` is called, **then** it returns `Err(ModelLoadError::...)` without panicking. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd missing_fixture_returns_error -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test threemf_fixture_e2e_tdd` — 14 GREEN tests pass; 3 RED tests (AC-Mod-1, AC-Mod-2, AC-Mod-3) fail with the documented assertion messages (not panics or unrelated errors).
- `cargo test -p slicer-host --test threemf_sidecar_classification_tdd sidecar_parser_extracts_object_metadata -- --exact --nocapture` — AC-Loader-1 GREEN (sidecar parser surfaces object-level metadata).
- `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd && cargo test -p slicer-host --test threemf_sidecar_classification_tdd && cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd && cargo test -p slicer-host --test threemf_transform_tdd` — regression sweep for Packets 56/56b/56c (loader fix's downstream impact).
- `cargo clippy --workspace -- -D warnings` — lint gate.
- Combined: 15 GREEN tests (across both fixture and sidecar files) + 3 RED tests with documented assertion messages.

## Doc Impact Statement

- `docs/07_implementation_status.md` — append TASK-208 row. Verify: `rg -c 'TASK-208.*3mf.fixture.e2e' docs/07_implementation_status.md` → 1.

## Deviations

- **D1.** [Scope, reqs + design] — Specified: No production code changes; this packet is test-only. | Implemented: ~170 lines added to `crates/slicer-host/src/model_loader.rs` (p:path production extension parser support). | Reason: Both `cube_positive_n_negative.3mf` and `bridge_support_enforcers.3mf` use p:path to reference external .model files inside the archive. The fixtures were unparseable without this capability. The scope boundary was violated because the fixtures were not validated for parser compatibility before packet authoring.
- **D2.** [AC-R1, AC-R2 + design locked decision #5] — Specified: RED tests use `assert!` on unfulfilled condition and produce a specific assertion failure. `#[should_panic]` explicitly rejected. | Implemented: RED tests were wrapped in `#[ignore]` and never ran. | Resolution: `#[ignore]` attributes removed; later, both AC-R1 and AC-R2 were **withdrawn entirely** — see D6.
- **D3.** [AC-R2 text] — Specified: Test asserts GCode contains T0 and T1 tool-change commands. | Implemented: Test was first downgraded to a `config_delta.fields["extruder"]` existence check. | Resolution: AC-R2 was ultimately **withdrawn** (along with AC-R1) — see D6. The downgrade and the original AC text were both based on an OrcaSlicer-divergent premise.
- **D4.** [TASK-205 collision] — Specified: Register TASK-205 as a new task ID in docs/07. | Implemented: TASK-205 was already claimed by Packet 65 ("Complete HostRunOptions, delete validate_run_options and CliError"). | Resolution: Packet 67 re-numbered to TASK-208.
- **D5.** [Packet slug] — Specified: Folder `67_3mf-fixture-e2e-hardening` implies packet 67. | Implemented: Front-matter used `57_3mf-fixture-e2e-hardening` and narrative text used "Packet 58" for the downstream target (which is Packet 68). | Resolution: All references normalized to 67 / 68.
- **D6.** [AC-R1, AC-R2 withdrawal — OrcaSlicer parity finding] — **Both AC-R1 and AC-R2 are withdrawn**, replaced by AC-Mod-1..6 with the correct contract. Root cause: both ACs assumed `ModifierVolume.config_delta.fields["extruder"]` on a `support_enforcer` propagates into a `T{n}` GCode tool change. OrcaSlicer source proves this is wrong: `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:590-594` — `model_volume_solid_or_modifier()` excludes `SUPPORT_ENFORCER` and `SUPPORT_BLOCKER` from region-config merging; their `extruder` field is decorative. `crates/slicer-host/src/paint_segmentation.rs:416` hardcodes `PaintValue::Flag(true)` for those semantics — `PaintValue::ToolIndex` is only constructed from per-facet MMU paint XML (`model_loader.rs:1187, 1504`), never from modifier_volume metadata. For `bridge_support_enforcers.3mf`, real OrcaSlicer emits only `T1`. AC-Mod-4, AC-Mod-5, and AC-Mod-6 are the new GREEN regression guards that catch anyone re-introducing the divergent contract.
- **D7.** [Packet 68 design amendment, bundled into this change per Q1] — Packet 68's `design.md` defined `stamp_modifier_config_deltas` to stamp all modifier types into `RegionPlan.config.extensions` with no subtype filter, which would cause spurious `T0` for the bridge fixture. This packet ships the amendment to Packet 68 (see `.ralph/specs/68_extruder-per-modifier-gcode/design.md` "Subtype filter" row): `support_enforcer` and `support_blocker` are excluded from stamping. Additionally, Packet 68's `AC-2` (originally T0+T1 on the bridge fixture) is retargeted to a synthetic-IR test, and a new `AC-Filter` is added that reuses Packet 67's `AC-Mod-4` as a permanent cross-packet regression guard.
- **D8.** [Scope expansion — loader production fix bundled into this packet] — Following the OrcaSlicer parity finding, the user-chosen resolution was to fix `crates/slicer-host/src/model_loader_sidecar.rs` (extract object-scoped sidecar `<metadata>` entries into a new `ObjectSidecarInfo.object_metadata` field), `crates/slicer-host/src/model_loader.rs` (populate `ObjectMesh.config.data` from the allowlist `extruder`/`enable_support`/`support_type`), and `crates/slicer-host/src/main.rs` (seed `object_config:<id>:<key>` entries into `config_source` mirroring the existing `object_height:<id>` pattern at lines 196-205). Production code touched: ~65 lines across 3 files. Justification follows the existing D1 precedent. **L-size rationale**: the aggregate change (loader fix + 8 new tests + Packet 67 doc updates + Packet 68 doc amendments + 3 follow-up TASK rows) trips the `CLAUDE.md` L-size split threshold, but the change is logically one decision tree — fixing AC-R2 surfaced the OrcaSlicer parity gap, which surfaced the Packet 68 design gap, which surfaced the loader gap. Splitting would impose cascading `depends_on:` overhead across three sequential closures; reviewing the diff in one place is cheaper. Future contributors must not read this as carte blanche for L bundles.
  - **Downstream gaps** (tracked as new `docs/07_implementation_status.md` rows per Q10):
    - `support_filament` GCode routing is unwired; without it, real-fixture multi-material on `bridge_support_enforcers.3mf` cannot emit a meaningful `T0` for supports.
    - Real-fixture multi-material E2E (GCode-level T-change assertion on a real 3MF, not synthetic) requires both the support_filament routing AND a fixture with a meaningful per-region extruder differential preserved through the loader.
    - Other object-level metadata keys (e.g., `nozzle_diameter`, `bed_temperature`) are not extracted; add to the allowlist when a consumer needs them.

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

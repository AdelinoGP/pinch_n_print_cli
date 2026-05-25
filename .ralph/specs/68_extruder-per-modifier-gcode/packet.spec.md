---
status: draft
packet: 68_extruder-per-modifier-gcode
task_ids:
  - TASK-208
  - TASK-209
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
depends_on:
  - 67_3mf-fixture-e2e-hardening (must be status: implemented; provides fixture test harness + RED extruder tests)
  - 56c_threemf-negative-and-support-subtype-routing (must be status: implemented; provides support paint-segmentation piggyback and modifier_volume population)
  - 64_paint-native-migration (must be status: implemented; provides host-native paint pipeline)
  - 51_regionmap-paint-semantic-aware (must be status: implemented; TASK-181 provides paint_overrides infrastructure and overlay_resolved)
unblocks: []
---

# Packet Contract: 68_extruder-per-modifier-gcode

> Implements full Region Config Modifiers: stamps ALL `config_delta.fields` from 3MF modifier volumes (per `ObjectMesh`) into `RegionPlan.config.extensions`. Replaces the narrow extruder-through-paint workaround with a general config-stamping mechanism that matches OrcaSlicer's `PrintApply::generate_print_object_regions()` behavior. Every `config_delta` key — `extruder`, `fuzzy_skin`, `matrix`, and future extensible keys — becomes a per-region config override. The `extruder` key feeds the existing `required_tool` → `T{n}` GCode emission path. Turns Packet 67's three RED tests GREEN and adds coverage proving non-extruder keys survive end-to-end.

## Goal

Stamp all `config_delta.fields` from overlapping `ModifierVolume` entries into `RegionPlan.config.extensions` during region mapping, so every 3MF modifier part's config keys (`extruder`, `fuzzy_skin`, `matrix`, and extensible) act as per-region config overrides — achieving OrcaSlicer parity for Region Config Modifiers without new IR fields, WIT changes, or pipeline stages.

## Scope Boundaries

In scope: `crates/slicer-host/src/region_mapping.rs` — add a `stamp_modifier_config_deltas()` path in `execute_region_mapping_with_cap()` that, for each region, finds overlapping `ModifierVolume` entries from `MeshIR`, reads their `config_delta.fields`, and stamps non-subtype keys into `RegionPlan.config.extensions` via the existing `overlay_resolved()` mechanism. `crates/slicer-host/src/layer_executor.rs` — add config-extensions-driven `required_tool` fallback so the `extruder` key in config.extensions picks the tool when no paint-derived tool exists. `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` — add GCode-level tests proving `T0`/`T1` tool changes from config-stamped extruder, plus tests proving non-extruder config keys (`fuzzy_skin`) survive the end-to-end config pipeline. `docs/07_implementation_status.md` — register TASK-208 and TASK-209.

Out of scope: WASM guest changes, WIT changes, IR schema changes, adding new typed fields to `RegionPlan` or `ResolvedConfig` (use `extensions` bucket), creating distinct `PrintRegion`-equivalent IR types, per-layer Z-range modifier intervals (every modifier applies globally across all layers), `negative_part` extruder semantics, `modifier_part` fuzzy-skin behavior (already handled by TASK-192a), GCode parse-structured verification (string-search is sufficient).

## Prerequisites and Blockers

- **Packet 67** (`67_3mf-fixture-e2e-hardening`) must be `status: implemented`. Provides fixture test harness + RED extruder tests.
- **Packet 56c** (`56c_threemf-negative-and-support-subtype-routing`) must be `status: implemented`. Provides `modifier_volumes` populated in `MeshIR` with `config_delta.fields`.
- **Packet 64** (`64_paint-native-migration`) must be `status: implemented`. Provides host-native `execute_paint_segmentation`.
- **Packet 51** (`51_regionmap-paint-semantic-aware`) must be `status: implemented`. Provides `overlay_resolved()` and `paint_overrides` infrastructure reused for config stamping.
- Fixtures `resources/bridge_support_enforcers.3mf`, `resources/cube_positive_n_negative.3mf`, and `resources/benchy_4color.3mf` must exist on disk (confirmed by Packet 67).

## Acceptance Criteria

- **AC-1. Given** `cube_positive_n_negative.3mf` is loaded and processed through `execute_region_mapping_with_cap`, **when** the resulting `RegionMapIR.entries[*].plan.config.extensions` is inspected for a region whose `object_id` is the parent of a `negative_part` modifier volume carrying `config_delta.fields["extruder"] = ConfigValue::Int(0)`, **then** `config.extensions["extruder"]` is `ConfigValue::Int(0)`. (The earlier draft of this AC named `bridge_support_enforcers.3mf` + a `support_enforcer` volume; that combination is unsatisfiable because `support_enforcer` is filtered by the locked Subtype filter per `PrintApply.cpp:590-594`. The AC text now names the actually-stamped subtype and fixture.) | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd config_delta_extruder_stamped_into_extensions -- --exact --nocapture`

- **AC-2. Given** a partial-pipeline synthetic harness builds a `RegionMapIR` with two `RegionPlan` entries carrying `config.extensions["extruder"] = ConfigValue::Int(0)` and `ConfigValue::Int(1)` respectively, paired with a `PerimeterIR` whose two regions key onto those plans, **when** `assemble_ordered_entities` runs against the staged blackboard, **then** the resulting `ordered_entities` carry both `region_key.region_id == 0` and `region_key.region_id == 1` — which `gcode_emit` later writes as `T0` and `T1` lines (the partial-pipeline path is the locked harness because the full WASM/GCode harness adds no signal beyond the `region_id` differential exercised here). | `cargo test -p slicer-host --test layer_executor_tdd extruder_synthetic_t0_t1_emission -- --exact --nocapture`

- **AC-Filter. Given** `bridge_support_enforcers.3mf` is processed through `execute_region_mapping_with_cap`, **when** `RegionMapIR.entries[*].plan.config.extensions` is inspected, **then** NO entry contains the `extruder` key — `support_enforcer` and `support_blocker` subtypes are excluded from `stamp_modifier_config_deltas` per OrcaSlicer parity (`PrintApply.cpp:590-594`). This AC reuses Packet 67's `AC-Mod-4` test verbatim; it is a permanent cross-packet regression guard. Packet 68 MUST NOT regress this test. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd support_enforcer_config_delta_not_stamped -- --exact --nocapture`

- **AC-3. Given** `benchy_4color.3mf` is processed through the full pipeline, **when** a `modifier_part` volume's `config_delta.fields` contains `"fuzzy_skin" = ConfigValue::String("external")` alongside `"extruder" = ConfigValue::Int(0)`, **then** both keys appear together in `RegionPlan.config.extensions` for at least one region (proving non-extruder keys survive the stamp). (The earlier draft named `bridge_support_enforcers.3mf` + `fuzzy_skin = "all"`; the only volumes in that fixture carrying both keys are `support_enforcer`, which is filtered by the locked Subtype filter, so the AC was unsatisfiable. `benchy_4color.3mf` has a non-filtered `modifier_part` carrying both keys.) | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd config_delta_non_extruder_key_survives -- --exact --nocapture`

- **AC-4. Given** `cube_positive_n_negative.3mf` is processed (negative_part with extruder=0), **when** region mapping runs, **then** the negative_part modifier_volume extruder key does **not** alter polygon output — `apply_negative_part_subtract` is geometry-only and config stamping on regions unaffected by negative_part is benign. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd negative_part_extruder_does_not_affect_subtract -- --exact --nocapture`

- **AC-5. Given** Packet 56c regression suites run after the region_mapping change, **when** `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd` executes, **then** all 10 synthetic tests remain GREEN (synthetic fixtures don't carry non-subtype config_delta keys — the default `ConfigDelta { fields: HashMap::new() }` stamps no extensions). | `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd`

- **AC-6. Given** the GCode emitter receives a layer with multiple regions carrying different `extruder` values in config.extensions, **when** `cargo test -p slicer-host --test gcode_emit_tdd` runs, **then** existing tool-change tests stay GREEN. | `cargo test -p slicer-host --test gcode_emit_tdd`

- **AC-7. Given** clippy is the lint gate, **when** final commit CI runs, **then** `cargo clippy --workspace -- -D warnings` is green. | `cargo clippy --workspace -- -D warnings`

- **AC-8. Given** this packet's doc registration step completes, **when** `docs/07_implementation_status.md` is inspected, **then** the file contains exactly two lines matching `TASK-208.*68_extruder-per-modifier-gcode` or `TASK-209.*68_extruder-per-modifier-gcode`. | `rg -c 'TASK-20[89].*68_extruder-per-modifier-gcode' docs/07_implementation_status.md` expecting `2`

- **AC-9. Given** this packet's doc registration step completes, **when** `docs/04_host_scheduler.md` is inspected, **then** the file mentions modifier config delta stamping in the region-mapping section. | `rg -q 'modifier.*config.delta' docs/04_host_scheduler.md; [ $? -eq 0 ] && echo PASS || echo FAIL`

## Negative Test Cases

- **AC-N1. Given** a modifier_volume with `config_delta.fields` containing only `"subtype"` (no other keys), **when** region mapping stamps config, **then** `RegionPlan.config.extensions` contains **no** non-subtype keys — the `"subtype"` key is excluded from stamping. | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd subtype_only_modifier_stamps_no_extensions -- --exact --nocapture`

- **AC-N2. Given** two overlapping modifier volumes with conflicting `config_delta.fields["extruder"]` values (extruder=0 and extruder=1), **when** region mapping stamps, **then** the higher-priority modifier's value wins (last writer based on overlay_resolved semantics). | `cargo test -p slicer-host --test threemf_fixture_e2e_tdd conflicting_extruder_modifier_priority_wins -- --exact --nocapture`

## Verification

- `cargo test -p slicer-host --test threemf_fixture_e2e_tdd` — all tests GREEN: pre-existing GREEN from Packet 67 + Packet 67's 3 RED tests (`modifier_part_stamps_extruder_into_extensions`, `modifier_part_stamps_fuzzy_skin_into_extensions`, `negative_part_stamps_extruder_into_extensions`) turned GREEN by `stamp_modifier_config_deltas` + the AC-Filter regression guards (`support_enforcer_config_delta_not_stamped`, `support_blocker_config_delta_not_stamped`) stay GREEN (subtype filter enforced).
- `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd` — 56c regression (must stay GREEN).
- `cargo clippy --workspace -- -D warnings` — lint gate.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `ModifierVolume.config_delta`, `ConfigDelta.fields`, `RegionPlan.config.extensions`, `ConfigValue` enum variants.
- `docs/04_host_scheduler.md` — region-mapping placement in prepass ordering, paint-semantic-aware config overlay flow (delegate SUMMARY).
- `docs/01_system_architecture.md` — `extensions` bucket contract for unknown config keys (delegate narrow search).

## Doc Impact Statement (Required)

- `docs/07_implementation_status.md` — append TASK-208 and TASK-209 rows after TASK-207. Verify: `rg -c 'TASK-208.*68_extruder-per-modifier-gcode' docs/07_implementation_status.md` → 1; `rg -c 'TASK-209.*68_extruder-per-modifier-gcode' docs/07_implementation_status.md` → 1.
- `docs/02_ir_schemas.md` — no changes. `ConfigDelta`, `RegionPlan.config.extensions`, and `ModifierVolume` already documented.
- `docs/04_host_scheduler.md` — add one sentence noting that modifier config delta stamping occurs in region-mapping after per-object config resolution. Verify: `rg -q 'modifier.*config.delta' docs/04_host_scheduler.md`.
- `docs/01_system_architecture.md` — no changes needed; `extensions` bucket contract is already documented.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` — `generate_print_object_regions()` at line 1049: canonical OrcaSlicer function that merges modifier volume config deltas into PrintRegion config and creates distinct regions when config differs. This packet replicates the config-stamping behavior (not the region-creation behavior).
- `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp` — `region_config_from_model_volume()` at line ~1100: how OrcaSlicer merges parent region config with modifier volume overrides.
- `OrcaSlicerDocumented/src/libslic3r/Print.hpp` — `PrintRegion::m_config` field and `VolumeRegion` struct: data structures carrying per-region config overrides.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

# Task Map: 67_3mf-fixture-e2e-hardening

## Purpose

This packet introduces TASK-208, which is not present in `docs/07_implementation_status.md` at packet-author time. Step 3 of `implementation-plan.md` appends it after the TASK-193 row registered by Packet 56c. This file maps TASK-208 to the implementation steps, the fixtures it exercises, and the packets whose behavior it verifies.

This packet is a hardening-only packet — it adds integration tests for existing functionality. It is the first of a two-packet chain: Packet 67 adds the test harness and RED extruder tests; Packet 68 implements the extruder consumer and turns the RED tests GREEN.

## Task-to-Step Mapping

| TASK ID | Topic | Implementation steps | Deviations addressed | Authoritative docs | OrcaSlicer ref(s) |
|---|---|---|---|---|---|
| TASK-208 | 3MF fixture E2E integration tests: load real on-disk 3MF files through `load_model()` → full pipeline, verify all five subtype consumers (negative_part subtract, support_enforcer/blocker paint emission, modifier_part regression). 12 test functions (11 GREEN, 1 RED). | Step 1 (author test file), Step 2 (regression sweep), Step 3 (doc registration), Step 4 (pre-ceremony verification). | See packet.spec.md Deviations section. | `docs/02_ir_schemas.md` (IR shape refs); `docs/04_host_scheduler.md` (prepass ordering). | None. |

## Deviation Map

| Deviation ID | Title | Registered by step | Closed by step | Owner packet |
|---|---|---|---|---|---|
| D1 | Scope violation: ~170 lines added to model_loader.rs for p:path extension support | — | Post-implementation review | 67 |
| D2 | RED tests used `#[ignore]` instead of `assert!` per spec | — | Post-implementation review (later: AC-R1/R2 withdrawn per D6) | 67 |
| D3 | AC-R2 text demanded GCode assertion; test only checks config_delta.fields | — | Post-implementation review (later: AC-R2 withdrawn per D6) | 67 |
| D4 | TASK-205 collision with pre-existing Packet 65 entry; renumbered to TASK-208 | — | Post-implementation review | 67 |
| D5 | Packet slug mismatch: folder says 67, frontmatter said 57; downstream "58" vs "68" | — | Post-implementation review | 67 |
| D6 | AC-R1, AC-R2 withdrawal — OrcaSlicer parity finding (support_enforcer extruder is decorative; replaced by AC-Mod-1..6) | Q-grilling | Re-open of P67 | 67 |
| D7 | Packet 68 design amendment bundled into this change (subtype filter + synthetic AC-2 retarget + cross-packet AC-Filter) | Q-grilling | Re-open of P67 | 67 / 68 |
| D8 | Loader production fix bundled into this packet (~65 lines across model_loader_sidecar.rs, model_loader.rs, main.rs); L-size aggregate accepted with rationale | Q-grilling | Re-open of P67 | 67 |

## Fixture Coverage Map

| Fixture | Subtypes exercised | Tests |
|---------|-------------------|-------|
| `resources/cube_positive_n_negative.3mf` | `normal_part` (2), `negative_part` (1, with transform) | AC-1, AC-2, AC-3, AC-Loader-2, AC-Mod-1 |
| `resources/bridge_support_enforcers.3mf` | `normal_part` (2), `support_enforcer` (3, obj 4), `support_blocker` (3, obj 5) | AC-4, AC-5, AC-8, AC-9, AC-Loader-2, AC-Mod-4, AC-Mod-5, AC-Mod-6 |
| `resources/benchy_4color.3mf` | `normal_part` (1), `modifier_part` (1) | AC-6, AC-7, AC-Loader-2, AC-Mod-2, AC-Mod-3 |

## Cross-Packet Dependencies

| Dependency | Direction | Note |
|---|---|---|
| Packet 56 (`56_threemf-sidecar-parser`) | This packet depends on | Provides `parse_3mf_sidecar` and `PartSubtype` enum consumed by `load_model()`. This packet extends the sidecar parser to surface object-scoped metadata (D8). |
| Packet 56b (`56b_threemf-modifier-part-ir-routing`) | This packet depends on | Provides `resolve_object` branching and `ObjectMesh.modifier_volumes` population. Tests verify modifier_part regression. |
| Packet 56c (`56c_threemf-negative-and-support-subtype-routing`) | This packet depends on | Provides `apply_negative_part_subtract` and support paint-segmentation piggyback. Tests verify both consumers end-to-end from disk fixtures. |
| Packet 64 (`64_paint-native-migration`) | This packet depends on | Provides host-native `execute_paint_segmentation` with `union_paint_regions_at_harvest` parameter. Tests call this function directly. |
| Packet 68 (`68_extruder-per-modifier-gcode`) | This packet unblocks AND amends | Adds the bridge-fixture parity guards (AC-Mod-4/5/6) and the RED gates (AC-Mod-1/2/3) for `stamp_modifier_config_deltas`. Bundles a P68 design amendment for the ENFORCER/BLOCKER subtype filter and AC-2 synthetic retarget (D7). P68's new AC-Filter reuses Packet 67's AC-Mod-4 as a cross-packet regression guard. |

## Notes for Implementer

- This packet has a bounded production code change (D1 + D8) — most edits are still test code, but the loader fix touches `model_loader_sidecar.rs`, `model_loader.rs`, and `main.rs`.
- All three 3MF fixtures exist on disk and are read-only. Tests use `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/<name>.3mf")` for path resolution.
- Three RED tests (AC-Mod-1/2/3) intentionally fail with documented `assert!` messages citing `stamp_modifier_config_deltas` (Packet 68) as the resolver. AC-R1/AC-R2 are withdrawn (D6) — their test bodies are deleted.
- No `cargo test --workspace` is needed — the regression sweep covers Packet 56/56b/56c suites plus clippy.
- The test file imports `load_model`, `execute_paint_segmentation`, `execute_region_mapping_with_cap`, plus helpers `build_execution_plan`, `ExecutionPlan`, `ExecutionPlanRequest` from `slicer_host`. See the existing `use slicer_host::...` blocks in `threemf_fixture_e2e_tdd.rs` and `region_mapping_tdd.rs`.
- `region_map_for_fixture` helper is the canonical entry for AC-Mod-1..5: loads the fixture, runs paint segmentation, calls `execute_region_mapping_with_cap` with an empty `ExecutionPlan` and empty `paint_semantic_configs`, returns the resulting `RegionMapIR`.

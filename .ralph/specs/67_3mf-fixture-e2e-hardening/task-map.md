# Task Map: 57_3mf-fixture-e2e-hardening

## Purpose

This packet introduces TASK-205, which is not present in `docs/07_implementation_status.md` at packet-author time. Step 3 of `implementation-plan.md` appends it after the TASK-193 row registered by Packet 56c. This file maps TASK-205 to the implementation steps, the fixtures it exercises, and the packets whose behavior it verifies.

This packet is a hardening-only packet — it adds integration tests for existing functionality. It makes zero production code changes. It is the first of a two-packet chain: Packet 57 adds the test harness and RED extruder tests; Packet 58 implements the extruder consumer and turns the RED tests GREEN.

## Task-to-Step Mapping

| TASK ID | Topic | Implementation steps | Deviations addressed | Authoritative docs | OrcaSlicer ref(s) |
|---|---|---|---|---|---|
| TASK-205 | 3MF fixture E2E integration tests: load real on-disk 3MF files through `load_model()` → full pipeline, verify all five subtype consumers (negative_part subtract, support_enforcer/blocker paint emission, modifier_part regression). 11 test functions (9 GREEN, 2 RED). | Step 1 (author test file), Step 2 (regression sweep), Step 3 (doc registration), Step 4 (pre-ceremony verification). | None. | `docs/02_ir_schemas.md` (IR shape refs); `docs/04_host_scheduler.md` (prepass ordering). | None. |

## Deviation Map

| Deviation ID | Title | Registered by step | Closed by step | Owner packet |
|---|---|---|---|---|---|
| (none new) | This packet is test-only. No deviations. | — | — | — |

## Fixture Coverage Map

| Fixture | Subtypes exercised | Tests |
|---------|-------------------|-------|
| `resources/cube_positive_n_negative.3mf` | `normal_part` (2), `negative_part` (1, with transform) | AC-1, AC-2, AC-3 |
| `resources/bridge_support_enforcers.3mf` | `normal_part` (2), `support_enforcer` (3, obj 4), `support_blocker` (3, obj 5) | AC-4, AC-5, AC-8, AC-N1, AC-R1, AC-R2 |
| `resources/benchy_4color.3mf` | `normal_part` (1), `modifier_part` (3) | AC-6, AC-7 |

## Cross-Packet Dependencies

| Dependency | Direction | Note |
|---|---|---|
| Packet 56 (`56_threemf-sidecar-parser`) | This packet depends on | Provides `parse_3mf_sidecar` and `PartSubtype` enum consumed by `load_model()`. Tests verify sidecar parsing via `modifier_volumes` population. |
| Packet 56b (`56b_threemf-modifier-part-ir-routing`) | This packet depends on | Provides `resolve_object` branching and `ObjectMesh.modifier_volumes` population. Tests verify modifier_part regression. |
| Packet 56c (`56c_threemf-negative-and-support-subtype-routing`) | This packet depends on | Provides `apply_negative_part_subtract` and support paint-segmentation piggyback. Tests verify both consumers end-to-end from disk fixtures. |
| Packet 64 (`64_paint-native-migration`) | This packet depends on | Provides host-native `execute_paint_segmentation` with `union_paint_regions_at_harvest` parameter. Tests call this function directly. |
| Packet 58 (`68_extruder-per-modifier-gcode`) | This packet unblocks | The two RED tests in this packet document expected extruder behavior. Packet 58 implements the consumer and turns them GREEN. |

## Notes for Implementer

- This packet is test-only. Zero production files are modified.
- All three 3MF fixtures exist on disk and are read-only. Tests use `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/<name>.3mf")` for path resolution.
- The two RED tests (AC-R1, AC-R2) intentionally fail. They use `assert!` on conditions that are not yet true. Test names include `_extruder_` and comments document the RED status.
- No `cargo test --workspace` is needed — the regression sweep covers Packet 56/56b/56c suites plus clippy.
- The test file can import `load_model` via `use slicer_host::model_loader::load_model` (or equivalent public path — verify at Step 1 via FACT dispatch).
- `execute_paint_segmentation` requires `Arc<MeshIR>`, `Arc<SurfaceClassificationIR>`, `Arc<LayerPlanIR>`, and a `bool`. For tests that only need paint region output (AC-4/AC-5), the implementer may need to produce placeholder `SurfaceClassificationIR` and `LayerPlanIR` — or find the minimal pipeline setup that produces these IRs from `load_model()` output.
- If producing `SurfaceClassificationIR`/`LayerPlanIR` from `load_model()` requires running additional prepass stages, the implementer should scope those as helper functions within the test file (not as production code changes).

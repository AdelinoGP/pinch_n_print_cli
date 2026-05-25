# Requirements: 67_3mf-fixture-e2e-hardening

## Problem Statement

Packets 56, 56b, and 56c implemented the full 3MF subtype consumer pipeline: sidecar parsing → `resolve_object` routing → `negative_part` subtract → `support_enforcer`/`support_blocker` paint emission → `modifier_part` fuzzy skin. However, all consumer-behavior tests are IR-level synthetic — they build `MeshIR`, `ModifierVolume`, and `SliceIR` structs in memory (`threemf_subtypes_synthetic_e2e_tdd.rs`). No integration test loads a real on-disk 3MF file through `load_model()` and verifies the full pipeline end-to-end.

This gap matters because:
1. **Transform baking** — the synthetic tests use identity transforms; real 3MF files have per-component transforms (`cube_positive_n_negative.3mf` has X-11.1 Y-11.9 offset on the negative cube). A transform bug in `resolve_object` would not be caught by synthetic tests.
2. **Sidecar parsing → modifier_volume wiring** — the full `model_settings.config` → `ObjectSidecarInfo` → `ModifierVolume` path is only tested for classification (56's `threemf_sidecar_classification_tdd.rs`), not for consumer behavior.
3. **Multi-object 3MF files** — `bridge_support_enforcers.3mf` has two objects with different support subtypes; no test verifies that each object's `modifier_volumes` are correctly partitioned.
4. **Duplicate part IDs** — `bridge_support_enforcers.3mf` has part id=3 appearing twice per object (two support enforcer/blocker instances); the parser's handling of duplicate keys needs test coverage.
5. **Extruder metadata gap** — `config_delta.fields["extruder"]` is parsed from sidecar metadata but no downstream consumer reads it. This packet adds RED tests documenting the expected behavior so Packet 68 can turn them GREEN.

This packet (67) adds `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` with 12 tests (11 GREEN, 1 RED) loading three real 3MF fixtures from `resources/`.

## Task ID

- **TASK-208** — 3MF fixture E2E integration tests: load real on-disk 3MF files through `load_model()` → full pipeline, verify all five subtype consumers end-to-end. 12 test functions (11 GREEN for existing functionality, 1 RED documenting pending extruder behavior for Packet 68).

## In Scope

- **Write:**
  - `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` — NEW. Integration tests loading `cube_positive_n_negative.3mf`, `bridge_support_enforcers.3mf`, `benchy_4color.3mf` from `resources/` through `load_model()` + full pipeline. Assertions on `SliceIR` polygon area, `PaintRegionIR` semantic entries, `ModifierVolume` metadata, multi-object partitioning, duplicate ID handling.
  - `docs/07_implementation_status.md` — append TASK-208 row.

- **Read-only:**
  - `crates/slicer-host/src/model_loader.rs` — `load_model()` public API (line 145). Informational only.
  - `crates/slicer-host/src/paint_segmentation.rs` — `execute_paint_segmentation()` entry point (line 253). Informational.
  - `crates/slicer-host/src/negative_part_subtract.rs` — `apply_negative_part_subtract()` (line 20). Informational.
  - `resources/cube_positive_n_negative.3mf` — negative_part fixture with component transforms.
  - `resources/bridge_support_enforcers.3mf` — support_enforcer + support_blocker fixture with two objects.
  - `resources/benchy_4color.3mf` — modifier_part regression fixture.

## Out of Scope

- Creating or modifying any 3MF fixture file (all three fixtures exist on disk).
- Any change to production source files (`crates/slicer-host/src/**`, `crates/slicer-ir/`, `crates/slicer-core/`).
- Implementing extruder GCode consumption (Packet 68).
- Adding `PaintValue::ToolIndex` emission (Packet 68).
- Per-region extruder tool-change GCode emission (Packet 68).
- Creating `support_blocker` fixture (already exists in `bridge_support_enforcers.3mf`).
- `<assemble>` / `<plate>` section parsing.
- WIT changes, SDK changes, macros changes.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `ModifierVolume`, `ConfigDelta`, `PaintRegionIR`, `SemanticRegion`, `PaintSemantic`, `SliceIR`, `SlicedRegion` shapes (delegate narrow search per field).
- `docs/04_host_scheduler.md` — prepass stage ordering for test setup (delegate SUMMARY).
- `docs/08_coordinate_system.md` — scaled integer units for area tolerance (±0.005 mm²).

## OrcaSlicer Reference Obligations

None. This packet is test-only. No OrcaSlicer parity is required — the tests verify host-native Rust pipeline behavior against known fixtures.

## Acceptance Summary (references ACs by ID)

- **AC-1 through AC-9** — Nine GREEN tests covering: negative_part subtract via full pipeline, transform baking, metadata population, support_enforcer paint emission, support_blocker paint emission, modifier_part regression, no-negative no-op, multi-object partitioning, duplicate part ID handling.
- **AC-N1** — Missing fixture path returns `Err(ModelLoadError)` without panicking.
- **AC-Loader-1, AC-Loader-2** — GREEN tests for the object-metadata loader fix (D8): sidecar parser surfaces object-scoped `<metadata>` into `ObjectSidecarInfo.object_metadata`; `load_model` populates `ObjectMesh.config.data` from the allowlist `extruder`/`enable_support`/`support_type`.
- **AC-Mod-1, AC-Mod-2, AC-Mod-3** — RED until Packet 68. Each asserts that a modifier_volume's `config_delta` key is stamped into at least one `RegionMapIR.entries[*].plan.config.extensions` after `execute_region_mapping_with_cap`. Resolved by Packet 68's `stamp_modifier_config_deltas`.
- **AC-Mod-4, AC-Mod-5** — GREEN OrcaSlicer-parity regression guards: no `support_enforcer`/`support_blocker` config_delta key reaches `RegionPlan.config.extensions` (per `PrintApply.cpp:590-594`).
- **AC-Mod-6** — GREEN paint-segmentation parity guard: `SupportEnforcer` `SemanticRegion.value` is always `PaintValue::Flag(_)`, never `PaintValue::ToolIndex(_)`.
- **AC-R1, AC-R2** — WITHDRAWN per D6 (test bodies deleted from `threemf_fixture_e2e_tdd.rs`). Both were premised on the OrcaSlicer-divergent claim that `support_enforcer` `extruder` propagates to a tool change.

## Verification Commands

| Command | Delegation hint | Expected |
|---------|----------------|----------|
| `cargo test -p slicer-host --test threemf_fixture_e2e_tdd` | FACT pass/fail per test | 14 GREEN, 3 RED (AC-Mod-1/2/3) with expected assertion messages |
| `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` | FACT pass/fail | All GREEN (56 regression + new AC-Loader-1) |
| `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd` | FACT pass/fail | All GREEN (56c regression) |
| `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` | FACT pass/fail | All GREEN (56b regression) |
| `cargo clippy --workspace -- -D warnings` | FACT pass/fail | Clean |

## Step Completion Expectations

None — cross-step invariants are adequately expressed by per-step preconditions/postconditions in `implementation-plan.md`.

## Packet-Specific Context Discipline Notes

This packet is test-only — zero production files are modified. The test file imports `load_model` from `slicer_host::model_loader` and exercises existing host functions. No sub-agent needs to read OrcaSlicer source, WIT, SDK, macros, or generated code. Read-only context is limited to confirming function signatures and IR type shapes.

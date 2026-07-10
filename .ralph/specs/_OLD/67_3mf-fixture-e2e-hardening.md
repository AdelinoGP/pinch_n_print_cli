---
status: implemented
packet: 67_3mf-fixture-e2e-hardening
task_ids:
  - TASK-208
---

# 67_3mf-fixture-e2e-hardening

## Goal

Add integration tests in `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` that load real on-disk 3MF files (`cube_positive_n_negative.3mf`, `bridge_support_enforcers.3mf`, `benchy_4color.3mf`) through `load_model()` → full pipeline, asserting that: negative_part reduces per-layer polygon area, support_enforcer and support_blocker emit `PaintRegionIR` entries, modifier_part fuzzy-skin is intact (regression), modifier_volumes carry correct subtype/extruder metadata, duplicate part IDs don't panic, and models without negative parts skip subtract. **Adds 8 new tests** covering (a) the loader fix that populates `ObjectConfig.data` from sidecar object-scoped metadata (AC-Loader-1, AC-Loader-2) and (b) the OrcaSlicer-parity modifier propagation contract for Packet 68 (AC-Mod-1..6: 3 RED tests gate Packet 68's `stamp_modifier_config_deltas`, 3 GREEN parity guards catch the failure modes where Packet 68 over-stamps enforcer/blocker or where someone re-wires the divergent paint-segmentation extruder routing). **Withdraws AC-R1 and AC-R2** — both were premised on the OrcaSlicer-divergent claim that `support_enforcer` `extruder` field propagates to a tool change. See Deviations (D6).

## Problem Statement

Packets 56, 56b, and 56c implemented the full 3MF subtype consumer pipeline: sidecar parsing → `resolve_object` routing → `negative_part` subtract → `support_enforcer`/`support_blocker` paint emission → `modifier_part` fuzzy skin. However, all consumer-behavior tests are IR-level synthetic — they build `MeshIR`, `ModifierVolume`, and `SliceIR` structs in memory (`threemf_subtypes_synthetic_e2e_tdd.rs`). No integration test loads a real on-disk 3MF file through `load_model()` and verifies the full pipeline end-to-end.

This gap matters because:
1. **Transform baking** — the synthetic tests use identity transforms; real 3MF files have per-component transforms (`cube_positive_n_negative.3mf` has X-11.1 Y-11.9 offset on the negative cube). A transform bug in `resolve_object` would not be caught by synthetic tests.
2. **Sidecar parsing → modifier_volume wiring** — the full `model_settings.config` → `ObjectSidecarInfo` → `ModifierVolume` path is only tested for classification (56's `threemf_sidecar_classification_tdd.rs`), not for consumer behavior.
3. **Multi-object 3MF files** — `bridge_support_enforcers.3mf` has two objects with different support subtypes; no test verifies that each object's `modifier_volumes` are correctly partitioned.
4. **Duplicate part IDs** — `bridge_support_enforcers.3mf` has part id=3 appearing twice per object (two support enforcer/blocker instances); the parser's handling of duplicate keys needs test coverage.
5. **Extruder metadata gap** — `config_delta.fields["extruder"]` is parsed from sidecar metadata but no downstream consumer reads it. This packet adds RED tests documenting the expected behavior so Packet 68 can turn them GREEN.

This packet (67) adds `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` with 12 tests (11 GREEN, 1 RED) loading three real 3MF fixtures from `resources/`.

## Architecture Constraints

- **Coordinate system**: Scaled integer units (1 unit = 100 nm). All area assertions use ±0.005 mm² tolerance for Clipper2 rounding. No direct mm-to-unit conversion needed — `load_model()` produces world-space coordinates in the correct unit system.
- **Bounded production code changes**: Originally specified as test-only; in practice, two scoped production fixes are now in scope: (a) the `p:path` external-`.model` parser in `model_loader.rs` (D1) and (b) the object-metadata loader fix across `model_loader_sidecar.rs`, `model_loader.rs`, and `main.rs` (D8). Zero edits to `crates/slicer-ir/` or `crates/slicer-core/`.
- **Public API surface**: Tests call only functions already marked `pub` on the host crate. No `pub(crate)` internals accessed.
- **Fixture immutability**: All three 3MF fixtures are read-only. Tests do not write or modify fixture files.
- **RED test discipline**: The three RED tests (AC-Mod-1, AC-Mod-2, AC-Mod-3) MUST fail with the specific assertion documented in each test body, not with panics, unrelated errors, or missing symbols. `#[should_panic]` is not used — each test uses `assert!` on the expected (currently unfulfilled) condition. All three turn GREEN once Packet 68 lands `stamp_modifier_config_deltas` (with the ENFORCER/BLOCKER subtype filter per D7).
- **No WASM**: No guest WASM is involved in these tests. Host-native pipeline only.

## Data and Contract Notes

- `load_model(path)` returns `Result<MeshIR, ModelLoadError>`. `MeshIR.objects` is `Vec<ObjectMesh>`. Each `ObjectMesh` has `modifier_volumes: Vec<ModifierVolume>`.
- `ModifierVolume.config_delta.fields` is `HashMap<String, ConfigValue>`. Access via `.get("subtype")` returns `Option<&ConfigValue>`.
- `PaintRegionIR.per_layer` is `HashMap<u32, LayerPaintMap>`. `LayerPaintMap.semantic_regions` is `HashMap<PaintSemantic, Vec<SemanticRegion>>`.
- `SemanticRegion.value` is `PaintValue`. Current modifier-volume entries use `PaintValue::Flag(true)`; the RED test asserts `PaintValue::ToolIndex(u32)` which requires a `match` or `if let`.
- `SliceIR.regions[i].polygons` is `Vec<ExPolygon>`. Area computation uses `polygon.area()` from `slicer_core` (scaled integer units → mm²).

## Locked Assumptions and Invariants

1. All three 3MF fixtures exist at `resources/` relative to workspace root and are valid, parseable 3MF files.
2. `load_model()` is the correct entry point for loading 3MF from disk; it internally calls `load_3mf` → `parse_3mf_sidecar` → `parse_3mf_model_xml` → `resolve_object`.
3. Tests run as integration tests (`crates/slicer-host/tests/`) and have access to `slicer_host` public API via `use slicer_host::...`.
4. RED tests fail with an assertion message, not a panic or compilation error. The test function compiles and runs; it just asserts a condition that isn't true yet.
5. No production code changes. This packet does not touch any `src/` file.
6. The three RED tests (AC-Mod-1, AC-Mod-2, AC-Mod-3) are documented as such in their banner comments and assertion messages, citing `stamp_modifier_config_deltas` (Packet 68) as the resolver. AC-Mod-4/5/6 messages cite the OrcaSlicer parity contract at `PrintApply.cpp:590-594` and `paint_segmentation.rs:416`.
7. `ObjectMesh.config.data` is now populated for 3MF inputs (was empty before this packet). Most consumers read it indirectly via the `object_config:<id>:<key>` seed that `main.rs` injects into `config_source` before `resolve_per_object_configs`. Direct `obj.config.data` reads are also valid; the only callers found at design time were synthetic-IR test constructors that build their own ObjectMesh with `data: HashMap::new()` — those are unaffected.

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|
| Fixtures change on disk (user modifies them) → tests break. | Tests assert specific subtype counts and metadata values. If fixtures change, test failures are explicit and point to the changed assertion. |
| `load_model()` is slow for large fixtures. | `benchy_4color.3mf` and `cube_positive_n_negative.3mf` are small (< 1 MB). `bridge_support_enforcers.3mf` has PNG thumbnails but the 3MF parsing ignores non-XML entries. Tests should complete in < 5 seconds each. |
| RED tests might be confusing in CI (they intentionally fail). | Test names include `_extruder_` prefix; comments in test body explain RED status. CI should be configured to allow known RED test failures. |
| Duplicate part id=3 behavior is unspecified — test may need updating if the parser is changed. | The test asserts "does not panic" and "at least N modifier_volumes exist" — loose enough to accommodate either supersede or accumulate behavior. |
| Component-level tests (not full scheduler) may miss integration issues. | Full scheduler E2E is tested by `benchy_4color_modifier_part_e2e_tdd.rs` and `benchy_painted_e2e_tdd.rs`. This packet's tests are complementary — they focus on the parse→route→consume chain that synthetic tests skip. |

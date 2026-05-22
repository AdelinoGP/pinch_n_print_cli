# Design: 67_3mf-fixture-e2e-hardening

## Controlling Code Paths

### Test entry points (public APIs exercised by tests)

- `slicer_host::model_loader::load_model(path: &Path) -> Result<MeshIR, ModelLoadError>` — loads 3MF from disk. Public, callable from integration tests.
- `slicer_host::paint_segmentation::execute_paint_segmentation(mesh_ir, surface_classification_ir, layer_plan_ir, union_paint_regions_at_harvest) -> Result<Arc<PaintRegionIR>, PaintSegmentationError>` — host-native paint segmentation (Packet 64). Callable from integration tests.
- `slicer_host::negative_part_subtract::apply_negative_part_subtract(slice_ir: &mut SliceIR, modifier_volumes: &[ModifierVolume])` — per-layer subtract (Packet 56c). Callable from integration tests.
- `slicer_host::region_mapping::execute_region_mapping(...)` — region mapping for paint_overrides flow (Packet 51).

### Test fixture data paths

| Fixture | Objects | Key subtypes | Tests using it |
|---------|---------|-------------|----------------|
| `resources/cube_positive_n_negative.3mf` | 1 composite (id=4) from 3 parts | parts: 2x normal_part (Cylinder, Cone), 1x negative_part (Cube with transform X-11.1 Y-11.9) | AC-1, AC-2, AC-3 |
| `resources/bridge_support_enforcers.3mf` | 2 objects (id=4, id=5) on one plate | obj4: 1x normal_part + 3x support_enforcer (part id=3 duplicated); obj5: 1x normal_part + 3x support_blocker (part id=3 duplicated) | AC-4, AC-5, AC-8, AC-N1, AC-R1 |
| `resources/benchy_4color.3mf` | 1 object with modifier parts | 1x normal_part + 3x modifier_part | AC-6, AC-7 |

### State before this packet

- `load_model()` loads 3MF from disk path — already implemented (`model_loader.rs:145`).
- All five subtype consumers exist and pass synthetic IR-level tests.
- No test loads a real 3MF file and verifies the full pipeline.
- `config_delta.fields["extruder"]` is populated but no consumer reads it.

### After this packet

- `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` exists with 11 tests.
- 9 GREEN tests exercise the full pipeline from disk 3MF → consumer behavior.
- 2 RED tests document expected extruder behavior for Packet 68.
- No production code changes.

## Neighboring Tests / Fixtures

- `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — Packet 56c's IR-level synthetic tests (682 lines, 10 tests). This packet's tests are complementary — they test the same consumers but through the real `load_model()` path with transform baking and sidecar parsing.
- `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — Packet 56's parser suite. Must stay GREEN (regression sweep).
- `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` — Packet 56b's fixture E2E. Must stay GREEN.

## Architecture Constraints

- **Coordinate system**: Scaled integer units (1 unit = 100 nm). All area assertions use ±0.005 mm² tolerance for Clipper2 rounding. No direct mm-to-unit conversion needed — `load_model()` produces world-space coordinates in the correct unit system.
- **No production code changes**: This packet is test-only. Zero edits to `crates/slicer-host/src/`, `crates/slicer-ir/`, `crates/slicer-core/`.
- **Public API surface**: Tests call only functions already marked `pub` on the host crate. No `pub(crate)` internals accessed.
- **Fixture immutability**: All three 3MF fixtures are read-only. Tests do not write or modify fixture files.
- **RED test discipline**: The two RED tests (AC-R1, AC-R2) MUST fail with the specific assertion documented in their test body, not with panics, unrelated errors, or missing symbols. `#[should_panic]` is not used — tests use `assert!` on the expected (currently unfulfilled) condition.
- **No WASM**: No guest WASM is involved in these tests. Host-native pipeline only.

## Selected Approach (Locked Decisions)

| Decision | Locked choice | Justification |
|---|---|---|
| Test framework | Standard `#[test]` functions in `crates/slicer-host/tests/`. No custom test harness. | Matches existing pattern in `threemf_sidecar_classification_tdd.rs`, `benchy_4color_modifier_part_e2e_tdd.rs`. |
| Fixture path resolution | `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/<name>.3mf")` | Matches existing pattern. Works with `cargo test` from workspace root or crate directory. |
| Pipeline test depth | Call `load_model()` then individual host functions directly (`execute_paint_segmentation`, `apply_negative_part_subtract`). Do NOT run the full scheduler/executor. | Tests should be fast and deterministic. Full scheduler adds ~30s overhead and scheduler state setup complexity. Component-level integration is sufficient to catch transform/parsing bugs. |
| Negative_part area verification | Compute polygon area before and after subtract. Assert reduction > 0 inside extent, bit-identical outside extent. | Same approach as `threemf_subtypes_synthetic_e2e_tdd.rs`. |
| RED test assertion style | Plain `assert!` on the desired condition. Test name prefixed with comment `// RED — passes after Packet 68`. | Clear documentation. `cargo test` output shows "FAILED" not "ok". |
| Duplicate part ID handling | Test that the loader does not panic. Document the actual behavior (supersede or accumulate). | The fixture has real-world duplicate IDs; the test hardens against regressions without prescribing the resolution strategy. |
| Extruder RED tests scope | AC-R1 asserts `PaintValue::ToolIndex` on `SemanticRegion`. AC-R2 asserts `T0`/`T1` in GCode (requires full pipeline — may be a larger test). | AC-R1 is a focused IR-level assertion. AC-R2 is an end-to-end GCode assertion — if too complex for this packet, it can be downgraded to a `// TODO` comment in the test file. |

## Rejected Alternatives

| Alternative | Reason rejected |
|---|---|
| Run full scheduler/executor for each test | Too slow (~30s per test). Component-level integration (load_model + call specific functions) is equally effective for catching transform/parsing bugs. |
| Use `#[should_panic]` for RED tests | `should_panic` hides the specific failure reason. Explicit `assert!` on the desired condition produces a clear failure message. |
| Add a shared test helper crate for fixture loading | Over-engineering for 3 tests. Each test builds its path inline (matches existing pattern). |
| Include GCode-level assertions in this packet | GCode emission requires `LayerCollectionIR` assembly which requires full scheduler execution. Deferred to Packet 68 which also implements the extruder consumer path. |

## Code Change Surface

Primary file this packet creates:

1. `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` — NEW. ~400 lines. Imports `load_model`, `execute_paint_segmentation`, `apply_negative_part_subtract` from `slicer_host`. 11 test functions.
2. `docs/07_implementation_status.md` — append TASK-208 row.

## Read-Only Context the Implementer Needs

| Path | Lines | Purpose |
|---|---|---|
| `crates/slicer-host/src/model_loader.rs` | narrow read at `load_model` (line 145) | Confirm public function signature and return type. |
| `crates/slicer-host/src/paint_segmentation.rs` | narrow read at `execute_paint_segmentation` (line 253) | Confirm 4-param signature. |
| `crates/slicer-host/src/negative_part_subtract.rs` | full (63 lines) | Confirm function signature and behavior. |
| `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` | search for area comparison patterns | Reuse area assertion approach (±0.005 mm² tolerance). |
| `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` | search for `Path::new(env!("CARGO_MANIFEST_DIR"))` | Reuse fixture path resolution pattern. |

## Out-of-Bounds Files (must not be loaded directly)

- All `crates/slicer-host/src/` files except those listed above for signature confirmation.
- `crates/slicer-ir/` — IR shapes are already documented in `docs/02_ir_schemas.md`.
- `crates/slicer-macros/`, `crates/slicer-sdk/` — no SDK or macro involvement.
- `wit/**`, `crates/slicer-host/src/wit_host.rs`, `dispatch.rs` — WIT clean.
- `OrcaSlicerDocumented/**` — no OrcaSlicer parity requirements.
- `target/`, `Cargo.lock`, generated code.

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
6. The two RED tests are documented as such in the test file with a `// RED — passes after Packet 68` comment.

## Risks and Tradeoffs

| Risk | Mitigation |
|---|---|
| Fixtures change on disk (user modifies them) → tests break. | Tests assert specific subtype counts and metadata values. If fixtures change, test failures are explicit and point to the changed assertion. |
| `load_model()` is slow for large fixtures. | `benchy_4color.3mf` and `cube_positive_n_negative.3mf` are small (< 1 MB). `bridge_support_enforcers.3mf` has PNG thumbnails but the 3MF parsing ignores non-XML entries. Tests should complete in < 5 seconds each. |
| RED tests might be confusing in CI (they intentionally fail). | Test names include `_extruder_` prefix; comments in test body explain RED status. CI should be configured to allow known RED test failures. |
| Duplicate part id=3 behavior is unspecified — test may need updating if the parser is changed. | The test asserts "does not panic" and "at least N modifier_volumes exist" — loose enough to accommodate either supersede or accumulate behavior. |
| Component-level tests (not full scheduler) may miss integration issues. | Full scheduler E2E is tested by `benchy_4color_modifier_part_e2e_tdd.rs` and `benchy_painted_e2e_tdd.rs`. This packet's tests are complementary — they focus on the parse→route→consume chain that synthetic tests skip. |

## Context Cost Estimate

- Aggregate: **M** (single new test file, ~400 lines, no production code changes).
- Largest step: Step 1 (authoring the test file with 11 test functions).
- No L-rated step. Reading is limited to confirming 3 function signatures and the existing path-resolution pattern.

## Open Questions

None. All design decisions are locked. The fixture set covers all five subtypes.

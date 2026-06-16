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
| `resources/cube_positive_n_negative.3mf` | 1 composite (id=4) from 3 parts; parent extruder=1 | parts: 2x normal_part (Cylinder, Cone), 1x negative_part (Cube with transform X-11.1 Y-11.9, extruder=0) | AC-1, AC-2, AC-3, AC-Loader-2, AC-Mod-1 |
| `resources/bridge_support_enforcers.3mf` | 2 objects (id=4, id=5) on one plate; obj4 parent extruder=1; obj5 parent extruder=1 + enable_support=1 + support_type=tree(auto) | obj4: 1x normal_part + 3x support_enforcer (part id=3 duplicated, extruder=0); obj5: 1x normal_part + 3x support_blocker (extruder=0) | AC-4, AC-5, AC-8, AC-9, AC-Loader-2, AC-Mod-4, AC-Mod-5, AC-Mod-6 |
| `resources/benchy_4color.3mf` | 1 object (id=3); parent extruder=1 | 1x normal_part + 1x modifier_part (extruder=0, fuzzy_skin=external) | AC-6, AC-7, AC-Loader-2, AC-Mod-2, AC-Mod-3 |

### State before this packet

- `load_model()` loads 3MF from disk path — already implemented (`model_loader.rs:145`).
- All five subtype consumers exist and pass synthetic IR-level tests.
- No test loads a real 3MF file and verifies the full pipeline.
- `config_delta.fields["extruder"]` is populated but no consumer reads it.

### After this packet

- `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` has 17 tests (14 GREEN + 3 RED).
  - 10 pre-existing GREEN tests (AC-1..9, AC-N1) unchanged.
  - 1 new GREEN integration test for the loader fix (AC-Loader-2: `load_model_populates_object_config_data`).
  - 5 new modifier-propagation tests:
    - 3 RED gates for Packet 68's `stamp_modifier_config_deltas`: AC-Mod-1 (negative_part extruder), AC-Mod-2 (modifier_part fuzzy_skin), AC-Mod-3 (modifier_part extruder).
    - 2 GREEN OrcaSlicer-parity guards: AC-Mod-4 (support_enforcer config not stamped), AC-Mod-5 (support_blocker config not stamped).
  - 1 GREEN paint-segmentation parity guard: AC-Mod-6 (`support_enforcer_paint_value_is_flag_not_tool_index`).
- `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` gains 1 new GREEN unit test for the sidecar parser change (AC-Loader-1: `sidecar_parser_extracts_object_metadata`).
- AC-R1 and AC-R2 are **withdrawn** (test bodies deleted) per D6 — both were premised on the OrcaSlicer-divergent contract that `support_enforcer` `extruder` propagates to a tool change.
- ~170 production-code lines previously added to `crates/slicer-host/src/model_loader.rs` for `p:path` support (D1) remain in place.
- ~65 additional production-code lines added across `crates/slicer-host/src/model_loader_sidecar.rs`, `crates/slicer-host/src/model_loader.rs`, and `crates/slicer-host/src/main.rs` for the object-metadata loader fix (D8): sidecar parser now extracts `<metadata>` entries when not inside a `<part>`; `load_model` populates `ObjectConfig.data` from an allowlist of `extruder`/`enable_support`/`support_type`; `main.rs` seeds `object_config:<id>:<key>` into `config_source` mirroring the existing `object_height:<id>` pattern at lines 196-205.
- Packet 68 is amended in the same change (D7): subtype filter row added to `design.md` to exclude `support_enforcer`/`support_blocker` from `stamp_modifier_config_deltas`; AC-2 retargeted to synthetic-IR test; new `AC-Filter` reuses Packet 67's AC-Mod-4 as a permanent cross-packet regression guard.

## Neighboring Tests / Fixtures

- `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — Packet 56c's IR-level synthetic tests (682 lines, 10 tests). This packet's tests are complementary — they test the same consumers but through the real `load_model()` path with transform baking and sidecar parsing.
- `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — Packet 56's parser suite. Must stay GREEN (regression sweep).
- `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` — Packet 56b's fixture E2E. Must stay GREEN.

## Architecture Constraints

- **Coordinate system**: Scaled integer units (1 unit = 100 nm). All area assertions use ±0.005 mm² tolerance for Clipper2 rounding. No direct mm-to-unit conversion needed — `load_model()` produces world-space coordinates in the correct unit system.
- **Bounded production code changes**: Originally specified as test-only; in practice, two scoped production fixes are now in scope: (a) the `p:path` external-`.model` parser in `model_loader.rs` (D1) and (b) the object-metadata loader fix across `model_loader_sidecar.rs`, `model_loader.rs`, and `main.rs` (D8). Zero edits to `crates/slicer-ir/` or `crates/slicer-core/`.
- **Public API surface**: Tests call only functions already marked `pub` on the host crate. No `pub(crate)` internals accessed.
- **Fixture immutability**: All three 3MF fixtures are read-only. Tests do not write or modify fixture files.
- **RED test discipline**: The three RED tests (AC-Mod-1, AC-Mod-2, AC-Mod-3) MUST fail with the specific assertion documented in each test body, not with panics, unrelated errors, or missing symbols. `#[should_panic]` is not used — each test uses `assert!` on the expected (currently unfulfilled) condition. All three turn GREEN once Packet 68 lands `stamp_modifier_config_deltas` (with the ENFORCER/BLOCKER subtype filter per D7).
- **No WASM**: No guest WASM is involved in these tests. Host-native pipeline only.

## Selected Approach (Locked Decisions)

| Decision | Locked choice | Justification |
|---|---|---|
| Test framework | Standard `#[test]` functions in `crates/slicer-host/tests/`. No custom test harness. | Matches existing pattern in `threemf_sidecar_classification_tdd.rs`, `benchy_4color_modifier_part_e2e_tdd.rs`. |
| Fixture path resolution | `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/<name>.3mf")` | Matches existing pattern. Works with `cargo test` from workspace root or crate directory. |
| Pipeline test depth | Most tests call `load_model()` then individual host functions directly (`execute_paint_segmentation`, `apply_negative_part_subtract`). The new AC-Mod-* tests additionally call `execute_region_mapping_with_cap` via the `region_map_for_fixture` helper. Do NOT run the full scheduler/executor or load WASM modules. | Tests should be fast and deterministic. Full scheduler adds ~30s overhead, WASM staleness fragility, and scheduler state setup complexity. Component-level integration is sufficient to catch transform/parsing bugs and to exercise Packet 68's `RegionMapIR.entries[*].plan.config.extensions` contract. |
| Modifier-propagation tests scope | 5 new `AC-Mod-*` tests assert on `RegionMapIR.entries[*].plan.config.extensions` after calling `execute_region_mapping_with_cap` via the `region_map_for_fixture` helper. The 6th (AC-Mod-6) asserts on `PaintRegionIR.per_layer[*].semantic_regions[SupportEnforcer]` after `execute_paint_segmentation`. No full scheduler / WASM module loading required. Three tests are RED until Packet 68 lands `stamp_modifier_config_deltas`; three are GREEN parity guards. | Tight contract on the data Packet 68 actually writes. Cheap to author. Sidesteps the loader-discard-of-parent-extruder gap (which would have blocked any GCode-level T-change assertion on the fixtures). |
| OrcaSlicer parity filter | AC-Mod-4/5/6 codify the contract that `support_enforcer` and `support_blocker` `config_delta` MUST NOT propagate to `RegionPlan.config.extensions` or to `PaintValue::ToolIndex`. Per `OrcaSlicerDocumented/src/libslic3r/PrintApply.cpp:590-594`, those subtypes are excluded from region-config merging. | Catches the failure mode where Packet 68 forgets the subtype filter (D7), or where someone re-wires the divergent paint-segmentation routing that AC-R1 was testing for (D6). |
| Loader fix scope | `crates/slicer-host/src/model_loader_sidecar.rs` extracts object-scoped `<metadata>` entries into a new `ObjectSidecarInfo.object_metadata` field; `crates/slicer-host/src/model_loader.rs` populates `ObjectMesh.config.data` with allowlist `extruder`/`enable_support`/`support_type`; `crates/slicer-host/src/main.rs` seeds `object_config:<id>:<key>` into `config_source` mirroring the existing `object_height:<id>` pattern (lines 196-205). | Smallest unblocking change for any future packet that wants legitimate multi-material GCode coverage. Allowlist discipline mirrors the part-level extraction at `model_loader.rs:761-801`. |
| Test assertion mechanic | All AC-Mod-* tests use the "any entry has the key" pattern — walk `RegionMapIR.entries` and assert at least one (or no entry, for regression guards) has the expected `plan.config.extensions[k] == v`. Tests do NOT reimplement bbox-overlap detection. | Avoids circularity with Packet 68's own bbox logic. The fixture set is small enough that "any entry has the unusual key" is a strong positive signal; AC-Mod-4/5 cover the "stamped on the wrong fixture" failure mode. |
| Negative_part area verification | Compute polygon area before and after subtract. Assert reduction > 0 inside extent, bit-identical outside extent. | Same approach as `threemf_subtypes_synthetic_e2e_tdd.rs`. |
| RED test assertion style | Plain `assert!` on the desired condition. Test name prefixed with comment `// RED — passes after Packet 68`. | Clear documentation. `cargo test` output shows "FAILED" not "ok". |
| Duplicate part ID handling | Test that the loader does not panic. Document the actual behavior (supersede or accumulate). | The fixture has real-world duplicate IDs; the test hardens against regressions without prescribing the resolution strategy. |
| Extruder RED tests scope (HISTORICAL — superseded by Modifier-propagation tests scope row above; AC-R1 and AC-R2 withdrawn per D6) | AC-R1 asserts `PaintValue::ToolIndex` on `SemanticRegion`. AC-R2 asserts `T0`/`T1` in GCode. | Both were premised on OrcaSlicer-divergent contracts (`PrintApply.cpp:590-594` excludes ENFORCER/BLOCKER from region-config merging). Replaced by AC-Mod-1..6. |

## Rejected Alternatives

| Alternative | Reason rejected |
|---|---|
| Run full scheduler/executor for each test | Too slow (~30s per test). Component-level integration (load_model + call specific functions) is equally effective for catching transform/parsing bugs. |
| Use `#[should_panic]` for RED tests | `should_panic` hides the specific failure reason. Explicit `assert!` on the desired condition produces a clear failure message. |
| Add a shared test helper crate for fixture loading | Over-engineering for 3 tests. Each test builds its path inline (matches existing pattern). |
| Include GCode-level assertions in this packet | GCode emission requires `LayerCollectionIR` assembly + full WASM scheduler execution. Sidesteppable via `RegionMapIR.entries[*].plan.config.extensions` assertions which Packet 68 directly modifies. Synthetic-IR GCode assertion deferred to Packet 68 AC-2 (per D7). |
| Reimplement bbox-overlap detection in AC-Mod-* tests | Would reimplement Packet 68's own overlap logic inside the test. Circular — a bug in Packet 68's bbox detection that also exists in the test would pass silently. "Any entry has the key" is sufficient because the stamped keys (`fuzzy_skin="external"`, the modifier's `extruder=0`) are uncommon enough that false-positive risk is near zero, and AC-Mod-4/5 catch "stamped wrong region" via the absence-on-bridge guard. |
| Bundle GCode-level T0+T1 assertion on `bridge_support_enforcers.3mf` | Premised on an OrcaSlicer-divergent contract (D6); `support_enforcer` `extruder` is decorative in OrcaSlicer. Replaced by the AC-Mod-4/5/6 parity guards. |

## Code Change Surface

Files this packet creates or modifies:

1. `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` — extends from 12 tests to 17 tests (14 GREEN + 3 RED). Adds `region_map_for_fixture` helper, `AC-Loader-2` integration test, `AC-Mod-1..6` modifier-propagation tests. Deletes AC-R1 and AC-R2 bodies.
2. `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — adds 1 new test (`AC-Loader-1: sidecar_parser_extracts_object_metadata`).
3. `crates/slicer-host/src/model_loader_sidecar.rs` — production: adds `object_metadata: BTreeMap<String, String>` field to `ObjectSidecarInfo`; parser extracts `<metadata>` entries when `inside_part == false` and `current_object_id.is_some()`. ~25 lines.
4. `crates/slicer-host/src/model_loader.rs` — production: adds `object_metadata_to_config_data` allowlist converter; threads `HashMap<String, ConfigValue>` through the per-item 4-tuple from `parse_3mf_model_xml` → `load_3mf` → `load_model`; populates `ObjectConfig.data` per object. ~70 lines.
5. `crates/slicer-host/src/main.rs` — production: adds `object_config:<id>:<key>` seeding loop mirroring the existing `object_height:<id>` pattern (lines 196-205). ~12 lines.
6. `.ralph/specs/68_extruder-per-modifier-gcode/design.md` — Packet 68 amendment: subtype filter row added; AC-2 retargeted to synthetic-IR test.
7. `.ralph/specs/68_extruder-per-modifier-gcode/packet.spec.md` — Packet 68 amendment: AC-2 retargeted; new `AC-Filter` reusing AC-Mod-4.
8. `docs/07_implementation_status.md` — register TASK-208 (this packet) plus three follow-up TASK rows for downstream gaps (support_filament routing, real-fixture multi-material E2E, extended object metadata keys).

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

## Context Cost Estimate

- Aggregate: **M** (single new test file, ~400 lines, no production code changes).
- Largest step: Step 1 (authoring the test file with 12 test functions: 11 GREEN + 1 RED).
- No L-rated step. Reading is limited to confirming 3 function signatures and the existing path-resolution pattern.

## Open Questions

None. All design decisions are locked. The fixture set covers all five subtypes.

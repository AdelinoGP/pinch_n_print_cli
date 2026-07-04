# Design: 113a-arachne-parity-closures

## Controlling Code Paths

- **Primary code path:** `crates/slicer-core/src/arachne/simplify.rs` (140 LOC) — replace DP with Visvalingam + `calculateExtrusionAreaDeviationError` gate; rename `dp_epsilon` → `visvalingam_area_threshold` in `ArachneParams` (`pipeline.rs:94`) and the WIT `arachne-params` record (`common.wit:32`).
- **Secondary code path:** `modules/core-modules/arachne-perimeters/src/lib.rs` (276 LOC) — `arachne_params_from_config` reads 7 newly-wired config keys with `units_to_mm` conversion.
- **Tertiary code path:** `crates/slicer-core/src/beading/factory.rs` (~250 LOC) — `BeadingFactoryParams` struct + `create_stack` function gain 4 new fields for the 4 unwired factory keys.
- **Test target 1 (new):** `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs` — feeds `cube_4color.3mf` painted facets to `slicer_core::algos::paint_segmentation`; asserts non-overlapping Voronoi partition.
- **Test target 2 (simplified):** `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` — remove "out-of-footprint" narrative, keep gcode-level structural assertions.
- **Test target 3 (tightened):** `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs:626` — exact-match `_with_config` instead of substring match.
- **OrcaSlicer comparison surface:** see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **Atomic rename constraint:** The `dp_epsilon` → `visvalingam_area_threshold` rename is a WIT breaking change to the `arachne-params` record in `common.wit`. The host-service bridge in `slicer-sdk/src/host.rs` and the WIT host impl in `slicer-wasm-host/src/host.rs` must be updated in the same commit. No external consumers exist (the bridge is WASM-internal), so the break is contained.

- **Visvalingam no-op on current input:** `simplify_toolpaths` will hit the `n <= 2` early return on the current 2-junction fragments from P112's `generate_toolpaths`. The Visvalingam port ships the faithful algorithm code-ready; it becomes active when P113b's faithful `connectJunctions` produces multi-junction input. The simplify fixture re-recording is deferred to P113b.

## Code Change Surface

- **Selected approach:** Single-pass algorithm swap in `simplify.rs` + cross-file rename + 4 new `ArachneParams` fields + 3 net-new manifest entries + 4 already-registered keys now read by `arachne_params_from_config` + new unit test on `paint_segmentation` + 3 new negative tests. Each item is a localized edit; no structural refactoring.
- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  - `crates/slicer-core/src/arachne/simplify.rs` (REPLACE DP with VW; ADD `calculate_extrusion_area_deviation_error` helper; same public API)
  - `crates/slicer-core/src/arachne/pipeline.rs` (RENAME `dp_epsilon` → `visvalingam_area_threshold` in `ArachneParams`; ADD 4 new fields: `wall_transition_length`, `wall_transition_angle`, `initial_layer_min_bead_width`, `outer_wall_offset`)
  - `crates/slicer-core/src/beading/factory.rs` (ADD 2 new fields to `BeadingFactoryParams`: `wall_transition_angle`, `initial_layer_min_bead_width`; THREAD through `create_stack`)
  - `crates/slicer-sdk/src/host.rs` (RENAME `dp_epsilon` → `visvalingam_area_threshold` in `ArachneParams` mirror struct; ADD 4 new fields to mirror; UPDATE wasm32 inline WIT record)
  - `crates/slicer-schema/wit/deps/common.wit` (RENAME `dp-epsilon` → `visvalingam-area-threshold` in `arachne-params` record; the 4 factory fields do not flow through WIT — they are `BeadingFactoryParams` only)
  - `crates/slicer-wasm-host/src/host.rs` (UPDATE `generate_arachne_walls` WIT host impl to read renamed field)
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` (ADD 3 net-new entries with `unit = "units"`: `min_central_distance`, `visvalingam_area_threshold`, `min_width`. The 4 already-registered keys gain no new schema entries.)
  - `modules/core-modules/arachne-perimeters/src/lib.rs` (UPDATE `arachne_params_from_config` lines 104-154 to read all 7 keys with `units_to_mm` conversion)
  - `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs` (NEW: 2 tests — `cube_4color_mmu_partition_is_non_overlapping` for AC-4, `cube_4color_mmu_cells_are_disjoint` for AC-N3)
  - `crates/slicer-core/tests/simplify.rs` (NEW test: `simplify_toolpaths_width_weighted_gate_preserves_junctions` for AC-N1)
  - `crates/slicer-core/tests/arachne_pipeline.rs` (NEW test: `arachne_params_defaults_when_keys_absent` for AC-N4)
  - `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` (REPLACE "Bounded deviation" section's out-of-footprint narrative with 2-line note referencing AC-4 + `D-112-MMU-TOPOLOGY`; KEEP "Honesty note" section)
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` (TIGHTEN: line 626 exact-match `_with_config` instead of substring)
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_arachne/` (NEW: directory + `expected_perimeter_ir.json` golden)
  - `docs/DEVIATION_LOG.md` (CLOSE: `D-112-SIMPLIFY-DP`, `D-112-THIN-WALL-WIDENING`. KEEP OPEN: `D-112-MMU-TOPOLOGY` with new "Verification" link to the AC-4 unit test.)
  - `.ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md` (ADD: a "M2 — Real Arachne" section recording the actual `148 files, +13,981/−206` stat)

- **Rejected alternatives:**
  - **Keep DP, add VW as a second pass behind a config flag.** Rejected: violates algorithm faithfulness (two algorithms, not one). The faithful port replaces DP entirely.
  - **Defer Visvalingam to P113b with the topology chain.** Rejected: P113a's Visvalingam is code-ready (the algorithm port is independent of the topology chain). Shipping P113a's Visvalingam and P113b's `connectJunctions` separately means each can be verified independently.
  - **Expose `SliceIR` from `SliceOutcome` for the MMU test.** Rejected: the two-level approach (unit test on `paint_segmentation` + executor wiring smoke) proves the same geometric invariant without production API changes.
  - **Add the `cube_4color_arachne/` fixture by re-running the executor test with a golden-snapshot flag.** Rejected: the fixture is a perimeter_parity gcode golden, not the executor test's gcode dump. The design named a `expected_perimeter_ir.json` golden; record it once, lock it.

## Files in Scope (read + edit)

- `crates/slicer-core/src/arachne/simplify.rs` — primary edit target; replace DP with VW + width-weighted area gate
- `modules/core-modules/arachne-perimeters/src/lib.rs` — secondary edit target; `arachne_params_from_config` reads 7 new keys
- `crates/slicer-core/src/arachne/pipeline.rs` — tertiary; `ArachneParams` rename + 4 new fields

## Read-Only Context

- `crates/slicer-core/src/beading/factory.rs` — read `BeadingFactoryParams` struct (lines 68-115) and `create_stack` (lines 163-213) only
- `crates/slicer-sdk/src/host.rs` — read `ArachneParams` mirror struct (lines 447-478) only
- `crates/slicer-schema/wit/deps/common.wit` — read `arachne-params` record (lines 26-39) only
- `crates/slicer-wasm-host/src/host.rs` — read `generate_arachne_walls` impl (lines 1767-1800) only
- `docs/02_ir_schemas.md` — range-read §"Point3WithWidth" only
- `docs/08_coordinate_system.md` — range-read §"Constant Conversion Table" only
- `docs/12_architecture_gate_metrics.md` — range-read §"Fixture Catalog" only
- `docs/specs/orca-mmu-perimeter-investigation.md` — read full (35 lines)
- `crates/slicer-core/src/algos/paint_segmentation/` — read module structure (find the function that takes painted facets and returns per-color `ExPolygon` sets)

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate parity checks; never load directly
- `target/`, `Cargo.lock`, generated code — never load
- `crates/slicer-core/src/skeletal_trapezoidation/*.rs` — P113b's domain; do not browse
- `crates/slicer-core/src/arachne/{stitch,remove_small}.rs` — not edited by this packet
- `crates/slicer-core/src/arachne/generate_toolpaths.rs` — not edited (P113b's domain)
- `modules/core-modules/arachne-perimeters/src/lib.rs`'s `run_perimeters` body (lines 193-275) — not edited; only `arachne_params_from_config` is touched

## Expected Sub-Agent Dispatches

- "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:248` `calculateExtrusionAreaDeviationError(A, B, C)` formula; return SUMMARY (≤ 200 words: input types, output type, 1-line formula description). No code." — purpose: port the width-weighted area gate faithfully
- "Run `cargo test -p slicer-core --features host-algos --test simplify -- simplify_toolpaths_vertex_count`; return FACT pass/fail." — purpose: validate AC-1
- "Run `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_pipeline_thin_wall_widening`; return FACT pass/fail." — purpose: validate AC-2
- "Run `cargo test -p slicer-core --test paint_segmentation_mmu_partition_tdd`; return FACT pass/fail." — purpose: validate AC-4
- "Run `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_fragments_walls_by_color`; return FACT pass/fail." — purpose: validate AC-5
- "Run `cargo test -p slicer-runtime --test integration -- main_production_entry_path_loads_real_modules_and_calls_live_helpers`; return FACT pass/fail." — purpose: validate AC-6
- "Run `cargo xtask build-guests --check`; return FACT clean / STALE list." — purpose: guest WASM coherence gate
- "Run `cargo clippy --workspace --all-targets -- -D warnings`; return FACT pass/fail." — purpose: clippy gate

## Data and Contract Notes

- **WIT contract change:** The `arachne-params` record in `common.wit:26-39` renames `dp-epsilon: f32` to `visvalingam-area-threshold: f32`. This is a WIT breaking change to the host-service interface, but the interface is unreleased (WASMinternal). The semantic also changes: `dp-epsilon` was a perpendicular-distance threshold in mm; `visvalingam-area-threshold` is a width-weighted area threshold in mm².
- **ArachneParams field additions:** `wall_transition_length: f64`, `wall_transition_angle: f64`, `initial_layer_min_bead_width: f64`, `outer_wall_offset: f64` — all in mm except `wall_transition_angle` which is in degrees (converted to radians at the factory boundary).
- **BeadingFactoryParams field additions:** `wall_transition_angle: f64` (radians) + `initial_layer_min_bead_width: f64` (mm). The other 2 keys (`wall_transition_length`, `outer_wall_offset`) already have factory fields at lines 75 and 92; they are just not threaded from `ArachneParams`.
- **MMU unit test invariant:** Per-color `ExPolygon` sets from `slicer_core::algos::paint_segmentation` must be (a) non-overlapping (intersection empty within `SCALED_EPSILON` ≈ 1 unit tolerance), (b) contained in the model XY bounding box, (c) non-zero area, (d) the union of all per-color cells must cover the full painted face area.
- **No schema bump:** No IR types change; no `CURRENT_SLICE_IR_SCHEMA_VERSION` bump needed.

## Locked Assumptions and Invariants

- The `simplify_toolpaths` public API signature `(lines: Vec<ExtrusionLine>, threshold: f64) -> Vec<ExtrusionLine>` is preserved (the parameter is renamed from `dp_epsilon` to `visvalingam_area_threshold`, but the position/type are unchanged; the semantic changes from distance-mm to area-mm²).
- `ArachneParams::default()` provides sensible defaults for all 4 newly-wired keys + the 3 net-new manifest keys (matching the factory's existing `BeadingFactoryParams::default()` values).
- The Visvalingam port is a no-op on P112's 2-junction input. The `simplify_toolpaths_vertex_count` test fixture continues to pass with the same vertex count (since VW is a no-op on 2-junction input). When P113b's faithful `connectJunctions` produces multi-junction input, the Visvalingam port will actually exercise vertex removal; that fixture re-baseline is P113b's scope.
- The MMU unit test reads `resources/cube_4color.3mf` (already present in the repo from P105). The test must not modify the 3MF.
- The `cube_4color_arachne/` fixture directory's `expected_perimeter_ir.json` is captured by running the arachne wall output on `cube_4color.3mf` once and committing the result. This is a one-time self-captured golden per `D-112-SELFCAPTURED-BASELINES`, not regenerated.

## Risks and Tradeoffs

- **WIT breaking change:** The `dp-epsilon` → `visvalingam-area-threshold` rename breaks the `arachne-params` record. Mitigation: the host-service interface is unreleased (WASM-internal); no external consumers to update. The change is atomic across `common.wit`, `slicer-sdk/src/host.rs`, `slicer-wasm-host/src/host.rs`, and `pipeline.rs::ArachneParams`.
- **Visvalingam no-op on P112 input:** The simplify fixture won't show vertex-count reduction until P113b's `connectJunctions` ships. Mitigation: the algorithm port is correct (verified by code review against OrcaSlicer's `calculateExtrusionAreaDeviationError`); the fixture re-baselines in P113b.
- **MMU unit test does not close `D-112-MMU-TOPOLOGY`:** The AC-4 unit test establishes a geometric partition invariant UPSTREAM of the arachne output. The actual "tens of mm outside the naive per-face footprint" symptom is governed by `arachne-perimeters` output topology (the per-edge 2-junction fragment emission pattern from `generate_toolpaths.rs`). P113b's quad/rib pass + faithful `connectJunctions` changes that emission pattern, so the deviation re-targets to P113b with a different hypothesis. The unit test added here is supporting evidence for the deviation, not a closure.
- **MMU unit test requires `resources/cube_4color.3mf` to be parseable by `slicer_core::algos::paint_segmentation`:** The test feeds the 3MF to `slicer_core::algos::paint_segmentation`. If the 3MF format or the `paint_segmentation` API has changed since P105, the test needs updating. Mitigation: P105's `cube_4color.3mf` is the reference fixture; the test uses the same loading path. The other `cube_4color_*` executor tests in the repo all read the same fixture successfully.
- **`visvalingam_area_threshold` default value:** The current DP default (`dp_epsilon: 0.025` mm) is a distance. The equivalent VW default is an area. There's no direct conversion — the implementer must choose a reasonable area threshold (likely `0.025 * 0.4 = 0.01` mm² based on the typical bead width). Mitigation: the default is a config-time parameter; users can tune it. The test fixture passes with whatever default the implementer chooses (the existing `simplify_toolpaths_vertex_count` test is a no-op on 2-junction input either way).
- **`D-112-MMU-TOPOLOGY` stays open across this packet:** the original 113a draft claimed the deviation would close. It does not. The deviation's "follow-up" column should be updated to point to P113b's quad/rib pass + `connectJunctions` as the target, not to AC-4's unit test.

## Context Cost Estimate

- **Aggregate (sum across all 6 steps):** M
- **Largest single step:** Step 4 (MMU unit test design) — S, requires reading `slicer_core::algos::paint_segmentation/` module structure to find the per-color polygon construction function
- **Highest-risk dispatch:** The `calculateExtrusionAreaDeviationError` SUMMARY — the implementer needs the exact formula to port faithfully; if the SUMMARY is too vague, the ported gate will have a different semantic than OrcaSlicer's. Required return format: SUMMARY (≤ 200 words, formula description with input/output types, no code).

## Open Questions

- [FWD] What is the exact `calculateExtrusionAreaDeviationError` formula? Resolve via SUMMARY dispatch against `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:248` before Step 1 implementation.
- [FWD] Does `BeadingStrategyFactory::create_stack` currently consume `wall_transition_angle` or `initial_layer_min_bead_width`? If not, which strategy should consume them? Resolve by reading `create_stack` (lines 163-213) before Step 2 implementation. If neither strategy consumes them, add a new strategy or extend `WideningBeadingStrategy` to consume `initial_layer_min_bead_width` as a layer-specific `min_output_width` override.
- [FWD] What is the default value for `visvalingam_area_threshold`? Resolve by choosing a reasonable area (mm²) based on the typical bead width and the current DP `dp_epsilon: 0.025` mm default. No `[BLOCK]` questions; all can be resolved mid-implementation.

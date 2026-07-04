---
status: active
packet: 113a-arachne-parity-closures
task_ids: []
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md (M2 follow-up)
context_cost_estimate: M
---

# Packet Contract: 113a-arachne-parity-closures

## Goal

Close 4 Arachne-pipeline deviations and 2 audit findings by implementing the 6 independent S/M items identified in the packet 112 audit: Visvalingam-Whyatt simplification, 7 unwired Arachne config keys, MMU test fix (two-level approach), loader source-guard tightening, `cube_4color_arachne/` fixture directory, and closure-log file-count correction.

## Scope Boundaries

This packet owns the 6 Arachne parity closures that do NOT depend on the synthetic quad/rib topology pass (which lands in P113b). It replaces Douglas-Peucker with OrcaSlicer's width-aware Visvalingam-like simplification, wires the remaining 7 `arachne-perimeters.toml` config keys through `ArachneParams`, fixes the MMU test's geometric invariant by splitting it across two test levels (unit test on `paint_segmentation` for geometry, executor test as wiring smoke), tightens the loader source-guard to pin the exact `_with_config` entry point, creates the missing `cube_4color_arachne/` fixture directory, and corrects the closure-log file-count inaccuracy. The quad/rib topology pass, faithful centrality predicate, per-NODE bead_count, faithful transition marking, and faithful `connectJunctions` belong to P113b.

## Prerequisites and Blockers

- Depends on: packet 112 (`d9466fd7`, `status: implemented` — branch `parity/arachne`) for the existing Arachne pipeline source, fixtures, and host-service bridge. Plan provenance: `docs/specs/perimeter-modules-orca-parity-roadmap.md` §"M2 — Real Arachne" (Phases 10-13, all DONE per T-234) and `docs/DEVIATION_LOG.md` D-112-* residual entries.
- Unblocks: P113b (Arachne topology faithfulness) can begin once P113a ships, since P113b re-validates downstream stages against the topology-changed input.
- Activation blockers: none. All source files exist; no forward-deps. The `resources/cube_4color.3mf` fixture is already present (P105). The 7-step sequence of 113a changes the host-side `simplify.rs`/`pipeline.rs`/`factory.rs` and the WASM module's `arachne_params_from_config`, but the implementation can land with this packet's `active` status flipped after the closure ceremony.

## Acceptance Criteria

- **AC-1.** `simplify_toolpaths` in `crates/slicer-core/src/arachne/simplify.rs` uses Visvalingam-Whyatt area-based vertex removal gated by `calculateExtrusionAreaDeviationError` (width-weighted area deviation from OrcaSlicer's `utils/ExtrusionLine.cpp:248`), replacing the current Douglas-Peucker implementation. The `dp_epsilon` field in `ArachneParams` and the `dp_epsilon` WIT field in the `arachne-params` record are renamed to `visvalingam_area_threshold`. The existing `simplify_toolpaths_vertex_count` test (P112) remains green. | `cargo test -p slicer-core --features host-algos --test simplify -- simplify_toolpaths_vertex_count 2>&1 | tee target/test-output-simplify.log`
- **AC-2.** `arachne_params_from_config` in `modules/core-modules/arachne-perimeters/src/lib.rs` reads 7 config keys via `config.get_float`/`get_int`/`get_bool` with `units_to_mm` conversion for `units`-tagged values: `min_central_distance` (new), `visvalingam_area_threshold` (new — renamed from no-rename; see AC-1), `min_width` (new), `wall_transition_length` (already in manifest, now wired), `wall_transition_angle` (already in manifest, now wired; degrees → radians at factory boundary), `initial_layer_min_bead_width` (already in manifest, now wired), `outer_wall_offset` (already in manifest, now wired). The corresponding `ArachneParams` struct in `crates/slicer-core/src/arachne/pipeline.rs` gains the 4 unwired fields (the 3 net-new ones + `wall_transition_angle`/`initial_layer_min_bead_width` added to `BeadingFactoryParams`). `to_beading_factory_params` threads `wall_transition_length` and `outer_wall_offset` into `BeadingFactoryParams`; `wall_transition_angle` and `initial_layer_min_bead_width` are added to `BeadingFactoryParams` and threaded through `BeadingStrategyFactory::create_stack`. The existing `arachne_pipeline_thin_wall_widening` test (P112) remains green. | `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_pipeline_thin_wall_widening 2>&1 | tee target/test-output-pipeline.log`
- **AC-3.** `arachne_params_from_config` in `modules/core-modules/arachne-perimeters/src/lib.rs` (lines 104-154) contains exactly 7 reads for the keys listed in AC-2: `get_float("min_central_distance")`, `get_float("visvalingam_area_threshold")`, `get_float("min_width")`, `get_float("wall_transition_length")`, `get_float("wall_transition_angle")`, `get_float("initial_layer_min_bead_width")`, `get_float("outer_wall_offset")` — all with `units_to_mm` (or degree→radian for `wall_transition_angle`) conversion. The 3 net-new keys (`min_central_distance`, `visvalingam_area_threshold`, `min_width`) are added to `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` with `unit = "units"`; the 4 already-registered keys (lines 68-127 of the manifest) gain no new schema entries. | `rg -c 'config\.(get_float|get_int|get_bool)\("(min_central_distance|visvalingam_area_threshold|min_width|wall_transition_length|wall_transition_angle|initial_layer_min_bead_width|outer_wall_offset)"' modules/core-modules/arachne-perimeters/src/lib.rs`
- **AC-4.** A NEW unit test in `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs` feeds the `resources/cube_4color.3mf` painted-facet input to `slicer_core::algos::paint_segmentation` and asserts: (a) the returned per-color `ExPolygon` sets form a non-overlapping Voronoi partition of the model's cross-section at a mid-body Z layer, (b) every per-color cell is contained within the model's overall XY bounding box, (c) every per-color cell has a non-zero area. The test does NOT assert that arachne's output extrusion points land inside the partition cells — that property is unrelated to the partition invariants and is governed by AC-5's wiring-smoke test plus `D-112-MMU-TOPOLOGY` (which remains open; see Out of Scope). | `cargo test -p slicer-core --test paint_segmentation_mmu_partition_tdd -- cube_4color_mmu_partition_is_non_overlapping 2>&1 | tee target/test-output-mmu-unit.log`
- **AC-5.** `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` retains its gcode-parsing harness as a wiring smoke test, asserting: 4 distinct tool indices appear, ≥1 mid-body layer has ≥3 per-color outer-wall fragments, every header's extrusion points are finite, and every header's traced length is ≥ 1.0mm. The "Bounded deviation from the classic test's 'self-closure' property" section's narrative about per-color extrusion points landing "tens of mm outside the naively-expected per-face footprint" is REPLACED with a 2-line note: (1) the geometric partition invariant lives in AC-4's unit test on `paint_segmentation`, (2) the upstream "extrusion-points-in-footprint" investigation is tracked separately as `D-112-MMU-TOPOLOGY` and out of scope for this packet. The existing "Honesty note (no OrcaSlicer oracle)" section above it is preserved. | `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_fragments_walls_by_color 2>&1 | tee target/test-output-cube4c.log`
- **AC-6.** `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs:626` (`main_production_entry_path_loads_real_modules_and_calls_live_helpers`) checks `run_src.contains("load_live_modules_for_plan_with_config")` (exact-match the renamed entry point), not the substring `load_live_modules_for_plan` that also matches the base name. | `cargo test -p slicer-runtime --test integration -- main_production_entry_path_loads_real_modules_and_calls_live_helpers 2>&1 | tee target/test-output-loader.log`
- **AC-7.** `crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_arachne/` exists (it does not currently — `D-112-SELFCAPTURED-BASELINES` cites the path but the directory was never created on disk) with a committed `expected_perimeter_ir.json` golden containing the arachne wall output for `resources/cube_4color.3mf`. The golden is captured ONCE by running the arachne wall output against `cube_4color.3mf` and committing the result — it is a self-captured regression baseline per `D-112-SELFCAPTURED-BASELINES`, not an OrcaSlicer oracle. | `test -f crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_arachne/expected_perimeter_ir.json && echo "PRESENT"`
- **AC-8.** `.ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md` gains a "M2 — Real Arachne" section that records the actual commit diff stat: `148 files, +13,981/−206`. The current closure-log has no file-count line at all (the previous "102 files" claim was inaccurate and was already removed); this AC-8 *adds* the count, not *corrects* an existing wrong one. | `rg -q '148 files.*13,981' .ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md && ! rg -q '102 files' .ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md`

## Negative Test Cases

- **AC-N1.** A NEW test `simplify_toolpaths_width_weighted_gate_preserves_junctions` in `crates/slicer-core/tests/simplify.rs` asserts that `simplify_toolpaths` does NOT drop a vertex whose removal would violate the width-weighted area gate: a fixture with three collinear junctions where the middle junction's `calculate_extrusion_area_deviation_error` exceeds the threshold is kept (vertex count unchanged). The test is added as part of this packet. | `cargo test -p slicer-core --features host-algos --test simplify -- simplify_toolpaths_width_weighted_gate_preserves_junctions 2>&1 | tee target/test-output-simplify-neg.log`
- **AC-N2.** `remove_small_lines` does NOT remove any `ExtrusionLine` where `is_closed == true && inset_idx == 0`, regardless of length. The existing `remove_small_lines_all_primary_invariant` test (P112) covers this. | `cargo test -p slicer-core --features host-algos --test remove_small -- remove_small_lines_all_primary_invariant 2>&1 | tee target/test-output-remove-neg.log`
- **AC-N3.** A NEW test `cube_4color_mmu_cells_are_disjoint` in the same `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs` file as AC-4 asserts that two per-color cells from the same face do NOT overlap: their polygon intersection is empty within `SCALED_EPSILON` tolerance. The test is added as part of this packet. | `cargo test -p slicer-core --test paint_segmentation_mmu_partition_tdd -- cube_4color_mmu_cells_are_disjoint 2>&1 | tee target/test-output-mmu-neg.log`
- **AC-N4.** A NEW test `arachne_params_defaults_when_keys_absent` in `crates/slicer-core/tests/arachne_pipeline.rs` asserts that an `ArachneParams` constructed via `arachne_params_from_config` from a config that omits all 7 wired keys falls back to `ArachneParams::default()` values for those keys, not an error. The test is added as part of this packet. | `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_params_defaults_when_keys_absent 2>&1 | tee target/test-output-params-neg.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (CLEAN — this packet edits `arachne-perimeters.toml` and `common.wit` which feed guest WASM)

## Authoritative Docs

- `docs/02_ir_schemas.md` — schema-version contract for additive changes (no schema bump needed for this packet; all changes are within existing types)
- `docs/03_wit_and_manifest.md` — WIT `arachne-params` record structure (range-read §"host-services" + `common.wit` schema)
- `docs/08_coordinate_system.md` — `units_to_mm` conversion (range-read §"Constant Conversion Table" only)
- `docs/12_architecture_gate_metrics.md` — `cube_4color.3mf` fixture catalog (range-read §"Fixture Catalog" only)
- `docs/specs/orca-mmu-perimeter-investigation.md` (from P105, 35 lines) — read full

For each doc, the implementer should range-read or delegate. Do not load any doc > 300 lines in full.

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` — update `D-112-SIMPLIFY-DP` Status to "Closed — 2026-07-03: Visvalingam-Whyatt port landed; width-weighted area gate via `calculateExtrusionAreaDeviationError`" — `rg -q 'D-112-SIMPLIFY-DP.*Closed' docs/DEVIATION_LOG.md`
- `docs/DEVIATION_LOG.md` — update `D-112-THIN-WALL-WIDENING` Status to "Closed — 2026-07-03: `arachne_params_from_config` now reads the 4 previously-unwired keys (`wall_transition_length`, `wall_transition_angle`, `initial_layer_min_bead_width`, `outer_wall_offset`); `wall_transition_angle` and `initial_layer_min_bead_width` added to `BeadingFactoryParams` and threaded through `BeadingStrategyFactory::create_stack`" — `rg -q 'D-112-THIN-WALL-WIDENING.*Closed' docs/DEVIATION_LOG.md`
- `docs/DEVIATION_LOG.md` — `D-112-MMU-TOPOLOGY` Status STAYS "Open". This packet does NOT close the deviation. The unit test added in AC-4 establishes the geometric partition invariant (per-color `ExPolygon` sets from `paint_segmentation` form a non-overlapping Voronoi partition), which is upstream of the arachne output — it is a necessary but not sufficient property. The "extrusion points land outside the naive per-face footprint on painted geometry" flag is a downstream-of-arachne concern, governed by `arachne-perimeters::run_perimeters` + the multi-island/non-convex cell hypothesis from the existing "Bounded deviation" doc comment. The new AC-4 test is added to the deviation's "Verification" column as supporting evidence; the deviation itself remains open and is re-targeted to P113b's quad/rib topology pass + `connectJunctions` (which changes the per-edge 2-junction fragment emission pattern that produced the "tens of mm outside" symptom). — `rg -q 'D-112-MMU-TOPOLOGY.*Open' docs/DEVIATION_LOG.md`
- `.ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md` — add a new section recording the actual commit diff stat: `148 files, +13,981/−206` — `rg -q '148 files.*13,981' .ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:248` — `calculateExtrusionAreaDeviationError(A, B, C)`: width-weighted area deviation formula for the Visvalingam-like simplification gate. The implementer needs the exact formula to port faithfully.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:152` — call site in the simplification loop: how the gate's return value is compared against the area threshold to decide vertex survival.
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:494` — `extract_colored_segments()`: leftmost-arc walk that produces per-color `ExPolygon` Voronoi cells. The unit test's expected polygon topology matches this walk's output shape.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

# Implementation Plan: 113a-arachne-parity-closures

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Visvalingam-Whyatt simplification + `dp_epsilon` → `visvalingam_area_threshold` rename + NEW negative test

- Task IDs:
  - none (M2 follow-up)
- Objective: Replace Douglas-Peucker in `simplify.rs` with Visvalingam-Whyatt area-based vertex removal gated by `calculateExtrusionAreaDeviationError`. Rename `dp_epsilon` → `visvalingam_area_threshold` in `ArachneParams`, WIT `arachne-params` record, SDK mirror, and WIT host impl. ADD a NEW test `simplify_toolpaths_width_weighted_gate_preserves_junctions` to `crates/slicer-core/tests/simplify.rs` that proves the gate's negative case (vertex whose removal would violate the threshold is kept).
- Precondition: OrcaSlicer `calculateExtrusionAreaDeviationError` formula obtained via SUMMARY dispatch.
- Postcondition: `simplify_toolpaths_vertex_count` test green (existing P112 test continues to pass on 2-junction input — VW is a no-op); `simplify_toolpaths_width_weighted_gate_preserves_junctions` test green (NEW); `cargo xtask build-guests --check` CLEAN.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/simplify.rs` (140 LOC, full)
  - `crates/slicer-core/src/arachne/pipeline.rs` — lines 69-116 (ArachneParams struct) only
  - `crates/slicer-core/tests/simplify.rs` — read full (small file, ~80 LOC)
  - `crates/slicer-sdk/src/host.rs` — lines 447-478 (ArachneParams mirror) only
  - `crates/slicer-schema/wit/deps/common.wit` — lines 26-39 (arachne-params record) only
  - `crates/slicer-wasm-host/src/host.rs` — lines 1767-1800 (generate_arachne_walls impl) only
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/simplify.rs` — primary edit target
  - `crates/slicer-core/src/arachne/pipeline.rs` — ArachneParams field rename
  - `crates/slicer-core/tests/simplify.rs` — ADD the new test
  - `crates/slicer-schema/wit/deps/common.wit` — WIT record field rename (with `slicer-sdk/src/host.rs` + `slicer-wasm-host/src/host.rs` as cross-cutting edits)
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - `crates/slicer-core/src/arachne/stitch.rs`, `remove_small.rs`, `generate_toolpaths.rs` — not edited
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — not edited in this step
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:248` `calculateExtrusionAreaDeviationError(A, B, C)`; return SUMMARY (≤ 200 words: input types, output type, formula description). No code." — purpose: port the width-weighted area gate
- Context cost: S
- Authoritative docs:
  - `docs/08_coordinate_system.md` — read §"Constant Conversion Table" only — purpose: units-to-mm conversion for the renamed threshold
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:248` — delegate; never load
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:152` — delegate; never load
- Verification:
  - `cargo test -p slicer-core --features host-algos --test simplify -- simplify_toolpaths_vertex_count 2>&1 | tee target/test-output-simplify.log` — dispatch as FACT pass/fail (AC-1; existing test)
  - `cargo test -p slicer-core --features host-algos --test simplify -- simplify_toolpaths_width_weighted_gate_preserves_junctions 2>&1 | tee target/test-output-simplify-neg.log` — dispatch as FACT pass/fail (AC-N1; NEW test)
  - `cargo xtask build-guests --check` — dispatch as FACT clean / STALE list
- Exit condition: AC-1 + AC-N1 green; guest-WASM CLEAN.

### Step 2: Add 4 new ArachneParams fields + BeadingFactoryParams threading

- Task IDs:
  - none (M2 follow-up)
- Objective: Add `wall_transition_length`, `wall_transition_angle`, `initial_layer_min_bead_width`, `outer_wall_offset` to `ArachneParams`. Thread 2 into `BeadingFactoryParams` (factory already consumes them). Add 2 new `BeadingFactoryParams` fields + strategy-level consumption for `wall_transition_angle` and `initial_layer_min_bead_width`.
- Precondition: Step 1's `dp_epsilon` → `visvalingam_area_threshold` rename green.
- Postcondition: `arachne_pipeline_thin_wall_widening` test green (existing P112 test).
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/arachne/pipeline.rs` — lines 69-116 (ArachneParams) + lines 203-220 (to_beading_factory_params) only
  - `crates/slicer-core/src/beading/factory.rs` — lines 68-115 (BeadingFactoryParams) + lines 163-213 (create_stack) only
  - `crates/slicer-sdk/src/host.rs` — lines 447-478 (ArachneParams mirror) only
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/arachne/pipeline.rs` — add 4 fields
  - `crates/slicer-core/src/beading/factory.rs` — add 2 fields + threading
  - `crates/slicer-sdk/src/host.rs` — add 4 fields to mirror struct + update wasm32 inline WIT
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — not edited in this step (Step 3)
- Expected sub-agent dispatches:
  - "Find which strategy in `BeadingStrategyFactory::create_stack` consumes `wall_transition_angle` and `initial_layer_min_bead_width`; return LOCATIONS (file:line + 1-line role). If neither strategy consumes them, identify which strategy is the natural extension point." — purpose: confirm where to add the consumption
- Context cost: M (4 new fields × 3 files + factory threading)
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — read §"host-services" + `common.wit` schema only — purpose: WIT record structure
- OrcaSlicer refs:
  - None directly (the beading strategy stack is P111's domain; this step only adds fields)
- Verification:
  - `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_pipeline_thin_wall_widening 2>&1 | tee target/test-output-pipeline.log` — dispatch as FACT pass/fail
- Exit condition: AC-2 green (existing test still passes; new fields are unused until Step 3 wires them through `arachne_params_from_config`).

### Step 3: Add 3 net-new manifest entries + 4 already-registered reads + 1 new defaults test in `arachne_params_from_config`

- Task IDs:
  - none (M2 follow-up)
- Objective: Add 3 net-new manifest entries to `arachne-perimeters.toml` (`min_central_distance`, `visvalingam_area_threshold`, `min_width`). Update `arachne_params_from_config` to read all 7 keys (the 4 from Step 2 + the 3 net-new) with `units_to_mm` conversion. ADD a NEW test `arachne_params_defaults_when_keys_absent` in `crates/slicer-core/tests/arachne_pipeline.rs` that proves an `ArachneParams` constructed from a config that omits all 7 wired keys falls back to `ArachneParams::default()`.
- Precondition: Step 2's new ArachneParams fields exist.
- Postcondition: AC-3 grep returns count = 7 for the `arachne_params_from_config` reads AND count = 3 for the net-new manifest entries; `arachne_pipeline_thin_wall_widening` still green (existing test); `arachne_params_defaults_when_keys_absent` (NEW) green.
- Files allowed to read (with line-range hints when relevant):
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — full (small file)
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — lines 104-154 (arachne_params_from_config) only
  - `crates/slicer-core/tests/arachne_pipeline.rs` — read full (small file)
- Files allowed to edit (≤ 3):
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — add 3 net-new entries
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — extend arachne_params_from_config to read 7 keys
  - `crates/slicer-core/tests/arachne_pipeline.rs` — add the new defaults test
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - `crates/slicer-core/src/arachne/pipeline.rs` — not edited
- Expected sub-agent dispatches:
  - None
- Context cost: S
- Authoritative docs:
  - `docs/08_coordinate_system.md` — read §"Constant Conversion Table" only — purpose: units-to-mm conversion
- OrcaSlicer refs:
  - None
- Verification:
  - `rg -c 'config\.(get_float|get_int|get_bool)\("(min_central_distance|visvalingam_area_threshold|min_width|wall_transition_length|wall_transition_angle|initial_layer_min_bead_width|outer_wall_offset)"' modules/core-modules/arachne-perimeters/src/lib.rs` — dispatch as FACT (count must be 7)
  - `rg -c '^\s*\[\[config\.schema\.(min_central_distance|visvalingam_area_threshold|min_width)\]\]' modules/core-modules/arachne-perimeters/arachne-perimeters.toml` — dispatch as FACT (count must be 3)
  - `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_params_defaults_when_keys_absent 2>&1 | tee target/test-output-params-neg.log` — dispatch as FACT pass/fail (AC-N4; NEW test)
  - `cargo xtask build-guests --check` — dispatch as FACT clean / STALE list
- Exit condition: AC-3 + AC-N4 green; guest-WASM CLEAN.

### Step 4: MMU unit test on `paint_segmentation` (NEW test file)

- Task IDs:
  - none (M2 follow-up)
- Objective: Create a NEW test file `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs` with TWO tests — `cube_4color_mmu_partition_is_non_overlapping` (AC-4) and `cube_4color_mmu_cells_are_disjoint` (AC-N3) — that feed `resources/cube_4color.3mf` painted facets to `slicer_core::algos::paint_segmentation` and assert non-overlapping Voronoi partition + containment in model XY bounding box + non-zero area per cell.
- Precondition: Steps 1-3 green.
- Postcondition: `cube_4color_mmu_partition_is_non_overlapping` + `cube_4color_mmu_cells_are_disjoint` tests pass.
- Files allowed to read (with line-range hints when > 300 lines):
  - `crates/slicer-core/src/algos/paint_segmentation/` — read module structure (find the function that takes painted facets and returns per-color `ExPolygon` sets); read only the public API surface
  - `crates/slicer-core/tests/` — read an existing test file as a template (e.g., `stitch.rs` at 100 LOC) for structure
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs` — NEW file with 2 tests
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` — Step 5 edits this
- Expected sub-agent dispatches:
  - "Find the function in `crates/slicer-core/src/algos/paint_segmentation/` that takes painted facets and returns per-color `ExPolygon` sets; return LOCATIONS (file:line + 1-line role). Note its public API (function name, input types, output types)." — purpose: identify the entry point for the unit test
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:494` `extract_colored_segments()` behavior; return SUMMARY (≤ 200 words: input types, output types, the per-color partition's expected properties: non-overlapping, shared bisector boundaries). No code." — purpose: confirm the unit test's expected partition invariants match OrcaSlicer's behavior
- Context cost: S
- Authoritative docs:
  - `docs/specs/orca-mmu-perimeter-investigation.md` (35 lines) — read full — purpose: per-color Voronoi partition behavior
  - `docs/12_architecture_gate_metrics.md` — read §"Fixture Catalog" only — purpose: `cube_4color.3mf` painted-face layout
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:494` — delegate; never load
- Verification:
  - `cargo test -p slicer-core --test paint_segmentation_mmu_partition_tdd -- cube_4color_mmu_partition_is_non_overlapping 2>&1 | tee target/test-output-mmu-unit.log` — dispatch as FACT pass/fail (AC-4)
  - `cargo test -p slicer-core --test paint_segmentation_mmu_partition_tdd -- cube_4color_mmu_cells_are_disjoint 2>&1 | tee target/test-output-mmu-neg.log` — dispatch as FACT pass/fail (AC-N3)
- Exit condition: AC-4 + AC-N3 green.

### Step 5: Simplify executor MMU test + tighten loader source-guard + create fixture dir + add closure-log section

- Task IDs:
  - none (M2 follow-up)
- Objective: (a) Edit `cube_4color_arachne.rs`: REPLACE the "Bounded deviation from the classic test's 'self-closure' property" section's narrative about per-color extrusion points landing "tens of mm outside the naively-expected per-face footprint" with a 2-line note referencing AC-4's unit test + `D-112-MMU-TOPOLOGY`. KEEP the "Honesty note (no OrcaSlicer oracle)" section verbatim. (b) Tighten `live_module_loading_tdd.rs:626` (NOT line 613 — the substring check is at line 626) to exact-match `_with_config`. (c) Create `cube_4color_arachne/` fixture directory with `expected_perimeter_ir.json` golden (the directory does not currently exist on disk). (d) ADD a "M2 — Real Arachne" section to `.ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md` recording the actual commit diff stat: `148 files, +13,981/−206`.
- Precondition: Step 4's MMU unit test green.
- Postcondition: AC-5 + AC-6 + AC-7 + AC-8 all green.
- Files allowed to read (with line-range hints when relevant):
  - `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` (544 LOC) — range-read §"Honesty note" + §"Bounded deviation..." comments only
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` — line 626 only
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/` — list existing fixture directories only
  - `.ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md` — read full (32 lines)
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` — replace the "Bounded deviation" section's out-of-footprint narrative; KEEP "Honesty note" section
  - `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs` — tighten line 626 guard
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_arachne/` — NEW directory
  - `.ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md` — ADD a "M2 — Real Arachne" section
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - `crates/slicer-core/src/arachne/pipeline.rs` — not edited
- Expected sub-agent dispatches:
  - "List the existing fixture directories under `crates/slicer-runtime/tests/fixtures/perimeter_parity/`; return LOCATIONS (one entry per dir, 1-line description)." — purpose: confirm `cube_4color_arachne/` is missing
  - "Capture the arachne wall output for `resources/cube_4color.3mf` as `expected_perimeter_ir.json`; save to `crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_arachne/`. This is a one-time self-captured golden per `D-112-SELFCAPTURED-BASELINES`, not regenerated. Use the existing arachne_perimeter_parity test infrastructure as a template." — purpose: create the golden fixture
- Context cost: S
- Authoritative docs:
  - None (this step is administrative)
- OrcaSlicer refs:
  - None
- Verification:
  - `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_fragments_walls_by_color 2>&1 | tee target/test-output-cube4c.log` — dispatch as FACT pass/fail (AC-5)
  - `cargo test -p slicer-runtime --test integration -- main_production_entry_path_loads_real_modules_and_calls_live_helpers 2>&1 | tee target/test-output-loader.log` — dispatch as FACT pass/fail (AC-6)
  - `test -f crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_arachne/expected_perimeter_ir.json && echo "PRESENT"` — dispatch as FACT (AC-7)
  - `rg -q '148 files.*13,981' .ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md && ! rg -q '102 files' .ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md` — dispatch as FACT (AC-8: both grep must succeed)
- Exit condition: AC-5 + AC-6 + AC-7 + AC-8 green.

### Step 6: Close 2 deviations in DEVIATION_LOG.md (D-112-MMU-TOPOLOGY stays open) + workspace gate

- Task IDs:
  - none (M2 follow-up)
- Objective: Close `D-112-SIMPLIFY-DP` and `D-112-THIN-WALL-WIDENING` in `docs/DEVIATION_LOG.md`. `D-112-MMU-TOPOLOGY` STAYS OPEN with updated "Verification" link to the AC-4 unit test. Run the workspace gate.
- Precondition: Steps 1-5 all green.
- Postcondition: 2 deviations closed; 1 deviation stays open with re-targeted follow-up; workspace gate green; packet ready for `status: implemented`.
- Files allowed to read (with line-range hints when > 300 lines):
  - `docs/DEVIATION_LOG.md` (50 lines) — read full
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md` — update 2 Status columns to Closed; update `D-112-MMU-TOPOLOGY` Status to "Open" with re-targeted follow-up
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/` — delegate; never load
  - All other source files — not edited
- Expected sub-agent dispatches:
  - "Run `cargo xtask test --workspace --summary 2>&1 | tee target/test-output.log`; return FACT pass/fail + summary line + count." — purpose: workspace gate (per CLAUDE.md §"Test Discipline" workspace-test exception)
- Context cost: S
- Authoritative docs:
  - None (this step is administrative)
- OrcaSlicer refs:
  - None
- Verification:
  - `rg -q 'D-112-SIMPLIFY-DP.*Closed' docs/DEVIATION_LOG.md` — dispatch as FACT (AC Doc Impact)
  - `rg -q 'D-112-THIN-WALL-WIDENING.*Closed' docs/DEVIATION_LOG.md` — dispatch as FACT (AC Doc Impact)
  - `rg -q 'D-112-MMU-TOPOLOGY.*Open' docs/DEVIATION_LOG.md` — dispatch as FACT (AC Doc Impact — must remain Open)
  - `rg -q '148 files.*13,981' .ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md` — dispatch as FACT (AC Doc Impact)
  - `cargo xtask test --workspace --summary 2>&1 | tee target/test-output.log` — dispatch as FACT pass/fail
- Exit condition: 2 deviation closures verified; 1 deviation stays open; workspace gate green.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Visvalingam + WIT rename |
| Step 2 | M | 4 new ArachneParams fields + factory threading |
| Step 3 | S | 7 manifest entries + 7 config reads |
| Step 4 | S | MMU unit test on paint_segmentation |
| Step 5 | S | Simplify executor + tighten guard + fixture dir + closure-log |
| Step 6 | S | Close 3 deviations + workspace gate |

Sum: M aggregate; no step is L.

## Packet Completion Gate

- All 6 steps complete.
- Every step exit condition is met.
- Packet acceptance criteria green (AC-1 through AC-8, AC-N1 through AC-N4 each verified by their pipe-suffixed command).
- 2 deviations closed in `docs/DEVIATION_LOG.md` (`D-112-SIMPLIFY-DP`, `D-112-THIN-WALL-WIDENING`); 1 deviation stays open (`D-112-MMU-TOPOLOGY`, re-targeted to P113b's quad/rib pass + `connectJunctions`).
- Closure-log "M2 — Real Arachne" section added with the actual `148 files, +13,981/−206` stat.
- `cargo xtask test --workspace --summary` green.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` optionally updated to mark P113a as a follow-on to the M2 plan (T-234 → P113a follow-up), but this is documentation hygiene not a packet gate.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`.
- Confirm packet-level verification commands are green.
- Record any remaining packet-local risk explicitly before moving to `status: implemented`.
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson for future spec-packet-generator runs.

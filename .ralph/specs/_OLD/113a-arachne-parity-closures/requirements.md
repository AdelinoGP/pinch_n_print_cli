# Requirements: 113a-arachne-parity-closures

## Packet Metadata

- Grouped task IDs: **none** (the M2 plan `docs/specs/perimeter-modules-orca-parity-roadmap.md` is the real provenance for this work; no `TASK-###` exists in `docs/07_implementation_status.md` for arachne follow-ups per the packet-112 handoff). Crosswalk: M2 plan Phase 12 items T-220..T-227 + Phase 13 items T-230..T-234 are all DONE; this packet implements the residuals of T-226 (DP → VW) and T-230 (config wiring of keys that were registered-but-dead) plus the audit findings A (MMU test), B (loader guard), C (fixture directory), D (closure-log accuracy).
- Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md` §"M2 — Real Arachne" + `docs/DEVIATION_LOG.md` lines 40-50 (D-112-SIMPLIFY-DP, D-112-THIN-WALL-WIDENING, D-112-MMU-TOPOLOGY, D-112-PROPAGATION-ADAPT, D-112-CENTRALITY-ADAPT, D-112-SELFCAPTURED-BASELINES)
- Packet status: `active`
- Aggregate context cost: `M` (6 steps, largest is Step 5 unit-test design at S)

## Problem Statement

The packet 112 audit (commit `d9466fd7`) identified 4 residual `D-112-*` deviations and 4 audit findings that block true OrcaSlicer parity. The implementation produced a from-first-principles adaptation (Douglas-Peucker, depth-floor centrality, folded transition marking, per-EDGE bead count, unwired `arachne_params_from_config` keys, weakened MMU test assertion, no MMU `cube_4color_arachne/` fixture directory, closure-log with no commit diff stat) rather than a literal port of OrcaSlicer's algorithms or a clean manifest↔config contract. The adaptations are functionally equivalent for the tested fixtures but are not algorithm-faithful — a future reviewer comparing the code to OrcaSlicer's source would see divergence. This packet implements 6 of the 8 total residual items: the 6 that do NOT depend on the synthetic quad/rib topology pass (which is L-effort and deferred to P113b). These 6 items are genuine parity closures: they replace the adaptation with OrcaSlicer's algorithm (DP → VW), wire the 4 config keys that P111 registered in the manifest but P112 left unread in `arachne_params_from_config` (plus add 3 net-new keys for parameters that have no manifest entry at all), fix the MMU executor test to be honest about what it asserts, add a `paint_segmentation` unit test for the geometric partition invariant (necessary but not sufficient to close `D-112-MMU-TOPOLOGY` — that deviation remains open and re-targets to P113b's quad/rib pass), tighten the loader source-guard to pin the exact renamed entry point, create the missing `cube_4color_arachne/` fixture directory + golden, and add the commit diff stat to the closure-log. The remaining 2 items (quad/rib topology pass, faithful centrality/bead_count/transitions/connectJunctions) belong to P113b.

## In Scope

- Replace Douglas-Peucker simplification in `crates/slicer-core/src/arachne/simplify.rs` with OrcaSlicer's Visvalingam-Whyatt area-based removal gated by `calculateExtrusionAreaDeviationError` (width-weighted area deviation). Rename `dp_epsilon` → `visvalingam_area_threshold` in `ArachneParams`, `BeadingParams`, the WIT `arachne-params` record, and the SDK `ArachneParams` mirror.
- Add 4 new fields to `ArachneParams` in `crates/slicer-core/src/arachne/pipeline.rs`: `wall_transition_length`, `wall_transition_angle`, `initial_layer_min_bead_width`, `outer_wall_offset`. Thread `wall_transition_length` and `outer_wall_offset` into `BeadingFactoryParams` (factory already consumes them at `factory.rs:178,200`). Add `wall_transition_angle` and `initial_layer_min_bead_width` to `BeadingFactoryParams` and thread through `BeadingStrategyFactory::create_stack` into whichever strategy reads them.
- Add 3 net-new manifest entries to `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`: `min_central_distance`, `visvalingam_area_threshold`, `min_width`. The 4 above-named keys (registered in P111) need no new schema entries; they are already in the manifest at lines 68-127. Update `arachne_params_from_config` in `modules/core-modules/arachne-perimeters/src/lib.rs` (lines 104-154) to read all 7 keys with `units_to_mm` conversion for `units`-tagged values, falling back to `ArachneParams::default()` for missing keys.
- Create a NEW unit test file `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs` that feeds `resources/cube_4color.3mf` painted facets to `slicer_core::algos::paint_segmentation` and asserts: (a) per-color `ExPolygon` sets form a non-overlapping Voronoi partition, (b) every cell is contained in the model XY bounding box, (c) every cell has non-zero area, (d) disjointness within `SCALED_EPSILON` tolerance.
- Simplify `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs`: KEEP the "Honesty note (no OrcaSlicer oracle)" section (it's accurate and required by `D-112-SELFCAPTURED-BASELINES`). REPLACE the "Bounded deviation from the classic test's 'self-closure' property" section's narrative about per-color extrusion points landing "tens of mm outside the naively-expected per-face footprint" with a 2-line note: (1) geometric partition invariant lives in AC-4's unit test, (2) the upstream "extrusion-points-in-footprint" investigation is tracked separately as `D-112-MMU-TOPOLOGY` and out of scope for this packet.
- Tighten `crates/slicer-runtime/tests/integration/live_module_loading_tdd.rs:626` source-guard from `contains("load_live_modules_for_plan")` (substring) to `contains("load_live_modules_for_plan_with_config")` (exact-match the renamed entry point).
- CREATE `crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_arachne/` with a committed `expected_perimeter_ir.json` golden (the directory does not currently exist on disk; `D-112-SELFCAPTURED-BASELINES` cites the path but no entry exists).
- ADD a "M2 — Real Arachne" section to `.ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md` recording the actual commit diff stat: `148 files, +13,981/−206`. (The current closure-log has no file-count line at all — this AC-8 *adds* the count, not *corrects* an existing wrong one.)
- Add 3 NEW tests: `simplify_toolpaths_width_weighted_gate_preserves_junctions` in `crates/slicer-core/tests/simplify.rs` (AC-N1), `cube_4color_mmu_cells_are_disjoint` in `crates/slicer-core/tests/paint_segmentation_mmu_partition_tdd.rs` (AC-N3), `arachne_params_defaults_when_keys_absent` in `crates/slicer-core/tests/arachne_pipeline.rs` (AC-N4).
- Close 2 deviations in `docs/DEVIATION_LOG.md`: `D-112-SIMPLIFY-DP`, `D-112-THIN-WALL-WIDENING` (residual). `D-112-MMU-TOPOLOGY` STAYS OPEN — the unit test added by AC-4 is supporting evidence for the deviation, not a closure (the deviation is governed by `arachne-perimeters` output topology, not by the partition invariant; P113b's quad/rib pass + `connectJunctions` is the target).

## Out of Scope

- Quad/rib topology pass (`makeRib` synthetic edge insertion) — P113b
- Faithful `filter_central` using `dR < dD * sin(angle/2)` on quad/rib topology — P113b
- Per-NODE bead_count (move from per-EDGE) — P113b
- Faithful `generateTransitionMids`/`applyTransitions` from `transition_ratio` on quad graph — P113b
- Faithful `connectJunctions` (stitch per-edge junction fans into full ExtrusionLines) — P113b
- Re-validation of `stitch_extrusions`/`simplify_toolpaths`/`remove_small_lines` against multi-junction input from `connectJunctions` — P113b
- Re-baselining the 8 self-captured regression fixtures for the faithful-algorithm output — P113b
- Closing `D-112-MMU-TOPOLOGY` — stays open, re-targeted to P113b's quad/rib + `connectJunctions`
- Closing `D-112-CENTRALITY-ADAPT` and `D-112-PROPAGATION-ADAPT` — P113b (these are not algorithm-faithful until the quad/rib topology exists)
- `D-112-SELFCAPTURED-BASELINES` — accepted limitation, no OrcaSlicer binary
- `D-112-HOSTSVC-BRIDGE`, `D-112-WALL-GENERATOR-SELECT`, `D-112-TOOLPATH-WIDTH` — already closed by P112
- New config keys beyond the 7 unwired in P111's registered surface
- Spiral-vase + non-planar — orthogonal sibling roadmaps
- Overhang pipeline — closed by P106/P107
- Classic-perimeters edits — M1 frozen
- ADR-0033 (Algorithm Faithfulness as OrcaSlicer Parity Definition) — was claimed as a P113a dependency in the original packet draft, but no such ADR is in the active ADRs (`docs/adr/`) and the user has not asked for one. Removed.

## Authoritative Docs

- `docs/02_ir_schemas.md` — range-read §"Point3WithWidth" only (90 lines); purpose: confirm no schema bump needed for this packet
- `docs/03_wit_and_manifest.md` — range-read §"host-services" + `common.wit` schema only (40 lines); purpose: WIT `arachne-params` record structure for the `dp_epsilon` → `visvalingam_area_threshold` rename
- `docs/08_coordinate_system.md` — range-read §"Constant Conversion Table" only (30 lines); purpose: `units_to_mm` conversion for the 7 wired config keys
- `docs/12_architecture_gate_metrics.md` — range-read §"Fixture Catalog" only (50 lines); purpose: `cube_4color.3mf` painted-face layout
- `docs/specs/orca-mmu-perimeter-investigation.md` (from P105) — read full (35 lines); purpose: per-color Voronoi partition behavior (non-overlapping cells with shared bisectors)

All other docs are not authoritative for this packet.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:248` — `calculateExtrusionAreaDeviationError(A, B, C)`: width-weighted area deviation formula for the Visvalingam-like simplification gate.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:152` — call site in the simplification loop: how the gate's return value is compared against the area threshold to decide vertex survival.
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:494` — `extract_colored_segments()`: leftmost-arc walk that produces per-color `ExPolygon` Voronoi cells. The unit test's expected polygon topology matches this walk's output shape.

## Acceptance Summary

Reference Acceptance Criteria by ID; do not copy them.

- Positive cases: `AC-1` (VW + width-weighted area gate; `dp_epsilon` → `visvalingam_area_threshold`), `AC-2` (4 `ArachneParams` field additions + `BeadingFactoryParams` threading for `wall_transition_angle`/`initial_layer_min_bead_width`), `AC-3` (3 net-new manifest entries + 4 already-registered keys now read by `arachne_params_from_config`), `AC-4` (NEW `paint_segmentation_mmu_partition_tdd.rs` unit test asserts non-overlapping Voronoi partition), `AC-5` (executor test stays as wiring smoke; "Bounded deviation" section reframes without removing the "Honesty note"), `AC-6` (loader guard tightened to exact-match `_with_config` at line 626), `AC-7` (NEW `cube_4color_arachne/` fixture directory + `expected_perimeter_ir.json` golden), `AC-8` (closure-log gains a "M2 — Real Arachne" section with the actual `148 files, +13,981/−206` stat).
- Negative cases: `AC-N1` (NEW `simplify_toolpaths_width_weighted_gate_preserves_junctions` test — width-weighted area gate preserves junctions that violate threshold), `AC-N2` (existing `remove_small_lines_all_primary_invariant` test preserved), `AC-N3` (NEW `cube_4color_mmu_cells_are_disjoint` test), `AC-N4` (NEW `arachne_params_defaults_when_keys_absent` test).
- Refinements not captured in Given/When/Then:
  - The `dp_epsilon` → `visvalingam_area_threshold` rename is a WIT breaking change to the `arachne-params` record in `common.wit`. The host-service bridge in `slicer-sdk/src/host.rs` and the WIT host impl in `slicer-wasm-host/src/host.rs` must be updated in the same commit. No external consumers exist (the bridge is WASM-internal), so the break is contained.
  - Of the 7 wired config keys, 4 (`wall_transition_length`, `wall_transition_angle`, `initial_layer_min_bead_width`, `outer_wall_offset`) are ALREADY in the manifest (registered by P111); only 3 (`min_central_distance`, `visvalingam_area_threshold`, `min_width`) are net-new manifest entries. The 4 already-registered keys need no schema change; the wiring is in `arachne_params_from_config`.
  - The 7 new/now-wired config keys are all `units`-tagged in the manifest (1 unit = 100 nm) except `wall_transition_angle` (degrees). `arachne_params_from_config` converts each via `units_to_mm` (or degree→radian) before assigning to `ArachneParams` fields, which are in millimeters/radians.
  - The unit test on `paint_segmentation` (AC-4) establishes a geometric partition invariant UPSTREAM of the arachne output. It is supporting evidence for `D-112-MMU-TOPOLOGY` but does NOT close the deviation. The deviation is governed by `arachne-perimeters` output topology, not by the partition invariant; the target is P113b's quad/rib pass + `connectJunctions` (which will re-target the deviation with a different hypothesis: per-edge 2-junction fragment emission produces the "tens of mm outside" symptom).
  - The "Honesty note (no OrcaSlicer oracle)" section in `cube_4color_arachne.rs` is preserved verbatim. It is required by `D-112-SELFCAPTURED-BASELINES` and the existing test convention.
  - The `cube_4color_arachne/` fixture directory's `expected_perimeter_ir.json` is a self-captured golden per `D-112-SELFCAPTURED-BASELINES` (no OrcaSlicer oracle in-repo). It locks in regression behavior, not OrcaSlicer geometric parity.

## Verification Commands

Full verification matrix. `packet.spec.md` §Verification carries only the gate subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test simplify -- simplify_toolpaths_vertex_count 2>&1 | tee target/test-output-simplify.log` | AC-1: Visvalingam swap (existing test remains green) | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test simplify -- simplify_toolpaths_width_weighted_gate_preserves_junctions 2>&1 | tee target/test-output-simplify-neg.log` | AC-N1: gate preserves junctions that violate threshold (NEW test) | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_pipeline_thin_wall_widening 2>&1 | tee target/test-output-pipeline.log` | AC-2: 4 fields + factory threading (existing test remains green) | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test arachne_pipeline -- arachne_params_defaults_when_keys_absent 2>&1 | tee target/test-output-params-neg.log` | AC-N4: defaults when keys absent (NEW test) | FACT pass/fail |
| `rg -c 'config\.(get_float\|get_int\|get_bool)\("(min_central_distance\|visvalingam_area_threshold\|min_width\|wall_transition_length\|wall_transition_angle\|initial_layer_min_bead_width\|outer_wall_offset)"' modules/core-modules/arachne-perimeters/src/lib.rs` | AC-3: 7 keys read by arachne_params_from_config | FACT: count must be 7 |
| `rg -c '^\s*\[\[config\.schema\.(min_central_distance\|visvalingam_area_threshold\|min_width)\]\]' modules/core-modules/arachne-perimeters/arachne-perimeters.toml` | AC-3: 3 net-new manifest entries present | FACT: count must be 3 |
| `cargo test -p slicer-core --test paint_segmentation_mmu_partition_tdd -- cube_4color_mmu_partition_is_non_overlapping 2>&1 | tee target/test-output-mmu-unit.log` | AC-4: paint_segmentation unit test (NEW) | FACT pass/fail |
| `cargo test -p slicer-core --test paint_segmentation_mmu_partition_tdd -- cube_4color_mmu_cells_are_disjoint 2>&1 | tee target/test-output-mmu-neg.log` | AC-N3: MMU cells disjoint (NEW test) | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_fragments_walls_by_color 2>&1 | tee target/test-output-cube4c.log` | AC-5: executor wiring smoke test (existing test) | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration -- main_production_entry_path_loads_real_modules_and_calls_live_helpers 2>&1 | tee target/test-output-loader.log` | AC-6: loader source-guard tightened (existing test) | FACT pass/fail |
| `test -f crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_arachne/expected_perimeter_ir.json && echo "PRESENT"` | AC-7: fixture directory created (NEW) | FACT PRESENT or [unverified] |
| `rg -q '148 files.*13,981' .ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md && ! rg -q '102 files' .ralph/specs/112_arachne-extrusion-and-wireup/closure-log.md` | AC-8: closure-log gains the actual diff stat, no stale "102 files" line remains | FACT: both grep must succeed |
| `cargo test -p slicer-core --features host-algos --test remove_small -- remove_small_lines_all_primary_invariant 2>&1 | tee target/test-output-remove-neg.log` | AC-N2: remove_small primary preservation (existing test) | FACT pass/fail |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence (manifest + WIT edits) | FACT clean / STALE list |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants the per-step blocks in `implementation-plan.md` cannot express:

- The `dp_epsilon` → `visvalingam_area_threshold` rename must land atomically across `simplify.rs`, `pipeline.rs::ArachneParams`, `slicer-sdk/src/host.rs::ArachneParams`, and `common.wit::arachne-params`. No intermediate state where the field name is inconsistent. A half-renamed build will fail to compile the WIT bindings.
- The 7 config key wirings (Steps 2 + 3) must land together: the new `ArachneParams` fields are added in Step 2, the manifest entries and `arachne_params_from_config` reads are added in Step 3. The order matters because the module's `arachne_params_from_config` constructor must reference fields that exist.
- The MMU test fix (Steps 4 + 5) lands in two steps: the unit test (Step 4) is added FIRST so the geometric invariant is proven before the executor test is weakened. The executor test simplification (Step 5) removes the "out-of-footprint" narrative AFTER the unit test is green. Reverse order risks a window where neither test asserts the correct invariant.
- The `cargo xtask build-guests --check` must be run after Steps 1 (Visvalingam WIT rename), 2+3 (manifest entries), and the closure gate. Stale-guest failures surface as typed-instantiation errors that look unrelated to the change.

## Context Discipline Notes

Packet-specific context hazards:

- `crates/slicer-core/src/arachne/pipeline.rs` (331 LOC) MUST be range-read, not full-loaded. Read the `ArachneParams` struct (lines 69-116) and `run_arachne_pipeline` (lines 244-310) only.
- `crates/slicer-core/src/beading/factory.rs` is ~250 LOC; read `BeadingFactoryParams` struct and `create_stack` function only.
- `crates/slicer-core/src/arachne/simplify.rs` (140 LOC) can be full-read — it is the primary edit target.
- `modules/core-modules/arachne-perimeters/src/lib.rs` (276 LOC) is the secondary edit target; read `arachne_params_from_config` (lines 104-154) only on subsequent passes.
- The Visvalingam `calculateExtrusionAreaDeviationError` formula MUST be obtained via SUMMARY dispatch against `OrcaSlicerDocumented/src/libslic3r/Arachne/utils/ExtrusionLine.cpp:248` — the implementer MUST NOT read the OrcaSlicer source directly. The dispatch should return the formula's input types, output type, and a 1-line description of the computation.
- Sub-agent return-format hint for the `calculateExtrusionAreaDeviationError` dispatch: `SUMMARY (≤ 200 words): formula description, input types (ExtrusionJunction A, B, C), output type (coordf_t area deviation), and the comparison threshold's semantic (width-weighted triangle area). No code.`
- Tempting reads to skip: `crates/slicer-core/src/arachne/stitch.rs` (not edited by this packet), `crates/slicer-core/src/arachne/remove_small.rs` (not edited), `crates/slicer-core/src/skeletal_trapezoidation/*.rs` (P113b's domain).

If none apply, write `None packet-specific.`

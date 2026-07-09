# Requirements: 149-pipeline-config-wiring

## Packet Metadata

- Grouped task IDs: none (audit-only; not yet on `docs/07_implementation_status.md`).
- Backlog source: `tmp/arachne_parity_audit_20260709.md` (working artifact; not canonical — `docs/DEVIATION_LOG.md` is canonical).
- Packet status: `draft`
- Aggregate context cost: `M` (sum of per-step S/S/S/M/S in `implementation-plan.md`).

## Problem Statement

The audit found four pipeline-wide config keys (`detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`, `min_width_top_surface`, `alternate_extra_wall`) that are absent from the PnP pipeline as a whole (none registered, none consumed), plus a missing `bridging_flow()` implementation that should reduce `flow_factor` for bridge segments (OrcaSlicer's `LayerRegion.cpp:135` computes a flow factor via `base_flow.with_flow_ratio(bridge_flow_ratio)` — the canonical formula is a ratio, not a constant 0.85; PnP always emits `flow_factor = 1.0`). The fifth pipeline-wide gap is concentric infill Arachne wiring (G23 / D-104f) — too large to fit in this packet, deferred to a follow-up workstream; the corresponding red test stays red as the explicit success criterion for closing D-104f. A sixth deviation (D-104g) is added to document the per-vertex `flow_factor` model vs OrcaSlicer's per-path `Flow` model divergence — the `bridge_flow` ratio is correctly modelable per-vertex, but PnP doesn't model Flow height/width/thread_diameter the way OrcaSlicer does (the `thick_bridges` branch in the helper is the realization site). Six deviation rows are added to `docs/DEVIATION_LOG.md` (D-104b, D-104c, D-104d, D-104e, D-104f, D-104g); four of them close when this packet lands (D-104b/c/d/e), one stays open-but-deferred (D-104f), and one stays open as a limited divergence (D-104g).

The classic path already has `extra_perimeters_on_overhangs` (T-077, P108) and reads `only_one_wall_top`. This packet's D1 + D2 sub-tasks re-publish those keys in the arachne manifest for discoverability, register the missing keys (`detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`, `min_width_top_surface`), and add the new keys (`alternate_extra_wall`, `bridge_flow`, `thick_bridges`). D3 and D4 add the alternating-layer `max_bead_count` bump (mirrors OrcaSlicer's `loop_number++` beading-stack mechanism) and the ratio-based bridge flow reduction.

## In Scope

- Edit `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`:
  - Add `[config.schema.detect_overhang_wall]` (bool, default `true`; matches OrcaSlicer `PrintConfig.cpp:5003-5066` default of `1`).
  - Add `[config.schema.overhang_reverse]` (bool, default `false`; matches `PrintConfig.cpp:5059-5066`).
  - Add `[config.schema.overhang_reverse_internal_only]` (bool, default `false`; matches `PrintConfig.cpp`).
  - Add `[config.schema.min_width_top_surface]` (float, default verified via sub-agent dispatch against `docs/ORCA_CONFIG_REFERENCE.md:135` BEFORE commit; the spec's guess of 1.2mm is unverified — the sub-agent must confirm the canonical default is percent or mm, and the resolved mm value for a 0.4mm nozzle).
  - Add `[config.schema.alternate_extra_wall]` (bool, default `false`; matches `PrintConfig.cpp:5059-5066`).
  - Re-publish `[config.schema.extra_perimeters_on_overhangs]` (already in classic; the arachne manifest's test for AC-1 asserts its presence; copy from `classic-perimeters.toml:45`).
  - Add `[config.schema.bridge_flow]` (float, default `1.0`; matches OrcaSlicer `PrintConfig.cpp:1327` coFloat default). This is the ratio applied to the bridge flow — the canonical OrcaSlicer formula is `base_flow.with_flow_ratio(bridge_flow_ratio)`.
  - Add `[config.schema.thick_bridges]` (bool, default `false`; matches OrcaSlicer `PrintConfig.cpp:1941` coBool default). When `true`, PnP's helper returns `flow_factor = 1.0` (PnP doesn't model Flow height/width/thread_diameter the way OrcaSlicer does — this divergence is D-104g).
- Edit `modules/core-modules/arachne-perimeters/src/lib.rs`:
  - **D3 mechanism rewrite**: on odd layers (`layer_index % 2 == 1 && !spiral_vase && sparse_infill_density > 0`), increment `ArachneParams.max_bead_count` by 1 (mirrors OrcaSlicer's `loop_number++` at `PerimeterGenerator.cpp:1227` (classic) and `:2133` (arachne), which flows into `WallToolPaths(..., coord_t(loop_number + 1), ...)` → `max_bead_count = 2 * inset_count` at `WallToolPaths.cpp:525`). NOT a post-hoc wall-count mutation.
  - **D4 mechanism rewrite**: in `run_perimeters`, after packet 148's `is_bridge` per-vertex flag is set, for each `path.points[i]` with `feature_flags[i].is_bridge == true`, set `pt.flow_factor = slicer_core::flow::bridging_flow(bridge_flow_ratio, thick_bridges)`. The helper returns `bridge_flow_ratio` (the user's chosen ratio, default 1.0) when `!thick_bridges`, and `1.0` when `thick_bridges` (PnP's per-vertex model diverges from OrcaSlicer's per-path `Flow` model — D-104g).
  - Read `bridge_flow` and `thick_bridges` from config (the manifest entries above).
- Edit `modules/core-modules/classic-perimeters/classic-perimeters.toml`:
  - Re-publish the 4 missing keys (`detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`, `min_width_top_surface`) and add the 4 new keys (`alternate_extra_wall`, `bridge_flow`, `thick_bridges`).
- Edit `modules/core-modules/classic-perimeters/src/lib.rs`:
  - Apply the same `bridging_flow()` flow_factor reduction on bridge segments (the classic path's `is_bridge` flag is set per-vertex at `lib.rs:677`; the same `pt.flow_factor` reduction applies for parity with arachne).
- Edit `crates/slicer-core/src/flow.rs`:
  - Add `pub fn bridging_flow(bridge_flow_ratio: f32, thick_bridges: bool) -> f32 { if thick_bridges { 1.0 } else { bridge_flow_ratio } }` (matches OrcaSlicer's `LayerRegion.cpp:135` formula simplified for PnP's per-vertex model — the real formula uses `base_flow.with_flow_ratio(bridge_flow_ratio)` for the non-thick branch; PnP's per-vertex `flow_factor` model is a divergence, D-104g).
- Add new unit-test files in `arachne-perimeters/tests/`:
  - `alternate_extra_wall_tdd.rs` (AC-3): unit test asserts the wall count on odd vs even layers when `alternate_extra_wall=true` and the gate conditions are met.
  - `bridge_flow_factor_tdd.rs` (AC-4): unit test asserts `flow_factor == bridge_flow_ratio` for bridge vertices when `bridge_areas` is non-empty and `bridge_flow < 1.0`.
- Edit `docs/DEVIATION_LOG.md`:
  - Add `D-104b-OVERHANG-FLOW-NONE`, `D-104c-OVERHANG-REVERSE-NONE`, `D-104d-MIN-WIDTH-TOP-SURFACE-NONE`, `D-104e-ALTERNATE-EXTRA-WALL-NONE`, `D-104f-CONCENTRIC-INFILL-NO-ARACHNE` rows, matching the existing-row format. D-104f's `Target Close` is `— (deferred; follow-up workstream TBD)` (no fabricated schedule). Add `D-104g-FLOW-FACTOR-PERVERTEX-DIVERGENCE` documenting the per-vertex `flow_factor` vs OrcaSlicer's per-path `Flow` model divergence (the thick_bridges branch in the helper is the canonical realization site).
- Edit `docs/14_deviation_audit_history.md`:
  - Append one row per new deviation (6 total).
- Edit `docs/15_config_keys_reference.md`:
  - Append the 8 new config keys (4 in §Overhangs, 1 in §Walls, 1 in §Strength, 2 in §Bridging).

## Out of Scope

- Concentric infill Arachne wiring (D5 / D-104f) — deferred; the corresponding red test stays red until the follow-up workstream lands.
- The arachne per-vertex parity gaps (G7, G10, G12, G18, G19, G20, G21) — packet 148.
- The host service bridge (`slicer_core::arachne::pipeline::run_arachne_pipeline`) — unchanged.
- The beading-strategy stack — unchanged.
- The classic path's existing `extra_perimeters_on_overhangs` consumer (T-077) — unchanged.

## Authoritative Docs

- `docs/02_ir_schemas.md` — 2221 lines, MUST be ranged or delegated. Relevant sections: §1520-1533 (WallFeatureFlags), §1542-1558 (Point3WithWidth.flow_factor).
- `docs/03_wit_and_manifest.md` — > 300 lines, MUST be delegated for the `[config.schema]` format.
- `docs/15_config_keys_reference.md` — direct read for the existing-rows format.
- `docs/DEVIATION_LOG.md` — direct read for the existing-rows format.
- `docs/14_deviation_audit_history.md` — direct read for the row format.
- `docs/ORCA_CONFIG_REFERENCE.md` — direct read for the OrcaSlicer default values; lines 135, 161, 165-168, 178 are the canonical defaults for the keys this packet adds.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:5003-5066` — `detect_overhang_wall`, `overhang_reverse`, `overhang_reverse_internal_only`, `extra_perimeters_on_overhangs`, `alternate_extra_wall` coBool defaults and groups.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:1491-1511` — `min_width_top_surface` coFloatOrPercent default (300%) and relationship to `only_one_wall_top`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:1658` — `thick_bridges` coBool default (`0`).
- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp:135` — `bridging_flow(frPerimeter, thick_bridges)` formula.
- `OrcaSlicerDocumented/src/libslic3r/FillConcentric.cpp:80-118` — D-104f (deferred, do not implement).

## Acceptance Summary

Reference Acceptance Criteria by ID. Do not copy them.

- Positive cases: AC-1 through AC-6. Measurable refinements:
- AC-1: the four keys MUST be present in the arachne manifest TOML (the test grep-asserts this). The keys MUST also be present in the classic manifest (manifest-parity for the wall_generator switch); this is a Doc Impact requirement, not a test requirement.
- AC-2: `min_width_top_surface` MUST be present in the arachne manifest with a default verified via sub-agent dispatch against `docs/ORCA_CONFIG_REFERENCE.md:135` BEFORE commit. The test predicate is a manifest-presence grep on the arachne manifest.
- AC-3: `alternate_extra_wall` MUST be present in the arachne manifest. The unit test in `arachne-perimeters/tests/alternate_extra_wall_tdd.rs` (NEW) sets up `wall_count=2`, `alternate_extra_wall=true`, the gate conditions `!spiral_vase && sparse_infill_density > 0`, and asserts the wall count is 3 on odd layers and 2 on even layers. Mechanism: bump `ArachneParams.max_bead_count` by 1 on odd layers (mirrors OrcaSlicer's `loop_number++` → `WallToolPaths(..., coord_t(loop_number + 1), ...)` → `max_bead_count = 2 * inset_count`).
- AC-4: the bridge flow factor test fixture sets `region.bridge_areas()` to a non-empty polygon set and asserts bridge vertices' `path.points[i].flow_factor` equals `bridge_flow_ratio` (config-driven, default 1.0). The helper `bridging_flow(bridge_flow_ratio, thick_bridges) -> f32` returns the ratio (or 1.0 when `thick_bridges` — PnP's per-vertex model diverges from OrcaSlicer's per-path `Flow` model; D-104g documents this). Both perimeter modules apply it. The test sets `bridge_flow = 0.7` and asserts `flow_factor == 0.7`.
- AC-5: the 6 new deviation log rows MUST be present (greps verify).
- AC-6: full `arachne_parity` shows 14 passed (3 packet-1 + 7 packet-148 + 4 packet-149), 1 red (D-104f only).
- Negative cases: AC-N1 (no manifest drift into packet 148's scope) and AC-N2 (D-104f's Target Close does not name a fabricated packet).
- Cross-packet impact: this packet closes D-104b/c/d/e at packet close. D-104f stays open with a deferred-implementation note; the follow-up workstream will close it.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test arachne_parity 2>&1 \| tee target/test-output.log` | AC-6: full file count | FACT pass/fail; `grep '^test result' target/test-output.log` |
| `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_pipeline_overhang_reverse_config_keys 2>&1 \| tee target/test-output.log` | AC-1 | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_pipeline_only_one_wall_top_vs_min_width_top_surface 2>&1 \| tee target/test-output.log` | AC-2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_pipeline_alternate_extra_wall_not_registered 2>&1 \| tee target/test-output.log` | AC-3 | FACT pass/fail |
| `cargo test -p slicer-runtime --test arachne_parity -- arachne_parity_pipeline_bridge_flow_factor_on_overhang 2>&1 \| tee target/test-output.log` | AC-4 | FACT pass/fail |
| `cargo test -p arachne-perimeters --test alternate_extra_wall_tdd 2>&1 \| tee target/test-output.log` | AC-3 unit | FACT pass/fail |
| `cargo test -p arachne-perimeters --test bridge_flow_factor_tdd 2>&1 \| tee target/test-output.log` | AC-4 unit | FACT pass/fail |
| `rg -q 'config\.schema\.(precise_outer_wall\|seam_candidate_angle_threshold_deg)' modules/core-modules/arachne-perimeters/arachne-perimeters.toml; [ $? -ne 0 ]` | AC-N1 | FACT exit 0 = pass |
| `rg -A1 'D-104f-CONCENTRIC-INFILL-NO-ARACHNE' docs/DEVIATION_LOG.md \| head -5` | AC-N2 | SNIPPETS, manual check |
| `rg -q 'D-104b-OVERHANG-FLOW-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104c-OVERHANG-REVERSE-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104d-MIN-WIDTH-TOP-SURFACE-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104e-ALTERNATE-EXTRA-WALL-NONE' docs/DEVIATION_LOG.md && rg -q 'D-104f-CONCENTRIC-INFILL-NO-ARACHNE' docs/DEVIATION_LOG.md` | AC-5 | FACT exit 0 = pass |
| `cargo clippy -p slicer-runtime --test arachne_parity -- -D warnings 2>&1 \| tee target/clippy-output.log` | gate | FACT exit 0 |
| `cargo xtask build-guests --check 2>&1 \| tee target/guest-check.log` | gate (manifest change rebuilds) | FACT STALE/Fresh |

All verification commands are delegation-friendly.

## Step Completion Expectations

Cross-step invariants that the per-step blocks in `implementation-plan.md` cannot express:

- The new arachne manifest keys MUST have byte-for-byte identical default values to the OrcaSlicer reference (per `docs/ORCA_CONFIG_REFERENCE.md`). The implementer should `diff` against the reference before committing. `min_width_top_surface`'s default is verified via sub-agent dispatch BEFORE commit (the spec's guess of 1.2mm is unverified).
- The new classic manifest keys MUST mirror the arachne ones (manifest-parity for the wall_generator switch).
- The bridge flow factor reduction MUST apply to BOTH the classic and arachne paths. The D4 helper `slicer_core::flow::bridging_flow(bridge_flow_ratio, thick_bridges)` is the single source of truth; both perimeter modules call it.
- The D3 (alternate_extra_wall) mechanism is bumping `ArachneParams.max_bead_count` by 1 on odd layers when `layer_index % 2 == 1 && !spiral_vase && sparse_infill_density > 0` (mirrors OrcaSlicer's `loop_number++` → `max_bead_count` beading-stack mechanism; NOT a wall-count mutation).
- The D-104f deviation row's `Target Close` field MUST be `— (deferred; follow-up workstream TBD)` (no fabricated schedule).
- D-104b/c/d/e flip to `Status: Closed — <date>: packet 149` at packet close; D-104f stays `Status: Open — deferred to follow-up workstream`; D-104g is `Status: Open` (documents the per-vertex `flow_factor` vs OrcaSlicer's per-path `Flow` model divergence — the `thick_bridges` branch in the helper is the canonical realization site; the `bridge_flow` ratio itself is correctly modelable per-vertex, so this is a limited divergence, not a gap).

## Context Discipline Notes

- `OrcaSlicerDocumented/` is forbidden to load directly; every parity check is a sub-agent dispatch.
- The 7 new manifest keys are atomically added in one step (Step 1) — splitting them across steps adds no value because they all share the same test-failure mode (manifest presence grep) and the same risk surface (default-value mismatches).
- The largest single step is Step 4 (D4 bridge flow) — it touches both perimeter modules, adds a `slicer_core::flow::bridging_flow` helper, and threads `thick_bridges` through both manifests. Cost M.
- The implementer should NOT re-open packet 148's files (the manifest keys for `precise_outer_wall` and `seam_candidate_angle_threshold_deg` are packet 148's; adding them again here is a manifest-drift bug caught by AC-N1).

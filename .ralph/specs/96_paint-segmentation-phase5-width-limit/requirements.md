# Requirements: 96_paint-segmentation-phase5-width-limit

## Packet Metadata

- Grouped task IDs:
  - `TASK-246` — OrcaSlicer `cut_segmented_layers` Phase 5 (width limiting + interlocking) for the paint-segmentation pipeline.
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P4"
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P95 ships Phases 1, 2, 3, 4, 6, 7 of the paint-segmentation pipeline. Phase 5 — `cut_segmented_layers` — is deferred because it's the only stage whose impact is purely geometric refinement of an already-correct variant assignment. With Phase 5 missing:

- The `mmu_segmented_region_max_width` config key has no geometric effect. A user who configures `0.4` (the OrcaSlicer default unit width) sees no change in the produced regions — the assignment is sharp-edged, no width limiting.
- The `mmu_segmented_region_interlocking_depth` config key has no effect. Without interlocking beams between layers, painted regions stack vertically with no inter-layer reinforcement, which is a print-quality regression for multi-color models.

The OrcaSlicer-parity goal stated in the v2 audit and the roadmap explicitly includes Phase 5. This packet closes the gap by porting `cut_segmented_layers` per spec §3 Phase 5:

- Per layer, per variant chain, erode the variant's polygons by `difference_ex(variant_polygons, offset_inward(input_expolygons, region_width))`.
- When `interlocking_depth > 0` and `interlocking_beam = false`: alternate the erosion depth between even and odd layers (creates the beam pattern).
- When `interlocking_depth > 0` and `interlocking_beam = true`: apply constant depth across all layers (no alternation; produces "beam" pattern in OrcaSlicer's parlance).
- When both keys are 0 (default): short-circuit (no Phase 5 work).

The pass plugs in AFTER Phase 7 (compose_variants from P95) and BEFORE the final `replace_slice_ir` commit. Reads config via `RegionMapIR::config_for(&region_key)` (P1a). Short-circuit on default config preserves byte-identical behavior on every existing test.

## In Scope

- New module file `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` (~150 LOC).
- `cut_segmented_layers` function implementing spec §3 Phase 5.
- Integration into `execute_paint_segmentation_v2` (post Phase 7, pre `replace_slice_ir` commit).
- Three new config-schema entries in the appropriate location (host or module manifest per P95 structural decision):
  - `mmu_segmented_region_max_width` (mm, default 0.0).
  - `mmu_segmented_region_interlocking_depth` (mm, default 0.0).
  - `mmu_segmented_region_interlocking_beam` (bool, default false).
- Three unit tests for the kernel (width-limit only; interlocking alternating; interlocking constant).
- Three integration tests in `cube_4color_paint_tdd.rs` (or a new `cube_4color_phase5_tdd.rs`):
  - `cube_4color_phase5_width_limit_bands`.
  - `cube_4color_phase5_interlocking_alternates`.
  - `cube_4color_phase5_interlocking_beam_constant`.
- Negative-case kernel tests for AC-N1, AC-N2, AC-N3.
- Optional small fixture `resources/cube_4color_tall.3mf` (≤ 100 KB) only if the existing `cube_4color.3mf` is too short to produce meaningful layer-alternation visibility (the height threshold is the number of layers required to assert alternation across at least 2 distinct band cycles).
- Closure-log entry capturing the wedge SHA (must match P95 baseline; AC-8) AND the cube_4color SHA (must match P95 baseline; AC-8) AND a screenshot reference (or HTML report path) confirming visual banding.

## Out of Scope

- Any change to Phases 1, 2, 3, 4, 6, 7 — P95 territory.
- WASM mesh-segmentation deletion — P5a (97).
- Loader symmetry — P5b (98).
- Doc updates — P5c (99).
- Performance optimization beyond Rayon (already in place from P95).
- Inter-extruder interlocking (Phase 5's "beam" mode is a specific implementation; future variations like adaptive depth are not in scope).

## Authoritative Docs

- `docs/specs/orca-paint-segmentation-parity.md` §3 Phase 5 — primary algorithm spec.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P4".
- `docs/02_ir_schemas.md` — `SlicedRegion.polygons` shape.
- `docs/08_coordinate_system.md` — coordinate-unit conversion.
- `crates/slicer-core/src/polygon_ops.rs` — `offset` and `difference_ex` (from P95 sub-step 0).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` Phase 5 section — SUMMARY confirming the algorithm.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-11`. Refinement:
  - The "banded" assertion in AC-5 measures band width within rounding tolerance (e.g., ± 5% of the configured width given f32 arithmetic + coordinate-unit conversion). The closure log records the measured vs. expected values.
- Negative cases: `AC-N1` (negative width rejected), `AC-N2` (oversize width yields empty — D15-compatible), `AC-N3` (beam flag ignored when depth = 0).
- Cross-packet impact: completes OrcaSlicer paint-pipeline parity. P5a/b/c are independent cleanup packets.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings | FACT pass/fail |
| `cargo test -p slicer-core paint_segmentation::width_limit 2>&1 \| tee target/test-output.log` | AC-1, AC-N1, AC-N2, AC-N3 — kernel | FACT pass/fail with breakdown |
| `cargo test -p slicer-runtime --test executor cube_4color_phase5 2>&1 \| tee target/test-output.log` | AC-5, AC-6, AC-7 — integration | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 \| tee target/test-output.log` | AC-10 — regression (12/12 still GREEN) | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 \| tee target/test-output.log` | AC-10 — regression (12/12 still GREEN) | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p96-wedge.gcode && sha256sum /tmp/p96-wedge.gcode` | AC-8 — wedge byte-identical | FACT (sha256) |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-cube.gcode && sha256sum /tmp/p96-cube.gcode` | AC-8 — cube byte-identical (default config) | FACT (sha256) |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-cube.gcode --report /tmp/p96-cube-report.html && test -f /tmp/p96-cube-report.html` | AC-9 — visual report | FACT pass/fail (file existence) |
| `cargo xtask build-guests --check` | AC-11 — guest clean | FACT pass/fail |

## Step Completion Expectations

- The kernel unit tests (Step 2) MUST pass before integration (Step 3) — width_limit algorithm correctness is the prerequisite for integration tests to be meaningful.
- The short-circuit guard MUST be in place before any integration test runs with default config (AC-8) — otherwise default-config byte-identical regression breaks.
- The config-schema entries (Step 4) land alongside the kernel because the kernel reads them; ordering within Steps 2+3+4 is flexible but the schema entries must exist before integration tests use them.

## Context Discipline Notes

- `docs/specs/orca-paint-segmentation-parity.md` is 1021 lines. Range-read §3 Phase 5 ONLY (likely 50-80 lines).
- The kernel file is single-purpose and small (~150 LOC); read in full when editing.
- Config-schema location depends on P95's structural choice (host vs module manifest). The first dispatch confirms.
- Cube fixtures are binary; never `Read`.

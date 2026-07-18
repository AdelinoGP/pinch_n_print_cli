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

Additionally, P95 closed with one acceptance test (`cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one`) deferred under deviation `D-95-AC22-BISECTOR-DEDUP`: every Voronoi edge between two differently-colored cells is traced as an outer wall by BOTH adjacent cells, doubling the per-layer outer-wall count for N-color slices. P96 OWNS the fix.

The OrcaSlicer-parity goal stated in the v2 audit and the roadmap explicitly includes Phase 5. This packet closes the gap by porting `cut_segmented_layers` per spec §3 Phase 5 AND implementing the bisector-edge ownership mechanism:

- Per layer, per variant chain, erode the variant's polygons by `difference_ex(variant_polygons, offset(input_expolygons, -depth_for_layer_mm, OffsetJoinType::Miter, OFFSET_ARC_TOLERANCE_MM))`. The inward-offset primitive is the existing `pub fn offset(polygons: &[ExPolygon], delta_mm: f32, join, arc_tolerance) -> Vec<ExPolygon>` at `crates/slicer-core/src/polygon_ops.rs:195` invoked with a NEGATIVE `delta_mm`; there is NO `offset_inward` / `offset_expolygons_inward` helper. Per-layer depth selection (OrcaSlicer parity, verified against `MultiMaterialSegmentation.cpp:1294`): `depth_for_layer = (layer_idx % 2 == 0 && interlocking_depth_units != 0) ? interlocking_depth_units : region_width_units`. The even-layer branch uses `interlocking_depth` STANDALONE — NOT additive with `region_width`. Conversion: `depth_mm = depth_units / 10_000.0` (1 unit = 100 nm = 1e-4 mm; see `docs/08_coordinate_system.md`).
- When `interlocking_beam = true`: the driver SKIPS the entire `cut_segmented_layers` call (OrcaSlicer parity verified against `MultiMaterialSegmentation.cpp:2452`). The original P96 draft assumed `beam = true` means "constant-depth alternation"; this was incorrect.
- When both `region_width = 0` and `interlocking_depth = 0`: kernel short-circuits internally (no-op).

The Phase 5 pass plugs into the driver `pub fn execute_paint_segmentation` at `crates/slicer-core/src/algos/paint_segmentation/mod.rs:393`, INSERTED after the inlined variant-composition block ends at `mod.rs:802` and before the final `Ok(Arc::new(working))` return at `mod.rs:999`. Reads config via `RegionMapIR::config_for(&region_key)` (P1a; helper defined at `slice_ir.rs:1230`). Default config (all three keys at declared defaults) preserves byte-identical behavior via the driver-level `!beam` guard.

The bisector-edge ownership mechanism (TASK-246-BISECTOR): extend `SlicedRegion` with a `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` field; populate it in `execute_paint_segmentation` between the variant block end and the Phase 5 call; consume it in `modules/core-modules/classic-perimeters/src/lib.rs` outer-wall emission (`run_perimeters` at `lib.rs:85`, polygons consumed at `lib.rs:94`). See `design.md` §"Bisector-Edge Ownership (AC-22b) Code Change Surface" for the full surface.

## In Scope

- New module file `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` (~200 LOC).
- `cut_segmented_layers` function with signature `pub fn cut_segmented_layers(variants_per_layer: &mut [BTreeMap<ChainKey, Vec<ExPolygon>>], input_expolygons_per_layer: &[Vec<ExPolygon>], region_width_units: i64, interlocking_depth_units: i64) -> Result<(), PaintSegmentationError>` (where `ChainKey = Vec<(String, PaintValue)>` re-exported from `compose_variants.rs:45`). Per-layer depth: `(layer_idx % 2 == 0 && interlocking_depth_units != 0) ? interlocking_depth_units : region_width_units` (OrcaSlicer parity; STANDALONE depth, not additive). NO `interlocking_beam` parameter — driver guards the call.
- Integration into `execute_paint_segmentation` (driver at `crates/slicer-core/src/algos/paint_segmentation/mod.rs:393`, after `mod.rs:802`, before `mod.rs:999`, with `if !interlocking_beam` guard).
- Three new `[config.schema.*]` entries in `modules/core-modules/mesh-segmentation/mesh-segmentation.toml`:
  - `mmu_segmented_region_max_width` (f32, mm, default 0.0, minimum 0.0).
  - `mmu_segmented_region_interlocking_depth` (f32, mm, default 0.0, minimum 0.0).
  - `mmu_segmented_region_interlocking_beam` (bool, default false).
- Six unit tests for the kernel (3 positive: width-only erosion / interlocking alternating / interlocking-zero degenerates; 3 negative: AC-N1 negative rejected / AC-N2 oversize → empty / driver-level beam-skip assertion).
- Three integration tests in NEW files under `crates/slicer-runtime/tests/executor/`:
  - `cube_4color_phase5_width_limit_bands_tdd.rs`.
  - `cube_4color_phase5_interlocking_alternates_tdd.rs`.
  - `cube_4color_phase5_interlocking_beam_skips_phase5_tdd.rs` (asserts byte-identicality vs baseline; replaces the misnamed "_beam_constant" test).
- **Bisector-edge ownership mechanism (TASK-246-BISECTOR):**
  - New field `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` on `SlicedRegion` in `crates/slicer-ir/src/slice_ir.rs:1273`.
  - Tagging logic in `execute_paint_segmentation` between the variant-composition block end (`mod.rs:802`) and the Phase 5 call: geometric edge-coincidence detection (see `design.md` §"Bisector-Edge Ownership" → "Algorithm") buckets each polygon edge by its sorted `Point2<i64>` endpoint pair into a `HashMap<EdgeKey, Vec<(region_idx, poly_idx, edge_idx, color_id)>>`. Buckets with ≥ 2 entries are geometric bisector edges: the higher-color-id region sets the skip bit; the region with `min(color_id)` over its `variant_chain` owns the edge. Boundary edges (bucket size = 1) stay `false`. 3-color corners resolve independently per edge.
  - Consumer change in `modules/core-modules/classic-perimeters/src/lib.rs` outer-wall emission (`run_perimeters` at line 85; polygons consumed at line 94; outer wall loop at lines 111–118; edge iteration at line 153) to skip masked edges.
  - Remove `#[ignore]` attribute at `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs:337`.
- Optional small fixture `resources/cube_4color_tall.3mf` (≤ 100 KB) only if the existing `cube_4color.3mf` is too short to produce meaningful layer-alternation visibility.
- Closure-log entry capturing: wedge SHA equality to baseline (AC-8), cube_4color SHA equality to baseline (AC-8), 21/21 cube tests GREEN (AC-10; 11 cube_4color_paint_tdd + 10 cube_fuzzy_painted_tdd), visual banding evidence (AC-9), AC-22b test count line (`1 passed; 0 failed; 0 ignored`).

## Out of Scope

- Any change to Phases 1, 2, 3, 4, 6, 7 logic — P95 territory. (NOTE: this packet ADDS a tagging stage between Phase 7 inlined output and Phase 5; that is bisector-edge ownership work scoped under TASK-246-BISECTOR, not a change to Phase 7 itself.)
- WASM mesh-segmentation logic deletion — P5a (97). NOTE: the schema landing site this packet uses (`modules/core-modules/mesh-segmentation/mesh-segmentation.toml`) is the EXISTING module manifest; if P5a deletes that module, P5a is responsible for relocating the three `[config.schema.mmu_segmented_region_*]` entries to whatever module owns paint-segmentation host config post-P5a.
- Loader symmetry — P5b (98).
- Doc updates beyond the Doc Impact Statement greps in `packet.spec.md` — P5c (99).
- Performance optimization beyond Rayon (already in place from P95).
- Inter-extruder interlocking variations beyond OrcaSlicer's two modes (alternating vs skip-entirely).

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

- Positive cases: `AC-1` through `AC-11` plus `AC-22b`. Refinement:
  - The "banded" assertion in AC-5 measures band width within rounding tolerance (± 5% of the configured width given f32 arithmetic + coordinate-unit conversion). The closure log records measured vs. expected values.
  - AC-7 asserts byte-identicality vs a `width=0,depth=0,beam=false` baseline; this proves the driver `!beam` skip-guard works (per OrcaSlicer parity `MultiMaterialSegmentation.cpp:2452`).
  - AC-22b asserts per-layer outer-wall extrusion-move count equality within ±1 vs the unpainted-cube baseline, via the bisector-edge ownership mechanism on `SlicedRegion.bisector_edge_skip_mask`.
- Negative cases: `AC-N1` (negative width rejected), `AC-N2` (oversize width yields empty — D15-compatible), `AC-N3` (`beam = true` short-circuits Phase 5 at the driver regardless of `depth`/`width` values — OrcaSlicer parity).
- Cross-packet impact: completes OrcaSlicer paint-pipeline parity. P5a/b/c are independent cleanup packets. AC-22b resolves the inherited P95 deviation `D-95-AC22-BISECTOR-DEDUP`.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-core --features host-algos paint_segmentation::width_limit 2>&1 \| tee target/test-output.log` | AC-1, AC-N1, AC-N2 — kernel (≥ 6 tests; `--features host-algos` required: slicer-core has `default = []`) | FACT (≥6 passed; 0 failed) |
| `mkdir -p target && cargo test -p slicer-core --features host-algos interlocking_beam_true_skips_phase5_driver 2>&1 \| tee target/test-output.log` | AC-N3 — driver skip when beam=true (filter has no `paint_segmentation::` prefix — substring match works whether test is at file root or in `mod tests`) | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_phase5 2>&1 \| tee target/test-output.log` | AC-5, AC-6, AC-7 — integration | FACT pass/fail |
| `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 \| tee target/test-output.log` | AC-10 — regression (11/11 still GREEN) | FACT (11 passed; 0 failed) |
| `mkdir -p target && cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 \| tee target/test-output.log` | AC-10 — regression (10/10 still GREEN) | FACT (10 passed; 0 failed) |
| `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one 2>&1 \| tee target/test-output.log` | AC-22b — bisector-edge dedup | FACT (1 passed; 0 failed; 0 ignored) |
| Step 0 baseline capture: `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p96-baseline-wedge.gcode && sha256sum /tmp/p96-baseline-wedge.gcode \| cut -d' ' -f1 > target/p96-baseline-wedge.sha` | AC-8 prerequisite | FACT (file written) |
| Step 0 baseline capture: `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-baseline-cube.gcode && sha256sum /tmp/p96-baseline-cube.gcode \| cut -d' ' -f1 > target/p96-baseline-cube.sha` | AC-8 prerequisite | FACT (file written) |
| AC-8 wedge equality: `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p96-wedge.gcode && [ "$(sha256sum /tmp/p96-wedge.gcode \| cut -d' ' -f1)" = "$(cat target/p96-baseline-wedge.sha)" ]` | AC-8 — wedge byte-identical (machine equality) | FACT pass/fail |
| AC-8 cube equality: `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-cube.gcode && [ "$(sha256sum /tmp/p96-cube.gcode \| cut -d' ' -f1)" = "$(cat target/p96-baseline-cube.sha)" ]` | AC-8 — cube byte-identical (machine equality) | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-cube-banded.gcode --report /tmp/p96-cube-banded-report.html && test -f /tmp/p96-cube-banded-report.html` | AC-9 — visual report (machine: file existence; manual: visual confirm) | FACT pass/fail |
| AC-3 schema TOML structural assertion (full command in `packet.spec.md` AC-3) | AC-3 — type/default/units/minimum fields | FACT pass/fail |
| `cargo xtask build-guests --check` | AC-11 — guest clean | FACT pass/fail |

## Step Completion Expectations

- The kernel unit tests (Step 2) MUST pass before integration (Step 4a) — width_limit algorithm correctness is the prerequisite for integration tests to be meaningful.
- The driver-level `!interlocking_beam` guard AND the kernel's `(width=0 && depth=0)` short-circuit MUST both be in place before any AC-8 byte-identicality check runs.
- The config-schema entries (Step 3) land before the integration code reads them; the driver in Step 4a expects the schema keys to exist for `RegionMapIR::config_for` to resolve.
- Steps 4b (IR field + tagging) and 4c (consumer change in classic-perimeters) MAY land sequentially in either order, but BOTH must precede the AC-22b verification command. Step 4b's IR addition MUST be additive and default to `None` to preserve AC-10 regression.

## Context Discipline Notes

- `docs/specs/orca-paint-segmentation-parity.md` is 1021 lines. Range-read §3 Phase 5 ONLY (likely 50-80 lines).
- The kernel file is single-purpose and small (~150 LOC); read in full when editing.
- Config-schema location depends on P95's structural choice (host vs module manifest). The first dispatch confirms.
- Cube fixtures are binary; never `Read`.

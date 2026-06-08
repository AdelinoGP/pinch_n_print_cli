---
status: draft
packet: 96
task_ids: [TASK-246]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 96 — Paint-Segmentation Phase 5: Width Limiting + Interlocking

## Goal

Implement OrcaSlicer's `cut_segmented_layers` per `docs/specs/orca-paint-segmentation-parity.md` §3 Phase 5 so the `mmu_segmented_region_max_width` and `mmu_segmented_region_interlocking_depth` (and `mmu_segmented_region_interlocking_beam`) config keys take geometric effect: per layer, per variant, erode the variant's polygons by `difference_ex(variant_polygons, input_expolygons offset_inward by region_width)` with depth alternating between even/odd layers when `interlocking_beam == false` and constant when `true`; wire the pass AFTER sub-step 12 (compose_variants) inside `execute_paint_segmentation_v2`; read config keys via the P1a interner helper (`RegionMapIR::config_for`); add config-schema TOML entries for the three keys to the appropriate core module / host config; extend `cube_4color_paint_tdd.rs` with the two new SHAPE-DEPENDENT tests the roadmap describes (tall cube with width-limit=2.0 mm produces banded extruder regions vertically; interlocking_depth=0.5 mm produces alternating bands across adjacent layers); make sure unpainted slicing is byte-identical to the post-P95 baseline (Phase 5 short-circuits when no variants exist).

## Scope Boundaries

Phase 5 is an OPTIONAL stage of the paint-segmentation pipeline that erodes per-variant polygons by a configured width and (optionally) creates interlocking beams between adjacent layers. The pass runs only when at least one variant has non-empty polygons; otherwise it short-circuits. The three config keys land in the host's effective config schema (or in the `paint-segmentation-default` module's manifest if that's the structural home decided in P95). The two new tests extend the cube_4color suite using a tall cube fixture (the existing `cube_4color.3mf` may already be tall enough; if not, a small `cube_4color_tall.3mf` is authored). Full in/out-of-scope lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: P95 (paint-segmentation port; Phases 1, 2, 3, 4, 6, 7) must be `implemented`. Phase 5 reads the variant-chain map produced by Phase 7 and writes back via the same `replace_slice_ir` channel.
- Unblocks: nothing structurally. With Phase 5 in place the paint pipeline matches OrcaSlicer parity completely; remaining packets (P5a/b/c) are deletion + symmetry + docs.
- Activation blockers: P95 closed.

## Acceptance Criteria

### AC-1 — `width_limit.rs` implements `cut_segmented_layers` per spec §3 Phase 5

**Given** the new file,
**When** `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` is inspected,
**Then** it exports a function with signature roughly `pub fn cut_segmented_layers(variants_per_layer: &mut [HashMap<Vec<(String, PaintValue)>, ExPolygons>], input_expolygons_per_layer: &[ExPolygons], region_width_units: i64, interlocking_depth_units: i64, interlocking_beam: bool) -> Result<(), PaintSegmentationError>`; the function performs the erosion per spec §3 Phase 5; has at least three unit tests covering (a) width-limit-only erosion (no interlocking), (b) interlocking with alternating depth, (c) interlocking with constant depth (interlocking_beam = true).

| `cargo test -p slicer-core paint_segmentation::width_limit 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [3-9]+ passed'`

### AC-2 — `cut_segmented_layers` runs AFTER `compose_variants` (sub-step 12) in `execute_paint_segmentation_v2`

**Given** the integration point,
**When** `crates/slicer-core/src/algos/paint_segmentation/mod.rs` is inspected,
**Then** the call to `cut_segmented_layers` appears after `compose_variants` and before the final `replace_slice_ir` commit; if `interlocking_depth_units == 0` and `region_width_units == 0`, the pass short-circuits (no-op).

| `rg -A20 'compose_variants' crates/slicer-core/src/algos/paint_segmentation/mod.rs | rg -q 'cut_segmented_layers'`

### AC-3 — Config keys `mmu_segmented_region_max_width`, `mmu_segmented_region_interlocking_depth`, `mmu_segmented_region_interlocking_beam` exist in the appropriate schema

**Given** the config-schema landing site (host's effective config schema OR `paint-segmentation-default` module's manifest — per P95 design choice),
**When** the schema TOML is inspected,
**Then** the three keys exist with documented `type`, `default`, `units`, and `description` fields. Defaults: `mmu_segmented_region_max_width = 0.0` (disabled), `mmu_segmented_region_interlocking_depth = 0.0` (disabled), `mmu_segmented_region_interlocking_beam = false`. Units: mm for the two depth/width keys, bool for the beam flag. Default values preserve byte-identical behavior on existing tests.

| `rg -q 'mmu_segmented_region_max_width' modules/core-modules/ crates/slicer-runtime/src/ && rg -q 'mmu_segmented_region_interlocking_depth' modules/core-modules/ crates/slicer-runtime/src/ && rg -q 'mmu_segmented_region_interlocking_beam' modules/core-modules/ crates/slicer-runtime/src/`

### AC-4 — Phase 5 reads config via `RegionMapIR::config_for`

**Given** the interning design from P1a,
**When** Phase 5 reads the three config keys,
**Then** it routes through `region_map.config_for(&region_key)` — NOT via a direct `plan.config` read (the latter shape was removed in P1a).

| `rg -q 'config_for' crates/slicer-core/src/algos/paint_segmentation/width_limit.rs`

### AC-5 — Tall cube + width_limit=2.0 mm produces banded extruder regions vertically

**Given** a synthetic test scenario: `cube_4color_tall.3mf` (or `cube_4color.3mf` if tall enough) with `mmu_segmented_region_max_width = 2.0`,
**When** the slice runs,
**Then** the produced SliceIR's per-variant polygons on a mid-layer show eroded bands of approximately 2 mm width (allowing rounding tolerance); a unit test asserts the band-width on a known mid-layer.

| `cargo test -p slicer-runtime --test executor cube_4color_phase5_width_limit_bands 2>&1 | tee target/test-output.log`

### AC-6 — interlocking_depth=0.5 mm produces alternating bands across adjacent layers

**Given** the same tall cube fixture with `mmu_segmented_region_interlocking_depth = 0.5` and `mmu_segmented_region_interlocking_beam = false`,
**When** the slice runs,
**Then** adjacent layers (even/odd Z) show alternating band positions per spec §3 Phase 5; a test asserts the alternation pattern between two specific layers.

| `cargo test -p slicer-runtime --test executor cube_4color_phase5_interlocking_alternates 2>&1 | tee target/test-output.log`

### AC-7 — `interlocking_beam = true` produces constant depth (no alternation)

**Given** `mmu_segmented_region_interlocking_depth = 0.5` AND `mmu_segmented_region_interlocking_beam = true`,
**When** the slice runs,
**Then** adjacent layers show identical band positions (no even/odd alternation); a test asserts identity between two specific layers.

| `cargo test -p slicer-runtime --test executor cube_4color_phase5_interlocking_beam_constant 2>&1 | tee target/test-output.log`

### AC-8 — Behavior preservation when both keys are 0 (default)

**Given** the default config (both keys 0.0),
**When** `pnp_cli slice` runs on `resources/regression_wedge.stl` AND on `resources/cube_4color.3mf` with default config,
**Then** the wedge g-code is byte-identical to the post-P95 baseline; the cube g-code is byte-identical to the post-P95 cube baseline (Phase 5 short-circuits when both keys are 0).

| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p96-wedge.gcode && sha256sum /tmp/p96-wedge.gcode && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-cube.gcode && sha256sum /tmp/p96-cube.gcode`

### AC-9 — Visual inspection via `pnp_cli --report` on cube_4color shows banded variant regions

**Given** the HTML slicer report,
**When** a slice runs with `--report /tmp/p96-cube-report.html` and `mmu_segmented_region_max_width = 2.0`,
**Then** the report HTML contains per-layer visualizations whose painted variant regions show banded structure (manual visual check via implementer; closure log notes the layer ID + screenshot reference).

Manual check (closure-log evidence). The report file existence is the machine gate.

| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-cube.gcode --report /tmp/p96-cube-report.html && test -f /tmp/p96-cube-report.html`

### AC-10 — 24 cube paint tests (12 cube_4color + 12 cube_fuzzy_painted) remain GREEN

**Given** the new pass is gated by non-zero config,
**When** the cube test suites run with default config,
**Then** all 24 tests still pass (no regression vs. P95).

| `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. 12 passed; 0 failed' && cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee -a target/test-output.log | grep -qE 'test result: ok\. 12 passed; 0 failed'`

### AC-11 — Guest WASM `--check` clean

| `cargo xtask build-guests --check`

## Negative Test Cases

### AC-N1 — Phase 5 with negative width is rejected

**Given** a manifest / config with a negative `mmu_segmented_region_max_width`,
**When** the slice runs,
**Then** the slice fails with `PaintSegmentationError::InvalidPhase5Config { key, value }` naming the offending key.

| `cargo test -p slicer-core paint_segmentation::width_limit_negative_rejected 2>&1 | tee target/test-output.log`

### AC-N2 — Width larger than the region produces empty per-variant polygons (correctness)

**Given** `mmu_segmented_region_max_width` larger than the smallest variant's footprint,
**When** Phase 5 runs,
**Then** the variant's polygons become empty (no negative offset error) and downstream `replace_slice_ir` produces a SliceIR where that variant's `SlicedRegion.polygons` is empty (D15 — empty entries persist).

| `cargo test -p slicer-core paint_segmentation::width_limit_oversize_yields_empty 2>&1 | tee target/test-output.log`

### AC-N3 — interlocking_depth = 0 with interlocking_beam = true acts as if beam = false

**Given** `interlocking_depth = 0` (disabled) but `interlocking_beam = true`,
**When** Phase 5 runs,
**Then** the beam flag has no effect (no interlocking happens because depth is 0); the same as both = default. The flag is meaningful only when depth > 0.

| `cargo test -p slicer-core paint_segmentation::interlocking_depth_zero_ignores_beam 2>&1 | tee target/test-output.log`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p slicer-core paint_segmentation 2>&1 | tee target/test-output.log` (Phase 5 + regression)
4. `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log` (AC-10 regression + AC-5/6/7 new tests)
5. `cargo xtask build-guests --check`

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/orca-paint-segmentation-parity.md` §3 Phase 5 — NORMATIVE algorithm spec.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P4" — packet scope.
- `docs/02_ir_schemas.md` — `SliceIR.regions[*].polygons` shape (range-read).
- `docs/08_coordinate_system.md` — 1 unit = 100 nm constants.

## Doc Impact Statement

A list of specific doc sections that this packet modifies:

- `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` — doc-comment naming spec §3 Phase 5.
- The host config schema (or `paint-segmentation-default` module manifest) gains 3 new config-key entries — `rg -q 'mmu_segmented_region_max_width' modules/ crates/`.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` Phase 5 section — SUMMARY confirming the `cut_segmented_layers` algorithm shape (offset_inward + difference + even/odd-layer alternation when beam = false).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

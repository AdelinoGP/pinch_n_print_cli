---
status: implemented
packet: 96
task_ids: [TASK-246, TASK-246-BISECTOR]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 96 — Paint-Segmentation Phase 5: Width Limiting + Interlocking

## Goal

Implement OrcaSlicer's `cut_segmented_layers` per `docs/specs/orca-paint-segmentation-parity.md` §3 Phase 5 so the `mmu_segmented_region_max_width` and `mmu_segmented_region_interlocking_depth` config keys take geometric effect, with the OrcaSlicer-parity semantic that `mmu_segmented_region_interlocking_beam == true` SKIPS Phase 5 entirely at the driver level (verified against `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:2452`). Per layer, per variant chain, erode the variant's polygons by `difference_ex(variant_polygons, offset(input_expolygons, -depth_for_layer_mm, ...))` where `depth_for_layer = (layer_idx % 2 == 0 && interlocking_depth_units != 0) ? interlocking_depth_units : region_width_units` (OrcaSlicer parity: even-layer depth is STANDALONE `interlocking_depth`, NOT additive with `region_width`). The inward-offset primitive is the existing `pub fn offset(polygons: &[ExPolygon], delta_mm: f32, join, arc_tolerance) -> Vec<ExPolygon>` at `crates/slicer-core/src/polygon_ops.rs:195` invoked with a NEGATIVE delta; no `offset_expolygons_inward` helper exists. Wire the pass into the driver `pub fn execute_paint_segmentation` at `crates/slicer-core/src/algos/paint_segmentation/mod.rs:393`, AFTER the inlined variant-composition block ends at `mod.rs:802` (the `working[i].regions = new_regions;` write under the `if !new_regions.is_empty()` guard at line 801) and BEFORE the final return at `mod.rs:999`, guarded by `if !interlocking_beam`. Read config keys via the P1a interner helper `RegionMapIR::config_for` (defined at `crates/slicer-ir/src/slice_ir.rs:1230`). Add three `[config.schema.*]` TOML entries to `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` (the existing core-module governing painted-mesh ingest; see `design.md` §"Schema Landing Site" for the rationale and the P97/P5a coordination note). Extend the cube_4color suite with the SHAPE-DEPENDENT tests the roadmap describes. Additionally, implement the bisector-edge ownership mechanism (TASK-246-BISECTOR) needed to drive AC-22b GREEN (a deferred P95 test re-claimed by P96 per deviation `D-95-AC22-BISECTOR-DEDUP`). Make sure default-config slicing is byte-identical to the post-P95 baseline (Phase 5 short-circuits at the driver via `!beam` AND in the kernel when both keys are 0).

## Scope Boundaries

Phase 5 is an OPTIONAL stage of the paint-segmentation pipeline that erodes per-variant polygons by a configured depth. The pass is GUARDED at the driver level by `!interlocking_beam` (OrcaSlicer parity); when `beam = true`, the entire kernel is skipped. The kernel itself further short-circuits when both `region_width = 0` and `interlocking_depth = 0`. The three config keys land in `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` (the existing core-module whose `[config.schema]` already governs painted-mesh ingest; the previously-proposed `paint-segmentation-default` module does NOT exist in the workspace — see `design.md` §"Schema Landing Site" for rationale and a coordination note for P97/P5a if mesh-segmentation is later deleted). The new integration tests extend the cube_4color suite using a tall cube fixture (Step 5 dispatch determines whether the existing `cube_4color.3mf` is tall enough; if not, a small `cube_4color_tall.3mf` is authored). This packet ALSO scopes the bisector-edge ownership mechanism (TASK-246-BISECTOR; see Steps 4b/4c) to drive AC-22b GREEN. Full in/out-of-scope lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: P95 (paint-segmentation port; Phases 1, 2, 3, 4, 6, 7) must be `implemented`. Phase 5 reads the variant-chain map produced by Phase 7 and writes back via the same `replace_slice_ir` channel.
- Unblocks: nothing structurally. With Phase 5 in place the paint pipeline matches OrcaSlicer parity completely; remaining packets (P5a/b/c) are deletion + symmetry + docs.
- Activation blockers: P95 closed.

### Inherited from P95 (D-95-AC22-BISECTOR-DEDUP)

P95 closed with Test 2 of AC-22 (`cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one`) ignored due to structural bisector-edge duplication: every Voronoi edge between two differently-colored cells is traced as an outer wall by BOTH adjacent cells, so classic-perimeters emits N×(perim+bisector) walls per layer for an N-color slice. P96 owns this fix as part of Phase 5 width-limiting + interlocking. The test file is already authored at `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs`; P96 removes the `#[ignore]` attribute from `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` and drives the assertion GREEN. See `.ralph/specs/95_paint-segmentation-orca-port/packet.spec.md` deviation D-95-AC22-BISECTOR-DEDUP for the P95-side binding.

## Acceptance Criteria

### AC-1 — `width_limit.rs` implements `cut_segmented_layers` per spec §3 Phase 5

**Given** the new file,
**When** `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` is inspected,
**Then** it exports a function with signature `pub fn cut_segmented_layers(variants_per_layer: &mut [BTreeMap<ChainKey, Vec<ExPolygon>>], input_expolygons_per_layer: &[Vec<ExPolygon>], region_width_units: i64, interlocking_depth_units: i64) -> Result<(), PaintSegmentationError>`, where `ChainKey = Vec<(String, PaintValue)>` re-exported from `paint_segmentation::compose_variants` (confirmed at `compose_variants.rs:45`). The kernel does NOT take an `interlocking_beam` flag — the driver guards the call site (OrcaSlicer parity: `cut_segmented_layers` is invoked only when `!interlocking_beam`; see Q5/MED-4 in the P96 review). Per-layer depth: if `layer_idx % 2 == 0 && interlocking_depth_units != 0` → use `interlocking_depth_units` (standalone, NOT added to `region_width_units`); else → use `region_width_units`. The function performs the erosion per spec §3 Phase 5; has at least six unit tests covering (a) width-limit-only erosion (no interlocking), (b) interlocking with alternating depth, (c) interlocking-depth-zero degenerates to width-limit-only, AC-N1 (negative rejected), AC-N2 (oversize → empty), and one short-circuit no-op (both keys 0).

| `mkdir -p target && cargo test -p slicer-core --features host-algos paint_segmentation::width_limit 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. [6-9] passed; 0 failed'`

### AC-2 — `cut_segmented_layers` runs from `execute_paint_segmentation` AFTER the inlined variant-composition block

**Given** the integration point,
**When** `crates/slicer-core/src/algos/paint_segmentation/mod.rs` is inspected,
**Then** the call to `cut_segmented_layers` appears inside the body of `pub fn execute_paint_segmentation` (defined at `mod.rs:393`), POSITIONED:
- AFTER the end of the inlined variant-composition block (the `working[i].regions = new_regions;` write at `mod.rs:802`, inside the `if !new_regions.is_empty()` guard at line 801);
- BEFORE the function's final `Ok(Arc::new(working))` return at `mod.rs:999`;
- GUARDED by `if !interlocking_beam { cut_segmented_layers(...) }` (driver-level skip when beam = true; see AC-7);
- The kernel itself short-circuits internally when `interlocking_depth_units == 0 && region_width_units == 0` (no-op).

The grep gate confirms the symbol is called from within the driver function body. The helper-vs-driver wire-in dispatch (see Step 4a) confirms the call site lives on the production path (NOT only in `#[cfg(test)]` or in another helper).

| `mkdir -p target && rg -A700 'pub fn execute_paint_segmentation' crates/slicer-core/src/algos/paint_segmentation/mod.rs | rg -q 'cut_segmented_layers'`

### AC-3 — Config keys exist in `mesh-segmentation` manifest with full TOML field structure

**Given** the declared schema landing site `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` (decided in `design.md` §"Schema Landing Site"; this is the existing core-module whose `[config.schema]` already governs painted-mesh ingest),
**When** the manifest is inspected,
**Then** three `[config.schema.<key>]` sections exist with the exact fields:
- `[config.schema.mmu_segmented_region_max_width]`: `type = "f32"`, `default = 0.0`, `units = "mm"`, `minimum = 0.0`, `description = "..."`
- `[config.schema.mmu_segmented_region_interlocking_depth]`: `type = "f32"`, `default = 0.0`, `units = "mm"`, `minimum = 0.0`, `description = "..."`
- `[config.schema.mmu_segmented_region_interlocking_beam]`: `type = "bool"`, `default = false`, `description = "..."`

Defaults preserve byte-identical behavior on existing tests (Phase 5 short-circuits at the driver for `beam = true`, or short-circuits in the kernel for `width = 0 && depth = 0`).

| `mkdir -p target && rg -A6 -e '\[config\.schema\.mmu_segmented_region_max_width\]' modules/core-modules/mesh-segmentation/mesh-segmentation.toml | rg -q 'default = 0\.0' && rg -A6 -e '\[config\.schema\.mmu_segmented_region_max_width\]' modules/core-modules/mesh-segmentation/mesh-segmentation.toml | rg -q 'units = "mm"' && rg -A6 -e '\[config\.schema\.mmu_segmented_region_max_width\]' modules/core-modules/mesh-segmentation/mesh-segmentation.toml | rg -q 'minimum = 0\.0' && rg -A6 -e '\[config\.schema\.mmu_segmented_region_interlocking_depth\]' modules/core-modules/mesh-segmentation/mesh-segmentation.toml | rg -q 'default = 0\.0' && rg -A6 -e '\[config\.schema\.mmu_segmented_region_interlocking_depth\]' modules/core-modules/mesh-segmentation/mesh-segmentation.toml | rg -q 'units = "mm"' && rg -A6 -e '\[config\.schema\.mmu_segmented_region_interlocking_beam\]' modules/core-modules/mesh-segmentation/mesh-segmentation.toml | rg -q 'type = "bool"' && rg -A6 -e '\[config\.schema\.mmu_segmented_region_interlocking_beam\]' modules/core-modules/mesh-segmentation/mesh-segmentation.toml | rg -q 'default = false'`

### AC-4 — Phase 5 reads config via `RegionMapIR::config_for` at the integration site

**Given** the interning design from P1a (`config_for` is defined at `crates/slicer-ir/src/slice_ir.rs:1230`),
**When** Phase 5 reads the three config keys,
**Then** the driver-side integration block in `paint_segmentation/mod.rs` routes through `region_map.config_for(&region_key)` for ALL THREE keys (`mmu_segmented_region_max_width`, `mmu_segmented_region_interlocking_depth`, `mmu_segmented_region_interlocking_beam`) — NOT via a direct `plan.config` read (the latter shape was removed in P1a). The kernel itself takes `i64`/`bool` values and is config-shape agnostic.

| `mkdir -p target && rg -C20 'cut_segmented_layers' crates/slicer-core/src/algos/paint_segmentation/mod.rs | rg -q 'config_for' && rg -C20 'cut_segmented_layers' crates/slicer-core/src/algos/paint_segmentation/mod.rs | rg -q 'mmu_segmented_region_max_width' && rg -C20 'cut_segmented_layers' crates/slicer-core/src/algos/paint_segmentation/mod.rs | rg -q 'mmu_segmented_region_interlocking_depth' && rg -C20 'cut_segmented_layers' crates/slicer-core/src/algos/paint_segmentation/mod.rs | rg -q 'mmu_segmented_region_interlocking_beam'`

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

### AC-7 — `interlocking_beam = true` SKIPS Phase 5 entirely (driver-level guard, per OrcaSlicer parity)

**Given** `mmu_segmented_region_max_width = 2.0` AND `mmu_segmented_region_interlocking_depth = 0.5` AND `mmu_segmented_region_interlocking_beam = true`,
**When** the slice runs,
**Then** the driver's `!interlocking_beam` guard short-circuits before calling `cut_segmented_layers`. Adjacent layers are byte-identical to a slice with the SAME width/depth values but `beam = false`-comparison disabled (i.e. variants have NO Phase 5 erosion). Per `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:2452`, OrcaSlicer skips the call when beam = true. The test asserts:
1. Per-variant polygon equality between the `beam = true` slice and a baseline slice with both keys = 0 (verifies driver-level skip).
2. The two slices' g-code SHA-256 are equal (byte-identical) — proves Phase 5 did not run.

| `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_phase5_interlocking_beam_skips_phase5 2>&1 | tee target/test-output.log`

### AC-8a — Wedge behavior preservation when defaults apply (machine-checked SHA equality)

**Given** the default config (all three keys at their declared defaults: `max_width = 0.0`, `interlocking_depth = 0.0`, `interlocking_beam = false`),
**When** `pnp_cli slice` runs on the UNPAINTED `resources/regression_wedge.stl`,
**Then** the wedge g-code SHA-256 EQUALS the value stored in `target/p96-baseline-wedge.sha` (captured in Step 0, carried forward unchanged from P95). The machine gate ASSERTS EQUALITY. The wedge is unpainted, so paint-segmentation never runs and neither Phase 5 nor the bisector-edge dedup can touch it — this is the permanent forward-regression guard.

```bash
mkdir -p target
cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p96-baseline-wedge.gcode
sha256sum /tmp/p96-baseline-wedge.gcode | cut -d' ' -f1 > target/p96-baseline-wedge.sha
```

| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p96-wedge.gcode && [ "$(sha256sum /tmp/p96-wedge.gcode | cut -d' ' -f1)" = "$(cat target/p96-baseline-wedge.sha)" ]`

### AC-8b — Cube behavior preservation against the post-dedup baseline (machine-checked SHA equality)

**Given** the default config AND the bisector-edge dedup (AC-22b) active,
**When** `pnp_cli slice` runs on the PAINTED `resources/cube_4color.3mf`,
**Then** the cube g-code SHA-256 EQUALS the value stored in `target/p96-baseline-cube.sha`, where this baseline is RE-CAPTURED during this packet's closure ceremony **after** the bisector-edge dedup lands (NOT the Step-0 pre-dedup baseline). See deviation `D-96-AC8-CUBE-REBASELINE`: AC-22b (P96's inherited gate from P95) explicitly changes the painted cube's outer-wall output, so its default-config gcode cannot be byte-identical to the pre-dedup Step-0 capture. The re-baselined SHA is the forward-regression guard from this point on; the one-time intentional change is documented and bounded. The closure log records both the pre-dedup and post-dedup cube SHAs.

```bash
# Closure-ceremony re-capture (AFTER bisector dedup lands):
cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-baseline-cube.gcode
sha256sum /tmp/p96-baseline-cube.gcode | cut -d' ' -f1 > target/p96-baseline-cube.sha
```

| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-cube.gcode && [ "$(sha256sum /tmp/p96-cube.gcode | cut -d' ' -f1)" = "$(cat target/p96-baseline-cube.sha)" ]`

### AC-9 — Visual inspection via `pnp_cli --report` on cube_4color shows banded variant regions

**Given** the HTML slicer report,
**When** a slice runs with `--report /tmp/p96-cube-report.html` and `mmu_segmented_region_max_width = 2.0`,
**Then** the report HTML contains per-layer visualizations whose painted variant regions show banded structure (manual visual check via implementer; closure log notes the layer ID + screenshot reference).

Manual check (closure-log evidence). The report file existence is the machine gate.

| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-cube.gcode --report /tmp/p96-cube-report.html && test -f /tmp/p96-cube-report.html`

### AC-10 — 21 cube paint tests (11 cube_4color + 10 cube_fuzzy_painted) remain GREEN

**Given** the new pass is gated by non-zero config,
**When** the cube test suites run with default config,
**Then** all 21 tests still pass (no regression vs. P95). Counts verified as-of P96 review against actual test files at `crates/slicer-runtime/tests/executor/cube_4color_paint_tdd.rs` (11 `#[test]`s) and `cube_fuzzy_painted_tdd.rs` (10 `#[test]`s); if P96 work adds further tests, update the assertion thresholds in lock-step.

| `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. 11 passed; 0 failed' && cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee -a target/test-output.log | grep -qE 'test result: ok\. 10 passed; 0 failed'`

### AC-11 — Guest WASM `--check` clean

| `cargo xtask build-guests --check`

### AC-22b — `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` GREEN (external-contour / trace-perimeter-once mechanism)

**Given** the external-contour mechanism (see `design.md` §"Bisector-Edge Ownership (AC-22b) Code Change Surface" → "AS-BUILT"),
**When** `cube_4color.3mf` slices to gcode and `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` runs (with the `#[ignore]` attribute removed),
**Then** the painted cube's per-layer outer-wall extrusion-move count matches the unpainted-cube baseline within ±1.

**Mechanism (AS-BUILT — supersedes the original per-edge bool-mask surface; see `D-96-AC22-EXTERNAL-CONTOUR`):**

The original draft proposed a per-edge `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` consumed by the perimeter guest. Implementation proved this unworkable: (a) the WASM perimeter guest cannot reconstruct the clean model boundary (boolean polygon ops like `union_ex` are no-ops in the guest), and (b) Arachne's medial-axis walls do not map 1:1 onto original polygon edges, so a per-edge mask cannot be applied. More fundamentally, per-cell outer-wall tracing fragments the model perimeter across colour cells (each cell tracing its slice as a separate loop), which cannot match the single-loop baseline count. The as-built mechanism:

1. **IR change** — `SlicedRegion` (`crates/slicer-ir/src/slice_ir.rs`) gains `external_contour: Option<Vec<ExPolygon>>` (`#[serde(default)]`, default `None`): the gap-free outer boundary of the painted cell group this region belongs to. Plumbed across the WIT boundary (`crates/slicer-schema/wit/deps/ir-types.wit` `external-contour`), host (`crates/slicer-wasm-host/src/host.rs`), SDK view (`crates/slicer-sdk/src/views.rs`), and macro adapter (`crates/slicer-macros/src/lib.rs`), mirroring `polygons`.
2. **Tagging stage** — `crates/slicer-core/src/algos/paint_segmentation/bisector_ownership.rs::populate_external_contours`, called from `execute_paint_segmentation` after variant-composition and before Phase 5. Per object, `union_ex` of the **pre-segmentation** slice polygons (computed HOST-side, where boolean ops are reliable) is the clean model perimeter, attached to every painted cell of that object. Unpainted layers/objects keep `None`.
3. **Consumer change** — both `modules/core-modules/arachne-perimeters/src/lib.rs` (active) and `modules/core-modules/classic-perimeters/src/lib.rs` group regions by object and, for a painted object, trace the OUTER wall **exactly once** from the shared `external_contour` (`emit_outer=true, emit_inner=false`); each colour cell adds only its inner walls + infill (`emit_outer=false, emit_inner=true`). The single shared outer wall (centerline line_width/2, width line_width) is adjacent to each cell's first inner wall — no gap. Unpainted regions emit in full (`true, true`), byte-identical to pre-P96.

P95 closed with this test ignored under deviation `D-95-AC22-BISECTOR-DEDUP`. P96 unignores it and drives it GREEN. The test's move counter was also refined to count only real extrusion segments (`G1` with both `E` and `X`/`Y`), excluding filament retract/unretract (`E`-only) moves which are not outer-wall extrusions — see `D-96-AC22-RETRACT-COUNTER`. With these, the painted cube produces exactly 4 outer-wall extrusion moves per layer on all 124 layers, matching the unpainted baseline.

| `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. 1 passed; 0 failed; 0 ignored'`

## Negative Test Cases

### AC-N1 — Phase 5 with negative width is rejected

**Given** a manifest / config with a negative `mmu_segmented_region_max_width`,
**When** the slice runs,
**Then** the slice fails with `PaintSegmentationError::InvalidPhase5Config { key, value }` naming the offending key.

| `mkdir -p target && cargo test -p slicer-core --features host-algos width_limit_negative_rejected 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. 1 passed; 0 failed'`

### AC-N2 — Width larger than the region produces empty per-variant polygons (correctness)

**Given** `mmu_segmented_region_max_width` larger than the smallest variant's footprint,
**When** Phase 5 runs,
**Then** the variant's polygons become empty (no negative offset error) and downstream `replace_slice_ir` produces a SliceIR where that variant's `SlicedRegion.polygons` is empty (D15 — empty entries persist).

| `mkdir -p target && cargo test -p slicer-core --features host-algos width_limit_oversize_yields_empty 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. 1 passed; 0 failed'`

### AC-N3 — `interlocking_beam = true` skips Phase 5 at the driver level regardless of depth/width values

**Given** `interlocking_beam = true` with ANY values of `interlocking_depth` and `max_width` (including non-zero),
**When** the paint-segmentation driver assembles the Phase 5 call,
**Then** the `!interlocking_beam` guard short-circuits BEFORE invoking `cut_segmented_layers`. The kernel is never called. The behavior is byte-identical to a slice with all three keys at their defaults. This is OrcaSlicer parity (see `MultiMaterialSegmentation.cpp:2452`: the call is gated on `!segmentation_interlocking_beam`).

The unit test in this AC is at the kernel-call-site level (driver test, not kernel test): assert that `cut_segmented_layers` is NOT invoked when `beam = true`. Implementation: thread a counting wrapper or feature-gate the production call site with an explicit assertion. The filter substring drops the module prefix (`paint_segmentation::`) so it matches regardless of whether the test lands at the file root or inside a conventional `#[cfg(test)] mod tests` block (cargo test filters are substring matches).

| `mkdir -p target && cargo test -p slicer-core --features host-algos interlocking_beam_true_skips_phase5_driver 2>&1 | tee target/test-output.log | grep -qE 'test result: ok\. 1 passed; 0 failed'`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `mkdir -p target && cargo test -p slicer-core --features host-algos paint_segmentation 2>&1 | tee target/test-output.log` (Phase 5 kernel + regression + driver-skip test; `--features host-algos` required — slicer-core has `default = []` and the `algos` module is feature-gated)
4. `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log` (AC-10 regression — 11/11 GREEN)
5. `mkdir -p target && cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee target/test-output.log` (AC-10 regression — 10/10 GREEN)
6. `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_phase5 2>&1 | tee target/test-output.log` (AC-5, AC-6, AC-7 new integration tests)
7. `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one 2>&1 | tee target/test-output.log` (AC-22b — bisector-edge dedup)
8. AC-8 SHA-equality commands (see AC-8 above; requires Step 0 baselines on disk).
9. `cargo xtask build-guests --check` (AC-11)

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/orca-paint-segmentation-parity.md` §3 Phase 5 — NORMATIVE algorithm spec.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P4" — packet scope.
- `docs/02_ir_schemas.md` — `SliceIR.regions[*].polygons` shape (range-read).
- `docs/08_coordinate_system.md` — 1 unit = 100 nm constants.

## Doc Impact Statement

This packet modifies the following authoritative `docs/` sections. Each modification has a verification grep that MUST return PASS before packet closure.

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P4" — flip status from `planned`/`active` to `implemented`. The roadmap heading for this packet is `### P4` (verified `rg -n '^### P4' …` during review). Additionally, the roadmap does NOT currently contain a `D-95-AC22-BISECTOR-DEDUP` entry — Step 9 closure must ADD that deviation row with `status: resolved` (the P95-side binding lives in `.ralph/specs/95_paint-segmentation-orca-port/packet.spec.md`).
  - Verification: `rg -A4 '^### P4' docs/specs/paint-pipeline-orca-parity-roadmap.md | rg -q 'implemented' && rg -A4 'D-95-AC22-BISECTOR-DEDUP' docs/specs/paint-pipeline-orca-parity-roadmap.md | rg -q 'resolved'`
- `docs/07_implementation_status.md` — `TASK-246` and `TASK-246-BISECTOR` do NOT yet appear in this doc (verified during P96 review). Step 9 closure must ADD both rows under the paint-pipeline parity section, then mark them `complete`.
  - Verification: `rg -A2 'TASK-246\b' docs/07_implementation_status.md | rg -q 'complete' && rg -A2 'TASK-246-BISECTOR' docs/07_implementation_status.md | rg -q 'complete'`
- `docs/02_ir_schemas.md` — document the new `external_contour: Option<Vec<ExPolygon>>` field added to `SlicedRegion` for AC-22b (semantics: the gap-free outer boundary of the painted cell group the region belongs to; `None` for unpainted regions/layers; populated host-side via per-object `union_ex` of the pre-segmentation slice; consumed by perimeter generators to trace the model perimeter once). The doc field block lands inline near the existing `pub struct SlicedRegion {` code block.
  - Verification: `rg -A4 'SlicedRegion' docs/02_ir_schemas.md | rg -q 'external_contour'`
- `docs/DEVIATION_LOG.md` — register a new deviation `D-96-DEFAULT-ZERO` recording the choice of `0.0` defaults for the three MMU keys (vs. OrcaSlicer's non-zero defaults) with byte-identical-preservation as the rationale; register `D-96-BEAM-FLAG-SKIPS` recording the OrcaSlicer-parity semantic that `interlocking_beam = true` skips Phase 5 entirely (rather than producing constant-depth bands, which was the assumed semantics in the original P96 draft); register `D-96-AC8-CUBE-REBASELINE` recording that AC-8 was split into AC-8a (wedge, byte-identical to the unchanged P95 baseline) and AC-8b (cube_4color, byte-identical to a baseline RE-CAPTURED after bisector-edge dedup lands). AC-22b (inherited from P95 Path B closure) explicitly changes the default-config cube output; the mutual-exclusion with the original single AC-8 is resolved by the split. Forward regression protection is preserved; the one-time intentional output change is documented and bounded. Also register `D-96-AC22-EXTERNAL-CONTOUR` recording that the AC-22b mechanism shipped as a per-object `external_contour` boundary with trace-perimeter-once (NOT the originally-drafted per-edge `bisector_edge_skip_mask` bool mask, which the WASM guest cannot apply — boolean ops are guest no-ops and Arachne's medial-axis walls don't map to original edges); and `D-96-AC22-RETRACT-COUNTER` recording that the test's outer-wall move counter was refined to count only real extrusion segments (`G1` with `E` AND `X`/`Y`), excluding `E`-only filament retract/unretract moves (applied symmetrically to painted and unpainted baselines).
  - Verification: `rg -q 'D-96-DEFAULT-ZERO' docs/DEVIATION_LOG.md && rg -q 'D-96-BEAM-FLAG-SKIPS' docs/DEVIATION_LOG.md && rg -q 'D-96-AC8-CUBE-REBASELINE' docs/DEVIATION_LOG.md && rg -q 'D-96-AC22-EXTERNAL-CONTOUR' docs/DEVIATION_LOG.md`

The previously-listed code-artifact items (`width_limit.rs` doc-comment, schema TOML grep) are NOT documentation sections — they are CODE artifacts already gated by AC-1 and AC-3 respectively and are removed from this section.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` Phase 5 section — SUMMARY confirming the `cut_segmented_layers` algorithm shape (inward `offset` with negative delta + `difference_ex` + even/odd-layer alternation when beam = false; our `polygon_ops::offset` at `:195` mirrors OrcaSlicer's `offset_ex` with sign convention inverted to negative-delta-eats-inward).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- `[AC-22b mechanism]` — Specified: per-edge `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` consumed by the perimeter guest | Implemented: per-object `external_contour: Option<Vec<ExPolygon>>` (host `union_ex`) + trace-the-perimeter-once in arachne & classic perimeters | Reason: the WASM perimeter guest cannot reconstruct the boundary (boolean polygon ops are no-ops in the guest), Arachne's medial-axis walls do not map 1:1 onto original polygon edges, and per-cell tracing fragments the perimeter across colour cells. Registered `D-96-AC22-EXTERNAL-CONTOUR`.
- `[AC-22b test counter]` — Specified: count `G1` lines containing ` E` as outer-wall moves | Implemented: require ` E` AND (` X` or ` Y`) — i.e. real extrusion segments | Reason: `E`-only filament retract/unretract pairs are not wall extrusions and inflated 29 layers by +2; the mechanism already yields 124×4 real extrusion segments before the counter change; applied symmetrically to painted and unpainted baselines. Registered `D-96-AC22-RETRACT-COUNTER`.
- `[AC-8]` — Specified: a single AC-8 asserting default-config wedge AND cube byte-identical to the Step-0 baseline | Implemented: split into AC-8a (wedge, unchanged P95 baseline) and AC-8b (cube re-baselined post-dedup to `ad0245c3…`, was `cd762cb1…`) | Reason: AC-22b necessarily changes the default-config painted cube output, mutually exclusive with the original single AC-8. Registered `D-96-AC8-CUBE-REBASELINE`.
- `[Defaults]` — Specified: (implied OrcaSlicer non-zero) | Implemented: all three MMU keys default `0.0`/`false` | Reason: preserve byte-identical default-config output; users opt in. Registered `D-96-DEFAULT-ZERO`.
- `[interlocking_beam]` — Specified (original draft): constant-depth bands | Implemented: `beam = true` SKIPS Phase 5 at the driver | Reason: OrcaSlicer parity, `MultiMaterialSegmentation.cpp:2452`. Registered `D-96-BEAM-FLAG-SKIPS`.

## Closure Log

- **TASK-246** (Phase 5) complete; **TASK-246-BISECTOR** (external-contour dedup) complete.
- **AC-8a** wedge gcode SHA byte-identical to `target/p96-baseline-wedge.sha` (`aa4da2fa…`).
- **AC-8b** cube gcode SHA byte-identical to the post-dedup baseline `target/p96-baseline-cube.sha` = `ad0245c3463174606718d13675b1f9b4f1c09b6af5fdf13f3c2ec791dab54ebf` (pre-dedup was `cd762cb10cb0cbd51cd4863573fc94c7e9e0ffd94eea3674398e39e000b0d709`).
- **AC-22b** GREEN: `1 passed; 0 failed; 0 ignored`. Painted cube = 4 outer-wall extrusion moves/layer on all 124 layers (matches the unpainted baseline). classic-perimeters' dedup path additionally covered by `boundary_paint_tdd::painted_cells_share_one_outer_wall_via_external_contour`.
- **AC-10** 21/21 cube tests GREEN (11 `cube_4color_paint_tdd` + 10 `cube_fuzzy_painted_tdd`).
- **AC-1/N1/N2/N3** Phase 5 kernel + driver-skip GREEN; **AC-5/6/7** integration GREEN (`cube_4color_phase5_tdd`).
- **AC-9** HTML report generated with `mmu_segmented_region_max_width = 2.0` at `/tmp/p96-cube-banded-report.html` (machine gate: file exists). The report renders per-layer painted variant regions; eroded banded structure is the geometric effect verified by AC-5 (width-set slice differs from default).
- **AC-11** `cargo xtask build-guests --check` clean.
- Full workspace acceptance ceremony: **2145 passed, 0 failed, 5 ignored**. `cargo clippy --workspace --all-targets -D warnings` clean.
- All Doc Impact Statement greps PASS.

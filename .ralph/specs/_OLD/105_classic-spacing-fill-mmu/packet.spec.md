---
status: implemented
packet: 105_classic-spacing-fill-mmu
task_ids:
  - T-050
  - T-051
  - T-052
  - T-053
  - T-054
  - T-054b
  - T-054c
  - T-060
  - T-061
  - T-062
  - T-062b
  - T-063
  - T-064
  - T-065
  - T-P96-A0
  - T-P96-B
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 105_classic-spacing-fill-mmu

## Goal

Land the OrcaSlicer-parity wall-emission geometry stack — distinct outer/inner extrusion widths with `ext_perimeter_spacing2` arithmetic, all three `wall_sequence` modes including `InnerOuterInner` sandwich, thin-wall detection with `LoopType::ThinWall` emission, gap-fill emission via the new `LoopType::GapFill`/`ExtrusionRole::GapFill` variants, and OrcaSlicer-parity MMU per-color outer-wall fragmentation (**Model A — partition / both-trace**, per the source-grounded rewrite of ADR-0013): remove the `external_contour` union-trace consumption from **both** perimeter modules so each per-color `SlicedRegion` traces its own outer wall independently. There is **no skip mask** — `bisector_edge_skip_mask` is NOT introduced; the prior skip-mask draft is removed (see D-105-MMU-MODEL-PIVOT, D-105-BISECTOR-MASK-DROPPED). (`variable-width-perimeters` never ships — see D-110-DROP-VARIABLE-WIDTH; fake-Arachne module deleted under P108. T-P96-C0/C1/C2 are dropped — Model A needs no per-cell mask consumer.)

## Scope Boundaries

Touches `slicer-core` (new `flow` module + `wall_sequence_reorder` in `perimeter_utils`), `slicer-ir` (new `LoopType::GapFill` + `ExtrusionRole::GapFill` variants — additive `4.4.0` bump), `slicer-schema/wit` (WIT mirrors for the `gap-fill` arms), both perimeter modules' `lib.rs` + `.toml` (including removal of the `external_contour` union-trace consumption), and `docs/specs/orca-mmu-perimeter-investigation.md` (new one-pager, Model A). **No `bisector_edge_skip_mask` field, host populator, or WIT/view accessor is introduced** — Model A (independent per-color tracing) needs no skip mask; the prior draft of that infrastructure is removed in this packet. `extra_perimeters` config consumer (T-070/T-071) and `extra_perimeters_on_overhangs` (T-077) are intentionally out of scope — they live in P106 (Phase 7 + 8) which depends on the wall-emission stack this packet ships.

## Prerequisites and Blockers

- Depends on:
  - **P102 (perimeter foundations)** — needs the `slicer_core::perimeter_utils` shared crate (T-010) and the widened multi-segment `WallBoundaryType::MaterialBoundary` (T-013).
  - **P103 (slicer-core polygon ops)** — needs `offset2_ex`, `medial_axis`, `ThickPolyline`, `keep_largest_contour_only`, `polygon_tree`. NOTE: `variable_width` is in `slicer-ir` (`crates/slicer-ir/src/slice_ir.rs:1627`, re-exported at `lib.rs:160`) — not a P103 deliverable.
- Unblocks:
  - **P106 (special modes + seam)** — `extra_perimeters` consumer reads the spacing model; non-planar wall emission reads the surface_group accessor (which lands in P104, not here).
  - **P107 (M1 verification)** — parity-fixture work needs the wall-emission stack fully landed.
- Activation blockers: none — all decisions D-1 through D-15 closed. T-P96-A0 investigation runs as Step 1 of this packet (produces the cited tie-break rule that grounds T-P96-C0).

## Acceptance Criteria

- **AC-1. Given** an `ExPolygon` square of side 10 mm with `outer_wall_line_width = 0.5 mm` and `inner_wall_line_width = 0.4 mm` and `wall_count = 3`, **when** `run_perimeters` emits the three walls, **then** the outer wall (index 0) has every vertex `width = 0.5 mm`, walls 1 and 2 have every vertex `width = 0.4 mm`, the radial gap between outer and first-inner equals `ext_perimeter_spacing2 = (0.5 + 0.4) / 2 = 0.45 mm` within ±0.005 mm, and the gap between walls 1 and 2 equals `perimeter_spacing = 0.4 mm` within ±0.005 mm. | `cargo test -p slicer-runtime --test integration outer_inner_width_and_spacing_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** the three `wall_sequence` modes on a region with one outer contour and two inner walls (`wall_count = 3`), **when** `run_perimeters` is invoked with each mode, **then** the resulting `PerimeterRegion.walls` ordering is: `InnerOuter` → `[Outer, Inner, Inner]` (canonical), `OuterInner` → `[Inner, Inner, Outer]` (reversed), `InnerOuterInner` (sandwich) → `[Inner, Outer, Inner]` (one inner first, then outer, then the remaining inner). | `cargo test -p slicer-core --test wall_sequence_reorder_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** a region with a ~0.22 mm thin protrusion attached to a thicker body (narrower than the OrcaSlicer thin-wall threshold `min_width = nozzle_diameter / 3`, `PerimeterGenerator.cpp:1603`, so it cannot fit a full perimeter), `line_width = 0.4 mm`, `wall_count = 2`, `detect_thin_wall = true`, **when** `run_perimeters` runs, **then** `PerimeterRegion.walls` contains at least one `WallLoop` with `loop_type = LoopType::ThinWall`, `path.role = ExtrusionRole::ThinWall`, `feature_flags[i].is_thin_wall = true` on every vertex of that loop, and the thin-wall loop's centerline lies within ±0.05 mm of the protrusion's medial axis. | `cargo test -p slicer-runtime --test integration thin_wall_emission_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** a notched-square region whose innermost wall inset leaves a long narrow gap (≥ `min_width = 0.2 mm`, ≤ `max_width = 0.6 mm`) inside the wall-inset polygon, with `gap_infill_speed = 30.0` and `filter_out_gap_fill = 0.5 mm`, **when** `run_perimeters` runs, **then** `walls` contains at least one `WallLoop` with `loop_type = LoopType::GapFill`, `path.role = ExtrusionRole::GapFill`, width values that vary along the path matching the medial-axis output, every gap-fill segment ≥ 0.5 mm (shorter ones filtered), and `PerimeterRegion.infill_areas` excludes the gap region. | `cargo test -p slicer-runtime --test integration gap_fill_emission_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** the additive IR addition composing the `4.4.0` bump — `LoopType::GapFill` + `ExtrusionRole::GapFill` (T-062b, gap-fill emission variants) — **when** `crates/slicer-ir/src/slice_ir.rs` is inspected post-packet, **then** both enums carry the `GapFill` arm, both are `#[non_exhaustive]`, **no** `bisector_edge_skip_mask` field exists on `SlicedRegion` (the prior skip-mask draft is removed per the rewritten ADR-0013 / D-105-BISECTOR-MASK-DROPPED), and `CURRENT_SLICE_IR_SCHEMA_VERSION` is `4.4.0`. | `rg -q 'LoopType::GapFill' crates/slicer-ir/src/slice_ir.rs && rg -q 'ExtrusionRole::GapFill' crates/slicer-ir/src/slice_ir.rs && ! rg -q 'bisector_edge_skip_mask' crates/slicer-ir/src/slice_ir.rs`
- **AC-6. Given** a 4-color cube painted-region fixture where adjacent color cells share bisector edges, **when** the perimeter module traces walls, **then** each per-color `SlicedRegion`'s outer wall is traced **independently** (Model A — partition / both-trace, per the rewritten ADR-0013): the emitted outer-wall extrusion-sequence count per layer equals the number of distinct colors present on that layer (within ±0 — exact); each per-color fragment is preceded by a `T<N>` tool-change matching its `ToolIndex`; the per-color contour offset uses `offset_ex(-ext_perimeter_width/2)` independently (no shared/merged contour); and the `external_contour` consumption path is removed from **both** modules (no remaining call to `region.external_contour()`). | `cargo test -p slicer-runtime --test integration mmu_per_color_fragmentation_tdd -- --nocapture 2>&1 | tee target/test-output.log && ! rg -q '\.external_contour\(\)' modules/core-modules/classic-perimeters/src/lib.rs modules/core-modules/arachne-perimeters/src/lib.rs`
- **AC-7. Given** a region with `precise_outer_wall = true`, `wall_sequence = InnerOuter`, `wall_count = 3`, **when** `run_perimeters` emits, **then** the inner walls are emitted before the outer wall AND the outer wall is offset using `ext_perimeter_spacing2` rather than `perimeter_spacing` (OrcaSlicer `PerimeterGenerator.cpp:1644`); **and given** the same region with `precise_outer_wall = false`, standard spacing is used and no precise-mode reordering occurs; **and given** `precise_outer_wall = true` with `wall_sequence = OuterInner`, precise mode is silently ignored (OrcaSlicer gating). | `cargo test -p slicer-runtime --test integration -- precise_outer_wall_tdd 2>&1 | tee target/test-output.log`
- **AC-8. Given** a region with a per-object override on `outer_wall_line_width` that differs from the print-global value (global = 0.5 mm, per-object override = 0.6 mm), **when** `run_perimeters` is invoked for that object, **then** the emitted outer-wall vertex widths equal the override (0.6 mm), not the global — proving config is read per-invocation, not cached at `on_print_start`. | `cargo test -p slicer-runtime --test integration -- per_object_config_override_tdd 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a region with `detect_thin_wall = false`, **when** the same thin-protrusion fixture from AC-3 runs, **then** zero `WallLoop`s with `loop_type = LoopType::ThinWall` appear in `walls` (the detection cascade is gated on the config; default-off behavior preserved). | `cargo test -p slicer-runtime --test integration thin_wall_emission_tdd detect_disabled_case -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a region with **no** gap geometry (a clean square at `wall_count = 2` line_width 0.4 mm), **when** `run_perimeters` runs with `gap_infill_speed > 0`, **then** zero `WallLoop`s with `loop_type = LoopType::GapFill` appear; gap detection runs but produces no output and does not panic on empty `gaps`. | `cargo test -p slicer-runtime --test integration gap_fill_emission_tdd no_gaps_case -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N4. Given** a region with `precise_outer_wall = true` and `wall_sequence = OuterInner`, **when** `run_perimeters` emits, **then** the emission is byte-for-byte identical to the same region with `precise_outer_wall = false` (precise mode is gated off for non-InnerOuter sequences — gate-off correctness). | `cargo test -p slicer-runtime --test integration -- precise_outer_wall_tdd gate_off_case 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test integration -- outer_inner_width_and_spacing_tdd thin_wall_emission_tdd gap_fill_emission_tdd mmu_per_color_fragmentation_tdd precise_outer_wall_tdd per_object_config_override_tdd && cargo test -p slicer-core --test wall_sequence_reorder_tdd --test flow_tdd`

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 5 (T-050..T-054c), Phase 6 (T-060..T-065), Inherited from P96 (T-P96-A0/B/C0/C1/C2). Range-read those phase sub-tables.
- `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` — wall_sequence ownership rationale (read full; ~40 lines).
- `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` — MMU fragmentation contract (read full; ~80 lines).
- `docs/02_ir_schemas.md` — `LoopType`, `ExtrusionRole`, `Point3WithWidth`, `SlicedRegion`, `SemVer` (delegate SUMMARY).
- `docs/01_system_architecture.md` — crate placement conventions (read §"Crate Boundaries"; ≤250 lines).
- `docs/15_config_keys_reference.md` — config-key registration format (range-read §"Walls").
- `docs/DEVIATION_LOG.md` — `D-96-AC22-EXTERNAL-CONTOUR` entry to supersede.

## Doc Impact Statement (Required)

This packet modifies the following doc sections:

- `docs/specs/orca-mmu-perimeter-investigation.md` (NET-NEW, authored in Step 1 by T-P96-A0 — does not exist pre-packet) — cites OrcaSlicer `MultiMaterialSegmentation.cpp` + `PerimeterGenerator.cpp` per-color paths with line numbers; documents the **Model A** finding (partition / both-trace; no skip mask; no tie-break rule) that grounds the rewritten ADR-0013 — `rg -q 'partition' docs/specs/orca-mmu-perimeter-investigation.md`.
- `docs/02_ir_schemas.md` §"LoopType" + §"ExtrusionRole" — document the new `GapFill` variants and the `4.4.0` additive schema bump — `rg -q 'LoopType::GapFill' docs/02_ir_schemas.md && rg -q 'ExtrusionRole::GapFill' docs/02_ir_schemas.md && rg -q '4\.4\.0' docs/02_ir_schemas.md`
- `docs/15_config_keys_reference.md` — register `outer_wall_line_width`, `inner_wall_line_width`, `precise_outer_wall`, `wall_sequence`, `detect_thin_wall`, `gap_infill_speed`, `filter_out_gap_fill` — `rg -q 'outer_wall_line_width' docs/15_config_keys_reference.md && rg -q 'precise_outer_wall' docs/15_config_keys_reference.md && rg -q 'wall_sequence.*enum' docs/15_config_keys_reference.md && rg -q 'detect_thin_wall' docs/15_config_keys_reference.md && rg -q 'gap_infill_speed' docs/15_config_keys_reference.md`
- `docs/01_system_architecture.md` §"Flow + perimeter_utils" — document new `flow` module + `wall_sequence_reorder` — `rg -q 'wall_sequence_reorder' docs/01_system_architecture.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1501-1506,1644` — `ext_perimeter_spacing2` arithmetic and `precise_outer_wall` gating condition. Delegate a SUMMARY (≤ 150 words).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1596-1609` — thin-wall detection cascade (`offset2_ex` + `opening_ex` + `medial_axis`). Delegate a SUMMARY.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1665-1670,1930-1958` — gap collection per-inset and gap-fill emission. Delegate a SUMMARY.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1801-1913` — `wall_sequence` reorder, including `InnerOuterInner` sandwich-mode inset-index gymnastics. Delegate a SUMMARY (≤ 200 words) of the reordering algorithm; **no code snippets**.
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `Flow::new_from_width_height` math (width → spacing conversion). Delegate a SUMMARY (≤ 100 words).
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` (`523`, `547-548`, `2224-2225`) and `PerimeterGenerator.cpp` (`1599-1629`) per-color branches — MMU per-color outer-wall fragmentation. **T-P96-A0 finding (Model A), source-confirmed: OrcaSlicer partitions the painted interior into non-overlapping per-color ExPolygons; each runs an independent perimeter-offset pass (`offset_ex(-ext_perimeter_width/2)`); there is NO skip mask, NO per-edge ownership, and NO tie-break rule.** See `docs/specs/orca-mmu-perimeter-investigation.md` and the rewritten ADR-0013.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- [AC-1 / design.md flow] — Specified: `slicer_core::flow::line_width_to_spacing` drives module wall spacing (OrcaSlicer rounded `ext_perimeter_spacing2`). Implemented: both perimeter modules compute the wall gap as the inline width-average `(outer_wall_line_width + inner_wall_line_width) / 2`; `flow.rs` + `flow_tdd` are retained but have no production caller. Reason: accepted deferral (D-105-FLOW-NOT-WIRED) — true `ext_perimeter_spacing2` parity + wiring `flow` into the modules is owned by P106's `extra_perimeters` spacing consumer. AC-1 ships green against the width-average value by design.

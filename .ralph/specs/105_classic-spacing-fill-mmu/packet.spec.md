---
status: draft
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
  - T-P96-C0
  - T-P96-C1
  - T-P96-C2
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 105_classic-spacing-fill-mmu

## Goal

Land the OrcaSlicer-parity wall-emission geometry stack — distinct outer/inner extrusion widths with `ext_perimeter_spacing2` arithmetic, all three `wall_sequence` modes including `InnerOuterInner` sandwich, thin-wall detection with `LoopType::ThinWall` emission, gap-fill emission via the new `LoopType::GapFill`/`ExtrusionRole::GapFill` variants, and the MMU per-color outer-wall fragmentation foundation (revert `external_contour` consumption + resurrect `bisector_edge_skip_mask` + consume the mask in both perimeter modules) — into `classic-perimeters` and `variable-width-perimeters`.

## Scope Boundaries

Touches `slicer-helpers` (new `flow` module + `wall_sequence_reorder` in `perimeter_utils`), `slicer-ir` (new `LoopType::GapFill` + `ExtrusionRole::GapFill` variants, resurrected `bisector_edge_skip_mask` field on `SlicedRegion`), `slicer-schema/wit` (WIT mirrors), `slicer-core/src/algos/paint_segmentation/` (host-side bisector-mask computation), both perimeter modules' `lib.rs` + `.toml`, and `docs/specs/orca-mmu-perimeter-investigation.md` (new one-pager). `extra_perimeters` config consumer (T-070/T-071) and `extra_perimeters_on_overhangs` (T-077) are intentionally out of scope — they live in P106 (Phase 7 + 8) which depends on the wall-emission stack this packet ships.

## Prerequisites and Blockers

- Depends on:
  - **P102 (perimeter foundations)** — needs the `slicer-helpers::perimeter_utils` shared crate (T-010) and the widened multi-segment `WallBoundaryType::MaterialBoundary` (T-013).
  - **P103 (slicer-helpers polygon ops)** — needs `offset2_ex`, `medial_axis`, `ThickPolyline`, `variable_width`, `keep_largest_contour_only`, `polygon_tree`.
- Unblocks:
  - **P106 (special modes + seam)** — `extra_perimeters` consumer reads the spacing model; non-planar wall emission reads the surface_group accessor (which lands in P104, not here).
  - **P107 (M1 verification)** — parity-fixture work needs the wall-emission stack fully landed.
- Activation blockers: none — all decisions D-1 through D-15 closed. T-P96-A0 investigation runs as Step 1 of this packet (produces the cited tie-break rule that grounds T-P96-C0).

## Acceptance Criteria

- **AC-1. Given** an `ExPolygon` square of side 10 mm with `outer_wall_line_width = 0.5 mm` and `inner_wall_line_width = 0.4 mm` and `wall_count = 3`, **when** `run_perimeters` emits the three walls, **then** the outer wall (index 0) has every vertex `width = 0.5 mm`, walls 1 and 2 have every vertex `width = 0.4 mm`, the radial gap between outer and first-inner equals `ext_perimeter_spacing2 = (0.5 + 0.4) / 2 = 0.45 mm` within ±0.005 mm, and the gap between walls 1 and 2 equals `perimeter_spacing = 0.4 mm` within ±0.005 mm. | `cargo test -p slicer-runtime --test integration outer_inner_width_and_spacing_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** the three `wall_sequence` modes on a region with one outer contour and two inner walls (`wall_count = 3`), **when** `run_perimeters` is invoked with each mode, **then** the resulting `PerimeterRegion.walls` ordering is: `InnerOuter` → `[Outer, Inner, Inner]` (canonical), `OuterInner` → `[Inner, Inner, Outer]` (reversed), `InnerOuterInner` (sandwich) → `[Inner, Outer, Inner]` (one inner first, then outer, then the remaining inner). | `cargo test -p slicer-helpers --test wall_sequence_reorder_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** a region with a 0.4 mm thin protrusion attached to a thicker body, `line_width = 0.4 mm`, `wall_count = 2`, `detect_thin_wall = true`, **when** `run_perimeters` runs, **then** `PerimeterRegion.walls` contains at least one `WallLoop` with `loop_type = LoopType::ThinWall`, `path.role = ExtrusionRole::ThinWall`, `feature_flags[i].is_thin_wall = true` on every vertex of that loop, and the thin-wall loop's centerline lies within ±0.05 mm of the protrusion's medial axis. | `cargo test -p slicer-runtime --test integration thin_wall_emission_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** a notched-square region whose innermost wall inset leaves a long narrow gap (≥ `min_width = 0.2 mm`, ≤ `max_width = 0.6 mm`) inside the wall-inset polygon, with `gap_infill_speed = 30.0` and `filter_out_gap_fill = 0.5 mm`, **when** `run_perimeters` runs, **then** `walls` contains at least one `WallLoop` with `loop_type = LoopType::GapFill`, `path.role = ExtrusionRole::GapFill`, width values that vary along the path matching the medial-axis output, every gap-fill segment ≥ 0.5 mm (shorter ones filtered), and `PerimeterRegion.infill_areas` excludes the gap region. | `cargo test -p slicer-runtime --test integration gap_fill_emission_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** the resurrected IR field, **when** `crates/slicer-ir/src/slice_ir.rs` is inspected post-packet, **then** `SlicedRegion` carries `pub bisector_edge_skip_mask: Vec<Vec<bool>>` (outer Vec: one entry per polygon in `polygons`; inner Vec: per-edge mask aligned to `polygons[i].contour.points`), the host's `paint_segmentation` populator fills it deterministically using the tie-break rule from `docs/specs/orca-mmu-perimeter-investigation.md` (default: lower color-ID owns the bisector edge), and `CURRENT_SLICE_IR_SCHEMA_VERSION` bumps to `4.3.0`. | `rg -q 'pub bisector_edge_skip_mask: Vec<Vec<bool>>' crates/slicer-ir/src/slice_ir.rs && rg -q 'pub const CURRENT_SLICE_IR_SCHEMA_VERSION: SemVer = SemVer \{ major: 4, minor: 3, patch: 0' crates/slicer-ir/src/slice_ir.rs && cargo test -p slicer-core --test paint_segmentation_bisector_mask_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** a 4-color cube painted-region fixture where adjacent color cells share bisector edges, **when** each perimeter module traces the outer wall per-cell, **then** edges with `bisector_edge_skip_mask[i][j] == true` are skipped (not traced) and edges with `false` are traced exactly once; the union of all per-color outer wall fragments covers the external perimeter exactly (no gap, no double-trace within ±0.01 mm); and the `external_contour` consumption path is removed from both modules (no remaining call to `region.external_contour()`). | `cargo test -p slicer-runtime --test integration mmu_bisector_dedup_tdd -- --nocapture 2>&1 | tee target/test-output.log && ! rg -q '\.external_contour\(\)' modules/core-modules/classic-perimeters/src/lib.rs modules/core-modules/arachne-perimeters/src/lib.rs`

## Negative Test Cases

- **AC-N1. Given** a region with `detect_thin_wall = false`, **when** the same thin-protrusion fixture from AC-3 runs, **then** zero `WallLoop`s with `loop_type = LoopType::ThinWall` appear in `walls` (the detection cascade is gated on the config; default-off behavior preserved). | `cargo test -p slicer-runtime --test integration thin_wall_emission_tdd detect_disabled_case -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a region with **no** gap geometry (a clean square at `wall_count = 2` line_width 0.4 mm), **when** `run_perimeters` runs with `gap_infill_speed > 0`, **then** zero `WallLoop`s with `loop_type = LoopType::GapFill` appear; gap detection runs but produces no output and does not panic on empty `gaps`. | `cargo test -p slicer-runtime --test integration gap_fill_emission_tdd no_gaps_case -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N3. Given** a single-color region (no paint, no bisector neighbors), **when** the host populates `bisector_edge_skip_mask`, **then** every entry is `false` (no skipped edges) and both perimeter modules trace every outer-wall edge once (count of distinct outer-wall extrusion sequences = 1, matching the unpainted baseline). | `cargo test -p slicer-runtime --test integration mmu_bisector_dedup_tdd single_color_unaffected_case -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test integration outer_inner_width_and_spacing_tdd thin_wall_emission_tdd gap_fill_emission_tdd mmu_bisector_dedup_tdd && cargo test -p slicer-helpers --test wall_sequence_reorder_tdd && cargo test -p slicer-core --test paint_segmentation_bisector_mask_tdd`

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 5 (T-050..T-054c), Phase 6 (T-060..T-065), Inherited from P96 (T-P96-A0/B/C0/C1/C2). Range-read those phase sub-tables.
- `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` — wall_sequence ownership rationale (read full; ~40 lines).
- `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` — MMU fragmentation contract (read full; ~80 lines).
- `docs/02_ir_schemas.md` — `LoopType`, `ExtrusionRole`, `Point3WithWidth`, `SlicedRegion`, `SemVer` (delegate SUMMARY).
- `docs/13_slicer_helpers_crate.md` — helper-crate export conventions (read full; ≤250 lines).
- `docs/15_config_keys_reference.md` — config-key registration format (range-read §"Walls").
- `docs/DEVIATION_LOG.md` — `D-96-AC22-EXTERNAL-CONTOUR` entry to supersede.

## Doc Impact Statement (Required)

This packet modifies the following doc sections:

- `docs/specs/orca-mmu-perimeter-investigation.md` (NEW one-pager from T-P96-A0) — cites OrcaSlicer `MultiMaterialSegmentation.cpp` + `PerimeterGenerator.cpp` per-color paths with line numbers; states the bisector tie-break rule used by T-P96-C0 — `rg -q 'tie-break.*lower color-ID|tie-break.*matching OrcaSlicer' docs/specs/orca-mmu-perimeter-investigation.md`
- `docs/02_ir_schemas.md` §"LoopType" + §"ExtrusionRole" — document the new `GapFill` variants — `rg -q 'LoopType::GapFill' docs/02_ir_schemas.md && rg -q 'ExtrusionRole::GapFill' docs/02_ir_schemas.md`
- `docs/02_ir_schemas.md` §"SlicedRegion" — document `bisector_edge_skip_mask` and 4.3.0 schema bump — `rg -q 'bisector_edge_skip_mask' docs/02_ir_schemas.md && rg -q '4\.3\.0.*bisector_edge_skip_mask|4\.3\.0.*GapFill' docs/02_ir_schemas.md`
- `docs/15_config_keys_reference.md` — register `outer_wall_line_width`, `inner_wall_line_width`, `precise_outer_wall`, `wall_sequence`, `detect_thin_wall`, `gap_infill_speed`, `filter_out_gap_fill` — `rg -q 'outer_wall_line_width' docs/15_config_keys_reference.md && rg -q 'precise_outer_wall' docs/15_config_keys_reference.md && rg -q 'wall_sequence.*enum' docs/15_config_keys_reference.md && rg -q 'detect_thin_wall' docs/15_config_keys_reference.md && rg -q 'gap_infill_speed' docs/15_config_keys_reference.md`
- `docs/13_slicer_helpers_crate.md` §"Flow + perimeter_utils" — document new `flow` module + `wall_sequence_reorder` — `rg -q 'wall_sequence_reorder' docs/13_slicer_helpers_crate.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1501-1506,1644` — `ext_perimeter_spacing2` arithmetic and `precise_outer_wall` gating condition. Delegate a SUMMARY (≤ 150 words).
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1596-1609` — thin-wall detection cascade (`offset2_ex` + `opening_ex` + `medial_axis`). Delegate a SUMMARY.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1665-1670,1930-1958` — gap collection per-inset and gap-fill emission. Delegate a SUMMARY.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1801-1913` — `wall_sequence` reorder, including `InnerOuterInner` sandwich-mode inset-index gymnastics. Delegate a SUMMARY (≤ 200 words) of the reordering algorithm; **no code snippets**.
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `Flow::new_from_width_height` math (width → spacing conversion). Delegate a SUMMARY (≤ 100 words).
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` and `PerimeterGenerator.cpp` per-color branches — MMU per-color outer-wall fragmentation + bisector tie-break rule. Delegate a SUMMARY (≤ 200 words) of which rule OrcaSlicer uses (deterministic ordering, color ID-based, or other); **this is the deliverable of T-P96-A0**.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

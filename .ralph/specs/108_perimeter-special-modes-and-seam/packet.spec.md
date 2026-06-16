---
status: draft
packet: 108_perimeter-special-modes-and-seam
task_ids:
  - T-070
  - T-071
  - T-072
  - T-073
  - T-074b
  - T-074c
  - T-074d
  - T-077
  - T-080
  - T-081
  - T-082
  - T-083
  - T-P98-SEAM
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 108_perimeter-special-modes-and-seam

## Goal

Land the Phase 7 wall-count overrides (`extra_perimeters` config bonus, narrow-island `smaller_perimeter` handling, `LoopType::NonPlanarShell` emission for regions in surface groups) and the Phase 8 seam-candidate quality work (sharp-corner threshold replacing every-vertex emission, painted `seam_enforcer`/`seam_blocker` consumption in candidate scoring + seam-placer selection).

## Scope Boundaries

Touches both perimeter modules' `lib.rs` + manifests, `slicer-core::perimeter_utils` (sharp-corner threshold + paint-seam consumption helpers), `seam-placer/src/lib.rs` (consume painted bias/exclusion), `docs/15_config_keys_reference.md`, and `docs/DEVIATION_LOG.md` (D-98-SEAM-NO-CONSUMER supersession). T-077 is now a real consumer of `region.overhang_areas()` — the upstream data flow lights up via P106 (overhang PrePass foundation) + P107 (overhang view-accessor + consumer refactor); this packet wires the perimeter-side consumption to actually add extra perimeters in overhang regions.

## Prerequisites and Blockers

- Depends on:
  - **P102** (foundations) — shared utils crate, multi-segment `MaterialBoundary`.
  - **P104** (propagation + surface rules) — `SliceRegionView::surface_group()` accessor (T-074b/c/d consume), `overhang_areas()` accessor (T-077 consumes — now returns non-empty after P106+P107).
  - **P105** (spacing + fill + MMU) — outer/inner widths, wall_sequence, ThinWall/GapFill emission (T-074d skips them for non-planar regions).
  - **P106** (overhang PrePass foundation) — populates `OverhangRegion.xy_footprint` at MeshAnalysis; new `PrePass::OverhangAnnotation` stage produces `overhang_quartile_polygons`.
  - **P107** (overhang consumers + refactor) — confirms P104's `overhang_areas()` stub returns non-empty data.
- Unblocks:
  - **P109 (M1 verification + closure)** — T-103 will close the deviations this packet registers (D-98-SEAM-NO-CONSUMER supersession).
- Activation blockers: none. All preconditions are concrete predecessor packets, not external sibling roadmaps.

## Acceptance Criteria

- **AC-1. Given** a region with base `wall_count = 2` and config override `extra_perimeters = 2`, **when** `run_perimeters` runs, **then** `PerimeterRegion.walls` contains exactly 4 walls (`loop_number = wall_count + extra_perimeters - 1 = 3` zero-indexed, i.e. 4 walls). With `extra_perimeters = 0`, the count stays at 2. | `cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** a long narrow rectangular island (length 20 mm, width 0.6 mm) with `smaller_perimeter_threshold_mm = 0.8` and `smaller_perimeter_line_width = 0.3`, **when** `run_perimeters` runs, **then** the outer wall on that island uses `width = 0.3 mm` per vertex (not the default `outer_wall_line_width`), and a wider island in the same fixture uses the default width. | `cargo test -p slicer-runtime --test integration narrow_island_smaller_perimeter_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** a `SlicedRegion` whose `nonplanar_surface` is `Some(SurfaceGroupId(7))` and the corresponding `SurfaceGroup.shell_count == 3`, **when** `run_perimeters` runs, **then** `PerimeterRegion.walls` contains exactly 3 walls all with `loop_type = LoopType::NonPlanarShell` (NOT `Outer` or `Inner`), `infill_areas` is empty, and no `ThinWall` or `GapFill` loops are emitted regardless of `detect_thin_wall` or `gap_infill_speed` config. | `cargo test -p slicer-runtime --test integration nonplanar_shell_emission_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** a square contour with 4 corners (90°) and an additional 30 redundant collinear points along each edge, **when** seam-candidate generation runs with `seam_candidate_angle_threshold_deg = 30.0`, **then** `PerimeterRegion.seam_candidates` contains exactly 4 entries (one per corner) — NOT 124 (every vertex) and NOT 0; corner positions match the 4 corner XYs within ±0.01 mm. | `cargo test -p slicer-core --test sharp_corner_seam_threshold_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** a `boundary_paint` region carrying `PaintSemantic::SeamEnforcer` over a flat (non-corner) wall segment AND a sharper corner candidate outside the enforced region, **when** `seam-placer` selects the seam, **then** the resolved seam falls inside the `SeamEnforcer` region (the enforcer's bias outweighs the sharper-corner geometric score). Given a `SeamBlocker` region covering a wall corner, **then** that corner is **excluded** from `seam_candidates` entirely. | `cargo test -p slicer-runtime --test integration painted_seam_enforcer_blocker_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** an overhang-ramp fixture where `region.overhang_areas()` returns non-empty (P106's `xy_footprint` populated + P107's view-accessor lighting up P104's stub) and `extra_perimeters_on_overhangs = true`, **when** `run_perimeters` runs on a layer with overhang regions, **then** the wall_count inside `region.overhang_areas()` polygons is N+1 (one extra perimeter beyond the configured base `wall_count = N`), while wall_count outside those polygons stays at N. With `extra_perimeters_on_overhangs = false`, the wall count is N everywhere regardless of overhang membership. | `cargo test -p slicer-runtime --test integration extra_perimeters_on_overhangs_tdd -- --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a non-planar region (`nonplanar_surface.is_some()`) and `detect_thin_wall = true`, **when** `run_perimeters` runs, **then** zero `WallLoop`s with `loop_type = LoopType::ThinWall` appear — the non-planar branch short-circuits thin-wall detection. | `cargo test -p slicer-runtime --test integration nonplanar_shell_emission_tdd nonplanar_skips_thin_wall_case -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a fixture that passes the sharp-corner threshold but a `SeamBlocker` region covers the only sharp corner present, **when** seam-candidate generation runs, **then** `seam_candidates` is empty for that region (blocker excludes the corner; no fallback to next-sharpest because no other candidates exist), AND `seam-placer` returns `Err(SeamPlacerError::NoCandidates)` (graceful failure — not silent). | `cargo test -p slicer-runtime --test integration painted_seam_enforcer_blocker_tdd blocker_exhausts_candidates_case -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd narrow_island_smaller_perimeter_tdd nonplanar_shell_emission_tdd painted_seam_enforcer_blocker_tdd extra_perimeters_on_overhangs_tdd && cargo test -p slicer-core --test sharp_corner_seam_threshold_tdd`

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 7 (T-070..T-077, including superseded T-074/T-075 + non-planar T-074b/c/d), Phase 8 (T-080..T-083), Inherited from P98 (T-P98-SEAM). Range-read those sub-tables.
- `docs/02_ir_schemas.md` — `LoopType::NonPlanarShell`, `SurfaceGroup`, `PaintSemantic` (delegate SUMMARY).
- `docs/05_module_sdk.md` — `SliceRegionView::surface_group()` accessor + `seam-placer`'s `seam_candidates()` consumer (delegate SUMMARY).
- `docs/15_config_keys_reference.md` — config-key registration format.
- `docs/DEVIATION_LOG.md` — D-98-SEAM-NO-CONSUMER (to be superseded); format reference for new entries.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — register `extra_perimeters` (int, default 0), `smaller_perimeter_line_width` (float, default 0.25), `smaller_perimeter_threshold_mm` (float, default 0.8), `narrow_loop_length_threshold_mm` (float, default 10.0), `seam_candidate_angle_threshold_deg` (float, default 30.0), `extra_perimeters_on_overhangs` (bool, default false) — `rg -q 'extra_perimeters' docs/15_config_keys_reference.md && rg -q 'smaller_perimeter_line_width' docs/15_config_keys_reference.md && rg -q 'seam_candidate_angle_threshold_deg' docs/15_config_keys_reference.md && rg -q 'extra_perimeters_on_overhangs' docs/15_config_keys_reference.md`
- `docs/DEVIATION_LOG.md` — supersede `D-98-SEAM-NO-CONSUMER` with `D-<packet>-SEAM-CONSUMED` — `rg -q 'D-.*-SEAM-CONSUMED' docs/DEVIATION_LOG.md`
- `docs/05_module_sdk.md` §"Seam-candidate generation" — document the sharp-corner threshold + paint-seam consumption convention — `rg -q 'seam_candidate_angle_threshold_deg' docs/05_module_sdk.md && rg -q 'SeamEnforcer.*bias|SeamBlocker.*exclude' docs/05_module_sdk.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1569` — `surface.extra_perimeters` bonus arithmetic (`loop_number = wall_loops + surface.extra_perimeters - 1`). Delegate a FACT.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1611-1628` — narrow-island `smaller_ext_perimeter_flow` handling. Delegate a SUMMARY ≤ 150 words.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` non-planar shell branch (if present in source) OR `SkeletalTrapezoidation`'s shell_count consumer — Delegate a SUMMARY ≤ 100 words for the shell_count → wall_count override semantics.
- `OrcaSlicerDocumented/src/libslic3r/Feature/SeamPlacer/SeamPlacer.cpp` (or analogous) — sharp-corner candidate selection + painted seam consumption rules. Delegate a SUMMARY ≤ 200 words.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

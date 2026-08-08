---
status: superseded
packet: 208-wall-flag-path-clip
task_ids:
  - TASK-324
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 208-wall-flag-path-clip

## Status Note — superseded (deferred by user decision 2026-08-07)

This packet is **not live work.** It was deferred by user decision on 2026-08-07, and its queue row in `docs/specs/deviation-remediation-206-212-plan.md` reads `superseded`. The directory holds only a partial draft — this contract plus `requirements.md`; there is no `design.md`, no `implementation-plan.md` and no task map, and none are to be authored.

The divergence the packet describes is real but **inert in production**: `build_wall_flags` reads `segment_annotations[Material]` and `segment_annotations[FuzzySkin]`, and no production writer populates either key, so `WallBoundaryType::MaterialBoundary` never occurs on a production slice. The reprojection path this packet set out to replace is therefore unreachable outside tests.

Revisit only once a `Material`/`FuzzySkin` `segment_annotations` writer actually exists — that is the business of packets 206 and 207. See `DEV-126` in `docs/DEVIATION_LOG.md` for the corrected consequence.

## Goal

Replace `build_wall_flags`' nearest-original-vertex reprojection with canonical path-geometry clipping: deliver paint areas as `ExPolygon` sets on `SlicedRegion`, port `Algorithm::split_line` into `slicer-core`, and assign per-vertex `WallFeatureFlags` from clipped-run membership in both perimeter modules.

## Scope Boundaries

In: a new `slicer_core::line_split` port of canonical `Algorithm::split_line`; a new `SlicedRegion.paint_areas` IR field with its `paint_segmentation` writer, WIT marshal and SDK accessor; a rewritten `build_wall_flags` that clips instead of reprojecting; both perimeter-module call sites; and the rewrite of the two `slicer-core` tests that pin the reprojection technique. Out: fuzzy-skin *jitter* generation (`fuzzy-skin` module), seam paint (packet 206), per-region shell config (packet 207), the Arachne `variant_fuzzy` wiring gap noted at the arachne call site, and any change to hole-ring flag handling.

## Prerequisites and Blockers

- Depends on: none (row #3 of `docs/specs/deviation-remediation-206-212-plan.md` is independent of every other row).
- Unblocks: any future consumer of area-shaped paint (`fuzzy-skin` region grouping, `top-surface-ironing` skip-ironing regions).
- Activation blockers: the `[BLOCK]` in `design.md` §Open Questions — the packet's honest aggregate context cost is above `M` and the queue owner must approve either the split point named there or an extended-band run. Status stays `draft` until answered.

## Acceptance Criteria

- **AC-1. Given** a 4-point square path `[(0,0),(100000,0),(100000,100000),(0,100000)]` and a clip set containing one `ExPolygon` covering `x ∈ [0, 50000]`, **when** `slicer_core::line_split::split_line(path, clip, true)` is called, **then** the returned `Vec<SplitJunction>` contains at least one junction with `clipped == true` and at least one with `clipped == false`, every junction's `src_idx` is either a valid index into `path` or a negative encoding `-(1 + first_source_index)`, and consecutive junction points are non-coincident. | `cargo test -p slicer-core --features host-algos --test line_split_tdd -- split_line_square_half_clip --nocapture 2>&1 | tail -5`
- **AC-2. Given** a path lying entirely inside the clip set, **when** `split_line(path, clip, true)` is called, **then** every returned junction has `clipped == true` and `is_src()` is true for all of them (no intersection-born points are inserted). | `cargo test -p slicer-core --features host-algos --test line_split_tdd -- split_line_fully_inside_all_clipped --nocapture 2>&1 | tail -5`
- **AC-3. Given** `execute_paint_segmentation` running on a mesh whose modifier volume paints `PaintSemantic::FuzzySkin` over part of a layer, **when** the slice completes, **then** the emitted `SlicedRegion.paint_areas` contains an entry keyed `PaintSemantic::FuzzySkin` whose value list holds `(PaintValue::Flag(true), areas)` with `areas` non-empty, and the union of `areas` is contained in the region's `polygons`. | `cargo test -p slicer-core --features host-algos --test paint_areas_writer_tdd -- fuzzy_modifier_emits_paint_areas --nocapture 2>&1 | tail -5`
- **AC-4. Given** the concave notched-rectangle fixture from `crates/slicer-core/tests/inner_wall_concave_reprojection_tdd.rs` expressed as two paint areas (tool 1 over the un-notched body, tool 2 over the notch neighbourhood) and the same 9-vertex inset ring, **when** `build_wall_flags` is called with those `paint_areas`, **then** `flags[3].tool_index == Some(1)` and `flags[4].tool_index == Some(2)`, and the function inserts no extra flag slots (`flags.len() == 9`). | `cargo test -p slicer-core --features host-algos --test inner_wall_paint_clip_tdd -- concave_notch_ring_vertex3_is_tool1 --nocapture 2>&1 | tail -5`
- **AC-5. Given** `paint_areas` absent (empty map) and `segment_annotations` carrying the two-tool 4-point annotation from `crates/slicer-core/tests/inner_wall_material_boundary_tdd.rs`, **when** `build_wall_flags(4, 0, …, is_outer = false, …)` is called, **then** it returns `WallBoundaryType::MaterialBoundary` with exactly 2 segments, `segments[0].near_tool == Some(1)` and `segments[0].far_tool == Some(2)` — the index-based fallback is preserved bit-for-bit. | `cargo test -p slicer-core --features host-algos --test inner_wall_material_boundary_tdd 2>&1 | tail -5`
- **AC-6. Given** the workspace tree after this packet, **when** `nearest_original_vertex` is searched for, **then** it has zero occurrences under `crates/` and `modules/`. | `rg -c 'nearest_original_vertex' crates/ modules/ ; test $? -eq 1`
- **AC-7. Given** `classic-perimeters` generating an inner wall for a painted region, **when** the module runs, **then** its `build_wall_flags` call passes the region's paint areas (resolved through `slicer_sdk`) rather than `inset_ring_points`/`original_polygons`. | `rg -q 'paint_areas' modules/core-modules/classic-perimeters/src/lib.rs && rg -q 'paint_areas' modules/core-modules/arachne-perimeters/src/lib.rs && echo PASS`
- **AC-8. Given** the WIT change, **when** `slice-region-view` is inspected, **then** it declares a `paint-areas` accessor and a `paint-area-entry` record, and `cargo check --workspace --all-targets` succeeds. | `rg -q 'paint-area-entry' crates/slicer-schema/wit/deps/ir-types.wit && rg -q 'paint-areas: func' crates/slicer-schema/wit/deps/ir-types.wit && echo PASS`
- **AC-9. Given** `docs/DEVIATION_LOG.md`, **when** the packet closes, **then** the DEV-126 row's status column records the clip-based replacement and names this packet. | `rg -q 'DEV-126.*208-wall-flag-path-clip' docs/DEVIATION_LOG.md && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a wall path that lies entirely outside every paint area, **when** `build_wall_flags` is called with a non-empty `paint_areas`, **then** every returned flag has `tool_index == None` and `fuzzy_skin == false`, and the boundary type is `WallBoundaryType::Interior` for `is_outer = false`. | `cargo test -p slicer-core --features host-algos --test inner_wall_paint_clip_tdd -- ring_outside_all_paint_areas_unflagged --nocapture 2>&1 | tail -5`
- **AC-N2. Given** a degenerate clip set (an `ExPolygon` whose contour has fewer than 3 points), **when** `split_line` is called, **then** it returns an empty `Vec` rather than panicking or dividing by zero. | `cargo test -p slicer-core --features host-algos --test line_split_tdd -- split_line_degenerate_clip_returns_empty --nocapture 2>&1 | tail -5`
- **AC-N3. Given** `variant_fuzzy = true` and an empty `paint_areas`, **when** `build_wall_flags` is called, **then** every flag still has `fuzzy_skin == true` — the D14 painted-variant seed is not regressed by the clip rewrite. | `cargo test -p slicer-core --features host-algos --test inner_wall_paint_clip_tdd -- variant_fuzzy_seeds_all_vertices --nocapture 2>&1 | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --features host-algos --test inner_wall_paint_clip_tdd`

## Authoritative Docs

- `docs/08_coordinate_system.md` — direct range read of the unit-conversion checklist; the clip runs in integer units.
- `docs/02_ir_schemas.md` — delegated SUMMARY of the `SlicedRegion` section; this packet adds a field to it.
- `docs/DEVIATION_LOG.md` — direct read of the DEV-126 row only; the file is long, never load it whole.
- `CLAUDE.md` §"WIT/Type Changes Checklist" and §"Guest WASM Staleness" — direct read; both fire on this change surface.

## Doc Impact Statement (Required)

- `docs/02_ir_schemas.md` section "SlicedRegion" — document `paint_areas` — `rg -q 'paint_areas' docs/02_ir_schemas.md`
- `docs/DEVIATION_LOG.md` DEV-126 row — record closure and name this packet — `rg -q 'DEV-126.*208-wall-flag-path-clip' docs/DEVIATION_LOG.md`
- `docs/07_implementation_status.md` — add the `TASK-324` entry — `rg -q 'TASK-324' docs/07_implementation_status.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Algorithm/LineSplit.hpp` and `LineSplit.cpp` — the `SplitLineJunction` struct (`p`, `clipped`, `src_idx`) and the `split_line` / `do_split_line` contract, including the `closed` flag's duplicate-first-point wrapping. Borrowed shape; the `ClipperZUtils::ZPath` src-index carrier is deliberately NOT borrowed (see `design.md`).
- `OrcaSlicerDocumented/src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp` — `apply_fuzzy_skin` (Polygon and `Arachne::ExtrusionLine` overloads), `group_region_by_fuzzify`, `should_fuzzify`. Borrowed: the region-as-`ExPolygons` grouping, the rotate-to-a-non-clipped-junction step, and the run-walking loop that brackets a clipped run with un-flagged anchor points.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

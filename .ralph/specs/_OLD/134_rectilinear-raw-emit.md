---
status: implemented
packet: 134_rectilinear-raw-emit
task_ids: [TASK-259]
---

# 134_rectilinear-raw-emit

## Goal

Rewrite `modules/core-modules/rectilinear-infill/src/lib.rs` to OrcaSlicer scan-line correctness under raw emit: `infill_direction` angle resolution, float-rotation, per-ExPolygon scan conversion with the half-open vertex test, `adjust_solid_spacing` for solid roles, `pattern_shift` for layer interleave — emitting raw 2-point segments only (linking, overlap, and filtering are the linker's, ADR-0025).

## Problem Statement

The module's geometry was wrong in four places: global edge merge across expolygons (cross-polygon pairing at `lib.rs:231-237`), missing vertex-test discipline (double-count at scan lines through vertices), missing solid-spacing adjustment, and missing bridge-angle priority. Under Architecture A the module must emit raw 2-point segments with correct geometry; everything post-emit is the linker's job.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm). Rotation rounding ≤ 50 nm is below the 100 nm floor (`docs/08_coordinate_system.md`).
- Raw emit boundary (ADR-0025): NO linking, overlap, chaining, or filtering code is added; the deleted concepts stay deleted (`fill_expolygon_multi`, `collect_edges` — zero-hit grep).
- Four-role emission structure, `solid_fill_role` mapping, `should_emit` gating, and the manifest stay untouched.
- Per-region density read through the packet-131 SDK region accessor inside the region loop.

## Data and Contract Notes

- `infill_direction` ported to the module (angle priority: bridge_angle > per-layer rotation > static base angle, FillBase.cpp:352-391; π/2 offset because fill lines run perpendicular to the angle; reference point). `adjust_solid_spacing` (FillBase.cpp:326-340) applied while generating raw segments for solid roles. `pattern_shift` (FillRectilinear.cpp:3023-3024) applied to the scan-line start x (sign-alternates per layer via `infill_shift_step` config).
- Per-ExPolygon scan conversion via `scan_expolygon` ported from `slice_region_by_vertical_lines` (half-open edge test: edge included at min_y, excluded at max_y; sort+pair crossings).
- Segment count formula: `floor(bb_height / line_spacing) + 1` (line_spacing = spacing/density), exactly 2 points per segment, endpoints on the polygon boundary ±2 units, no shared endpoints (no linking).
- Stale-test reconciliation: each rewritten test carries a header comment naming the OLD bug it encoded (cross-polygon pairing, missing vertex test, missing solid-spacing) — no silent re-pinning.

## Locked Assumptions and Invariants

- `bridge_paths_use_bridge_orientation_not_sparse_alternation` and `bridge_areas_emit_bridge_infill_at_oriented_angle` stay green (AC-6 regression pin).
- Top/bottom/bridge role-emission tests (`top_bottom_fill_tdd.rs`, 7 tests) stay green through the rewrite.
- Hole-crossing scan lines yield exactly 2 segments per side with no point strictly inside the hole (AC-2).
- AC-N1: the half-open vertex test yields the exact analytic intersection count at a vertex-crossing x.
- Wave-core byte-identical is a rectilinear follow-up (per-region density follow-up 2026-07-19 wired `resolve_float` helper).

## Risks and Tradeoffs

- Until packet 133 lands, user-visible print is degraded raw segments (ADR-0025 degraded-not-failed trade-off) — this packet ships regardless; raw emit is source-of-truth and the linker is a pure-function pass over it.

## Implementation Deviations (recorded at close)

None. Doc Impact: `none` — module-internal algorithm rewrite; the architectural raw-emit behavior is already documented by ADR-0025 and the infill-parity spec.

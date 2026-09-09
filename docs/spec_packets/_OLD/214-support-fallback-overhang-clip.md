---
status: superseded
packet: 214-support-fallback-overhang-clip
task_ids:
  - TASK-323
superseded_by: 220-support-analysis-family-contracts + 222-traditional-support-family (fallback fillers removed; traditional family planner replaces clipping `region.overhang_areas()` inside a per-layer filler)
---

# 214-support-fallback-overhang-clip

## Goal

Restrict fallback support generation to each region's pre-filtered `overhang_areas()` and derive `needs_support` from whether those overhang areas are non-empty, while preserving full-polygon filling for enforced regions.

## Problem Statement

Fallback support modules currently iterate every `region.polygons()` expolygon, so they fill model interiors. The host already computes region-clipped `overhang_areas`, but `needs_support` is hardcoded true. Reusing those existing values is one coherent fallback eligibility and fill-boundary fix.

## Architecture Constraints

- `overhang_areas()` is already region-clipped and flattened from `overhang_quartile_polygons`; do not recompute or broaden it.
- Paint precedence remains Blocked, Enforced, then DefaultEligible. Only the DefaultEligible fill input changes.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR/manifest contracts: no changes; `SliceRegionData.needs_support` and `overhang_areas` retain their existing shapes.
- WIT boundary: no identifier or schema change; only the host-populated boolean changes.
- Determinism/scheduler constraints: preserve module selection and paint policy ordering; polygon iteration order comes from existing vectors.

## Locked Assumptions and Invariants

- Enforced fills full `region.polygons()` even when `overhang_areas()` is empty.
- DefaultEligible with empty `overhang_areas()` emits nothing because host `needs_support` is false and module selection also uses the empty clip.
- Blocked emits nothing regardless of either polygon set.

## Risks and Tradeoffs

- Existing fixtures intentionally using default `needs_support: true` may need explicit overhang areas or an enforced policy; do not weaken the production contract to preserve stale fixtures.
- This fix clips fallback geometry but does not create downward propagation; planner packet TASK-322 owns plate-reaching tree geometry.

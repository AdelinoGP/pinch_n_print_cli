---
status: implemented
packet: 102_perimeter-modules-foundations
task_ids:
  - T-010
  - T-011
  - T-012
  - T-013
  - T-014
  - T-015
  - T-016
  - T-017
  - T-018
  - T-019
---

# 102_perimeter-modules-foundations

## Goal

Establish shared infrastructure for both perimeter modules: extract duplicated paint/seam/conversion helpers into `slicer-core::perimeter_utils`, widen `WallBoundaryType::MaterialBoundary` to carry per-segment transition lists, plumb per-layer config overrides through `run_perimeters`, and propagate `PerimeterOutputBuilder` `Result`s via `?`.

## Problem Statement

The two perimeter modules (`classic-perimeters` and `arachne-perimeters`) share ≈170 LOC of duplicated paint-propagation, seam-candidate, and point-conversion helpers. That duplication is a maintenance hazard — every future per-vertex flag (T-020 bridge, T-021/T-022 inner-wall material boundary, T-074b/c/d non-planar emission) would otherwise have to land twice with risk of drift. The audit also surfaced four discrete defects that the modules currently carry: `WallBoundaryType::MaterialBoundary { adjacent_tool: u32 }` records only the first transition on a multi-tool polygon and silently drops the rest; `let _ = output.…` swallows `PerimeterOutputBuilder` `Result`s so capacity / contract violations become invisible; the `_config` and `_layer_index` parameters in `run_perimeters` are unread, making the host's `LayerOverrides` mechanism inoperative for these modules; and manifest-vs-code defaults disagree on `wall_count` (3 vs 2), `outer_wall_speed` (30.0 vs 50.0), and `inner_wall_speed` (45.0 vs 50.0) — when manifest validation is bypassed the silent divergence is a latent footgun.

This packet closes the four defects together with the duplication extraction because they all share the same file surface; bundling them avoids two rounds of cherry-picking through the same `lib.rs` files. None of the four defects can be fixed in isolation without first paying the merge cost on the duplicated helpers.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- Schema-version contract: `CURRENT_SLICE_IR_SCHEMA_VERSION` bumps `4.1.0 → 4.2.0` (minor, additive). The migration adapter MUST deserialize the pre-bump `MaterialBoundary { adjacent_tool: u32 }` shape into a single-element `Vec<MaterialBoundarySegment>` (with `near_tool: None`, `far_tool: Some(adjacent_tool)`, and `point_range: 0..1`) so committed test fixtures stay parseable.
- WIT type identity: the `wall-boundary-type` variant must match across `crates/slicer-schema/wit/deps/ir-types.wit` (canonical), the host `bindgen!` consumers, and the guest macro inputs (`#[slicer_module]` via `slicer-macros`). Per CLAUDE.md WIT/Type Changes Checklist, `cargo build --tests` must pass before declaring Step 2 done.
- Both perimeter modules' `_paint: &PaintRegionLayerView` parameter remains semantically passive in this packet (per T-019). The decision is recorded as "the consumer for paint regions outside `segment_annotations` is Phase 2 work"; the doc-comment must spell this out so the next reader does not believe the unused parameter is an accident.

## Data and Contract Notes

- IR or manifest contracts touched: `WallBoundaryType` variant payload widens. Backward-compatible via `#[serde(default)]` migration adapter. Schema version bumps additively. Test fixtures committed before this packet stay parseable; new fixtures must use the new shape.
- WIT boundary considerations: `wall-boundary-type` variant payload changes from a single `u32` to `list<material-boundary-segment>`. The `material-boundary-segment` record is new. Both must be declared in `crates/slicer-schema/wit/deps/ir-types.wit` (the canonical single source — there is no inline copy per CLAUDE.md).
- Determinism or scheduler constraints: none. The shared utils' helpers are pure functions; the dispatch order through `run_perimeters` is unchanged.
- `PerimeterOutputBuilder` failure-mode contract is newly documented: callers MUST propagate `?`. Capacity / contract-violation errors must surface as `ModuleError` rather than be silently discarded. AC-N1 enforces this with a mock-builder fixture.

## Locked Assumptions and Invariants

- The two perimeter modules remain sibling-independent — neither imports the other; both consume the shared utils from `slicer-core`.
- `perimeter_utils.rs` placed in `slicer-core` per docs/13 §Out of Scope (per-layer geometry operations belong in slicer-core, not slicer-helpers). Part of roadmap-wide correction `D-ROADMAP-CRATE-PLACEMENT` matching the P103 pattern.
- The shared utils' API is **pure** (no I/O, no logging, no state). This invariant is preserved so the helpers can be called from both guest WASM contexts without host-services dependency.
- `WallBoundaryType::MaterialBoundary` semantics: every boundary segment names exactly one transition between two tools (`near_tool` → `far_tool`); polygons with N transitions emit N segments in clockwise order matching the polygon's contour winding.
- `BASE_SPEED = 50.0` (mm/s) remains the outer-wall normalisation reference. Bumped by mutual agreement only — neither manifest defaults nor code fallbacks may change this in isolation.
- Per-layer config reads in `run_perimeters` MUST use `_config.get*` directly each call; caching the `on_print_start` values for re-use across layers is forbidden because it defeats the layer-override mechanism.

## Risks and Tradeoffs

- WIT-type-identity break: editing `ir-types.wit` without rebuilding guest WASM produces silent test failures that look unrelated. Mitigation: explicit `cargo xtask build-guests --check` gate in Step 2's exit condition.
- Schema-bump test-fixture regression: existing committed `SliceIR` JSON fixtures with the old `MaterialBoundary { adjacent_tool: u32 }` shape might not deserialize without the migration adapter. Mitigation: include the migration adapter in Step 2, not Step 5; add a parse-old-shape test in `material_boundary_widening_tdd`.
- Helper-extraction sequencing: extracting helpers and migrating both modules in one step is too large (>3 files / step). Mitigated by Step 1 doing only the `slicer-core::perimeter_utils.rs` creation + `classic-perimeters` migration, leaving `arachne-perimeters` migration as Step 1b (Step 1's second half — see implementation plan).
- Manifest reconcile direction: the roadmap defaults to "manifest is source of truth". If the maintainers prefer the code values, this is a 1-line edit to the manifest instead of the code. Documented as `[FWD]` in §Open Questions.

---
status: implemented
packet: 104_perimeter-propagation-and-surface-rules
task_ids:
  - T-020
  - T-021
  - T-022
  - T-023
  - T-024
  - T-025
  - T-030
  - T-031
  - T-032
  - T-033
---

# 104_perimeter-propagation-and-surface-rules

## Goal

Make both perimeter modules read the per-region data already exposed by upstream PrePass IRs — per-vertex `is_bridge` from `region.bridge_areas()`, multi-segment `MaterialBoundary` on **inner** walls (not just outer), and `surface_group()` lookup for the future non-planar shell_count — and honour two surface-driven wall-count overrides (`only_one_wall_top` and `only_one_wall_first_layer`) that the roadmap's Phase 3 introduces.

## Problem Statement

PrePass already exposes per-region bridge polygons (`SlicedRegion.bridge_areas`, populated by `MeshAnalysis` per packet 36-rev1), top/bottom shell indices (`top_shell_index`, `bottom_shell_index`), non-planar surface IDs (`nonplanar_surface`), and full `SurfaceClassificationIR.OverhangRegion` data. The perimeter modules **read none of this beyond polygon outlines** — `is_bridge` is hardcoded false on every emitted vertex; inner walls carry hardcoded `WallBoundaryType::Interior` regardless of multi-tool paint; `top_shell_index == Some(0)` does not reduce wall count even when the user sets `only_one_wall_top`; first-layer wall count is the same as mid-print wall count even when the user sets `only_one_wall_first_layer`. These four defaults silently override real upstream data, producing wall geometry that disagrees with the user's intent and with OrcaSlicer parity.

This packet wires that data through, end to end: extends `SliceRegionView` with the missing accessors (`overhang_areas`, `surface_group`), renames and extends the shared helper in packet 102 from `build_outer_wall_flags` → `build_wall_flags` (adding `is_outer: bool`) to compute per-vertex flags for both outer and inner walls, and adds the two top/first-layer wall-count overrides. `overhang_quartile` per-vertex propagation (T-024) is the one exception: the algorithm needs cross-layer mesh-cross-section data the sibling roadmap (`overhang-pipeline-restructuring`) is preparing. Until that lands, this packet documents the deferral as a registered deviation rather than emit incorrect data or leave the field dead in IR.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- View-accessor convention: `overhang_areas()` and `surface_group()` follow the existing `bridge_areas()` / `has_nonplanar()` pattern — pre-filtered per-region at view construction; the guest receives only data relevant to the current region. No raw `SurfaceClassificationIR` access from guest space. Note: the existing non-planar accessor is `has_nonplanar() -> bool`, NOT `nonplanar_surface()`.
- `T-024` deferral invariant: `Point3WithWidth.overhang_quartile` MUST be set to `None` in every emit path (NOT left at field default, NOT inherited from caller). The doc-comment cites the sibling roadmap.
- Per-layer config rule (carries over from packet 102, T-015): `only_one_wall_top` and `only_one_wall_first_layer` MUST be read from `_config.get_bool` per `run_perimeters` invocation, not cached at `on_print_start`. Per-layer overrides take effect.
- `only_one_wall_top` parity scope: fires for `top_shell_index() == Some(0)` (blanket top-shell gate) AND for `Some(N>0)` sub-top shells via the `split_top_surfaces` carve against `top_solid_fill` (implemented this session); `None` is a no-op.

## Data and Contract Notes

- IR or manifest contracts touched: `SliceRegionView` gains two read-only accessors. WIT side gains a new `surface-group` record definition, a `surface-group-id` type alias, and two `func()` declarations on `slice-region-view`. `SliceRegionData` (host-side mirror) gains two fields. No IR-side struct shape changes — `Point3WithWidth` already has `flow_factor: f32` and `overhang_quartile: Option<u8>` (confirmed at `crates/slicer-ir/src/slice_ir.rs:1503`). `WallFeatureFlags` has `tool_index: Option<u32>`, `fuzzy_skin: bool`, `is_bridge: bool`, `is_thin_wall: bool`, `skip_ironing: bool`, `custom: HashMap<String, PaintValue>` (confirmed at `crates/slicer-ir/src/slice_ir.rs:1479`). `Point3WithWidth` does NOT have `is_bridge` or `tool_index` fields.
- WIT boundary considerations: per CLAUDE.md WIT/Type Changes Checklist, `cargo build --tests` must pass after the WIT edit before Step 1 closes. The WIT `surface-group` record is NEW — it does not exist in `ir-types.wit` yet (only `surface-group-proposal` in `world-prepass.wit`, which is a different type).
- Determinism or scheduler constraints: none. The per-vertex propagation is deterministic (point-in-polygon is a pure function over its inputs); the two wall-count gates are deterministic conditionals.
- `T-024` deferral contract: every emit path that constructs a `Point3WithWidth` MUST set `overhang_quartile: None`. The doc-comment cites `docs/specs/overhang-pipeline-restructuring.md` O-T031 as the future producer. When the sibling roadmap lands, T-024's full implementation is a small follow-up packet that flips this `None` to a point-in-quartile-polygon test.

## Locked Assumptions and Invariants

- `is_bridge` semantics: a wall vertex is `is_bridge: true` if and only if its XY point lies inside one of `region.bridge_areas()`. Edge ambiguity (vertex exactly on the boundary) defaults to `false` (strict-inside test).
- Inner-wall `WallBoundaryType` is computed by the same `build_wall_flags` logic as outer walls (with `is_outer=false`). There is no shortcut path. If inner-wall paint is empty, the result is `WallBoundaryType::Interior` (no material boundary); if paint exists with no transitions, `ExteriorSurface`; if transitions exist, `MaterialBoundary { segments: vec![...] }`.
- `Point3WithWidth.overhang_quartile = None` is invariant until the sibling roadmap lands. The deviation registration documents this.
- `only_one_wall_top` triggers when `region.top_shell_index() == Some(0)` (topmost solid layer). Sub-top shells (`Some(1)`, `Some(2)`, …) also trigger via the `split_top_surfaces` carve (top_solid_fill-scoped), implemented this session; no deviation is registered for sub-top reduction.
- `only_one_wall_first_layer` triggers **only** when `_layer_index == 0`. Layer 1 onwards is unaffected.
- `perimeter_utils` consumed from `slicer-core` per docs/13 §Out of Scope. Part of roadmap-wide correction `D-ROADMAP-CRATE-PLACEMENT` (P102, P103, P105, P108, P110, P111, P112 also renamed).
- `overhang_areas()` forward dependency: the IR field `OverhangRegion.xy_footprint: Vec<ExPolygon>` is **net-new — added by P106/O-T010** and absent from the current tree (`OverhangRegion`, `slice_ir.rs:586`, has no footprint field today; `slice_ir.rs:581` is `BridgeRegion.xy_footprint`). Until P106 ships both the field and its population, the host populator returns `Vec::new()` and does NOT reference the field. This is an honest forward-dep, documented in the closure log; the empty return is pinned by AC-3-EMPTY as a regression bed, not treated as a defect.

## Risks and Tradeoffs

- Host-populator `overhang_areas` returns `Vec::new()` until P106 (`106_overhang-pipeline-prepass-foundation`, status: draft) lands. Because `OverhangRegion.xy_footprint` is net-new (added by P106), the populator in THIS packet must not reference it — only the accessor signature + WIT func + empty-stub populator land now. When P106 adds and populates the field, the implementer wires the `xy_footprint`-intersection into the populator body (a small follow-up); the accessor signature does not change. Document in the closure log.
- Inner-wall paint extraction depends on the inner polygon's contour having `segment_annotations` keyed by the inner contour's vertex indices. The current modules build inner walls via iterative offset, and the offset operation does **not** carry paint values forward — segment_annotations are on the original SlicedRegion's polygons, not on the inset polygons. Mitigation: the inner-wall flag computation in this packet uses the **original** region's `segment_annotations`, sampled by nearest-vertex projection from the inner-polygon vertices back to the original-polygon vertices. Documented in `perimeter_utils.rs` doc-comment with a `TODO` for a more precise inner-wall paint sampler in Phase 5 work.
- The `only_one_wall_top`/`only_one_wall_first_layer` gates change wall geometry. Existing single-color test fixtures may have been calibrated against the pre-packet wall count. AC-N2 catches the case where the flag is supposed to be a no-op; the integration-tests-touching files MUST be re-baselined per fixture if needed. Document re-baselined SHAs in the closure log.

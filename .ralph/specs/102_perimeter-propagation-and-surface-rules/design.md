# Design: 102_perimeter-propagation-and-surface-rules

## Controlling Code Paths

- Primary code path: `SliceRegionView` (SDK) → `build_wall_flags` (shared utils, extended) → per-vertex `WallFeatureFlags` writes inside `run_perimeters` (both modules). Two new config gates (`only_one_wall_top`, `only_one_wall_first_layer`) read in `run_perimeters` to override the effective `wall_count`.
- Neighboring tests / fixtures: 5 new TDD files under `crates/slicer-runtime/tests/contract/` and `crates/slicer-helpers/tests/`. Existing `boundary_paint_tdd.rs` regression coverage in both modules must stay green.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- View-accessor convention: `overhang_areas()` and `surface_group()` follow the existing `bridge_areas()` / `nonplanar_surface()` pattern — pre-filtered per-region at view construction; the guest receives only data relevant to the current region. No raw `SurfaceClassificationIR` access from guest space.
- `T-024` deferral invariant: `Point3WithWidth.overhang_quartile` MUST be set to `None` in every emit path (NOT left at field default, NOT inherited from caller). The doc-comment cites the sibling roadmap.
- Per-layer config rule (carries over from packet 100): `only_one_wall_top` and `only_one_wall_first_layer` MUST be read from `_config.get_bool` per `run_perimeters` invocation, not cached at `on_print_start`. Per-layer overrides take effect.

## Code Change Surface

- Selected approach: add the two view accessors with **host-side pre-filtering** — the host populator intersects `OverhangRegion.xy_footprint` (currently empty until sibling roadmap O-T010 lands) with the region's polygon, and resolves `nonplanar_surface: Option<SurfaceGroupId>` to `Option<&SurfaceGroup>` by lookup in `SurfaceClassificationIR.per_object[…].surface_groups`. Both accessors return `&[ExPolygon]` / `Option<&SurfaceGroup>` view references — no Vec cloning. `build_wall_flags` in the shared utils gains an `is_outer: bool` parameter; the existing outer-wall logic moves under `if is_outer` and a new inner-wall code path runs the same Material / FuzzySkin extraction logic against the inner polygon (which has its own contour vertices and its own paint values per packet 100's IR widening). Per-vertex `is_bridge` derivation runs once per wall vertex via a point-in-polygon helper against `region.bridge_areas()`. `only_one_wall_top` and `only_one_wall_first_layer` are checked at the head of `run_perimeters`'s wall-emission loop; when either fires, `wall_count` is locally clamped to 1 before the loop iterates.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-sdk/src/views.rs` — add `pub fn overhang_areas`, `pub fn surface_group`; add the corresponding fields on `SliceRegionView` struct.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — `overhang-areas` and `surface-group` accessors on `slice-region-view`.
  - `crates/slicer-wasm-host/src/host.rs` — `SliceRegionData` field additions; populator fills both fields at view-construction.
  - `crates/slicer-helpers/src/perimeter_utils.rs` — extend `build_wall_flags` with `is_outer` parameter and inner-wall code path; add `point_in_any_polygon(&Point2, &[ExPolygon]) -> bool` helper.
  - `modules/core-modules/classic-perimeters/src/lib.rs` — call `build_wall_flags(.., is_outer=false)` for inner walls; read `region.bridge_areas()` for `is_bridge`; read `_config.get_bool("only_one_wall_top")` and `_config.get_bool("only_one_wall_first_layer")`; explicitly set `overhang_quartile = None` with deferred-roadmap doc-comment.
  - `modules/core-modules/arachne-perimeters/src/lib.rs` — same as classic.
  - `modules/core-modules/{classic,arachne}-perimeters/*.toml` — register the two new config keys.
  - 5 new TDD files.
  - 3 docs per Doc Impact Statement.
- Rejected alternatives that were considered and why they were not chosen:
  - Separate `SurfaceClassificationView` parameter to `run_perimeters` (option (b) of D-4): rejected per the grilling D-4 closure — view churn for one consumer.
  - Compute `overhang_areas` on-demand inside the perimeter module: rejected — pushes mesh-cross-section work into Tier 2 parallel layers, which is forbidden by Tier 1/Tier 2 separation. Host pre-filters at view construction.
  - Cache `only_one_wall_top` / `only_one_wall_first_layer` at `on_print_start`: rejected — defeats per-layer config override (T-015 invariant).

## Files in Scope (read + edit)

Primary edit surface exceeds 3 files; the packet bundles 10 roadmap tasks per the user's directive. The **three highest-LOC-delta** files are listed first:

- `crates/slicer-helpers/src/perimeter_utils.rs` — role: `build_wall_flags` extension + `point_in_any_polygon` helper; expected change: ~80 LOC.
- `modules/core-modules/classic-perimeters/src/lib.rs` — role: consume new view accessors, read bridge_areas per-vertex, gate wall_count on the two new flags; expected change: ~60 LOC.
- `modules/core-modules/arachne-perimeters/src/lib.rs` — role: mirror of classic; expected change: ~60 LOC.
- `crates/slicer-sdk/src/views.rs` — role: two new accessors + struct fields; expected change: ~30 LOC.
- `crates/slicer-wasm-host/src/host.rs` — role: populator fills new fields; expected change: ~20 LOC.
- `crates/slicer-schema/wit/deps/ir-types.wit` — role: WIT mirror; expected change: ~5 LOC.
- `modules/core-modules/{classic,arachne}-perimeters/*.toml` — role: register two config keys; expected change: ~10 LOC each.
- 5 new TDD files.
- 3 doc files per Doc Impact Statement.

## Read-Only Context

- `docs/specs/overhang-pipeline-restructuring.md` — read full (~150 lines) — purpose: confirm AC-3 accessor signature matches sibling-roadmap O-T030 plan + understand the T-024 deferral context.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — range-read §"Phase 2" and §"Phase 3" only — purpose: scope confirmation per task.
- `docs/02_ir_schemas.md` — delegate SUMMARY for `SurfaceClassificationIR`, `BridgeRegion`, `OverhangRegion`, `SurfaceGroup` — purpose: confirm field shapes the host populator reads.
- `docs/05_module_sdk.md` — delegate SUMMARY for `SliceRegionView` accessor + WIT convention — purpose: align new accessor style.
- `docs/15_config_keys_reference.md` — range-read §"Walls" — purpose: match existing config-key format.
- `modules/core-modules/classic-perimeters/tests/boundary_paint_tdd.rs` — read — purpose: confirm regression coverage; do not edit.
- `modules/core-modules/arachne-perimeters/tests/boundary_paint_tdd.rs` — read — same as classic.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate parity checks; never load.
- `target/`, `Cargo.lock`, generated bindgen output — never load.
- Vendored deps — never load.
- `crates/slicer-core/src/algos/mesh_analysis.rs` — out of scope. The sibling roadmap (overhang-pipeline-restructuring O-T010) edits this file to add `OverhangRegion.xy_footprint`; this packet's `overhang_areas()` accessor consumes whatever exists (currently empty Vec until O-T010 lands).
- `crates/slicer-core/src/algos/prepass_slice.rs` — out of scope.
- `crates/slicer-runtime/src/region_partition.rs` — out of scope.
- All `slicer-helpers` files except `perimeter_utils.rs` — out of scope.
- All modules under `modules/core-modules/` except the two perimeter modules — out of scope.

## Expected Sub-Agent Dispatches

- "Summarize `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1574-1577,1715` for `only_one_wall_top` and `only_one_wall_first_layer` gating; return SUMMARY ≤ 100 words" — purpose: Step 4 gate-logic confirmation.
- "Summarize `docs/02_ir_schemas.md` for the `SurfaceGroup` struct shape and the host populator pattern for `bridge_areas`; return SUMMARY ≤ 150 words" — purpose: Step 1 view-accessor + host-populator template.
- "Run `cargo check --workspace --all-targets` after each step; return FACT pass/fail + SNIPPETS ≤ 20 lines on fail" — purpose: cross-crate compile gate.
- "Run `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd`; return FACT pass/fail + assertion text on fail" — purpose: AC-1 verification.
- "Run `cargo test -p slicer-helpers --test inner_wall_material_boundary_tdd`; return FACT pass/fail" — purpose: AC-2.
- "Run `cargo test -p slicer-runtime --test contract only_one_wall_top_tdd only_one_wall_first_layer_tdd`; return FACT pass/fail per test" — purpose: AC-4 + AC-5.
- "Run `cargo xtask build-guests --check`; return FACT (clean / STALE list ≤ 5 entries)" — purpose: Step 1 closure gate after WIT change.

## Data and Contract Notes

- IR or manifest contracts touched: `SliceRegionView` gains two read-only accessors. WIT side gains two `func()` declarations. `SliceRegionData` (host-side mirror) gains two fields. No IR-side struct shape changes — `Point3WithWidth` already has the `is_bridge`/`tool_index`/`overhang_quartile` fields the modules now populate.
- WIT boundary considerations: per CLAUDE.md WIT/Type Changes Checklist, `cargo build --tests` must pass after the WIT edit before Step 1 closes. The `surface-group` record type already exists from `surface-classification-ir` plumbing; the new accessor uses it without redefining.
- Determinism or scheduler constraints: none. The per-vertex propagation is deterministic (point-in-polygon is a pure function over its inputs); the two wall-count gates are deterministic conditionals.
- `T-024` deferral contract: every emit path that constructs a `Point3WithWidth` MUST set `overhang_quartile: None`. The doc-comment cites `docs/specs/overhang-pipeline-restructuring.md` O-T031 as the future producer. When the sibling roadmap lands, T-024's full implementation is a small follow-up packet that flips this `None` to a point-in-quartile-polygon test.

## Locked Assumptions and Invariants

- `is_bridge` semantics: a wall vertex is `is_bridge: true` if and only if its XY point lies inside one of `region.bridge_areas()`. Edge ambiguity (vertex exactly on the boundary) defaults to `false` (strict-inside test).
- Inner-wall `WallBoundaryType` is computed by the same `build_wall_flags` logic as outer walls. There is no shortcut path. If inner-wall paint is empty, the result is `WallBoundaryType::Interior` (no material boundary); if paint exists with no transitions, `ExteriorSurface`; if transitions exist, `MaterialBoundary { segments: vec![...] }`.
- `Point3WithWidth.overhang_quartile = None` is invariant until the sibling roadmap lands. The deviation registration documents this.
- `only_one_wall_top` triggers **only** when `region.top_shell_index() == Some(0)` (exactly the topmost solid layer). `Some(1)` and `Some(2)` (sub-top shells) do NOT trigger — matches OrcaSlicer's `top_shell_index == 0` gate.
- `only_one_wall_first_layer` triggers **only** when `_layer_index == 0`. Layer 1 onwards is unaffected.

## Risks and Tradeoffs

- Host-populator `overhang_areas` returns an empty Vec until sibling roadmap O-T010 lands (which adds `OverhangRegion.xy_footprint`). The AC-3 accessor signature is correct; the data flow is correct; the values are just empty. Document in the closure log so the implementer who lands O-T010 knows this packet's accessor consumes the new data automatically without further changes here.
- Inner-wall paint extraction depends on the inner polygon's contour having `segment_annotations` keyed by the inner contour's vertex indices. The current modules build inner walls via iterative offset, and the offset operation does **not** carry paint values forward — segment_annotations are on the original SlicedRegion's polygons, not on the inset polygons. Mitigation: the inner-wall flag computation in this packet uses the **original** region's `segment_annotations`, sampled by nearest-vertex projection from the inner-polygon vertices back to the original-polygon vertices. Documented in `perimeter_utils.rs` doc-comment with a `TODO` for a more precise inner-wall paint sampler in Phase 5 work.
- The `only_one_wall_top`/`only_one_wall_first_layer` gates change wall geometry. Existing single-color test fixtures may have been calibrated against the pre-packet wall count. AC-N2 catches the case where the flag is supposed to be a no-op; the integration-tests-touching files MUST be re-baselined per fixture if needed. Document re-baselined SHAs in the closure log.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 3 — per-vertex is_bridge + inner-wall material boundary propagation across both modules; two-module rewrite + new tests).
- Highest-risk dispatch (the one whose return could blow budget if mis-shaped): "Summarize `docs/02_ir_schemas.md` SurfaceGroup + bridge_areas populator pattern" — MUST return ≤ 150 words. Anything longer likely includes code; re-dispatch tighter.

## Open Questions

- `[FWD]` Inner-wall paint sampling strategy: nearest-vertex projection is a pragmatic stopgap (good enough for most cases; lossy for sub-line-width features). If the implementer finds the regression baseline shifts more than 5% of vertex flags during AC-2 testing, escalate; otherwise document the choice in the perimeter_utils doc-comment and proceed.
- `[FWD]` `flow_factor` config key shape (T-025): the roadmap defers actual flow-compensation. If the implementer wants a config key registered now (e.g. `flow_compensation: float`, default `1.0`), add it under §"Walls" in `docs/15_config_keys_reference.md`. Otherwise the per-vertex `flow_factor: 1.0` hardcode stays, documented as "future work".

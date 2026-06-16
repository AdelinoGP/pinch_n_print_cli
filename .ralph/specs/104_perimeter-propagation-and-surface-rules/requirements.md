# Requirements: 104_perimeter-propagation-and-surface-rules

## Packet Metadata

- Grouped task IDs:
  - `T-020` — Per-vertex `is_bridge` from `region.bridge_areas()` containment
  - `T-021` — Per-vertex `tool_index` propagated to **inner** walls (not just outer)
  - `T-022` — Drop hardcoded `WallBoundaryType::Interior` for inner walls
  - `T-023` — Expose `OverhangRegion` lookup (`overhang_areas()`) on `SliceRegionView` (scoped to `extra_perimeters_on_overhangs` consumer; quartile work is sibling roadmap)
  - `T-024` — Per-vertex `overhang_quartile` derivation **deferred**: ship as `None` with registered deviation (sibling roadmap precondition unmet)
  - `T-025` — Per-vertex `flow_factor` plumbing: read from config when present; document `1.0` default rationale
  - `T-030` — Register `only_one_wall_top` config key
  - `T-031` — Implement `only_one_wall_top` (top-shell-index gated wall_count = 1)
  - `T-032` — Register `only_one_wall_first_layer` config key
  - `T-033` — Implement `only_one_wall_first_layer` (layer-0 gated wall_count = 1)
- Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

PrePass already exposes per-region bridge polygons (`SlicedRegion.bridge_areas`, populated by `MeshAnalysis` per packet 36-rev1), top/bottom shell indices (`top_shell_index`, `bottom_shell_index`), non-planar surface IDs (`nonplanar_surface`), and full `SurfaceClassificationIR.OverhangRegion` data. The perimeter modules **read none of this beyond polygon outlines** — `is_bridge` is hardcoded false on every emitted vertex; inner walls carry hardcoded `WallBoundaryType::Interior` regardless of multi-tool paint; `top_shell_index == Some(0)` does not reduce wall count even when the user sets `only_one_wall_top`; first-layer wall count is the same as mid-print wall count even when the user sets `only_one_wall_first_layer`. These four defaults silently override real upstream data, producing wall geometry that disagrees with the user's intent and with OrcaSlicer parity.

This packet wires that data through, end to end: extends `SliceRegionView` with the missing accessors (`overhang_areas`, `surface_group`), extends the shared `build_wall_flags` helper from packet 102 to compute per-vertex flags for both outer and inner walls (not just outer), and adds the two top/first-layer wall-count overrides. `overhang_quartile` per-vertex propagation (T-024) is the one exception: the algorithm needs cross-layer mesh-cross-section data the sibling roadmap (`overhang-pipeline-restructuring`) is preparing. Until that lands, this packet documents the deferral as a registered deviation rather than emit incorrect data or leave the field dead in IR.

## In Scope

- `crates/slicer-sdk/src/views.rs`: add `pub fn overhang_areas(&self) -> &[ExPolygon]` and `pub fn surface_group(&self) -> Option<&SurfaceGroup>` accessors on `SliceRegionView`. Host populator (`crates/slicer-wasm-host/src/host.rs`) fills both from `SurfaceClassificationIR` at view-construction.
- `crates/slicer-schema/wit/deps/ir-types.wit`: mirror `overhang-areas: func() -> list<ex-polygon>;` and `surface-group: func() -> option<surface-group>;` (the `surface-group` record itself already exists from PrePass IRs).
- `crates/slicer-core/src/perimeter_utils.rs`: extend `build_wall_flags` to accept an `is_outer: bool` parameter (default trait usage is the existing outer path); add a new code path that runs the same Material/FuzzySkin propagation on inner walls. Add point-in-polygon helper for `is_bridge` derivation. Add `flow_factor` resolution helper.
- Both `lib.rs` files in `classic-perimeters` and `arachne-perimeters`: call `build_wall_flags` for inner walls in addition to outer; consume `region.bridge_areas()` for per-vertex `is_bridge`; read `only_one_wall_top` and `only_one_wall_first_layer` from `_config`; explicitly set `Point3WithWidth.overhang_quartile = None` with doc-comment citing sibling roadmap.
- Both manifests: register `only_one_wall_top` (bool, default `false`) and `only_one_wall_first_layer` (bool, default `false`).
- `docs/15_config_keys_reference.md`: register both new keys.
- `docs/05_module_sdk.md`: document the two new `SliceRegionView` accessors.
- `docs/DEVIATION_LOG.md`: register `D-104-OVERHANG-QUARTILE-NONE`.
- 5 new TDD files covering AC-1 through AC-5 + the negatives.

## Out of Scope

- Per-vertex `overhang_quartile` actual derivation — sibling roadmap `overhang-pipeline-restructuring`.
- `extra_perimeters_on_overhangs` (T-077) — needs `overhang_areas` (this packet supplies it) but is itself a Phase 7 task; the consumer wiring lives in a later packet.
- Non-planar wall emission (T-074b/c/d) — depends on `surface_group()` (this packet supplies the accessor) but the wall-emission branching is Phase 7.
- Wall-sequence reordering — Phase 5.
- Thin-walls / gap-fill — Phase 6.
- `flow_factor` actual flow-compensation computation — T-025 explicitly defers the algorithm. This packet only ensures the field is **read** from config when present and **documented** as `1.0` default when absent.

## Authoritative Docs

| Doc | Size | Read strategy |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | ~600 lines | Range-read §"Phase 2 — Upstream-data propagation" and §"Phase 3 — Surface-driven wall-count rules". |
| `docs/specs/overhang-pipeline-restructuring.md` | ~150 lines | Read full — small, sibling-roadmap context for T-024 deferral + AC-3 accessor signature alignment. |
| `docs/02_ir_schemas.md` | ~900 lines | Delegate SUMMARY for `BridgeRegion`, `OverhangRegion`, `SurfaceGroup`, `SurfaceClassificationIR`. Range-read around `Point3WithWidth` directly. |
| `docs/05_module_sdk.md` | ~500 lines | Delegate SUMMARY for `SliceRegionView` accessor + WIT-mirror convention. |
| `docs/15_config_keys_reference.md` | ~300 lines | Range-read §"Walls" to align the two new key entries with existing format. |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1574-1577,1715` — `only_one_wall_top` and `only_one_wall_first_layer` gating conditions. Delegate a SUMMARY (≤ 100 words) of the gate logic.

## Acceptance Summary

- Positive cases: `AC-1` (is_bridge propagation), `AC-2` (inner-wall MaterialBoundary), `AC-3` (view accessors), `AC-4` (only_one_wall_top), `AC-5` (only_one_wall_first_layer), `AC-6` (T-024 deferred + deviation logged).
- Negative cases: `AC-N1` (empty bridge_areas → no panics, no false positives), `AC-N2` (non-top layer → top-only-wall flag is no-op).
- Refinements not captured in Given/When/Then:
  - `AC-3`'s WIT accessor name MUST be `overhang-areas`, not `overhang_regions`, because the data shape is `Vec<ExPolygon>` (already-projected XY footprints), not raw `OverhangRegion` structs. Naming convention follows `bridge-areas`. The accessor is host-populated by intersecting `OverhangRegion.xy_footprint` (which the sibling roadmap O-T010 adds) with this region's polygon; until O-T010 lands, the accessor returns an empty slice — silent dependency that this packet documents in the closure log.
- Cross-packet impact: depends on packet `102_perimeter-modules-foundations`; independent of packet `103_slicer-helpers-polygon-ops`. Unblocks Phase 5/6 packets that consume the per-vertex flag and the view accessors.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Cross-crate compile after SDK + WIT additions | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace clippy gate | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd` | AC-1 + AC-N1 | FACT pass/fail |
| `cargo test -p slicer-core --test inner_wall_material_boundary_tdd` | AC-2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract only_one_wall_top_tdd` | AC-4 + AC-N2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract only_one_wall_first_layer_tdd` | AC-5 | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence after WIT change | FACT clean / STALE list |
| `rg -q 'D-104-OVERHANG-QUARTILE-NONE' docs/DEVIATION_LOG.md` | AC-6 deviation entry landed | FACT pass/fail |

## Step Completion Expectations

- Cross-step invariant: existing `boundary_paint_tdd.rs` tests in both perimeter modules MUST stay green after every step. They cover outer-wall paint propagation regression — the per-region paint paths must not regress when inner-wall propagation is added.
- Step ordering rationale: SDK + WIT accessors land first (Step 1) because the perimeter modules consume them. `build_wall_flags` extension lands second (Step 2) — same reason. Per-vertex `is_bridge` consumption (Step 3) follows because the test fixture needs the view accessors AND the helper extension. Then the two surface rules (Step 4) and finally docs (Step 5).
- Shared scratch state: none.

## Context Discipline Notes

- `crates/slicer-sdk/src/views.rs` is ~360 lines — range-read by `rg -n 'impl SliceRegionView|fn (bridge_areas|top_shell_index|nonplanar_surface)'` then ±40 lines around each hit.
- `crates/slicer-wasm-host/src/host.rs` is large — DO NOT load in full. Range-read by `rg -n 'sliced_region_to_data|SliceRegionData'` and edit only the populator path.
- Both perimeter modules' `lib.rs` files are post-packet-102 state (≈ 400–600 LOC each after the helper extraction). Range-read each file's `run_perimeters` body only.
- Likely temptation read: `crates/slicer-core/src/algos/mesh_analysis.rs` to see how `OverhangRegion.xy_footprint` is computed. **Skip** — that's sibling roadmap O-T010 territory. The accessor introduced here just reads whatever `xy_footprint` exists (currently empty until O-T010 lands); the data flow is correct regardless.
- Sub-agent return-format for the heaviest dispatch: the `only_one_wall_top/first_layer` OrcaSlicer SUMMARY must return ≤ 100 words. Anything longer indicates the SUMMARY is including code instead of behavior description; re-dispatch tighter.

---
status: draft
packet: 102_perimeter-propagation-and-surface-rules
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
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 102_perimeter-propagation-and-surface-rules

## Goal

Make both perimeter modules read the per-region data already exposed by upstream PrePass IRs — per-vertex `is_bridge` from `region.bridge_areas()`, multi-segment `MaterialBoundary` on **inner** walls (not just outer), and `surface_group()` lookup for the future non-planar shell_count — and honour two surface-driven wall-count overrides (`only_one_wall_top` and `only_one_wall_first_layer`) that the roadmap's Phase 3 introduces.

## Scope Boundaries

Touches `slicer-sdk` (new view accessors for `overhang_areas` and `surface_group`), `slicer-ir` WIT mirror for those accessors, the shared `slicer-helpers::perimeter_utils` (extend `build_wall_flags` to drive both outer and inner walls), and both `classic-perimeters` / `arachne-perimeters` `lib.rs` + `.toml` files to consume the new view accessors and register the two new config keys. `overhang_quartile` per-vertex propagation (T-024) lands as `None` with a registered deviation since the sibling roadmap (`overhang-pipeline-restructuring`) has not yet shipped its Phase 3 accessors.

## Prerequisites and Blockers

- Depends on:
  - Packet `100_perimeter-modules-foundations` — needs the shared `slicer-helpers::perimeter_utils` crate and the widened `WallBoundaryType::MaterialBoundary` Vec representation (T-013 in packet 100 → T-021/T-022 inner-wall material boundary here).
- Unblocks:
  - Phase 5 (Classic spacing model + wall sequencing) and Phase 6 (thin-walls + gap-fill) in M1 — once per-vertex propagation is correct, those phases consume the same flag types.
- Activation blockers: none — D-4 closed (extend `SliceRegionView` per ADR-level decision in the roadmap), D-10 closed (overhang quartile derivation deferred to sibling roadmap), D-11 closed (non-planar wall emission in scope).
- Conditional precondition: T-024 ships in "leave None" mode if `overhang-pipeline-restructuring` Phase 3 (O-T030/O-T031) has not landed at implementation time. The deviation is registered in `docs/DEVIATION_LOG.md` as part of this packet.

## Acceptance Criteria

- **AC-1. Given** a `SlicedRegion` whose `bridge_areas` contains a single rectangle covering exactly the right half of its outer polygon, **when** `run_perimeters` emits the outer wall, **then** wall vertices whose XY lies inside the bridge rectangle have `feature_flags[i].is_bridge == true` and vertices outside have `is_bridge == false` (point-in-polygon test; one transition expected at the rectangle boundary). | `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** a multi-tool polygon where outer **and** an inner wall (perimeter_index = 1) cross a material boundary between tools 1 and 2, **when** `build_wall_flags` is invoked for each wall, **then** the inner wall's `boundary_type` is `WallBoundaryType::MaterialBoundary { segments: vec![MaterialBoundarySegment { near_tool: Some(1), far_tool: Some(2), .. }] }` (NOT the pre-packet hardcoded `Interior`), and the inner wall's `feature_flags[i].tool_index` reflects per-vertex tool membership. | `cargo test -p slicer-helpers --test inner_wall_material_boundary_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the extended `SliceRegionView`, **when** the public surface is inspected, **then** the view exposes `pub fn overhang_areas(&self) -> &[ExPolygon]` and `pub fn surface_group(&self) -> Option<&SurfaceGroup>` accessors (visible via `cargo doc` and via the WIT-mirrored `slice-region-view` interface), and the host-side populator fills both fields from `SurfaceClassificationIR` at view-construction. | `rg -q 'pub fn overhang_areas\(&self\) -> &\[ExPolygon\]' crates/slicer-sdk/src/views.rs && rg -q 'pub fn surface_group\(&self\) -> Option<&SurfaceGroup>' crates/slicer-sdk/src/views.rs && rg -q 'overhang-areas: func\(\) -> list<ex-polygon>' crates/slicer-schema/wit/deps/ir-types.wit`
- **AC-4. Given** a region with `top_shell_index() == Some(0)`, a base `wall_count = 4`, and the config `only_one_wall_top = true`, **when** `run_perimeters` runs, **then** the resulting `PerimeterRegion.walls` contains exactly **1** outer wall (`loop_type = Outer`) and zero inner walls. With `only_one_wall_top = false` on the same fixture, the wall count is 4. | `cargo test -p slicer-runtime --test contract only_one_wall_top_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** a region with base `wall_count = 4` running at `layer_index = 0` and config `only_one_wall_first_layer = true`, **when** `run_perimeters` runs, **then** `walls.len() == 1`. At `layer_index = 5` on the same fixture, `walls.len() == 4`. | `cargo test -p slicer-runtime --test contract only_one_wall_first_layer_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the current state of `docs/specs/overhang-pipeline-restructuring.md` (sibling roadmap Phase 3 not landed at packet implementation time), **when** any vertex flag is emitted, **then** every `Point3WithWidth.overhang_quartile` is `None`, the module doc-comment explicitly states the deferral, and `docs/DEVIATION_LOG.md` carries a `D-<packet>-OVERHANG-QUARTILE-NONE` entry referencing this packet and the sibling roadmap. | `rg -q 'overhang_quartile.*None.*sibling roadmap' modules/core-modules/classic-perimeters/src/lib.rs && rg -q 'overhang_quartile.*None.*sibling roadmap' modules/core-modules/arachne-perimeters/src/lib.rs && rg -q 'D-.*-OVERHANG-QUARTILE-NONE' docs/DEVIATION_LOG.md`

## Negative Test Cases

- **AC-N1. Given** a `SlicedRegion` with **empty** `bridge_areas` (the common case for non-bridge layers), **when** the outer wall is emitted, **then** every `feature_flags[i].is_bridge == false` and no point-in-polygon test panics on the empty polygon list. | `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd no_bridge_areas_case -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a region with `top_shell_index() == None` (a non-top layer), **when** `only_one_wall_top = true` is set, **then** the wall count is **unchanged** at the configured base value (`wall_count = 4` stays `4`). The flag does not apply outside top regions. | `cargo test -p slicer-runtime --test contract only_one_wall_top_tdd non_top_layer_case -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd only_one_wall_top_tdd only_one_wall_first_layer_tdd && cargo test -p slicer-helpers --test inner_wall_material_boundary_tdd`

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 2 (T-020 through T-025), Phase 3 (T-030 through T-033). Range-read the two sub-tables.
- `docs/specs/overhang-pipeline-restructuring.md` — sibling roadmap; range-read Phase 3 (O-T030/O-T031) to confirm the planned accessor signatures (so AC-3's `overhang_areas` accessor matches).
- `docs/02_ir_schemas.md` — `SurfaceClassificationIR`, `BridgeRegion`, `SurfaceGroup`, `Point3WithWidth` (delegate SUMMARY).
- `docs/05_module_sdk.md` — `SliceRegionView` accessors + WIT plumbing convention (delegate SUMMARY).
- `docs/15_config_keys_reference.md` — current config-key registration format.

## Doc Impact Statement (Required)

This packet modifies the following doc sections:

- `docs/15_config_keys_reference.md` §"Walls" — register `only_one_wall_top` (bool, default false) and `only_one_wall_first_layer` (bool, default false) — `rg -q 'only_one_wall_top.*bool.*default: false' docs/15_config_keys_reference.md && rg -q 'only_one_wall_first_layer.*bool.*default: false' docs/15_config_keys_reference.md`
- `docs/05_module_sdk.md` §"SliceRegionView accessors" — document new `overhang_areas()` and `surface_group()` accessors — `rg -q 'overhang_areas.*ExPolygon' docs/05_module_sdk.md && rg -q 'surface_group.*SurfaceGroup' docs/05_module_sdk.md`
- `docs/DEVIATION_LOG.md` — add `D-<packet>-OVERHANG-QUARTILE-NONE` entry — `rg -q 'D-.*-OVERHANG-QUARTILE-NONE' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `only_one_wall_top` (≈ line 1574, 1715) and `only_one_wall_first_layer` (≈ line 1574) gating conditions. Delegate a SUMMARY (≤ 100 words) of the gate logic.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

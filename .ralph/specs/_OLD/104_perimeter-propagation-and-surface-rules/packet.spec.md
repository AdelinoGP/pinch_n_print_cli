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
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 104_perimeter-propagation-and-surface-rules

## Goal

Make both perimeter modules read the per-region data already exposed by upstream PrePass IRs — per-vertex `is_bridge` from `region.bridge_areas()`, multi-segment `MaterialBoundary` on **inner** walls (not just outer), and `surface_group()` lookup for the future non-planar shell_count — and honour two surface-driven wall-count overrides (`only_one_wall_top` and `only_one_wall_first_layer`) that the roadmap's Phase 3 introduces.

## Scope Boundaries

Touches `slicer-sdk` (new view accessors for `overhang_areas` and `surface_group`), `slicer-ir` WIT mirror for those accessors (including NEW `surface-group` WIT record definition), the shared `slicer_core::perimeter_utils` (rename `build_outer_wall_flags` → `build_wall_flags` adding an `is_outer: bool` parameter — defensible P102 spillover), and both `classic-perimeters` / `arachne-perimeters` `lib.rs` + `.toml` files to consume the new view accessors and register the two new config keys. `overhang_quartile` per-vertex propagation (T-024) lands as `None` with a registered deviation since the sibling roadmap (`overhang-pipeline-restructuring`) has not yet shipped its Phase 3 accessors.

## Prerequisites and Blockers

- Depends on:
  - Packet `102_perimeter-modules-foundations` — status **implemented**. Needs the shared `slicer_core::perimeter_utils` module (containing `build_outer_wall_flags` at `crates/slicer-core/src/perimeter_utils.rs:30`) and the widened `WallBoundaryType::MaterialBoundary` Vec representation (T-013 in packet 102 → T-021/T-022 inner-wall material boundary here).
- Forward dependencies (both packets are `status: draft` — do NOT treat as satisfied):
  - Packet `106_overhang-pipeline-prepass-foundation` (status `draft`) — adds the **net-new** field `OverhangRegion.xy_footprint: Vec<ExPolygon>` AND its population via O-T010. The field does **not** exist in the current tree (`OverhangRegion` at `crates/slicer-ir/src/slice_ir.rs:586` has no footprint field; `slice_ir.rs:581` is the analogous `BridgeRegion.xy_footprint`). Because the field is absent, this packet's `overhang_areas()` host populator returns `Vec::new()` and does **not** reference the field — only the accessor signature, WIT func, and empty-stub populator land here. This is a true forward-dep but **not** a build/activation blocker: the empty-stub populator compiles against the current tree (it never touches the missing field). AC-3-EMPTY pins the empty return as a regression bed. When P106 lands the field, a small follow-up wires the `xy_footprint`-intersection into the populator body.
- Activation blockers: none. Grilling-decision status (per `docs/specs/perimeter-modules-orca-parity-roadmap.md`): **D-10 explicitly closed** (overhang-quartile derivation moved to the sibling roadmap, strikethrough-tagged); **D-4 resolved** (resolution "Extend `SliceRegionView`" recorded inline, no closed-tag) — this packet implements that resolution; **D-11 resolved-direction "include"** (non-planar wall emission in scope) but the decision is still referenced as active driving roadmap T-074b, so it is "resolved/in-progress", not formally closed. None gate P104 activation.
- Conditional precondition: T-024 ships in "leave None" mode if `overhang-pipeline-restructuring` Phase 3 (O-T030/O-T031) has not landed at implementation time. The deviation is registered in `docs/DEVIATION_LOG.md` as part of this packet.

## Acceptance Criteria

- **AC-1. Given** a `SlicedRegion` whose `bridge_areas` contains a single rectangle covering exactly the right half of its outer polygon, **when** `run_perimeters` emits the outer wall, **then** wall vertices whose XY lies inside the bridge rectangle have `feature_flags[i].is_bridge == true` and vertices outside have `is_bridge == false` (point-in-polygon test; one transition expected at the rectangle boundary). | `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** a multi-tool polygon where outer **and** an inner wall (perimeter_index = 1) cross a material boundary between tools 1 and 2, **when** `build_wall_flags(…, is_outer: false, …)` is invoked for the inner wall, **then** the inner wall's `boundary_type` is `WallBoundaryType::MaterialBoundary { segments: vec![MaterialBoundarySegment { near_tool: Some(1), far_tool: Some(2), .. }] }` (NOT the pre-packet hardcoded `Interior`), and the inner wall's `feature_flags[i].tool_index` reflects per-vertex tool membership. | `cargo test -p slicer-core --test inner_wall_material_boundary_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-2b. Given** the same multi-tool fixture run end-to-end through `run_perimeters`, **when** the inner wall's `boundary_type` is inspected from the `slicer-runtime` contract level, **then** it is `WallBoundaryType::MaterialBoundary{..}` (not `Interior`). | `cargo test -p slicer-runtime --test contract inner_wall_boundary_type_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the extended `SliceRegionView`, **when** the public surface is inspected, **then** the view exposes `pub fn overhang_areas(&self) -> &[ExPolygon]` and `pub fn surface_group(&self) -> Option<&SurfaceGroup>` accessors (visible via `cargo doc` and via the WIT-mirrored `slice-region-view` interface with a newly-defined `surface-group` WIT record). The `surface_group()` field is host-populated from `SurfaceClassificationIR` at view-construction; `overhang_areas()` is populated to an empty Vec (forward-dep on P106's net-new `OverhangRegion.xy_footprint` — see Prerequisites). | `rg -q 'pub fn overhang_areas\(&self\) -> &\[ExPolygon\]' crates/slicer-sdk/src/views.rs && rg -q 'pub fn surface_group\(&self\) -> Option<&SurfaceGroup>' crates/slicer-sdk/src/views.rs && rg -q 'overhang-areas: func\(\) -> list<ex-polygon>' crates/slicer-schema/wit/deps/ir-types.wit && rg -q 'record surface-group' crates/slicer-schema/wit/deps/ir-types.wit`
- **AC-3-EMPTY. Given** the current tree (P106 not yet landed, so `OverhangRegion.xy_footprint` does not exist), **when** a `SliceRegionView` is constructed for any region via the host populator, **then** `overhang_areas()` returns an empty slice (`is_empty() == true`) and constructing/calling the accessor does not reference the missing field (compiles against the current tree). This is the regression bed that P106 later flips to a non-empty assertion. | `cargo test -p slicer-runtime --test contract overhang_areas_empty_until_p106_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** a region with `top_shell_index() == Some(0)`, a base `wall_count = 4`, and the config `only_one_wall_top = true`, **when** `run_perimeters` runs, **then** the resulting `PerimeterRegion.walls` contains exactly **1** outer wall (`loop_type = Outer`) and zero inner walls. With `only_one_wall_top = false` on the same fixture, the wall count is 4. **And** for a sub-top region (`top_shell_index() == Some(N>0)`) with `top_solid_fill` covering part of the region and `only_one_wall_top = true`, the `split_top_surfaces` carve emits a 1-wall band over `region ∩ top_solid_fill` and full `wall_count` over `region ∖ top_solid_fill` (tested by `sub_top_layer_carve_case`); with `only_one_wall_top = false` the sub-top region keeps full `wall_count` (tested by `sub_top_layer_noop_when_flag_disabled`). | `cargo test -p slicer-runtime --test contract only_one_wall_top_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** a region with base `wall_count = 4` running at `layer_index = 0` and config `only_one_wall_first_layer = true`, **when** `run_perimeters` runs, **then** `walls.len() == 1`. At `layer_index = 5` on the same fixture, `walls.len() == 4`. | `cargo test -p slicer-runtime --test contract only_one_wall_first_layer_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the current state of `docs/specs/overhang-pipeline-restructuring.md` (sibling roadmap Phase 3 not landed at packet implementation time), **when** any wall vertex is emitted, **then** every `Point3WithWidth.overhang_quartile` is `None` at every construction site, each carrying an inline doc-comment citing the sibling roadmap, and `docs/DEVIATION_LOG.md` carries a `D-104-OVERHANG-QUARTILE-NONE` entry. The construction sites are: the **shared** `expolygon_to_path3d` helper (`crates/slicer-core/src/perimeter_utils.rs` — used by BOTH classic and arachne; this is where classic's per-vertex fields originate, classic has no inline `Point3WithWidth` literal) and arachne's inline variable-width path (`modules/core-modules/arachne-perimeters/src/lib.rs`). Both already set `overhang_quartile: None` / `flow_factor: 1.0`; this AC adds the deferral doc-comment. | `rg -q 'overhang_quartile.*None.*sibling roadmap' crates/slicer-core/src/perimeter_utils.rs && rg -q 'overhang_quartile.*None.*sibling roadmap' modules/core-modules/arachne-perimeters/src/lib.rs && rg -q 'D-104-OVERHANG-QUARTILE-NONE' docs/DEVIATION_LOG.md`

## Surface-Rule Parity Note (MED-2)

`only_one_wall_top` reduces walls on top solid surfaces across all three `top_shell_index()` branches. For the topmost layer (`Some(0)`) it is a blanket gate (1 outer wall, region-wide — OrcaSlicer parity). For sub-top layers (`Some(N>0)`) the `split_top_surfaces` carve (ported from OrcaSlicer `PerimeterGenerator.cpp:775`, adapted to reuse our pre-classified `top_solid_fill` rather than re-deriving from `upper_slices`) partitions the region: `region ∩ top_solid_fill` emits a single wall, `region ∖ top_solid_fill` keeps the full `wall_count`. For non-top layers (`None`) the key is a no-op. (Scope note: the sub-top carve was added during implementation — the original packet plan deferred it. The previously planned deviation `D-104-ONLY-ONE-WALL-TOP-SUBTOP` is therefore NOT registered; see `closure-log.md` §Post-Activation Scope Expansion.)

## Negative Test Cases

- **AC-N1. Given** a `SlicedRegion` with **empty** `bridge_areas` (the common case for non-bridge layers), **when** the outer wall is emitted, **then** every `feature_flags[i].is_bridge == false` and no point-in-polygon test panics on the empty polygon list. | `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd no_bridge_areas_case -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a region with `top_shell_index() == None` (a non-top layer), **when** `only_one_wall_top = true` is set, **then** the wall count is **unchanged** at the configured base value (`wall_count = 4` stays `4`). The flag does not apply outside top regions. | `cargo test -p slicer-runtime --test contract only_one_wall_top_tdd non_top_layer_case -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test contract per_vertex_is_bridge_propagation_tdd only_one_wall_top_tdd only_one_wall_first_layer_tdd inner_wall_boundary_type_tdd 2>&1 | tee target/test-output.log && cargo test -p slicer-core --test inner_wall_material_boundary_tdd 2>&1 | tee -a target/test-output.log`

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 2 (T-020 through T-025), Phase 3 (T-030 through T-033). Range-read the two sub-tables.
- `docs/specs/overhang-pipeline-restructuring.md` — sibling roadmap; range-read Phase 3 (O-T030/O-T031) to confirm the planned accessor signatures (so AC-3's `overhang_areas` accessor matches).
- `docs/02_ir_schemas.md` — `SurfaceClassificationIR`, `BridgeRegion`, `SurfaceGroup`, `Point3WithWidth` (delegate SUMMARY).
- `docs/05_module_sdk.md` — `SliceRegionView` accessors + WIT plumbing convention (delegate SUMMARY).
- `docs/15_config_keys_reference.md` — no "Walls" section exists yet; implementer CREATES it.

## Doc Impact Statement (Required)

This packet modifies the following doc sections:

- `docs/15_config_keys_reference.md` — CREATE new §"Walls" section; register `only_one_wall_top` (bool, default false) and `only_one_wall_first_layer` (bool, default false) — `rg -q 'only_one_wall_top.*bool.*default: false' docs/15_config_keys_reference.md && rg -q 'only_one_wall_first_layer.*bool.*default: false' docs/15_config_keys_reference.md`
- `docs/05_module_sdk.md` §"SliceRegionView accessors" — document new `overhang_areas()` and `surface_group()` accessors — `rg -q 'overhang_areas.*ExPolygon' docs/05_module_sdk.md && rg -q 'surface_group.*SurfaceGroup' docs/05_module_sdk.md`
- `docs/DEVIATION_LOG.md` — CREATE `D-104-OVERHANG-QUARTILE-NONE` entry — `rg -q 'D-104-OVERHANG-QUARTILE-NONE' docs/DEVIATION_LOG.md` (Note: the originally planned `D-104-ONLY-ONE-WALL-TOP-SUBTOP` is NOT registered — sub-top reduction was implemented this session via `split_top_surfaces`, not deferred.)

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

## Deviations

- [T-024 / AC-6] — Specified: per-vertex `overhang_quartile` derivation | Implemented: `overhang_quartile` left `None` at all `Point3WithWidth` construction sites with a sibling-roadmap doc-comment | Reason: genuine structural forward-dependency on `overhang-pipeline-restructuring` Phase 3 (O-T031), which introduces the quartile classification IR; closing inline would duplicate sibling-roadmap work against an unsettled IR shape. Registered as `D-104-OVERHANG-QUARTILE-NONE`. (This is the sole remaining deviation. The originally planned `D-104-ONLY-ONE-WALL-TOP-SUBTOP` and `D-104-SURFACE-GROUP-NOT-THREADED` were retired by implementing the sub-top `split_top_surfaces` carve and the runtime `SurfaceClassificationIR` threading this session — see `closure-log.md` §Post-Activation Scope Expansion.)

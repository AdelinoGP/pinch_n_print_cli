# Task Map: 104_perimeter-propagation-and-surface-rules

Maps packet task IDs (T-020…T-025, T-030…T-033) to their source rows in the roadmap and to the implementation-plan steps that deliver them.

Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md` Phase 2 (lines ~184–198) and Phase 3 (lines ~200–206).

## Phase 2 — Upstream-data propagation into per-vertex flags

| Task ID | Roadmap Title | Roadmap Files | Packet Step | AC |
| --- | --- | --- | --- | --- |
| T-020 | Per-vertex `is_bridge` from `region.bridge_areas()` containment | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs`, `crates/slicer-core/src/perimeter_utils.rs` | Step 2 (helper), Step 3 (consumer) | AC-1, AC-N1 |
| T-021 | Per-vertex `tool_index` propagated to **inner** walls when material boundary exists | `crates/slicer-core/src/perimeter_utils.rs` (shared `build_wall_flags`) | Step 2 (helper), Step 3 (consumer) | AC-2, AC-2b |
| T-022 | Drop hardcoded `WallBoundaryType::Interior` for inner walls; compute boundary_type via same logic as outer | `crates/slicer-core/src/perimeter_utils.rs` | Step 2 (helper), Step 3 (consumer) | AC-2, AC-2b |
| T-023 | Expose `OverhangRegion` lookup on per-layer-per-region view (scoped to `overhang_areas()` accessor; quartile derivation is sibling roadmap) | `crates/slicer-sdk/src/views.rs`, `crates/slicer-schema/wit/deps/ir-types.wit`, `crates/slicer-wasm-host/src/host.rs` | Step 1 | AC-3 |
| T-024 | Per-vertex `overhang_quartile` derivation — **deferred**: ships as `None` with registered deviation `D-104-OVERHANG-QUARTILE-NONE` (sibling roadmap O-T031 precondition unmet) | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs` | Step 3 (doc-comment), Step 5 (deviation) | AC-6 |
| T-025 | Per-vertex `flow_factor` plumbing — hardcoded `1.0` with doc-comment; no config key registered in this packet | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs` | Step 3 | (inline doc-comment, no separate AC) |

## Phase 3 — Surface-driven wall-count rules

| Task ID | Roadmap Title | Roadmap Files | Packet Step | AC |
| --- | --- | --- | --- | --- |
| T-030 | Register `only_one_wall_top` config key in `docs/15_config_keys_reference.md` and both manifests | `docs/15_config_keys_reference.md`, both `.toml` manifests | Step 4a (manifests), Step 5 (doc) | AC-4 |
| T-031 | Read `region.top_shell_index() == Some(0)` and `only_one_wall_top == true`; force `wall_count = 1` | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs` | Step 4b | AC-4, AC-N2 |
| T-032 | Register `only_one_wall_first_layer` config key in `docs/15_config_keys_reference.md` and both manifests | `docs/15_config_keys_reference.md`, both `.toml` manifests | Step 4a (manifests), Step 5 (doc) | AC-5 |
| T-033 | Read `_layer_index == 0` and `only_one_wall_first_layer == true`; force `wall_count = 1` | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs` | Step 4b | AC-5 |

## Deferred / Deviation Registrations

| Deviation ID | Reason | Registered in Step | AC |
| --- | --- | --- | --- |
| `D-104-OVERHANG-QUARTILE-NONE` | T-024 deferred: `overhang_quartile` stays `None` until sibling roadmap O-T031 (`overhang-pipeline-restructuring` Phase 3) lands | Step 5 | AC-6 |
| `D-104-ONLY-ONE-WALL-TOP-SUBTOP` | MED-2: `only_one_wall_top` for sub-top layers (`top_shell_index() == Some(1+)`) deferred — requires `split_top_surfaces`-equivalent `top_solid_fill`-scoped reduction, planned for Phase-5 packet | Step 5 | (no separate AC — deviation documenting parity gap) |

## Forward Dependencies

| Symbol | Producing Packet | Status | Impact |
| --- | --- | --- | --- |
| `OverhangRegion.xy_footprint` population by `MeshAnalysis` | `106_overhang-pipeline-prepass-foundation` (O-T010) | draft | `overhang_areas()` returns empty slice until P106 ships; accessor signature and data flow are correct |

## New Test Files (aggregator registration required)

All files under `crates/slicer-runtime/tests/contract/` require a `mod <name>;` entry in `crates/slicer-runtime/tests/contract/main.rs`.

| Test File | Step Created | Aggregator Registration Step | Target AC |
| --- | --- | --- | --- |
| `crates/slicer-core/tests/inner_wall_material_boundary_tdd.rs` | Step 2a | N/A (slicer-core standalone `--test`) | AC-2 |
| `crates/slicer-runtime/tests/contract/per_vertex_is_bridge_propagation_tdd.rs` | Step 2a | Step 2b (`mod per_vertex_is_bridge_propagation_tdd;`) | AC-1, AC-N1 |
| `crates/slicer-runtime/tests/contract/inner_wall_boundary_type_tdd.rs` | Step 3b | Step 3b (`mod inner_wall_boundary_type_tdd;`) | AC-2b |
| `crates/slicer-runtime/tests/contract/only_one_wall_top_tdd.rs` | Step 4c | Step 4d (`mod only_one_wall_top_tdd;`) | AC-4, AC-N2 |
| `crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs` | Step 4c | Step 4d (`mod only_one_wall_first_layer_tdd;`) | AC-5 |

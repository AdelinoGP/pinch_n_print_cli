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
| T-024 | Per-vertex `overhang_quartile` derivation — **deferred**: already `None` at all sites; this packet adds the deferral doc-comment + registers `D-104-OVERHANG-QUARTILE-NONE` (sibling roadmap O-T031 precondition unmet) | shared helper `crates/slicer-core/src/perimeter_utils.rs:139` (classic + arachne helper path) + `modules/core-modules/arachne-perimeters/src/lib.rs:428` (arachne inline) | Step 2 (helper doc-comment), Step 3 (arachne inline), Step 5 (deviation) | AC-6 |
| T-025 | Per-vertex `flow_factor` plumbing — already `1.0` at both sites; document `1.0` default rationale; no config key registered in this packet | `crates/slicer-core/src/perimeter_utils.rs:138` + `modules/core-modules/arachne-perimeters/src/lib.rs:428` | Step 2 / Step 3 | (inline doc-comment, no separate AC) |

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

## Forward Dependencies

| Symbol | Producing Packet | Status | Impact |
| --- | --- | --- | --- |
| `OverhangRegion.xy_footprint: Vec<ExPolygon>` — **net-new field** (added by P106, mirrors existing `BridgeRegion.xy_footprint` at `slice_ir.rs:581`) + its population by `MeshAnalysis` | `106_overhang-pipeline-prepass-foundation` (O-T010) | draft | Field is ABSENT from current tree; `overhang_areas()` populator returns `Vec::new()` and does not reference it (compiles against current tree). AC-3-EMPTY pins the empty return. When P106 lands the field, a follow-up wires the intersection body — accessor signature unchanged. |

## New Test Files (aggregator registration required)

All files under `crates/slicer-runtime/tests/contract/` require a `mod <name>;` entry in `crates/slicer-runtime/tests/contract/main.rs`.

| Test File | Step Created | Aggregator Registration Step | Target AC |
| --- | --- | --- | --- |
| `crates/slicer-core/tests/inner_wall_material_boundary_tdd.rs` | Step 2a | N/A (slicer-core standalone `--test`) | AC-2 |
| `crates/slicer-runtime/tests/contract/overhang_areas_empty_until_p106_tdd.rs` | Step 1b | Step 1b (`mod overhang_areas_empty_until_p106_tdd;`) | AC-3-EMPTY |
| `crates/slicer-runtime/tests/contract/per_vertex_is_bridge_propagation_tdd.rs` | Step 2a | Step 2b (`mod per_vertex_is_bridge_propagation_tdd;`) | AC-1, AC-N1 |
| `crates/slicer-runtime/tests/contract/inner_wall_boundary_type_tdd.rs` | Step 3b | Step 3b (`mod inner_wall_boundary_type_tdd;`) | AC-2b |
| `crates/slicer-runtime/tests/contract/only_one_wall_top_tdd.rs` | Step 4c | Step 4d (`mod only_one_wall_top_tdd;`) | AC-4, AC-N2 |
| `crates/slicer-runtime/tests/contract/only_one_wall_first_layer_tdd.rs` | Step 4c | Step 4d (`mod only_one_wall_first_layer_tdd;`) | AC-5 |

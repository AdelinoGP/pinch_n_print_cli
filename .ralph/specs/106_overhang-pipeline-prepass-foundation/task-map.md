# Task Map: 106_overhang-pipeline-prepass-foundation

## Task IDs → Steps

| Task ID | Description | Step | Status |
| ------- | ----------- | ---- | ------ |
| O-T001  | Author `docs/adr/0022-overhang-classification-at-prepass.md` (ADR slot 0022 — next free after 0021; slot 0012 is taken by `0012-spatial-indexing-as-reconstruction-only-companions.md`) | Step 1 | pending |
| O-T002  | Close O-1..O-8 decisions inline in `docs/specs/overhang-pipeline-restructuring.md` (O-1 → ADR-0022; O-3 → extract wrapper from `triangle_mesh_slicer.rs`) | Step 1 | pending |
| O-T010  | Add `pub xy_footprint: Vec<ExPolygon>` to `OverhangRegion` (net-new, mirrors `BridgeRegion.xy_footprint` at line ~581); populate at `MeshAnalysis` construction site | Step 2 | pending |
| O-T011  | Add `pub struct QuartileBand { pub quartile: u8, pub polygons: Vec<ExPolygon> }` (net-new; P107 consumer uses same name) and `pub overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>` to `SurfaceClassificationIR`; bump `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` (currently 1.1.0) — NOT `CURRENT_SLICE_IR_SCHEMA_VERSION` | Step 2 | pending |
| O-T012  | Create `crates/slicer-core/src/algos/mesh_cross_section.rs` (NET-NEW) wrapping `slice_mesh_ex` from `crates/slicer-core/src/triangle_mesh_slicer.rs`; declare `pub mod mesh_cross_section` in `mod.rs`. NOTE: `support_geometry.rs` has NO plane-triangle intersection to promote — no changes to that file. | Step 3 | pending |
| O-T020  | Declare `"PrePass::OverhangAnnotation"` in `STAGE_ORDER` array in `crates/slicer-scheduler/src/execution_plan.rs` (the canonical stage list; `stage_order.rs` is a thin import wrapper) — after `"PrePass::LayerPlanning"` | Step 5 | pending |
| O-T021  | Implement classifier algorithm in `crates/slicer-core/src/algos/overhang_annotation.rs` (NET-NEW): per consecutive layer pair, compute cross-sections via `mesh_cross_section::cross_section_at_z`, derive distance field, partition into 4 `QuartileBand` bands | Step 4 | pending |
| O-T022  | Wire quartile thresholds to config (`outer_wall_line_width × {0.5, 1.0, 1.5, 2.0}`; fall back to `line_width` if P105 not yet shipped) | Step 4 | pending |
| O-T023  | Host stage runner: invoke `overhang_annotation` after MeshAnalysis + LayerPlanning commit; write to Blackboard `SurfaceClassificationIR.overhang_quartile_polygons` | Step 5 | pending |

## Cross-Packet Contracts

- **P104/P107/P108 consume `OverhangRegion.xy_footprint`**: field is net-new in this packet; downstream consumers reference it by name `xy_footprint: Vec<ExPolygon>`.
- **P107 consumes `QuartileBand`**: type is net-new in this packet (sole producer). P107 (O-T031 view accessor) must use `QuartileBand` by the same name from `slicer-ir`.

## Test File Placements

| Test File | Binary | Aggregator |
| --------- | ------ | ---------- |
| `crates/slicer-runtime/tests/unit/mesh_analysis_overhang_xy_footprint_tdd.rs` | `--test unit` | `crates/slicer-runtime/tests/unit/main.rs` — add `mod mesh_analysis_overhang_xy_footprint_tdd;` |
| `crates/slicer-core/tests/overhang_annotation_ramp_tdd.rs` | `--test overhang_annotation_ramp_tdd` (standalone) | no aggregator needed |
| `crates/slicer-core/tests/overhang_annotation_no_overhang_case.rs` | `--test overhang_annotation_no_overhang_case` (standalone) | no aggregator needed |
| `crates/slicer-runtime/tests/executor/prepass_overhang_annotation_stage_order_tdd.rs` | `--test executor` | verify against `tests/executor/main.rs` (or equivalent aggregator) |

## Notes on Fix #8 (ADR Dangling Refs — VERIFIED CLEAN)

grep of `docs/01_system_architecture.md` and `docs/04_host_scheduler.md` for `0022-overhang` and `overhang-classification-at-prepass` returned NO MATCHES. There are no premature references to the overhang ADR in those files. No fix required.

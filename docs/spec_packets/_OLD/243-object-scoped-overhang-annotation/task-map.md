# Task Map: 243-object-scoped-overhang-annotation

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-353` | `Step 1` | `docs/02_ir_schemas.md` §"IR Versioning Contract" | `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-ir/tests/ir_tests.rs` | - | M | Field-type replacement on the two `SurfaceClassificationIR` maps + major bump `1.3.0 → 2.0.0`; owns the full blast radius (pre-baked in Step 1). |
| `TASK-353` | `Step 2` | `docs/specs/wave-overhangs-bridge-fill-plan.md` §"Packet 1" | `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`, `crates/slicer-wasm-host/src/marshal/in_.rs`, `crates/slicer-runtime/src/visual_debug_render.rs` | - | M | Re-key the three production consumers to object-scoped nested maps; marshal reads `(region.object_id, global_layer_index)`. |
| `TASK-353` | `Step 3` | `docs/specs/wave-overhangs-bridge-fill-plan.md` §"Tests" | `crates/slicer-runtime/tests/executor/prepass_overhang_annotation_stage_order_tdd.rs`, `crates/slicer-runtime/tests/contract/slice_region_view_overhang_areas_non_empty_tdd.rs`, `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs`, `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`, `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs` | - | M | Test fallout re-keys + rename/rewrite the multi-object producer test to assert object isolation + add the no-cross-object-leak contract test. |
| `TASK-353` | `Step 4` | `docs/02_ir_schemas.md` §"IR 2 — SurfaceClassificationIR" | `docs/02_ir_schemas.md` | - | S | Update schema version to `2.0.0` and the object-scoped keying prose + packet-193 provenance note. |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.

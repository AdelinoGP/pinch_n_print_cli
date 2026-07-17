# Task Map: 172-mm-e2e-and-object-keys

Multi-task packet; this crosswalk owns the `docs/07_implementation_status.md` reconciliation. All three rows exist and are open at `docs/07_implementation_status.md:137-139`; they are flipped at closure via a worker dispatch, never a full-file read. Fork handoff item 9 folds into TASK-212 (no new ID minted). Note for the closure dispatch: the docs/07 TASK-212 row cites `model_loader.rs` — the actual symbol is `crates/slicer-model-io/src/loader.rs::object_metadata_to_config_data`; correct the row text when flipping. TASK-210's flip must record the accepted deviation: selection is global (flat `SupportIR`, no per-object identity), not per-object.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-210` | Step 2 (routing), Step 4 (docs) | `docs/02_ir_schemas.md` | `crates/slicer-runtime/src/layer_executor.rs`, `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/src/pipeline.rs` | `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` | M | Unit test proves support/interface/raft/ironing entities carry the configured filament indices instead of hardcoded 0 |
| `TASK-211` | Step 3 | `docs/02_ir_schemas.md` | `crates/slicer-runtime/tests/e2e/mm_real_fixture_gcode_tdd.rs` (new) | none (fixtures are in-repo Orca exports) | M | Real-fixture G-code E2E asserts T0/T1 emission, codifying the manual Orca-viewer verification |
| `TASK-212` | Step 1 (allowlist), Step 4 (docs) | `docs/02_ir_schemas.md` | `crates/slicer-model-io/src/loader.rs`, `crates/slicer-model-io/tests/threemf_sidecar_classification_tdd.rs` | `OrcaSlicerDocumented/src/slic3r/GUI/GUI_Factories.cpp`, `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` | M | Typed-conversion tests over all 18 new keys + unknown-key logging prove fork-authored per-object keys survive the loader |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.

# Task Map: 03-rev2_wit-canonical-source-and-validation

Use this file because the packet spans three task IDs and reopens prior packet work (03_wit-canonical-source-and-validation and 03-rev1).

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-144` | Step 1 (test fix) | `crates/slicer-host/tests/wit_drift_detection_tdd.rs` | `wit_drift_detection_tdd.rs:146-151` | None | Fix wrong `with:` key format in test assertion. The test was checking for `slicer:world-layer/config-types/config-view` but wasmtime `bindgen!` emits `slicer:world-layer/config-types@1.0.0.config-view`. |
| `TASK-144` | Step 2 (clippy fixes) | `crates/slicer-host/src/` | `execution_plan.rs`, `dag.rs`, `layer_executor.rs`, `manifest.rs`, `mesh_analysis.rs`, `prepass.rs`, `region_mapping.rs`, `slice_postprocess.rs`, `dispatch.rs`, `wit_host.rs` | None | Fix all clippy errors introduced or exposed by Rust/Cargo 1.94.0. |
| `TASK-145` | Step 2 (clippy fixes) | `crates/slicer-host/src/` | Same as above | None | Clippy errors are in slicer-host, the crate TASK-145 already covers. |
| `TASK-146` | Step 2 (clippy fixes) | `crates/slicer-host/src/` | Same as above | None | Clippy errors are in slicer-host, the crate TASK-146 already covers. |

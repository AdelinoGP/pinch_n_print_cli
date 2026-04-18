# Task Map: 04_custom-payload-widening

Use this file because the packet spans two task IDs and has a cross-packet dependency on Packet A.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-149` | Step 1 (WIT type changes) | `docs/03_wit_and_manifest.md` | Temporary proof-of-concept (not committed) | None | wasmtime confirmed to support `list<tuple<string, paint-value>>` for `wall-feature-flag.custom`. No POC needed — proceed directly to Step 2. |
| `TASK-149` | Step 2 (WIT disk files) | `docs/03_wit_and_manifest.md` | `wit/deps/types.wit`, `wit/deps/ir-types.wit` | None | Apply three WIT type changes: extrusion-role variant, paint-semantic variant, wall-feature-flag custom field. Canonical source (from Packet A). |
| `TASK-150` | Step 3 (macro converters) | `crates/slicer-macros/src/lib.rs` | `crates/slicer-macros/src/lib.rs` | None | Update `__slicer_ir_role_to_wit`, `__slicer_wit_semantic_to_ir`, `__slicer_ir_feature_to_wit`, `__slicer_wit_feature_to_ir`, `ir_to_wit_paint_semantic` for widened types. |
| `TASK-150` | Step 4 (host converters) | `crates/slicer-host/src/wit_host.rs` | `crates/slicer-host/src/wit_host.rs` | None | Update host-side converters to decode widened WIT types back to IR HashMap/String. |
| `TASK-150` | Step 5 (round-trip tests) | `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` | `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs` OR new `custom_payload_roundtrip_tdd.rs` | None | Three test cases for custom payload survival: ExtrusionRole, PaintSemantic, WallFeatureFlags. |
| `TASK-150` | Step 6 (drift detection) | Packet A step 7 | `crates/slicer-host/tests/wit_drift_detection_tdd.rs` | None | Update expected values in drift detection test to reflect widened WIT types from Packet A. |
| `TASK-149`, `TASK-150` | Step 7 (workspace gate) | — | Workspace-wide | None | `cargo build --workspace && clippy` — final workspace gate. |

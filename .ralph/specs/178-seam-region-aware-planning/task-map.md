# Task Map: 178-seam-region-aware-planning

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-284` | `Step 1` | `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, `docs/11_operational_governance_and_acceptance_gate.md` | `crates/slicer-schema/wit/**`, `crates/slicer-sdk/**`, guest shims | `SeamPlacer.hpp` identity records | M | Versioned active-region input and variant-aware output. ID re-derived 2026-07-22: the original `TASK-281` is closed under packet 117 (`support-planner::tapered_radius`). |
| `TASK-284` | `Step 2` | `docs/01_system_architecture.md`, `docs/04_host_scheduler.md` | `crates/slicer-runtime/src/prepass.rs`, `crates/slicer-wasm-host/src/{dispatch.rs,marshal/in_.rs}` | `SeamPlacer.cpp::extract_perimeter_polygons` | M | Late prepass projection and full-key harvest. |
| `TASK-284` | `Step 3` | `docs/02_ir_schemas.md`, `docs/05_module_sdk.md` | `seam-planner-default`, `slicer-ir`, `seam-placer` identity paths | `SeamPlacer.cpp::process_perimeter_polygon` | M | Vertical multi-region proof; narrows `D-168-SEAM-PREPASS-SOURCE` part (1) only. |
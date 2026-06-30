# Task Map: 127_sdk_wit_origin_propagation

This packet spans one inferred task ID (TASK-252, no existing `docs/07` entry) and cross-references three predecessor tasks. The map clarifies which predecessor packets this one builds on and which step carries the evidence for TASK-252's closure.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-252` (inferred) | Steps 1-9 | `docs/03_wit_and_manifest.md`, `docs/adr/0021-marshal-boundary-flat-functions-over-origin-bucket.md` | `crates/slicer-schema/wit/deps/ir-types.wit`, `crates/slicer-sdk/src/builders.rs`, `crates/slicer-wasm-host/src/host.rs`, `crates/slicer-macros/src/lib.rs`, 4 guest modules | `PerimeterGenerator.cpp:1501-1506,1644`, `PrintObject.cpp:1541-1892`, `Layer.cpp:296-332` | M | This task is the packet itself. Closure evidence: AC-1 (gcode metric T1 >= 1000), AC-3 (new parity test), AC-4 (new host origin test). |
| `TASK-250` (predecessor, closed) | — | — | — | — | — | Packet 126 (MMU painted-cube parity) introduced the multi-region `variant_chain` that creates the dispatch scenario this bug surfaces on. Not re-edited by this packet; cross-referenced in `docs/07` TASK-252 entry. |
| `TASK-245` (predecessor, closed) | — | — | — | — | — | Packet 95 (paint-segmentation OrcaSlicer parity port) introduced per-color region splitting. Not re-edited; cross-referenced. |
| `TASK-246` (predecessor, closed) | — | — | — | — | — | Packet 95 Phase 5 (width-limit + interlocking for multi-region). Not re-edited; cross-referenced. |

The `Context cost` column copies the per-step estimate from `implementation-plan.md`. The aggregate for TASK-252 across Steps 1-9 is `M` (no single step is `L`).
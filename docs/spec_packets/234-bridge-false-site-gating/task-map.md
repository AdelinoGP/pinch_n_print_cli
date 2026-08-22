# Task Map: 234-bridge-false-site-gating

This packet has no `docs/07_implementation_status.md` task ID — the backlog slot is the plan's W-A row (`docs/specs/bridge-parity-plan.md` §4, "Bridge classification & false-site gating", "new packet (no prior owner)"). The docs/07 crosswalk is therefore N-A; the row below records the backlog source and the code surface that closes the slot.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A (backlog_source: `docs/specs/bridge-parity-plan.md` §4 W-A row) | Step 1 | `docs/08_coordinate_system.md` | `crates/slicer-core/src/algos/prepass_slice.rs` (`gate_bridge_areas_by_unsupported_span`) | `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` (`detect_bridging_direction`), `BridgeDetector.cpp` (`unsupported_edges`) | `M` | Ports the canonical unsupported-span test (bridge area minus grown lower-layer anchors); the pure function is the F1 fix's core. |
| N-A (backlog_source: `docs/specs/bridge-parity-plan.md` §4 W-A row) | Step 2 | `docs/04_host_scheduler.md` | `crates/slicer-runtime/src/slice_postprocess_prepass.rs` (`commit_shell_classification_builtin`) | `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` (`process_external_surfaces`, `voids = diff(voids, *lower_layer_covered)`) | `M` | Wires the gate post-slice, reading `prev_layer_boundaries`/committed `SliceIR`; no new scheduler dependency (Q3 resolved). |
| N-A (backlog_source: `docs/specs/bridge-parity-plan.md` §4 W-A row) | Step 3 | none | `docs/spec_packets/234-bridge-false-site-gating/design.md` (decision note) | none | `S` | Measures the `is_valid` pre-filter keep/discard and records the rationale. |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.

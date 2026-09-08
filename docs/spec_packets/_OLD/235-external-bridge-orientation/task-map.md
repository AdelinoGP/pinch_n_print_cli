# Task Map: 235-external-bridge-orientation

## ISSUE-84 split (explicit)

Backlog issue `docs/specs/orca-feature-gap/issues/84-author-packet-p77-quality-bridging-classic-perimeters.md` owns TWO P77 keys: `bridge_angle` and `counterbore_hole_bridging`. This packet covers ONLY the **`bridge_angle` half** — the auto-detection semantics (floating-edge candidates, PC fallback, SCALED_EPSILON expand, ADR-0061 tie-break, D6 boundary representation) that replace the HEAD heuristic. The user-facing `bridge_angle` override-key plumbing (custom angle / relative-angle branches) and the whole of **`counterbore_hole_bridging` REMAINS with ISSUE-84 for a later packet.** ISSUE-84 stays open until that later packet is authored; it is not closed by this packet.

## Crosswalk

This packet has no `docs/07_implementation_status.md` task ID; the docs/07 crosswalk is therefore N-A (backlog = ISSUE-84 `bridge_angle` half). Rows record each step's backlog source, docs, code surface, and canonical refs.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| N-A (backlog: ISSUE-84 `bridge_angle` half) | Step 1 | `docs/adr/0061-deterministic-bridge-orientation-tie-break.md`, `docs/specs/bridge-parity-plan.md` §3/F2 | `crates/slicer-core/src/algos/prepass_slice.rs` (`detect_bridging_direction_deg`, `floating_edges_of_gated_area`), net-new `crates/slicer-core/tests/bridge_orientation_tdd.rs` | `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` (inline `detect_bridging_direction` overloads), `PrincipalComponents2D.cpp` (`compute_principal_components`) | `M` | Ports the active inline path with ADR-0061 tie-break and PC fallback; pure functions only, no wiring. |
| N-A (backlog: ISSUE-84 `bridge_angle` half) | Step 2 | `docs/04_host_scheduler.md` (`ShellClassification` section) | `crates/slicer-runtime/src/slice_postprocess_prepass.rs` (`commit_shell_classification_builtin` seam call), `prepass_slice.rs` (`update_external_bridge_orientation`) | `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` (`process_external_surfaces` call site, `PI + atan2` storage) | `S` | Orientation derives from 234's GATED geometry; D6 degrees-mod-180 preserved at the seam. |
| N-A (backlog: ISSUE-84 `bridge_angle` half) | Step 3 | `docs/specs/bridge-parity-plan.md` §6 (I3/I7) | `crates/slicer-core/src/algos/mesh_analysis.rs` (retire `compute_bridge_direction_deg`), `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs` (re-pin assertions) | `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` (`detect_angle` legacy class — rejected alternative, AC-N2 grounding) | `M` | Heuristic retirement + blast-radius closure; end-to-end determinism/I2/I7 guards on `resources/overhang.obj`. |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.

# Task Map: 168-seam-aligned-modes

This packet mints a new backlog task. At Step 8, add the following row to `docs/07_implementation_status.md` (via worker dispatch, matching the existing `- [x] TASK-### <description>` checklist format; highest existing ID is TASK-271, and TASK-272/273 are minted by queue packets 166/167):

`- [ ] TASK-274 Port OrcaSlicer SeamPlacer's aligned/aligned_back seam path (seam-string chaining + least-squares smoothing) into the seam-planner-default prepass and add aligned/aligned_back SeamMode consumption in seam-placer. Spec: packet 168-seam-aligned-modes.`

Mark it `[x]` with a closure note only at the acceptance ceremony.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-274` | `Step 1` | `docs/11_operational_governance_and_acceptance_gate.md` | `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`, `crates/slicer-sdk/src/traits.rs`, `crates/slicer-wasm-host/src/dispatch.rs` | none | M | Layer-plan parameter is the enabling contract change for real layer z's |
| `TASK-274` | `Step 2` | `docs/15_config_keys_reference.md` | `modules/core-modules/seam-placer/{src/lib.rs,seam-placer.toml}`, `modules/core-modules/seam-planner-default/seam-planner-default.toml` | none | S | Mode surface proves the config contract half of the task |
| `TASK-274` | `Steps 3-5` | `docs/08_coordinate_system.md`, `docs/ORCASLICER_ATTRIBUTION.md` | `modules/core-modules/seam-planner-default/src/{comparator,contours,visibility,align}.rs` | `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`, `SeamPlacer.hpp`, `Geometry/Curves.hpp` | M | The parity port itself |
| `TASK-274` | `Step 6` | none | `modules/core-modules/seam-planner-default/src/lib.rs` | none | M | Whole-object aligned plan emission (AC-4/AC-5) |
| `TASK-274` | `Step 7` | none | `modules/core-modules/seam-placer/src/lib.rs` | none | S | Consumption/snap path (AC-6) |
| `TASK-274` | `Step 8` | `docs/03_wit_and_manifest.md`, `docs/DEVIATION_LOG.md`, `docs/adr/` | docs only | none | S | Crosswalk mint + ADR-0046 + D-168 row |

Copy costs from `implementation-plan.md`. Split before activation if any row is L or aggregate exceeds M.

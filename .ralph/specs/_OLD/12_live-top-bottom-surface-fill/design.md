# Design: live-top-bottom-surface-fill

## Controlling Code Paths

- Primary code path: `modules/core-modules/rectilinear-infill/src/lib.rs` — canonical infill generator for the Benchy path.
- Host integration path: `crates/slicer-host/src/dispatch.rs` and `crates/slicer-host/src/layer_executor.rs` — live infill dispatch and `assemble_ordered_entities()`.
- Neighboring tests or fixtures: existing `rectilinear-infill` tests plus new `top_bottom_fill_tdd.rs` and `crates/slicer-host/tests/live_top_bottom_fill_tdd.rs`.
- OrcaSlicer comparison surface: `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` and `Fill/Fill.hpp`.

## Architecture Constraints

- The packet stays on the canonical Benchy-path generator: `rectilinear-infill`.
- Restored fill must use the exact `ExtrusionRole` variants already declared in `slicer-ir`; do not invent new role names.
- Host integration must prove those roles survive into `LayerCollectionIR.ordered_entities`, because packet `11` consumes that surface.

## Code Change Surface

- Selected approach:
  - add focused module tests for top, bottom, bridge, and sparse-only cases
  - repair the live `SliceRegionView` to infill-module handoff if any surface-classification data is missing
  - add one host integration test that asserts final ordered roles after layer execution
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-sdk/src/views.rs` — add `is_top_surface`, `is_bottom_surface`, `is_bridge` fields to `SliceRegionView` so surface classification data from `SurfaceClassificationIR` can reach the infill module
  - `modules/core-modules/rectilinear-infill/tests/top_bottom_fill_tdd.rs`
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/tests/live_top_bottom_fill_tdd.rs`
- Rejected alternatives that were considered and why they were not chosen:
  - restoring all infill generators in one packet: rejected because TASK-120a is about the live Benchy default path, not every infill variant
  - testing only on final GCode text: rejected because packet `11` already owns text emission; this packet must prove role generation before text formatting

## Data and Contract Notes

- IR or manifest contracts touched:
  - `SliceRegionView.infill_areas`
  - `InfillIR.regions[*].solid_infill`
  - `ExtrusionRole::{TopSolidInfill, BottomSolidInfill, BridgeInfill, SparseInfill}`
  - `LayerCollectionIR.ordered_entities[*].role`
- WIT boundary considerations:
  - no world change is expected; the packet stays inside existing layer-world infill inputs and outputs
- Determinism or scheduler constraints:
  - output ordering must remain deterministic so repeated runs preserve the same role sequence

## Locked Assumptions and Invariants

- `rectilinear-infill` is the canonical infill generator for the live Benchy path.
- Packet `11` consumes these roles for final text emission, so this packet must preserve exact `ExtrusionRole` values.

## Risks and Tradeoffs

- Risk: surface classification may already exist but not reach the module. Mitigation: add the host integration test before broad code changes.
- Risk: bridge fill can regress into sparse fill silently. Mitigation: direct `BridgeInfill` assertions.

## Open Questions

- None. The packet stays narrowly on the canonical infill path.
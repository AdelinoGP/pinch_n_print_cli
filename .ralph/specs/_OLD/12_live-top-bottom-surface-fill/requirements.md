# Requirements: live-top-bottom-surface-fill

## Packet Metadata

- Grouped task IDs:
  - `TASK-120a` — restore top/bottom surface fill generation on the live Benchy path
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

The real Benchy path still misses top and bottom surface fill even though the host owns the slice, surface-classification, and infill-stage plumbing required to produce it. The coherent slice here is not generic infill feature work. It is the specific live-path gap between classified top/bottom surfaces and the canonical `rectilinear-infill` generator so that real `TopSolidInfill`, `BottomSolidInfill`, and `BridgeInfill` paths reach `LayerCollectionIR`.

## In Scope

- `SliceRegionView` surface classification fields (`is_top_surface`, `is_bottom_surface`, `is_bridge`) plumbed from `SurfaceClassificationIR` into the module-facing view, so the infill stage receives top/bottom/bridge signal data
- canonical `rectilinear-infill` support for top, bottom, and bridge-sensitive solid fill roles
- live host dispatch of those inputs into the infill stage
- `InfillIR` and `LayerCollectionIR` regression coverage for the emitted roles

## Out of Scope

- Orca-facing GCode labels for those roles
- support generation
- seam placement and travel behavior
- non-default infill generators and experimental fill patterns

## Authoritative Docs

- `docs/01_system_architecture.md`
- `docs/02_ir_schemas.md`
- `docs/04_host_scheduler.md`
- `docs/07_implementation_status.md`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — top/bottom and external-surface preparation behavior
- `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` — surface-role taxonomy
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — classification path into fill generation
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.hpp` — canonical fill generator handoff surface

## Acceptance Summary

### Positive Cases

- Top-surface regions emit `TopSolidInfill` paths.
- Bottom-surface regions emit `BottomSolidInfill` paths.
- Bridge-sensitive fill areas emit `BridgeInfill` paths.
- The host preserves those roles from `InfillIR` into `LayerCollectionIR.ordered_entities`.

### Negative Cases

- Sparse-only regions do not fabricate top, bottom, or bridge roles.

### Measurable Outcomes

- The canonical infill module emits non-empty paths with exact `ExtrusionRole` variants.
- Host integration tests assert the final ordered entity roles, not just the presence of non-zero infill.

### Cross-Packet Impact

- Packet `11` consumes the restored roles for emitted `;TYPE:` labels.
- Packet `21` uses this packet's roles as Benchy evidence for top/bottom fill restoration.

## Verification Commands

- `cargo test -p rectilinear-infill --test top_bottom_fill_tdd top_surface_region_emits_top_solid_infill -- --exact --nocapture`
- `cargo test -p rectilinear-infill --test top_bottom_fill_tdd bottom_surface_region_emits_bottom_solid_infill -- --exact --nocapture`
- `cargo test -p rectilinear-infill --test top_bottom_fill_tdd bridge_surface_region_emits_bridge_infill_role -- --exact --nocapture`
- `cargo test -p rectilinear-infill --test top_bottom_fill_tdd sparse_only_region_does_not_fabricate_surface_fill_roles -- --exact --nocapture`
- `cargo test -p slicer-host --test live_top_bottom_fill_tdd layer_execution_preserves_top_and_bottom_fill_roles -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition: the relevant infill input shape or host dispatch gap is identified
- Postcondition: one exact `ExtrusionRole` reaches the real infill or host assembly surface
- Falsifying check: the narrowest role assertion fails if the path silently downgrades to sparse infill or disappears
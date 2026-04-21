---
status: implemented
packet: live-top-bottom-surface-fill
task_ids:
  - TASK-120a
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: live-top-bottom-surface-fill

## Goal

Restore live top and bottom surface fill generation on the real Benchy path by ensuring top-surface, bottom-surface, and bridge-sensitive fill regions reach the canonical infill generator and survive into `LayerCollectionIR.ordered_entities` with the exact `ExtrusionRole` variants the GCode emit path expects.

## Scope Boundaries

- In scope:
  - top and bottom surface detection reaching the live infill stage on the real host path
  - canonical `rectilinear-infill` generation of `TopSolidInfill`, `BottomSolidInfill`, and `BridgeInfill` paths from surface-classified `SliceRegionView` inputs
  - host integration proving generated fill survives `InfillIR` assembly and `LayerCollectionIR.ordered_entities`
  - deterministic regression coverage for top/bottom/bridge fill roles
- Out of scope:
  - Orca comment or `;TYPE:` emission for those roles (packet `11`)
  - support generation (packet `13`)
  - seam placement or travel policy (packets `14` and `15`)
  - non-default infill generators beyond the canonical Benchy-path `rectilinear-infill` module

## Prerequisites and Blockers

- Depends on:
  - the live infill dispatch path in `crates/slicer-host/src/dispatch.rs`
  - the canonical Benchy-path infill generator `modules/core-modules/rectilinear-infill`
  - `SliceRegionView` carrying surface classification fields (`is_top_surface`, `is_bottom_surface`, `is_bridge`) from `SurfaceClassificationIR` — this is added in Step 0 of this packet before any tests can be authored
- Unblocks:
  - TASK-135 Benchy evidence for top/bottom fill
  - TASK-119 role emission checks for `TopSolidInfill`, `BottomSolidInfill`, and `BridgeInfill`
- Activation blockers:
  - None. The packet is `draft` by default.

## Acceptance Criteria

- **Given** a `SliceRegionView` with non-empty `infill_areas` and top-surface classification reaching the canonical infill generator, **when** `rectilinear-infill` runs on that region, **then** `InfillIR.regions[0].solid_infill` contains at least one path whose `role` is exactly `ExtrusionRole::TopSolidInfill` and whose `points.len()` is greater than `1`. | `cargo test -p rectilinear-infill --test top_bottom_fill_tdd top_surface_region_emits_top_solid_infill -- --exact --nocapture`
- **Given** a `SliceRegionView` with non-empty `infill_areas` and bottom-surface classification, **when** the same live infill path runs, **then** `InfillIR.regions[0].solid_infill` contains at least one path whose `role` is exactly `ExtrusionRole::BottomSolidInfill` and whose `points.len()` is greater than `1`. | `cargo test -p rectilinear-infill --test top_bottom_fill_tdd bottom_surface_region_emits_bottom_solid_infill -- --exact --nocapture`
- **Given** a bridge-sensitive fill area routed to the live infill stage, **when** the infill generator commits paths for that area, **then** at least one committed path uses `ExtrusionRole::BridgeInfill` instead of silently downgrading the region to `SparseInfill`. | `cargo test -p rectilinear-infill --test top_bottom_fill_tdd bridge_surface_region_emits_bridge_infill_role -- --exact --nocapture`
- **Given** a host-driven layer execution with real infill dispatch and a slice fixture that contains one top-surface region and one bottom-surface region, **when** `assemble_ordered_entities()` builds the finalized layer, **then** `LayerCollectionIR.ordered_entities[*].role` contains both `TopSolidInfill` and `BottomSolidInfill` entries in deterministic order. | `cargo test -p slicer-host --test live_top_bottom_fill_tdd commit_layer_outputs_preserves_top_solid_infill_role commit_layer_outputs_preserves_bottom_solid_infill_role -- --exact --nocapture`

## Negative Test Cases

- **Given** a slice region with only sparse infill eligibility and no top-surface, bottom-surface, or bridge classification, **when** the canonical infill generator runs, **then** `solid_infill` is empty and no emitted path uses `TopSolidInfill`, `BottomSolidInfill`, or `BridgeInfill`. | `cargo test -p rectilinear-infill --test top_bottom_fill_tdd sparse_only_region_does_not_fabricate_surface_fill_roles -- --exact --nocapture`

## Verification

- `cargo test -p rectilinear-infill --test top_bottom_fill_tdd top_surface_region_emits_top_solid_infill -- --exact --nocapture`
- `cargo test -p rectilinear-infill --test top_bottom_fill_tdd bottom_surface_region_emits_bottom_solid_infill -- --exact --nocapture`
- `cargo test -p rectilinear-infill --test top_bottom_fill_tdd bridge_surface_region_emits_bridge_infill_role -- --exact --nocapture`
- `cargo test -p rectilinear-infill --test top_bottom_fill_tdd sparse_only_region_does_not_fabricate_surface_fill_roles -- --exact --nocapture`
- `cargo test -p slicer-host --test live_top_bottom_fill_tdd layer_execution_preserves_top_and_bottom_fill_roles -- --exact --nocapture`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — stage ownership and fill/surface semantics
- `docs/02_ir_schemas.md` — `SliceIR`, `InfillIR`, and `ExtrusionRole` contracts
- `docs/04_host_scheduler.md` — live infill-stage execution order and host assembly path
- `docs/07_implementation_status.md` — TASK-120a scope

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp`
- `OrcaSlicerDocumented/src/libslic3r/Surface.hpp`
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp`
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.hpp`

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
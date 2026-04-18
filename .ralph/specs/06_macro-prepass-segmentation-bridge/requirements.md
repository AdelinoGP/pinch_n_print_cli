# Requirements: macro-prepass-segmentation-bridge

## Packet Metadata

- Grouped task IDs:
  - `TASK-128` — parent: resolve prepass segmentation input-shape gaps on the macro/WIT path
  - `TASK-128a` — child: provide usable `MeshSegmentation` inputs on the macro path by sourcing real geometry for `MeshObjectView` instead of object-id-only shells
  - `TASK-128b` — child: provide usable `PaintSegmentation` inputs on the macro path, including transform matrices, paint layers, and participating layer indices
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

DEV-025 tracks that the prepass segmentation stages (`PrePass::MeshSegmentation` and `PrePass::PaintSegmentation`) receive hollow SDK inputs on the macro/WIT path. Concretely:

1. **MeshSegmentation**: Macro-authored modules that implement `PrepassModule::run_mesh_segmentation` receive `&[MeshObjectView]` but the current dispatch wiring (`dispatch_prepass_call`) passes only `object_ids: &[String]` — no geometry is attached. The SDK type `MeshObjectView` is defined in `crates/slicer-sdk/src/prepass_types.rs` with real fields (`vertices`, `triangles`, `paint_layers`), but the host dispatch path never populates them.

2. **PaintSegmentation**: Macro-authored modules that implement `PrepassModule::run_paint_segmentation` receive `&[PaintSegmentationObjectView]` with the same gap — `object_ids` are passed without the corresponding `transform_matrix`, `paint_layers`, or `participating_layer_indices` that the SDK type requires.

The result is that segmentation modules execute with empty geometry and paint data, making the prepass stages no-ops on the macro path even when the host IR contains valid mesh and paint data.

If this packet reopens or supersedes a prior packet: N/A — no prior packet addressed DEV-025 for the macro/WIT path.

## In Scope

- Host dispatch wiring for `PrePass::MeshSegmentation` and `PrePass::PaintSegmentation` to convert host IR (`MeshIR`, `LayerPlanIR`) into SDK-compatible `MeshObjectView` and `PaintSegmentationObjectView` before calling the WASM guest.
- `wit/deps/ir-types.wit` definitions for `mesh-object-view` and `paint-segmentation-object-view` (WIT-side counterparts to the SDK types in `crates/slicer-sdk/src/prepass_types.rs`).
- WIT converter / bindgen glue in `crates/slicer-host/src/wit_host.rs` to convert between host IR and WIT representations.
- TDD harness files:
  - `macro_mesh_segmentation_geometry_tdd` — proves `MeshObjectView` geometry fields are populated
  - `macro_paint_segmentation_input_tdd` — proves `PaintSegmentationObjectView` has transform, paint layers, participating layer indices
  - `macro_paint_region_roundtrip_tdd` — proves `PaintRegionIR` round-trips non-empty `SemanticRegion` data through WIT
  - `macro_mesh_raycast_z_down_tdd` — proves `raycast_z_down` host service works on the macro path

## Out of Scope

- TASK-129 non-segmentation WIT boundary gaps (live path gaps not involving these two prepass stages)
- TASK-147/148 mesh-query host services (`surface_normal_at`, `object_bounds`) — stubs are sufficient here
- Path-optimization behavior
- Postpass GCode boundary coverage
- Module output collection (TASK-130a handles draining `PaintSegmentationOutput` back through WIT)

## Authoritative Docs

- `docs/01_system_architecture.md` — Tier 1 PrePass description, especially `MeshSegmentation` and `PaintSegmentation` stage I/O
- `docs/02_ir_schemas.md` — `MeshIR` struct, `FacetPaintData`, `PaintLayer`, `PaintSemantic`, `LayerPlanIR` `GlobalLayer`, `PaintRegionIR`, `SemanticRegion`
- `docs/03_wit_and_manifest.md` — WIT world definitions, `deps/ir-types.wit`, module manifest schema
- `docs/05_module_sdk.md` — SDK usage, `PrepassModule` trait, `MeshObjectView`, `PaintSegmentationObjectView`
- `docs/07_implementation_status.md` — TASK-128, TASK-128a, TASK-128b, DEV-025

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/` files are not directly referenced by this packet — segmentation is a ModularSlicer-native behavior with no OrcaSlicer counterpart.
- The paint segmentation approach (projecting 3D facet paint onto 2D per-layer polygons) is documented in `docs/01_system_architecture.md` and does not require OrcaSlicer reference borrowing.

## Acceptance Summary

- Positive cases:
  - `MeshObjectView` received by `PrepassModule::run_mesh_segmentation` contains real `vertices` and `triangles` extracted from `MeshIR` for each object.
  - `PaintSegmentationObjectView` received by `PrepassModule::run_paint_segmentation` contains non-empty `transform_matrix`, `paint_layers`, and `participating_layer_indices`.
  - `PaintRegionLayerView::get_regions(&PaintSemantic::Material)` returns non-empty `SemanticRegion` slice with valid polygon data when paint is present.
  - `host.mesh().raycast_z_down(...)` returns `Some(world_z)` with correct Z when called from a macro-authored prepass module on a painted object.
- Negative cases:
  - Empty `vertices` or `triangles` in `MeshObjectView` produces a fatal contract diagnostic at the WIT boundary.
  - Missing transform or empty `participating_layer_indices` in `PaintSegmentationObjectView` produces a diagnostic and does not silently proceed.
- Measurable outcomes:
  - 4 new TDD test files in `crates/slicer-host/tests/` — all must pass
  - 0 new dead `Noop*Runner` stubs introduced by this packet
  - `cargo clippy --workspace -- -D warnings` clean before packet completion
- Cross-packet impact:
  - Unblocks TASK-130a (WIT `push-paint-region` drain path)
  - Unblocks TASK-130b (end-to-end macro-path round-trip tests)
  - Provides input wiring that TASK-130's output path depends on

## Verification Commands

- `cargo test -p slicer-host --test macro_mesh_segmentation_geometry_tdd`
- `cargo test -p slicer-host --test macro_paint_segmentation_input_tdd`
- `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd`
- `cargo test -p slicer-host --test macro_mesh_raycast_z_down_tdd`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

Each step in `implementation-plan.md` must have:

- Precondition: what must be true before the step starts
- Postcondition: what must be true after the step completes
- Falsifying check: a specific command or assertion that proves the step is NOT complete

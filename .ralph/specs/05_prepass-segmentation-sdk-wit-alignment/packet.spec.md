---
status: draft
packet: prepass-segmentation-sdk-wit-alignment
task_ids:
  - TASK-128
  - TASK-128a
  - TASK-128b
  - TASK-130
  - TASK-130a
  - TASK-130b
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: prepass-segmentation-sdk-wit-alignment

## Goal

Resolve prepass segmentation input-shape gaps on the macro/WIT path so segmentation modules stop receiving hollow SDK inputs (DEV-025). Provide usable `MeshSegmentation` inputs on the macro path by sourcing real geometry for `MeshObjectView` instead of object-id-only shells. Provide usable `PaintSegmentation` inputs on the macro path including transform matrices, paint layers, and participating layer indices. Finish the `#[slicer_module]` prepass segmentation bridge for macro-authored modules. Drain `PaintSegmentationOutput` back through WIT `push-paint-region` so macro-authored modules can emit paint regions without hand-written `wit-guest` glue. Add end-to-end macro-path regression tests proving `MeshSegmentation` and `PaintSegmentation` round-trip real data through WIT.

## Scope Boundaries

- In scope:
  - TASK-128a: Provide usable `MeshSegmentation` inputs on the macro path by sourcing real geometry for `MeshObjectView` instead of object-id-only shells. Covers DEV-025.
  - TASK-128b: Provide usable `PaintSegmentation` inputs on the macro path, including transform matrices, paint layers, and participating layer indices. Continues DEV-025.
  - TASK-130: Finish the `#[slicer_module]` prepass segmentation bridge for macro-authored modules. Covers DEV-025.
  - TASK-130a: Drain `PaintSegmentationOutput` back through WIT `push-paint-region` so macro-authored modules can emit paint regions without hand-written `wit-guest` glue. Covers DEV-025.
  - TASK-130b: Add end-to-end macro-path regression tests proving `MeshSegmentation` and `PaintSegmentation` round-trip real data through WIT. Continues DEV-025.

- Out of scope:
  - Z-envelope enforcement (TASK-127 — separate packet)
  - Non-segmentation WIT-boundary gaps (TASK-129 series — separate packet)
  - Manifest population (Workstream 1)
  - Runtime access audit (Workstream 1)

## Acceptance Criteria

- **Given** a `MeshSegmentation` invocation on the macro path, **when** the module calls `MeshObjectView` methods, **then** it receives real geometry (triangle data, vertex positions) not just object-id shells.
- **Given** a `PaintSegmentation` invocation on the macro path, **when** the module calls paint region methods, **then** it receives transform matrices, paint layers, and the correct participating layer indices for each semantic.
- **Given** a macro-authored prepass module using `#[slicer_module]`, **when** it is compiled and loaded, **then** the prepass segmentation bridge compiles and runs without hand-written `wit-guest` glue.
- **Given** a macro-authored module that produces `PaintSegmentationOutput`, **when** it calls `push-paint-region`, **then** the paint regions are correctly drained back through the WIT boundary.
- **Given** an end-to-end macro path run, **when** `MeshSegmentation` and `PaintSegmentation` are exercised, **then** round-trip tests prove real data (not stubs) crosses the WIT boundary.

## Verification

- `cargo test --package slicer-host --test prepass_macro_path_roundtrip -- --nocapture` (test to be added)
- `cargo build --package slicer-host` (verifies bridge compiles)
- Macro-authored module compilation succeeds

## Authoritative Docs

- `docs/01_system_architecture.md` — PrePass pipeline (MeshSegmentation, PaintSegmentation), Stage I/O Contract
- `docs/02_ir_schemas.md` — MeshIR, PaintRegionIR, PaintSemantic
- `docs/03_wit_and_manifest.md` — world-prepass.wit, WIT boundary
- `docs/05_module_sdk.md` — `#[slicer_module]` macro, prepass segmentation bridge

## OrcaSlicer Reference Obligations

- If OrcaSlicer has reference behavior for segmentation (mesh segmentation algorithm, paint region computation), cite `OrcaSlicerDocumented/` paths. Otherwise note none.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`
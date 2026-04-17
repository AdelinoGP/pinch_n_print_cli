# Requirements: prepass-segmentation-sdk-wit-alignment

## Packet Metadata

- Grouped task IDs:
  - `TASK-128` / `TASK-128a` / `TASK-128b` — Resolve prepass segmentation input-shape gaps on macro/WIT path. Covers DEV-025.
  - `TASK-130` / `TASK-130a` / `TASK-130b` — Finish `#[slicer_module]` prepass segmentation bridge for macro-authored modules and add round-trip regression tests. Continues DEV-025.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`

## Problem Statement

DEV-025: Prepass segmentation SDK↔WIT shapes are still misaligned. Segmentation modules (MeshSegmentation, PaintSegmentation) on the macro path receive hollow or incomplete SDK inputs — `MeshObjectView` provides only object IDs instead of real geometry, and `PaintSegmentation` inputs lack transform matrices, paint layers, and participating layer indices. The `#[slicer_module]` prepass bridge is also incomplete.

## In Scope

- TASK-128a: Source real geometry (triangle data, vertex positions) for `MeshObjectView` on the macro path instead of object-id-only shells.
- TASK-128b: Provide transform matrices, paint layers, and participating layer indices as inputs to `PaintSegmentation` on the macro path.
- TASK-130: Complete the `#[slicer_module]` prepass segmentation bridge for macro-authored modules.
- TASK-130a: Drain `PaintSegmentationOutput` back through WIT `push-paint-region` for macro-authored modules.
- TASK-130b: Add end-to-end macro-path regression tests for `MeshSegmentation` and `PaintSegmentation` round-trip real data through WIT.

## Out of Scope

- Z-envelope enforcement (TASK-127)
- Non-segmentation WIT-boundary gaps (TASK-129 series)
- Manifest population (Workstream 1)
- Runtime access audit (Workstream 1)

## Authoritative Docs

- `docs/01_system_architecture.md` — PrePass pipeline (MeshSegmentation, PaintSegmentation), Stage I/O Contract
- `docs/02_ir_schemas.md` — MeshIR, PaintRegionIR, PaintSemantic, PaintLayer, Transform3d
- `docs/03_wit_and_manifest.md` — world-prepass.wit, deps/ir-types.wit
- `docs/05_module_sdk.md` — `#[slicer_module]` macro

## OrcaSlicer Reference Obligations

- Check `OrcaSlicerDocumented/` for segmentation reference behavior. If none exists, note the packet has no OrcaSlicer dependency.

## Acceptance Summary

- `MeshObjectView` on macro path provides real geometry, not just object-id shells.
- `PaintSegmentation` inputs on macro path include transform matrices, paint layers, and participating layer indices.
- `#[slicer_module]` prepass bridge compiles and works for macro-authored modules.
- `PaintSegmentationOutput` correctly drains through WIT `push-paint-region`.
- End-to-end round-trip tests prove real data crosses the WIT boundary.

## Verification Commands

- `cargo build --package slicer-host` (verifies bridge compiles)
- `cargo test --package slicer-host --test prepass_macro_path_roundtrip -- --nocapture`
- Macro-authored prepass module compilation and load test
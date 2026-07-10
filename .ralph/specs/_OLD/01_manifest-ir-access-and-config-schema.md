---
status: implemented
packet: manifest-ir-access-and-config-schema
task_ids:
  - TASK-121
  - TASK-122
---

# 01_manifest-ir-access-and-config-schema

## Goal

Populate `[ir-access]` declarations and `[config.schema]` for all 17 core-module manifests so the Stage I/O Contract enforcement (DEV-002) and `config-schema` CLI (DEV-008) both go live.

## Problem Statement

All 17 core-module TOML manifests currently have empty `[ir-access]` and `[config.schema]` sections. This means:
1. The host cannot enforce the Stage I/O Contract at runtime (DEV-002) because declarations are missing.
2. The `config-schema` CLI returns no usable output (DEV-008) because schemas are absent.

Both gaps block the architecture acceptance gate.

## Architecture Constraints

- Each module's `[ir-access]` is determined solely by its declared stage (`[stage].id`).
- The 17 core modules and their stages:
  - `mesh-segmentation` → `PrePass::MeshSegmentation`
  - `paint-segmentation` → `PrePass::PaintSegmentation`
  - `layer-planner-default` → `PrePass::LayerPlanning`
  - `classic-perimeters` / `arachne-perimeters` → `Layer::Perimeters`
  - `seam-placer` → `Layer::PerimetersPostProcess`
  - `rectilinear-infill` / `gyroid-infill` / `lightning-infill` → `Layer::Infill`
  - `fuzzy-skin` → `Layer::PerimetersPostProcess`
  - `paint-region-annotator` → `Layer::SlicePostProcess`
  - `traditional-support` / `tree-support` → `Layer::Support`
  - `support-surface-ironing` → `Layer::SupportPostProcess`
  - `skirt-brim` / `wipe-tower` → `PostPass::LayerFinalization`
  - `path-optimization-default` → `Layer::PathOptimization`

## Data and Contract Notes

- IR paths must exactly match field names in `crates/slicer-ir/src/` (e.g., `SliceIR.regions.infill_areas`, not `SliceIR.regions.infill-areas`).
- Paint region reads must include semantic-specific paths or `PaintRegionIR.Custom.<module-id>` for custom semantics.
- `layer-parallel-safe = false` must be set on finalization modules; the TOML template already has it but it must be verified.
- Config schema `type` must be one of: `"bool"`, `"int"`, `"float"`, `"string"`, `"enum"`, `"float-list"`, `"string-list"`.

## Risks and Tradeoffs

- Some modules may have config fields in source that are not yet documented in the schema reference. Use the fields that are clearly present; leave unknown ones as `[config.schema]` (empty) until they are confirmed.
- Stage I/O Contract table may not cover every nuance of a module's actual IR usage — use the table as the authoritative baseline, not a substitute for reading the source.

---
status: implemented
packet: 22_live-seam-contract-repair
task_ids:
  - TASK-120c
  - TASK-151
supersedes: 14-rev1_live-seam-placement-and-consumption
---

# 22_live-seam-contract-repair

## Goal

Repair the live seam contract on the current `Layer::PerimetersPostProcess` and `Layer::PathOptimization` path without widening scope into PrePass planning. This packet corrects four concrete defects in the current implementation: `seam-placer` must choose from `PerimeterIR.regions[*].seam_candidates` instead of waiting for a pre-populated `resolved_seam`; `convert_perimeter_output` must apply a chosen seam only to the emitting origin region instead of broadcasting it to every bucket; seam rotation must preserve sibling wall loops in the same region instead of letting `rotated_wall_loops` replace the region with a single rotated wall; and `path_optimization_emit_layer_markers = false` must suppress all marker output.

## Problem Statement

Packet `14-rev1_live-seam-placement-and-consumption` corrected the earlier replay-at-PathOptimization design, but the current code still violates the intended live seam contract in four places:

1. `modules/core-modules/seam-placer/src/lib.rs` reads `region.resolved_seam()` and never chooses from `region.seam_candidates()`, so the module cannot resolve a seam on the live path unless some other surface pre-populates `resolved_seam`
2. `crates/slicer-host/src/wit_host.rs::convert_perimeter_output` applies one `resolved_seam` to every origin bucket, so a seam selected for one `(object_id, region_id)` leaks into sibling regions
3. `crates/slicer-host/src/wit_host.rs::convert_perimeter_output` treats `rotated_wall_loops` as a full replacement of `wall_loops`; if seam-placer emits only the rotated target wall, sibling walls disappear from the committed `PerimeterIR`
4. `modules/core-modules/path-optimization-default/src/lib.rs` parses `path_optimization_emit_layer_markers` but unconditionally emits a marker comment, violating the config contract already locked by `dispatch_tdd`

This packet narrows the fix to the current layer-stage contract. It does not add a new prepass stage or new IR; those deeper architecture changes belong in packet `23`.

## Architecture Constraints

- `Layer::PerimetersPostProcess` is still the owning stage for live seam application in this packet; no new prepass stage is introduced here
- `PerimeterIR.regions[*].seam_candidates` remains the canonical input to seam selection on the live path
- `rotated_wall_loops` continues to replace `wall_loops` in `convert_perimeter_output`; therefore the module must emit a full region-preserving wall-loop set rather than only the target wall
- `Layer::PathOptimization` remains comment-only for this seam slice; travel policy stays with packet `15`

## Data and Contract Notes

- `PerimeterIR.regions[*].seam_candidates[*].position.{x,y,z}` is the only allowed source of the chosen seam point in this packet
- `PerimeterIR.regions[*].resolved_seam.point.{x,y,z}` and `.wall_index` must match the chosen candidate exactly
- `PerimeterIR.regions[*].walls[*].path.points`, `feature_flags`, and `width_profile.widths` must stay parallel after rotation
- `LayerAnnotationKind::Comment` and `LayerAnnotationKind::Raw` must both be absent when `path_optimization_emit_layer_markers = false`

## Risks and Tradeoffs

- Re-emitting the full wall-loop set per region slightly increases module-side work, but it avoids widening the host WIT contract mid-slice
- Failing when a chosen seam point is absent from the target wall loop is stricter than silently preserving geometry; this is intentional to keep the contract falsifiable and prevent silent wrong seams
- The packet leaves PrePass seam planning unresolved on purpose; full Orca parity still requires packet `23`

## Locked Assumptions and Invariants

- `convert_perimeter_output` continues to treat `rotated_wall_loops` as canonical replacement geometry
- origin tags remain the authoritative key for mapping emitted wall loops and seams back to `PerimeterRegion`
- `Layer::PathOptimization` does not reopen move replay in this packet

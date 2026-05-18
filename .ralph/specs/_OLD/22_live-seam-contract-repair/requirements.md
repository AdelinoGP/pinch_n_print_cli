# Requirements: 22_live-seam-contract-repair

## Packet Metadata

- Grouped task IDs:
  - `TASK-120c` — Restore seam placement on real wall-loop seam candidates
  - `TASK-151` — Teach `path-optimization-default` to consume seam-placement output without reopening replay semantics
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented` (2026-04-22 — activation blocker resolved when packet `15_live-travel-retraction-policy` was moved to `draft`)
- Supersedes: `14-rev1_live-seam-placement-and-consumption`

## Problem Statement

Packet `14-rev1_live-seam-placement-and-consumption` corrected the earlier replay-at-PathOptimization design, but the current code still violates the intended live seam contract in four places:

1. `modules/core-modules/seam-placer/src/lib.rs` reads `region.resolved_seam()` and never chooses from `region.seam_candidates()`, so the module cannot resolve a seam on the live path unless some other surface pre-populates `resolved_seam`
2. `crates/slicer-host/src/wit_host.rs::convert_perimeter_output` applies one `resolved_seam` to every origin bucket, so a seam selected for one `(object_id, region_id)` leaks into sibling regions
3. `crates/slicer-host/src/wit_host.rs::convert_perimeter_output` treats `rotated_wall_loops` as a full replacement of `wall_loops`; if seam-placer emits only the rotated target wall, sibling walls disappear from the committed `PerimeterIR`
4. `modules/core-modules/path-optimization-default/src/lib.rs` parses `path_optimization_emit_layer_markers` but unconditionally emits a marker comment, violating the config contract already locked by `dispatch_tdd`

This packet narrows the fix to the current layer-stage contract. It does not add a new prepass stage or new IR; those deeper architecture changes belong in packet `23`.

## In Scope

- Select the live seam from `PerimeterIR.regions[*].seam_candidates` inside `seam-placer`
- Emit a full region-preserving wall-loop set so only the target wall rotates and sibling walls survive
- Scope committed `resolved_seam` to the emitting origin region in `convert_perimeter_output`
- Honor `path_optimization_emit_layer_markers = false` exactly
- Add deterministic host regressions for candidate selection, origin scoping, sibling preservation, and marker suppression

## Out of Scope

- Introducing `PrePass::SeamPlanning` or any new prepass blackboard slot
- Extending the WIT world with new prepass builders or layer-view handles
- Travel policy, retract/unretract, z-hop, tool ordering, or cooling decisions
- Broad Benchy acceptance closure beyond seam-specific evidence

## Authoritative Docs

- `docs/01_system_architecture.md` — seam claim ownership and per-layer execution responsibilities
- `docs/02_ir_schemas.md` — canonical `PerimeterIR` and wall-loop field names
- `docs/03_wit_and_manifest.md` — `perimeter-output-builder` contract and manifest access rules
- `docs/04_host_scheduler.md` — stage routing and commit ownership
- `docs/07_implementation_status.md` — backlog ownership for `TASK-120c` and `TASK-151`

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — seam candidate scoring and chosen-seam semantics
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — interface-level seam contract
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — provenance of perimeter seam candidates

## Acceptance Summary

**Measured success path:**

- `PerimeterIR.regions[0].resolved_seam.point.{x,y,z}` and `.wall_index` are chosen directly from `PerimeterIR.regions[0].seam_candidates[*]`
- `PerimeterIR.regions[0].walls.len()` is unchanged by seam rotation, and only the targeted `walls[wall_index].path.points[0]` moves to the chosen seam vertex
- Origin-bucketed conversion applies `resolved_seam` only to the region that emitted it
- `path_optimization_emit_layer_markers = false` produces zero deferred annotations on the live host path
- Repeated identical inputs produce byte-identical seam output

**Explicit negative cases:**

- a seam point missing from the target wall-loop vertices is a fatal contract error, not a silent preserve-or-broadcast fallback
- `CARDINALITY_MISMATCH` from `push-reordered-wall-loop` still aborts the commit path and leaves the arena empty

## Cross-Packet Dependencies and Unblockers

- Absorbs the unfinished seam assumptions from `14-rev1_live-seam-placement-and-consumption`
- Provides the corrected layer-stage baseline that packet `23_prepass-seam-planning-orca-parity` will later consume
- Stays intentionally separate from packet `15_live-travel-retraction-policy`, which remains the owner of travel-policy decisions and is currently `active`

## Verification Commands

- `cargo test -p slicer-host --test live_seam_path_tdd seam_placer_selects_lowest_effective_score_candidate -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd seam_rotation_preserves_non_target_walls -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd resolved_seam_is_applied_only_to_origin_region -- --exact --nocapture`
- `cargo test -p slicer-host --test dispatch_tdd path_optimization_emit_layer_markers_false_suppresses_output -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd seam_contract_is_deterministic_across_repeated_dispatch -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd seam_candidate_missing_from_target_wall_rejects_dispatch -- --exact --nocapture`
- `cargo test -p slicer-host --test live_seam_path_tdd rotated_points_cardinality_mismatch_rejected -- --exact --nocapture`

## Step Completion Expectations

Each implementation step must name:

- the failing test or explicit falsifying check that proves the current bug still exists
- the exact function or manifest surface being changed
- the postcondition in terms of committed `PerimeterIR` fields or emitted annotations

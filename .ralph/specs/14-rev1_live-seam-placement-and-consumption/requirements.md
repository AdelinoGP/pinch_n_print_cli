# Requirements: 14-rev1_live-seam-placement-and-consumption

## Packet Metadata

- Grouped task IDs:
  - `TASK-120c` — Restore seam placement on real wall-loop seam candidates
  - `TASK-151` — Teach `path-optimization-default` to consume seam-placement output and stop acting as a comment-only slot filler on real wall loops
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Supersedes: `14_live-seam-placement-and-consumption` (original, status: implemented) — that packet implemented Option A (replay at PathOptimization via `push-move`) which violates the `Layer::PathOptimization` WIT contract and uses incorrect E coordinate semantics (local width instead of cumulative extrusion)

## Problem Statement

The original packet `14` attempted to implement seam-started wall loop replay via `push-move` at `Layer::PathOptimization`. This approach has three critical failures:

1. **`push-move` is rejected at `Layer::PathOptimization`** per docs/03 § Path Optimization Output Contract — the host was silently accepting Move commands instead of rejecting them with a fatal diagnostic
2. **E coordinate uses local extrusion width** — `cmd.e` was set to `pt.width` (per-vertex width in mm), not cumulative extrusion amount, producing semantically incorrect GCode
3. **Moves committed as Raw G1 annotations** — not as proper `ordered_entities` in `LayerCollectionIR`

The documentation in `docs/02_ir_schemas.md` is ambiguous on whether `WallLoop.path.points[0]` must be the seam vertex. The current `SeamPosition { point, wall_index }` description in the schema reads as a reference annotation, not a geometry directive.

This packet resolves the ambiguity: **`resolved_seam` is a directive** — the wall loop must be stored with `path.points[0]` as the seam vertex. Printing begins at the seam and ends at the seam.

## In Scope

- Documentation clarification: update `docs/02_ir_schemas.md` to explicitly state that `WallLoop.path.points[0]` is the seam-first vertex after `Layer::PerimetersPostProcess` completes
- New WIT method `push-reordered-wall-loop(pos: point3-with-width, wall-index: u32, rotated-wall-loop: wall-loop-view)` on `perimeter-output-builder` at `Layer::WallPostProcess`
- `seam-placer` implementation: rotate wall loop so seam is first, emit via `push-reordered-wall-loop`
- WIT boundary wiring: `perimeter_region_to_data` maps rotated geometry through the WIT boundary
- `path-optimization-default` reverts to comment-only output (marker emission only — no Move replay)
- `resolved_seam` WIT field on `perimeter-region-view` remains for diagnostic reads only (no longer drives replay logic)
- Deterministic regression tests for seam-rotated wall loop geometry

## Out of Scope

- Generic travel ordering, retract/no-retract policy, Z-hop planning (packet `15`)
- PathOptimization entity reordering beyond seam-first loop storage (packet `18`)
- Orca-facing GCode text emission for seam-started loops (packet `11`)
- Changes to `Layer::PathOptimization` output contract (no new WIT methods needed there)
- Changes to `PerimeterIR.seam_candidates` generation (candidates come from perimeter generators upstream)

## Authoritative Docs

- `docs/01_system_architecture.md` — Stage I/O contract, seam placement definition
- `docs/02_ir_schemas.md` — `PerimeterIR`, `WallLoop`, `PerimeterRegion`, `ExtrusionPath3D`, `SeamPosition`
- `docs/03_wit_and_manifest.md` — `perimeter-output-builder` WIT resource, IR access path format
- `docs/04_host_scheduler.md` — `Layer::WallPostProcess` execution order, commit path

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — wall loop rotation logic and seam selection algorithm; `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — interface and constants
- The OrcaSlicer `SeamPlacer` rotates wall loop vertices so the seam is the first point in the emitted sequence; this packet mirrors that behavior at the WIT boundary

## Acceptance Summary

**Positive cases:**
- Seam-rotated wall loops are committed to `PerimeterIR` with `path.points[0]` exactly at the `resolved_seam.point` position
- Rotated wall loops have `feature_flags` and `width_profile` cardinality matching rotated `path.points`
- OOB `seam_idx` preserves original wall loop order (non-fatal, no error)
- Determinism: repeated identical inputs produce byte-identical `PerimeterIR`
- `path-optimization-default` emits only the marker comment when PerimeterIR is already seam-first

**Negative cases:**
- Rotated points cardinality mismatch (points vs widths) → commit rejected
- Seam Z outside layer envelope → `push-reordered-wall-loop` returns error
- No `resolved_seam` → original wall loop order preserved

**Cross-packet impact:**
- Packet `14` (original): superseded by this revision
- Packet `15` (travel retraction policy): now unblocked — `PerimeterIR` contains seam-started wall geometry, so travel policy can assume seam-first perimeters without needing to replay
- Packet `11` (orca-gcode-emission-contract): `DefaultGCodeEmitter` will need to verify that seam-first wall loops emit correctly through the GCode emit path

## Verification Commands

- `cargo test -p slicer-host --test live_seam_path_tdd -- --nocapture` — all live seam path integration tests
- `cargo test -p path-optimization-default --test seam_consumption_tdd -- --nocapture` — seam consumption module tests
- `cargo build --workspace` — no compilation errors
- `cargo clippy --workspace -- -D warnings` — no lints

## Step Completion Expectations

Each step in `implementation-plan.md` must produce:
- Precondition: what must be true before the step starts
- Postcondition: what the step produces or changes
- Falsifying check: the cheapest test that proves the step succeeded
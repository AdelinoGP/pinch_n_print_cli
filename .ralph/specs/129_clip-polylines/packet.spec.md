---
status: draft
packet: 129_clip-polylines
task_ids:
  - TASK-254
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: 129_clip-polylines

## Goal

Add `clip_polylines` — a generic Clipper2 open-path intersection of polylines against an
`ExPolygon` set — to `crates/slicer-core/src/polygon_ops.rs`, using `clipper2-rust 1.0.3`'s
native `Clipper64::add_open_subject` + `execute(…, Some(&mut solution_open))` API.

## Scope Boundaries

One new public function in `slicer-core::polygon_ops` plus its TDD suite in the existing
`polygon_ops_tdd.rs`, and a one-line SDK-helper-surface doc mention. No consumer is wired in
this packet — the infill-linker (packet 133) is the first caller; gyroid's broken per-vertex
clipping is deleted separately (packet 135). No WIT, manifest, or host change.

## Prerequisites and Blockers

- Depends on: nothing (first packet of the infill-parity roadmap).
- Unblocks: `133_infill-linker-module` (sole consumer of `clip_polylines`).
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** a single polyline strictly inside a square `ExPolygon`, **when**
  `clip_polylines` runs, **then** it returns exactly 1 polyline whose points equal the input
  (order preserved, no splitting). | `cargo test -p slicer-core --test polygon_ops_tdd -- clip_polylines_line_fully_inside_returned_whole 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** a polyline crossing one boundary edge of the clip polygon exactly once,
  **when** `clip_polylines` runs, **then** it returns exactly 1 polyline covering only the
  inside portion, with the crossing endpoint on the boundary (±2 units). | `cargo test -p slicer-core --test polygon_ops_tdd -- clip_polylines_line_crossing_once_split 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** a polyline that enters, exits, and re-enters the clip polygon
  (enter-exit-enter), **when** `clip_polylines` runs, **then** it returns exactly 2 disjoint
  inside sub-polylines. | `cargo test -p slicer-core --test polygon_ops_tdd -- clip_polylines_line_crossing_twice_two_segments 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** an `ExPolygon` with a hole and a polyline passing straight through the hole,
  **when** `clip_polylines` runs, **then** it returns exactly 2 sub-polylines (one per side of
  the hole) and no returned point lies strictly inside the hole. | `cargo test -p slicer-core --test polygon_ops_tdd -- clip_polylines_line_through_hole_split_around_hole 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-5. Given** a polyline collinear with a contour edge of the clip polygon, **when**
  `clip_polylines` runs, **then** the edge-coincident span is returned (Clipper2 boundary
  rule: on-edge counts as inside). | `cargo test -p slicer-core --test polygon_ops_tdd -- clip_polylines_line_along_edge_inside 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-6. Given** 3 input polylines (one inside, one outside, one crossing) against one clip
  polygon, **when** `clip_polylines` runs, **then** it returns exactly 2 polylines (the inside
  one whole + the crossing one's inside part). | `cargo test -p slicer-core --test polygon_ops_tdd -- clip_polylines_multi_polyline_clip 2>&1 | tee target/test-output.log | grep "^test result"`

## Negative Test Cases

- **AC-N1. Given** a polyline entirely outside the clip polygon, **when** `clip_polylines`
  runs, **then** it returns an empty `Vec` (the polyline is dropped, not passed through). | `cargo test -p slicer-core --test polygon_ops_tdd -- clip_polylines_line_fully_outside_dropped 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-N2. Given** an empty polyline slice, an empty clip slice, or both, **when**
  `clip_polylines` runs, **then** it returns an empty `Vec` without panicking. | `cargo test -p slicer-core --test polygon_ops_tdd -- clip_polylines_empty_input_returns_empty 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo test -p slicer-core --test polygon_ops_tdd 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo clippy -p slicer-core --all-targets -- -D warnings`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/specs/infill-parity-rectilinear-gyroid-linker.md` — Phase 0 (load §Phase 0 only,
  lines 124-176; open-question 2 resolution near end of file).
- `docs/08_coordinate_system.md` — delegate a SUMMARY if unit questions arise (383 lines).
- `docs/05_module_sdk.md` — load line 63 region only (pure-geometry primitives list in
  §Guest Build Invariants; Doc Impact target).

## Doc Impact Statement (Required)

- `docs/05_module_sdk.md` §Guest Build Invariants — add `clip_polylines` to the pure-geometry
  primitives list that names `polygon_ops` (~line 63) — `rg -q 'clip_polylines' docs/05_module_sdk.md`

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

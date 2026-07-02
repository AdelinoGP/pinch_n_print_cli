---
status: draft
packet: 134_rectilinear-raw-emit
task_ids:
  - TASK-259
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 134_rectilinear-raw-emit

## Goal

Rewrite `modules/core-modules/rectilinear-infill/src/lib.rs` to OrcaSlicer scan-line
correctness under raw emit: `infill_direction` angle resolution, float-rotation, per-ExPolygon
scan conversion with the half-open vertex test, `adjust_solid_spacing` for solid roles,
`pattern_shift` for layer interleave — emitting raw 2-point segments only (linking, overlap,
and filtering are the linker's, ADR-0025).

## Scope Boundaries

One module's algorithm rewrite plus its TDD suite. The current stub already emits 2-point
segments — this packet fixes the geometry that is wrong (global edge merge across expolygons,
missing vertex-test discipline, missing solid-spacing adjustment, missing bridge-angle
priority) and keeps the four-role emission structure, `solid_fill_role` mapping, `should_emit`
gating, and manifest untouched. No linking-related code is added (deleted concepts stay
deleted per the spec's "NOT added" list).

## Prerequisites and Blockers

- Depends on: `133_infill-linker-module` (raw output is linked the day this lands; roadmap
  D1), `131_per-region-config-delivery` (per-region density is readable — the module reads
  config through the region accessor inside its region loop).
- Unblocks: `136_infill-parity-integration`.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** a 10 mm square wall-inset polygon at `infill_density = 0.2` and default
  line width, **when** the module runs for the sparse role, **then** it emits exactly
  `floor(bb_height / line_spacing) + 1` segments (line_spacing = spacing/density), each with
  exactly 2 points, both endpoints on the polygon boundary (±2 units), and no two segments
  share an endpoint (no linking). | `cargo test -p rectilinear-infill -- square_10mm_density_20_emits_n_raw_segments 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-2. Given** an ExPolygon with a central hole, **when** scan lines cross the hole,
  **then** each such scan line yields exactly 2 segments (one per side), and no emitted point
  lies strictly inside the hole. | `cargo test -p rectilinear-infill -- polygon_with_hole_segments_split_around_hole 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-3. Given** two disjoint ExPolygons in one region role, **when** the module runs,
  **then** every segment lies entirely within one ExPolygon (per-ExPolygon scan conversion —
  no cross-polygon pairing, the current stub's global-edge-merge bug). | `cargo test -p rectilinear-infill -- two_disjoint_expolygons_independent_scan_conversion 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-4. Given** the same polygon at `infill_angle = 45°` and `0°`, **when** the 45° output
  is rotated by −45° about the same reference point, **then** it matches the 0° output
  geometry within 2 units per endpoint. | `cargo test -p rectilinear-infill -- angle_45_rotated_output_matches_unrotated_after_inverse 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-5. Given** a solid role polygon whose width is not an integer multiple of the line
  width, **when** the module runs, **then** the spacing is adjusted per
  `adjust_solid_spacing` (FillBase.cpp:326-340) so the emitted line count fills the width
  exactly (first/last lines at the boundary, uniform adjusted spacing). | `cargo test -p rectilinear-infill -- solid_spacing_adjusted_for_solid_role 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-6. Given** a bridge-role polygon with `bridge_angle` set and per-layer rotation
  active, **when** the module runs, **then** the bridge segments follow the bridge angle
  (priority: bridge_angle > per-layer rotation > static base angle, FillBase.cpp:352-391). | `cargo test -p rectilinear-infill -- bridge_angle_overrides_layer_rotation 2>&1 | tee target/test-output.log | grep "^test result"`
- **AC-7. Given** two consecutive layers with `pattern_shift` semantics
  (FillRectilinear.cpp:3023-3024, applied to the scan-line start x), **when** both layers
  run, **then** layer N+1's scan lines are offset from layer N's by the shift (segments
  interleave, not stack). | `cargo test -p rectilinear-infill -- pattern_shift_interleaves_layers 2>&1 | tee target/test-output.log | grep "^test result"`

## Negative Test Cases

- **AC-N1. Given** a scan line passing exactly through a polygon vertex, **when**
  intersections are classified with the half-open edge test (edge included at `min_y`,
  excluded at `max_y`), **then** the intersection count at that x is exact — no
  double-count, no missing pair (segment count matches the analytic expectation). | `cargo test -p rectilinear-infill -- half_open_vertex_test_no_double_count 2>&1 | tee target/test-output.log | grep "^test result"`

## Verification

- `cargo test -p rectilinear-infill 2>&1 | tee target/test-output.log | grep "^test result"`
- `cargo clippy -p rectilinear-infill --all-targets -- -D warnings`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/specs/infill-parity-rectilinear-gyroid-linker.md` §Phase 2 — the algorithm contract
  (load Phase 2 only).
- `docs/adr/0025-…` — raw-emit boundary (what must NOT be added); delegate SUMMARY.
- `docs/08_coordinate_system.md` — delegate SUMMARY; rotation rounding note (≤ 50 nm is
  below the 100 nm floor).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp:2979-3143` — `fill_surface_by_lines` (scan-line driver; port up to, and excluding, the link-graph stages).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp:842-1154` — `slice_region_by_vertical_lines` (edge-intersection discipline; single-level, no ExPolygonWithOffset).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillRectilinear.cpp:3023-3024` — `pattern_shift` application to the reference x.
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp:352-391` — `infill_direction` (angle priority + π/2 + reference point).
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillBase.cpp:326-340` — `adjust_solid_spacing`.

## Doc Impact Statement (Required)

**`none`** — module-internal algorithm rewrite: no IR field, WIT type, scheduler rule, claim,
manifest schema, host service, or SDK contract changes; the architectural behavior (raw emit)
is already documented by ADR-0025 and the infill-parity spec.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

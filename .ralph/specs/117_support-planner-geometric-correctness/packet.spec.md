---
status: draft
packet: 117
task_ids:
  - TASK-254
  - TASK-255
backlog_source: docs/07_implementation_status.md
context_cost_estimate: S
---

# Packet Contract: support-planner-geometric-correctness

## Goal

Replace `support-planner::tapered_radius` with a two-piece formula that produces the 45° tip cone OrcaSlicer's `calc_branch_radius` defines (eliminating the current `clamp(_, branch_radius, MAX)` floor that suppresses any tip), and delete the DIY `inflate_polygon` vertex-offset routine, replacing it with calls to the existing Clipper-backed `slicer_core::polygon_ops::offset` helper at the avoidance-inflation call site.

## Scope Boundaries

Touches one source file (`modules/core-modules/support-planner/src/lib.rs`) and adds two test files for the new behaviors. No IR change, no WIT change, no manifest change. Both fixes are local correctness changes with self-evident unit-test oracles — tip cone is checkable by computing `tapered_radius` at the cone boundary, polygon offset correctness is checkable by feeding an L-shaped polygon and asserting no self-intersection.

## Prerequisites and Blockers

- Depends on: packet `116_support-modules-doc-honesty-cleanup` recommended to land first (shares review surface in `support-planner/src/lib.rs`) but not strictly required — the two packets edit disjoint regions (doc-honesty edits the `//!` block + struct + parse; this packet edits `tapered_radius` body and the `inflate_polygon` site).
- Unblocks: Block C algorithm packets (`121_support-planner-smooth-nodes`, `122_support-planner-multi-neighbour-mst`) — those depend on the validation harness in packet 4, but `tapered_radius` and avoidance-inflation correctness are common preconditions for them too.
- Activation blockers: none.

## Acceptance Criteria

- **AC-1. Given** `support_planner::tapered_radius(branch_radius=2.5, tan_diameter_angle=tan(5°), dist_to_top=0, effective_layer_height=0.2)`, **when** invoked, **then** the return value is `0.0` (within `1e-6`). | `cargo test -p support-planner --test tapered_radius_tip_cone -- tapered_radius_at_tip_is_zero --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** `support_planner::tapered_radius(branch_radius=2.5, tan_diameter_angle=tan(5°), dist_to_top=12, effective_layer_height=0.2)` (so `mm_to_top = 2.4 < branch_radius = 2.5`, still inside the cone), **when** invoked, **then** the return value equals `2.4` (within `1e-6`) — the 45° cone formula `radius = mm_to_top`. | `cargo test -p support-planner --test tapered_radius_tip_cone -- tapered_radius_inside_cone_is_mm_to_top --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** `support_planner::tapered_radius(branch_radius=2.5, tan_diameter_angle=tan(5°), dist_to_top=50, effective_layer_height=0.2)` (so `mm_to_top = 10.0 > branch_radius`), **when** invoked, **then** the return value equals `2.5 + (10.0 - 2.5) * tan(5°)` (within `1e-6`) — the linear-above-cone formula. | `cargo test -p support-planner --test tapered_radius_tip_cone -- tapered_radius_above_cone_is_linear --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** `support_planner::tapered_radius(branch_radius=2.5, tan_diameter_angle=tan(80°), dist_to_top=10_000, effective_layer_height=0.5)` (an unbounded linear ramp), **when** invoked, **then** the return value equals exactly `MAX_BRANCH_RADIUS_MM = 6.0` (within `1e-6`) — the upper clamp still fires. | `cargo test -p support-planner --test tapered_radius_tip_cone -- tapered_radius_clamps_at_max --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** `modules/core-modules/support-planner/src/lib.rs`, **when** searched for `fn inflate_polygon` and for any private call site of `inflate_polygon(`, **then** no match exists; the prior call site in `run_support_geometry` (formerly around line 226 of the spec's audit) now calls `slicer_core::polygon_ops::offset(...)`. | `! rg -q 'fn inflate_polygon' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'inflate_polygon\(' modules/core-modules/support-planner/src/lib.rs && rg -q 'slicer_core::polygon_ops::offset' modules/core-modules/support-planner/src/lib.rs`
- **AC-6. Given** an L-shaped concave `ExPolygon` (5×5 mm outer arm + 5×5 mm orthogonal arm), **when** passed through the new offset call inside `support-planner` with a `delta_mm = 0.5` inflation, **then** the result has no self-intersections at the concave corner and Clipper2's validity check passes. | `cargo test -p support-planner --test avoidance_offset_concave -- offset_concave_l_shape_no_self_intersection --nocapture 2>&1 | tee target/test-output.log`
- **AC-7. Given** an `ExPolygon` with a single hole (10×10 mm contour, 4×4 mm hole), **when** passed through the new offset call with `delta_mm = 0.5`, **then** the result preserves the hole as a proportionally-eroded interior contour (hole present, hole area shrunk by approximately `Δ-area = π·delta² ± offset perimeter effects`). | `cargo test -p support-planner --test avoidance_offset_concave -- offset_polygon_with_hole_preserves_hole --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** `support_planner::tapered_radius(branch_radius=2.5, tan_diameter_angle=tan(5°), dist_to_top=10, effective_layer_height=0.2)` (`mm_to_top = 2.0`, inside the cone), **when** invoked, **then** the return value is NOT `2.5` (the previous broken behavior of returning the floor `branch_radius` while inside the cone) — it is `2.0`. | `cargo test -p support-planner --test tapered_radius_tip_cone -- tapered_radius_no_longer_floors_at_branch_radius --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check -p support-planner --all-targets`
- `cargo clippy -p support-planner --all-targets -- -D warnings`
- `cargo test -p support-planner 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §B5 (tip cone formula), §B6 (inflate replacement), §D2 (Bucket B cutline).
- `docs/08_coordinate_system.md` — read directly (≈30 lines); confirms `1 unit = 100 nm` so the implementer applies the right factor when passing `delta_mm` to `polygon_ops::offset`.
- `crates/slicer-core/src/polygon_ops.rs` — read the `pub fn offset(...)` definition at line 205 (±20 lines) only; do not browse the whole file.

## Doc Impact Statement (Required)

`none` — this packet replaces in-place algorithm internals. The public function signatures (`tapered_radius`, the planner's call sites) are unchanged. The IR shape is unchanged. The user-facing config schema is unchanged. The acceptance evidence is the new unit tests (`tapered_radius_tip_cone`, `avoidance_offset_concave`) which document the new behavior in code.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::calc_branch_radius` (second overload, signature `(coordf_t base_radius, coordf_t mm_to_top, double diameter_angle_scale_factor, bool use_min_distance)`). The 45° tip cone (`radius = mm_to_top` while `mm_to_top <= base_radius`, then `base_radius + (mm_to_top - base_radius) * diameter_angle_scale_factor` above) is the exact formula this packet ports.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

---
status: implemented
packet: 117
task_ids: [281, 282]
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-planner-geometric-correctness

## Goal

Correct `support_planner::tapered_radius`'s tip geometry and route support-outline avoidance through the existing guest-compatible `slicer_sdk::host::offset_polygons` API with preserved `ExPolygon` holes and explicit mm/scaled-unit boundaries.

## Scope Boundaries

This packet changes `tapered_radius`, the support planner's avoidance-cache representation and SDK host-geometry call, focused unit tests, and the existing radius-parity test that asserts the obsolete floor behavior. It does not add a direct `slicer-core` dependency to the guest, change IR/WIT schemas, planner connectivity, Block C algorithms, or unrelated geometry helpers. The packet owns TASK-281 (B5 — `tapered_radius` two-piece tip-cone formula) and TASK-282 (B6 — `run_support_geometry` offset replacement via `slicer_sdk::host::offset_polygons` with `ExPolygon` hole retention). Both rows were added to `docs/07_implementation_status.md` and closed 2026-07-19 alongside this packet.

## Prerequisites and Blockers

- Depends on: packet `116_support-modules-doc-honesty-cleanup` is queued first because both touch `support-planner/src/lib.rs`; the geometric work has no semantic dependency on its comments or warning.
- Unblocks: packet `119_support-validation-wedge-harness` and the Block C planner packets that consume corrected widths and avoidance geometry.
- Activation blockers: `[BLOCK]` source-plan `TASK-254` and `TASK-255` collide with unrelated current backlog entries; no canonical support rows own B5 and B6.

## Acceptance Criteria

- **AC-1. Given** `tapered_radius(2.5, tan(5°), 0, 0.2)`, **when** it is called, **then** it returns `0.0` within `1e-6`. | `cargo test -p support-planner --all-targets -- tapered_radius_at_tip_is_zero --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** `tapered_radius(2.5, tan(5°), 12, 0.2)`, **when** it is called, **then** it returns `2.4` within `1e-6` because `mm_to_top = 2.4` remains inside the 45-degree tip cone. | `cargo test -p support-planner --all-targets -- tapered_radius_inside_cone_is_mm_to_top --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** `tapered_radius(2.5, tan(5°), 50, 0.2)`, **when** it is called, **then** it returns `2.5 + (10.0 - 2.5) * tan(5°)` within `1e-6`. | `cargo test -p support-planner --all-targets -- tapered_radius_above_cone_is_linear --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** `tapered_radius(2.5, tan(80°), 10_000, 0.5)`, **when** it is called, **then** it returns exactly `MAX_BRANCH_RADIUS_MM = 6.0` within `1e-6`. | `cargo test -p support-planner --all-targets -- tapered_radius_clamps_at_max --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** `support-planner/src/lib.rs`, **when** it is searched, **then** no `inflate_polygon` definition or call remains, the planner calls the existing `slicer_sdk::host::offset_polygons` API (via the `slicer_sdk::prelude` re-export of `host`, so the in-scope form is `host::offset_polygons` at the call site), the replacement uses `OffsetJoinType::Miter`, and the support-planner manifest does not add `slicer-core`. | `! rg -q 'fn inflate_polygon|inflate_polygon\(' modules/core-modules/support-planner/src/lib.rs && ( rg -q 'slicer_sdk::host::offset_polygons' modules/core-modules/support-planner/src/lib.rs || rg -q 'host::offset_polygons' modules/core-modules/support-planner/src/lib.rs ) && rg -q 'OffsetJoinType::Miter' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'slicer-core' modules/core-modules/support-planner/Cargo.toml`
- **AC-6. Given** a concave L-shaped `ExPolygon` built with `Point2::from_mm`, **when** the support planner's SDK offset operation inflates it by `0.5` mm, **then** the returned outer contour passes the test-local edge-intersection invariant and contains no self-intersection at the concave corner. | `cargo test -p support-planner --all-targets -- offset_concave_l_shape_no_self_intersection --nocapture 2>&1 | tee target/test-output.log`
- **AC-7. Given** an `ExPolygon` with a single 10 mm square contour and a 4 mm square hole, **when** the support planner's offset operation inflates it by `0.5` mm, **then** one hole remains and its area is smaller than the original hole area. | `cargo test -p support-planner --all-targets -- offset_polygon_with_hole_preserves_hole --nocapture 2>&1 | tee target/test-output.log`
- **AC-8. Given** the existing `radius_tapers_with_distance_to_top` test, **when** it runs after the tip-cone change, **then** its top-radius assertion expects `0.0` rather than the obsolete `branch_radius` floor and the test passes. | `cargo test -p support-planner --test orca_parity_tdd --all-targets -- radius_tapers_with_distance_to_top --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** `tapered_radius(2.5, tan(5°), 10, 0.2)`, **when** it is called, **then** it returns `2.0` within `1e-6` and is not the old floor value `2.5`. | `cargo test -p support-planner --all-targets -- tapered_radius_no_longer_floors_at_branch_radius --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a 1 mm square represented by scaled `Point2::from_mm` coordinates, **when** the support offset uses `delta_mm = 0.5`, **then** the resulting outer span is approximately 2.0 mm after `units_to_mm`, proving raw scaled integers were not treated as millimeters. | `cargo test -p support-planner --all-targets -- offset_preserves_mm_coordinate_boundary --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check -p support-planner --all-targets`
- `cargo clippy -p support-planner --all-targets -- -D warnings`
- `cargo test -p support-planner --all-targets 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` - direct read of §B5, §B6, and §D2; source of the two-piece radius formula and offset replacement boundary.
- `docs/08_coordinate_system.md` - direct read of the coordinate rule and Clipper2 integration sections; source of 10,000 units/mm and mm-valued float conventions.
- `docs/05_module_sdk.md` - bounded read of the guest dependency rules and existing SDK host-geometry seam.
- `docs/adr/0023-arachne-port-strategy.md` - current host-side crate strategy; no silent ADR amendment is permitted.
- `crates/slicer-sdk/src/host.rs` - bounded read of `offset_polygons` and `OffsetJoinType`; current guest-facing wrapper and hole-preserving return shape.
- `crates/slicer-schema/wit/deps/common.wit` - existing `offset-polygons` host-service contract; no new WIT change is proposed.

## Doc Impact Statement (Required)

**`none`** - the public IR/WIT shape and user-facing schema do not change; tests and function documentation describe the corrected local behavior.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegated `TreeSupport::calc_branch_radius` second overload; confirm the 45-degree tip-cone branch and linear-above-cone branch being asserted by AC-1 through AC-4.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

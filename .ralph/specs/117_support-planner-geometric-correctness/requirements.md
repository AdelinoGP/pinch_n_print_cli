# Requirements: support-planner-geometric-correctness

## Packet Metadata

- Grouped source-plan work items: B5 and B6; no current `docs/07_implementation_status.md` task IDs are mapped.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`support_planner::tapered_radius` currently clamps its expansion to `[branch_radius, MAX_BRANCH_RADIUS_MM]`, so `dist_to_top = 0` returns the branch radius instead of a point tip. The source plan's B5 formula is still an open correctness correction even though the older broad `TASK-163 (algorithmic)` row is closed; that row cannot be treated as proof that this exact fix landed because the current function still has the floor.

The current B6 packet was also stale about the guest boundary. `slicer-core` is not a dependency of `support-planner`, and ADR-0023/guest-build rules forbid adding the host-only crate directly to this guest graph. The existing `slicer_sdk::host::offset_polygons` API is the guest-compatible geometry seam; it accepts `&[ExPolygon]`, a millimeter `f32` delta, and `OffsetJoinType`, and returns `Vec<ExPolygon>` with hole nesting preserved. The planner currently flattens scaled integer points into raw `f32` values, calls a deleted-in-scope DIY `inflate_polygon`, and stores avoidance contours in a hole-less shape. The correction must use the SDK seam and keep the mm/scaled-unit boundary explicit.

B5 and B6 are one coherent local geometry slice: both affect the support planner's local geometric oracles, share one source file and the existing SDK geometry seam, and gate the downstream validation harness. Their source-plan task IDs are not canonical current backlog ownership, so activation is blocked rather than guessed.

## In Scope

- Replace `tapered_radius` with `mm_to_top = dist_to_top * effective_layer_height`, a `mm_to_top.max(0.0)` tip-cone branch through `branch_radius`, a linear-above-cone branch using `tan_diameter_angle`, and a final clamp to `[0.0, MAX_BRANCH_RADIUS_MM]`.
- Update `tapered_radius`'s function documentation to describe the two-piece formula without claiming a broader unported overload or interface-aware widening.
- Use the existing `slicer_sdk::host::offset_polygons` API and `OffsetJoinType::Miter`; do not add a direct `slicer-core` dependency to the guest module.
- Delete `inflate_polygon` and replace its sole call in `run_support_geometry` with the SDK offset operation over the complete input `ExPolygon`, passing `avoid_inflate` as a millimeter delta.
- Change the internal collision/avoidance cache and containment/clamping helpers as needed to retain `ExPolygon` holes and compare planner mm coordinates against scaled integer polygons through `Point2::from_mm`/`mm_to_units` or equivalent canonical helpers.
- Add focused unit tests to the existing `support-planner/src/lib.rs` test module for the radius cases, concave offset simplicity via a test-local edge-intersection invariant, hole preservation, and the mm coordinate boundary.
- Migrate the existing `radius_tapers_with_distance_to_top` test in `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` from the obsolete floor expectation to the tip-cone expectation; update its geometry fixture to use canonical scaled coordinates if the cache type change requires it.

## Out of Scope

- Assigning or inventing replacement backlog IDs for B5/B6.
- Changing `SupportPlanIR`, `SupportGeometryIR`, WIT packages, manifests, scheduler behavior, or public planner function signatures.
- Porting interface-aware radius widening, `smooth_nodes`, multi-neighbour MST propagation, buildplate pruning, raft behavior, or paint migration.
- Reorganizing `slicer-core::polygon_ops`, changing the global coordinate convention, or deleting other private geometry helpers such as `point_in_polygon` and `closest_point_on_segment`.
- Replacing self-capture tests with real Orca output; the separate Orca-reference backlog item remains blocked by fixture/runner/metric decisions.

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` - direct read of §B5, §B6, and §D2; source of the formula and the local offset-removal intent.
- `docs/08_coordinate_system.md` - direct read of the rule, conversion, and Clipper2 sections; source of 10,000 scaled units/mm and mm-valued float boundaries.
- `docs/05_module_sdk.md` - bounded read of the guest dependency rules and the prohibition on adding host-only crates to guest modules.
- `crates/slicer-sdk/src/host.rs` - bounded read of `offset_polygons` and `OffsetJoinType`; current guest-facing API authority.
- `crates/slicer-schema/wit/deps/common.wit` - existing `offset-polygons` host-service signature; no WIT change is proposed.
- `docs/adr/0023-arachne-port-strategy.md` - current host-side crate strategy; do not silently amend it for B6.
- `docs/specs/support-modules-orca-port-plan.md` - packet 117 queue row only; source-plan labels and dependency order.
- `docs/07_implementation_status.md` - targeted lookup of all colliding `TASK-254`/`TASK-255` entries and support `TASK-163` rows; current ledger authority, not edited here.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegated `TreeSupport::calc_branch_radius` second overload; confirm only the tip-cone and linear-above-cone formula is asserted, while interface-aware widening remains out of scope.

## Acceptance Summary

Reference, never duplicate, the criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-8`; pure radius cases, static SDK-helper replacement without a direct guest dependency, concave simplicity, hole retention, coordinate boundary, and migration of the existing old-radius test.
- Negative: `AC-N1` through `AC-N2`; the old branch-radius floor and raw scaled-coordinate interpretation must not return.
- Cross-packet impact: packet 119 must capture validation evidence after this correction; packets 121 and 122 consume the changed radii; packet 116 is queued first for shared-file review. No packet may infer B5/B6 backlog closure from `TASK-163 (algorithmic)` or the colliding IDs.

## Verification Commands

This is the authoritative full matrix; cargo commands use `--all-targets`, and every test invocation tees output as required.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p support-planner --all-targets -- tapered_radius_at_tip_is_zero --nocapture 2>&1 \| tee target/test-output.log` | AC-1 tip radius. | FACT pass/fail; bounded failure SNIPPETS |
| `cargo test -p support-planner --all-targets -- tapered_radius_inside_cone_is_mm_to_top --nocapture 2>&1 \| tee target/test-output.log` | AC-2 cone branch. | FACT pass/fail; bounded failure SNIPPETS |
| `cargo test -p support-planner --all-targets -- tapered_radius_above_cone_is_linear --nocapture 2>&1 \| tee target/test-output.log` | AC-3 linear branch. | FACT pass/fail; bounded failure SNIPPETS |
| `cargo test -p support-planner --all-targets -- tapered_radius_clamps_at_max --nocapture 2>&1 \| tee target/test-output.log` | AC-4 upper clamp. | FACT pass/fail; bounded failure SNIPPETS |
| `! rg -q 'fn inflate_polygon|inflate_polygon\(' modules/core-modules/support-planner/src/lib.rs && rg -q 'slicer_sdk::host::offset_polygons' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'slicer-core' modules/core-modules/support-planner/Cargo.toml` | AC-5 removal, SDK replacement, and guest dependency boundary. | FACT pass/fail |
| `cargo test -p support-planner --all-targets -- offset_concave_l_shape_no_self_intersection --nocapture 2>&1 \| tee target/test-output.log` | AC-6 concave offset validity. | FACT pass/fail; bounded failure SNIPPETS |
| `cargo test -p support-planner --all-targets -- offset_polygon_with_hole_preserves_hole --nocapture 2>&1 \| tee target/test-output.log` | AC-7 hole retention. | FACT pass/fail; bounded failure SNIPPETS |
| `cargo test -p support-planner --test orca_parity_tdd --all-targets -- radius_tapers_with_distance_to_top --nocapture 2>&1 \| tee target/test-output.log` | AC-8 migration of the existing old oracle. | FACT pass/fail; bounded failure SNIPPETS |
| `cargo test -p support-planner --all-targets -- offset_preserves_mm_coordinate_boundary --nocapture 2>&1 \| tee target/test-output.log` | AC-N2 unit-boundary regression. | FACT pass/fail; bounded failure SNIPPETS |
| `cargo check -p support-planner --all-targets` | Compile production, unit, and integration targets. | FACT pass/fail |
| `cargo clippy -p support-planner --all-targets -- -D warnings` | Lint gate. | FACT pass/fail; bounded failure SNIPPETS |
| `cargo xtask build-guests --check` | Freshness gate after planner source edits. | FACT `up to date` or `STALE: <path>` |

## Step Completion Expectations

- The radius test migration owns the existing direct oracle; no old assertion may continue to require `tapered_radius(0) == branch_radius`.
- The offset call must consume complete `ExPolygon` values and the internal cache must not flatten away holes merely to preserve the old `Vec<Vec<[f32; 2]>>` shape.
- `avoid_inflate` remains a millimeter scalar; all cache comparisons with planner node coordinates use the canonical 10,000-units/mm boundary.
- The current SDK `offset_polygons` signature and `OffsetJoinType::Miter` are authoritative. Do not prescribe the stale `JoinType` name, an arc-tolerance argument absent from the SDK seam, or a direct host-only planner dependency.
- The exact call count in `support-planner/src/lib.rs` is one production use of `slicer_sdk::host::offset_polygons`; tests may call the SDK helper as their local oracle.

## Context Discipline Notes

- `modules/core-modules/support-planner/src/lib.rs` is long; read only `tapered_radius`, `run_support_geometry`, `LayerCollisionCache`, the containment/clamping helpers, and the existing `tests` module.
- `crates/slicer-sdk/src/host.rs` is long; read only `OffsetJoinType` and `offset_polygons` plus their immediate bodies.
- `docs/07_implementation_status.md` is mutable ledger state; re-derive the collision evidence at mapping time and never edit it in this packet.
- Delegate the Orca source lookup and all cargo commands; return Orca as `SUMMARY` and cargo as `FACT` with bounded failure snippets.

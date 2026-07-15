# Design: 156-arachne-region-order

## Selected Approach

Port Orca's region-order behavior as an end-to-end ordering contract, not a
local sort. The module resolves the configured three-state wall sequence, the
WIT record transports it unchanged, core orders finalized Arachne lines, the
module commits the resulting sequence, and the optimizer may optimize travel
without inverting it.

## Domain Boundaries

- **Perimeter module:** sole owner of config interpretation and final committed
  `WallLoop` sequence.
- **WIT/SDK/host:** transports the resolved sequence; it does not re-read or
  reinterpret module config.
- **slicer-core Arachne:** computes canonical region constraints and walks
  finalized lines.
- **path optimizer:** preserves committed wall sequence for these walls; any
  allowed travel optimization is subordinate to that relation.
- **SparsePointGrid:** returns cell candidates. `get_region_order` applies the
  canonical pair eligibility predicate.

## Algorithm Constraints

- Match Orca's pair guards: reject the same line, equal `inset_idx`, and
  differences greater than one inset before adding a constraint.
- Use set semantics so multiple nearby junction pairs cannot duplicate an edge.
- Retain Orca's odd/even predicate after the guards.
- Generate only acyclic canonical constraints; do not force-emit a cycle.
- Apply the walk after generation, stitching, small-line removal, contour
  separation, simplification, and empty-line removal.
- Preserve all three modes. `InnerOuterInner` cannot be represented by
  `outer_to_inner: bool`; port its canonical layer-sensitive behavior.

## Expected Change Surface

- `crates/slicer-core/src/arachne/{region_order.rs,sparse_point_grid.rs,pipeline.rs,separate_inner_contour.rs}`
- `crates/slicer-schema/wit/deps/common.wit` and generated-binding consumers
- `crates/slicer-sdk/src/host.rs`, `crates/slicer-wasm-host/src/host.rs`
- `modules/core-modules/arachne-perimeters/src/lib.rs`
- `modules/core-modules/path-optimization-default/src/lib.rs`
- focused core, WIT/module, and end-to-end tests
- docs named in the packet Doc Impact Statement

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs`: `ExtrusionLine`, `ExtrusionJunction`,
  and `WallLoop` shapes.
- `docs/01_system_architecture.md:888-895` and ADR-0011: existing wall-order
  ownership.
- `docs/03_wit_and_manifest.md`: host-service and WIT contract rules.
- `docs/08_coordinate_system.md`: f32-mm coordinate convention.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/`, `target/`, generated bindgen output, and lockfiles.
- Unrelated beading, simplify, G-code, scheduler, and classic-perimeter code.
- Any new config key or change to raw `wall_sequence` configuration values.

## Expected Sub-Agent Dispatches

- Canonical `getRegionOrder`, grid, walk, and sandwich-mode reads: `SNIPPETS`
  or `SUMMARY`, never full Orca source.
- WIT binding call-site inventory: `LOCATIONS` <=20.
- Guest freshness, focused tests, workspace check, and clippy: `FACT` only.

## Data and Contract Notes

- `arachne-params` changes from a derived bool to a three-state sequence value.
- `WallLoop` and `ExtrusionLine` IR shapes remain unchanged.
- No new config key is introduced; the module transports the existing resolved
  `wall_sequence` value.

## Locked Assumptions and Invariants

- The module owns configuration interpretation and final committed wall order.
- WIT/SDK/host transport the resolved sequence without substituting a default.
- Region ordering consumes finalized lines and is a permutation.
- The optimizer may not invert committed wall sequence.

## Rejected Alternatives

- Boolean `outer_to_inner`: loses `InnerOuterInner` semantics.
- Host-side config derivation: violates module config ownership.
- Pre-stitch region order: operates on non-canonical entities.
- Unconditional `perimeter_index` sort or optimizer role regrouping: destroys
  selected wall sequence.
- Cycle fallback: masks invalid constraints rather than matching Orca.

## Risks

- WIT shape changes require guest rebuild and all bindgen users to compile.
- The existing optimizer has a hard-coded role order; correcting it can affect
  classic consumers, so scope behavior by the committed ordering contract and
  add regression fixtures.
- `InnerOuterInner` has distinct initial-layer behavior and must not be tested
  as an alias for `OuterInner`.

## Open Questions

None. The prior ownership, boundary, grid, stage, and test-evidence questions
were resolved during the packet revision.

---
status: implemented
packet: 156-arachne-region-order
task_ids:
  - none
backlog_source: docs/18_arachne_parity_audit.md
context_cost_estimate: M
---

# Packet Contract: 156-arachne-region-order

## Goal

Close G12 with an end-to-end faithful port of OrcaSlicer's Arachne region
ordering. The selected `wall_sequence` must survive module configuration, the
WASM boundary, final `WallLoop` commitment, and path optimization without
being collapsed, re-sorted, or inverted.

## Scope Boundaries

- Port canonical `getRegionOrder` pair eligibility and set semantics from
  `WallToolPaths.cpp`, including same-line, equal-inset, and non-adjacent-inset
  exclusions; emit no duplicate constraints.
- Port canonical `SparsePointGrid` candidate-cell semantics. The grid returns
  candidates; `get_region_order` owns the exact pair predicate.
- Port the canonical topological walk over finalized extrusion lines. Do not
  force-emit cycles; canonical constraints must be acyclic. Preserve Orca's
  open-line cursor behavior and stable input-order tie behavior.
- Represent the complete configured sequence at every boundary:
  `InnerOuter`, `OuterInner`, and `InnerOuterInner`. A boolean is insufficient.
- Add the sequence to the existing `arachne-params` WIT record and propagate it
  through SDK, guest, and host. The perimeter module remains the sole resolver
  of `wall_sequence` config.
- Run region ordering after `remove_empty_toolpaths`, immediately before
  `run_arachne_pipeline` returns finalized lines.
- Make `arachne-perimeters` commit a wall-sequence-aware final `WallLoop`
  order. It must not unconditionally restore ascending `perimeter_index`.
- Make `path-optimization-default` preserve committed wall sequence for these
  walls while still optimizing permitted travel. Its role grouping must not
  invert `OuterInner` or `InnerOuterInner`.

Out of scope: unrelated G11/G15/G20 behavior, new user-visible config keys,
and unrelated optimizer heuristics. This packet does change WIT, SDK, host,
module, and optimizer contracts as required to preserve the existing config.

## Acceptance Criteria

- **AC-1 (canonical constraints).** Given nearby lines, when
  `get_region_order` runs, then it excludes same-line, same-inset, and
  non-adjacent-inset pairs; applies the canonical odd/even predicate; and
  returns each `(before, after)` pair once. |
  `cargo test -p slicer-core --test region_order_tdd -- region_order_get_matches_canonical_pair_guards region_order_excludes_same_line_same_inset_and_non_adjacent_insets region_order_deduplicates_constraints_from_multiple_junction_pairs region_order_constraints_are_unique_and_acyclic`
- **AC-2 (candidate grid).** Given points in cells touched by a query circle,
  when `SparsePointGrid::get_nearby` runs, then it returns candidate-cell
  contents without an independent exact-distance filter. |
  `cargo test -p slicer-core --test sparse_point_grid_tdd -- sparse_point_grid_returns_touched_cell_candidates --exact`
- **AC-3 (canonical walk).** Given canonical acyclic constraints over open and
  closed lines, when `topological_walk` runs, then the fixture returns exactly
  `[0, 1, 2, 3, 5, 4]`, proving first-input cursor initialization, open-line
  endpoint updates, open-before-closed iteration, stable input-order ties, and
  constraint unlocking; no cycle fallback exists. |
  `cargo test -p slicer-core --test region_order_tdd -- region_order_topological_walk_matches_canonical_open_line_cursor --exact`
- **AC-4 (finalized-line integration).** Given Arachne output requiring
  stitching and simplification, when the pipeline returns, then region order
  was applied after `remove_empty_toolpaths` and before return. |
  `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_parity_wall_region_order_odd_after_enclosing --exact`
- **AC-5 (three-state boundary).** Given each `wall_sequence` value and layer
  0/later-layer `InnerOuterInner`, when the real WASM module invokes the host,
  then a host-side test capture made immediately after WIT decoding equals the
  selected `WallSequence` variant, including `InnerOuterInner` on layer 0. |
  `cargo test -p slicer-runtime --test arachne_parity_round2 -- arachne_wall_sequence_survives_wasm_boundary --exact`
- **AC-6 (committed sequence).** Given each sequence mode, when the Arachne
  perimeter module commits walls, then committed `WallLoop` order honors the
  configured mode and preserves distinguishable core path order
  `outer-A < odd-inner-A < outer-B`, even though `inner-A.perimeter_index` is
  greater than `outer-B.perimeter_index`. |
  `cargo test -p arachne-perimeters --test wall_sequence_commit_tdd`
- **AC-7 (optimizer preservation).** Given committed Arachne walls in each
  sequence mode, when `path-optimization-default` runs, then it does not
  invert their selected sequence. A live Arachne Perimeters -> optimizer
  sandwich fixture on layer 1 must preserve path identities `[1, 0, 2]` and
  roles `[InnerWall, OuterWall, InnerWall]`; role-only fixtures are invalid. |
  `cargo test -p slicer-runtime --test arachne_wall_sequence_e2e_tdd`
- **AC-8 (permutation and regressions).** The region-order pass is a
  permutation, all G12 tests pass, and all non-D-104f Arachne locks remain
  green. D-104f concentric infill is an explicitly unrelated known red. |
  `cargo test -p slicer-runtime --test arachne_parity_round2 && cargo test -p slicer-runtime --test arachne_parity -- --skip arachne_parity_pipeline_concentric_infill_uses_arachne && cargo test -p slicer-core`

## Negative Criteria

- Empty and zero-width inputs produce no constraints without constructing a
  zero-cell grid.
- No generated canonical constraint graph contains a cycle.
- No WIT/host path silently substitutes `InnerOuter` or `false` for a selected
  mode.

## Verification

- `cargo xtask build-guests --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Run every pipe-suffixed command above and `cargo test -p slicer-core`.

## Doc Impact Statement

- `docs/18_arachne_parity_audit.md`: correct G12's canonical references and
  keep G12 implementation-complete but not closed until AC-1 through AC-8 and
  the final acceptance review pass. Pin the pending state —
  `rg -q 'closure remains pending' docs/18_arachne_parity_audit.md`.
- `docs/03_wit_and_manifest.md`: document the extended `arachne-params`
  boundary and the three-state WIT enum, including the optimizer-preservation
  clause — `rg -q 'arachne-params' docs/03_wit_and_manifest.md && rg -q 'wall-sequence' docs/03_wit_and_manifest.md && rg -q 'preserve' docs/03_wit_and_manifest.md`.
- `docs/01_system_architecture.md` and ADR-0011: reconcile final committed
  wall sequence with optimizer behavior — `rg -q 'own wall sequencing' docs/01_system_architecture.md && rg -q 'optimiz' docs/01_system_architecture.md && rg -q 'final print order' docs/adr/0011-perimeter-module-owns-wall-sequencing.md`.
- `docs/DEVIATION_LOG.md`: retain D-157's intentional behaviorally equivalent
  deviations and keep its closure pending the final acceptance ceremony. Pin
  the pending state — `rg -q 'pending final packet acceptance ceremony' docs/DEVIATION_LOG.md`.
- `CONTEXT.md`: the packet-refinement session already added the resolved
  glossary terms; verify they remain — `rg -q 'Committed wall sequence' CONTEXT.md`

## OrcaSlicer Reference Obligations

Delegate all reads. Verify against the real current source, not historical
line numbers: `WallToolPaths.cpp` `getRegionOrder`, `SparseGrid.hpp` lookup,
`PerimeterGenerator.cpp` walk and `InnerOuterInner` behavior, and
`PrintConfig` sequence definitions/defaults.

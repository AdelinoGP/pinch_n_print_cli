---
status: implemented
packet: test-quality-remediation_06_core-wall-sequence-dup-merges
task_ids:
  - core/DUP-CORE (wall sequence)
---

# test-quality-remediation_06_core-wall-sequence-dup-merges

## Goal

Consolidate the three wall-sequence inline twins into their strictly stronger `wall_sequence_reorder_tdd` survivors, import the inline module's distinct N==2 sandwich case, and delete the complete duplicate `#[cfg(test)]` module while preserving all three modes, both edge cases, and the two-wall sandwich input.

## Problem Statement

`perimeter_utils.rs` repeats three wall-sequence tests inside a production file's `#[cfg(test)]` module that the standalone `wall_sequence_reorder_tdd.rs` already asserts strictly stronger on an identical 3-wall fixture: the TDD tests check both loop-type slots and per-slot `perimeter_index` identity, while the inline twins check only `perimeter_index`. Conversely, the inline module owns one case the standalone home lacks — the N==2 `InnerOuterInner` swap — so deleting by file alone would silently lose the two-wall sandwich witness, and deleting by name alone could lose the loop-type assertions or the distinct edge cases.

## Architecture Constraints

- This is a merge-with-union test consolidation. It may remove only `inner_outer_is_canonical_no_reorder`, `outer_inner_reverses`, and `inner_outer_inner_sandwich` (each strictly subsumed by its TDD survivor) plus the inline definition of `inner_outer_inner_with_two_walls_swaps_outer_and_first_inner` (whose case migrates); every distinct fixture and assertion remains in a named survivor.
- The TDD target is ungated: the census entry carries `required: []` and the `[[test]]` stanza has no `required-features`. Every target command runs without a feature flag; introducing or asserting a `host-algos` gate here is a scope violation (unlike the four prior core-wave packets, whose targets required `host-algos`).
- The `perimeter_utils.rs` edit is confined to the `#[cfg(test)] mod wall_sequence_reorder_tests` block at the file tail; `perimeter_utils` is a plain `pub mod` of `slicer-core` (not `host-algos`-gated), the removed block is neither a doc example nor a guest-WASM input, so guest freshness is not implicated.
- Preserve canonical parity supremacy: the three surviving order tests' exact per-slot orderings (`[0,1,2]`, `[2,1,0]`, `[1,0,2]`) and the N==2 `[1,0]` sandwich swap are retained, never loosened or replaced by weaker witnesses.

## Data and Contract Notes

- IR/manifest contracts: no shape or version changes. Tests continue to construct `slicer_ir::WallLoop` values with `ExtrusionPath3D`, `LoopType`, and `WallBoundaryType::ExteriorSurface` exactly as both pre-edit homes already do.
- WIT boundary: none.
- Determinism/scheduler constraints: `wall_sequence_reorder` is a documented pure function (same input → same output); the mode assertions `[0,1,2]`, `[2,1,0]`, `[1,0,2]`, and the N==2 `[1,0]` swap are deterministic index expectations, and the empty/single-wall edge cases pin the function's no-op boundaries.

## Locked Assumptions and Invariants

- TDD source count is `5 → 6`; inline count is `4 → 0`. Every delta is accounted by the survivor map above.
- The three inline twins are strict assertion subsets of their TDD survivors on the identical `[Outer(0), Inner(1), Inner(2)]` fixture; the N==2 case is distinct and migrates.
- The census manifest's role is target/feature registration only (`required: []`); fresh `-- --list` output is the per-function execution census; no packet-time count may substitute for it at implementation.
- No public symbol, new test file, new helper, or later-packet export is introduced.
- The implementation-time ledger remains `partial`; it records this slice and leaves `core-geometry-dup-review` as remaining core work. Packet Queue edits are not part of implementation.

## Risks and Tradeoffs

- Deleting the entire inline module could accidentally remove production code if its boundary is misidentified. The edit begins at the sole `#[cfg(test)] mod wall_sequence_reorder_tests` (file tail, lines 962–1054 at generation time) and ends at that module's closing brace; production ends before it, and the doc-comment landmark is read-only.
- A count-only merge can look correct while losing assertion strength. AC-2 pins the loop-type plus per-slot identity assertions in each named survivor; AC-3 and AC-4 pin the migrated N==2 and the two distinct edge cases.
- The inline twins assert only `perimeter_index`, so a reviewer might misread them as distinct coverage. AC-2's body-scoped greps prove the survivors carry strictly more assertions on the same fixture, making the absorption loss-free.
- Ungated-target commands run without a feature flag; a feature-blind-run false green is not possible here, but silently "upgrading" the target to `host-algos` would change crate test-graph economics — AC-5 locks `required: []` and the stanza without `required-features`.
- A broad documentation edit could conflate implementation evidence with queue generation. Step 2 permits only the six-cell §7 `core` row and AC-7 anchors parsing to the Ledger section.

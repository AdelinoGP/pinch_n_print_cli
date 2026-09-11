# Design: core-wall-sequence-dup-merges

## Controlling Code Paths

- Primary code path: `slicer_core::perimeter_utils::wall_sequence_reorder` (`crates/slicer-core/src/perimeter_utils.rs`), called with all three `WallSequence` variants; its production body, the `WallSequence` enum, and the `tree` parameter's documented M2 grouping fallback are read-only.
- Neighboring tests/fixtures: `make_wall(perimeter_index, loop_type, role)` and `three_wall_set()` in `wall_sequence_reorder_tdd.rs`; the inline module's duplicate `make_wall` helper (with its `wall_loop_base()` exhaustive-waist base) is deleted with the `#[cfg(test)]` module.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- This is a merge-with-union test consolidation. It may remove only `inner_outer_is_canonical_no_reorder`, `outer_inner_reverses`, and `inner_outer_inner_sandwich` (each strictly subsumed by its TDD survivor) plus the inline definition of `inner_outer_inner_with_two_walls_swaps_outer_and_first_inner` (whose case migrates); every distinct fixture and assertion remains in a named survivor.
- The TDD target is ungated: the census entry carries `required: []` and the `[[test]]` stanza has no `required-features`. Every target command runs without a feature flag; introducing or asserting a `host-algos` gate here is a scope violation (unlike the four prior core-wave packets, whose targets required `host-algos`).
- The `perimeter_utils.rs` edit is confined to the `#[cfg(test)] mod wall_sequence_reorder_tests` block at the file tail; `perimeter_utils` is a plain `pub mod` of `slicer-core` (not `host-algos`-gated), the removed block is neither a doc example nor a guest-WASM input, so guest freshness is not implicated.
- Preserve canonical parity supremacy: the three surviving order tests' exact per-slot orderings (`[0,1,2]`, `[2,1,0]`, `[1,0,2]`) and the N==2 `[1,0]` sandwich swap are retained, never loosened or replaced by weaker witnesses.

## Code Change Surface

- Selected approach: one explicit survivor map. Add the migrated N==2 sandwich test to `wall_sequence_reorder_tdd.rs` (with the `make_wall`/`three_wall_set` helpers already there; the two-wall case uses two direct `make_wall` calls, not `three_wall_set()`), then delete the complete `#[cfg(test)] mod wall_sequence_reorder_tests` block from `perimeter_utils.rs` and its now-unneeded inline-use imports (`slicer_ir::{ExtrusionPath3D, ExtrusionRole, LoopType, WallLoop}` inside the module).
- Exact functions, tests, and assertions:
  - Inline canonical absorbed → survivor: `inner_outer_is_canonical_no_reorder` → `inner_outer_canonical_order`; identical 3-wall fixture and per-slot `perimeter_index` `[0,1,2]`; survivor also asserts loop types per slot.
  - Inline reversed absorbed → survivor: `outer_inner_reverses` → `outer_inner_reversed_order`; identical fixture and per-slot `[2,1,0]`; survivor also asserts loop types.
  - Inline sandwich absorbed → survivor: `inner_outer_inner_sandwich` → `inner_outer_inner_sandwich_order`; identical fixture and per-slot `[1,0,2]`; survivor also asserts loop types.
  - Migrated distinct case: `inner_outer_inner_with_two_walls_swaps_outer_and_first_inner` moves to the TDD home with input `[Outer(0), Inner(1)]`, mode `InnerOuterInner`, and assertions `walls[0].perimeter_index == 1`, `walls[1].perimeter_index == 0`; its doc comment retains the `[Inner_0, Outer]` / N==2 result statement (`N == 2` greppable in the survivor body).
  - Distinct survivors: `empty_walls_is_noop` (empty input, `InnerOuterInner`, still empty) and `single_wall_unchanged_for_all_modes` (all three modes, perimeter_index 42, length 1) remain unchanged.
- Rejected alternatives: retain inline wrappers (leaves duplicate coverage in a production file); split survivor homes (two homes for one API; user-approved single TDD home); delete the inline module wholesale without migrating the N==2 case (loses the only two-wall sandwich witness); parameterize or rename survivors (unnecessary churn and breaks the approved map); alter production logic or the `tree` parameter (outside scope); delete the production doc comments (the removed block is tests, not doc examples).

## Files in Scope (read + edit)

- `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs` - role: selected wall-sequence test home; expected change: add the migrated `inner_outer_inner_with_two_walls_swaps_outer_and_first_inner` test; all five existing tests and both helpers remain.
- `crates/slicer-core/src/perimeter_utils.rs` - role: duplicate inline-test owner; expected change: delete only the complete `#[cfg(test)] mod wall_sequence_reorder_tests` block, leaving production code unchanged.
- `docs/specs/test-quality-remediation-plan.md` §7 `core` row only - role: mandatory program bookkeeping after the merge; expected change: retain `partial` state, record this packet's survivor/validation evidence, and name `core-geometry-dup-review` as remaining work without changing the Packet Queue.

## Read-Only Context

- `crates/slicer-core/src/perimeter_utils.rs` - `wall_sequence_reorder` signature, `WallSequence` enum, and the `#[cfg(test)]` block (lines 886–1054) only - symbol shape and inline-twin disposition; the file is 1054 lines, so no whole-file read.
- `crates/slicer-core/tests/wall_sequence_reorder_tdd.rs` - full 122-line file - fixture helpers and survivor bodies.
- `crates/slicer-core/Cargo.toml` - the `wall_sequence_reorder_tdd` `[[test]]` stanza only - ungated registration.
- `docs/specs/test-quality-remediation-census.json` - the matching `slicer-core` entry only - target registration with `required: []`; it does not carry function counts.
- `docs/specs/test-quality-remediation-plan.md` - §§1–4, §5.1 DUP-CORE, §§6–7, and Packet Queue row #6/resume exports only.
- `docs/22_test_quality.md` §§1–5, ADR-0064, ADR-0065, `docs/01_system_architecture.md` wall-sequence range - contract context only.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegated only; canonical `PerimeterGenerator::process` sequencing behavior named in the `WallSequence` doc comment.

## Out-of-Bounds Files

- Every other `crates/slicer-core/tests/**` file, including `flow_tdd.rs`, `bridge_false_site_gating_tdd.rs`, `algo_region_mapping_tdd.rs`, `support_overhang_detection_tdd.rs`, and `algo_support_geometry_tdd.rs`; these are owned by the four prior packets and no overlap exists.
- Every `crates/slicer-core/src/**` file other than `perimeter_utils.rs`; within `perimeter_utils.rs`, all production statements and docs above the `#[cfg(test)]` module are out of bounds.
- `modules/core-modules/classic-perimeters/src/lib.rs` and `modules/core-modules/arachne-perimeters/src/lib.rs` (production `wall_sequence_reorder` call sites), all other crates/modules, `xtask/**`, WIT/schema/config/manifest files, existing `docs/**` outside this packet directory except the narrowly allowed §7 `core` row, sibling packet directories, generated code, vendored dependencies, `target/`, and `Cargo.lock`.
- Within `docs/specs/test-quality-remediation-plan.md`, the Packet Queue, every non-`core` ledger row, and all prose/sections outside the Step 2 §7 `core` row are out of bounds.
- Direct reads of `OrcaSlicerDocumented/**`; use the bounded delegation contract instead.

## Expected Sub-Agent Dispatches

- Question: re-derive the two pre-edit source inventories and confirm the survivor relationships have not drifted; scope: the two in-scope Rust files; return: `FACT` with `5`, `4`, the three absorbed names, the migrated name, and the subset relationship; purpose: precondition for Step 1.
- Question: confirm canonical `PerimeterGenerator::process` wall-emission sequencing (outer-first, reversed, sandwich) without proposing production edits; scope: `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp`; return: `SUMMARY` ≤200 words; purpose: protect the retained parity order witnesses.
- Question: run the ungated target and the closure gates; scope: `requirements.md` verification matrix; return: `FACT` pass/fail with ≤20 failure lines; purpose: census, behavior, and lint validation.

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

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: pre-edit survivor-map revalidation, `FACT` with the two counts plus assertion-subset result.

## Open Questions

None.
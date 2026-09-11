# Design: core-flow-consolidation

## Controlling Code Paths

- Primary code path: existing public `line_width_to_spacing`, `flow_to_width`, `bridging_flow`, and `resolve_role_width` in `crates/slicer-core/src/flow.rs`; no function bodies change.
- Neighboring tests/fixtures: inline `slicer_core::flow::tests` in `crates/slicer-core/src/flow.rs`; integration tests in `crates/slicer-core/tests/flow_tdd.rs`.
- OrcaSlicer comparison: not applicable to this test-only consolidation; existing module comments already document the pre-existing formula and this packet does not port canonical code.

## Architecture Constraints

- The exact edit surface is the test module in `crates/slicer-core/src/flow.rs` plus `crates/slicer-core/tests/flow_tdd.rs`; no production behavior or export changes.
- The single integration test binary is `flow_tdd`; `crates/slicer-core/Cargo.toml` confirms it has no `required-features` stanza, but all commands still use `--features host-algos` to avoid feature-blind verification under the core-wave policy.
- Preserve the canonical rejection rule `spacing <= 0.0`, including zero/negative widths and the true threshold; do not reintroduce a width-vs-layer-height guard or a zero sentinel.
- Preserve `NegativeSpacingError` fields `width_mm`, `layer_height_mm`, and `spacing_mm`, plus the three actionable message fragments named by the acceptance criteria.
- No schema, WIT, coordinate conversion, scheduler, manifest, or WASM contract is touched; the units remain existing millimetres at the flow API boundary.
- `flow_correction_stays_positive_for_vertical_input` in `crates/slicer-core/src/lib.rs` is explicitly out of bounds and remains for `core-strengthen`.

## Code Change Surface

- Selected approach: inventory the union of inline and integration cases; move inline-only cases to the integration file; merge exact duplicates by retaining one integration witness with the union of assertions; retain all distinct role and flow-to-width cases; strengthen the wider-bead oracle with an independently computed expected formula.
- Exact functions/types/fields/tests: `line_width_to_spacing`, `flow_to_width`, `bridging_flow`, `resolve_role_width`; `NegativeSpacingError::{width_mm,layer_height_mm,spacing_mm}`; `RoleWidthContext`; `ExtrusionRole`; all named flow tests in the requirements matrix.
- Rejected alternatives: leave inline tests in place (violates single-home boundary); delete overlapping cases without a survivor map (loses distinct regression inputs); change production exports or formulas (outside scope and parity-prohibited); move the vertical correction case (`core-strengthen` ownership).

## Files in Scope (read + edit)

- `crates/slicer-core/src/flow.rs` - role: current inline test owner; expected change: delete only its `#[cfg(test)] mod tests` block.
- `crates/slicer-core/tests/flow_tdd.rs` - role: approved single test home; expected change: add/merge all inline cases and strengthen the wider-bead oracle while retaining current role-width cases.
- `docs/specs/test-quality-remediation-plan.md` §7 only - role: mandatory program bookkeeping; expected change: update only the `core` ledger row with partial flow status, actual validations, surviving coverage, and the remaining `core-bridge-dup-merge` gap.

## Read-Only Context

- `crates/slicer-core/Cargo.toml` - lines 7–14 and 94–99 only - feature declaration and `flow_tdd` target wiring.
- `crates/slicer-core/src/flow.rs` - lines 42–275 and 277–400 only - public API, fields, formula/error contract, and inline tests.
- `crates/slicer-core/tests/flow_tdd.rs` - lines 1–306 only - existing integration cases and fixture helpers.
- `crates/slicer-core/src/lib.rs` - symbol lookup only for `flow_correction_stays_positive_for_vertical_input`; do not edit.
- `docs/specs/test-quality-remediation-plan.md` - ranges §1, §4, §5.1, §6, §7 Ledger, Packet Queue/boundary only.

## Out-of-Bounds Files

- `crates/slicer-core/src/lib.rs`, except read-only ownership confirmation for the excluded vertical correction test.
- All other `crates/slicer-core` tests, source modules, manifests, generated artifacts, `target/`, and `Cargo.lock`.
- `docs/07_implementation_status.md`, parent plan sections other than the implementation-time §7 Ledger edit explicitly allowed above, canonical backlog, other packet directories, and source/tests outside the two named files.
- `OrcaSlicerDocumented/...` - no inspection is applicable; if implementation discovers a parity claim requiring external inspection, stop and request a delegated reference lookup rather than inventing a path.

## Expected Sub-Agent Dispatches

- Question: inventory current `flow_tdd` test discovery and post-edit before/after registration; scope: `crates/slicer-core/tests/flow_tdd.rs`, `crates/slicer-core/Cargo.toml`; return: `LOCATIONS` with at most 20 entries; purpose: prove one target and feature wiring.
- Question: compare pre-edit inline and integration test names/assertion inputs and produce the survivor crosswalk; scope: the two in-scope files only; return: `SUMMARY` of at most 200 words; purpose: prevent loss of a distinct case before deletion.
- Question: run the feature-correct exact target checks and gates; scope: commands in `requirements.md`; return: `FACT` pass/fail, with at most 20 failure lines; purpose: implementation validation. (No generation-time test execution.)

## Data and Contract Notes

- IR/manifest contracts: none.
- WIT boundary: none.
- Determinism/scheduler constraints: none; the role matrix must retain deterministic explicit role ordering and independently derived expected values.

## Locked Assumptions and Invariants

- Every distinct inline regression input remains asserted in `flow_tdd`; duplicate names may change only when their assertions and inputs are unioned into the surviving integration test.
- The wider-bead expected value is computed from the literal formula, not from another call to `line_width_to_spacing`.
- Zero and negative widths remain rejection cases; exact threshold remains rejection; zero layer height with positive width remains exactly positive.
- No new public symbol is introduced and no net-new export is expected.

## Risks and Tradeoffs

- Moving tests across the crate boundary could expose private access assumptions; current inline cases call public `flow` APIs and public fields, so integration placement is viable. Any private access discovered is a blocker, not permission to widen production visibility.
- Merging by test name alone could lose a distinct input; the survivor crosswalk and exact assertion inventory must be completed before deletion.
- Feature flags can make a narrow run appear green without compiling the intended target; discovery and exact-name guards are mandatory.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: survivor crosswalk, `SUMMARY` ≤200 words.

## Open Questions

None. `[FWD]` The implementation worker may choose final survivor names only if each name remains concrete, unique, and represented in the AC discovery command. `[BLOCK]` Any private-access requirement, missing public API, or inability to preserve a distinct input without production edits stops this packet rather than expanding scope.

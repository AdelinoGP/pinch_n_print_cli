---
status: implemented
packet: test-quality-remediation_01_core-flow-consolidation
task_ids:
  - core/DUP-CORE
  - core/RETIRE
  - core/DUP-CORE-strengthen
---

# test-quality-remediation_01_core-flow-consolidation

## Goal

Consolidate the inline and integration flow tests into `crates/slicer-core/tests/flow_tdd.rs`, preserving every distinct input/assertion case while preserving and verifying the existing independent-formula oracle and its strict wider-than-canonical comparison.

## Problem Statement

The flow implementation currently has inline unit tests in `slicer_core::flow::tests` and overlapping public-API tests in `flow_tdd`. The approved `core-flow-consolidation` slice must establish one integration-test home without losing any distinct regression input, while preserving and verifying the wider-bead test's existing independent-formula oracle and strict wider-than-canonical comparison. This is a test-only consolidation; it does not alter flow behavior or close any other core-wave finding.

## Architecture Constraints

- The exact edit surface is the test module in `crates/slicer-core/src/flow.rs` plus `crates/slicer-core/tests/flow_tdd.rs`; no production behavior or export changes.
- The single integration test binary is `flow_tdd`; `crates/slicer-core/Cargo.toml` confirms it has no `required-features` stanza, but all commands still use `--features host-algos` to avoid feature-blind verification under the core-wave policy.
- Preserve the canonical rejection rule `spacing <= 0.0`, including zero/negative widths and the true threshold; do not reintroduce a width-vs-layer-height guard or a zero sentinel.
- Preserve `NegativeSpacingError` fields `width_mm`, `layer_height_mm`, and `spacing_mm`, plus the three actionable message fragments named by the acceptance criteria.
- No schema, WIT, coordinate conversion, scheduler, manifest, or WASM contract is touched; the units remain existing millimetres at the flow API boundary.
- `flow_correction_stays_positive_for_vertical_input` in `crates/slicer-core/src/lib.rs` is explicitly out of bounds and remains for `core-strengthen`.

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

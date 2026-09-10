# Requirements: core-beading-threshold-review

## Packet Metadata

- Grouped task IDs: `core/DUP-CORE (beading factory)`
- Backlog source: `docs/specs/test-quality-remediation-plan.md` (approved plan wave/item mapping; not `docs/07_implementation_status.md` `TASK-###`)
- Packet status: `draft`
- Aggregate context cost: `S`

## Problem Statement

The §5.1 `DUP-CORE` entry names threshold propagation in `beading_factory.rs`. The grounded test body contains three related but distinct witnesses: `beading_factory_passes_split_middle_thresholds` protects the public `BeadingFactoryParams::default()` `.99/.99` seeds; `beading_factory_threshold_propagates_through_full_stack` protects forwarding through `Limited → OuterWallInset → Widening → Redistribute → Distributed`; and `beading_factory_threshold_clamp_bounds_are_canonical` protects lower, interior, and upper formula outcomes. The approved row-#8 scope explicitly preserves all three rather than presuming they are redundant.

The propagation test's existing optional-stack case uses shipped default widths, so its expected `.99/.99` values overlap the default seed and upper-clamp witnesses. The additive contrasting case uses independently derived literals: split `(2 × 3000 / 5000) - 1 = 0.20` and add `3000 / 4000 = 0.75`. This catches a wrapper or factory regression that silently hard-codes `.99` or swaps the two getter paths, while leaving the original propagation case intact. Runtime production calls and fixed numeric assertions are the acceptance oracle; later code review verifies that the authored assertions remain in the selected test.

## In Scope

- Preserve `beading_factory_passes_split_middle_thresholds` and its two independent default-field assertions, each comparing to literal `.99` within `TOLERANCE`; add a KEEP rationale that `Default` seeding is distinct from `create_stack` recomputation.
- Preserve `beading_factory_threshold_propagates_through_full_stack` and its existing `outer_wall_offset = 300.0`, `print_thin_walls = true`, default-width `.99/.99` case; add a second production-stack case with `min_output_width = 3000.0`, `preferred_bead_width_outer = 5000.0`, `optimal_width = 4000.0`, the same optional-stack flags, and independent literal `.20/.75` getter expectations within `TOLERANCE`.
- Preserve `beading_factory_threshold_clamp_bounds_are_canonical` and its production-stack fixtures `min_output_width = 100.0` and `min_output_width = 100_000.0`; retain independent runtime assertions for split `.01`, add `.025`, and split `.99`, each within `TOLERANCE`, and add a KEEP rationale for its distinct boundary matrix.
- Update only the §7 `core` ledger row after test validation, recording the strengthened propagation and all three surviving threshold witnesses.

## Out of Scope

- `crates/slicer-core/src/beading/factory.rs` production code, including `BeadingFactoryParams::default()` values and `BeadingStrategyFactory::create_stack` formulas.
- Any test file other than `crates/slicer-core/tests/beading/factory.rs`; no test deletion, new test target, new API, schema, manifest, or WIT change.
- The other five factory tests, `factory_orca_reference.json`, and every other JSON fixture.
- `crates/slicer-core/Cargo.toml` registration or feature declarations; the existing standalone `beading_factory` target remains the test home.
- Any queue row, non-`core` ledger row, `docs/07_implementation_status.md`, or other packet directory.
- Any guest/WASM artifact or freshness work; this is a host-side test-only change.

## Authoritative Docs

- `docs/specs/test-quality-remediation-plan.md` - §5.1, §6, §7, Packet Queue row #8, and continuation approval; bounded reads only.
- `docs/22_test_quality.md` - §§1–5; independent oracle, counterfactual, and gate requirements.
- `docs/21_data_defaults_and_fixtures.md` - §§1, 4, and 6; watched `BeadingFactoryParams` literals must use FRU or a justified waiver.
- `docs/15_config_keys_reference.md` - Arachne beading-stack terminology and read-only default context.
- `docs/adr/0064-existing-tests-retire-if-unjustified.md` - KEEP/retirement burden and accounted survivor coverage.
- `docs/adr/0065-test-quality-gate-with-delayed-enforce-mode.md` - touched-scope report-mode zero-finding obligation.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (full repo-relative file + function + one-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` - delegated `WallToolPaths::generate()` clamp formulas and factory call.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategyFactory.cpp` - delegated `BeadingStrategyFactory::makeStrategy` forwarding.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/DistributedBeadingStrategy.cpp` - delegated constructor and `DistributedBeadingStrategy::getOptimalBeadCount` threshold consumption.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/BeadingStrategy/BeadingStrategy.cpp` - delegated constructor, `BeadingStrategy::getTransitionThickness`, and `BeadingStrategy::getSplitMiddleThreshold` behavior.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-5`; AC-1 observes the default seed, AC-2 proves additive contrasting propagation, AC-3 observes the clamp matrix, AC-4 proves the registered target remains green, and AC-5 records scoped ledger evidence.
- Cross-packet impact: `core-geometry-dup-review` is a generation-only dependency and exports no symbols, APIs, or files; this packet exports none and changes no production surface.

## Verification Commands

| Command identity (executed through the packet wrapper) | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --features host-algos --test beading_factory beading_factory_passes_split_middle_thresholds -- --exact --nocapture` | default-seed witness | FACT: exact one test and true result; read `target/test-output.log` |
| `cargo test -p slicer-core --features host-algos --test beading_factory beading_factory_threshold_propagates_through_full_stack -- --exact --nocapture` | original plus contrasting propagation cases | FACT: exact one test and true result; read `target/test-output.log` |
| `cargo test -p slicer-core --features host-algos --test beading_factory beading_factory_threshold_clamp_bounds_are_canonical -- --exact --nocapture` | clamp boundary matrix | FACT: exact one test and true result; read `target/test-output.log` |
| `cargo test -p slicer-core --features host-algos --test beading_factory -- --nocapture` | registered target regression check | FACT: positive passing count and zero failures; read `target/test-output.log` |
| `set -euo pipefail; mkdir -p target; cargo check --workspace --all-targets 2>&1 \| tee target/check-output.log >/dev/null` | workspace compile gate | FACT: exit 0 |
| `set -euo pipefail; mkdir -p target; cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/clippy-output.log >/dev/null` | workspace lint gate | FACT: exit 0 |
| `set -euo pipefail; mkdir -p target; cargo xtask check-literals 2>&1 \| tee target/literals-output.log >/dev/null` | watched-struct literal gate | FACT: exit 0 |
| `set -euo pipefail; mkdir -p target; cargo xtask check-test-quality --report crates/slicer-core/tests/beading/factory.rs 2>&1 \| tee target/quality-output.log >/dev/null` | touched-file quality report | FACT: no finding for the edited file |
| Python doc-only parser and named in-memory controls defined in implementation Step 2 | ledger shape, real-heading match, and failure controls | FACT: real-heading positive copy accepted; named bad-state, prior-flag/gap, wrong-column, and missing-evidence controls rejected |

All cargo **test** commands must use `set -euo pipefail`, `mkdir -p target`, and `tee target/test-output.log`; no test result is recovered by re-running. Check, clippy, and xtask gates use the compact-output wrappers and dedicated logs shown above rather than the test log. No `cargo xtask build-guests --check` is required because no guest/component test or source is touched.

## Step Completion Expectations

- Step 1 keeps all three threshold tests and their existing inputs/oracles: the default test's two `.99`/`TOLERANCE` field assertions, the propagation test's default-width `.99/.99`/`TOLERANCE` assertions, and the clamp test's `100.0`/`100_000.0` fixtures with `.01/.025/.99`/`TOLERANCE` assertions. The new `.20/.75` case is additive and remains inside the propagation test.
- Step 1's non-default expected values remain literal independent derivations, never calculated by calling `create_stack` or a getter under test.
- Step 2 reads the real §7 heading and current accumulated `core` row, appends this packet's evidence without deleting prior cell content, changes only the `core` row, and never edits the Packet Queue or non-`core` rows. The checker does not freeze non-`core` row contents.

## Context Discipline Notes

- `docs/specs/test-quality-remediation-plan.md` is long; read only §5.1, §6, §7, and the row-#8 continuation text.
- `OrcaSlicerDocumented/` is delegated only; the author and implementer do not load it directly.
- Do not open JSON fixtures, `target/`, lockfiles, or unrelated packet designs; the factory fixture is explicitly read-only context.

# Design: core-beading-threshold-review

## Controlling Code Paths

- Primary code path: `BeadingStrategyFactory::create_stack` in `crates/slicer-core/src/beading/factory.rs` computes local split/add ratios, constructs `DistributedBeadingStrategy`, and returns the decorator chain; production remains read-only.
- Primary test path: `crates/slicer-core/tests/beading/factory.rs` functions `beading_factory_passes_split_middle_thresholds`, `beading_factory_threshold_propagates_through_full_stack`, and `beading_factory_threshold_clamp_bounds_are_canonical`.
- Neighboring forwarding paths: `BeadingStrategy` threshold getters in `crates/slicer-core/src/beading/mod.rs`, storage in `DistributedBeadingStrategy`, and forwarding in `RedistributeBeadingStrategy`, `WideningBeadingStrategy`, `OuterWallInsetBeadingStrategy`, and `LimitedBeadingStrategy`; all are read-only evidence.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules here.

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The two threshold outputs are dimensionless ratios. `min_output_width`, `optimal_width`, and `preferred_bead_width_outer` are already slicer-unit widths; the test's `.20` and `.75` expectations are hand-derived literals and require no unit conversion.
- `BeadingFactoryParams::default()` seeds the public fields at `.99/.99`, while `create_stack` recomputes local values from the width inputs. The packet must preserve and explain this distinction; it must not change defaults or make the factory consume the seeded fields.
- The original optional-stack propagation case is retained verbatim: `outer_wall_offset = 300.0`, `print_thin_walls = true`, default widths, and `.99/.99` getter assertions. The contrasting case is additive, uses the same optional-stack flags, and expects `.20/.75` independently.
- All edited `BeadingFactoryParams` literals retain `..Default::default()`; the type is a watched public struct under `docs/21_data_defaults_and_fixtures.md`.
- No guest/WASM surface is fed by this host-side test-only edit, so the `wasm-staleness` snippet does not apply and no guest artifact check belongs to the packet.

## Code Change Surface

- Selected approach: additive test-quality repair with KEEP rationales; do not consolidate or delete any of the three threshold tests. Add one contrasting production-stack case inside the existing propagation test and retain all existing inputs/assertions/oracles. Acceptance is runtime behavior; source-text checks are not used as behavioral evidence.
- Exact functions and assertions:
  - `beading_factory_passes_split_middle_thresholds`: retain the two direct assertions `(params.wall_split_middle_threshold - 0.99).abs() < TOLERANCE` and `(params.wall_add_middle_threshold - 0.99).abs() < TOLERANCE`; document that they pin `Default` seeding independently of factory recomputation.
  - `beading_factory_threshold_propagates_through_full_stack`: retain the default `.99/.99` full optional-stack case and its exact `(stack.get_split_middle_threshold() - 0.99).abs() < TOLERANCE` and `(stack.get_add_middle_threshold() - 0.99).abs() < TOLERANCE` assertions; add a second stack with `min_output_width: 3000.0`, `preferred_bead_width_outer: 5000.0`, `optimal_width: 4000.0`, the same `outer_wall_offset: 300.0` and `print_thin_walls: true` flags, and literal getter assertions against `.20` and `.75` within `TOLERANCE`.
  - `beading_factory_threshold_clamp_bounds_are_canonical`: retain `min_output_width: 100.0` and `min_output_width: 100_000.0` FRU fixtures, with exact `(stack_lo.get_split_middle_threshold() - 0.01).abs() < TOLERANCE`, `(stack_lo.get_add_middle_threshold() - 0.025).abs() < TOLERANCE`, and `(stack_hi.get_split_middle_threshold() - 0.99).abs() < TOLERANCE` assertions; document its independent boundary coverage.
- Rejected alternatives and reasons:
  - Replace the original propagation case with the contrasting case: rejected because it deletes the approved default full-stack witness.
  - Merge or delete the default and clamp tests: rejected because their public-default and boundary inputs are distinct regression contracts under ADR-0064.
  - Compute expected `.20/.75` by invoking production formula code or getters: rejected as a self-referential oracle under `docs/22_test_quality.md` §2.1.
  - Edit `crates/slicer-core/src/beading/factory.rs` or add a public threshold API: rejected; the row is a test-only remediation.

## Files in Scope (read + edit)

- `crates/slicer-core/tests/beading/factory.rs` - role: sole source-test edit; expected change: KEEP rationales plus additive contrasting propagation case and literal `.20/.75` assertions.
- `docs/specs/test-quality-remediation-plan.md` - role: implementation-only Step 2 bookkeeping; expected change: update only the §7 `core` ledger row after Step 1 passes, appending to accumulated content without freezing other rows.

## Read-Only Context

- `crates/slicer-core/src/beading/factory.rs` - `BeadingFactoryParams`, its `Default`, and `BeadingStrategyFactory::create_stack`; purpose: verify default seeds versus recomputed clamp values and production immutability.
- `crates/slicer-core/src/beading/mod.rs` - `BeadingStrategy::get_split_middle_threshold` and `get_add_middle_threshold`; purpose: verify object-safe getter contract.
- `crates/slicer-core/src/beading/distributed.rs` - constructor storage and getter returns; purpose: verify the factory's destination.
- `crates/slicer-core/src/beading/redistribute.rs`, `widening.rs`, `outer_wall_inset.rs`, and `limited.rs` - forwarding methods; purpose: verify the full optional chain is the propagation surface.
- `crates/slicer-core/Cargo.toml` - `beading_factory` `[[test]]` registration and feature declarations; purpose: verify the exact `--test` target and that the target is ungated while packet commands remain explicitly feature-correct.
- `docs/specs/test-quality-remediation-plan.md` - §5.1, §6, §7, row #8, and continuation approval; purpose: authoritative scope, log discipline, and ledger contract; ranged reads only.
- `docs/22_test_quality.md`, `docs/21_data_defaults_and_fixtures.md`, and `docs/15_config_keys_reference.md` - relevant sections only; purpose: oracle, KEEP, FRU, and width/default obligations.

## Out-of-Bounds Files

- `crates/slicer-core/src/beading/factory.rs` and every other production source file - read-only; no production/default/formula edits.
- `crates/slicer-core/tests/fixtures/beading/factory_orca_reference.json` and all other fixtures - read-only; no fixture re-recording.
- `crates/slicer-core/Cargo.toml` - read-only; no target or feature registration changes.
- `docs/specs/test-quality-remediation-plan.md` Packet Queue and every ledger row other than `core` - parent/step boundary; no queue update during generation.
- `docs/07_implementation_status.md`, other packet directories, `target/` except delegated log reads, lockfiles, generated code, and vendored dependencies - never edit or load directly.
- `OrcaSlicerDocumented/` - delegate; never load in the author or implementer context.

## Expected Sub-Agent Dispatches

- Question: run the three exact filtered tests and return anchored `running 1 test`, named `... ok`, and `test result: ok. 1 passed; 0 failed;` lines; scope: delegated Cargo commands and `target/test-output.log`; return: `FACT`; purpose: prove each threshold witness executes.
- Question: run the full `beading_factory` target and return its positive `test result` line; scope: delegated Cargo command and `target/test-output.log`; return: `FACT`; purpose: prove neighboring factory tests remain green without freezing a count.
- Question: run `cargo xtask check-literals` and `cargo xtask check-test-quality --report crates/slicer-core/tests/beading/factory.rs`; scope: touched test file and command output; return: `FACT`; purpose: prove FRU and test-quality gates.
- Question: run the real-heading section-anchor check, parse the accumulated §7 `core` row, and exercise the named in-memory controls; scope: `docs/specs/test-quality-remediation-plan.md` §7 and doc-only strings; return: `FACT`; purpose: prove this packet's evidence without freezing mutable non-`core` rows or accepting queue-only/wrong-column evidence.

## Data and Contract Notes

- IR/manifest contracts: none; `BeadingFactoryParams` is an internal core parameter bundle and no field is added or changed.
- WIT boundary: none; no host/module or component type is touched.
- Determinism/scheduler constraints: none beyond fixed literal inputs and deterministic getter results; no pipeline or scheduler execution is introduced.

## Locked Assumptions and Invariants

- All three threshold tests remain registered and named exactly as they are today; none is retired, renamed, or moved.
- The original propagation case's `.99/.99` assertions remain alongside the additive `.20/.75` case; the latter is not a replacement.
- `.20` is the independent expected result of `(2 × 3000 / 5000) - 1`; `.75` is the independent expected result of `3000 / 4000`; neither is calculated by implementation code or a test-local production mirror.
- The default `.99/.99` field assertions remain separate from `create_stack`'s local recomputation, and the clamp test remains a separate lower/interior/upper matrix.
- No production symbol, public signature, test target, fixture, schema, WIT type, or manifest entry changes.
- The §7 implementation ledger remains `partial`; this packet does not close the core wave or claim prior/later packets are implemented.

## Risks and Tradeoffs

- The propagation test now contains two cases in one function, so a failure in either case reports through one exact filter; independent literals prevent the default case from masking a hard-coded `.99` implementation, and later code review verifies the authored ordinary Rust assertions.
- The default seed fields are not the values consumed by `create_stack`; comments must make that deliberate distinction explicit rather than suggesting field propagation.
- The explicit `--features host-algos` form is retained for core-wave discipline even though `beading_factory` has no `required-features` entry; omitting it would make this packet inconsistent with §6, not more correct.
- The target is host-only and no guest freshness check is needed; a future guest test failure must not be attributed to this packet without the repository freshness gate.

## Context Cost Estimate

- Aggregate: `S`
- Largest step: `S`
- Highest-risk dispatch and required return format: delegated exact-test logs and §7 accumulated-row parser/synthetic controls; `FACT` with only anchored result lines or named doc-only control outcomes.

## Open Questions

None.

# Implementation Plan: core-beading-threshold-review

## Execution Rules

- Work one atomic step at a time; map every step to `core/DUP-CORE (beading factory)`.
- Preserve all three existing tests and their existing inputs/oracles before adding the approved contrasting case; do not replace a witness with a stronger-looking substitute.
- Every cargo **test** command is delegated, opens with `set -euo pipefail; mkdir -p target`, tees combined output to `target/test-output.log`, and is adjudicated from that log without re-running. Check, clippy, and xtask gates use compact-output wrappers with dedicated logs.
- Generation does not edit the remediation plan or source; these are implementation-step boundaries described by this packet.

## Steps

### Step 1: Add the contrasting propagation witness and KEEP rationales

- Task IDs: `core/DUP-CORE (beading factory)`
- Objective: edit only `crates/slicer-core/tests/beading/factory.rs`; retain all three selected tests with every existing input and oracle, add the second propagation case, and document why all three cases remain distinct. Acceptance is runtime production behavior, not a source-text roster.
- Precondition: `core-geometry-dup-review` is `generated` only and exports nothing; the three named tests and the explicit `beading_factory` target registration exist; `BeadingFactoryParams` literals use FRU. Re-derive names from the file before editing and do not assume any implementation count.
- Postcondition: the default test independently compares both default fields to literal `.99` within `TOLERANCE`; the propagation test executes the original default-width `.99/.99` case and the additive `3000/5000/4000` `.20/.75` case through the same optional stack, with fixed `TOLERANCE` assertions; the clamp test retains `100.0`/`100_000.0` fixtures and fixed `.01/.025/.99` runtime assertions within `TOLERANCE`; no selected test input/oracle is removed and no new target or fixture exists. Later code review verifies these authored assertions and any correct local renamings.
- Assertion-preservation contract for code review: retain `(params.wall_split_middle_threshold - 0.99).abs() < TOLERANCE` and `(params.wall_add_middle_threshold - 0.99).abs() < TOLERANCE`; retain the original stack's two `.99` getter comparisons; retain `(stack_lo.get_split_middle_threshold() - 0.01).abs() < TOLERANCE`, `(stack_lo.get_add_middle_threshold() - 0.025).abs() < TOLERANCE`, and `(stack_hi.get_split_middle_threshold() - 0.99).abs() < TOLERANCE`. Add ordinary Rust runtime assertions for the contrasting stack's split `.20` and add `.75`; do not add a source-grep or test-of-a-test harness.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-core/tests/beading/factory.rs` - whole file; source-test edit surface and exact existing bodies.
  - `crates/slicer-core/src/beading/factory.rs` - `BeadingFactoryParams` and `BeadingStrategyFactory::create_stack` blocks only; production formulas/defaults are read-only.
  - `crates/slicer-core/src/beading/mod.rs`, `distributed.rs`, `redistribute.rs`, `widening.rs`, `outer_wall_inset.rs`, `limited.rs` - threshold getter/storage/forwarding methods only.
  - `crates/slicer-core/Cargo.toml` - `beading_factory` target stanza and feature declarations only.
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 and §6 only; no queue or ledger edit in this step.
- Files allowed to edit (at most 3):
  - `crates/slicer-core/tests/beading/factory.rs`
- Files explicitly out of bounds:
  - All production source, `Cargo.toml`, JSON fixtures, other tests, and other packet directories.
  - `docs/specs/test-quality-remediation-plan.md`, including §7 and Packet Queue.
  - `docs/07_implementation_status.md`, `target/` except delegated log reads, lockfiles, generated code, vendored dependencies, and `OrcaSlicerDocumented/`.
- Blast-radius discipline:
  - Not triggered: no struct field, schema constant, version constant, target registration, or public API changes. The existing `BeadingFactoryParams` literals remain FRU-compliant; no new watched-type literal is introduced.
- Expected sub-agent dispatches:
  - Question: run AC-1's exact default test and return the three anchored result lines; scope: delegated Cargo command and `target/test-output.log`; return: `FACT`.
  - Question: run AC-2's exact propagation test and AC-3's exact clamp test; scope: delegated Cargo commands and `target/test-output.log`; return: `FACT` with one result block per command.
  - Question: run `cargo xtask check-literals` and `cargo xtask check-test-quality --report crates/slicer-core/tests/beading/factory.rs`; scope: touched test file; return: `FACT`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §5.1 and §6, ranged read.
  - `docs/22_test_quality.md` - §§1–4, ranged read.
  - `docs/21_data_defaults_and_fixtures.md` - §§1, 4, and 6, ranged read.
- OrcaSlicer parity: use only the delegated full paths/functions listed in `packet.spec.md` and `requirements.md`; no direct read.
- Verification:
  - Run AC-1, AC-2, AC-3, and AC-4 exactly as written in `packet.spec.md`.
  - Run the wrapped `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check-literals`, and `cargo xtask check-test-quality --report crates/slicer-core/tests/beading/factory.rs` gate commands through delegated workers; return bounded `FACT` results from their dedicated logs.
  - Read `target/test-output.log` after each cargo test before launching another; read the dedicated check/clippy/xtask log after its gate; never re-run only to obtain omitted output.
- Exit condition: all three exact filtered tests pass, the full registered target has a positive passing result with zero failures, and all four packet gates are clean. Runtime failures of the fixed `.99`, `.20/.75`, or `.01/.025/.99` behaviors falsify this step and require redesign before Step 2; no source parser is used to pretend to prove test implementation.

### Step 2: Record the scoped §7 core ledger row

- Task IDs: `core/DUP-CORE (beading factory)`
- Objective: after Step 1 passes, update only the `core` row in §7 of `docs/specs/test-quality-remediation-plan.md` to record this packet's strengthened propagation and three surviving threshold witnesses, with state `partial` and the current remaining-core markers.
- Precondition: Step 1 exit condition holds. Read the real §7 heading and current accumulated `core` row immediately before editing; preserve prior cell content and do not assume an old `core` state or mutable non-`core` row contents.
- Postcondition: AC-5 passes against the real plan; the `core` row is the sole intended ledger edit, has exactly six cells and state `partial` (optionally parenthesized detail), and contains meaningful accumulated changed/surviving/feature-correct validation/remaining-gap evidence in the correct columns. The Packet Queue and all non-`core` rows are not edited, but their contents are not frozen by the checker.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/test-quality-remediation-plan.md` - §7 Ledger only, bounded by the suffixed heading anchor `^## 7[.] Ledger[ ]+...` and `^## Packet Queue`.
- Files allowed to edit (at most 3):
  - `docs/specs/test-quality-remediation-plan.md` - only the `core` row in §7.
- Files explicitly out of bounds:
  - Every non-`core` ledger row, the §7 header, and the Packet Queue.
  - All source/tests/fixtures, `docs/07_implementation_status.md`, all packet directories, `target/`, lockfiles, generated code, and `OrcaSlicerDocumented/`.
- Blast-radius discipline:
  - Not triggered: Markdown ledger bookkeeping adds no code field or schema/version constant.
- Expected sub-agent dispatches:
  - Question: extract the post-edit `core` row and confirm the diff is scoped to that row without asserting non-`core` contents; scope: plan §7; return: `SNIPPETS` for the row plus `FACT` for scope.
  - Question: exercise the exact AC-5 parser against a valid copy using the real heading and accumulated prior content, then named in-memory controls for queue-only/wrong-column evidence, `bad-state-open`, `bad-state-done`, `missing-prior-flag`, `missing-prior-gap`, `missing-changed-evidence`, `missing-surviving-evidence`, `missing-validation-evidence`, and `duplicate-core`; scope: doc-only strings; return: `FACT`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/test-quality-remediation-plan.md` - §7 ledger conventions and remaining-core queue, ranged read.
  - `docs/22_test_quality.md` - §1/§4 survivor and claim discipline, ranged read.
- OrcaSlicer refs:
  - None; this step is doc-only.
- Verification:
  - Run the doc-impact section-anchor grep and AC-5's exact parser from `packet.spec.md` against the real plan; the parser must match the real suffixed heading.
  - Run the same parser logic in memory against a valid copy built from the real heading plus accumulated prior content and all named invalid controls; failures must raise nonzero without writing files or invoking Cargo.
  - Review the implementation diff to confirm only the `core` ledger row changed; do not compare or freeze mutable non-`core` row contents.
- Exit condition: AC-5 returns `LEDGER-PASS`, every named synthetic invalid control is rejected, the real-heading positive copy is accepted, and the implementation diff is scoped to the `core` row with prior `core` content retained.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | one existing test file; three exact filters plus bounded gates |
| Step 2 | S | one section-anchored Markdown row and in-memory controls; no Cargo |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every executable AC command returns PASS; cargo test output follows the test-log rule, while check/clippy/xtask output uses the dedicated compact logs.
- The implementation worker records the §7 ledger through the narrow worker dispatch; no `docs/07` TASK mapping exists under the approved exemption.
- No queue or packet status is changed by implementation; the parent batch controller updates row #8 only after independent preflight and generation bookkeeping.
- `packet.spec.md` is ready for `status: implemented` only after implementation acceptance; current packet status remains `draft`.

## Acceptance Ceremony

- Re-dispatch AC-1 through AC-5 plus the packet-level gates; read all findings from their tee'd logs.
- Record packet-local risk: the additive contrasting case proves non-default forwarding, while the original default case separately protects the `.99` seed/recomputed-default distinction.
- Confirm context stayed within the standard band; no workspace suite or guest build is required for this test-only packet.

All `cargo check` and `cargo clippy` invocations use `--all-targets`; targeted `cargo test` invocations use the exact registered `beading_factory` test target and are not forced through `--all-targets`. Cargo test commands use `target/test-output.log`; check, clippy, and xtask gates use dedicated compact logs.

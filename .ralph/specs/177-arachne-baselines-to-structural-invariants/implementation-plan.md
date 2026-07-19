# Implementation Plan: 177-arachne-baselines-to-structural-invariants

## Execution Rules

- Work one atomic step at a time; this packet has no `TASK-###` mapping.
- Use TDD: add the named falsifying assertion, run its narrow command, then
  replace the snapshot path or implementation and rerun the command.
- Steps `1a -> 1b -> 2 -> 3 -> 4` are ordered. Steps 5 and 6 follow Step 4;
  Step 7 is last because it closes the deviation only after all gates pass.
- Every cargo test invocation is feature-qualified where required and tee'd to
  `target/test-output.log`. `cargo check` and `cargo clippy` use
  `--all-targets`; target-specific tests do not add a contradictory all-targets
  flag.
- Never invoke a recorder against a deleted snapshot. Never load the deleted
  JSON files.

## Step 1a: Correct production defaults

- Task IDs: none; backlog source `D-112-SELFCAPTURED-BASELINES`.
- Objective: set both production `max_bead_count` defaults to literal `10` and
  explain canonical even-count parity without claiming an odd branch.
- Precondition: `rg -n 'max_bead_count:\s*9' crates/slicer-core/src/` returns
  the current production inventory.
- Postcondition: the production inventory is empty; both adjacent comments are
  accurate.
- Files allowed to read:
  - `crates/slicer-core/src/arachne/pipeline.rs`, default ±40 lines.
  - `crates/slicer-core/src/beading/factory.rs`, default ±40 lines.
- Files allowed to edit:
  - `crates/slicer-core/src/arachne/pipeline.rs`.
  - `crates/slicer-core/src/beading/factory.rs`.
- Expected dispatches:
  - Orca fact for `WallToolPaths.cpp::generate` and
    `LimitedBeadingStrategy.cpp::compute`; return `FACT` <=5 lines.
- Context cost: S.
- Verification: `rg -n 'max_bead_count:\s*9' crates/slicer-core/src/`; `cargo check -p slicer-core --all-targets`.
- Exit condition: both production defaults are `10`, comments match the
  canonical rationale, and the production inventory is empty.

## Step 1b: Correct test helpers

- Task IDs: none.
- Objective: change every surviving odd test helper to `10`, re-deriving the
  inventory rather than trusting this plan.
- Precondition: Step 1a is complete and
  `rg -n 'max_bead_count:\s*9' crates/slicer-core/tests/` returns the surface.
- Postcondition: `rg -n 'max_bead_count:\s*9' crates/slicer-core/` returns no
  matches; no already-correct helper is edited.
- Files allowed to read/edit: exactly the files returned by the inventory,
  each at a time; `generate_toolpaths.rs` and `bead_count.rs` are read-only if
  they already carry `10`.
- Expected dispatches: affected test-binary result lines; return `FACT` with
  one line per binary.
- Context cost: S.
- Verification: `rg -n 'max_bead_count:\s*9' crates/slicer-core/`; affected
  `cargo test -p slicer-core --features host-algos --test <binary>` commands.
- Exit condition: crate-wide inventory is empty and affected binaries retain
  their tests; changed behavior is recorded rather than reverted.

## Step 2: Replace core JSON consumers with in-memory structural cases

- Task IDs: none.
- Objective: remove serialized snapshot dependence from centrality, propagation,
  bead-count, toolpath, and core invariant tests while preserving scenario
  coverage through source geometry.
- Precondition: Steps 1a and 1b complete; the eight JSON paths are inventoried.
- Postcondition: named structural tests exist and pass; no core test loads or
  writes an Arachne JSON fixture; the eight JSON files are deleted.
- Files allowed to read:
  - `crates/slicer-core/tests/centrality.rs`, fixture helpers/assertions ±60 lines.
  - `crates/slicer-core/tests/propagation.rs`, fixture helpers/assertions ±80 lines.
  - `crates/slicer-core/tests/bead_count.rs`, fixture helpers/assertions ±60 lines.
  - `crates/slicer-core/tests/generate_toolpaths.rs`, fixture helpers/assertions ±60 lines.
  - `crates/slicer-core/tests/arachne_invariants.rs`, helper block ±80 lines.
  - JSON files only through bounded filename/key searches before deletion.
- Files allowed to edit:
  - The five Rust test files above.
  - The eight exact JSON files listed in `design.md` for deletion.
- Expected dispatches: none beyond targeted test commands.
- Context cost: M.
- Verification:
  - `cargo test -p slicer-core --features host-algos --test centrality`.
  - `cargo test -p slicer-core --features host-algos --test propagation`.
  - `cargo test -p slicer-core --features host-algos --test bead_count`.
  - `cargo test -p slicer-core --features host-algos --test generate_toolpaths`.
  - `cargo test -p slicer-core --features host-algos --test arachne_invariants`.
- Exit condition: every assertion names a structural class; no snapshot path is
  active; the named AC-7 tests each collect exactly one test.

## Step 3: Extract runtime harness and measure source coverage

- Task IDs: none.
- Objective: create a standalone runtime measurement seam over the five Arachne
  STL coverage subjects and record aligned classic/Arachne ratios.
- Precondition: Steps 1a, 1b, and 2 complete; source STL paths and module
  selection are verified.
- Postcondition: the shared harness runs both generators on identical source
  inputs, joins the same global Z planes, reports X extents, repeats each
  subject, and fills every measured row in `design.md`.
- Files allowed to read:
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs`, public
    capture helpers and fixture loading only.
  - `crates/slicer-runtime/tests/common/mod.rs`.
  - `crates/slicer-runtime/tests/fixtures/perimeter_parity/<subject>/config.json`
    and STL filenames for the five subjects.
  - `crates/slicer-scheduler/src/execution_plan.rs`, wall-generator selection
    symbols only.
  - `crates/slicer-runtime/src/run.rs`, loader call-site ±30 lines.
- Files allowed to edit:
  - `crates/slicer-runtime/tests/common/perimeter_harness.rs` (new).
  - `crates/slicer-runtime/tests/common/mod.rs`.
  - `crates/slicer-runtime/tests/arachne_structural_invariants.rs` (new).
  - `design.md`, Measured Coverage Baseline section only.
- Expected dispatches:
  - classic/Arachne selection locations; return `LOCATIONS` <=20.
  - measurement run; return `FACT` with five ratios and repeat deltas only.
- Context cost: M.
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test arachne_structural_invariants -- coverage_subjects_repeat_and_record_ratios --exact --nocapture 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. 1 passed'`.
  - `rg` checks for five numeric rows, observed minimum, margin, threshold,
    and repeatability prose in `design.md`.
- Exit condition: all five rows are measured at aligned Z; margin equals the
  maximum repeat delta and is no greater than `0.02`; otherwise stop blocked.
  If the derived threshold admits `0.668`, stop blocked and do not tune.

## Step 4: Encode coverage floor and falsify D5

- Task IDs: none.
- Objective: encode Step 3's numeric threshold in the standalone runtime test,
  assert the five-subject floor, and prove D5 discrimination independently of
  live geometry.
- Precondition: Step 3's table is filled; no code may invent or remeasure the
  threshold here.
- Postcondition: `coverage_threshold_rejects_d5_broken_ratio`,
  `coverage_invariant_rejects_synthetic_d5_regression`,
  `arachne_coverage_floor_over_source_corpus`, and the accept-0.990 test pass;
  failures include fixture, Z, extents, ratio, and threshold.
- Files allowed to read:
  - `design.md`, Measured Coverage Baseline only.
  - `crates/slicer-runtime/tests/common/perimeter_harness.rs`.
  - `crates/slicer-core/tests/arachne_d5_taper_coverage.rs`, parser shape only.
- Files allowed to edit:
  - `crates/slicer-runtime/tests/arachne_structural_invariants.rs`.
- Expected dispatches: targeted runtime test result; return `FACT` or bounded
  failure snippets.
- Context cost: M.
- Verification: the AC-4, AC-5, AC-N1 commands and the complete standalone test
  binary.
- Exit condition: threshold constant equals `design.md`; 0.668 rejects, 0.990
  admits, and all five source subjects pass.

## Step 5: Convert tapered-wedge parity and delete its snapshot

- Task IDs: none.
- Objective: remove every self-captured perimeter JSON and make the perimeter
  suite source-geometry structural instead of snapshot-based.
- Precondition: Steps 3 and 4 pass; no recorder has been invoked.
- Postcondition: `tapered_wedge_parity_is_structural` passes in the standalone
  binary; existing perimeter integration tests no longer load any expected IR
  snapshot or recorder; all eleven perimeter snapshot paths are absent.
- Files allowed to read:
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs`, fixture
    definitions, structural assertions, and snapshot call sites ±100 lines.
  - `crates/slicer-runtime/tests/common/perimeter_harness.rs`.
- Files allowed to edit:
  - `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (remove
    `load_expected_perimeters`, snapshot comparison helpers, six classic
    snapshot tests, five Arachne snapshot calls, and all recorder functions;
    retain structural source-fixture tests).
  - `crates/slicer-runtime/tests/arachne_structural_invariants.rs`.
  - Every `expected_perimeter_ir.json` under
    `crates/slicer-runtime/tests/fixtures/perimeter_parity/` (deletion only).
- Expected dispatches: integration and standalone test result lines; return
  `FACT` with evidence.
- Context cost: M.
- Verification: `cargo test -p slicer-runtime --test arachne_structural_invariants -- tapered_wedge_parity_is_structural --exact`; `cargo test -p slicer-runtime --test integration perimeter_parity`; AC-8; AC-N3.
- Exit condition: paired structural test passes, no expected-IR loader or
  recorder remains, and all eleven deleted files stay absent.

## Step 6: Rehome red tests and correct the runtime header

- Task IDs: none.
- Objective: make red test paths describe their stage while preserving every
  body and test name; correct only the stale runtime header.
- Precondition: capture the pre-move names immediately before moving:
  `rm -f /tmp/pnp-177-pre-test-names.txt && cargo test -p slicer-core --features host-algos -- --list 2>/dev/null | rg ': test$' | sort > /tmp/pnp-177-pre-test-names.txt`.
- Postcondition: all nine exact old paths are absent, all new paths exist, and
  the collected name set equals the captured file.
- Files allowed to read: the nine red files' headers/test names and the first
  30 lines of `crates/slicer-runtime/tests/arachne_parity.rs`.
- Files allowed to edit/move: exactly the nine old-to-new mappings in
  `design.md`, plus the runtime header file.
- Expected dispatches: pre/post test-list result and D-104f status; return
  `FACT` with counts/names only.
- Context cost: S.
- Verification: AC-9, AC-10, and `cargo test -p slicer-core --features host-algos --test arachne_invariants`.
- Exit condition: no old path remains, names are preserved, and D-104f remains
  the sole named open red runtime case.

## Step 7: Update authoritative docs and close the deviation

- Task IDs: none.
- Objective: record the measured threshold and corrected artifact history in
  the recovery doc, ADR-0042, D-112, and the glossary.
- Precondition: Steps 1-6 and every pipe-suffixed AC pass.
- Postcondition: documentation states the five source coverage subjects, the
  repeatability margin, the deleted snapshots, and the corrected Track B state.
- Files allowed to read:
  - `docs/specs/arachne-parity-recovery.md`, Track B sections only.
  - `docs/adr/0042-arachne-parity-structural-invariants-over-fixtures.md`,
    invariant/bead-width/Consequences sections only.
  - `docs/DEVIATION_LOG.md`, D-112 row only.
  - `CONTEXT.md`, new glossary terms only.
- Files allowed to edit: those four files and no other docs.
- Expected dispatches: bounded FACT for D-112 current status and Track B/ADR
  fact checks; never return long rows.
- Context cost: S.
- Verification: the four Doc Impact greps plus AC-N3 and full packet command registry.
- Exit condition: D-112 is `Closed` only after all gates pass; ADR-0042 and the
  recovery doc agree with the table; glossary terms exist.

## Packet Completion Gate

- All seven steps and exits complete.
- Every pipe-suffixed AC passes with a nonzero test count where applicable.
- No deleted JSON path is loaded or recreated.
- The threshold constant equals the measured table and rejects D5 `0.668`.
- D-112 is closed, ADR-0042 is instantiated, recovery prose is corrected, and
  D-104f remains open.
- `cargo check --workspace --all-targets` and
  `cargo clippy --workspace --all-targets -- -D warnings` pass.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and every packet-level verification.
- Run `cargo xtask build-guests --check` before attributing any guest failure.
- Run the full suite once via `cargo xtask test --workspace --summary`,
  dispatched as a bounded `FACT` result.
- Confirm no implementation step introduced a new snapshot or recorder path.

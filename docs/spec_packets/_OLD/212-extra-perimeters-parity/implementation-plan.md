# Implementation Plan: 212-extra-perimeters-parity

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Red tests — pin arachne's `extra_perimeters` gap and the cross-generator equality

- Task IDs: `TASK-328`
- Objective: extend `crates/slicer-runtime/tests/integration/extra_perimeters_config_tdd.rs` with the five arachne/cross-generator tests named in `packet.spec.md` AC-1, AC-2, AC-3, AC-N1, AC-N2, and observe them FAIL for the right reason (arachne ignoring `extra_perimeters`), recording the exact observed counts.
- Precondition: `cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd` currently reports `2 passed` (the two classic tests) and the file contains no arachne reference.
- Postcondition: the same command reports `2 passed; 3 failed` or `2 passed; 4 failed` — AC-2's zero-case (`arachne_extra_perimeters_zero_is_noop`) and AC-N1's explicit-override case are expected to pass already, since neither depends on the fix. AC-1, AC-3 and AC-N2 must fail with an `assertion \`left == right\`` showing the pre-fix count. Those pre-fix numbers are recorded verbatim in the step's notes.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/extra_perimeters_config_tdd.rs` - whole file (under 80 lines)
  - `modules/core-modules/arachne-perimeters/tests/alternate_extra_wall_tdd.rs` - whole file (under 130 lines); source of the 20 mm square / 1.0 mm bead fixture, the `wall_loop_count` driver shape, and the measured `emitted == max_bead_count / 2` mapping
  - `modules/core-modules/arachne-perimeters/src/lib.rs` - the `arachne_params_from_config` `max_bead_count` derivation window only (the `max_bead_count_explicit` / `wall_count` reads and the following `match`)
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/integration/extra_perimeters_config_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/**` (no production edit in this step)
  - `crates/slicer-runtime/tests/integration/main.rs` (the `mod extra_perimeters_config_tdd;` line already exists — verified; do not touch)
  - `docs/**`
  - `OrcaSlicerDocumented/**`
- Blast-radius discipline: not applicable — no struct field and no schema/version constant is added or changed by this step.
- Expected sub-agent dispatches:
  - Question: "Run `mkdir -p target && cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd 2>&1 | tee target/test-output.log`; report the `^test result` line and, for every failing test, its name plus the exact `assertion \`left == right\`` left/right values."; scope: `crates/slicer-runtime`; return: `FACT` (<=5 lines)
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - `rg` for the `extra_perimeters` owner row only; do not read the file
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegate; never load. Behaviour already stated in `requirements.md`.
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd 2>&1 | tee target/test-output.log | rg '^test result'` - FACT: must show failures, and the failing names must be exactly the arachne/cross-generator ones
  - `cargo check --workspace --all-targets` - FACT pass/fail; the new tests must compile
- Exit condition: at least `arachne_extra_perimeters_bonus_adds_to_wall_count`, `extra_perimeters_survives_wall_generator_switch`, and `arachne_extra_perimeters_composes_with_alternate_extra_wall` fail with a numeric count mismatch (NOT a compile error, NOT a `ModuleError`, NOT a zero-wall degenerate-geometry result), and the two pre-existing classic tests still pass. If any arachne test fails for a geometry reason (0 walls emitted), the fixture is wrong — fix the fixture, not the assertion, before proceeding.

### Step 2: Fold `extra_perimeters` into arachne's auto-derived `max_bead_count`

- Task IDs: `TASK-328`
- Objective: register `[config.schema.extra_perimeters]` in the arachne manifest, read it in `arachne_params_from_config` with classic's exact expression, change the auto-derive arm to `2 * (wall_count + extra_perimeters)`, and add the matching `ARACHNE_FALLBACKS` row so the exhaustive reconcile guard stays green.
- Precondition: Step 1's exit condition met; the arachne/cross-generator tests are red with recorded numeric mismatches.
- Postcondition: `cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd` reports all 7 tests passing; `manifest_default_reconcile_tdd` passes; `alternate_extra_wall_tdd` still reports `2 passed`; `cargo xtask build-guests --check` reports clean (after a rebuild if needed).
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/classic-perimeters/src/lib.rs` - the `base_wall_count` window in `run_perimeters` ONLY: the `extra_perimeters` read/addition, the DEV-125 `alternate_extra_wall` four-conjunct guard, and the `only_one_wall_first_layer` clamp
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` - the `[config.schema.extra_perimeters]` block only
  - `crates/slicer-ir/src/slice_ir.rs` - `ConfigView::from_declared` only
  - `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs` - the `ARACHNE_FALLBACKS` constant only; do NOT read `CLASSIC_FALLBACKS` or the module doc
- Files allowed to edit (at most 3):
  - `modules/core-modules/arachne-perimeters/src/lib.rs`
  - `modules/core-modules/arachne-perimeters/arachne-perimeters.toml`
  - `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/classic-perimeters/**` (read-only; classic behaviour must not move)
  - `crates/slicer-runtime/tests/integration/extra_perimeters_config_tdd.rs` (frozen after Step 1 — the tests must pass unmodified)
  - `crates/slicer-schema/wit/**` (no WIT change)
  - `docs/**`
  - `OrcaSlicerDocumented/**`
- Blast-radius discipline: no struct field and no schema/version constant. The one enumeration-style blast radius IS budgeted here: `crates/slicer-runtime/tests/integration/manifest_default_reconcile_tdd.rs`'s `ARACHNE_FALLBACKS` is checked for set-equality against the manifest's `[config.schema]` table in BOTH directions by `assert_exhaustive_reconcile`, so adding the manifest key without the row `("extra_perimeters", Int(0))` fails the suite. It is in "Files allowed to edit" for exactly that reason; it must not be discovered by a follow-up `cargo check`.
- Expected sub-agent dispatches:
  - Question: "Run `cargo xtask build-guests --check`; report `clean` or every `STALE:` line verbatim."; scope: repo root; return: `FACT`
  - Question: "Run `mkdir -p target && cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd 2>&1 | tee target/test-output.log`; report the `^test result` line and any failing test name with its assertion values."; scope: `crates/slicer-runtime`; return: `FACT` (<=5 lines)
  - Question: "Run `mkdir -p target && cargo test -p slicer-runtime --test integration manifest_default_reconcile_tdd 2>&1 | tee target/test-output.log`; report the `^test result` line and the panic message on failure."; scope: `crates/slicer-runtime`; return: `FACT`
  - Question: "Run `mkdir -p target && cargo test -p arachne-perimeters --test alternate_extra_wall_tdd 2>&1 | tee target/test-output.log`; report the `^test result` line."; scope: `modules/core-modules/arachne-perimeters`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` - delegated SUMMARY only, and only if the `[config.schema.<key>]` field set for an `int` key is in doubt; `classic-perimeters.toml`'s own block is the working template
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Arachne/WallToolPaths.cpp` and `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` - delegate; never load. The `max_bead_count = 2 * inset_count` / `coord_t(loop_number + 1)` relation is already stated in `requirements.md` and is what the new comment must cite by function name.
- Verification:
  - `cargo xtask build-guests --check` - FACT clean; MUST run after the `modules/core-modules/arachne-perimeters/**` edit and before attributing any failure to the change
  - `mkdir -p target && cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd 2>&1 | tee target/test-output.log | rg '^test result'` - FACT: `7 passed`
  - `mkdir -p target && cargo test -p slicer-runtime --test integration manifest_default_reconcile_tdd 2>&1 | tee target/test-output.log | rg '^test result'` - FACT pass/fail
  - `mkdir -p target && cargo test -p arachne-perimeters --test alternate_extra_wall_tdd 2>&1 | tee target/test-output.log | rg '^test result'` - FACT: `2 passed` (AC-N3 no-regression)
  - `test "$(rg -A6 -N '^\[config\.schema\.extra_perimeters\]\r?$' modules/core-modules/arachne-perimeters/arachne-perimeters.toml | rg -c '^\s*(type\s*=\s*"int"|default\s*=\s*0|min\s*=\s*0|max\s*=\s*10)\s*$')" = "4" && echo PASS` - FACT `PASS`
- Exit condition: all seven tests in `extra_perimeters_config_tdd` pass with `extra_perimeters_config_tdd.rs` unmodified since Step 1, `manifest_default_reconcile_tdd` passes, `alternate_extra_wall_tdd` reports `2 passed`, and `build-guests --check` is clean. If any arachne test still fails, the falsifying check is whether the auto-derive arm actually computes an EVEN `max_bead_count` — an odd cap changes `LimitedBeadingStrategy`'s branch and invalidates the `emitted == max_bead_count / 2` mapping.

### Step 3: Regenerate the generated doc tables

- Task IDs: `TASK-328`
- Objective: bring `docs/15_config_keys_reference.md`'s generated config-key table (and any other generated section touched by the new manifest key) back in sync via `cargo xtask gen-config-docs` (the generator that owns doc 15's `BEGIN GENERATED: module-config-keys` block; `cargo xtask check-deviations --check` only VERIFIES doc 15 and regenerates doc 07's Open Deviation Map), with zero hand edits.
- Precondition: Step 2's exit condition met; the arachne manifest declares `extra_perimeters`.
- Postcondition: `cargo xtask check-deviations --check` exits `0` and `docs/15_config_keys_reference.md` contains an `extra_perimeters` row whose owner column is `arachne-perimeters`, alongside the pre-existing `classic-perimeters` row.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/15_config_keys_reference.md` - `rg`-targeted lines around `extra_perimeters` only; the file is large and mostly generated, never read it in full
- Files allowed to edit (at most 3):
  - `docs/15_config_keys_reference.md` - VIA `cargo xtask gen-config-docs` ONLY; no manual edit to any generated table
- Files explicitly out of bounds:
  - `xtask/**` (the generator itself is not in scope)
  - `docs/config/host-keys.toml` (host keys unaffected)
  - all code files
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: "Run `cargo xtask check-deviations`, then `cargo xtask check-deviations --check`; report the second command's exit code and, if non-zero, its stderr."; scope: repo root; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/15_config_keys_reference.md` - its own header states the generated tables are produced by xtask and that CI fails on drift; honour that, do not hand-edit
- OrcaSlicer refs:
  - None for this step.
- Verification:
  - `cargo xtask check-deviations --check && echo PASS` - FACT `PASS` / exit code
  - `rg -q '^\| .extra_perimeters. \| int \|.*arachne-perimeters' docs/15_config_keys_reference.md && echo PASS` - FACT `PASS`
- Exit condition: `--check` exits `0` AND the arachne-owned `extra_perimeters` row is present. If `--check` still fails, the cause is a hand edit or an unrelated stale generated section — diagnose before proceeding; do not suppress the check.

### Step 4: Split the deviation ledger and file the backlog row

- Task IDs: `TASK-328`
- Objective: record half (a) as closed on `DEV-132`, file a newly-allocated `DEV-###` carrying half (b) (the per-`Surface` modelling divergence) forward with its evidence, add the `TASK-328` backlog line to `docs/07_implementation_status.md`, and regenerate the Open Deviation Map view.
- Precondition: Steps 2 and 3 complete; the code fix is green and the generated config tables are in sync.
- Postcondition: `DEV-132`'s `Status` names the read gap closed by packet 212 and cites the new ID; the new row's `Rationale` states that `Surface::extra_perimeters` (`Surface.hpp`) is written in code only by `PrintObject::make_perimeters` (`PrintObject.cpp`), whose loop body the BBS patch short-circuits with a bare `continue` (so the field is effectively always `0` upstream), and that PnP cannot express per-region config at this seam because `SliceRegionView` (`crates/slicer-sdk/src/views.rs`) carries no `config_id` and `LayerModule::run_perimeters` takes one layer-wide `&ConfigView`; `docs/07_implementation_status.md` has a checked `TASK-328` line; `cargo xtask check-deviations --check` exits `0`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/DEVIATION_LOG.md` - the `DEV-132` row and the last few rows only; do not read the file in full
  - `docs/07_implementation_status.md` - over 300 lines; delegate. Read only the surrounding lines of the backlog section where the new task line goes
- Files allowed to edit (at most 3):
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md` (the `TASK-328` line by hand; the Open Deviation Map view via `cargo xtask check-deviations` only)
- Files explicitly out of bounds:
  - all code files and manifests (frozen after Step 2)
  - `docs/spec_packets/` directories other than this packet's
  - `docs/specs/deviation-remediation-206-212-plan.md` - the orchestrator owns the queue-row status, not this packet
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - Question: "Run `rg -o '^\\| DEV-[0-9]{3}' docs/DEVIATION_LOG.md | sort -u | tail -1` and report the single line."; scope: `docs/DEVIATION_LOG.md`; return: `FACT` (one line). Run this IMMEDIATELY before writing the row — the ID is a ledger fact and packets 206-211 may allocate concurrently.
  - Question: "Run `rg -o 'TASK-[0-9]{3}' docs/07_implementation_status.md | sort -u | tail -3` and report the lines."; scope: `docs/07_implementation_status.md`; return: `FACT`
  - Question: "Run `cargo xtask check-deviations` then `cargo xtask check-deviations --check`; report the exit code."; scope: repo root; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/DEVIATION_LOG.md` - its own header declares itself the single source of truth for deviation status and forbids hand-editing the generated views elsewhere
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Surface.hpp`, `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` - delegate; never load. The evidence to quote is already stated in `requirements.md` §Problem Statement; cite by function name (`PrintObject::make_perimeters`), never by line number.
- Verification:
  - `rg -q 'DEV-132.*[Cc]losed' docs/DEVIATION_LOG.md && rg -q 'Surface::extra_perimeters' docs/DEVIATION_LOG.md && rg -q 'PrintObject::make_perimeters' docs/DEVIATION_LOG.md && echo PASS` - FACT `PASS`
  - `rg -q '^- \[x\] TASK-328' docs/07_implementation_status.md && echo PASS` - FACT `PASS`
  - `cargo xtask check-deviations --check && echo PASS` - FACT `PASS`
- Exit condition: all three verification commands return `PASS`. If `check-deviations --check` fails after the ledger edit, the Open Deviation Map view is stale — regenerate, never hand-edit it.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Two short test files read in full; one test file edited; one bounded test dispatch |
| Step 2 | S | Three edits, all narrow; four bounded FACT dispatches; `ARACHNE_FALLBACKS` is a one-line addition |
| Step 3 | S | One xtask invocation plus two `rg` checks; no file read in full |
| Step 4 | S | Ledger edits only; all reads ranged or delegated |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All four steps and their exit conditions complete.
- Every pipe-suffixed AC command in `packet.spec.md` (AC-1 through AC-8, AC-N1 through AC-N3) returns PASS.
- `docs/07_implementation_status.md` updated through a worker dispatch (Step 4), never a full backlog read.
- No reopened/superseded status transitions apply — this packet supersedes nothing and reopens nothing. DEV-132 is split, not silently retired: half (b) survives as a newly-allocated open row.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command and the three packet-level gate commands (`cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, and the targeted `extra_perimeters_config_tdd` run).
- Re-run `cargo xtask build-guests --check` one final time; the packet edited `modules/core-modules/arachne-perimeters/**`, a guest-WASM input path, so a clean `--check` is a closure precondition.
- Record remaining packet-local risk: the `emitted == max_bead_count / 2` mapping is empirical; and DEV-132 half (b) remains open by design under its newly-allocated ID.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the command form admits it (`cargo test -p <crate> --test <file>` names its target explicitly and does not).

# Implementation Plan: 196-literal-sweep-core-ir-gcode

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation. (For this sweep, "TDD" means: capture the pre-sweep baseline first; the baseline IS the failing/passing contract every later step must preserve.)
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Baseline capture and violation enumeration

- Task IDs: `TASK-318`
- Objective: record the pre-sweep ground truth this packet must preserve, and the violation list that drives Steps 2-4.
- Precondition: packets 194 and 195 are `implemented` (probe: `cargo xtask check-literals --report crates/slicer-ir >/dev/null` exits 0 — the tool defines no `--help` flag, unknown flags exit 2 with USAGE; and `print_entity_base` exists); working tree clean of unrelated edits.
- Postcondition: `target/sweep-196-report.txt`, `target/sweep-196-{slicer-ir,slicer-core,slicer-gcode}-baseline.txt`, `target/sweep-196-assert-baseline.txt`, `target/sweep-196-testattr-baseline.txt` all exist and are non-empty; every baseline `test result` line shows `0 failed`.
- Files allowed to read, with ranges when over 300 lines:
  - none directly — all commands are dispatched; the step consumes only their FACT/LOCATIONS returns.
- Files allowed to edit (at most 3):
  - none (scratch files under `target/` are command outputs, not edits).
- Files explicitly out of bounds:
  - everything; this is a read/record step.
- Blast-radius discipline: not applicable — no struct field or schema constant changes in this packet.
- Expected sub-agent dispatches:
  - Question: run, in order: `mkdir -p target`, `cargo xtask check-literals --report crates/slicer-ir crates/slicer-core crates/slicer-gcode | tee target/sweep-196-report.txt | tail -1`, then for each crate the baseline pipeline `cargo test -p slicer-ir 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result: ' target/test-output.log | sed 's/; finished in .*//' | sort > target/sweep-196-slicer-ir-baseline.txt` (repeat for `slicer-core` with `--features host-algos --no-fail-fast`, and `slicer-gcode`), then `rg -o 'assert(_eq|_ne)?!' crates/slicer-ir crates/slicer-core crates/slicer-gcode | wc -l > target/sweep-196-assert-baseline.txt` and `rg -o '#\[test\]' crates/slicer-ir crates/slicer-core crates/slicer-gcode | wc -l > target/sweep-196-testattr-baseline.txt`. Report the report's summary line, the violating-file list grouped per crate (path + violation count), and whether any baseline `test result` line lacks ` 0 failed`; scope: workspace; return: `LOCATIONS` ≤20 entries per crate + `FACT` for baselines.
- Context cost: `M` (three suite runs; slicer-core is slow)
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY of the conversion rule + waiver format sections.
- OrcaSlicer refs: none.
- Verification:
  - `test -s target/sweep-196-report.txt && test -s target/sweep-196-slicer-ir-baseline.txt && test -s target/sweep-196-slicer-core-baseline.txt && test -s target/sweep-196-slicer-gcode-baseline.txt && echo PASS` - FACT PASS/FAIL
  - `cat target/sweep-196-assert-baseline.txt target/sweep-196-testattr-baseline.txt` - FACT: two integers
- Exit condition: all five scratch files exist; baselines fully green (a pre-existing red test halts the packet — record it as a blocker rather than sweeping over it); violation list in hand. Falsified if the report lists zero violations for all three crates (then the packet is a no-op — stop and report).

### Step 2: slicer-ir sweep

- Task IDs: `TASK-318`
- Objective: zero `check-literals` violations in `crates/slicer-ir` with the suite green.
- Precondition: Step 1 scratch files exist.
- Postcondition: `cargo xtask check-literals crates/slicer-ir` exits 0; `cargo test -p slicer-ir` summary multiset equals baseline.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-196-report.txt` - `grep '^crates/slicer-ir/'` only
  - `crates/slicer-ir/src/slice_ir.rs` - ranged reads around struct definitions and reported violation lines only
  - `crates/slicer-sdk/src/test_support/fixtures.rs` - fixture signatures only
- Files allowed to edit (at most 3; bounded glob counts as one sweep surface):
  - `crates/slicer-ir/tests/**/*.rs` (bounded glob — only files named in the Step-1 report)
  - `crates/slicer-ir/src/slice_ir.rs` (`#[cfg(test)]` mod contents only)
- Files explicitly out of bounds:
  - `crates/slicer-ir/src/**` outside `#[cfg(test)]` mods; `crates/slicer-ir/Cargo.toml` (no sdk dev-dep — design decision)
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: after edits, does `cargo xtask check-literals crates/slicer-ir` exit 0, and does `mkdir -p target && cargo test -p slicer-ir 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result: ' target/test-output.log | sed 's/; finished in .*//' | sort | diff - target/sweep-196-slicer-ir-baseline.txt` report no difference?; scope: `crates/slicer-ir`; return: `FACT` PASS/FAIL + ≤5 lines on failure.
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - waiver format section (carrier-test and file-local-base reasons).
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals crates/slicer-ir; test $? -eq 0 && echo PASS` - FACT PASS/FAIL
  - `mkdir -p target && cargo test -p slicer-ir 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result: ' target/test-output.log | sed 's/; finished in .*//' | sort | diff - target/sweep-196-slicer-ir-baseline.txt && echo PASS` - FACT PASS/FAIL
- Exit condition: both commands PASS. Falsified if any test count changes or any waiver lacks a reason (`rg -n '// exhaustive:[[:space:]]*$' crates/slicer-ir` non-empty).

### Step 3: slicer-gcode sweep (dev-dep + fixtures)

- Task IDs: `TASK-318`
- Objective: zero violations in `crates/slicer-gcode`; `PrintEntity` sites routed through `print_entity_base`.
- Precondition: Step 1 scratch files exist.
- Postcondition: `cargo xtask check-literals crates/slicer-gcode` exits 0; `cargo test -p slicer-gcode` summary multiset equals baseline; dev-dep present.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-196-report.txt` - `grep '^crates/slicer-gcode/'` only
  - `crates/slicer-sdk/src/test_support/fixtures.rs` - fixture signatures only
  - `crates/slicer-runtime/Cargo.toml` - the existing renamed sdk dev-dep line only, as a syntax reference
- Files allowed to edit (at most 3; bounded glob counts as one sweep surface):
  - `crates/slicer-gcode/Cargo.toml` (add `[dev-dependencies]` `slicer-sdk = { path = "../slicer-sdk", features = ["test"] }`)
  - `crates/slicer-gcode/tests/**/*.rs` (bounded glob — only files named in the Step-1 report)
  - `crates/slicer-gcode/src/emit.rs` (`#[cfg(test)]` mod contents only, if reported)
- Files explicitly out of bounds:
  - `crates/slicer-gcode/src/**` outside the `emit.rs` cfg-test mod; `crates/slicer-sdk/**` (fixtures are consumed, never edited here)
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: after edits, does `cargo xtask check-literals crates/slicer-gcode` exit 0 and does the suite diff clean against `target/sweep-196-slicer-gcode-baseline.txt` (same pipeline as Step 2, crate swapped)?; scope: `crates/slicer-gcode`; return: `FACT` PASS/FAIL + ≤5 lines on failure.
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - fixture-policy section.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals crates/slicer-gcode; test $? -eq 0 && echo PASS` - FACT PASS/FAIL
  - `mkdir -p target && cargo test -p slicer-gcode 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result: ' target/test-output.log | sed 's/; finished in .*//' | sort | diff - target/sweep-196-slicer-gcode-baseline.txt && echo PASS` - FACT PASS/FAIL
  - `rg -q 'slicer.sdk.*features\s*=\s*\[\s*"test"' crates/slicer-gcode/Cargo.toml && rg -q 'print_entity_base' crates/slicer-gcode/tests && echo PASS` - FACT PASS/FAIL
- Exit condition: all three commands PASS. Falsified if a `PrintEntity` site remains exhaustive without a reasoned waiver, or the dev-dep breaks `cargo check -p slicer-gcode --tests`.

### Step 4: slicer-core sweep

- Task IDs: `TASK-318`
- Objective: zero violations in `crates/slicer-core` (tests, benches, reported cfg-test src mods) with the feature-gated suite green.
- Precondition: Step 1 scratch files exist.
- Postcondition: `cargo xtask check-literals crates/slicer-core` exits 0; `cargo test -p slicer-core --features host-algos --no-fail-fast` summary multiset equals baseline.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-196-report.txt` - `grep '^crates/slicer-core/'` only
  - reported src files (`lib.rs`, `perimeter_utils.rs`, `arachne/*.rs`) - ranged reads around reported violation lines only
- Files allowed to edit (at most 3; bounded glob counts as one sweep surface):
  - `crates/slicer-core/tests/**/*.rs` (bounded glob — only files named in the Step-1 report)
  - `crates/slicer-core/benches/**/*.rs` (bounded glob — only files named in the Step-1 report)
  - `#[cfg(test)]` mods in `crates/slicer-core/src/**` (only files named in the Step-1 report)
- Files explicitly out of bounds:
  - `crates/slicer-core/src/**` outside `#[cfg(test)]` mods; `crates/slicer-core/Cargo.toml` (no sdk dev-dep — design decision; no feature changes)
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: after edits, does `cargo xtask check-literals crates/slicer-core` exit 0 and does `mkdir -p target && cargo test -p slicer-core --features host-algos --no-fail-fast 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result: ' target/test-output.log | sed 's/; finished in .*//' | sort | diff - target/sweep-196-slicer-core-baseline.txt` report no difference?; scope: `crates/slicer-core`; return: `FACT` PASS/FAIL + ≤5 lines on failure.
- Context cost: `M` (largest file set; slow suite)
- Authoritative docs:
  - `CLAUDE.md` §Feature-gated test files - the mandatory `--features host-algos` invocation and binary-count reconciliation rule.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals crates/slicer-core; test $? -eq 0 && echo PASS` - FACT PASS/FAIL
  - `mkdir -p target && cargo test -p slicer-core --features host-algos --no-fail-fast 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result: ' target/test-output.log | sed 's/; finished in .*//' | sort | diff - target/sweep-196-slicer-core-baseline.txt && echo PASS` - FACT PASS/FAIL
- Exit condition: both commands PASS. Falsified if the post run's binary count is lower than the baseline's (blind run — wrong flags), or any arachne file edit stops compiling under `host-algos`.

### Step 5: Area gate, invariance guards, freshness, workspace gates

- Task IDs: `TASK-318`
- Objective: prove the whole area clean, counts invariant, guests fresh, workspace compiling and lint-clean.
- Precondition: Steps 2-4 exits met.
- Postcondition: every packet AC PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-196-*` scratch files - diff/grep only
- Files allowed to edit (at most 3):
  - none (fix-up edits re-enter the owning step's surface and re-run that step's verification)
- Files explicitly out of bounds:
  - all source outside Steps 2-4 surfaces.
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: run AC-1 through AC-6 and AC-N1 through AC-N3 exactly as written in `packet.spec.md` (including `cargo xtask build-guests --check`, rebuilding without `--check` first if `STALE:`), plus `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings`; scope: workspace; return: `FACT` per command, PASS/FAIL list + first failure ≤10 lines.
- Context cost: `M` (suite re-runs)
- Authoritative docs:
  - `CLAUDE.md` §Guest WASM Staleness - rebuild-then-recheck protocol.
- OrcaSlicer refs: none.
- Verification:
  - All `packet.spec.md` AC commands - FACT PASS/FAIL each
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: every command PASS. Falsified by any count drift (AC-2/3/4/N3), any residual violation (AC-1), a `Default` appearing on a locked type (AC-N1), an empty waiver reason (AC-N2), or `STALE:` persisting after rebuild (AC-6).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | three baseline suite runs, report capture |
| Step 2 | S | ~7 files (sizing estimate 2026-08-07; re-derive from report) |
| Step 3 | S | ~10 files + 1 manifest |
| Step 4 | M | ~15-20 files incl. benches + cfg-test src mods; slow suite |
| Step 5 | M | suite re-runs + workspace gates |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read (add the TASK-318 row; re-derive the insertion point at write time).
- Update the plan's Packet Queue row #3 status via a worker dispatch (`docs/specs/struct-literal-churn-gate-plan.md`).
- Reconcile reopened/superseded status transitions: none.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (expected: waiver inventory for packet 199's audit; any checker defects found, filed as deviations against packet 194).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.

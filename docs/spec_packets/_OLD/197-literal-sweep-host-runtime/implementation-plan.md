# Implementation Plan: 197-literal-sweep-host-runtime

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation. (For this sweep, the pre-sweep baseline IS the contract every later step must preserve.)
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Baseline capture and violation enumeration

- Task IDs: `TASK-319`
- Objective: record pre-sweep ground truth and the violation list driving Steps 2-6.
- Precondition: packets 194 and 195 `implemented`; `cargo xtask build-guests --check` clean (predecessor artifacts fresh — rebuild without `--check` first if `STALE:`); tree clean of unrelated edits.
- Postcondition: `target/sweep-197-report.txt`, `target/sweep-197-{slicer-runtime,slicer-scheduler,slicer-wasm-host,pnp-cli}-baseline.txt`, `target/sweep-197-assert-baseline.txt`, `target/sweep-197-testattr-baseline.txt` exist; every baseline `test result` line shows `0 failed`.
- Files allowed to read, with ranges when over 300 lines:
  - none directly — commands dispatched; consume FACT/LOCATIONS returns only.
- Files allowed to edit (at most 3):
  - none.
- Files explicitly out of bounds:
  - everything; read/record step.
- Blast-radius discipline: not applicable — no struct field or schema constant changes in this packet.
- Expected sub-agent dispatches:
  - Question: run, in order: `mkdir -p target`, `cargo xtask check-literals --report crates/slicer-runtime crates/slicer-scheduler crates/slicer-wasm-host crates/pnp-cli | tee target/sweep-197-report.txt | tail -1`; for each crate `c` in {slicer-runtime, slicer-scheduler, slicer-wasm-host, pnp-cli}: `cargo test -p $c 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort > target/sweep-197-$c-baseline.txt`; then `rg -o 'assert(_eq|_ne)?!' crates/slicer-runtime crates/slicer-scheduler crates/slicer-wasm-host crates/pnp-cli -g '!**/test-guests/**' | wc -l > target/sweep-197-assert-baseline.txt` and `rg -o '#\[test\]' crates/slicer-runtime crates/slicer-scheduler crates/slicer-wasm-host crates/pnp-cli -g '!**/test-guests/**' | wc -l > target/sweep-197-testattr-baseline.txt`. Report summary line, per-crate violating files (path + count), baseline greenness; scope: workspace; return: `LOCATIONS` ≤20 entries per crate + `FACT`.
- Context cost: `M` (four suite runs; runtime slow)
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY of conversion rule + waiver format.
- OrcaSlicer refs: none.
- Verification:
  - `test -s target/sweep-197-report.txt && test -s target/sweep-197-slicer-runtime-baseline.txt && test -s target/sweep-197-slicer-scheduler-baseline.txt && test -s target/sweep-197-slicer-wasm-host-baseline.txt && test -s target/sweep-197-pnp-cli-baseline.txt && echo PASS` - FACT PASS/FAIL
  - `cat target/sweep-197-assert-baseline.txt target/sweep-197-testattr-baseline.txt` - FACT: two integers
- Exit condition: scratch files exist; baselines fully green (pre-existing red halts the packet as a blocker); violation list in hand. Falsified if the report lists zero violations for all four crates (no-op — stop and report).

### Step 2: pnp-cli sweep (dev-dep, twin activation, fixtures)

- Task IDs: `TASK-319`
- Objective: zero violations in `crates/pnp-cli`; e2e `PipelineConfig` sites through the twin; `dead_code` allow removed; `PrintEntity` sites through `print_entity_base`.
- Precondition: Step 1 scratch files exist.
- Postcondition: `cargo xtask check-literals crates/pnp-cli` exits 0; `cargo test -p pnp-cli` multiset equals baseline.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-197-report.txt` - `grep '^crates/pnp-cli/'` only
  - `crates/slicer-sdk/src/test_support/fixtures.rs` - fixture signatures only
  - `crates/pnp-cli/tests/e2e_integration_tdd.rs` - ranged reads around `fn pipeline_config_base` and the reported literal lines
- Files allowed to edit (at most 3; bounded glob counts as one sweep surface):
  - `crates/pnp-cli/Cargo.toml` (dev-dep `slicer-sdk = { path = "../slicer-sdk", features = ["test"] }`)
  - `crates/pnp-cli/tests/**/*.rs` (bounded glob — only files named in the Step-1 report)
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/**`; the twin's signature/base values (only its `#[allow(dead_code)]` attribute and its callers change)
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: after edits, does `cargo xtask check-literals crates/pnp-cli` exit 0 and does `mkdir -p target && cargo test -p pnp-cli 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort | diff - target/sweep-197-pnp-cli-baseline.txt` report no difference?; scope: `crates/pnp-cli`; return: `FACT` PASS/FAIL + ≤5 lines on failure.
- Context cost: `M` (e2e tests run real slices)
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - fixture-policy section.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals crates/pnp-cli; test $? -eq 0 && echo PASS` - FACT PASS/FAIL
  - `mkdir -p target && cargo test -p pnp-cli 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort | diff - target/sweep-197-pnp-cli-baseline.txt && echo PASS` - FACT PASS/FAIL
  - `! (rg -B3 'fn pipeline_config_base' crates/pnp-cli/tests/e2e_integration_tdd.rs | rg -q 'dead_code') && echo PASS` - FACT PASS/FAIL
- Exit condition: all three PASS. Falsified if any e2e assertion line changed (`git diff -- crates/pnp-cli/tests | grep '^-' | grep -q 'assert'` finding a removed assert not re-added identically) or the twin remains uncalled.

### Step 3: slicer-scheduler sweep

- Task IDs: `TASK-319`
- Objective: zero violations in `crates/slicer-scheduler` (`ExecutionPlan`/`GlobalLayer` FRU).
- Precondition: Step 1 scratch files exist.
- Postcondition: `cargo xtask check-literals crates/slicer-scheduler` exits 0; suite multiset equals baseline.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-197-report.txt` - `grep '^crates/slicer-scheduler/'` only
  - `crates/slicer-scheduler/src/execution_plan.rs` - ranged reads: `pub struct ExecutionPlan` + `impl Default for ExecutionPlan` only
- Files allowed to edit (at most 3; bounded glob counts as one sweep surface):
  - `crates/slicer-scheduler/tests/**/*.rs` (bounded glob — only files named in the Step-1 report)
  - reported `#[cfg(test)]` mods in `crates/slicer-scheduler/src/**` (cfg-test contents only)
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/**` outside cfg-test mods; `crates/slicer-scheduler/Cargo.toml`
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: after edits, `check-literals crates/slicer-scheduler` exit code + suite diff vs `target/sweep-197-slicer-scheduler-baseline.txt` (pipeline as Step 2, crate swapped); scope: `crates/slicer-scheduler`; return: `FACT` PASS/FAIL + ≤5 lines.
- Context cost: `S` (smallest surface: ~2 candidate files measured 2026-08-07; re-derive)
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - conversion rule.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals crates/slicer-scheduler; test $? -eq 0 && echo PASS` - FACT PASS/FAIL
  - `mkdir -p target && cargo test -p slicer-scheduler 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort | diff - target/sweep-197-slicer-scheduler-baseline.txt && echo PASS` - FACT PASS/FAIL
- Exit condition: both PASS. Falsified if an `ExecutionPlan` FRU changes a plan a test builds (suite diff catches it).

### Step 4: slicer-wasm-host sweep

- Task IDs: `TASK-319`
- Objective: zero violations in `crates/slicer-wasm-host` host-side tests; carrier tests waivered, helpers FRU'd; `test-guests/**` untouched.
- Precondition: Step 1 scratch files exist.
- Postcondition: `cargo xtask check-literals crates/slicer-wasm-host` exits 0; suite multiset equals baseline; `git status --porcelain crates/slicer-wasm-host/test-guests` empty.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-197-report.txt` - `grep '^crates/slicer-wasm-host/'` only
  - `crates/slicer-wasm-host/tests/common/mod.rs` - ranged reads around reported helper fns
- Files allowed to edit (at most 3; bounded glob counts as one sweep surface):
  - `crates/slicer-wasm-host/tests/**/*.rs` (bounded glob — only files named in the Step-1 report)
  - reported `#[cfg(test)]` mods in `crates/slicer-wasm-host/src/**` (cfg-test contents only; candidates 2026-08-07: `host.rs`, `marshal/leaf.rs`)
- Files explicitly out of bounds:
  - `crates/slicer-wasm-host/test-guests/**` (rule-exempt, guest-feeding); `src/**` outside cfg-test mods (marshal checkpoints); `crates/slicer-wasm-host/Cargo.toml`
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: after edits, `check-literals crates/slicer-wasm-host` exit code + suite diff vs `target/sweep-197-slicer-wasm-host-baseline.txt`; scope: `crates/slicer-wasm-host`; return: `FACT` PASS/FAIL + ≤5 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - waiver format (carrier reason).
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals crates/slicer-wasm-host; test $? -eq 0 && echo PASS` - FACT PASS/FAIL
  - `mkdir -p target && cargo test -p slicer-wasm-host 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort | diff - target/sweep-197-slicer-wasm-host-baseline.txt && echo PASS` - FACT PASS/FAIL
  - `git status --porcelain crates/slicer-wasm-host/test-guests | wc -l | grep -qx '0' && echo PASS` - FACT PASS/FAIL
- Exit condition: all three PASS. Falsified if a carrier test got FRU'd (its exhaustiveness assertion intent lost — review the waiver list) or `test-guests` shows any diff.

### Step 5: slicer-runtime sweep A — bucket binaries (unit, contract, integration)

- Task IDs: `TASK-319`
- Objective: convert all reported sites in the `unit`, `contract`, `integration` buckets and the shared `tests/common/` tree (incl. `perimeter_harness.rs`'s exhaustive `PipelineConfig` literal → `pipeline_config_base` or FRU over it), including `pipeline_tdd.rs`'s 14 `PipelineConfig` sites → `common::pipeline_config_base` and `SliceRunOptions` → FRU.
- Precondition: Step 1 scratch files exist.
- Postcondition: the three bucket binaries pass; their reported files show zero violations (`check-literals` on the individual file paths exits 0).
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-197-report.txt` - `grep -E '^crates/slicer-runtime/tests/(unit|contract|integration|common)'` only
  - `crates/slicer-runtime/tests/common/mod.rs` - ranged reads around `pipeline_config_base` + reported helpers
  - `crates/slicer-runtime/src/run.rs` - `SliceRunOptions` + `Default` impl only
- Files allowed to edit (at most 3; bounded glob counts as one sweep surface):
  - `crates/slicer-runtime/tests/unit/**/*.rs`, `crates/slicer-runtime/tests/contract/**/*.rs`, `crates/slicer-runtime/tests/integration/**/*.rs` (bounded globs — only files named in the Step-1 report)
  - `crates/slicer-runtime/tests/common/**/*.rs` (bounded glob — only files named in the Step-1 report; incl. `perimeter_harness.rs`'s exhaustive `PipelineConfig` literal; internal helper-fn conversions + new sibling helpers only, packet-195 `pipeline_config_base` signature untouched)
- Files explicitly out of bounds:
  - `tests/executor/**`, `tests/e2e/**`, top-level test files, `benches/**` (Step 6); `crates/slicer-runtime/src/**` except the reported `layer_executor.rs` cfg-test mod (Step 6)
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: do `cargo test -p slicer-runtime --test unit`, `--test contract`, `--test integration` each pass (`2>&1 | tee target/test-output.log | grep -E '^test result'`)?; scope: three binaries; return: `FACT` per binary + failing names ≤5 lines.
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - conversion rule.
- OrcaSlicer refs: none.
- Verification:
  - `mkdir -p target && cargo test -p slicer-runtime --test integration 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: all `0 failed`
  - `mkdir -p target && cargo test -p slicer-runtime --test unit 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: all `0 failed`
  - `mkdir -p target && cargo test -p slicer-runtime --test contract 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: all `0 failed`
  - `rg -q 'pipeline_config_base\(' crates/slicer-runtime/tests/integration/pipeline_tdd.rs && echo PASS` - FACT PASS/FAIL
- Exit condition: all commands PASS. Falsified if any bucket count drops vs its baseline share (full-crate diff comes in Step 7) or a `pipeline_config_base` FRU changed a constructed config (field-by-field review rule in `design.md`).

### Step 6: slicer-runtime sweep B — executor, e2e, top-level, benches, cfg-test src

- Task IDs: `TASK-319`
- Objective: convert all remaining reported runtime sites: `executor`/`e2e` buckets, the ten top-level test binaries, `benches/**`, and the `layer_executor.rs` cfg-test mod.
- Precondition: Step 5 exit met.
- Postcondition: `cargo xtask check-literals crates/slicer-runtime` exits 0.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-197-report.txt` - `grep '^crates/slicer-runtime/' | grep -v 'tests/(unit|contract|integration)'` only
  - `crates/slicer-sdk/src/test_support/fixtures.rs` - fixture signatures only
- Files allowed to edit (at most 3; bounded glob counts as one sweep surface):
  - `crates/slicer-runtime/tests/executor/**/*.rs`, `crates/slicer-runtime/tests/e2e/**/*.rs`, `crates/slicer-runtime/tests/*.rs` (bounded globs — only files named in the Step-1 report)
  - `crates/slicer-runtime/benches/**/*.rs` (only files named in the Step-1 report)
  - `crates/slicer-runtime/src/layer_executor.rs` (`#[cfg(test)]` mod contents only)
- Files explicitly out of bounds:
  - `crates/slicer-runtime/src/**` outside the `layer_executor.rs` cfg-test mod; `crates/slicer-runtime/Cargo.toml`
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: do `cargo test -p slicer-runtime --test executor` and `--test e2e` pass, and does `cargo xtask check-literals crates/slicer-runtime` exit 0?; scope: `crates/slicer-runtime`; return: `FACT` per command + ≤5 lines on failure.
- Context cost: `M`
- Authoritative docs:
  - `CLAUDE.md` §Guest WASM Staleness - triage rule if executor/e2e fail (stale predecessor artifacts, not this packet's edits).
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals crates/slicer-runtime; test $? -eq 0 && echo PASS` - FACT PASS/FAIL
  - `mkdir -p target && cargo test -p slicer-runtime --test executor 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: all `0 failed`
  - `mkdir -p target && cargo test -p slicer-runtime --test e2e 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT: all `0 failed`
- Exit condition: all PASS. Falsified if an executor/e2e failure survives a clean `cargo xtask build-guests --check` triage (then it IS this packet's bug).

### Step 7: Area gate, invariance guards, workspace gates

- Task IDs: `TASK-319`
- Objective: prove the whole area clean, counts invariant, workspace compiling and lint-clean.
- Precondition: Steps 2-6 exits met.
- Postcondition: every packet AC PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-197-*` scratch files - diff/grep only
- Files allowed to edit (at most 3):
  - none (fix-ups re-enter the owning step's surface and re-verify there)
- Files explicitly out of bounds:
  - all source outside Steps 2-6 surfaces.
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: run AC-1 through AC-7 and AC-N1 through AC-N4 exactly as written in `packet.spec.md`, plus `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings`; scope: workspace; return: `FACT` PASS/FAIL list + first failure ≤10 lines.
- Context cost: `M` (four suite re-runs)
- Authoritative docs:
  - `CLAUDE.md` §Test Discipline - tee/log-reading rules for the re-runs.
- OrcaSlicer refs: none.
- Verification:
  - All `packet.spec.md` AC commands - FACT PASS/FAIL each
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: every command PASS. Falsified by count drift (AC-2/3/N3), residual violations (AC-1), a locked type gaining `Default` (AC-N1), empty waiver reasons (AC-N2), or a `test-guests` diff (AC-N4).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | four baseline suite runs, report capture |
| Step 2 | M | pnp-cli e2e runs real slices |
| Step 3 | S | ~2 candidate files (sizing estimate; re-derive) |
| Step 4 | S | ~7 files incl. common helpers |
| Step 5 | M | biggest bucket set + pipeline_tdd conversions |
| Step 6 | M | executor/e2e + top-level + benches |
| Step 7 | M | suite re-runs + workspace gates |

Split before activation if aggregate cost exceeds M or any step is L. (Aggregate here is M: steps are sequential sweeps over disjoint file sets with bounded per-step budgets; no step carries cross-step reading debt.)

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read (TASK-319 row; re-derive insertion point at write time).
- Update the plan's Packet Queue row #4 status via a worker dispatch (`docs/specs/struct-literal-churn-gate-plan.md`).
- Reconcile reopened/superseded status transitions: none.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (waiver inventory + any new file-local base fns for 199's audit; checker defects filed as deviations against packet 194).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.

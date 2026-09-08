# Implementation Plan: 198-literal-sweep-sdk-modules

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation. (For this sweep, the pre-sweep baseline IS the contract every later step must preserve.)
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Baseline capture and violation enumeration

- Task IDs: `TASK-320`
- Objective: record pre-sweep ground truth (sdk suite with `--features test`, per-module aggregate, counts) and the violation list + module list driving Steps 2-4.
- Precondition: packets 194 and 195 `implemented`; `cargo xtask build-guests --check` clean (predecessor artifacts fresh — rebuild first if `STALE:`); tree clean of unrelated edits.
- Postcondition: `target/sweep-198-report.txt`, `target/sweep-198-slicer-sdk-baseline.txt`, `target/sweep-198-modules.txt`, `target/sweep-198-modules-baseline.txt`, `target/sweep-198-assert-baseline.txt`, `target/sweep-198-testattr-baseline.txt` exist; every baseline `test result` line shows `0 failed`.
- Files allowed to read, with ranges when over 300 lines:
  - none directly — commands dispatched; consume FACT/LOCATIONS returns only.
- Files allowed to edit (at most 3):
  - none.
- Files explicitly out of bounds:
  - everything; read/record step.
- Blast-radius discipline: not applicable — no struct field or schema constant changes in this packet.
- Expected sub-agent dispatches:
  - Question: run, in order: `mkdir -p target`; `cargo xtask check-literals --report crates/slicer-sdk modules/core-modules | tee target/sweep-198-report.txt | tail -1`; `grep -oE '^modules/core-modules/[^/]+' target/sweep-198-report.txt | sed 's|modules/core-modules/||' | sort -u > target/sweep-198-modules.txt`; sdk baseline `cargo test -p slicer-sdk --features test 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort > target/sweep-198-slicer-sdk-baseline.txt`; module baseline `rm -f target/sweep-198-modules-base-raw.txt; while read -r m; do cargo test -p "$m" 2>&1 | tee -a target/sweep-198-modules-base-raw.txt >/dev/null; done < target/sweep-198-modules.txt; grep -E '^test result' target/sweep-198-modules-base-raw.txt | sed 's/; finished in .*//' | sort > target/sweep-198-modules-baseline.txt`; counts `rg -o 'assert(_eq|_ne)?!' crates/slicer-sdk modules/core-modules/*/tests | wc -l > target/sweep-198-assert-baseline.txt` and `rg -o '#\[test\]' crates/slicer-sdk modules/core-modules/*/tests | wc -l > target/sweep-198-testattr-baseline.txt`. Report summary line, per-area violating files (path + count), module list, baseline greenness; scope: workspace; return: `LOCATIONS` ≤20 entries per area + `FACT`.
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - delegated SUMMARY of conversion rule + waiver format.
- OrcaSlicer refs: none.
- Verification:
  - `test -s target/sweep-198-report.txt && test -s target/sweep-198-slicer-sdk-baseline.txt && test -s target/sweep-198-modules.txt && test -s target/sweep-198-modules-baseline.txt && echo PASS` - FACT PASS/FAIL
  - `cat target/sweep-198-assert-baseline.txt target/sweep-198-testattr-baseline.txt && wc -l < target/sweep-198-modules.txt` - FACT: three integers
- Exit condition: scratch files exist; baselines fully green (pre-existing red halts the packet as a blocker); violation and module lists in hand. Falsified if the report lists zero violations for both areas (no-op — stop and report).

### Step 2: slicer-sdk sweep and manifest gating

- Task IDs: `TASK-320`
- Objective: zero violations in `crates/slicer-sdk`; newly fixture-consuming test files gated with `[[test]] required-features = ["test"]`; fixture-base literals waivered where reported; bare run still green.
- Precondition: Step 1 scratch files exist.
- Postcondition: `cargo xtask check-literals crates/slicer-sdk` exits 0; `--features test` suite multiset equals baseline; bare `cargo test -p slicer-sdk` compiles and passes.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-198-report.txt` - `grep '^crates/slicer-sdk/'` only
  - `crates/slicer-sdk/src/test_support/fixtures.rs` - fixture signatures + base values only
  - `crates/slicer-sdk/Cargo.toml` - `[[test]]` tail only
- Files allowed to edit (at most 3; bounded glob counts as one sweep surface):
  - `crates/slicer-sdk/tests/**/*.rs` (bounded glob — only files named in the Step-1 report)
  - `crates/slicer-sdk/Cargo.toml` (append `[[test]]` gating entries only)
  - `crates/slicer-sdk/src/test_support/**` (waiver comments only)
- Files explicitly out of bounds:
  - `crates/slicer-sdk/src/**` outside `test_support/**`; fixture signatures/values (packet-195 contract)
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: after edits, do (a) `cargo xtask check-literals crates/slicer-sdk` exit 0, (b) `mkdir -p target && cargo test -p slicer-sdk --features test 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort | diff - target/sweep-198-slicer-sdk-baseline.txt` report no difference, (c) `cargo test -p slicer-sdk` (bare) pass?; scope: `crates/slicer-sdk`; return: `FACT` PASS/FAIL per command + ≤5 lines on failure.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0004-test-support-lives-in-slicer-sdk.md` - gating rationale; guests never enable `test`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals crates/slicer-sdk; test $? -eq 0 && echo PASS` - FACT PASS/FAIL
  - `mkdir -p target && cargo test -p slicer-sdk --features test 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort | diff - target/sweep-198-slicer-sdk-baseline.txt && echo PASS` - FACT PASS/FAIL
  - `ok=1; for f in crates/slicer-sdk/tests/*.rs; do rg -q 'test_support' "$f" || continue; n=$(basename "${f%.rs}"); rg -A2 "name = \"$n\"" crates/slicer-sdk/Cargo.toml | rg -q 'required-features' || { echo "MISSING: $n"; ok=0; }; done; test $ok -eq 1 && echo PASS` - FACT PASS/FAIL
- Exit condition: all commands PASS. Falsified if the `--features test` binary count drops vs baseline (a gated file was lost, not gated), or the bare run breaks (an ungated file references `test_support`).

### Step 3: Module sweep batch A (largest modules)

- Task IDs: `TASK-320`
- Objective: zero violations in the four largest measured module test dirs — `seam-placer`, `infill-linker`, `path-optimization-default`, `wipe-tower` (2026-08-07 sizing: 7+4+3+3 files; re-derive membership from `target/sweep-198-modules.txt` — batch A is the top half of the list by violating-file count).
- Precondition: Step 1 scratch files exist.
- Postcondition: `cargo xtask check-literals` on batch A module paths exits 0; each batch-A `cargo test -p <module>` passes.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-198-report.txt` - per-batch-A-module greps only
  - `crates/slicer-sdk/src/test_support/fixtures.rs` - fixture signatures + base values only
- Files allowed to edit (at most 3; bounded glob counts as one sweep surface):
  - `modules/core-modules/{seam-placer,infill-linker,path-optimization-default,wipe-tower}/tests/**/*.rs` (bounded glob — only files named in the Step-1 report; substitute the re-derived batch-A membership)
- Files explicitly out of bounds:
  - `modules/core-modules/*/src/**`, `*/wit-guest/**`, `*/Cargo.toml`; all other modules (batch B)
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: after edits, does `cargo xtask check-literals modules/core-modules/<m>` exit 0 and `cargo test -p <m>` pass for each batch-A module?; scope: batch A; return: `FACT` per module + ≤5 lines on failure.
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - conversion rule.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals modules/core-modules/seam-placer modules/core-modules/infill-linker modules/core-modules/path-optimization-default modules/core-modules/wipe-tower; test $? -eq 0 && echo PASS` (substitute re-derived batch-A paths) - FACT PASS/FAIL
  - `mkdir -p target && for m in seam-placer infill-linker path-optimization-default wipe-tower; do cargo test -p $m 2>&1 | tee -a target/test-output.log >/dev/null || echo FAIL:$m; done; echo DONE` (substitute re-derived batch-A names) - FACT: no `FAIL:` lines
- Exit condition: both PASS. Falsified if any module suite count changes (aggregate diff in Step 5 catches drift batch-locally invisible).

### Step 4: Module sweep batch B (remaining modules)

- Task IDs: `TASK-320`
- Objective: zero violations in the remaining listed modules (2026-08-07 candidates: `fuzzy-skin`, `skirt-brim`, `support-planner`, `arachne-perimeters`, `overhang-classifier-default`, `part-cooling`; re-derive as `target/sweep-198-modules.txt` minus batch A).
- Precondition: Step 3 exit met.
- Postcondition: `cargo xtask check-literals modules/core-modules` exits 0 (whole module tree); each batch-B `cargo test -p <module>` passes.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-198-report.txt` - per-batch-B-module greps only
  - `crates/slicer-sdk/src/test_support/fixtures.rs` - fixture signatures + base values only
- Files allowed to edit (at most 3; bounded glob counts as one sweep surface):
  - `modules/core-modules/{fuzzy-skin,skirt-brim,support-planner,arachne-perimeters,overhang-classifier-default,part-cooling}/tests/**/*.rs` (bounded glob — only files named in the Step-1 report; substitute the re-derived batch-B membership)
- Files explicitly out of bounds:
  - `modules/core-modules/*/src/**`, `*/wit-guest/**`, `*/Cargo.toml`; batch-A modules (done)
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: after edits, does `cargo xtask check-literals modules/core-modules` exit 0 and `cargo test -p <m>` pass for each batch-B module?; scope: batch B; return: `FACT` per module + ≤5 lines on failure.
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - conversion rule.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask check-literals modules/core-modules; test $? -eq 0 && echo PASS` - FACT PASS/FAIL
  - `mkdir -p target && for m in fuzzy-skin skirt-brim support-planner arachne-perimeters overhang-classifier-default part-cooling; do cargo test -p $m 2>&1 | tee -a target/test-output.log >/dev/null || echo FAIL:$m; done; echo DONE` (substitute re-derived batch-B names) - FACT: no `FAIL:` lines
- Exit condition: both PASS. Falsified if the whole-tree gate still reports a module outside both batches (list drift — re-derive and extend).

### Step 5: Guest rebuild, area gate, invariance guards, workspace gates

- Task IDs: `TASK-320`
- Objective: rebuild guests after the sdk manifest edit; prove the whole area clean, counts invariant, guests fresh, workspace compiling and lint-clean.
- Precondition: Steps 2-4 exits met.
- Postcondition: every packet AC PASS.
- Files allowed to read, with ranges when over 300 lines:
  - `target/sweep-198-*` scratch files - diff/grep only
- Files allowed to edit (at most 3):
  - none (fix-ups re-enter the owning step's surface and re-verify there)
- Files explicitly out of bounds:
  - all source outside Steps 2-4 surfaces.
- Blast-radius discipline: not applicable — no struct field or schema constant changes.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask build-guests` (full rebuild — `STALE:` from the manifest edit is expected) then `cargo xtask build-guests --check`; then AC-1 through AC-7 and AC-N1 through AC-N4 exactly as written in `packet.spec.md`; then `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings`; scope: workspace; return: `FACT` PASS/FAIL list + first failure ≤10 lines.
- Context cost: `M` (guest rebuild + suite re-runs)
- Authoritative docs:
  - `CLAUDE.md` §Guest WASM Staleness - rebuild-then-recheck protocol.
- OrcaSlicer refs: none.
- Verification:
  - `cargo xtask build-guests --check; test $? -eq 0 && echo PASS` - FACT PASS/FAIL (after the rebuild)
  - All `packet.spec.md` AC commands - FACT PASS/FAIL each
  - `cargo check --workspace --all-targets` - FACT pass/fail
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail
- Exit condition: every command PASS. Falsified by count drift (AC-2/3/N3), residual violations (AC-1), a locked type gaining `Default` (AC-N1), empty waiver reasons (AC-N2), a module manifest diff (AC-N4), or `STALE:` persisting after rebuild (AC-5).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | sdk + per-module baseline runs, report capture |
| Step 2 | M | sdk conversions + gating + three-way verification |
| Step 3 | M | ~17 files across 4 modules (sizing estimate; re-derive) |
| Step 4 | S | ~9 files across 6 modules (sizing estimate; re-derive) |
| Step 5 | M | guest rebuild + suite re-runs + workspace gates |

Split before activation if aggregate cost exceeds M or any step is L. (Aggregate here is M: sequential sweeps over disjoint surfaces; the rebuild is one dispatched command consuming a FACT.)

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read (TASK-320 row; re-derive insertion point at write time).
- Update the plan's Packet Queue row #5 status via a worker dispatch (`docs/specs/struct-literal-churn-gate-plan.md`).
- Reconcile reopened/superseded status transitions: none.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk (waiver inventory + newly gated sdk test files for 199's audit; the widened sdk bare-run blind spot; checker defects filed as deviations against packet 194).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.

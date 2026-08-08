# Implementation Plan: 199-literal-gate-enforcement

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Re-derive residue and capture baselines

- Task IDs: `TASK-321`
- Objective: replace the packet's grounded residue inventory with a tool-derived one, and freeze pre-edit test/assert baselines for the three residue crates.
- Precondition: packets 194–198 `implemented`; `cargo xtask check-literals --report` runs.
- Postcondition: `target/gate-199-{slicer-model-io,slicer-helpers,slicer-macros}-baseline.txt` (sorted, time-stripped `test result` multisets), `target/gate-199-assert-baseline.txt`, `target/gate-199-testattr-baseline.txt` exist; a residue file list (scratch note in the swarm ledger, not committed) reconciled against `requirements.md` §In Scope.
- Files allowed to read, with ranges when over 300 lines:
  - none directly — this step is dispatch-only.
- Files allowed to edit (at most 3):
  - none (baseline files under `target/` are command outputs, not edits).
- Files explicitly out of bounds:
  - everything; all work goes through dispatches.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask check-literals --report` and list every reported violation file outside `crates/slicer-{ir,core,gcode,runtime,scheduler,wasm-host,sdk}`, `crates/pnp-cli`, and `modules/core-modules`; scope: workspace; return: `LOCATIONS` (<=20)
  - Question: run, for each of slicer-model-io / slicer-helpers / slicer-macros: `mkdir -p target && cargo test -p <crate> 2>&1 | tee target/test-output.log >/dev/null; grep -E '^test result' target/test-output.log | sed 's/; finished in .*//' | sort > target/gate-199-<crate>-baseline.txt` then `a=$(rg -o 'assert(_eq|_ne)?!' crates/slicer-model-io crates/slicer-helpers crates/slicer-macros | wc -l); echo $a > target/gate-199-assert-baseline.txt; t=$(rg -o '#\[test\]' crates/slicer-model-io crates/slicer-helpers crates/slicer-macros | wc -l); echo $t > target/gate-199-testattr-baseline.txt`; confirm every baseline line reports `0 failed`; scope: the three crates; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - direct read (short)
- OrcaSlicer refs:
  - none.
- Verification:
  - `ls target/gate-199-slicer-model-io-baseline.txt target/gate-199-slicer-helpers-baseline.txt target/gate-199-slicer-macros-baseline.txt target/gate-199-assert-baseline.txt target/gate-199-testattr-baseline.txt` - FACT pass/fail
- Exit condition: all five baseline files exist, every baseline `test result` line reports `0 failed`, and the residue list is reconciled (delta vs. the packet's grounded inventory recorded; a violation inside a 196/197/198 area triggers escalation, not editing).

### Step 2: Residue conversion — slicer-model-io

- Task IDs: `TASK-321`
- Objective: make `cargo xtask check-literals crates/slicer-model-io` exit 0 by routing test `ObjectMesh` construction through a shared FRU base, with one reasoned waiver in the src cfg-test mod.
- Precondition: Step 1 baselines exist; Step-1 residue list confirms the slicer-model-io files.
- Postcondition: `cargo xtask check-literals crates/slicer-model-io` exits 0; `cargo test -p slicer-model-io` green with unchanged summary multiset.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-model-io/tests/model_writer_roundtrip_tdd.rs`, `threemf_writer_roundtrip_tdd.rs`, `world_z_below_floor_tdd.rs`, `world_z_canonical_surface_tdd.rs` - full (each <300 lines)
  - `crates/slicer-model-io/src/loader.rs` - `#[cfg(test)]` mod region only (locate via `rg -n 'fn make_object' crates/slicer-model-io/src/loader.rs`; file >3000 lines)
  - `crates/slicer-ir/src/slice_ir.rs` - `ObjectMesh` definition region only
- Files allowed to edit (at most 3; residue sweep uses the sanctioned bounded per-crate glob):
  - `crates/slicer-model-io/tests/common/mod.rs` (new: waivered `object_mesh_base()` per `design.md`)
  - `crates/slicer-model-io/tests/{model_writer_roundtrip_tdd,threemf_writer_roundtrip_tdd,world_z_below_floor_tdd,world_z_canonical_surface_tdd}.rs` (bounded glob: add `mod common;`, convert `ObjectMesh` literals to overrides + `..common::object_mesh_base()`, omitting default-equal fields)
  - `crates/slicer-model-io/src/loader.rs` (cfg-test `make_object` literal: `// exhaustive:` waiver with reason — helper computes `world_z_extent`; exhaustive routing is the intent)
- Files explicitly out of bounds:
  - all other crates; `crates/slicer-model-io/src/**` outside the cfg-test mod; production `ObjectMesh` construction in `loader.rs` (exempt by rule).
- Blast-radius discipline: no struct field or schema constant changes in this step; the literal sites are the LOCATIONS result cited in `requirements.md` §In Scope (measured 2026-08-07; Step-1 re-derivation is authoritative).
- Expected sub-agent dispatches:
  - Question: post-edit, run `cargo xtask check-literals crates/slicer-model-io` and `cargo test -p slicer-model-io` (tee to `target/test-output.log`), report exit codes and the sorted summary-multiset diff vs `target/gate-199-slicer-model-io-baseline.txt`; scope: one crate; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - direct read (conversion + waiver rules)
- OrcaSlicer refs:
  - none.
- Verification:
  - `cargo xtask check-literals crates/slicer-model-io; test $? -eq 0 && echo PASS` - FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-model-io 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail (multiset diff vs baseline)
- Exit condition: area gate exits 0 AND suite multiset is byte-identical to baseline; any assertion edit is a step failure.

### Step 3: Residue conversion — slicer-helpers and slicer-macros

- Task IDs: `TASK-321`
- Objective: make `cargo xtask check-literals crates/slicer-helpers crates/slicer-macros` exit 0 via reasoned waivers (per `design.md`: helpers' file-local constructor helpers are the FRU bases; macros' 1-field name-colliding mocks cannot be renamed or FRU'd).
- Precondition: Step 1 baselines exist.
- Postcondition: area gate green for both crates; both suites green with unchanged summary multisets.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-helpers/tests/decimate_tdd.rs`, `crates/slicer-helpers/tests/repair_tdd.rs` - constructor-helper regions
  - `crates/slicer-macros/tests/slicer_module_tdd.rs` - mock impl region (~lines 90–140; re-locate via `rg -n 'impl InfillOutputBuilder|impl PerimeterOutputBuilder'`)
- Files allowed to edit (at most 3):
  - `crates/slicer-helpers/tests/decimate_tdd.rs` (one waiver line + reason above the `ObjectMesh` literal)
  - `crates/slicer-helpers/tests/repair_tdd.rs` (same)
  - `crates/slicer-macros/tests/slicer_module_tdd.rs` (waivers on the two watched-name `Self { ... }` literals)
- Files explicitly out of bounds:
  - `crates/slicer-macros/src/**` and `crates/slicer-macros/Cargo.toml` (guest-fingerprinted; touching them triggers the build-guests protocol per `design.md` — this step must not need them)
  - `crates/slicer-helpers/src/**` except reading; all other crates.
- Blast-radius discipline: no struct/schema changes; waiver-only edits.
- Expected sub-agent dispatches:
  - Question: post-edit, run `cargo xtask check-literals crates/slicer-helpers crates/slicer-macros`, `cargo test -p slicer-helpers`, `cargo test -p slicer-macros` (tee), report exit codes and multiset diffs vs the two baselines; scope: two crates; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/21_data_defaults_and_fixtures.md` - direct read (waiver format: `// exhaustive: <reason>`, same line or line above)
- OrcaSlicer refs:
  - none.
- Verification:
  - `cargo xtask check-literals crates/slicer-helpers crates/slicer-macros; test $? -eq 0 && echo PASS` - FACT pass/fail
  - `rg -n '// exhaustive:[[:space:]]*$' crates/slicer-helpers crates/slicer-macros; test $? -eq 1 && echo PASS` - FACT pass/fail
- Exit condition: both area gates exit 0, both suites match baselines, every new waiver carries a non-empty reason.

### Step 4: Preflight wiring in `cargo xtask test`

- Task IDs: `TASK-321`
- Objective: `test_command` (`xtask/src/test.rs`) blocks on check-literals violations before the guest-freshness gate, with unit tests proving block and pass paths.
- Precondition: Steps 2–3 complete (workspace gate green on the real tree, so wiring cannot brick `cargo xtask test`); the LOCATIONS dispatch below has identified 194's scan entry.
- Postcondition: AC-3, AC-N1, AC-N2, AC-N3 all pass.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/test.rs` - full (`test_command` and the `#[cfg(test)]` mod)
  - `xtask/src/check_literals.rs` - scan entry region only (from the LOCATIONS dispatch)
  - `xtask/src/main.rs` - USAGE block and `test` arm only
- Files allowed to edit (at most 3):
  - `xtask/src/test.rs` (helper `check_literals_preflight`, call site at top of Step 1 before `build_guests::check_command`, failure line `xtask test: check-literals preflight failed; fix violations or add reasoned waivers (docs/21_data_defaults_and_fixtures.md), then re-run.`, two unit tests per `design.md`)
  - `xtask/src/check_literals.rs` (ONLY the thin `run_enforce` wrapper if the scan entry is CLI-argv-shaped; otherwise untouched)
  - `xtask/src/main.rs` (USAGE `test [ARGS...]` description)
- Files explicitly out of bounds:
  - `xtask/src/build_guests.rs` (read-only), all non-xtask crates, CLAUDE.md/docs (Step 5).
- Blast-radius discipline: no struct/schema changes.
- Expected sub-agent dispatches:
  - Question: name/signature of the callable scan entry in `xtask/src/check_literals.rs` (takes ws_root + filters? returns exit code?); scope: `xtask/src/check_literals.rs`; return: `LOCATIONS`
  - Question: run the AC-N2 and AC-N3 probe commands verbatim from `packet.spec.md` and report PASS/FAIL with the first 10 output lines on failure; scope: workspace root; return: `FACT`
- Probe-file hygiene (both negative ACs inject a real file into the tree): the `rm -f` in each command is unconditional and runs before the assertion chain, so an assertion failure cannot strand the probe — but an interrupted or killed run can. After ANY probe run, and before this step's exit condition is evaluated, confirm the tree is clean: `test ! -e crates/slicer-ir/tests/data/gate_probe_199_tmp.rs && git status --porcelain crates/slicer-ir | grep -qv gate_probe || true`. A stranded probe would make every later gate run fail with a violation the packet did not author — treat a non-empty result as this step's failure, not as a residue finding.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decision 4 (wiring next to `build-guests --check`)
- OrcaSlicer refs:
  - none.
- Verification:
  - `mkdir -p target && cargo test -p xtask preflight 2>&1 | tee target/test-output.log | grep -E '^test result'` - FACT pass/fail
  - AC-N2 probe command (verbatim from `packet.spec.md`) - FACT pass/fail
  - AC-N3 probe command (verbatim from `packet.spec.md`) - FACT pass/fail
- Exit condition: unit tests green; probe run shows abort-before-tests with the exact failure line; clean tree still enters the guest-freshness gate (verified by AC-N2's second, clean invocation exiting 0 and by `cargo xtask check-literals` exit 0).

### Step 4b: CI gate step in `.github/workflows/ci.yml`

- Task IDs: `TASK-321`
- Objective: CI fails a push/PR that introduces an exhaustive watched literal, closing the gap that the Step-4 preflight alone leaves (the `test` job calls `cargo test -p ...` directly and never routes through `cargo xtask test`).
- Precondition: Steps 2–3 complete, so the workspace gate is already green on the real tree — otherwise this step turns CI red on master.
- Postcondition: AC-9 passes.
- Files allowed to read, with ranges when over 300 lines:
  - `.github/workflows/ci.yml` - full (84 lines at grounding time; re-read, do not trust the count)
  - `.cargo/config.toml` - the `[alias]` block only
- Files allowed to edit (at most 3):
  - `.github/workflows/ci.yml` (ONE new step appended to the `docs-guard` job, after its existing `check-deviations --check` step, matching that step's `cargo run -q -p xtask --` invocation form):

    ```yaml
      - name: Struct-literal gate
        run: cargo run -q -p xtask -- check-literals
    ```

- Files explicitly out of bounds:
  - every other job in `ci.yml` (`fmt`, `clippy`, `test` — in particular do NOT reroute the `test` job through `cargo xtask test`), all Rust sources, CLAUDE.md/docs (Step 5).
- Blast-radius discipline: no struct/schema changes; no CI job added or renamed.
- Expected sub-agent dispatches:
  - Question: does `docs-guard` still declare exactly its pre-existing steps plus the one new step, and does the file parse as YAML with all four job names intact? scope: `.github/workflows/ci.yml`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decision 4 (enforcement wiring), extended by the user's 2026-08-07 ruling that CI is in scope for this packet.
- OrcaSlicer refs:
  - none.
- Verification:
  - AC-9 command (verbatim from `packet.spec.md`) - FACT pass/fail
- Exit condition: AC-9 passes; the step runs the gate in enforce mode (no `--report`, no path filter) so a violation anywhere fails the job. Falsified if the YAML parse fails, a job name changes, or the step lands in a job other than `docs-guard`.

### Step 5: Enforcement flip in CLAUDE.md and docs/21

- Task IDs: `TASK-321`
- Objective: flip all gate-off wording to enforced state and repair §Feature-gated test files (end-state), per the Doc Impact Statement.
- Precondition: Steps 2–4 complete (never advertise a required gate the tree or tooling fails).
- Postcondition: AC-4, AC-5, AC-6, AC-7 all pass.
- Files allowed to read, with ranges when over 300 lines:
  - `CLAUDE.md` - sections: Build & Test Commands; Test Discipline (both subsections); the packet-194 rule section
  - `docs/21_data_defaults_and_fixtures.md` - full
  - `crates/slicer-runtime/Cargo.toml`, `crates/slicer-sdk/Cargo.toml`, `crates/slicer-wasm-host/Cargo.toml`, `crates/slicer-gcode/Cargo.toml` - dependency sections only (re-verify host-algos facts at the moment of writing; ledger facts rot)
- Files allowed to edit (at most 3):
  - `CLAUDE.md` (four section edits: commit-gate line; rule-section marker → `enforced since packet 199`; gated-entry-point pipeline gains the `check-literals preflight` step before `build-guests --check`; §Feature-gated end-state — stale gcode claim absent, `no production \`slicer-core\` dependency` clarification with the packet-196 dev-dep nuance, `cargo test -p slicer-sdk --features test` hazard in the same hazard class as host-algos)
  - `docs/21_data_defaults_and_fixtures.md` (gate-off phrasing → enforced-state wording with the `enforced since packet 199` anchor)
- MEASURE BEFORE WRITING the sdk hazard sentence: run `cargo test -p slicer-sdk --no-run 2>&1 | grep -c Executable` and `cargo test -p slicer-sdk --features test --no-run 2>&1 | grep -c Executable` and compare. `crates/slicer-sdk/Cargo.toml` carries a self dev-dep (`slicer-sdk = { path = ".", features = ["test"] }`) which may already enable the feature, so the "bare runs silently skip" phrasing is a hypothesis, not a fact. If the counts differ, write the skip-hazard wording with the measured delta; if they match, write that the self dev-dep enables it and `--features test` is explicit belt-and-braces. Either way `cargo test -p slicer-sdk --features test` is the documented invocation, so AC-7's grep holds under both outcomes. Never assert the delta without the measurement in hand.
- Files explicitly out of bounds:
  - `.claude/doc-index.md`, `docs/00_project_overview.md` (packet 194 already indexed the page; no index change from a wording flip), all code files.
- Blast-radius discipline: no struct/schema changes.
- Expected sub-agent dispatches:
  - Question: run the four AC-4..AC-7 grep commands verbatim and report per-command PASS/FAIL; scope: `CLAUDE.md` + `docs/21_data_defaults_and_fixtures.md`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - locked decisions 4 and 5
- OrcaSlicer refs:
  - none.
- Verification:
  - AC-4, AC-5, AC-6, AC-7 commands (verbatim from `packet.spec.md`) - FACT pass/fail each
- Exit condition: all four grep ACs pass; no other CLAUDE.md section modified (spot-check via `git diff --stat CLAUDE.md`).

### Step 6: Waiver audit, workspace gates, docs/07 crosswalk

- Task IDs: `TASK-321`
- Objective: audit the waiver inventory, run the closure gates, and update the backlog crosswalk.
- Precondition: Steps 1–5 complete.
- Postcondition: packet ready for `status: implemented`; close notes carry the re-derived waiver count and any residue-inventory delta.
- Files allowed to read, with ranges when over 300 lines:
  - none directly — dispatch-only step.
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (via worker dispatch: TASK-321 row)
  - `docs/spec_packets/199-literal-gate-enforcement/packet.spec.md` (status flip at ceremony only)
- Files explicitly out of bounds:
  - everything else.
- Expected sub-agent dispatches:
  - Question: run `rg -n '// exhaustive:' crates modules | wc -l` and `rg -n '// exhaustive:[[:space:]]*$' crates modules; echo rc=$?`; report both (the count is recorded in close notes ONLY — never written into an AC or doc as a frozen number); scope: workspace; return: `FACT`
  - Question: run `cargo check --workspace --all-targets` then `cargo clippy --workspace --all-targets -- -D warnings`; scope: workspace; return: `FACT pass/fail`
  - Question: update `docs/07_implementation_status.md` for TASK-321 per `task-map.md`; scope: that file; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/struct-literal-churn-gate-plan.md` - queue row 6 closure semantics (plan closes with this packet)
- OrcaSlicer refs:
  - none.
- Verification:
  - `cargo xtask check-literals; test $? -eq 0 && echo PASS` - FACT pass/fail
  - `rg -n '// exhaustive:[[:space:]]*$' crates modules; test $? -eq 1 && echo PASS` - FACT pass/fail
- Exit condition: AC-1 and AC-N4 pass, both workspace gates clean, docs/07 updated, close notes record waiver count + deltas.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | dispatch-only re-derivation + baselines |
| Step 2 | M | 6-file model-io conversion |
| Step 3 | S | 4 waiver lines across 3 files |
| Step 4 | M | wiring + unit tests + probe runs |
| Step 4b | S | one CI YAML step |
| Step 5 | S | doc edits + greps |
| Step 6 | S | audit + gates + crosswalk |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected; packet 194's point-in-time AC-9 invalidation is recorded in `requirements.md` §Acceptance Summary, not a status change).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Queue-final whole-plan gate: dispatch `cargo xtask test --summary --workspace` to a sub-agent with a `FACT pass/fail` return (CLAUDE.md §Test Discipline packet-close allowance; the xtask entry point guarantees the guest-freshness gate fires first — and, after this packet, the literals preflight).
- Record remaining packet-local risk (expected: the reviewer-count vs. tool-count residue delta, waiver count at close).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.

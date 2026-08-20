# Implementation Plan: 231-guest-closure-fingerprint

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Author the closure-walk tests (red)

- Task IDs: `TASK-342`
- Objective: add the twelve `build_guests::tests::*` tests named by AC-1 through AC-7, AC-15 and AC-N1 through AC-N4 against the not-yet-existing `guest_closure_input_paths` / `ClosureCache` / `ClosureError` API, so the API's shape is fixed by its callers before it is written.
- Precondition: packet 230 is `status: implemented`; `cargo test -p xtask` is green on the pre-change tree.
- Postcondition: the new tests exist and the crate does **not** compile (the symbols under test do not exist yet). This is the expected red state.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — long (well over the direct-read threshold; re-derive with `wc -l` if a number is needed); read only the `#[cfg(test)] mod tests` block (for the `TempDir` helper and the existing `GuestSpec` literal in `missing_fingerprint_metadata_is_stale`), plus `shared_input_paths`, `guest_input_paths`, `input_files`, `has_parent_path_dep`.
  - `crates/slicer-sdk/Cargo.toml` — small; the `[target.'cfg(not(target_arch = "wasm32"))'.dependencies] slicer-core` fixture shape for AC-2.
  - `modules/core-modules/classic-perimeters/wit-guest/Cargo.toml` and `modules/core-modules/classic-perimeters/Cargo.toml` — small; the AC-3 real-tree chain.
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
- Files explicitly out of bounds:
  - `xtask/src/test.rs`, `crates/pnp-cli-locator/src/lib.rs`, `docs/07_implementation_status.md` — later steps.
  - Every `docs/spec_packets/` directory other than this one.
- Blast-radius discipline: no struct field and no schema/version constant is added in this step. `GuestSpec` is unchanged; its single test-code literal (in `missing_fingerprint_metadata_is_stale`) is not on the `cargo xtask check-literals` watchlist because that gate watches `crates/*/src` and `GuestSpec` lives in `xtask/src`.
- Expected sub-agent dispatches:
  - Question: which discovered test guests under `crates/slicer-wasm-host/test-guests/` declare zero path dependencies in any dependency table? scope: `crates/slicer-wasm-host/test-guests/*/Cargo.toml`; return: `FACT` with the count and 3 example names — needed to pin AC-4's fixture guest and its "11 of 21" claim at implementation time.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — direct read of locked decisions C2 and C8 and of Round 5 finding R5-6.
- OrcaSlicer refs:
  - None. This packet ports no canonical algorithm.
- Verification:
  - `mkdir -p target && cargo test -p xtask build_guests 2>&1 | tee target/test-output.log | rg 'error\[E0425\]|error\[E0433\]|cannot find'` — FACT: the run fails to compile naming the not-yet-existing `guest_closure_input_paths`, proving the tests bind to the intended symbol rather than passing vacuously.
- Exit condition: exactly twelve new test functions exist with the names given in `packet.spec.md`, and the compile failure names `guest_closure_input_paths`. Falsified if `cargo test -p xtask build_guests` compiles and passes at this step — that would mean the tests assert against existing behaviour. Note that AC-15's test (`module_manifest_toml_edit_marks_core_guest_stale`) must fail on the *pre-change* input set for the right reason: a module `.toml` edit currently leaves the guest fresh because neither `guest_input_paths` nor `shared_input_paths` charges that file.

### Step 2: Implement the closure walk and delete the shared-set model

- Task IDs: `TASK-342`
- Objective: add `ClosureCache`, `ClosureError`, `path_dep_manifests` and `guest_closure_input_paths`; extend `guest_input_paths`' `GuestTree::Core` branch to charge every `*.toml` directly under the parent module directory (the module manifest `<module>/<module>.toml`, closing the C5 coverage hole); delete `shared_input_paths`, `compute_shared_freshness` and `stage_wit_snapshot` together with their two dedicated tests; re-thread `compute_guest_freshness`, `is_stale`, `build_one`'s post-build fingerprint call, packet 230's `CheckContext`, and `missing_fingerprint_metadata_is_stale`.
- Precondition: Step 1's tests exist and fail to compile.
- Postcondition: `cargo test -p xtask build_guests` is green, including AC-15 and AC-N3; no reference to the three deleted functions survives in `xtask/`.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/build_guests.rs` — long (well over the direct-read threshold; re-derive with `wc -l` if a number is needed); read only `shared_input_paths`, `guest_input_paths`, `compute_guest_freshness`, `is_stale`, `stage_wit_snapshot`, `build_one`'s fingerprint-recording tail, `snapshot_from_paths`, `fingerprint_entries`, `metadata_matches`, `fingerprint_metadata_path`, `discover_guests`, `CheckContext`, `stale_reason`, `check_command`, and the `#[cfg(test)] mod tests` block.
  - `crates/slicer-core/Cargo.toml` — small; the `[dev-dependencies] slicer-model-io` shape that AC-N1 and AC-N2 turn on.
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` — small; read only its `[stage]` header, to confirm the file `parse_stage_id_from_module_manifest` reads is the one being charged (AC-15).
- Files allowed to edit (at most 3):
  - `xtask/src/build_guests.rs`
- Files explicitly out of bounds:
  - `xtask/src/test.rs` — Step 3 owns it, even though it currently references `compute_shared_freshness`. Expect this step to leave `xtask/src/test.rs` non-compiling; that is the intended hand-off and Step 3's precondition.
  - `xtask/src/dist.rs`, `xtask/src/main.rs`.
  - `crates/pnp-cli-locator/src/lib.rs`, `docs/07_implementation_status.md`.
- Blast-radius discipline: no new struct field on a watched type and no schema/version constant is added. The one `GuestSpec` literal that must be updated is `missing_fingerprint_metadata_is_stale`, in the same file and the same edit budget; it changes only because its `compute_shared_freshness` call disappears, not because `GuestSpec` changed.
- Expected sub-agent dispatches:
  - Question: what is the exact declared name and type of `CheckContext`'s shared-snapshot field as packet 230 shipped it, and which functions read it? scope: `xtask/src/build_guests.rs`; return: `FACT` (<=5 lines).
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — locked decisions C2 and C8; Round 5 finding R5-6; direct read.
  - `docs/adr/0014-xtask-guest-discovery-via-validated-filesystem-walk.md` — short; direct read of "## Rejected alternatives" and "## Future reviewers" to confirm `cargo_metadata` stays rejected. Read-only: packet 232 edits this ADR, this step does not.
- OrcaSlicer refs:
  - None.
- Verification:
  - `mkdir -p target && cargo test -p xtask build_guests 2>&1 | tee target/test-output.log | rg '^test result:'; if rg -q '^test result: FAILED|test result: FAILED' target/test-output.log; then echo FAIL; else echo PASS; fi` — FACT pass/fail; bounded SNIPPETS on failure.
  - `if rg -q 'shared_input_paths|compute_shared_freshness|stage_wit_snapshot' xtask/src/build_guests.rs; then echo FAIL; else echo PASS; fi` — FACT PASS/FAIL (AC-8, this file's half).
  - `rg -q 'pub stage_id: Option<String>' xtask/src/build_guests.rs && rg -q 'parse_stage_id_from_module_manifest' xtask/src/build_guests.rs && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-9).
- Exit condition: all twelve Step-1 tests pass and the two deletion/retention greps return PASS. AC-15 must pass because the module `.toml` entered the input set, not because the guest was stale for an unrelated reason — assert in the same test that the guest is fresh before the `.toml` edit and stale after it. Falsified if AC-N3's test passes only because the guest under test has an empty *input set* rather than an empty *closure* — assert the guest's own `src/**` is still charged in the same test.

### Step 3: Make pnp_cli freshness unconditional

- Task IDs: `TASK-342`
- Objective: delete the mtime gate inside `ensure_pnp_cli_fresh_with` so the rebuild closure always runs; delete `newest_mtime_in`; add the AC-10 test; keep `pnp_cli_rebuild_abort_is_nonzero_with_named_failure_detail` passing.
- Precondition: Step 2 is complete; `xtask/src/test.rs` currently fails to compile because `build_guests::compute_shared_freshness` no longer exists.
- Postcondition: `xtask` compiles again; `cargo test -p xtask test::` is green; `xtask/src/test.rs` contains no `compute_shared_freshness` and no `fn newest_mtime_in`.
- Files allowed to read, with ranges when over 300 lines:
  - `xtask/src/test.rs` — long (over the direct-read threshold); read only `newest_mtime_in`, `PnpCliFreshness`, `ensure_pnp_cli_fresh`, `ensure_pnp_cli_fresh_with`, the `use` block at the top, and `pnp_cli_rebuild_abort_is_nonzero_with_named_failure_detail`.
- Files allowed to edit (at most 3):
  - `xtask/src/test.rs`
- Files explicitly out of bounds:
  - `xtask/src/build_guests.rs` — finished in Step 2; reopening it here means Step 2's exit condition was wrong.
  - `crates/pnp-cli-locator/src/lib.rs`, `docs/07_implementation_status.md`.
- Blast-radius discipline: not applicable; no struct field or version constant is added. `PnpCliFreshness` keeps both fields and both variants of behaviour.
- Expected sub-agent dispatches:
  - Question: after this edit, does `cargo clippy --workspace --all-targets -- -D warnings` report any unused-import or dead-code warning in `xtask/src/test.rs`? scope: clippy run; return: `FACT pass/fail` plus <=20 lines of the first failure.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/guest-freshness-artifact-verification-plan.md` — locked decision C7; direct read.
  - `CLAUDE.md` — "Test Discipline" and the `cargo xtask test` gated-entry-point subsection; direct read, because this step changes what that entry point does before every test run.
- OrcaSlicer refs:
  - None.
- Verification:
  - `mkdir -p target && cargo test -p xtask test:: 2>&1 | tee target/test-output.log | rg '^test result:'; if rg -q '^test result: FAILED|test result: FAILED' target/test-output.log; then echo FAIL; else echo PASS; fi` — FACT pass/fail.
  - `if rg -q 'compute_shared_freshness|fn newest_mtime_in' xtask/src/test.rs; then echo FAIL; else echo PASS; fi` — FACT PASS/FAIL (AC-11 and AC-8's second file).
- Exit condition: the AC-10 test proves the closure runs even with a binary newer than every source, and the pre-existing abort test still passes unchanged. Falsified if the AC-10 test passes because no `pnp_cli` binary exists on the test machine — construct the fixture so the binary is present and provably newer.

### Step 4: Reconcile the ADR-0054 mirror in `pnp-cli-locator`

- Task IDs: `TASK-342`
- Objective: update `staleness_reason`'s rustdoc so the model it documents is the dependency-closure fingerprint, discharging ADR-0054 Decision rule 5, without touching the signature, the message strings, or the crate's std-only dependency posture.
- Precondition: Steps 2 and 3 are complete, so `is_stale`'s final shape is on disk and can be described accurately.
- Postcondition: AC-12's grep returns PASS and the three tests in `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` still pass.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli-locator/src/lib.rs` — short (well under the direct-read threshold); full read permitted.
  - `docs/adr/0054-host-side-test-support-crate.md` — moderate; read "## Decision" (rules 1-5) only.
  - `xtask/src/build_guests.rs` — long (well over the direct-read threshold; re-derive with `wc -l` if a number is needed); re-read `is_stale` and `guest_closure_input_paths` only, to describe them correctly.
- Files allowed to edit (at most 3):
  - `crates/pnp-cli-locator/src/lib.rs`
- Files explicitly out of bounds:
  - `docs/adr/0054-host-side-test-support-crate.md` — this packet conforms to the ADR and does not amend it; see `design.md` §Locked Assumptions and Invariants.
  - `crates/slicer-runtime/tests/common/slicer_cache.rs` and `crates/slicer-runtime/tests/integration/pnp_cli_freshness_tdd.rs` — they must pass **unchanged**; editing them to make AC-13 green would be gaming the gate.
  - `xtask/**`, `docs/07_implementation_status.md`.
- Blast-radius discipline: no struct field or constant changes, so no literal fallout. The relevant blast radius is the *consumer* set, which must remain untouched: confirm it with the `LOCATIONS` dispatch below before editing, and confirm afterwards that no consumer file changed.
- Expected sub-agent dispatches:
  - Question: list every call site of `staleness_reason`, `newest_source_mtime` and `pnp_cli_bin` outside `crates/pnp-cli-locator/`. scope: `crates/**`; return: `LOCATIONS` (<=20 entries, one context line each).
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/0054-host-side-test-support-crate.md` — "## Decision" rules 1-5; direct ranged read.
  - `CLAUDE.md` — "In-Tree Citation Style (MUST follow)"; the rustdoc must cite `is_stale` by symbol name with the crate-qualified path `xtask/src/build_guests.rs` and must not introduce a line number.
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q 'is_stale' crates/pnp-cli-locator/src/lib.rs && rg -q 'xtask/src/build_guests.rs' crates/pnp-cli-locator/src/lib.rs && rg -q 'dependency closure' crates/pnp-cli-locator/src/lib.rs && if rg -q 'per-stage WIT package' crates/pnp-cli-locator/src/lib.rs; then echo FAIL; else echo PASS; fi` — FACT PASS/FAIL (AC-12).
  - `mkdir -p target && cargo test -p slicer-runtime --test integration pnp_cli_freshness_tdd 2>&1 | tee target/test-output.log | rg '^test result: ok\. 3 passed'` — FACT pass/fail (AC-13).
  - `git diff --name-only -- crates/slicer-runtime crates/slicer-scheduler | rg . && echo FAIL || echo PASS` — FACT PASS/FAIL: no consumer file was edited to make the tests pass.
- Exit condition: AC-12 PASS, AC-13 shows `3 passed`, and no file under `crates/slicer-runtime/` or `crates/slicer-scheduler/` appears in the diff. Falsified if the rustdoc still claims the fingerprint covers a per-stage WIT package.

### Step 5: Ledger row and closure gates

- Task IDs: `TASK-342`
- Objective: add the `TASK-342` row to `docs/07_implementation_status.md` under "### Workstream 5 — Governance and closure drift" in that section's local terser format, then run the packet's closure gates.
- Precondition: Steps 1-4 complete; all AC test commands pass individually.
- Postcondition: AC-14's grep returns PASS; `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` are green.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/07_implementation_status.md` — long; **do not read directly**. Dispatch the append; the section heading and the row format are given in the dispatch below.
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md`
- Files explicitly out of bounds:
  - `xtask/**`, `crates/**` — code work is finished; any change here means an earlier exit condition was wrong.
  - Every `docs/spec_packets/` directory other than this one.
- Blast-radius discipline: not applicable; no code changes in this step.
- Expected sub-agent dispatches:
  - Question: append one row under the heading `### Workstream 5 — Governance and closure drift` in `docs/07_implementation_status.md`, matching that section's existing terser style (for example `- [~] TASK-139 Close the DEV-020 source/docs drift ...` — a status box, the bare ID, then a verb phrase, with no em-dash after the ID). The row is `TASK-342`, describing the per-guest dependency-closure fingerprint, the deletion of `compute_shared_freshness` / `stage_wit_snapshot`, the unconditional `cargo build --bin pnp_cli` gate, and the `crates/pnp-cli-locator::staleness_reason` rustdoc reconciliation under ADR-0054 rule 5. Confirm with a grep. scope: `docs/07_implementation_status.md`; return: `FACT pass/fail` plus the appended line.
- Context cost: `S`
- Authoritative docs:
  - `docs/07_implementation_status.md` — delegated append only; never a direct read.
  - `CLAUDE.md` — "Ledger Facts Must Be Re-derived, Not Quoted (MUST follow)": re-derive that `TASK-342` is still free at the moment of writing (`rg -o '^\| ?TASK-[0-9]{3}|TASK-[0-9]{3}' docs/07_implementation_status.md | sort -u | tail -1`) and renumber if a parallel packet claimed it.
- OrcaSlicer refs:
  - None.
- Verification:
  - `rg -q '^- \[.\] TASK-342 ' docs/07_implementation_status.md && echo PASS || echo FAIL` — FACT PASS/FAIL (AC-14).
  - `cargo check --workspace --all-targets` — FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail.
  - `cargo xtask build-guests --check; echo "exit=$?"` — FACT: report the exit code. `0` = every guest fresh, `1` = at least one stale, and packet 230's `EXIT_INFRA_ERROR` = `wasm-tools` unavailable. Never grep for `STALE:` (R5-3).
- Exit condition: AC-14 PASS, both workspace gates green, and the `--check` exit code recorded. Falsified if `--check` returns packet 230's infrastructure code and that is reported as freshness.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Test authoring only; one file; ranged reads |
| Step 2 | M | Closure walk + module-manifest charge + 3 deletions + 3 re-threaded call sites in a long file |
| Step 3 | S | One function shrinks, one helper deleted, one test added |
| Step 4 | S | One rustdoc paragraph; consumer set confirmed by dispatch, not edited |
| Step 5 | S | Delegated ledger append plus the two workspace gates |

Aggregate: `M`. No step is L; no split is required before activation.

## Packet Completion Gate

- All five steps and their exit conditions complete.
- Every pipe-suffixed AC command in `packet.spec.md` returns PASS, re-run fresh rather than quoted from the step that first passed it.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- No reopened or superseded packet status transitions apply: this packet supersedes nothing and reopens nothing.
- Confirm packets 229 and 230 are `status: implemented` before flipping this packet's status.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command and the three packet-level gate commands.
- Record `cargo xtask build-guests --check`'s exit code as evidence, together with the count of guests reported stale. Quote no timing figure that was not measured in the session.
- Record remaining packet-local risk: the closure walk's under-approximation surface (any Cargo manifest table form not covered by AC-2) and the un-enforced ADR-0054 rustdoc obligation.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the subcommand supports it, so the test, bench, and example targets compile.

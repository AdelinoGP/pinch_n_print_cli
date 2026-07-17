# Implementation Plan: 165_cli-binary-locator-extraction

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".
- **Step 0 gate:** before any edit, dispatch the precondition check from `packet.spec.md` §Prerequisites. `BLOCKED` ⇒ stop; packet 162 has not landed and this packet must not run against the pre-162 tree.

## Steps

### Step 1: Author the ADR deciding the host-side test-support home

- Task IDs: `TASK-146d`
- Objective: write `docs/adr/<NNNN>-host-side-test-support-crate.md` at the next free number, recording the `slicer-test-support` decision and the five weighed alternatives per `design.md` §Code Change Surface.
- Precondition: Step 0 gate returned `READY`; the derived ADR number is fresh (derive it in this step, immediately before writing — `ls docs/adr | rg -o '^[0-9]{4}' | sort | tail -1`, +1; a parallel session may have consumed a number since any earlier derivation).
- Postcondition: the ADR file exists; AC-7's command prints `PASS`.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (72 lines, whole)
  - `crates/pnp-cli/Cargo.toml` (44 lines, whole)
  - `.ralph/specs/162_wit-lifecycle-export-removal/design.md` - §"CLI freshness — three sites, fixed in place" and §"Open Questions" only
- Files allowed to edit (at most 3):
  - `docs/adr/<NNNN>-host-side-test-support-crate.md` (new)
- Files explicitly out of bounds:
  - all `crates/**`, `Cargo.toml` (root) — no code in this step
- Expected sub-agent dispatches:
  - Question: "`ls docs/adr | rg -o '^[0-9]{4}' | sort | tail -1` — report the value"; scope: `docs/adr/`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/adr-0045-per-stage-wit-packages-plan.md` - §"Grounding corrections" items 1/4/6 only (ranged)
- OrcaSlicer refs: none — no parity content.
- Verification:
  - AC-7 command from `packet.spec.md` - FACT PASS/FAIL
- Exit condition: AC-7 prints `PASS`; the ADR names all of: `slicer-test-support`, ADR-0004, packet 78, the `pnp-cli` `test-support`-feature alternative with the `report` feature-unification analysis, and the `xtask` bin-only constraint. Missing any ⇒ step not done.

### Step 2: Create `crates/slicer-test-support` and register it

- Task IDs: `TASK-146d`
- Objective: create the crate (`Cargo.toml` with zero `[dependencies]` + `[lints] workspace = true`; `src/lib.rs` with `workspace_root`, `newest_source_mtime`, `staleness_reason`, `pnp_cli_bin` moved from the post-162 `slicer_cache.rs`, rustdoc citing the Step-1 ADR and the `is_stale` mirror pin) and add the workspace member line to the root `Cargo.toml`.
- Precondition: Step 1 done (the rustdoc cites the ADR by its real number). The moved code is copied from the post-162 `slicer_cache.rs` locator block — behavior-identical, message strings unchanged.
- Postcondition: `cargo check -p slicer-test-support` passes; the three original copies still exist (deleted in Step 3) — AC-2 is expected to FAIL at this point, by design.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/slicer_cache.rs` - locator block only (locate `pnp_cli_bin`, `staleness_reason`, `newest_source_mtime` by name)
  - `xtask/src/build_guests.rs` - `is_stale` fn only (locate by name)
  - `Cargo.toml` (root) - `[workspace] members` list only
- Files allowed to edit (at most 3):
  - `crates/slicer-test-support/Cargo.toml` (new)
  - `crates/slicer-test-support/src/lib.rs` (new)
  - `Cargo.toml` (root — one member line inside the `crates/*` block, before the first `modules/core-modules/` entry; the list is grouped, not alphabetical)
- Files explicitly out of bounds:
  - the three consumer sites (Step 3); `crates/pnp-cli/**`; `xtask/**` beyond the read-only `is_stale` lookup
- Expected sub-agent dispatches:
  - Question: "Run `cargo check -p slicer-test-support`; pass/fail + first 20 error lines on failure"; scope: workspace; return: `FACT` + SNIPPETS ≤20
- Context cost: `S`
- Authoritative docs:
  - `docs/adr/<NNNN>-host-side-test-support-crate.md` (Step 1 output) - whole
- OrcaSlicer refs: none — no parity content.
- Verification:
  - AC-1 command from `packet.spec.md` - FACT PASS/FAIL
- Exit condition: AC-1 prints `PASS`. A crate that compiles but exposes a fn-count other than exactly {1,1,1,1}, or carries any `[dependencies]` entry, fails the step.

### Step 3a: Point the two slicer-runtime sites at the crate

- Task IDs: `TASK-146d`
- Objective: add the `slicer-test-support` dev-dependency to `crates/slicer-runtime/Cargo.toml`; in `slicer_cache.rs` delete the moved fn bodies and add `pub use slicer_test_support::{pnp_cli_bin, staleness_reason, newest_source_mtime};`; in `gate_evidence.rs` delete the `pnp_cli_bin` mirror, import from the crate, and correct the module doc-comment whose self-containment justification is now void.
- Precondition: Step 2 done (the crate compiles and is a member).
- Postcondition: `cargo check -p slicer-runtime --all-targets` passes (tests + benches compile); no locator fn body remains in either file.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/slicer_cache.rs` - locator block + module header only
  - `crates/slicer-runtime/benches/gate_evidence.rs` (154 lines pre-162; read whole if grown ≤300, else locator block + doc-comment)
  - `crates/slicer-runtime/Cargo.toml` - `[dev-dependencies]` section only
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/Cargo.toml`
  - `crates/slicer-runtime/tests/common/slicer_cache.rs`
  - `crates/slicer-runtime/benches/gate_evidence.rs`
- Files explicitly out of bounds:
  - every file under `crates/slicer-runtime/tests/e2e/**` and `tests/integration/**` — the re-export exists so they need zero edits; touching one means the re-export is wrong
  - `crates/slicer-scheduler/**` (Step 3b)
- Expected sub-agent dispatches:
  - Question: "Run `cargo check -p slicer-runtime --all-targets`; pass/fail + first 20 error lines"; scope: workspace; return: `FACT` + SNIPPETS ≤20
- Context cost: `S`
- Authoritative docs:
  - `.ralph/specs/162_wit-lifecycle-export-removal/design.md` - §"CLI freshness" only (the message/shape contract being preserved)
- OrcaSlicer refs: none — no parity content.
- Verification:
  - `cargo check -p slicer-runtime --all-targets` - FACT pass/fail
  - `cd F:/slicerProject/pinch_n_print_cli && cargo bench -p slicer-runtime --bench gate_evidence --no-run 2>&1 | tail -3` - FACT pass/fail (compile-only; never run the bench)
- Exit condition: both commands pass AND `rg -c 'fn (pnp_cli_bin|staleness_reason|newest_source_mtime)\(' crates/slicer-runtime/` finds no match. A green check with a surviving local fn body means the site still shadows the crate — step not done.

### Step 3b: Point the slicer-scheduler site at the crate

- Task IDs: `TASK-146d`
- Objective: add the `slicer-test-support` dev-dependency to `crates/slicer-scheduler/Cargo.toml`; in `dag_cli_integration.rs` delete `fn bin()` and route all its call sites through `slicer_test_support::pnp_cli_bin()`.
- Precondition: Step 2 done. (Independent of Step 3a; either order.)
- Postcondition: `cargo check -p slicer-scheduler --all-targets` passes; `fn bin(` gone from the file.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs` (316 lines pre-162; whole if ≤300 post-162, else the header + `bin`/`workspace_root` block and the `Command::new` call sites by grep)
  - `crates/slicer-scheduler/Cargo.toml` - `[dev-dependencies]` section only
- Files allowed to edit (at most 3):
  - `crates/slicer-scheduler/Cargo.toml`
  - `crates/slicer-scheduler/tests/integration/dag_cli_integration.rs`
- Files explicitly out of bounds:
  - all other `crates/slicer-scheduler/tests/**`; `crates/slicer-scheduler/src/**`
- Expected sub-agent dispatches:
  - Question: "Run `cargo check -p slicer-scheduler --all-targets`; pass/fail + first 20 error lines"; scope: workspace; return: `FACT` + SNIPPETS ≤20
- Context cost: `S`
- Authoritative docs:
  - `.ralph/specs/162_wit-lifecycle-export-removal/design.md` - §"CLI freshness" only (the `cargo build -p pnp-cli` panic-message contract, which now must hold in the shared crate)
- OrcaSlicer refs: none — no parity content.
- Verification:
  - `cargo check -p slicer-scheduler --all-targets` - FACT pass/fail
  - AC-3 command from `packet.spec.md` - FACT PASS/FAIL (all three sites now migrated)
- Exit condition: AC-3 prints `PASS` and AC-2 prints `PASS` (single definition workspace-wide — first point in the plan where AC-2 can go green).

### Step 4: Full gates, baseline, and backlog row

- Task IDs: `TASK-146d`
- Objective: run every packet gate; add the TASK-146d row to `docs/07_implementation_status.md` by dispatch.
- Precondition: Steps 1–3b done.
- Postcondition: all AC commands PASS; backlog row present.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - `^test result` lines and failure context only (never re-run to see more output)
- Files allowed to edit (at most 3):
  - none directly (`docs/07_implementation_status.md` via dispatch only)
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` (dispatch, never read); all source files (no code edits in this step — a failure returns to the owning step)
- Expected sub-agent dispatches:
  - Question: "Run `cargo check --workspace --all-targets` then `cargo clippy --workspace --all-targets -- -D warnings`; pass/fail each + first 20 error lines on failure"; scope: workspace; return: `FACT` + SNIPPETS ≤20
  - Question: "Run the AC-4, AC-5, `perimeter_parity`, and `legacy_zero_matches_golden` commands from `requirements.md` §Verification Commands (each already rg-filtered); return the four `test result:` lines"; scope: workspace; return: `FACT` ≤5 lines
  - Question: "Run the AC-1, AC-2, AC-3, AC-7, AC-N1 audit commands; return the five PASS/FAIL lines"; scope: repo files; return: `FACT` ≤5 lines
  - Question: "Append the TASK-146d row to `docs/07_implementation_status.md` (TASK-119a/TASK-194a sub-letter convention); return the added line"; scope: `docs/07_implementation_status.md`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `CLAUDE.md` §"Test Discipline" - direct read (already loaded)
- OrcaSlicer refs: none — no parity content.
- Verification:
  - every pipe-suffixed AC command in `packet.spec.md` - FACT PASS/FAIL each
  - baseline: `perimeter_parity` reports `12 passed; 0 failed` and `legacy_zero_matches_golden` reports `1 passed; 0 failed` (both name-filtered; `0 passed` = FAIL)
- Exit condition: all ACs PASS, clippy clean, baseline green, `rg -q 'TASK-146d' docs/07_implementation_status.md` succeeds. Any red returns to the owning step; do not patch forward from here.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | ADR authoring; number derived in-step |
| Step 2 | S | code moved, not written |
| Step 3a | S | 3 files, mechanical |
| Step 3b | S | 2 files, mechanical |
| Step 4 | S | dispatch-only gates |

Aggregate `S`. No step approaches L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- No reopened/superseded transitions: packet 162 stays `implemented`; its `[FWD]` on locator extraction is resolved by this packet (note it in the closure report; do not edit 162's files).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- This packet's `packet.spec.md`/`requirements.md` do **not** require `cargo test --workspace`: the surface is dev-only plumbing; `--all-targets` check/clippy plus the targeted runs cover every consumer. Do not run it.
- Record remaining packet-local risk (expected: the scan-scope over-approximation noted in `design.md` §Risks).
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.

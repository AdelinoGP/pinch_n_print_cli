# Packet 82 — Implementation Plan

## Execution Rules

- Each step ends with a falsifying check that gates green before the next step starts.
- Files allowed to read per step are bounded; large files (`helpers_cmd.rs` 744 LOC, `report/*` 1 597 LOC total) are NEVER loaded in full. Use grep + delegated SUMMARY.
- The packet closure gate runs only after every step has gated green.
- P81 MUST be closed before this packet starts. The dispatch in Step 0 verifies the predecessor state.

---

## Step 0 — Verify P81 closure and capture pre-packet baselines

**Objective.** Confirm P81 left a clean state: `slicer-model-io` exists, `slicer-runtime/src/helpers_cmd.rs` imports `slicer_model_io`, no `slicer-runtime::model_loader` paths remain. Capture four `mesh *` subcommand output SHAs as the AC-7 baseline.

**Precondition.** P81 is `status: superseded`. Working tree clean.

**Postcondition.** All four SHAs recorded in the implementation log: `mesh convert`, `mesh repair`, `mesh decimate`, `mesh import` against canonical fixtures.

**Files allowed to read.** None directly.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: confirm `test -f crates/slicer-model-io/Cargo.toml && grep -qE 'slicer_model_io::' crates/slicer-runtime/src/helpers_cmd.rs`. Return FACT pass/fail.
- Dispatch: capture SHA for each: `cargo run --bin pnp_cli --release -- mesh convert --input resources/benchy.stl --output /tmp/p82-pre-convert.obj --format obj && sha256sum /tmp/p82-pre-convert.obj`. Repeat for `repair`, `decimate`, `import`. Return SNIPPETS (4 lines, one per subcommand).

**Context cost: S.**

**Narrow verification.** P81 verification returns pass; 4 SHA strings captured.

**Falsifying check / exit condition.** Any of the four subcommands fails to run → P81 incomplete; abort this packet.

---

## Step 1 — Enumerate `SliceRunOptions`, test imports, and report call sites

**Objective.** Surface every consumer of items being moved or gated, so Step 3 doesn't miss a site.

**Precondition.** Step 0 green.

**Postcondition.** Three lists in the implementation log:
- (a) `SliceRunOptions` consumers outside `cli.rs`.
- (b) Test files importing `HostCli` / `HostCommands` / `helpers_cmd::*`.
- (c) `run.rs` lines referencing `report::*` or `report_alloc::*`.

**Files allowed to read.** None directly — three dispatches.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch #1: "Is `SliceRunOptions` referenced anywhere outside `cli.rs`?" Scope: `crates/slicer-runtime/src/`, `crates/pnp-cli/src/`, `crates/slicer-runtime/tests/`. Return LOCATIONS (≤ 10 entries).
- Dispatch #2: "Which test files under `crates/slicer-runtime/tests/` import `HostCli`, `HostCommands`, or `slicer_runtime::helpers_cmd::*`?" Return LOCATIONS (≤ 20 entries).
- Dispatch #3: "Which lines in `crates/slicer-runtime/src/run.rs` reference `report::*` or `report_alloc::*`?" Return LOCATIONS (file:line, ≤ 10 entries).

**Context cost: S.**

**Narrow verification.** Three lists exist. Empty lists are acceptable and confirmed by the dispatch returning "no matches".

**Falsifying check / exit condition.** A site surfaces at `cargo build` in Step 3 that is not on any list → return here and widen the search.

---

## Step 2 — Add the `report` feature to `slicer-runtime/Cargo.toml` and propagate to pnp-cli's Cargo.toml

**Objective.** Establish the feature gate at the manifest level before any source `#[cfg]` is added.

**Precondition.** Step 1 lists in hand.

**Postcondition.** `crates/slicer-runtime/Cargo.toml` has `[features] default = ["report"] report = []`. `crates/pnp-cli/Cargo.toml` has `[features] default = ["report"] report = ["slicer-runtime/report"]`. `cargo build --workspace` still green (no source `#[cfg]` yet; behavior unchanged).

**Files allowed to read.** `crates/slicer-runtime/Cargo.toml`, `crates/pnp-cli/Cargo.toml`.
**Files allowed to edit.**
1. `crates/slicer-runtime/Cargo.toml`.
2. `crates/pnp-cli/Cargo.toml`.

**Expected sub-agent dispatch.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** Build green. `grep -qE '^default *= *\["report"\]' crates/slicer-runtime/Cargo.toml` passes.

**Falsifying check / exit condition.** Build fails → likely a feature-syntax typo. Revert and retry.

---

## Step 3 — Move helpers_cmd to pnp-cli; lift OutputFormat + write_with_parents; delete cli.rs's dead types; gate report

**Objective.** The bulk move. After this step, AC-1, AC-2, AC-3, AC-4 all gate green; both `cargo build --workspace` and `cargo build --no-default-features -p slicer-runtime` are green.

**Precondition.** Step 2 complete.

**Postcondition.** `cargo build --workspace && cargo build --no-default-features -p slicer-runtime` both green. `slicer-runtime/src/{cli.rs,helpers_cmd.rs}` deleted. `pnp-cli` builds and dispatches the four `mesh *` subcommands to functions that live in `crates/pnp-cli/src/commands/`.

**Files allowed to read.**
- `crates/slicer-runtime/src/cli.rs` (271 LOC — OK to load in full).
- `crates/slicer-runtime/src/helpers_cmd.rs` (744 LOC — line ranges only; use grep for the four `pub fn` signatures and their bodies).
- `crates/slicer-runtime/src/lib.rs`, `crates/slicer-runtime/src/run.rs` (gate-site identification per Step 1 dispatch #3).
- `crates/pnp-cli/src/main.rs` and any existing submodules.

**Files allowed to edit.**
1. `crates/pnp-cli/src/` — new `commands/` subtree (or single `helpers_cmd.rs` sibling) + `io.rs` for `write_with_parents`. Counted as one primary target.
2. `crates/slicer-runtime/src/lib.rs` — remove `pub mod cli;` + `pub mod helpers_cmd;` + their `pub use ...;`. Gate `pub mod report;` and its re-exports with `#[cfg(feature = "report")]`. Keep `pub mod dag_cli;`.
3. `crates/slicer-runtime/src/run.rs` — `#[cfg(feature = "report")]` guards on each site enumerated by Step 1 dispatch #3.
4. `crates/pnp-cli/src/main.rs` — wire the new `commands::` module; dispatch subcommands; gate `--report` flag definition + handler with `#[cfg(feature = "report")]`.
5. Delete `crates/slicer-runtime/src/cli.rs` and `crates/slicer-runtime/src/helpers_cmd.rs`.

If Step 1 dispatch #1 returned `SliceRunOptions` consumers in `slicer-runtime::run.rs`, move the `SliceRunOptions` definition INTO `run.rs` before deleting `cli.rs`. If no consumers exist, delete it with `cli.rs`.

**Expected sub-agent dispatches.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail.
- Dispatch: `cargo build --no-default-features -p slicer-runtime`. Return FACT pass/fail.
- Dispatch: `cargo clippy --workspace --all-targets -- -D warnings`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.**
1. `test ! -f crates/slicer-runtime/src/helpers_cmd.rs && test ! -f crates/slicer-runtime/src/cli.rs` → pass.
2. `! grep -qE '^pub mod (cli|helpers_cmd);' crates/slicer-runtime/src/lib.rs` → pass.
3. `grep -qE '^default *= *\["report"\]' crates/slicer-runtime/Cargo.toml` → pass.
4. Both `cargo build` invocations green.

**Falsifying check / exit condition.** Default build fails: typically a missed `pub use` cleanup. No-default-features build fails: a missed `#[cfg(feature = "report")]` guard at a `report::*` call site → return to Step 1 dispatch #3.

---

## Step 4 — Migrate or delete tests; rewire test imports

**Objective.** Tests that imported deleted symbols are deleted or rewired. Aggregator `mod` declarations are cleaned up.

**Precondition.** Step 3 complete.

**Postcondition.** `cargo test -p slicer-runtime -p pnp-cli` green. Any test referencing `HostCli`/`HostCommands` is deleted (those types are gone). Any test exercising the four `mesh *` helper functions either moves to `crates/pnp-cli/tests/` or is deleted if redundant with a pnp-cli equivalent.

**Files allowed to read.** Test files identified in Step 1 dispatch #2.
**Files allowed to edit.**
1. `crates/slicer-runtime/tests/integration/main.rs`, `crates/slicer-runtime/tests/executor/main.rs` — drop `mod` declarations for migrated/deleted tests.
2. The test files identified by dispatch #2 — delete or move.
3. `crates/pnp-cli/tests/**` — receive any migrated integration tests for the helper subcommands.

**Expected sub-agent dispatches.**
- Dispatch: `cargo test -p slicer-runtime`. Return FACT pass/fail + count.
- Dispatch: `cargo test -p pnp-cli`. Return FACT pass/fail + count.

**Context cost: M.**

**Narrow verification.** Both test runs pass. Aggregator files no longer reference removed test modules.

**Falsifying check / exit condition.** A test fails because of a stale import → fix the import (forwarding to `pnp-cli`'s parser types where applicable).

---

## Step 5 — Confirm AC-7 SHA parity for the four `mesh *` subcommands

**Objective.** Each subcommand's output SHA matches the Step 0 baseline.

**Precondition.** Steps 3 and 4 green.

**Postcondition.** Four post-packet SHAs match four Step 0 baselines.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatches.**
- Dispatch: per subcommand, run the command from Step 0 and `sha256sum` the output. Return SNIPPETS (4 lines).
- Dispatch: compare to Step 0 baselines. Return FACT match/mismatch per subcommand.

**Context cost: S.**

**Narrow verification.** Four-for-four matches.

**Falsifying check / exit condition.** A SHA differs → bisect by reverting each portion of the helpers_cmd move until parity returns; identify the divergent edit.

---

## Step 6 — Verify AC-8 report HTML still works on default features

**Objective.** Confirm the report file is generated and structurally valid on the default build.

**Precondition.** Step 3 green.

**Postcondition.** `pnp_cli slice ... --report /tmp/p82-report.html` produces a non-empty HTML file containing the expected sentinel strings.

**Files allowed to read.** `docs/16_slicer_report.md` if dispatch #8 didn't already cache its sentinel patterns.
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p82.gcode --report /tmp/p82-report.html && head -5 /tmp/p82-report.html`. Return SNIPPETS (5 lines).

**Context cost: S.**

**Narrow verification.** First line matches `<!DOCTYPE html`. File size > 1 KB.

**Falsifying check / exit condition.** Report flag silently ignored or file empty → the feature wiring in `pnp-cli` is wrong; the `--report` handler probably wasn't ungated for the default build.

---

## Step 7 — AC-N2 ceremony: confirm report symbols are excluded under `--no-default-features`

**Objective.** Prove the feature gate is load-bearing.

**Precondition.** Step 3 green.

**Postcondition.** A working-tree-only experiment that adds a `slicer_runtime::report::Collector::new(...)` call under `--no-default-features` fails to compile with `unresolved import` or similar; the experiment is reverted.

**Files allowed to read.** None.
**Files allowed to edit.** None permanently (the experiment is git-stash-friendly).

**Expected sub-agent dispatch.**
- Dispatch: write a temp test file that imports `slicer_runtime::report::Collector`, run `cargo build --no-default-features -p slicer-runtime --tests`. Return FACT (expected: fail with "unresolved import").

**Context cost: S.**

**Narrow verification.** The build fails with the documented unresolved-import error; the temp file is removed.

**Falsifying check / exit condition.** The build succeeds → the gate isn't excluding `report::*` from compilation; investigate `lib.rs` for missing `#[cfg]`.

---

## Per-Step Budget Roll-Up

| Step | Cost |
|---|---|
| 0 P81 verification + 4 baseline SHAs | S |
| 1 Enumerate consumers | S |
| 2 Add `[features]` block | S |
| 3 Bulk move + cli.rs delete + report gate | M |
| 4 Test migration / rewiring | M |
| 5 AC-7 SHA parity check | S |
| 6 AC-8 report HTML structural check | S |
| 7 AC-N2 gate-exclusion ceremony | S |

Aggregate: **M** (no L step). Total step count: 8.

## Packet Completion Gate

Per the deepening-batch policy (deviation recorded in P81), the workspace-wide `cargo test --workspace` is NOT required at P82 close.

1. `cargo build --workspace` — green.
2. `cargo build --no-default-features -p slicer-runtime` — green.
3. `cargo clippy --workspace --all-targets -- -D warnings` — green.
4. `cargo test -p slicer-runtime -p pnp-cli` — green; counts delta net zero (any test count reduction matches deleted dead-type tests).
5. `cargo xtask build-guests --check` — clean.
6. AC-7 four SHA matches against Step 0 baselines.
7. AC-8 report HTML structural check passes.
8. AC-N2 ceremony documented as performed.

## Acceptance Ceremony

- All 9 ACs (AC-1 .. AC-9) and 2 negative cases (AC-N1, AC-N2) gate green per the inline verification commands in `packet.spec.md`.
- The implementation log records: P81 verification status, four pre/post SHAs for `mesh *` subcommands, list of deleted test files, list of `#[cfg(feature = "report")]` sites added.
- `status: draft` → `status: superseded` after gate green AND user confirms closure.

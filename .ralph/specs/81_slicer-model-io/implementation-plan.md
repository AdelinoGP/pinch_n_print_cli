# Packet 81 — Implementation Plan

## Execution Rules

- Each step is atomic and ends with its own falsifying check. A step is "done" only when its postcondition gates green.
- Files allowed to read per step are bounded; delegate sub-agent dispatches before pulling large files into the implementer's own context.
- No step may take more than the listed `Files allowed to edit` set; the test of "scope creep" is "would another file change appear in this step's diff?".
- The packet closure gate (the bottom of this file) runs only after every step has gated green.

---

## Step 0 — Capture pre-packet g-code SHA baseline

**Objective.** Record the SHA of `pnp_cli slice resources/benchy.stl ...` before any source edit, so AC-6 has a falsifiable target.

**Precondition.** Working tree clean on `master` (or branch base). `resources/benchy.stl` exists.

**Postcondition.** A baseline SHA is recorded in the implementation log AND captured in a working file outside the source tree (e.g., `/tmp/p81-baseline.sha`).

**Files allowed to read.** None (executes a command).
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: run `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-prep81.gcode` then `sha256sum /tmp/benchy-prep81.gcode`. Return FACT `<hex>`.

**Context cost: S.**

**Narrow verification.** The dispatch returns a 64-character hex SHA. Implementer logs it.

**Falsifying check / exit condition.** No SHA captured → re-run before proceeding.

---

## Step 1 — Enumerate `model_loader`'s external consumers

**Objective.** Build the precise list of `pub(crate)` items that need promotion to `pub` AND the precise list of test files that import `model_loader`/`model_writer`/`model_loader_sidecar` symbols.

**Precondition.** Step 0 complete.

**Postcondition.** Two lists exist in the implementation log:
- (a) `pub(crate)` items to promote (with their `crates/slicer-runtime/src/model_loader.rs:LINE` ref each).
- (b) Test files to update or move (with `crates/slicer-runtime/tests/*:LINE`).

**Files allowed to read.** None directly — delegate.
**Files allowed to edit.** None (this step is discovery).

**Expected sub-agent dispatches.**
- Dispatch #1: "Which `pub(crate)` symbols in `crates/slicer-runtime/src/model_loader.rs` are referenced from `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/src/helpers_cmd.rs`, or `crates/slicer-runtime/tests/`?" → LOCATIONS (≤ 20 entries).
- Dispatch #2: "Which files under `crates/slicer-runtime/tests/` import `model_loader::`, `model_writer::`, or `model_loader_sidecar::`?" → LOCATIONS (≤ 20 entries).

**Context cost: S.**

**Narrow verification.** Both dispatches return non-empty LOCATIONS lists or "no matches". If empty: confirm the search scope was correct before proceeding.

**Falsifying check / exit condition.** A consumer surfaces at `cargo build` in step 3 that is not on either list → return to this step and widen the search.

---

## Step 2 — Create the `slicer-model-io` crate scaffold

**Objective.** New crate exists; empty `lib.rs` compiles; `Cargo.toml` declares the right deps; workspace `Cargo.toml` knows about it.

**Precondition.** Step 1 lists in hand.

**Postcondition.** `cargo build -p slicer-model-io` succeeds against the empty lib.rs.

**Files allowed to read.** `Cargo.toml` (workspace), `crates/slicer-runtime/Cargo.toml` (to copy the dep versions).
**Files allowed to edit.**
1. `Cargo.toml` (workspace) — add `"crates/slicer-model-io"` to `members`.
2. `crates/slicer-model-io/Cargo.toml` — CREATE.
3. `crates/slicer-model-io/src/lib.rs` — CREATE (initially a single `// placeholder` line plus a module declaration that will be filled in step 3).

**Expected sub-agent dispatch.**
- Dispatch: `cargo build -p slicer-model-io`. Return FACT pass/fail.

**Context cost: S.**

**Narrow verification.** `cargo build -p slicer-model-io` green.

**Falsifying check / exit condition.** Build fails → fix `Cargo.toml` (likely a workspace inheritance issue or a missing feature flag on `zip`/`uuid`).

---

## Step 3 — Move the three files, promote symbols, delete from runtime, rewire `lib.rs` + `run.rs` + `helpers_cmd.rs` + `Cargo.toml`

**Objective.** The bulk move. After this step, AC-1, AC-2, AC-3, AC-4, AC-5 all gate green; the workspace builds.

**Precondition.** Step 2 complete; the lists from Step 1 in hand.

**Postcondition.** `cargo build --workspace` green. No `slicer-runtime/src/model_*` files remain. `slicer-runtime/Cargo.toml` no longer declares the five file-format deps.

**Files allowed to read.**
- `crates/slicer-runtime/src/model_loader.rs` (line-range only — do not load the full 2 439 lines; use grep + delegated SUMMARY for any range > 200 lines).
- `crates/slicer-runtime/src/model_loader_sidecar.rs` (253 LOC — OK to load in full).
- `crates/slicer-runtime/src/model_writer.rs` (194 LOC — OK to load in full).
- `crates/slicer-runtime/src/lib.rs`, `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/src/helpers_cmd.rs`, `crates/slicer-runtime/Cargo.toml`.
- `crates/pnp-cli/src/main.rs` (and any submodule the slice subcommand lives in), `crates/pnp-cli/Cargo.toml`.

**Files allowed to edit.** All 14 files listed in `design.md` §"Code Change Surface". Three primary edit targets: the new `slicer-model-io` crate (counts as one), `slicer-runtime/src/lib.rs`, `slicer-runtime/src/run.rs`. The rest are mechanical follow-on (Cargo.toml deletions, import rewrites in `helpers_cmd.rs`, slice-subcommand insertion in pnp-cli).

**Expected sub-agent dispatches.**
- Dispatch: `cargo build --workspace`. Return FACT pass/fail + first failing crate name if fail.
- Dispatch: `cargo clippy --workspace --all-targets -- -D warnings`. Return FACT pass/fail.

**Context cost: M.**

**Narrow verification.**
1. `test ! -f crates/slicer-runtime/src/model_loader.rs && test ! -f crates/slicer-runtime/src/model_loader_sidecar.rs && test ! -f crates/slicer-runtime/src/model_writer.rs` → pass.
2. `! grep -qE '^(stl_io|tobj|zip|quick-xml|uuid) *=' crates/slicer-runtime/Cargo.toml` → pass.
3. `grep -E 'pub fn run_slice' crates/slicer-runtime/src/run.rs | head -1 | grep -qE '(MeshIR|Arc<MeshIR>)'` → pass.
4. `cargo build --workspace` → pass.

**Falsifying check / exit condition.** Build error referencing a missing `pub(crate)` symbol → return to Step 1 and add it to the promotion list.

---

## Step 4 — Migrate / update tests; confirm narrow per-crate test gates

**Objective.** Tests whose SUT moved are now in `slicer-model-io/tests/`; tests that use loaders as fixtures still in `slicer-runtime/tests/` but with rewritten imports. The two test-aggregator files (`tests/integration/main.rs`, `tests/executor/main.rs`) lose `mod` declarations for moved tests if any.

**Precondition.** Step 3 complete; workspace builds.

**Postcondition.** `cargo test -p slicer-runtime -p slicer-model-io -p pnp-cli` green. At least the three round-trip tests (STL, OBJ, 3MF) in `slicer-model-io/tests/` exist and pass.

**Files allowed to read.** The test files identified in Step 1's dispatch #2 LOCATIONS.
**Files allowed to edit.**
1. `crates/slicer-model-io/tests/*` — CREATE/MOVE as needed.
2. `crates/slicer-runtime/tests/integration/main.rs`, `crates/slicer-runtime/tests/executor/main.rs` — drop `mod` declarations for any moved tests.
3. The non-moved runtime tests that use loaders as fixtures — rewrite `use slicer_runtime::model_loader::...` to `use slicer_model_io::...`.

**Expected sub-agent dispatches.**
- Dispatch: `cargo test -p slicer-model-io`. Return FACT pass/fail + pass count.
- Dispatch: `cargo test -p slicer-runtime`. Return FACT pass/fail + pass count + delta vs pre-packet.
- Dispatch: `cargo test -p pnp-cli`. Return FACT pass/fail + pass count.

**Context cost: M.**

**Narrow verification.** All three test runs pass. Test-count delta in `slicer-runtime`: zero (if no tests moved) or negative (if N tests moved into `slicer-model-io`, with the same N appearing as positive delta there).

**Falsifying check / exit condition.** A test that previously passed now fails → check whether the failure is (a) a missed import rewrite (fixable in step 4), (b) a SHA divergence that should surface in Step 5's AC-6 (jump to Step 5).

---

## Step 5 — Confirm AC-6 byte-identical g-code

**Objective.** Run the same slice that produced the baseline SHA in Step 0; confirm the post-packet SHA matches.

**Precondition.** Steps 0–4 complete; build green, narrow tests green.

**Postcondition.** Post-packet SHA = pre-packet SHA from Step 0. Both SHAs recorded in the implementation log.

**Files allowed to read.** None.
**Files allowed to edit.** None.

**Expected sub-agent dispatch.**
- Dispatch: `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p81.gcode && sha256sum /tmp/benchy-p81.gcode`. Return FACT `<hex>`.

**Context cost: S.**

**Narrow verification.** The two SHAs match.

**Falsifying check / exit condition.** SHAs differ → bisect: the divergence is in one of the moved files OR in the `run_slice` signature reshape. Bisect by re-running the slice with intermediate states (loader change reverted; loader change in but signature reshape reverted; etc.) until the divergent edit is isolated.

---

## Step 6 — Verify the seam is enforced by the dep graph (AC-N2 ceremony)

**Objective.** Prove that the runtime cannot regress by accidentally reaching back to file I/O.

**Precondition.** Step 5 green.

**Postcondition.** A working-tree-only patch that adds `slicer-model-io` as a `slicer-runtime` dep and calls `load_mesh` from `run.rs` is demonstrated to compile (proving the seam is conventional today but enforceable by removing the dep). The patch is reverted before proceeding.

**Files allowed to read.** None.
**Files allowed to edit.** None permanently. The ceremony is "git stash" friendly: change, observe, revert.

**Expected sub-agent dispatch.**
- Dispatch: `cargo tree -p slicer-runtime --depth 5 --edges normal 2>&1 | grep -E '\b(stl_io|tobj|zip|quick-xml|uuid)\b' | wc -l`. Expected: 0. Return FACT `<integer>`.

**Context cost: S.**

**Narrow verification.** The integer returned is exactly 0.

**Falsifying check / exit condition.** Non-zero → a transitive dep is leaking. Investigate via `cargo tree -p slicer-runtime --duplicates` to find the indirect path.

---

## Per-Step Budget Roll-Up

| Step | Cost |
|---|---|
| 0 Capture baseline SHA | S |
| 1 Enumerate consumers | S |
| 2 Crate scaffold | S |
| 3 Bulk move + dep delete + signature reshape | M |
| 4 Tests migration / rewiring | M |
| 5 Confirm AC-6 SHA match | S |
| 6 AC-N2 ceremony | S |

Aggregate: **M** (no L step). Total step count: 7.

## Packet Completion Gate

All of the following must gate green before the packet moves from `draft` to `superseded` (i.e., closed). Per the deepening-batch policy (recorded in `docs/DEVIATION_LOG.md` at P81 close), the workspace-wide `cargo test --workspace` is NOT required at P81 close — it runs only at P83, P85, P88. P81 closes on the narrow gates below.

1. `cargo build --workspace` — green.
2. `cargo clippy --workspace --all-targets -- -D warnings` — green.
3. `cargo test -p slicer-model-io -p slicer-runtime -p pnp-cli` — green; `slicer-runtime` count delta ≤ 0 (any reduction matches a test moved into `slicer-model-io`).
4. `cargo xtask build-guests --check` — clean. (This packet edits no guest-feeding path; STALE here is unrelated and is a pre-existing drift to flag, not an introduced regression.)
5. AC-6 post-packet SHA equals Step 0 baseline SHA.
6. AC-N1 dep-tree check empty.

## Acceptance Ceremony

- All 8 ACs (AC-1 .. AC-8) and 2 negative cases (AC-N1, AC-N2) gate green per the inline verification commands in `packet.spec.md`.
- The implementation log records: pre-packet SHA, post-packet SHA, `slicer-runtime` test count pre/post, `slicer-model-io` test count, list of `pub(crate)` symbols promoted (from Step 1 dispatch #1), list of test files moved (from Step 1 dispatch #2).
- `docs/DEVIATION_LOG.md` gains an entry recording the relaxed workspace-test policy for this batch (P81 only — the entry is written once, lands in P81, referenced by P82..P88).
- `status: draft` → `status: superseded` once gate green AND the user confirms closure. (Per the spec-generator skill default — the packet does not flip itself to active.)

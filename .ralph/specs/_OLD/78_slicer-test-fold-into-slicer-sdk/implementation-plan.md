# Implementation Plan — Packet 78

## Execution Rules

- This packet hard-depends on packet 77 being closed (`status: implemented`). Step 1 verifies this preflight.
- Steps are atomic and ordered. The fold-and-delete ordering is load-bearing: source files MUST move before the `slicer-test` crate is deleted. Skipping ahead breaks the workspace.
- All cargo invocations delegated to sub-agents returning `FACT: pass/fail + ≤ 5 lines`. The implementer never absorbs full cargo stdout.
- Narrow tests only. **Do not run `cargo test --workspace`** at any step. The closure gate uses `cargo test -p slicer-sdk -p arachne-perimeters -p rectilinear-infill -p pnp-cli --test module_new_tdd` (four packages, fast).
- Workspace `Cargo.toml` member-list edits regenerate `Cargo.lock`. Commit the lockfile change in the same commit as the member edit.

## Steps

### Step 1 — Preflight: verify packet 77 closed; baseline workspace member count

- **Task IDs**: TASK-225
- **Objective**: Confirm packet 77's `status: implemented` and capture the current workspace-member count (should be 30; will become 29 by AC-1).
- **Precondition**: This packet is `draft`; packet 77's `packet.spec.md` should exist with `status: implemented`.
- **Postcondition**: A baseline count is recorded; if packet 77 is not closed, stop and surface the blocker.
- **Files to read**: `.ralph/specs/77_test-support-wire-and-adapter/packet.spec.md` (frontmatter only).
- **Files to edit**: none.
- **Expected dispatches**: dispatch 5 (workspace member count).
- **Context cost**: S
- **Narrow verification**: `grep -E '^status:' .ralph/specs/77_test-support-wire-and-adapter/packet.spec.md | grep -q implemented && cargo metadata --format-version=1 --no-deps`
- **Exit condition**: packet 77 closed; baseline = 30 members.

### Step 2 — Move source files; update internal `use` paths

- **Task IDs**: TASK-225
- **Objective**: Source files relocated. Internal cross-file `use` paths inside the moved code corrected.
- **Precondition**: Step 1 complete.
- **Postcondition**: `crates/slicer-sdk/src/test_support/{mock_host,capture,fixtures,assert_paths}.rs` exist; `crates/slicer-test/src/{mock_host,capture,fixtures,assert_paths}.rs` no longer exist; `crates/slicer-sdk/src/test_support/mod.rs` declares the four new submodules.
- **Files to read**: `crates/slicer-test/src/lib.rs` (confirms what's exported), the four `slicer-test/src/*.rs` source files (for internal `use` paths).
- **Files to edit**:
  - `crates/slicer-sdk/src/test_support/mod.rs` (add `pub mod mock_host; pub mod capture; pub mod fixtures; pub mod assert_paths;` under the existing feature gate)
  - Move the four source files (via `git mv` if using git, or write-new + delete-old)
  - Adjust `use slicer_ir::*` paths if any moved file used a relative path that no longer resolves
- **Expected dispatches**: dispatch 1 (file enumeration in `slicer-test`).
- **Context cost**: M
- **Narrow verification**: `cargo check -p slicer-sdk --features test --tests`
- **Exit condition**: clean check.

### Step 3 — Move test files; rename with `test_support_*` prefix; absorb smoke

- **Task IDs**: TASK-225
- **Objective**: All test files from `crates/slicer-test/tests/` relocated to `crates/slicer-sdk/tests/` with disambiguating prefix.
- **Precondition**: Step 2 complete.
- **Postcondition**: 9 new files in `crates/slicer-sdk/tests/` (5 original `slicer-test` tests + 4 from packet 77, all with `test_support_` prefix); content of `crates/slicer-test/tests/smoke.rs` absorbed into the existing `crates/slicer-sdk/tests/smoke.rs`; `crates/slicer-test/tests/` no longer exists.
- **Files to read**: every `crates/slicer-test/tests/*.rs` (each ≈ 50-200 lines), `crates/slicer-sdk/tests/smoke.rs` (recon shows it exists; load to know how to extend).
- **Files to edit**:
  - The 9 moved test files (rename + relocate)
  - `crates/slicer-sdk/tests/smoke.rs` (extend with absorbed content)
  - `crates/slicer-sdk/tests/main.rs` if present (per dispatch 2 — add `mod test_support_*;` entries)
- **Expected dispatches**: dispatch 2 (sdk-tests aggregator check).
- **Context cost**: M
- **Narrow verification**: `cargo test -p slicer-sdk` (delegated; FACT pass/fail + 9 file names confirmed via `ls crates/slicer-sdk/tests/`).
- **Exit condition**: `slicer-sdk` test suite still green (all moved tests now run from their new location).

### Step 4 — Create `test_prelude.rs`; wire into `lib.rs`

- **Task IDs**: TASK-225
- **Objective**: AC-3 satisfied. Module authors can `use slicer_sdk::test_prelude::*` in test files.
- **Precondition**: Steps 1-3 complete.
- **Postcondition**: `crates/slicer-sdk/src/test_prelude.rs` exists, gated whole-module, re-exports the documented symbols; `crates/slicer-sdk/src/lib.rs` declares `pub mod test_prelude;` under the same feature gate; `crates/slicer-sdk/src/prelude.rs` is unchanged (negative-grep verifiable).
- **Files to read**: `crates/slicer-sdk/src/prelude.rs` (47 lines per recon — to confirm what to NOT touch).
- **Files to edit**:
  - `crates/slicer-sdk/src/test_prelude.rs` (new)
  - `crates/slicer-sdk/src/lib.rs` (add one line under the feature gate, near the existing `pub mod test_support;`)
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: `cargo check -p slicer-sdk --features test && head -3 crates/slicer-sdk/src/test_prelude.rs | grep -qE '^\#!\[cfg' && for sym in MockHost ConfigViewBuilder SliceRegionViewBuilder square_polygon rect_path; do grep -q "$sym" crates/slicer-sdk/src/test_prelude.rs || exit 1; done && ! grep -qE 'MockHost|ConfigViewBuilder' crates/slicer-sdk/src/prelude.rs`
- **Exit condition**: all checks pass.

### Step 5 — Rewrite `pnp_cli module new` scaffold + assertions

- **Task IDs**: TASK-225
- **Objective**: AC-6 satisfied. Scaffold emits `[dev-dependencies] slicer-sdk = { ..., features = ["test"] }` only.
- **Precondition**: Steps 1-4 complete.
- **Postcondition**: `crates/pnp-cli/src/module_new.rs:188-207` rewritten; `crates/pnp-cli/src/module_new.rs:545` assertion updated; `crates/pnp-cli/tests/module_new_tdd.rs:36` assertion updated; `cargo test -p pnp-cli --test module_new_tdd` green.
- **Files to read**: `crates/pnp-cli/src/module_new.rs` lines 180-220 + 530-560 (only those windows; the file is ≥ 600 lines).
- **Files to edit**:
  - `crates/pnp-cli/src/module_new.rs` (lines 200-207 + 545)
  - `crates/pnp-cli/tests/module_new_tdd.rs` (line 36)
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: `cargo test -p pnp-cli --test module_new_tdd && grep -qE 'features = \[.*"test".*\]' crates/pnp-cli/src/module_new.rs && ! grep -qE 'slicer-test' crates/pnp-cli/src/module_new.rs`
- **Exit condition**: all checks pass.

### Step 6 — Remove workspace member; commit Cargo.lock regeneration

- **Task IDs**: TASK-225
- **Objective**: `Cargo.toml` member-list shrinks; lockfile regenerates.
- **Precondition**: Steps 1-5 complete. NOTHING in the workspace should still reference `slicer-test` by this point (verify with grep).
- **Postcondition**: `"crates/slicer-test",` removed from `Cargo.toml:10`; `Cargo.lock` regenerated and committed in the same commit; `cargo check --workspace --all-targets` clean.
- **Files to read**: `Cargo.toml` (workspace root, 110 lines).
- **Files to edit**:
  - `Cargo.toml` (one line removed)
  - `Cargo.lock` (commit the auto-regenerated diff)
- **Expected dispatches**: dispatch 10 (workspace clippy) after the edit.
- **Context cost**: S
- **Narrow verification**: `! grep -qE '"crates/slicer-test"' Cargo.toml && cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings`
- **Exit condition**: all clean; lockfile diff staged.

### Step 7 — Delete `crates/slicer-test/` directory

- **Task IDs**: TASK-225
- **Objective**: AC-1 satisfied. The crate no longer exists.
- **Precondition**: Step 6 complete (workspace doesn't reference the crate any more).
- **Postcondition**: `test ! -d crates/slicer-test` true; workspace builds clean.
- **Files to read**: none (`ls crates/slicer-test` only to confirm what's being deleted).
- **Files to edit**:
  - Delete `crates/slicer-test/` entirely (via `rm -rf` or `git rm -r`)
- **Expected dispatches**: dispatch 5 (recount workspace members → should now be 29).
- **Context cost**: S
- **Narrow verification**: `bash -c 'test ! -d crates/slicer-test && [ "$(awk "/^members[[:space:]]*=[[:space:]]*\[/,/^\]/" Cargo.toml | grep -cE "^[[:space:]]*\"[^\"]+\"")" = "29" ]'` (no Python; counts entries in the workspace `members = [...]` block).
- **Exit condition**: directory gone; member count 29 (was 30 pre-fold).

### Step 8 — Verify the gate is real (AC-5 manual probe; AC-N1 production-build symbol scan)

- **Task IDs**: TASK-225
- **Objective**: Confirm `test_support` is truly unreachable from non-test, non-feature production builds.
- **Precondition**: Steps 1-7 complete.
- **Postcondition**: AC-5's manual probe documented (probe added, error captured, probe removed, clean re-check); AC-N1's `nm` scan of `target/release/*.rlib` returns no `test_support` symbols.
- **Files to read**: `crates/slicer-sdk/src/lib.rs` (47 lines).
- **Files to edit** (temporary, then revert):
  - `crates/slicer-sdk/src/lib.rs` — add `pub use crate::test_support::MockHost as _gate_probe;` near top; run `cargo check -p slicer-sdk` (no features); record the error message; **remove the probe**; run `cargo check -p slicer-sdk` again; confirm clean.
- **Expected dispatches**: dispatch 8 (`nm` scan).
- **Context cost**: S
- **Narrow verification**: `! grep -qE '_gate_probe' crates/slicer-sdk/src/lib.rs && cargo build --workspace --release && ! find target/release -name '*.rlib' -exec nm {} \; 2>/dev/null | grep -qE 'test_support::(MockHost|ConfigViewBuilder)'`
- **Exit condition**: probe gone, release build clean of test_support symbols.

### Step 9 — Migrate `arachne-perimeters` (exemplar #1)

- **Task IDs**: TASK-226
- **Objective**: AC-7 + AC-9 satisfied for arachne-perimeters.
- **Precondition**: Steps 1-8 complete.
- **Postcondition**: `modules/core-modules/arachne-perimeters/Cargo.toml` gains the dev-dep line; six `make_*` helpers in `modules/core-modules/arachne-perimeters/tests/*.rs` rewritten to single-expression builder chains; every original assertion preserved verbatim; `cargo test -p arachne-perimeters` green.
- **Files to read**: every `modules/core-modules/arachne-perimeters/tests/*.rs` (per dispatch 3, extract the field-name surface first); `modules/core-modules/arachne-perimeters/src/lib.rs` (only the config-key strings used — to confirm the migration's setter keys match the production code).
- **Files to edit**:
  - `modules/core-modules/arachne-perimeters/Cargo.toml` (one line addition under `[dev-dependencies]`)
  - `modules/core-modules/arachne-perimeters/tests/*.rs` (helper bodies rewritten; `use slicer_sdk::test_prelude::*;` added)
- **Expected dispatches**: dispatch 3 (pre-migration field-name extraction).
- **Context cost**: M
- **Narrow verification**: `cargo test -p arachne-perimeters && grep -A5 '\[dev-dependencies\]' modules/core-modules/arachne-perimeters/Cargo.toml | grep -qE 'slicer-sdk.*features = \[.*"test".*\]'`
- **Exit condition**: tests green; dev-dep wired.

### Step 10 — Verify dev-dep is load-bearing (AC-N2 manual probe)

- **Task IDs**: TASK-226
- **Objective**: AC-N2 documented. Temporarily remove the `features = ["test"]` from arachne-perimeters' dev-dep line; confirm `cargo test -p arachne-perimeters` fails with a clear unresolved-symbol error; restore the line.
- **Precondition**: Step 9 complete.
- **Postcondition**: AC-N2's documented failure mode is recorded (in this step's implementation notes — not committed to a test file).
- **Files to read**: none.
- **Files to edit** (temporary, then revert):
  - `modules/core-modules/arachne-perimeters/Cargo.toml` — remove `features = ["test"]`; observe failure; restore.
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: After restore, `cargo test -p arachne-perimeters` is green again. The transient failure during the probe is the proof.
- **Exit condition**: restoration confirmed green.

### Step 11 — Migrate `rectilinear-infill` (exemplar #2)

- **Task IDs**: TASK-226
- **Objective**: AC-8 + AC-9 satisfied for rectilinear-infill.
- **Precondition**: Steps 1-10 complete.
- **Postcondition**: Same as Step 9 but for rectilinear-infill's six `make_*` helpers (`make_square_expolygon`, `make_test_region`, `make_config`, `make_square_region`, two `make_bridge_region` variants).
- **Files to read**: every `modules/core-modules/rectilinear-infill/tests/*.rs`; `modules/core-modules/rectilinear-infill/src/lib.rs` config-key strings.
- **Files to edit**:
  - `modules/core-modules/rectilinear-infill/Cargo.toml` (+1 dev-dep line)
  - `modules/core-modules/rectilinear-infill/tests/*.rs` (helper bodies rewritten). **NOTE**: `make_square_expolygon` is defined in **two** files (`top_bottom_fill_tdd.rs` ≈ L18 and `bridge_infill_emission_tdd.rs` ≈ L19) — both bodies must be rewritten in lockstep. AC-8's grep deliberately catches both occurrences and asserts each body is ≤ 4 lines.
- **Expected dispatches**: dispatch 4 (pre-migration field-name extraction).
- **Context cost**: M
- **Narrow verification**: `cargo test -p rectilinear-infill && grep -A5 '\[dev-dependencies\]' modules/core-modules/rectilinear-infill/Cargo.toml | grep -qE 'slicer-sdk.*features = \[.*"test".*\]'`
- **Exit condition**: tests green; dev-dep wired.

### Step 12 — Structural rewrite of `docs/05_module_sdk.md:445-624`

- **Task IDs**: TASK-226
- **Objective**: AC-10 satisfied. Section heading renamed; every `use slicer_test::*` becomes `use slicer_sdk::test_prelude::*`; opening line cites ADR-0004 + the scaffold convention.
- **Precondition**: Steps 1-11 complete.
- **Postcondition**: `docs/05_module_sdk.md` matches AC-10's grep gates.
- **Files to read**: `docs/05_module_sdk.md` lines 445-624 (only that window); `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (≈ 60 lines from packet 77 — quote its Decision paragraph).
- **Files to edit**: `docs/05_module_sdk.md` (range edits only).
- **Expected dispatches**: dispatch 9 (post-edit doc grep gate).
- **Context cost**: M
- **Narrow verification**: AC-10's compound command.
- **Exit condition**: command returns clean.

### Step 13 — Update `docs/00_project_overview.md` crate inventory

- **Task IDs**: TASK-226
- **Objective**: AC-11 satisfied. No row for `slicer-test`.
- **Precondition**: Step 7 complete (the crate truly doesn't exist).
- **Postcondition**: `docs/00_project_overview.md` contains zero `slicer-test` references.
- **Files to read**: `docs/00_project_overview.md` lines 115-165 (the crate table + directory tree window).
- **Files to edit**: `docs/00_project_overview.md` (range edits only).
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: `! grep -qE 'slicer-test' docs/00_project_overview.md`
- **Exit condition**: grep clean.

### Step 14 — Update `CLAUDE.md` for `slicer-test` references

- **Task IDs**: TASK-226
- **Objective**: Project root `CLAUDE.md` no longer mentions the deleted crate.
- **Precondition**: Step 13 complete.
- **Postcondition**: `CLAUDE.md` `slicer-test` mentions are either deleted or rewritten to point at `slicer_sdk::test_support` / `slicer_sdk::test_prelude`.
- **Files to read**: project root `CLAUDE.md` (its sections that mention test crates).
- **Files to edit**: `CLAUDE.md` (text edits only).
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: `grep -c 'slicer-test' CLAUDE.md` — accept the count to be 0, OR for every remaining occurrence, the implementer documents in this step's notes why it stays (e.g., a historical reference in commentary that doesn't claim the crate exists).
- **Exit condition**: count ≤ acceptable threshold (typically 0).

### Step 15 — Final wasm-target gate (AC-4)

- **Task IDs**: TASK-226
- **Objective**: AC-4 satisfied. Production wasm builds do not pull in `test_support`.
- **Precondition**: Steps 1-14 complete.
- **Postcondition**: Both `cargo check --target wasm32-unknown-unknown -p arachne-perimeters` and `-p rectilinear-infill` clean; `cargo tree --target wasm32-unknown-unknown -p arachne-perimeters` shows `slicer-sdk` WITHOUT `feature="test"`.
- **Files to read**: none.
- **Files to edit**: none.
- **Expected dispatches**: dispatch 6 (wasm check) + dispatch 7 (cargo tree feature check).
- **Context cost**: M (cargo build for wasm is heavier than native check).
- **Narrow verification**: AC-4's command from `packet.spec.md`.
- **Exit condition**: clean wasm checks + cargo-tree confirms no `test` feature.

## Per-Step Budget Roll-Up

| Step | Cost | Cumulative | Notes |
|---|---|---|---|
| 1 | S | S | Preflight |
| 2 | M | M | Source moves |
| 3 | M | M+M=L⁻ | Test moves — checkpoint at 60% |
| 4 | S | L⁻ | test_prelude |
| 5 | S | L⁻ | Scaffold |
| 6 | S | L⁻ | Cargo.toml |
| 7 | S | L⁻ | Delete |
| 8 | S | L⁻ | Gate probe |
| 9 | M | L | Exemplar #1 — checkpoint |
| 10 | S | L | AC-N2 probe |
| 11 | M | L | Exemplar #2 |
| 12 | M | L | docs/05 |
| 13 | S | L | docs/00 |
| 14 | S | L | CLAUDE.md |
| 15 | M | L | wasm gate |

**Aggregate**: L. **No single step is L** — every step's individual cost is S or M, so the packet stays activatable. The L aggregate reflects 15 sequential steps × M-ish average, not a single oversized step. Implementer should plan for a fresh-context handoff between Step 8 (gate verified) and Step 9 (exemplar migrations begin) if context utilization hits 60% — those two sub-blocks (1-8 = fold; 9-15 = migrate + docs) are clean handoff boundaries.

## Packet Completion Gate

Run sequentially as the final closure check. Each delegated.

1. `cargo xtask build-guests --check` — if STALE: rebuild via `cargo xtask build-guests` (drop `--check`), then re-run with `--check`. Required because the workspace-member removal may trigger a build-script input recomputation.
2. `cargo check --workspace --all-targets`
3. `cargo check -p slicer-sdk` (no features) — confirms `test_support` is gated.
4. `cargo check --target wasm32-unknown-unknown -p arachne-perimeters` AND `cargo check --target wasm32-unknown-unknown -p rectilinear-infill`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test -p slicer-sdk -p arachne-perimeters -p rectilinear-infill -p pnp-cli --test module_new_tdd`

**Do not run `cargo test --workspace`** — narrow per-package sweep is the gate.

## Acceptance Ceremony

After the completion gate passes:

- Update `packet.spec.md` frontmatter: `status: implemented`, add `closed: <ISO date>`.
- Append closure detail to `docs/07_implementation_status.md`: change TASK-225 and TASK-226 from `[ ]` to `[x]`, add `Closed YYYY-MM-DD — packet 78; verified by <test names>` suffix.
- Open follow-up: packet 79 may now proceed with the bulk migration + builder extension; mark its `requires: 78` prerequisite as resolved when 79 is activated.
- The Cargo.lock regeneration is committed in Step 6's commit, not separately at ceremony time — the diff is part of the load-bearing structural change.

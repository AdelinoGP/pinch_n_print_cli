# Implementation Plan — Packet 77

## Execution Rules

- TDD discipline applies: for each behavioral change, write the failing test first, confirm red, then write the production code, confirm green.
- Steps are atomic and ordered. Each step's exit condition must be green before the next starts.
- Sub-agent delegations follow the contracts in `design.md` §Expected Sub-Agent Dispatches. Do not absorb full cargo output into the implementer's context — return `FACT: pass/fail + ≤ 5 lines of context`.
- Narrow tests only. **Do not run `cargo test --workspace`** at any step. The closure gate uses `cargo test -p slicer-test -p slicer-macros -p slicer-sdk` (three packages, fast).
- After Step 3 (the macro edit), `cargo xtask build-guests --check` will report STALE; rebuild without `--check` once before continuing, then again at the closure gate.

## Steps

### Step 1 — Add `test` feature + scaffold `test_support` module (red side of the contract)

- **Task IDs**: TASK-223
- **Objective**: Make `slicer_sdk::test_support` exist as a feature-gated empty module, so the macro's future fully-qualified path is resolvable when the feature is on.
- **Precondition**: Current tree builds clean (`cargo check --workspace --all-targets`).
- **Postcondition**: `cargo check -p slicer-sdk --features test` clean; `cargo check -p slicer-sdk` (no features) clean; `slicer_sdk::test_support` module exists with stub `reset_global_state` / `install_panic_handler` / `mock_host_setup` / `mock_host_teardown` (each `pub fn name() {}`).
- **Files to read**: `crates/slicer-sdk/src/lib.rs` (full, 47 lines — small), `crates/slicer-sdk/Cargo.toml` (15 lines).
- **Files to edit**:
  - `crates/slicer-sdk/Cargo.toml` (add `[features] test = []`)
  - `crates/slicer-sdk/src/lib.rs` (add `#[cfg(any(test, feature = "test"))] pub mod test_support;`)
  - `crates/slicer-sdk/src/test_support/mod.rs` (new, stub bodies)
- **Expected dispatches**: none — direct edits.
- **Context cost**: S
- **Authoritative docs**: none (mechanical).
- **OrcaSlicer refs**: none.
- **Narrow verification**: `cargo check -p slicer-sdk --features test && cargo check -p slicer-sdk`
- **Exit condition**: both `cargo check` invocations clean; `grep -A1 '^\[features\]' crates/slicer-sdk/Cargo.toml | grep -qE '^test = \[\]$'` returns 0.

### Step 2 — Implement the four hook bodies using existing `host::test_support` primitives

- **Task IDs**: TASK-223
- **Objective**: Make AC-2/AC-6/AC-7 implementable. Replace stub bodies with the four functions per `design.md` §Controlling Code Paths code block.
- **Precondition**: Step 1 complete.
- **Postcondition**: All four functions have real bodies. `install_panic_handler` chains via `take_hook` + `set_hook`. `reset_global_state` and `mock_host_teardown` both drain log buffer + clear mesh source. `mock_host_setup` installs log capture.
- **Files to read**: `crates/slicer-sdk/src/host.rs:108-343` (delegate dispatch 2 from `design.md` if not already in cache).
- **Files to edit**: `crates/slicer-sdk/src/test_support/mod.rs`.
- **Expected dispatches**: dispatch 2 (`host::test_support` signatures) if not yet confirmed.
- **Context cost**: S
- **Narrow verification**: `cargo test -p slicer-sdk` (existing tests must still pass; the new module is unreferenced at this point).
- **Exit condition**: green `cargo test -p slicer-sdk`; eyeball-confirmation that the panic hook captures `take_hook()` *before* `set_hook(...)`.

### Step 3 — Rewrite the `#[module_test]` macro to fully-qualified calls

- **Task IDs**: TASK-223
- **Objective**: Stop emitting unqualified `__slicer_test_*` identifiers; emit `::slicer_sdk::test_support::*` instead in both `quote!` branches.
- **Precondition**: Steps 1+2 complete.
- **Postcondition**: AC-3 satisfied. Macro file contains zero `__slicer_test_` substrings outside the new `::slicer_sdk::test_support::` paths.
- **Files to read**: `crates/slicer-macros/src/lib.rs` lines 2680-2760 only (delegate dispatch 1 from `design.md` to get the current expansion verbatim).
- **Files to edit**: `crates/slicer-macros/src/lib.rs` (the two `quote!` blocks, lines ≈ 2715-2733 and 2736-2754).
- **Expected dispatches**: dispatch 1 (current expansion snippet).
- **Context cost**: S
- **Narrow verification**: `cargo check -p slicer-macros && grep -nE 'slicer_test_(reset_global_state|install_panic_handler|mock_host_(setup|teardown))' crates/slicer-macros/src/lib.rs | grep -vE 'slicer_sdk::test_support'`
- **Exit condition**: grep returns empty; `cargo check -p slicer-macros` clean.
- **Guest WASM staleness check**: After this step, `cargo xtask build-guests --check` will report STALE. Rebuild via `cargo xtask build-guests` (without `--check`) before Step 8.

### Step 4 — Refactor `MockHost` as a `MeshSource` adapter; update `slicer-test/Cargo.toml`; add `prelude`

- **Task IDs**: TASK-224
- **Objective**: AC-4 satisfied. `MockHost` implements `MeshSource`; builder methods exist; `install(self)` calls `host::test_support::install_mesh_source`; `log_warn` routes through `host::log_warn`. `slicer-test::prelude` re-exports the documented API surface.
- **Precondition**: Steps 1-3 complete.
- **Postcondition**: `cargo check -p slicer-test --tests` clean; the file contains `impl slicer_sdk::host::MeshSource for MockHost`, the four with-* builder methods, and `install(self)`. `slicer-test/src/lib.rs` exports `pub mod prelude`.
- **Files to read**: `crates/slicer-sdk/tests/host_wrappers_tdd.rs` lines 67-100 (the `StubMesh` precedent the new shape mirrors — confirm trait method signatures match), `crates/slicer-test/src/{fixtures.rs,capture.rs,assert_paths.rs}` headers only (to know what `prelude` re-exports).
- **Files to edit**:
  - `crates/slicer-test/src/mock_host.rs` (full rewrite, ≈ 200 LoC net after the new structure)
  - `crates/slicer-test/Cargo.toml` (change `slicer-sdk` dep to include `features = ["test"]`)
  - `crates/slicer-test/src/lib.rs` (+ `pub mod prelude;` and the prelude file)
  - `crates/slicer-test/src/prelude.rs` (new — re-exports MockHost, ConfigViewBuilder, SliceRegionViewBuilder, PerimeterRegionViewBuilder, square_polygon, rect_path, the three capture types, all `assert_paths_*`)
- **Expected dispatches**: none — the headers fit in direct reads.
- **Context cost**: M
- **Narrow verification**: `cargo check -p slicer-test --tests` and `cargo clippy -p slicer-test --all-targets -- -D warnings`.
- **Exit condition**: both clean.

### Step 5 — Write the four new TDDs (AC-5, AC-6, AC-7, AC-N2)

- **Task IDs**: TASK-224
- **Objective**: AC-5, AC-6, AC-7, AC-N2 all green.
- **Precondition**: Step 4 complete (production code in place so tests can go green immediately; per project convention, the new tests do not need a separate red phase since they are protocol-locking regressions, not feature drivers — the feature drivers were the AC text itself).
- **Postcondition**: Four new test files exist under `crates/slicer-test/tests/`, all named per AC text, each containing the corresponding test function name from the AC.
- **Files to read**: existing tests under `crates/slicer-test/tests/smoke.rs` for the project's test boilerplate style.
- **Files to edit**:
  - `crates/slicer-test/tests/mock_host_adapter_tdd.rs` (new — AC-5 + AC-N2)
  - `crates/slicer-test/tests/mock_host_isolation_tdd.rs` (new — AC-6)
  - `crates/slicer-test/tests/log_capture_round_trip_tdd.rs` (new — log buffer round-trip)
  - `crates/slicer-test/tests/panic_handler_drains_logs_tdd.rs` (new — AC-7)
- **Expected dispatches**: none.
- **Context cost**: M
- **Narrow verification**: `cargo test -p slicer-test --test mock_host_adapter_tdd --test mock_host_isolation_tdd --test log_capture_round_trip_tdd --test panic_handler_drains_logs_tdd`
- **Exit condition**: all four tests green.
- **Note for AC-7**: `panic_handler_drains_logs_tdd.rs` uses `std::panic::catch_unwind` to keep the test runner happy. The "captured logs printed to stderr" assertion uses `gag` would normally be the cleanest, but to avoid adding a dep, the test installs a prior `std::panic::set_hook` that flips a thread-local before panic; the test asserts that the flag was flipped (proving chain delegation) and `host::test_support::take_log_messages()` is empty after the panic (proving drain).

### Step 6 — Update `slicer-macros` dev-deps + delete local stubs in `smoke.rs` / `module_test_tdd.rs`

- **Task IDs**: TASK-224
- **Objective**: AC-8 satisfied. `slicer-macros` tests no longer carry the `__slicer_test_*` local stubs; they rely on the real `slicer-sdk::test_support`.
- **Precondition**: Steps 1-5 complete.
- **Postcondition**: `cargo test -p slicer-macros --test smoke --test module_test_tdd` green; grep for `fn __slicer_test_*` in those two files returns empty.
- **Files to read**: `crates/slicer-macros/tests/smoke.rs` (full, ≈ 60 lines), `crates/slicer-macros/tests/module_test_tdd.rs` (full, ≈ 440 lines — this one is larger; load only the test-body sections where the tracking flags are asserted).
- **Files to edit**:
  - `crates/slicer-macros/Cargo.toml` (add `slicer-sdk = { path = "../slicer-sdk", features = ["test"] }` to `[dev-dependencies]`)
  - `crates/slicer-macros/tests/smoke.rs` (delete the four local `__slicer_test_*` `pub fn` stubs)
  - `crates/slicer-macros/tests/module_test_tdd.rs` (delete the four local stubs; rewrite tracking-flag-based assertions to use observable behavior — e.g., after a `#[module_test] fn t() { host::log_warn("X"); }`, the test outside asserts `host::test_support::take_log_messages()` would be empty if called after teardown — equivalent observable check)
- **Expected dispatches**: none.
- **Context cost**: M
- **Narrow verification**: `cargo test -p slicer-macros --test smoke --test module_test_tdd && grep -qE 'fn __slicer_test_(mock_host_setup|mock_host_teardown|install_panic_handler|reset_global_state)' crates/slicer-macros/tests/{smoke,module_test_tdd}.rs && exit 1; exit 0`
- **Exit condition**: tests green, grep returns empty.

### Step 7 — Clean doc fictions in `docs/05_module_sdk.md:445-624`

- **Task IDs**: TASK-224
- **Objective**: AC-9 satisfied. The doc no longer references APIs that don't exist.
- **Precondition**: Steps 1-6 complete (the production code that the docs now describe is in place).
- **Postcondition**: Lines 445-624 contain none of the six forbidden tokens; replacement examples use the actual `MockHost` builder API and `region_id(42)` u64 form.
- **Files to read**: `docs/05_module_sdk.md:445-624` only (delegate dispatch 3 from `design.md` first to enumerate every offending line).
- **Files to edit**: `docs/05_module_sdk.md` (range edits only, no whole-file rewrite).
- **Expected dispatches**: dispatch 3 (line-by-line inventory of forbidden tokens).
- **Context cost**: M
- **Narrow verification**: `awk 'NR>=445 && NR<=624' docs/05_module_sdk.md | grep -nE 'circle_polygon|rect_polygon|path_length\(|with_logging|clip_polygons_call_count|region_id\("'`
- **Exit condition**: grep returns empty.

### Step 8 — Write ADR-0004; verify gate-is-real (AC-N1)

- **Task IDs**: TASK-224
- **Objective**: AC-10 + AC-N1 satisfied.
- **Precondition**: Steps 1-7 complete.
- **Postcondition**: `docs/adr/0004-test-support-lives-in-slicer-sdk.md` exists with the required sections; AC-N1's gate-is-real probe was run and the temporary probe line was removed.
- **Files to read**: `docs/adr/0001-prepass-builtins-commit-in-stage.md` (style template; ≈ 50 lines).
- **Files to edit**:
  - `docs/adr/0004-test-support-lives-in-slicer-sdk.md` (new)
  - (temporary, then revert) `crates/slicer-sdk/src/lib.rs` — add `pub use crate::test_support::reset_global_state as _gate_probe;` near top, run `cargo check -p slicer-sdk` (no features), record the error message, **remove the probe line**, run `cargo check -p slicer-sdk` again, confirm clean. Document the round-trip in this step's implementation notes.
- **Expected dispatches**: none.
- **Context cost**: S
- **Narrow verification**: `test -f docs/adr/0004-test-support-lives-in-slicer-sdk.md && grep -qE '^## Status$' docs/adr/0004-test-support-lives-in-slicer-sdk.md && grep -qE 'rename slicer-test to slicer-sdk-test' docs/adr/0004-test-support-lives-in-slicer-sdk.md && grep -qE 'delete slicer-test outright' docs/adr/0004-test-support-lives-in-slicer-sdk.md && cargo check -p slicer-sdk`
- **Exit condition**: all of the above true; the probe line is gone (`grep -c '_gate_probe' crates/slicer-sdk/src/lib.rs` returns 0).

## Per-Step Budget Roll-Up

| Step | Cost | Cumulative |
|---|---|---|
| 1 | S | S |
| 2 | S | S+S = M |
| 3 | S | M |
| 4 | M | M+M = L⁻ — checkpoint at 60% |
| 5 | M | L⁻ |
| 6 | M | L |
| 7 | M | L (steady-state; doc-only) |
| 8 | S | L |

**Aggregate**: M (large M; not L because no single step is L; ratchet by step 4 prompts a checkpoint).

## Packet Completion Gate

Run sequentially as the final closure check. Each delegated to a sub-agent returning `FACT: clean/failed`.

1. `cargo xtask build-guests --check` — if STALE: rebuild via `cargo xtask build-guests` (drop `--check`), then re-run with `--check`. Required because step 3's macro change affects bindgen text in every guest.
2. `cargo check --workspace --all-targets`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test -p slicer-test -p slicer-macros -p slicer-sdk`

**Do not run `cargo test --workspace`** — narrow per-package sweep is the gate.

## Acceptance Ceremony

After the completion gate passes:

- Update `packet.spec.md` frontmatter: `status: implemented`, add `closed: <ISO date>`.
- Append closure detail to `docs/07_implementation_status.md`: change TASK-223 and TASK-224 from `[ ]` to `[x]`, add `Closed YYYY-MM-DD — packet 77; verified by <test names>` suffix.
- The four AC verification commands recorded in `packet.spec.md` are the closure evidence — capture exit codes in the commit message.
- Open a follow-up: packet 78 may now proceed. Mark its `requires: 77` prerequisite as resolved in packet 78's `packet.spec.md` when 78 is activated.

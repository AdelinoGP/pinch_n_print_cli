# Implementation Plan: 29_slicer-cli-cmd-run-cross-platform

## Step 1 — Add `host_binary_name()` helper + thread it into both call sites

**Task IDs**: DEV-031
**Objective**: Introduce a single internal helper that resolves the host
binary name from `SLICER_HOST_BIN` (default `"slicer-host"`), and update
both `check_host_binary()` and the `Command::new(...)` invocation in
`execute_in` to consume it. Production behavior unchanged when the env
var is unset.
**Precondition**: None.
**Postcondition**:
- `cli/slicer-cli/src/cmd_run.rs` defines `fn host_binary_name() -> String`.
- `check_host_binary()` calls `Command::new(host_binary_name())`.
- The `Command::new(...)` call in `execute_in` (around line 202) uses
  `host_binary_name()`.
- `cargo build --workspace` exits 0.
- `cargo test -p slicer-cli` runs the suite (the two target tests still
  fail at this step — fixes land in Steps 2 and 3).
**Files**: `cli/slicer-cli/src/cmd_run.rs`.
**Verification**:
- `grep -nE 'fn host_binary_name|host_binary_name\(\)' cli/slicer-cli/src/cmd_run.rs`
  returns ≥3 matches (definition + 2 call sites).
- `grep -nE 'Command::new\(\"slicer-host\"\)' cli/slicer-cli/src/cmd_run.rs`
  returns 0 matches outside comments (no remaining hardcoded
  `Command::new("slicer-host")`).
- `cargo build --workspace 2>&1 | tail -3` exits 0.
**Exit**: Helper exists; both call sites resolved through it; build green.
**OrcaSlicer refs**: None.

## Step 2 — Add `TestEnvGuard` RAII helper + fix `execute_in_reaches_host_check`

**Task IDs**: DEV-031
**Objective**: Add a small `TestEnvGuard` RAII helper inside the `mod
tests {}` block that saves and restores a single env var on `Drop`.
Use it to set `SLICER_HOST_BIN=__slicer_host_test_marker_does_not_exist__`
for the duration of `execute_in_reaches_host_check`.
**Precondition**: Step 1 complete.
**Postcondition**:
- `cli/slicer-cli/src/cmd_run.rs::tests` defines `struct TestEnvGuard`
  with `set(key, value)`, `unset(key)`, and a `Drop` impl that restores
  the prior value (`Some(v) => set_var(key, v)` or `None => remove_var(key)`).
- `execute_in_reaches_host_check` now starts with
  `let _guard = TestEnvGuard::set("SLICER_HOST_BIN", "__slicer_host_test_marker_does_not_exist__");`
- `cargo test -p slicer-cli cmd_run::tests::execute_in_reaches_host_check
  -- --test-threads=1` exits 0 on a developer machine where
  `slicer-host` is on `PATH`.
**Files**: `cli/slicer-cli/src/cmd_run.rs`.
**Verification**:
- `grep -nE 'struct TestEnvGuard|impl Drop for TestEnvGuard' cli/slicer-cli/src/cmd_run.rs`
  returns ≥2 matches.
- `grep -nE 'TestEnvGuard::set\(\"SLICER_HOST_BIN\"' cli/slicer-cli/src/cmd_run.rs`
  returns ≥1 match inside `execute_in_reaches_host_check`.
- `cargo test -p slicer-cli cmd_run::tests::execute_in_reaches_host_check
  -- --test-threads=1 --nocapture 2>&1 | tail -5` shows `test result:
  ok. 1 passed`.
**Exit**: The host-binary test passes regardless of whether
`slicer-host` is on the developer's `PATH`.
**OrcaSlicer refs**: None.

## Step 3 — Fix `find_wasm_binary_correct_path_from_module_name` for Windows separators

**Task IDs**: DEV-031
**Objective**: Replace the
`path.to_string_lossy().contains("target/slicer/cool-perimeters.wasm")`
substring assertion with a `PathBuf`-aware equality check that works on
Windows, Linux, and macOS. Add a `// LEGACY_BROKEN_FORM:` comment
documenting the prior assertion so future maintainers do not
reintroduce it.
**Precondition**: Step 2 complete.
**Postcondition**:
- `find_wasm_binary_correct_path_from_module_name` asserts
  `path == dir.path().join("target").join("slicer").join("cool-perimeters.wasm")`.
- The function body contains a `// LEGACY_BROKEN_FORM:` comment block
  with at least one line citing
  `path.to_string_lossy().contains("target/slicer/...")` as the
  forbidden form.
- `cargo test -p slicer-cli cmd_run::tests::find_wasm_binary_correct_path_from_module_name`
  exits 0.
**Files**: `cli/slicer-cli/src/cmd_run.rs`.
**Verification**:
- `grep -nE 'LEGACY_BROKEN_FORM' cli/slicer-cli/src/cmd_run.rs`
  returns ≥1 match.
- `grep -nE 'to_string_lossy\(\)\.contains' cli/slicer-cli/src/cmd_run.rs`
  returns 0 matches outside the `LEGACY_BROKEN_FORM` comment block.
- `cargo test -p slicer-cli cmd_run::tests::find_wasm_binary_correct_path_from_module_name
  2>&1 | tail -5` shows `test result: ok. 1 passed`.
**Exit**: The path test passes on all three platforms.
**OrcaSlicer refs**: None.

## Step 4 — Add `check_host_binary_default_is_slicer_host` lock-down + audit

**Task IDs**: DEV-031
**Objective**: Add a focused negative test that nails the production
default. Confirm via grep that all `SLICER_HOST_BIN` consumers are
confined to `cli/slicer-cli/src/cmd_run.rs` (the helper, the
`TestEnvGuard` set/unset/Drop call sites, and the new lock-down test);
no other crate or binary in the workspace reads or writes the variable.
**Precondition**: Steps 1–3 complete.
**Postcondition**:
- `check_host_binary_default_is_slicer_host` exists in `mod tests {}`.
  It uses `TestEnvGuard::unset("SLICER_HOST_BIN")` and asserts
  `host_binary_name() == "slicer-host"`.
- A single workspace-level grep confirms no other code reads or writes
  `SLICER_HOST_BIN`:
  `grep -rnE 'SLICER_HOST_BIN' cli/ crates/ modules/` returns matches
  ONLY in `cli/slicer-cli/src/cmd_run.rs`.
**Files**: `cli/slicer-cli/src/cmd_run.rs`.
**Verification**:
- `cargo test -p slicer-cli cmd_run::tests::check_host_binary_default_is_slicer_host
  -- --test-threads=1 2>&1 | tail -5` shows `test result: ok. 1 passed`.
- `grep -rnE 'SLICER_HOST_BIN' cli/ crates/ modules/ 2>&1 | awk -F: '{print $1}' | sort -u`
  returns exactly the single line `cli/slicer-cli/src/cmd_run.rs`.
**Exit**: New negative test passes; no env-var leak elsewhere in the
workspace.
**OrcaSlicer refs**: None.

## Step 5 — Add DEV-031 row to `docs/DEVIATION_LOG.md`

**Task IDs**: DEV-031
**Objective**: Append a single DEV-031 row at the bottom of the active
deviations list, mirroring adjacent DEV-### entries' format. Cite both
test names and reference this packet by slug.
**Precondition**: Steps 1–4 complete.
**Postcondition**:
- `docs/DEVIATION_LOG.md` contains a row matching the regex
  `DEV-031.*(slicer-cli|cmd_run|29_slicer-cli-cmd-run-cross-platform)`.
- The row names BOTH affected tests:
  `find_wasm_binary_correct_path_from_module_name` and
  `execute_in_reaches_host_check`.
**Files**: `docs/DEVIATION_LOG.md`.
**Verification**:
- `grep -nE 'DEV-031' docs/DEVIATION_LOG.md` returns ≥1 match.
- `grep -nE 'find_wasm_binary_correct_path_from_module_name|execute_in_reaches_host_check' docs/DEVIATION_LOG.md`
  returns ≥2 matches (one per test name) under the DEV-031 row.
- `grep -nE '29_slicer-cli-cmd-run-cross-platform' docs/DEVIATION_LOG.md`
  returns ≥1 match.
**Exit**: DEV-031 row landed; deviation registry reflects the closure.
**OrcaSlicer refs**: None.

## Step 6 — Packet completion gate

**Task IDs**: DEV-031
**Objective**: Run the focused matrix and workspace-level checks for
packet 29.
**Precondition**: Steps 1–5 complete.
**Postcondition**: All commands exit 0.
**Files**: All changed files.
**Verification**:
```
cargo test -p slicer-cli cmd_run -- --test-threads=1 2>&1 | tail -5
cargo build --workspace 2>&1 | tail -3
cargo clippy --workspace -- -D warnings 2>&1 | tail -3
grep -nE 'DEV-031' docs/DEVIATION_LOG.md
grep -nE 'fn host_binary_name|TestEnvGuard|LEGACY_BROKEN_FORM' cli/slicer-cli/src/cmd_run.rs
```
**Exit**: All five commands exit 0; the `cargo test` line shows
`test result: ok. 29 passed; 0 failed` for `cmd_run`. Packet-close
review can proceed.
**OrcaSlicer refs**: None.

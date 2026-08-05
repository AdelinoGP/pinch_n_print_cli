---
status: implemented
packet: 29_slicer-cli-cmd-run-cross-platform
task_ids: []
---

# 29_slicer-cli-cmd-run-cross-platform

## Goal

Make the two pre-existing `slicer-cli::cmd_run` unit tests
(`find_wasm_binary_correct_path_from_module_name` and
`execute_in_reaches_host_check`) deterministic on Windows, Linux, and macOS
without changing the production behavior of `find_wasm_binary` or
`execute_in`. The first test currently fails because it uses a Unix-style
substring match against a `PathBuf` that uses backslash separators on
Windows. The second test currently fails on any developer machine where
`slicer-host` is on `PATH` (e.g. `cargo install`-ed or a workspace bin
directory in `PATH`), because the test asserts `MissingHostBinary` with
no way to force the host-discovery probe to fail.

## Problem Statement

Two `slicer-cli::cmd_run::tests::*` unit tests have been failing on `master`
on Windows development machines for an extended period:

1. **`find_wasm_binary_correct_path_from_module_name`** — asserts
   `path.to_string_lossy().contains("target/slicer/cool-perimeters.wasm")`.
   On Windows, `PathBuf::join` produces `target\slicer\cool-perimeters.wasm`
   (backslash separators), so the substring match against a forward-slash
   literal returns `false`. The assertion message even includes the
   double-backslash path, confirming the platform mismatch:

   ```
   path should use module name with hyphens preserved:
   "C:\\Users\\agpen\\AppData\\Local\\Temp\\.tmp4W22u6\\target\\slicer\\cool-perimeters.wasm"
   ```

2. **`execute_in_reaches_host_check`** — explicitly asserts the
   pipeline aborts with `Err(RunError::MissingHostBinary)`, with the
   inline comment `// slicer-host is not installed in test env, so this
   should fail at the host check`. That premise is wrong on every
   developer machine where `slicer-host` is on `PATH` (which is the
   common case — `cargo install --path crates/slicer-host` or any
   workspace `bin/` directory in `PATH` puts it there). On those
   machines the test instead reaches the actual host invocation and
   fails with `Err(HostExecutionFailed("exit code: 1"))` because the
   real host binary rejects the synthetic `fake-stl-data` model.

Both failures were verified to be present on a clean `master` baseline
during the packet-28 cleanup work (`git stash -u && cargo test -p
slicer-cli cmd_run` reproduced both failures with no in-flight changes).
This packet eliminates the platform and environment assumptions in both
tests so the suite runs cleanly on Windows, Linux, and macOS regardless
of the developer's local `PATH`.

## Architecture Constraints

- **Production behavior is locked to byte-equivalence when
  `SLICER_HOST_BIN` is unset.** Both `check_host_binary()` and the
  `Command::new(...)` call in `execute_in` MUST default to the literal
  string `"slicer-host"`. The new helper exists only to give tests a
  hook; production callers must observe zero behavioral diff. Verified
  by the new `check_host_binary_default_is_slicer_host` test that asserts
  the helper returns `"slicer-host"` when the env var is unset.
- **Tests must not leak environment changes to other tests.** The
  `TestEnvGuard` RAII helper saves the prior `SLICER_HOST_BIN` value on
  construction and restores it on `Drop` (including on panic via the
  drop-during-unwind guarantee). The packet's verification command
  uses `--test-threads=1` so the env-var manipulation cannot race with
  parallel tests inside the same process — `std::env::set_var` is
  process-wide, and `tempfile`-using tests in the same suite are
  numerous.
- **Path equality must use `PathBuf`, not strings.** The fixed
  `find_wasm_binary_correct_path_from_module_name` test compares
  `path == dir.path().join("target").join("slicer").join("cool-perimeters.wasm")`.
  Both sides are `PathBuf`; their `PartialEq` impl compares components,
  not raw bytes, and is platform-correct on Windows.
- **No new dependencies.** Specifically: do NOT add the `temp-env`,
  `serial_test`, or `which` crates. The RAII guard and env-var read are
  trivial and stay inline in the test module.

## Data and Contract Notes

- `SLICER_HOST_BIN` is an internal-only env var. It is NOT documented
  as user-facing in `docs/05_module_sdk.md` because the only consumer
  is the test suite. If real users ever need to point `slicer run` at a
  non-default host binary, that becomes a separate packet that adds it
  to the documented CLI surface.
- The `LEGACY_BROKEN_FORM:` comment block in
  `find_wasm_binary_correct_path_from_module_name` carries the
  documented anti-pattern so a future maintainer who sees a similar
  substring check elsewhere has the receipt for why it's wrong.

## Risks and Tradeoffs

- **Risk:** `std::env::set_var` is process-wide and racy with other
  tests that read `SLICER_HOST_BIN`. *Mitigation:* the verification
  command pins `--test-threads=1`, and the packet's two
  env-var-manipulating tests are the only consumers of the variable in
  the workspace. `grep -rE 'SLICER_HOST_BIN' .` will be run during
  Step 5 to confirm no other consumers exist.
- **Risk:** Future Rust versions may deprecate `std::env::set_var` in
  test contexts (the Rust team has discussed this). *Mitigation:* the
  `TestEnvGuard` RAII shape makes the migration to a deprecation-safe
  alternative (e.g. `temp-env`) a one-place edit. Not done now to
  avoid the dependency.
- **Tradeoff:** Adding an env-var override slightly grows the cmd_run
  contract surface. Acceptable because the production default path is
  byte-identical, and the cost (one helper + ~3 lines per call site)
  is small compared to the cross-platform CI value.

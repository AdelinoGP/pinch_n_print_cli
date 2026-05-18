# Requirements: 29_slicer-cli-cmd-run-cross-platform

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

## Grouped Task IDs

This packet does NOT add a `TASK-###` row to
`docs/07_implementation_status.md`. Per packet-authoring decision, the
two failures are recorded as a single deviation:

- **DEV-031** — `slicer-cli::cmd_run` unit tests assume Unix path
  separators and a `slicer-host`-absent test environment; both fail on
  Windows or on any developer machine where `slicer-host` is on `PATH`.
  Affects: `find_wasm_binary_correct_path_from_module_name`,
  `execute_in_reaches_host_check`. Closed by packet
  `29_slicer-cli-cmd-run-cross-platform`.

The DEV-031 row is added to `docs/DEVIATION_LOG.md` by Step 4 of the
implementation plan.

## In-Scope

- `cli/slicer-cli/src/cmd_run.rs`:
  - **Production change A (additive, backwards-compatible):** introduce
    a single internal helper that resolves the host binary name from
    `std::env::var("SLICER_HOST_BIN")` with the literal default
    `"slicer-host"`. Both `check_host_binary()` and the
    `Command::new(...)` invocation in `execute_in` MUST consume that
    helper. When the env var is unset, behavior is byte-identical to
    today (still `Command::new("slicer-host")`).
  - **Test change A (path assertion fix):** rewrite
    `find_wasm_binary_correct_path_from_module_name` to assert
    `path == dir.path().join("target").join("slicer").join("cool-perimeters.wasm")`
    using `PathBuf` equality. Drop the `to_string_lossy().contains(...)`
    form. Add a `// LEGACY_BROKEN_FORM:` comment block recording the
    prior assertion so future maintainers do not reintroduce it.
  - **Test change B (env override + RAII guard):** rewrite
    `execute_in_reaches_host_check` to set
    `SLICER_HOST_BIN=__slicer_host_test_marker_does_not_exist__` for
    the duration of the test using a small `TestEnvGuard` RAII helper
    (defined in the same `mod tests {}` block) that saves the prior
    value on `new` and restores it on `Drop`. Keep the existing
    `MissingHostBinary` assertion.
  - **New negative test (default behavior lock-down):** add
    `check_host_binary_default_is_slicer_host`. The test uses the
    `TestEnvGuard` to ensure `SLICER_HOST_BIN` is unset, then asserts
    that the helper returns the literal `"slicer-host"` string (the
    helper's return type is `String`, not a `bool`). This nails the
    production default and prevents accidental environment leakage
    across tests.
- `docs/DEVIATION_LOG.md`: add DEV-031 row.

## Out-of-Scope

- Any change to `cmd_build.rs`, `cmd_test.rs`, `cmd_validate.rs`,
  `cmd_new.rs`, or other `slicer-cli` subcommands.
- Replacing `Command::new(...)` PATH-lookup discovery with a more
  sophisticated mechanism (e.g. `which`-based resolution, vendored
  binary registry).
- Auditing the rest of the workspace for similar Unix-path assumptions.
  A separate sweep packet can address that if a CI matrix or CI alert
  surfaces additional offenders.
- Adding a new `TASK-###` row to `docs/07`. Per packet decision, this
  cleanup is captured as a deviation only.

## Authoritative Docs

- `docs/05_module_sdk.md` — `slicer-cli` developer experience; `slicer
  run` flow.
- `docs/DEVIATION_LOG.md` — deviation registry where DEV-031 is added.

## OrcaSlicer Reference Obligations

- None.

## Acceptance Summary

After this packet lands:

1. `cargo test -p slicer-cli cmd_run` exits 0 on Windows, Linux, and
   macOS, with all 29 `cmd_run::*` tests passing (the existing 27 plus
   the two formerly-failing tests plus the one new negative
   `check_host_binary_default_is_slicer_host` lock-down).
2. The production `slicer run` command path resolution is byte-identical
   to today when `SLICER_HOST_BIN` is unset; setting the env var to a
   non-existent name cleanly produces `Err(RunError::MissingHostBinary)`.
3. The `find_wasm_binary` test asserts a `PathBuf`-aware comparison and
   no longer depends on the platform's path separator.
4. `docs/DEVIATION_LOG.md` records DEV-031 with the affected test names
   and a pointer to this packet.

## Cross-Packet Dependencies and Unblockers

- **Depends on:** none.
- **Does not supersede:** any prior packet.
- **Unblocks:** future Windows / cross-platform CI matrices.

## Verification

```
cargo test -p slicer-cli cmd_run -- --test-threads=1
cargo build --workspace
cargo clippy --workspace -- -D warnings
grep -nE 'DEV-031.*slicer-cli|DEV-031.*cmd_run|29_slicer-cli-cmd-run-cross-platform' docs/DEVIATION_LOG.md
```

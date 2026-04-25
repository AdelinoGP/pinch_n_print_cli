---
status: draft
packet: 29_slicer-cli-cmd-run-cross-platform
task_ids: []
deviation_ids:
  - DEV-031
backlog_source: docs/07_implementation_status.md
---

# Packet Contract: 29_slicer-cli-cmd-run-cross-platform

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

## Scope Boundaries

- **In scope:**
  - `cli/slicer-cli/src/cmd_run.rs`:
    - Replace the substring assertion in
      `find_wasm_binary_correct_path_from_module_name` with a
      component-aware comparison
      (`assert_eq!(path, dir.path().join("target").join("slicer").join("cool-perimeters.wasm"))`)
      so backslash-separator paths on Windows match.
    - Add an env-var override read by both `check_host_binary()` and the
      `Command::new(...)` call in `execute_in`. The variable name is
      `SLICER_HOST_BIN`; when unset, behavior is unchanged
      (`Command::new("slicer-host")`). When set, both call sites use the
      override value verbatim.
    - Update `execute_in_reaches_host_check` to set
      `SLICER_HOST_BIN=__slicer_host_test_marker_does_not_exist__` for
      the duration of the test using a `TestEnvGuard` RAII helper that
      saves and restores the prior value, so the assertion
      `Err(RunError::MissingHostBinary)` holds regardless of whether the
      dev machine has a real `slicer-host` on `PATH`.
  - `docs/DEVIATION_LOG.md`: add a row for DEV-031 with a 1-line
    description, the affected test names, and pointer to this packet.
- **Out of scope:**
  - The other 27 passing `cmd_run` tests (no behavior change required).
  - Any change to `cmd_build`, `cmd_test`, `cmd_validate`, or other
    `slicer-cli` subcommands.
  - Replacing the host-binary discovery mechanism (PATH lookup via
    `Command::new`) with anything more sophisticated; the env-var
    override is the smallest viable surface.
  - Adding a new `TASK-###` row to `docs/07_implementation_status.md`
    (per packet decision: DEV-031 entry only).
  - Hardening any other test in the workspace that may have similar
    Unix-path or PATH-lookup assumptions.

## Prerequisites and Blockers

- **Depends on:** none. This is independent of any active or in-flight
  packet. The two target tests have been broken on `master` since at
  least packet 26's CLI work; multiple sessions have confirmed they fail
  on a clean baseline by stashing all in-flight changes and re-running
  the test.
- **Unblocks:** restoring a green `cargo test --workspace` on Windows
  development machines; future cross-platform CI matrices.
- **Activation blockers:** confirm no other packet is `status: active`
  before flipping this packet's status. Packet 28 was `status: active`
  during this packet's authoring; ensure that has been moved to
  `implemented` (or back to `draft`) before activating packet 29.

## Acceptance Criteria

- **Given** the updated `find_wasm_binary_correct_path_from_module_name`
  test, **when** `cargo test -p slicer-cli cmd_run::tests::find_wasm_binary_correct_path_from_module_name`
  is run on Windows, Linux, or macOS, **then** the test passes and
  asserts `path == dir.path().join("target").join("slicer").join("cool-perimeters.wasm")`
  using `PathBuf` equality (not `to_string_lossy().contains(...)`). |
  `cargo test -p slicer-cli cmd_run::tests::find_wasm_binary_correct_path_from_module_name 2>&1 | tail -5`
- **Given** `cli/slicer-cli/src/cmd_run.rs` after the env-var override
  lands, **when** read, **then** both `check_host_binary()` and the
  `Command::new(...)` invocation inside `execute_in` resolve the binary
  name from `std::env::var("SLICER_HOST_BIN").unwrap_or_else(|_|
  String::from("slicer-host"))` (or an internal helper that returns
  exactly that string). |
  `grep -nE 'SLICER_HOST_BIN|fn check_host_binary|Command::new' cli/slicer-cli/src/cmd_run.rs | head -10`
- **Given** the updated `execute_in_reaches_host_check` test, **when**
  `cargo test -p slicer-cli cmd_run::tests::execute_in_reaches_host_check`
  is run on a machine where `slicer-host` IS on `PATH`, **then** the
  test still passes by overriding `SLICER_HOST_BIN` to a guaranteed-
  missing binary name and asserting `Err(RunError::MissingHostBinary)`. |
  `cargo test -p slicer-cli cmd_run::tests::execute_in_reaches_host_check 2>&1 | tail -5`
- **Given** the full `slicer-cli` test suite, **when**
  `cargo test -p slicer-cli` is run on the local host, **then** all
  `cmd_run::*` tests pass (the count must be 29 — the existing 27 plus
  the two formerly-failing tests). |
  `cargo test -p slicer-cli cmd_run 2>&1 | grep -E '^test result:' | tail -1`
- **Given** `docs/DEVIATION_LOG.md`, **when** read, **then** it contains
  a `DEV-031` row naming both failing tests and pointing at this packet
  by slug. |
  `grep -nE 'DEV-031.*slicer-cli|DEV-031.*cmd_run|29_slicer-cli-cmd-run-cross-platform' docs/DEVIATION_LOG.md`

## Negative Test Cases

- **Given** the production behavior of `execute_in` (no env var set),
  **when** `SLICER_HOST_BIN` is unset and `slicer-host` is not on
  `PATH`, **then** `execute_in` still returns
  `Err(RunError::MissingHostBinary)` exactly as before, proving the
  env-var override defaults to the prior production string. The test
  asserts this by saving `SLICER_HOST_BIN`, removing it from the env,
  and exercising `check_host_binary()` against a `PATH` that has been
  scrubbed of `slicer-host`. (If the dev machine has a system-wide
  `slicer-host`, this test must instead temporarily set
  `SLICER_HOST_BIN` to a known-missing name to keep the assertion
  meaningful — same RAII guard as the positive test.) |
  `cargo test -p slicer-cli cmd_run::tests::check_host_binary_default_is_slicer_host -- --nocapture 2>&1 | tail -10`
- **Given** the test `find_wasm_binary_correct_path_from_module_name`
  before the fix, **when** the substring assertion
  `path.to_string_lossy().contains("target/slicer/cool-perimeters.wasm")`
  is restored, **then** running the test on Windows fails with the
  documented error
  `path should use module name with hyphens preserved: ".../target\\slicer\\cool-perimeters.wasm"`.
  This case is intentionally non-runnable — the failing assertion form
  is recorded inline in `cmd_run.rs` as a `// LEGACY_BROKEN_FORM:`
  comment so future maintainers don't reintroduce it. |
  `grep -nE 'LEGACY_BROKEN_FORM' cli/slicer-cli/src/cmd_run.rs`

## Verification

- `cargo test -p slicer-cli cmd_run -- --test-threads=1`
  (`--test-threads=1` ensures the `SLICER_HOST_BIN` env-var
  manipulation in `execute_in_reaches_host_check` cannot race with
  parallel tests in the same process.)
- `cargo build --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/05_module_sdk.md` — `slicer-cli` developer-experience surface;
  `slicer run` flow.
- `docs/07_implementation_status.md` — Workstream 5 (Governance and
  closure drift); deviation map.
- `docs/DEVIATION_LOG.md` — canonical deviation registry; DEV-031 row
  added by this packet.

## OrcaSlicer Reference Obligations

- None. The two failing tests exercise pure CLI plumbing
  (`tempfile`-based `find_wasm_binary` discovery and `Command::new`
  PATH lookup) with no algorithmic equivalent in OrcaSlicer.

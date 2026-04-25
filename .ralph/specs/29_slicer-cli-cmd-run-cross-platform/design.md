# Design: 29_slicer-cli-cmd-run-cross-platform

## Controlling Code Paths

1. **`cli/slicer-cli/src/cmd_run.rs`** — both production and test code
   live here. The two failing tests (`find_wasm_binary_correct_path_from_module_name`,
   `execute_in_reaches_host_check`) and the helpers they reference
   (`write_cargo_toml`, `write_wasm_binary`, `write_model_file`,
   `write_config_file`, `write_valid_manifest`) are all in the same
   `mod tests {}` block at the bottom of the file.
2. **`docs/DEVIATION_LOG.md`** — append-only registry. Add a single
   DEV-031 row at the bottom of the active list, mirroring the row
   format used by adjacent DEV-### entries.

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

## Implementation Approach (selected)

A two-call-site env-var override with a small inline helper, plus an
RAII test guard. Concretely:

```rust
// New internal helper near `check_host_binary` in cmd_run.rs.
//
// Reads SLICER_HOST_BIN; defaults to "slicer-host" when unset or when
// the value contains a NUL byte (a malformed env var should never
// silently override; treat as unset).
fn host_binary_name() -> String {
    match std::env::var("SLICER_HOST_BIN") {
        Ok(v) if !v.is_empty() && !v.contains('\0') => v,
        _ => String::from("slicer-host"),
    }
}

// Updated check_host_binary:
pub fn check_host_binary() -> bool {
    Command::new(host_binary_name())
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// Updated invocation in execute_in (was: Command::new("slicer-host")):
let host_output = Command::new(host_binary_name())
    .args(&args)
    .status()
    .map_err(|e| RunError::HostExecutionFailed(e.to_string()))?;
```

```rust
// New TestEnvGuard inside `mod tests {}`:
struct TestEnvGuard {
    key: &'static str,
    prior: Option<String>,
}

impl TestEnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prior = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, prior }
    }

    fn unset(key: &'static str) -> Self {
        let prior = std::env::var(key).ok();
        std::env::remove_var(key);
        Self { key, prior }
    }
}

impl Drop for TestEnvGuard {
    fn drop(&mut self) {
        match &self.prior {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}
```

```rust
// Fixed find_wasm_binary_correct_path_from_module_name:
#[test]
fn find_wasm_binary_correct_path_from_module_name() {
    let dir = tempfile::tempdir().unwrap();
    write_cargo_toml(dir.path(), "cool-perimeters");
    write_wasm_binary(dir.path(), "cool-perimeters");

    let path = find_wasm_binary(dir.path()).unwrap();
    let expected = dir
        .path()
        .join("target")
        .join("slicer")
        .join("cool-perimeters.wasm");
    assert_eq!(
        path, expected,
        "find_wasm_binary must return target/slicer/<module-name>.wasm \
         joined under the cargo project root, with the module name \
         (hyphens preserved); got {path:?}"
    );
    // LEGACY_BROKEN_FORM:
    //   `path.to_string_lossy().contains("target/slicer/cool-perimeters.wasm")`
    // failed on Windows because PathBuf::join uses '\\' separators.
    // Do not reintroduce a substring match against a forward-slash literal.
}
```

```rust
// Fixed execute_in_reaches_host_check:
#[test]
fn execute_in_reaches_host_check() {
    let _guard = TestEnvGuard::set(
        "SLICER_HOST_BIN",
        "__slicer_host_test_marker_does_not_exist__",
    );

    let dir = tempfile::tempdir().unwrap();
    write_cargo_toml(dir.path(), "my-module");
    write_valid_manifest(dir.path());
    write_wasm_binary(dir.path(), "my-module");
    write_model_file(dir.path(), "cube.stl");
    write_config_file(dir.path(), "config.json", r#"{"density": 0.15}"#);

    let model_path = dir.path().join("cube.stl");
    let config_path = dir.path().join("config.json");
    let result = execute_in(
        dir.path(),
        &model_path.to_string_lossy(),
        Some(&config_path.to_string_lossy()),
        Some("output.gcode"),
    );
    assert!(
        matches!(result, Err(RunError::MissingHostBinary)),
        "expected MissingHostBinary when SLICER_HOST_BIN points at a \
         non-existent binary; got: {result:?}"
    );
}
```

```rust
// New negative lock-down:
#[test]
fn check_host_binary_default_is_slicer_host() {
    let _guard = TestEnvGuard::unset("SLICER_HOST_BIN");
    assert_eq!(
        host_binary_name(),
        "slicer-host",
        "SLICER_HOST_BIN unset must default to literal \"slicer-host\""
    );
}
```

### Rejected alternatives

- **Use the `temp-env` crate.** Adds a dependency for a 30-line RAII
  helper. Rejected.
- **Use `serial_test::serial` to gate the env-var-touching tests.**
  Adds a dependency and a procmacro. The `--test-threads=1` flag in
  the verification command is sufficient and keeps the test surface
  dependency-free.
- **Replace `Command::new("slicer-host")` with `which::which`-based
  discovery and pass the resolved `PathBuf` everywhere.** Bigger
  surface than the bug warrants; punt to a future packet if real PATH
  discovery (e.g. workspace-relative resolution) is ever needed.
- **Refactor `execute_in` to take a `host_binary: &str` parameter.**
  Forces callers to thread the name through main.rs and any future
  callers. The env-var indirection is a smaller blast radius and keeps
  the public API stable.

## Explicit Code Change Surface

### Files modified
- `cli/slicer-cli/src/cmd_run.rs` — add `host_binary_name()` helper; update
  `check_host_binary()` and `execute_in`'s `Command::new(...)` call;
  rewrite `find_wasm_binary_correct_path_from_module_name`; rewrite
  `execute_in_reaches_host_check`; add `TestEnvGuard` and
  `check_host_binary_default_is_slicer_host` to the test module.
- `docs/DEVIATION_LOG.md` — append the DEV-031 row (single line).

### Files created
- None.

### Artifacts rebuilt
- None (Rust-only change; no `.wasm` rebuild required).

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

## Open Questions

All packet-scope open questions are resolved:

- **Q1: env var name.** *Resolved —* `SLICER_HOST_BIN` (matches the
  ad-hoc convention from `MAX_SAMPLES_PER_EXPOLY` etc.; project-wide
  env-var convention is `SLICER_*` per CLAUDE.md naming patterns).
- **Q2: scope of the env override.** *Resolved —* both call sites,
  internal-only, default unchanged.
- **Q3: deviation vs task row in docs/07.** *Resolved —* DEV-031
  deviation only, per user choice during packet authoring.
- **Q4: dependency on `serial_test` or `temp-env`.** *Resolved —* no
  new dependencies; inline `TestEnvGuard` + `--test-threads=1`.
- **Q5: should we also fix the `find_wasm_binary_found` test
  (line 298) which uses `path.ends_with("my-infill.wasm")`?**
  *Resolved —* no. `Path::ends_with` is already
  component-aware and works correctly on Windows. The bug is specific
  to the substring `to_string_lossy().contains(...)` form.

## Locked Assumptions

1. Production callers of `slicer run` do not set `SLICER_HOST_BIN`.
   The env var is documented as an internal test hook only.
2. `Command::new(name)` with `name: String` resolves the binary via
   `PATH` on all three target platforms; this is the same behavior as
   `Command::new("slicer-host")` today.
3. `tempfile::tempdir()` returns a `PathBuf` whose `join(...)` produces
   the platform-native separator. This is true across Windows, Linux,
   macOS by `std::path` design.
4. The `TestEnvGuard::Drop` impl runs even on panic. The Rust panic
   runtime guarantees `Drop` for in-scope locals during stack
   unwinding, so the env-var restore is panic-safe.

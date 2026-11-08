use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::build_guests;
use crate::check_literals;

// Touch ClosureCache::len for clippy -D dead_code (build_guests::ClosureCache::len is otherwise
// only referenced from #[cfg(test)] code, which clippy without --tests considers unused).
#[allow(dead_code)]
fn _touch_closure_cache_len() {
    let _ = crate::build_guests::ClosureCache::new().len();
}

/// `cargo xtask test [--summary] [--summary-from <FILE>] [ARGS...]`
///
/// Modes:
///   `--summary`             Run tests, then print a compact LLM-friendly digest
///                           (summary lines + failure detail + verdict) instead
///                           of streaming every per-test `ok` line.
///   `--summary-from <FILE>` Skip the test run entirely; just parse `<FILE>`
///                           (or `target/test-output.log` if `<FILE>` is `-`)
///                           and print the digest. Useful for re-summarizing an
///                           existing log without re-running tests.
///   (neither flag)          Live-stream `cargo test` output to the terminal,
///                           tee'd to the log file (original behaviour).
///
/// Non-`--summary-from` modes run the check-literals preflight first, then the
/// guest-WASM freshness check (`build-guests --check`), rebuilding if stale.
/// `--summary-from` is gate-free (no test run = no gate).
///
/// This is the gated entry point for "whole suite" / regression-diagnosis runs.
/// Narrow single-test invocations should still use plain `cargo test` directly.
pub fn test_command(ws_root: &Path, passthrough: &[String]) -> i32 {
    // Step 0: parse our flags; pass the rest to `cargo test`.
    let options = match parse_test_options(passthrough) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("xtask test: {error}");
            return 2;
        }
    };
    let TestOptions {
        accelerated,
        summary,
        summary_from,
        test_args,
    } = options;

    // --- --summary-from: parse-only shortcut (no test run, no freshness gate) ---
    if let Some(from) = summary_from {
        let log_path = if from == "-" {
            ws_root.join("target").join("test-output.log")
        } else {
            Path::new(&from).to_path_buf()
        };
        if !log_path.exists() {
            eprintln!(
                "xtask test: --summary-from file not found: {}",
                log_path.display()
            );
            return 2;
        }
        print_summary(&log_path, false);
        let log_display = log_path
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/");
        println!();
        println!("Source log: {log_display}");
        return 0;
    }

    let accelerated_wrapper = if accelerated {
        if let Err(error) = crate::rustc_driver::clear_latch(ws_root) {
            eprintln!("xtask test --accelerated: failed to clear driver latch: {error}");
            return 1;
        }
        match crate::rustc_driver::create_shim_wrapper(ws_root) {
            Ok(path) => Some(path),
            Err(error) => {
                eprintln!("xtask test --accelerated: failed to create shim wrapper: {error}");
                return 1;
            }
        }
    } else {
        None
    };

    // Step 0b: enforce the Arachne parity gate + quarantine roster.
    //
    // The arachne parity suite (~34 test files in slicer-core) is gated behind
    // `#![cfg(feature = "host-algos")]`. `host-algos` is NOT a default feature, so
    // a narrow `cargo test -p slicer-core` run alone gets `default = []` and those
    // files silently compile to empty no-ops — exactly how packet 155's regressions
    // escaped (its `-p slicer-core` verification never saw them). We do NOT flip
    // slicer-core's Cargo default (that would pull rayon/boostvoronoi into the five
    // module crates' wasm32 guest builds, which don't compile). Instead we enforce
    // the feature at the `cargo test` invocation here.
    //
    // The feature is injected only when the selection includes slicer-core
    // itself: a qualified `slicer-core/host-algos` name is invalid on any other
    // `-p <package>` (t108 finding A1). See `selection_includes_slicer_core`.

    // Split caller args at the first `--` into cargo-level and libtest-level args so
    // `--features` lands on the cargo side and `--skip` on the libtest side.
    let (cargo_args, libtest_args): (Vec<String>, Vec<String>) =
        match test_args.iter().position(|a| a == "--") {
            Some(i) => (test_args[..i].to_vec(), test_args[i + 1..].to_vec()),
            None => (test_args.clone(), Vec::new()),
        };

    let test_args = build_test_arguments(ws_root, cargo_args, libtest_args);

    // Step 1: three independent read-only preflights — check-literals, the
    // test-quality gate (report mode until the remediation program's final wave
    // flips enforce — ADR-0065; `TEST_QUALITY_ENFORCED` is the single promotion
    // point), and the guest-freshness check — run concurrently. They share no
    // state, so the gate's fixed cost is the slowest of the three rather than
    // their sum. Failure reporting keeps the original order: the source scans
    // are reported before the freshness outcome is consumed, so a literals or
    // test-quality failure still aborts without a guest rebuild.
    //
    // ONE invocation context for both halves of the gate: `xtask test` probes
    // the toolchain and parses the canonical WIT set once, not once per phase.
    let inv = build_guests::Invocation::new(ws_root);
    let (literals_code, test_quality_code, freshness) = run_preflights_with(
        || check_literals_preflight(ws_root),
        || {
            crate::check_test_quality::run(
                ws_root,
                !crate::check_test_quality::TEST_QUALITY_ENFORCED,
                &[],
            )
        },
        || {
            if accelerated {
                build_guests::CheckOutcome {
                    stale: Vec::new(),
                    code: build_guests::EXIT_FRESH,
                }
            } else {
                build_guests::check_command_in(ws_root, &inv)
            }
        },
    );

    if literals_code != 0 {
        eprintln!(
            "xtask test: check-literals preflight failed; fix violations or add reasoned waivers (docs/21_data_defaults_and_fixtures.md), then re-run."
        );
        return 1;
    }
    if test_quality_code != 0 {
        eprintln!(
            "xtask test: check-test-quality preflight failed; fix findings or add reasoned waivers (docs/22_test_quality.md), then re-run."
        );
        return 1;
    }

    if accelerated {
        let code = build_guests::accelerated_build_command(ws_root, false);
        if code != 0 {
            eprintln!("xtask test: accelerated guest build failed; aborting test run.");
            return code;
        }
    } else if let Some(code) = handle_guest_freshness_with(
        ws_root,
        || freshness,
        |stale| build_guests::build_stale_command(ws_root, stale, &inv),
    ) {
        return code;
    }

    let freshness = match accelerated_wrapper.as_deref() {
        Some(wrapper) => ensure_pnp_cli_fresh_accelerated(ws_root, wrapper),
        None => ensure_pnp_cli_fresh(ws_root),
    };
    if freshness.code != 0 {
        if let Some(detail) = freshness.failure_detail {
            eprintln!("{detail}");
        }
        return freshness.code;
    }

    // Step 2: ensure target/ exists; choose output strategy.
    fs::create_dir_all(ws_root.join("target")).ok();
    let log_path = ws_root.join("target").join("test-output.log");
    // Render the path for display without Windows' `\\?\` verbatim prefix
    // (Path::display() emits it after canonicalization on Windows).
    let log_display = log_path
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/");

    let (exit_code, ran) = if summary {
        // --summary: run cargo test with piped output; write to log; do NOT
        // stream anything to the terminal. Then parse & print the digest.
        let mut cmd = Command::new("cargo");
        cmd.arg("test");
        if let Some(wrapper) = accelerated_wrapper.as_deref() {
            cmd.current_dir(ws_root);
            configure_accelerated_host_command(&mut cmd, wrapper, ws_root, true);
        }
        cmd.args(&test_args);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        eprintln!(
            "xtask test: running `cargo test {}` (summary mode)",
            test_args.join(" ")
        );

        let out = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("xtask test: failed to spawn cargo test: {e}");
                return 1;
            }
        };

        // Write combined stdout+stderr to the log.
        let mut combined: Vec<u8> = Vec::with_capacity(out.stdout.len() + out.stderr.len());
        combined.extend_from_slice(&out.stdout);
        combined.extend_from_slice(&out.stderr);
        if let Err(e) = fs::write(&log_path, &combined) {
            eprintln!("xtask test: failed to write {log_display}: {e}");
        }

        let code = out.status.code().unwrap_or(1);
        let succeeded = out.status.success();
        (code, succeeded)
    } else {
        // Stream both child pipes directly so a shell pipeline cannot replace
        // cargo's failure status with `tee`'s successful exit status.
        eprintln!("xtask test: running `cargo test {}`", test_args.join(" "));
        match run_streaming_test(
            ws_root,
            &test_args,
            &log_path,
            accelerated_wrapper.as_deref(),
        ) {
            Ok(code) => (code, code == 0),
            Err(error) => {
                eprintln!("xtask test: {error}");
                return 1;
            }
        }
    };

    if summary {
        print_summary(&log_path, ran);
        println!();
        println!("Full output written to: {log_display}");
        println!("Inspect with: grep \"^test result:\" {log_display}   (summaries)");
        println!("               grep -n \"FAILED|panicked at\" {log_display} (failures)");
    }

    exit_code
}

/// Deliberate RED parity anchors / out-of-scope tests quarantined from the gate.
///
/// Skipped via the libtest `--skip` filter so a green gate stays meaningful
/// (only NEW breakage fails). These are tracked in
/// docs/specs/arachne-parity-recovery.md. We skip them at the runner rather
/// than `#[ignore]`-ing them: sibling RED-anchor files (arachne_parity_gaps.rs,
/// arachne_parity_round2.rs) carry a checked-in policy forbidding `#[ignore]`
/// on this test family, and the roster stays diffable here.
const QUARANTINED_TESTS: &[&str] = &[
    // Concentric-infill-through-Arachne — out of scope (D-104f; user decision
    // 2026-07-15: not on the roadmap, may never be Arachne).
    "arachne_parity_pipeline_concentric_infill_uses_arachne",
];

/// Build the final `cargo test` argument vector.
///
/// Two policies live here, both testable without spawning cargo:
/// - `slicer-core/host-algos` is injected only when the caller set no features
///   and the selection includes `slicer-core` itself (see
///   `selection_includes_slicer_core`);
/// - the quarantined parity tests are skipped on the libtest side.
fn build_test_arguments(
    ws_root: &Path,
    mut cargo_args: Vec<String>,
    mut libtest_args: Vec<String>,
) -> Vec<String> {
    let caller_set_features = cargo_args.iter().any(|arg| {
        arg == "--features" || arg.starts_with("--features=") || arg == "--all-features"
    });
    if !caller_set_features && selection_includes_slicer_core(ws_root, &cargo_args) {
        cargo_args.push("--features".to_string());
        cargo_args.push("slicer-core/host-algos".to_string());
    }

    for name in QUARANTINED_TESTS {
        libtest_args.push("--skip".to_string());
        libtest_args.push((*name).to_string());
    }

    let mut test_args = cargo_args;
    if !libtest_args.is_empty() {
        test_args.push("--".to_string());
        test_args.extend(libtest_args);
    }
    test_args
}

/// True when a `-p` / `--exclude` spec names `slicer-core`, including cargo's
/// `name@version` pkgid form.
fn names_slicer_core(package: &str) -> bool {
    package == "slicer-core" || package.starts_with("slicer-core@")
}

/// True when cargo's package selection includes `slicer-core` itself.
///
/// The Arachne parity suite is gated by `#![cfg(feature = "host-algos")]`, and
/// `host-algos` is not a default feature, so the gated entry point forces it
/// on. A qualified `package/feature` name is valid only when that package is
/// part of the selection: `cargo test -p xtask --features
/// slicer-core/host-algos` fails with "the package 'xtask' does not contain
/// this feature" (t108 finding A1). Every in-repo package that depends on
/// `slicer-core` and needs the feature enables it in its own manifest, so the
/// injection only has to cover `slicer-core` itself.
fn selection_includes_slicer_core(ws_root: &Path, cargo_args: &[String]) -> bool {
    let mut packages: Vec<&str> = Vec::new();
    let mut excludes: Vec<&str> = Vec::new();
    let mut workspace = false;
    let mut manifest_path: Option<&str> = None;

    let mut index = 0;
    while index < cargo_args.len() {
        let arg = cargo_args[index].as_str();
        if arg == "--" {
            break;
        }
        match arg {
            "-p" | "--package" => {
                if let Some(value) = cargo_args.get(index + 1) {
                    packages.push(value.as_str());
                    index += 1;
                }
            }
            "--exclude" => {
                if let Some(value) = cargo_args.get(index + 1) {
                    excludes.push(value.as_str());
                    index += 1;
                }
            }
            "--manifest-path" => {
                if let Some(value) = cargo_args.get(index + 1) {
                    manifest_path = Some(value.as_str());
                    index += 1;
                }
            }
            "--workspace" | "--all" => workspace = true,
            _ => {
                if let Some(value) = arg.strip_prefix("--package=") {
                    packages.push(value);
                } else if let Some(value) = arg.strip_prefix("--exclude=") {
                    excludes.push(value);
                } else if let Some(value) = arg.strip_prefix("--manifest-path=") {
                    manifest_path = Some(value);
                } else if let Some(value) = arg.strip_prefix("-p") {
                    // cargo accepts the attached short form: `-pfoo`.
                    if !value.is_empty() {
                        packages.push(value);
                    }
                }
            }
        }
        index += 1;
    }

    if !packages.is_empty() {
        return packages.iter().copied().any(names_slicer_core);
    }
    if workspace {
        return !excludes.iter().copied().any(names_slicer_core);
    }
    if let Some(path) = manifest_path {
        return manifest_declares_package(ws_root, path, "slicer-core");
    }
    // No selector at a virtual workspace root selects every member, slicer-core
    // included.
    true
}

/// Read a Cargo manifest and report whether its `[package]` name matches.
///
/// A missing or unreadable manifest returns `false`: cargo reports the real
/// error, and skipping the injection avoids turning an unrelated failure into
/// a feature-resolution failure.
fn manifest_declares_package(ws_root: &Path, manifest: &str, expected: &str) -> bool {
    let candidate = Path::new(manifest);
    let path = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        ws_root.join(candidate)
    };
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(parsed) = text.parse::<toml::Value>() else {
        return false;
    };
    parsed
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        == Some(expected)
}

#[derive(Debug, PartialEq, Eq)]
struct TestOptions {
    accelerated: bool,
    summary: bool,
    summary_from: Option<String>,
    test_args: Vec<String>,
}

fn parse_test_options(passthrough: &[String]) -> Result<TestOptions, String> {
    let mut accelerated = false;
    let mut summary = false;
    let mut summary_from: Option<String> = None;
    let mut test_args: Vec<String> = Vec::with_capacity(passthrough.len());

    let mut iter = passthrough.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--accelerated" if !accelerated => accelerated = true,
            "--accelerated" => return Err("--accelerated may only be specified once".to_string()),
            "--summary" => summary = true,
            "--summary-from" => {
                summary_from = iter.next().cloned();
                if summary_from.is_none() {
                    return Err("--summary-from requires a file path argument".to_string());
                }
            }
            other => test_args.push(other.to_string()),
        }
    }

    if accelerated && test_args.iter().any(|arg| is_doctest_argument(arg)) {
        return Err(
            "--accelerated does not support doctests; remove --doc before retrying".to_string(),
        );
    }

    Ok(TestOptions {
        accelerated,
        summary,
        summary_from,
        test_args,
    })
}

fn is_doctest_argument(arg: &str) -> bool {
    arg == "--doc" || arg.starts_with("--doc=")
}

const ACCELERATED_TEST_CONFIG: &[&str] = &[
    "--config",
    "profile.dev.package.slicer-core.opt-level=3",
    "--config",
    "profile.dev.package.slicer-core.debug=2",
    "--config",
    "profile.dev.package.slicer-core.debug-assertions=true",
];

fn configure_accelerated_host_command(
    cmd: &mut Command,
    wrapper: &Path,
    ws_root: &Path,
    test_profile: bool,
) {
    cmd.env("RUSTC", wrapper)
        .env(
            crate::rustc_driver::PNP_XTASK_EXE_ENV,
            std::env::current_exe().expect("resolve current xtask executable"),
        )
        .env(crate::rustc_driver::PNP_WORKSPACE_ROOT_ENV, ws_root)
        .env(crate::rustc_driver::ACCELERATED_ENV, "1")
        .env(crate::rustc_driver::ACCELERATED_MODE_ENV, "host-debug");
    if test_profile {
        cmd.args(ACCELERATED_TEST_CONFIG);
    }
}

fn check_literals_preflight(ws_root: &Path) -> i32 {
    check_literals::run(ws_root, false, &[])
}

/// Run the three preflights concurrently and join them in a fixed order.
///
/// The two source scans run on scoped threads; `freshness` runs on this thread
/// so the caller-owned `Invocation` (which holds a non-`Send` canonical-WIT
/// loader closure) stays put and can be reused for a stale rebuild. All three
/// overlap, so the gate costs `max(latency)` instead of their sum.
///
/// The generic seam exists so a test can assert both properties this function
/// promises: the outcomes are preserved one-for-one, and the three closures
/// genuinely overlap in time.
fn run_preflights_with<L, Q, F>(
    literals: L,
    test_quality: Q,
    freshness: F,
) -> (i32, i32, build_guests::CheckOutcome)
where
    L: FnOnce() -> i32 + Send,
    Q: FnOnce() -> i32 + Send,
    F: FnOnce() -> build_guests::CheckOutcome,
{
    std::thread::scope(|scope| {
        let literals = scope.spawn(literals);
        let test_quality = scope.spawn(test_quality);
        // Overlaps both scanners; see the doc comment for why it is not spawned.
        let freshness = freshness();
        let literals = literals
            .join()
            .expect("check-literals preflight thread panicked");
        let test_quality = test_quality
            .join()
            .expect("check-test-quality preflight thread panicked");
        (literals, test_quality, freshness)
    })
}

fn handle_guest_freshness_with(
    _ws_root: &Path,
    check: impl FnOnce() -> build_guests::CheckOutcome,
    rebuild: impl FnOnce(&[build_guests::GuestSpec]) -> i32,
) -> Option<i32> {
    let outcome = check();
    if outcome.code == build_guests::EXIT_INFRA_ERROR {
        eprintln!("xtask test: guest freshness check failed (infrastructure error); aborting.");
        return Some(outcome.code);
    }
    if outcome.code == build_guests::EXIT_STALE {
        eprintln!("xtask test: guest artifacts are stale; rebuilding...");
        let build_code = rebuild(&outcome.stale);
        if build_code != 0 {
            eprintln!("xtask test: guest rebuild failed; aborting test run.");
            return Some(build_code);
        }
    }
    None
}

/// Outcome of the pnp_cli freshness gate: the exit code `test_command` should
/// propagate (0 = fresh or rebuilt) plus, on abort, the named process-failure
/// detail identifying which subprocess failed.
struct PnpCliFreshness {
    code: i32,
    failure_detail: Option<String>,
}

fn ensure_pnp_cli_fresh(ws_root: &Path) -> PnpCliFreshness {
    ensure_pnp_cli_fresh_with(ws_root, |ws_root| {
        Command::new("cargo")
            .args(["build", "--bin", "pnp_cli"])
            .current_dir(ws_root)
            .status()
    })
}

fn ensure_pnp_cli_fresh_accelerated(ws_root: &Path, wrapper: &Path) -> PnpCliFreshness {
    ensure_pnp_cli_fresh_with(ws_root, |ws_root| {
        let mut cmd = Command::new("cargo");
        cmd.args(["build", "--bin", "pnp_cli"]).current_dir(ws_root);
        configure_accelerated_host_command(&mut cmd, wrapper, ws_root, true);
        cmd.status()
    })
}

fn ensure_pnp_cli_fresh_with(
    ws_root: &Path,
    run_rebuild: impl FnOnce(&Path) -> io::Result<std::process::ExitStatus>,
) -> PnpCliFreshness {
    eprintln!("xtask test: pnp_cli is stale or absent; rebuilding...");
    match run_rebuild(ws_root) {
        Ok(status) if status.success() => PnpCliFreshness {
            code: 0,
            failure_detail: None,
        },
        Ok(_) => {
            // Do not propagate the platform-specific process status: the
            // xtask entry point narrows i32 codes to ExitCode's u8 and a
            // failed subprocess code can otherwise wrap to zero.
            let detail = "xtask test: pnp_cli rebuild failed; aborting test run.".to_string();
            PnpCliFreshness {
                code: 1,
                failure_detail: Some(detail),
            }
        }
        Err(error) => {
            let detail = format!("xtask test: failed to start pnp_cli rebuild: {error}");
            PnpCliFreshness {
                code: 1,
                failure_detail: Some(detail),
            }
        }
    }
}

fn run_streaming_test(
    ws_root: &Path,
    test_args: &[String],
    log_path: &Path,
    accelerated_wrapper: Option<&Path>,
) -> Result<i32, String> {
    let log = fs::File::create(log_path)
        .map_err(|error| format!("failed to create {}: {error}", log_path.display()))?;
    let log = Arc::new(Mutex::new(log));

    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    if let Some(wrapper) = accelerated_wrapper {
        configure_accelerated_host_command(&mut cmd, wrapper, ws_root, true);
    }
    let mut child = cmd
        .args(test_args)
        .current_dir(ws_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn cargo test: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "cargo test stdout pipe was not available".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "cargo test stderr pipe was not available".to_string())?;

    let stdout_log = Arc::clone(&log);
    let stdout_thread = thread::spawn(move || tee_reader(stdout, io::stdout(), stdout_log));
    let stderr_thread = thread::spawn(move || tee_reader(stderr, io::stderr(), log));

    let status = child
        .wait()
        .map_err(|error| format!("failed waiting for cargo test: {error}"))?;
    stdout_thread
        .join()
        .map_err(|_| "cargo test stdout tee thread panicked".to_string())?
        .map_err(|error| format!("failed writing cargo stdout: {error}"))?;
    stderr_thread
        .join()
        .map_err(|_| "cargo test stderr tee thread panicked".to_string())?
        .map_err(|error| format!("failed writing cargo stderr: {error}"))?;

    Ok(status.code().unwrap_or(1))
}

fn tee_reader<R, W>(mut reader: R, mut terminal: W, log: Arc<Mutex<fs::File>>) -> io::Result<()>
where
    R: Read,
    W: Write,
{
    let mut buffer = [0_u8; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(());
        }

        terminal.write_all(&buffer[..count])?;
        let mut log = log
            .lock()
            .map_err(|_| io::Error::other("test log mutex was poisoned"))?;
        log.write_all(&buffer[..count])?;
    }
}

/// Print a compact, LLM-friendly digest of the test log.
///
/// Emits, in order:
///   1. Every `test result: ...` summary line (one per test binary).
///   2. Failure detail: every `FAILED` test name plus its
///      `---- <name> stdout ----` block (panic messages / captured output).
///      Skipped entirely on a green run.
///   3. A final `PASS` / `FAIL` verdict line.
fn print_summary(log_path: &Path, succeeded: bool) {
    let content = match fs::read_to_string(log_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("xtask test: could not read log for summary: {e}");
            return;
        }
    };
    let lines: Vec<&str> = content.lines().collect();

    // 1. Summary lines.
    let mut summaries: Vec<&str> = Vec::new();
    for line in &lines {
        if line.starts_with("test result:") {
            summaries.push(line);
        }
    }
    if summaries.is_empty() {
        println!("(no `test result:` lines found — build may have failed before tests ran)");
    } else {
        for s in &summaries {
            println!("{s}");
        }
    }

    // 2. Failure detail blocks.
    //
    // libtest's per-test failure block looks like:
    //
    //   ---- test_name stdout ----
    //   <captured stdout/stderr from the test body>
    //   <...>
    //
    //   (blank line ends the block)
    //
    // Failures also emit `FAILED` on the test-name line earlier, but the block
    // is the useful part. We collect each block header + its body up to the
    // next blank line, then print them.
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if l.starts_with("---- ") && l.contains(" stdout ----") {
            let header = l.to_string();
            let mut body: Vec<String> = Vec::new();
            i += 1;
            while i < lines.len() {
                let b = lines[i];
                if b.trim().is_empty() {
                    break;
                }
                body.push(b.to_string());
                i += 1;
            }
            blocks.push((header, body));
        } else {
            i += 1;
        }
    }

    // Also catch panic-location lines that libtest prints outside a captured
    // stdout block (e.g. `thread 'main' panicked at ...` from a process-level
    // panic, not a per-test failure). Skip lines already captured in a block
    // body so we don't duplicate them.
    let in_block: Vec<String> = blocks.iter().flat_map(|(_, b)| b.iter()).cloned().collect();
    let bare_panics: Vec<&str> = collect_bare_panics(&lines, &in_block);

    let process_details = collect_process_failure_details(&lines);
    let has_failures = !blocks.is_empty() || !bare_panics.is_empty() || !process_details.is_empty();
    if has_failures {
        println!();
        println!("---- failure detail ----");
        for (header, body) in &blocks {
            println!("{header}");
            for b in body {
                println!("{b}");
            }
            println!();
        }
        for p in &bare_panics {
            println!("{p}");
        }
        for detail in &process_details {
            println!("{detail}");
        }
    }

    // 3. Final verdict.
    println!();
    if succeeded && !has_failures {
        println!("VERDICT: PASS");
    } else {
        println!("VERDICT: FAIL");
    }
}

/// Collect bare panic-location lines that libtest prints outside a captured
/// stdout block (e.g. `thread 'main' panicked at ...` from a process-level
/// panic, not a per-test failure).
///
/// The `--summary` flow echoes an instructional grep line into the log
/// (`grep -n "FAILED|panicked at" <log>`). That line contains the literal
/// `panicked at` substring but is not a real panic, so a later
/// `--summary-from` parse must not count it as failure evidence. A genuine
/// panic line never contains the `FAILED|panicked at` grep pattern itself.
fn collect_bare_panics<'a>(lines: &[&'a str], in_block: &[String]) -> Vec<&'a str> {
    lines
        .iter()
        .filter(|line| {
            line.contains("panicked at")
                && !line.contains("FAILED|panicked at")
                && !line.starts_with("----")
                && !in_block.iter().any(|b| b == *line)
        })
        .copied()
        .collect()
}

/// Extract failure evidence that libtest does not format as a per-test block.
///
/// An allocator abort can terminate a test binary while the other workspace
/// binaries have already printed green summaries. In that case there is no
/// `FAILED` test block or panic line for `print_summary` to collect, but Cargo
/// still records the abnormal process exit and the allocator may leave a
/// diagnostic marker in the log.
fn collect_process_failure_details(lines: &[&str]) -> Vec<String> {
    let hard_markers = [
        "error: test failed",
        "process didn't exit successfully:",
        "OOM-GUARD TRIPPED",
        "requested SINGLE allocation",
    ];
    let has_hard_marker = lines
        .iter()
        .any(|line| hard_markers.iter().any(|marker| line.contains(marker)));
    if !has_hard_marker {
        return Vec::new();
    }

    lines
        .iter()
        .filter(|line| {
            hard_markers.iter().any(|marker| line.contains(marker))
                || line.contains("has been running for over")
        })
        .map(|line| (*line).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{collect_process_failure_details, parse_test_options, ACCELERATED_TEST_CONFIG};

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_string()).collect()
    }

    #[test]
    fn host_algos_selection_follows_the_package_selector() {
        let ws = std::path::Path::new("unused-for-package-selections");
        let cases: &[(&[&str], bool)] = &[
            (&["-p", "slicer-core"], true),
            (&["-p", "xtask"], false),
            (&["--package", "slicer-scheduler"], false),
            (&["--package=slicer-core"], true),
            (&["-pslicer-core"], true),
            (&["-p", "slicer-core@0.1.0"], true),
            (&["-p", "slicer-config", "-p", "slicer-core"], true),
            (&["--workspace"], true),
            (&["--workspace", "--exclude", "slicer-core"], false),
            (&["--workspace", "--exclude=slicer-core"], false),
            (&["--workspace", "--exclude", "slicer-core@0.1.0"], false),
            (&["--all"], true),
            (&[], true),
        ];
        for (args, expected) in cases {
            assert_eq!(
                super::selection_includes_slicer_core(ws, &strings(args)),
                *expected,
                "selection {args:?}"
            );
        }
    }

    #[test]
    fn foreign_package_selection_does_not_activate_slicer_core_features() {
        let args = super::build_test_arguments(
            std::path::Path::new("unused"),
            strings(&["-p", "xtask"]),
            Vec::new(),
        );
        assert_eq!(
            args,
            strings(&[
                "-p",
                "xtask",
                "--",
                "--skip",
                "arachne_parity_pipeline_concentric_infill_uses_arachne",
            ]),
            "a feature of a package outside the selection must not be requested"
        );
    }

    #[test]
    fn slicer_core_selection_activates_host_algos() {
        let args = super::build_test_arguments(
            std::path::Path::new("unused"),
            strings(&["-p", "slicer-core"]),
            Vec::new(),
        );
        assert_eq!(
            args,
            strings(&[
                "-p",
                "slicer-core",
                "--features",
                "slicer-core/host-algos",
                "--",
                "--skip",
                "arachne_parity_pipeline_concentric_infill_uses_arachne",
            ])
        );
    }

    #[test]
    fn caller_feature_choice_is_not_overridden() {
        for feature_args in [
            strings(&["-p", "slicer-core", "--features", "foo"]),
            strings(&["-p", "slicer-core", "--features=foo"]),
            strings(&["-p", "slicer-core", "--all-features"]),
        ] {
            let args = super::build_test_arguments(
                std::path::Path::new("unused"),
                feature_args,
                Vec::new(),
            );
            assert!(
                !args.iter().any(|arg| arg == "slicer-core/host-algos"),
                "{args:?}"
            );
        }
    }

    #[test]
    fn manifest_path_selection_reads_the_package_name() {
        let ws = std::env::temp_dir().join(format!("xtask-t109-manifest-{}", std::process::id()));
        std::fs::create_dir_all(ws.join("core")).expect("create core dir");
        std::fs::write(
            ws.join("core/Cargo.toml"),
            "[package]\nname = \"slicer-core\"\n",
        )
        .expect("write core manifest");
        std::fs::create_dir_all(ws.join("tool")).expect("create tool dir");
        std::fs::write(ws.join("tool/Cargo.toml"), "[package]\nname = \"xtask\"\n")
            .expect("write tool manifest");

        assert!(super::selection_includes_slicer_core(
            &ws,
            &strings(&["--manifest-path", "core/Cargo.toml"])
        ));
        assert!(super::selection_includes_slicer_core(
            &ws,
            &strings(&["--manifest-path=core/Cargo.toml"])
        ));
        assert!(!super::selection_includes_slicer_core(
            &ws,
            &strings(&["--manifest-path", "tool/Cargo.toml"])
        ));
        assert!(
            !super::selection_includes_slicer_core(
                &ws,
                &strings(&["--manifest-path", "missing/Cargo.toml"])
            ),
            "an unreadable manifest must not claim slicer-core"
        );

        std::fs::remove_dir_all(&ws).ok();
    }

    #[test]
    fn accelerated_entry_points_flag_parses() {
        let options = parse_test_options(&strings(&["--accelerated", "--summary", "--workspace"]))
            .expect("accelerated test flag parses");
        assert!(options.accelerated);
        assert!(options.summary);
        assert_eq!(options.test_args, strings(&["--workspace"]));
    }

    #[test]
    fn accelerated_entry_points_reject_doctest_combinations() {
        for args in [
            strings(&["--accelerated", "--doc"]),
            strings(&["--doc", "--accelerated"]),
            strings(&["--accelerated", "--doc=rustdoc"]),
            strings(&["--accelerated", "--", "--doc"]),
        ] {
            let error = parse_test_options(&args).expect_err("accelerated doctests must fail");
            assert!(error.contains("doctests"), "unexpected error: {error}");
        }
    }

    #[test]
    fn accelerated_entry_points_apply_controlled_debug_profile() {
        assert_eq!(
            ACCELERATED_TEST_CONFIG,
            &[
                "--config",
                "profile.dev.package.slicer-core.opt-level=3",
                "--config",
                "profile.dev.package.slicer-core.debug=2",
                "--config",
                "profile.dev.package.slicer-core.debug-assertions=true",
            ]
        );
    }

    #[test]
    fn accelerated_entry_points_ordinary_mode_unchanged() {
        let options =
            parse_test_options(&strings(&["--workspace"])).expect("ordinary test arguments parse");
        assert!(!options.accelerated);
        assert!(!options.summary);
        assert_eq!(options.summary_from, None);
        assert_eq!(options.test_args, strings(&["--workspace"]));
    }

    #[test]
    fn summary_ignores_instructional_grep_line_as_panic_evidence() {
        let lines = [
            "test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s",
            "               grep -n \"FAILED|panicked at\" target/test-output.log (failures)",
        ];

        let panics = super::collect_bare_panics(&lines, &[]);

        assert!(
            panics.is_empty(),
            "instructional grep line must not be counted as a panic, got: {panics:?}"
        );
    }

    #[test]
    fn summary_still_collects_real_panic_line() {
        let lines = [
            "thread 'main' panicked at 'boom', src/main.rs:12:5",
            "               grep -n \"FAILED|panicked at\" target/test-output.log (failures)",
        ];

        let panics = super::collect_bare_panics(&lines, &[]);

        assert_eq!(panics.len(), 1, "real panic must still be collected");
        assert!(panics[0].contains("boom"));
    }

    #[test]
    fn summary_collects_allocator_abort_without_libtest_failure_block() {
        let lines = [
            "test cube_4color_gcode_output_tdd::mmu_no_oversized_alloc_repeat has been running for over 60 seconds",
            "=================== OOM-GUARD TRIPPED (SINGLE) ===================",
            "requested SINGLE allocation = 1744830464 bytes  (1.625 GiB)",
            "Caused by:",
            "process didn't exit successfully: executor.exe (exit code: 173)",
        ];

        let details = collect_process_failure_details(&lines);

        assert!(details
            .iter()
            .any(|line| line.contains("mmu_no_oversized_alloc_repeat")));
        assert!(details.iter().any(|line| line.contains("1744830464")));
        assert!(details.iter().any(|line| line.contains("exit code: 173")));
    }

    #[test]
    fn summary_ignores_long_running_notice_without_failure_marker() {
        let lines = [
            "test slow_test has been running for over 60 seconds",
            "test slow_test ... ok",
        ];

        assert!(collect_process_failure_details(&lines).is_empty());
    }

    #[test]
    fn preflights_overlap_and_preserve_every_outcome() {
        use std::sync::{Arc, Mutex};
        use std::time::{Duration, Instant};

        // Each closure records the interval it was running in. If the three ran
        // one after another the intervals could not overlap and the assertion
        // below would fail — this tests the concurrency claim itself.
        let spans: Arc<Mutex<Vec<(Instant, Instant)>>> = Arc::new(Mutex::new(Vec::new()));
        let record = |spans: &Arc<Mutex<Vec<(Instant, Instant)>>>| {
            let started = Instant::now();
            std::thread::sleep(Duration::from_millis(250));
            spans
                .lock()
                .expect("span log poisoned")
                .push((started, Instant::now()));
        };

        let literals_spans = Arc::clone(&spans);
        let quality_spans = Arc::clone(&spans);
        let freshness_spans = Arc::clone(&spans);

        let (literals, test_quality, freshness) = super::run_preflights_with(
            move || {
                record(&literals_spans);
                11
            },
            move || {
                record(&quality_spans);
                22
            },
            move || {
                record(&freshness_spans);
                crate::build_guests::CheckOutcome {
                    stale: Vec::new(),
                    code: 33,
                }
            },
        );

        assert_eq!(literals, 11, "literals outcome must be preserved");
        assert_eq!(test_quality, 22, "test-quality outcome must be preserved");
        assert_eq!(freshness.code, 33, "freshness outcome must be preserved");

        let spans = spans.lock().expect("span log poisoned");
        assert_eq!(spans.len(), 3, "all three preflights must run");
        let latest_start = spans.iter().map(|(start, _)| *start).max().unwrap();
        let earliest_end = spans.iter().map(|(_, end)| *end).min().unwrap();
        assert!(
            latest_start < earliest_end,
            "preflights must overlap; recorded spans were {spans:?}"
        );
    }

    #[test]
    fn pnp_cli_rebuild_abort_is_nonzero_with_named_failure_detail() {
        // An empty workspace root has no target/{release,debug}/pnp_cli binary,
        // so the freshness gate deterministically decides a rebuild is needed
        // and invokes the injected runner instead of the real cargo build.
        let ws_root = std::env::temp_dir().join(format!(
            "xtask-pnp-cli-rebuild-abort-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&ws_root).expect("create fake workspace root");

        #[cfg(windows)]
        let failed_status = std::os::windows::process::ExitStatusExt::from_raw(1);
        #[cfg(unix)]
        let failed_status = std::os::unix::process::ExitStatusExt::from_raw(1 << 8);

        let outcome = super::ensure_pnp_cli_fresh_with(&ws_root, move |_| Ok(failed_status));

        std::fs::remove_dir_all(&ws_root).ok();

        assert_ne!(outcome.code, 0, "rebuild abort must return nonzero");
        let detail = outcome
            .failure_detail
            .expect("rebuild abort must report named process-failure detail");
        assert!(
            detail.contains("pnp_cli rebuild failed"),
            "failure detail must name the failing process, got: {detail}"
        );
    }

    #[test]
    fn preflight_blocks_on_violating_fixture_tree() {
        let root = std::env::temp_dir().join(format!(
            "xtask-check-literals-preflight-block-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&root).ok();
        let src = root.join("crates/probe/src");
        let tests = root.join("crates/probe/tests");
        std::fs::create_dir_all(&src).expect("create probe source directory");
        std::fs::create_dir_all(&tests).expect("create probe test directory");
        std::fs::write(
            src.join("lib.rs"),
            "pub struct Probe { a: i32, b: i32, c: i32, d: i32, e: i32 }\n",
        )
        .expect("write probe definition");
        std::fs::write(
            tests.join("probe.rs"),
            "fn probe() { let _ = Probe { a: 1, b: 2, c: 3, d: 4, e: 5 }; }\n",
        )
        .expect("write violating probe literal");

        let result = super::check_literals_preflight(&root);
        std::fs::remove_dir_all(&root).expect("remove fixture tree");

        assert_ne!(result, 0);
    }

    #[test]
    fn preflight_passes_on_clean_fixture_tree() {
        let root = std::env::temp_dir().join(format!(
            "xtask-check-literals-preflight-clean-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&root).ok();
        let src = root.join("crates/probe/src");
        let tests = root.join("crates/probe/tests");
        std::fs::create_dir_all(&src).expect("create probe source directory");
        std::fs::create_dir_all(&tests).expect("create probe test directory");
        std::fs::write(
            src.join("lib.rs"),
            "pub struct Probe { a: i32, b: i32, c: i32, d: i32, e: i32 }\n",
        )
        .expect("write probe definition");
        std::fs::write(
            tests.join("probe.rs"),
            "fn probe() { let _ = Probe { a: 1, b: 2, c: 3, d: 4, e: 5, ..Default::default() }; }\n",
        )
        .expect("write clean probe literal");

        let result = super::check_literals_preflight(&root);
        std::fs::remove_dir_all(&root).expect("remove fixture tree");

        assert_eq!(result, 0);
    }

    fn guest_spec(name: &str) -> crate::build_guests::GuestSpec {
        crate::build_guests::GuestSpec {
            // exhaustive: 7-field GuestSpec (AC-9/10/N4 fixtures)
            crate_name: name.to_string(),
            lib_name: name.replace('-', "_"),
            manifest_path: std::path::PathBuf::from(format!("{name}/Cargo.toml")),
            guest_dir: std::path::PathBuf::from(name),
            artifact_path: std::path::PathBuf::from(format!("{name}.wasm")),
            tree: crate::build_guests::GuestTree::Core,
            stage_id: None,
        }
    }

    #[test]
    fn infrastructure_error_aborts_without_rebuilding() {
        use std::cell::Cell;
        use std::rc::Rc;

        let ws = std::path::PathBuf::from("/tmp/fake-ws");
        let rebuilt = Rc::new(Cell::new(false));
        let rebuilt_clone = Rc::clone(&rebuilt);

        let code = super::handle_guest_freshness_with(
            &ws,
            || crate::build_guests::CheckOutcome {
                stale: Vec::new(),
                code: crate::build_guests::EXIT_INFRA_ERROR,
            },
            |_| {
                rebuilt_clone.set(true);
                0
            },
        );

        assert_eq!(code, Some(crate::build_guests::EXIT_INFRA_ERROR));
        assert!(!rebuilt.get(), "infra error must not invoke rebuild");
    }

    #[test]
    fn test_command_rebuilds_only_the_stale_specs() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let ws = std::path::PathBuf::from("/tmp/fake-ws");
        let stale = vec![guest_spec("guest-a"), guest_spec("guest-b")];
        let stale_names: Vec<String> = stale.iter().map(|g| g.crate_name.clone()).collect();
        let seen: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let seen_clone = Rc::clone(&seen);

        let code = super::handle_guest_freshness_with(
            &ws,
            move || crate::build_guests::CheckOutcome {
                code: crate::build_guests::EXIT_STALE,
                stale: stale.clone(),
            },
            move |received| {
                *seen_clone.borrow_mut() = received.iter().map(|g| g.crate_name.clone()).collect();
                0
            },
        );

        assert_eq!(
            code, None,
            "successful stale rebuild must return None (continue)"
        );
        assert_eq!(*seen.borrow(), stale_names);
    }

    #[test]
    fn failed_stale_rebuild_aborts_the_suite() {
        let ws = std::path::PathBuf::from("/tmp/fake-ws");
        let stale = vec![guest_spec("guest-a")];

        let code = super::handle_guest_freshness_with(
            &ws,
            move || crate::build_guests::CheckOutcome {
                code: crate::build_guests::EXIT_STALE,
                stale: stale.clone(),
            },
            |_| 1,
        );

        assert_ne!(code, Some(0));
        assert!(
            code.is_some(),
            "failed rebuild must abort with non-zero code"
        );
    }
    #[test]
    fn pnp_cli_rebuild_closure_always_runs_even_when_binary_is_newer() {
        use std::cell::Cell;
        use std::rc::Rc;
        use std::thread;
        use std::time::Duration;

        let ws_root =
            std::env::temp_dir().join(format!("xtask-pnp-cli-always-runs-{}", std::process::id()));
        // Touch ClosureCache::len so cargo clippy -D warnings stays green
        // (build_guests.rs defines len but it is otherwise unused until future steps).
        let _ = crate::build_guests::ClosureCache::new().len();
        std::fs::remove_dir_all(&ws_root).ok();
        let src_dir = ws_root.join("crates/pnp-cli/src");
        std::fs::create_dir_all(&src_dir).expect("create src dir");
        std::fs::write(src_dir.join("main.rs"), "fn main() {}").expect("write source");
        std::fs::write(
            ws_root.join("crates/pnp-cli/Cargo.toml"),
            "[package]\nname = \"pnp-cli\"\n",
        )
        .expect("write manifest");
        std::fs::create_dir_all(ws_root.join("crates/pnp-cli")).ok();
        thread::sleep(Duration::from_millis(1100));
        let exe_name = if cfg!(windows) {
            "pnp_cli.exe"
        } else {
            "pnp_cli"
        };
        let bin_path = ws_root.join("target").join("debug").join(exe_name);
        std::fs::create_dir_all(bin_path.parent().unwrap()).expect("create bin dir");
        std::fs::write(&bin_path, b"fake binary").expect("write binary");
        thread::sleep(Duration::from_millis(50));
        assert!(bin_path.is_file(), "binary fixture must exist");
        let bin_mtime = std::fs::metadata(&bin_path).and_then(|m| m.modified()).ok();
        let src_mtime = std::fs::metadata(src_dir.join("main.rs"))
            .and_then(|m| m.modified())
            .ok();
        if let (Some(b), Some(s)) = (bin_mtime, src_mtime) {
            assert!(
                b > s,
                "binary must be provably newer than source: {b:?} <= {s:?}"
            );
        }
        let invoked = Rc::new(Cell::new(false));
        let invoked_clone = Rc::clone(&invoked);
        #[cfg(windows)]
        let success_status = std::os::windows::process::ExitStatusExt::from_raw(0);
        #[cfg(unix)]
        let success_status = std::os::unix::process::ExitStatusExt::from_raw(0);
        let outcome = super::ensure_pnp_cli_fresh_with(&ws_root, move |_| {
            invoked_clone.set(true);
            Ok(success_status)
        });
        std::fs::remove_dir_all(&ws_root).ok();
        assert!(
            invoked.get(),
            "closure must be invoked even though binary is newer than every source"
        );
        assert_eq!(outcome.code, 0);
        assert!(outcome.failure_detail.is_none());
    }
}

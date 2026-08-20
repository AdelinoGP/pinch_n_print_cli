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
    let mut summary = false;
    let mut summary_from: Option<String> = None;
    let mut test_args: Vec<String> = Vec::with_capacity(passthrough.len());

    let mut iter = passthrough.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--summary" => summary = true,
            "--summary-from" => {
                summary_from = iter.next().cloned();
                if summary_from.is_none() {
                    eprintln!("xtask test: --summary-from requires a file path argument");
                    return 2;
                }
            }
            other => test_args.push(other.to_string()),
        }
    }

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
    // We also quarantine deliberate RED parity anchors / out-of-scope tests via the
    // libtest `--skip` filter so a green gate stays meaningful (only NEW breakage
    // fails). These are tracked in docs/specs/arachne-parity-recovery.md. We skip
    // them at the runner rather than `#[ignore]`-ing them: sibling RED-anchor files
    // (arachne_parity_gaps.rs, arachne_parity_round2.rs) carry a checked-in policy
    // forbidding `#[ignore]` on this test family, and the roster stays diffable here.
    const QUARANTINED_TESTS: &[&str] = &[
        // Concentric-infill-through-Arachne — out of scope (D-104f; user decision
        // 2026-07-15: not on the roadmap, may never be Arachne).
        "arachne_parity_pipeline_concentric_infill_uses_arachne",
    ];

    // Split caller args at the first `--` into cargo-level and libtest-level args so
    // `--features` lands on the cargo side and `--skip` on the libtest side.
    let (mut cargo_args, mut libtest_args): (Vec<String>, Vec<String>) =
        match test_args.iter().position(|a| a == "--") {
            Some(i) => (test_args[..i].to_vec(), test_args[i + 1..].to_vec()),
            None => (test_args.clone(), Vec::new()),
        };

    // Enforce host-algos unless the caller already chose features explicitly.
    let caller_set_features = cargo_args
        .iter()
        .any(|a| a == "--features" || a.starts_with("--features=") || a == "--all-features");
    if !caller_set_features {
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

    // Step 1: check-literals preflight, then freshness check.
    let literals_code = check_literals_preflight(ws_root);
    if literals_code != 0 {
        eprintln!(
            "xtask test: check-literals preflight failed; fix violations or add reasoned waivers (docs/21_data_defaults_and_fixtures.md), then re-run."
        );
        return 1;
    }

    if let Some(code) = handle_guest_freshness_with(
        ws_root,
        || build_guests::check_command(ws_root),
        |stale| build_guests::build_stale_command(ws_root, stale),
    ) {
        return code;
    }

    let freshness = ensure_pnp_cli_fresh(ws_root);
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
        match run_streaming_test(ws_root, &test_args, &log_path) {
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

fn check_literals_preflight(ws_root: &Path) -> i32 {
    check_literals::run(ws_root, false, &[])
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
) -> Result<i32, String> {
    let log = fs::File::create(log_path)
        .map_err(|error| format!("failed to create {}: {error}", log_path.display()))?;
    let log = Arc::new(Mutex::new(log));

    let mut child = Command::new("cargo")
        .arg("test")
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
    use super::collect_process_failure_details;

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
        crate::build_guests::GuestSpec { // exhaustive: 7-field GuestSpec (AC-9/10/N4 fixtures)
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

        assert_eq!(code, None, "successful stale rebuild must return None (continue)");
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
        assert!(code.is_some(), "failed rebuild must abort with non-zero code");
    }
    #[test]
    fn pnp_cli_rebuild_closure_always_runs_even_when_binary_is_newer() {
        use std::cell::Cell;
        use std::rc::Rc;
        use std::thread;
        use std::time::Duration;

        let ws_root = std::env::temp_dir().join(format!(
            "xtask-pnp-cli-always-runs-{}",
            std::process::id()
        ));
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
        let bin_mtime = std::fs::metadata(&bin_path)
            .and_then(|m| m.modified())
            .ok();
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

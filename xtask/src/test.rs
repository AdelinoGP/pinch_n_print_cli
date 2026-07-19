use std::fs;
use std::path::Path;
use std::process::Command;

use crate::build_guests;

// Note: we spawn via a shell (cmd /C on Windows, sh -c on Unix) so we can use
// `tee` to stream live output while also capturing it to the log file.

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
/// All modes run the guest-WASM freshness check first (`build-guests --check`),
/// rebuilding if stale, UNLESS `--summary-from` is given (no test run = no gate).
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

    // Step 1: freshness check.
    let check_code = build_guests::check_command(ws_root);
    if check_code != 0 {
        eprintln!("xtask test: guest artifacts are stale; rebuilding...");
        let build_code = build_guests::build_command(ws_root);
        if build_code != 0 {
            eprintln!("xtask test: guest rebuild failed; aborting test run.");
            return build_code;
        }
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
        // Live-stream mode: shell out with `tee` so output is visible AND logged.
        #[cfg(windows)]
        {
            let test_cmd = format!(
                "cargo test {} 2>&1 | tee {}",
                test_args.join(" "),
                log_path.display()
            );
            eprintln!("xtask test: running `cargo test {}`", test_args.join(" "));
            match Command::new("cmd").arg("/C").arg(&test_cmd).status() {
                Ok(s) if s.success() => (s.code().unwrap_or(0), true),
                Ok(s) => (s.code().unwrap_or(1), false),
                Err(e) => {
                    eprintln!("xtask test: failed to spawn cargo test: {e}");
                    return 1;
                }
            }
        }
        #[cfg(not(windows))]
        {
            let test_cmd = format!(
                "cargo test {} 2>&1 | tee {}",
                test_args.join(" "),
                log_path.display()
            );
            eprintln!("xtask test: running `cargo test {}`", test_args.join(" "));
            match Command::new("sh").arg("-c").arg(&test_cmd).status() {
                Ok(s) if s.success() => (s.code().unwrap_or(0), true),
                Ok(s) => (s.code().unwrap_or(1), false),
                Err(e) => {
                    eprintln!("xtask test: failed to spawn cargo test: {e}");
                    return 1;
                }
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
    let mut bare_panics: Vec<&str> = Vec::new();
    for line in &lines {
        if line.contains("panicked at")
            && !line.starts_with("----")
            && !in_block.iter().any(|b| b == line)
        {
            bare_panics.push(line);
        }
    }

    let has_failures = !blocks.is_empty() || !bare_panics.is_empty();
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
    }

    // 3. Final verdict.
    println!();
    if succeeded && !has_failures {
        println!("VERDICT: PASS");
    } else {
        println!("VERDICT: FAIL");
    }
}

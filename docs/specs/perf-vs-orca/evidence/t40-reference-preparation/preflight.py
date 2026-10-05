"""Run preparation prerequisites only; no slices or acceptance timings.

Every Rust test's combined output is teed to target/test-output.log and copied
here before another test can overwrite it. Missing named tests fail closed.
"""

import json
import os
from pathlib import Path
import subprocess
import sys

EVIDENCE = Path(__file__).resolve().parent
ROOT = EVIDENCE.parents[4]
TEST_LOG = ROOT / "target/test-output.log"
RESULTS = []


def save():
    (EVIDENCE / "preflight-results.json").write_text(
        json.dumps(RESULTS, indent=2) + "\n", encoding="utf-8"
    )


def run(name, command, *, tests=(), xtask=False):
    print(f"CHECK {name}: {subprocess.list2cmdline(command)}", flush=True)
    log = EVIDENCE / f"{name}.log"
    TEST_LOG.parent.mkdir(parents=True, exist_ok=True)
    with log.open("w", encoding="utf-8") as capture:
        test_capture = (
            TEST_LOG.open("w", encoding="utf-8") if tests and not xtask else None
        )
        try:
            with subprocess.Popen(
                command,
                cwd=ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                encoding="utf-8",
                errors="replace",
            ) as process:
                for line in process.stdout:
                    capture.write(line)
                    if test_capture:
                        test_capture.write(line)
                    print(line, end="", flush=True)
                code = process.wait()
        finally:
            if test_capture:
                test_capture.close()
    text = (
        TEST_LOG.read_text(encoding="utf-8")
        if tests
        else log.read_text(encoding="utf-8")
    )
    if tests:
        (EVIDENCE / f"{name}.full.log").write_text(text, encoding="utf-8")
    missing = [
        test
        for test in tests
        if not any(
            line.startswith("test ") and test in line and line.endswith(" ... ok")
            for line in text.splitlines()
        )
    ]
    result = {
        "name": name,
        "command": command,
        "exit_code": code,
        "required_tests": list(tests),
        "missing_successful_tests": missing,
    }
    RESULTS.append(result)
    save()
    if code or missing:
        raise RuntimeError(
            f"prerequisite failed: {name}, exit={code}, missing={missing}"
        )
    return text


def main():
    for path in (EVIDENCE / "preflight-results.json",):
        if path.exists():
            raise RuntimeError(f"will not overwrite preparation evidence: {path}")
    environment = {
        key: value
        for key, value in os.environ.items()
        if key.startswith(("CARGO", "RUST", "PNP_", "SLICER_"))
    }
    (EVIDENCE / "build-environment.json").write_text(
        json.dumps(environment, indent=2) + "\n", encoding="utf-8"
    )
    forbidden = (
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_RUSTC",
        "CARGO_BUILD_RUSTFLAGS",
        "PNP_ACCELERATED",
        "PNP_ACCELERATED_MODE",
        "SLICER_MODULE_PATH",
    )
    present = [key for key in forbidden if os.environ.get(key)]
    if present:
        raise RuntimeError(f"unexpected ordinary build/module environment: {present}")
    compiler = run("toolchain", ["rustc", "-vV"])
    identity = dict(
        line.split(": ", 1) for line in compiler.splitlines() if ": " in line
    )
    expected = {
        "release": "1.96.0",
        "commit-hash": "ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96",
        "host": "x86_64-pc-windows-msvc",
        "LLVM version": "22.1.2",
    }
    if any(identity.get(key) != value for key, value in expected.items()):
        raise RuntimeError(f"compiler policy mismatch: {identity}")
    run("ordinary-freshness", ["cargo", "xtask", "build-guests", "--check"])
    run(
        "support-repair",
        [
            "cargo",
            "test",
            "-p",
            "slicer-wasm-host",
            "--test",
            "contract",
            "--",
            "support_plan_validation",
            "--nocapture",
        ],
        tests=(
            "identity_aggregate_spread_across_the_plate_is_measured_per_body",
            "support_body_wider_than_the_deleted_routing_cell_is_retained",
        ),
    )
    run(
        "empty-infill-producers",
        [
            "cargo",
            "test",
            "-p",
            "slicer-wasm-host",
            "--test",
            "contract",
            "--",
            "infill_postprocess_empty_commit_tdd",
            "--nocapture",
        ],
        tests=(
            "wasm_infill_postprocess_empty_output_commits_the_empty_replacement",
            "native_infill_postprocess_empty_output_commits_the_empty_replacement",
            "wasm_infill_empty_output_still_commits_nothing",
            "native_infill_empty_output_still_commits_nothing",
            "both_legs_agree_on_the_empty_replacement_shape",
        ),
    )
    run(
        "empty-infill-runtime",
        [
            "cargo",
            "xtask",
            "test",
            "--summary",
            "-p",
            "slicer-runtime",
            "--test",
            "contract",
            "--",
            "infill_postprocess_contract_tdd",
            "--nocapture",
        ],
        xtask=True,
        tests=(
            "infill_postprocess_empty_replacement_supersedes_prior_ir",
            "infill_postprocess_inside_path_survives_so_the_clip_is_real",
        ),
    )
    print(
        "PASS: ordinary reference preparation prerequisites; no reference generation yet."
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"STOP: {error}", file=sys.stderr)
        sys.exit(1)

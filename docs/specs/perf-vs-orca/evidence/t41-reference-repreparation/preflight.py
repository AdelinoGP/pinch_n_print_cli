"""Run t41 preparation prerequisites only; no slices and no benchmark timing.

Human-authorized fresh ordinary-only re-preparation (issue 41), NOT a retry of
t40: this script writes fresh evidence into this directory and never consumes
t40's preflight/state as its prerequisites. The durable payload root it
authorizes lives under `.local-artifacts/perimeter-reference-preparation/`
(outside `target/`).

Every Rust test's combined output is teed to `target/test-output.log` and copied
here before another test can overwrite it. Missing named tests, an unexpected
passed count, or a freshness "could not form an opinion" (exit 3) fail closed.
"""

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import runpy
import subprocess
import sys

EVIDENCE = Path(__file__).resolve().parent
ROOT = EVIDENCE.parents[4]
TEST_LOG = ROOT / "target" / "test-output.log"

# Exact passed counts observed on the t40 reference tree; the source-input set
# is byte-identical, so these are the expected counts. A mismatch is a STOP,
# never a pass.
EXPECTED_PASSED = {
    "support-repair": 8,
    "empty-infill-producers": 6,
    "empty-infill-runtime": 9,
}

REQUIRED_TESTS = {
    "support-repair": (
        "identity_aggregate_spread_across_the_plate_is_measured_per_body",
        "support_body_wider_than_the_deleted_routing_cell_is_retained",
    ),
    "empty-infill-producers": (
        "wasm_infill_postprocess_empty_output_commits_the_empty_replacement",
        "native_infill_postprocess_empty_output_commits_the_empty_replacement",
        "wasm_infill_empty_output_still_commits_nothing",
        "native_infill_empty_output_still_commits_nothing",
        "both_legs_agree_on_the_empty_replacement_shape",
    ),
    "empty-infill-runtime": (
        "infill_postprocess_empty_replacement_supersedes_prior_ir",
        "infill_postprocess_inside_path_survives_so_the_clip_is_real",
    ),
}

FORBIDDEN_ENV = (
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

# The full controlled-build identity, exactly as the docs/23 policy grammar
# states it. Every constant is compared, including the wasm target; a widened
# value or an extra constant fails.
ALLOWED_COMPILE_POLICY = {
    "RELEASE": "1.96.0",
    "COMMIT_HASH": "ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96",
    "HOST": "x86_64-pc-windows-msvc",
    "LLVM_VERSION": "22.1.2",
    "WASM_TARGET": "wasm32-unknown-unknown",
}

RESULTS = {"status": "running", "gates": [], "checks": {}}


def utc():
    return datetime.now(timezone.utc).isoformat()


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def save():
    write_json(EVIDENCE / "preflight-results.json", RESULTS)


def parse_counts(text):
    totals = {
        "summary_lines": 0,
        "passed": 0,
        "failed": 0,
        "ignored": 0,
        "measured": 0,
        "filtered_out": 0,
    }
    pattern = re.compile(
        r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
        r"(\d+) measured; (\d+) filtered out"
    )
    for line in text.splitlines():
        match = pattern.match(line)
        if match:
            totals["summary_lines"] += 1
            for index, key in enumerate(
                ("passed", "failed", "ignored", "measured", "filtered_out"), start=2
            ):
                totals[key] += int(match.group(index))
    return totals


def run(
    name,
    command,
    *,
    tests=(),
    xtask=False,
    expected_exact=None,
    allow_nonzero=False,
):
    """Run one prerequisite, tee output, and record a fail-closed gate result."""
    print(f"CHECK {name}: {subprocess.list2cmdline(command)}", flush=True)
    log = EVIDENCE / f"{name}.log"
    TEST_LOG.parent.mkdir(parents=True, exist_ok=True)
    if tests and xtask:
        # `cargo xtask test` writes the log itself; remove a stale file so an
        # abort before the test run cannot masquerade as a green summary.
        TEST_LOG.unlink(missing_ok=True)
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
    if tests and xtask and not TEST_LOG.is_file():
        raise RuntimeError(f"{name}: xtask test produced no {TEST_LOG}")
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
    counts = parse_counts(text)
    result = {
        "name": name,
        "command": command,
        "exit_code": code,
        "required_tests": list(tests),
        "missing_successful_tests": missing,
        "passed_counts": counts,
        "combined_output_sha256": hashlib.sha256(text.encode("utf-8")).hexdigest(),
    }
    if expected_exact is not None:
        result["expected_passed_exact"] = expected_exact
    RESULTS["gates"].append(result)
    save()
    if not allow_nonzero and (code or missing):
        raise RuntimeError(
            f"prerequisite failed: {name}, exit={code}, missing={missing}"
        )
    return result


def require_counts(result):
    expected = EXPECTED_PASSED[result["name"]]
    counts = result["passed_counts"]
    if counts["summary_lines"] < 1 or counts["passed"] != expected or counts["failed"]:
        raise RuntimeError(
            f"prerequisite count mismatch: {result['name']}: expected exactly "
            f"{expected} passed / 0 failed, got {counts}"
        )


def freshness_gate():
    """Ordinary guest freshness with the policy-approved recovery path.

    exit 0 -> proceed; exit 1 -> ordinary rebuild (no lock sync, no
    production change) then recheck; exit 3 -> STOP (checker cannot form an
    opinion). A rebuild that leaves the check non-clean is a STOP.
    """
    first = run(
        "ordinary-freshness",
        ["cargo", "xtask", "build-guests", "--check"],
        allow_nonzero=True,
    )
    if first["exit_code"] == 3:
        raise RuntimeError(
            "ordinary guest freshness checker could not form an opinion (exit 3); "
            "stopping without lock sync or repair"
        )
    if first["exit_code"] == 1:
        rebuild = run(
            "ordinary-guest-rebuild",
            ["cargo", "xtask", "build-guests"],
            allow_nonzero=True,
        )
        if rebuild["exit_code"]:
            raise RuntimeError(
                f"ordinary guest rebuild failed: exit {rebuild['exit_code']}"
            )
        recheck = run(
            "ordinary-freshness-recheck",
            ["cargo", "xtask", "build-guests", "--check"],
            allow_nonzero=True,
        )
        RESULTS["freshness"] = {
            "initial_exit": 1,
            "rebuild_exit": rebuild["exit_code"],
            "recheck_exit": recheck["exit_code"],
            "final_exit": recheck["exit_code"],
            "note": "stale artifacts rebuilt with plain `build-guests`; no --sync-locks, no --force",
        }
        if recheck["exit_code"]:
            raise RuntimeError(
                "ordinary guest freshness is still not clean after rebuild; lock "
                "divergence or unfixable staleness remains (NOT syncing locks "
                "without explicit permission)"
            )
    else:
        RESULTS["freshness"] = {
            "initial_exit": first["exit_code"],
            "rebuild_exit": None,
            "recheck_exit": None,
            "final_exit": first["exit_code"],
            "note": "artifacts were already fresh; no rebuild ran",
        }
    save()
    if RESULTS["freshness"]["final_exit"] != 0:
        raise RuntimeError("ordinary guest freshness gate did not reach exit 0")


def read_allowlist(driver_text):
    """Parse the controlled-build allowlist constants out of rustc_driver.rs."""
    return dict(
        re.findall(r'pub\(crate\) const ALLOWED_(\w+): &str = "([^"]+)"', driver_text)
    )


def toolchain_gate():
    result = run("toolchain", ["rustc", "-vV"])
    identity = dict(
        line.split(": ", 1)
        for line in (EVIDENCE / "toolchain.log")
        .read_text(encoding="utf-8")
        .splitlines()
        if ": " in line
    )
    expected = {
        "release": "1.96.0",
        "commit-hash": "ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96",
        "host": "x86_64-pc-windows-msvc",
        "LLVM version": "22.1.2",
    }
    if any(identity.get(key) != value for key, value in expected.items()):
        raise RuntimeError(f"compiler policy mismatch: {identity}")
    # Independently re-read the controlled-build allowlist so a widened or
    # range-based policy is caught here, not only by xtask at build time.
    # Every ALLOWED_* constant is compared, including WASM_TARGET; an extra
    # constant the comparison set does not know about also fails.
    driver = (ROOT / "xtask/src/rustc_driver.rs").read_text(encoding="utf-8")
    allowlist = read_allowlist(driver)
    if allowlist != ALLOWED_COMPILE_POLICY:
        raise RuntimeError(
            f"controlled-build allowlist mismatch: {allowlist} != "
            f"{ALLOWED_COMPILE_POLICY}"
        )
    return result


def capture_source_inputs():
    """Reuse prepare.py's collector so the set cannot drift between scripts."""
    prepare = runpy.run_path(str(EVIDENCE / "prepare.py"))
    return prepare["source_inputs"]()


def inspect_targets():
    """Verify the named tests exist, are ungated, and their targets compile.

    This is the authoring-time features/target-cfg check the t40 lesson
    demands: a `required-features` entry or a file-level `#![cfg(...)]` would
    silently compile a target to zero tests and report a green wall.
    """
    import tomllib

    required = {
        "crates/slicer-wasm-host/tests/contract/support_plan_validation.rs": [
            "identity_aggregate_spread_across_the_plate_is_measured_per_body",
            "support_body_wider_than_the_deleted_routing_cell_is_retained",
        ],
        "crates/slicer-wasm-host/tests/contract/infill_postprocess_empty_commit_tdd.rs": [
            "wasm_infill_postprocess_empty_output_commits_the_empty_replacement",
            "native_infill_postprocess_empty_output_commits_the_empty_replacement",
            "wasm_infill_empty_output_still_commits_nothing",
            "native_infill_empty_output_still_commits_nothing",
            "both_legs_agree_on_the_empty_replacement_shape",
        ],
        "crates/slicer-runtime/tests/contract/infill_postprocess_contract_tdd.rs": [
            "infill_postprocess_empty_replacement_supersedes_prior_ir",
            "infill_postprocess_inside_path_survives_so_the_clip_is_real",
        ],
    }
    report = {"ok": True, "crates": {}, "required_tests": {}}
    for crate in ("slicer-wasm-host", "slicer-runtime"):
        data = tomllib.loads(
            (ROOT / f"crates/{crate}/Cargo.toml").read_text(encoding="utf-8")
        )
        test_targets = data.get("test", [])
        report["crates"][crate] = {
            "test_targets": [target.get("name") for target in test_targets],
            "targets_with_required_features": {
                target["name"]: target["required-features"]
                for target in test_targets
                if target.get("required-features")
            },
            "contract_declared_explicitly": any(
                target.get("name") == "contract" for target in test_targets
            ),
            "contract_path_exists": (
                ROOT / f"crates/{crate}/tests/contract/main.rs"
            ).is_file(),
            "features_default": data.get("features", {}).get("default"),
        }
    for relative, names in required.items():
        text = (ROOT / relative).read_text(encoding="utf-8")
        report["required_tests"][relative] = {"file_level_cfg": []}
        for match in re.finditer(r"#!\[cfg\(([^\]]*)\)\]", text):
            report["required_tests"][relative]["file_level_cfg"].append(match.group(1))
        if report["required_tests"][relative]["file_level_cfg"]:
            report["ok"] = False
        for name in names:
            index = text.find(f"fn {name}(")
            if index < 0:
                report["required_tests"][relative][name] = "missing"
                report["ok"] = False
                continue
            window = text[max(0, index - 400) : index]
            gated = bool(re.search(r"#\[cfg\((?!test\b)", window))
            report["required_tests"][relative][name] = "gated" if gated else "ungated"
            if gated:
                report["ok"] = False
    if not report["ok"]:
        raise RuntimeError("required test is missing or cfg-gated; refusing to run")
    return report


def main():
    if (EVIDENCE / "preflight-results.json").exists():
        raise RuntimeError("will not overwrite preparation preflight evidence")
    environment = {
        key: value
        for key, value in os.environ.items()
        if key.startswith(("CARGO", "RUST", "PNP_", "SLICER_"))
    }
    write_json(EVIDENCE / "build-environment.json", environment)
    present = [key for key in FORBIDDEN_ENV if os.environ.get(key)]
    if present:
        raise RuntimeError(f"unexpected ordinary build/module environment: {present}")

    RESULTS["started_utc"] = utc()
    save()

    # Capture the source-input set before any prerequisites build; the late
    # capture must be identical, and prepare.py must agree with the late set.
    source_inputs_early = capture_source_inputs()

    toolchain_gate()
    freshness_gate()

    support = run(
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
        tests=REQUIRED_TESTS["support-repair"],
        expected_exact=EXPECTED_PASSED["support-repair"],
    )
    require_counts(support)

    producers = run(
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
        tests=REQUIRED_TESTS["empty-infill-producers"],
        expected_exact=EXPECTED_PASSED["empty-infill-producers"],
    )
    require_counts(producers)

    runtime = run(
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
        tests=REQUIRED_TESTS["empty-infill-runtime"],
        expected_exact=EXPECTED_PASSED["empty-infill-runtime"],
    )
    require_counts(runtime)

    inspection = inspect_targets()
    source_inputs_late = capture_source_inputs()
    if source_inputs_late != source_inputs_early:
        raise RuntimeError("build inputs changed while prerequisites ran")
    write_json(EVIDENCE / "source-inputs.json", source_inputs_late)
    RESULTS["checks"] = {
        "source_inputs": {
            "count": len(source_inputs_late),
            "stable_across_preflight": True,
            "sha256": digest(EVIDENCE / "source-inputs.json"),
        },
        "target_inspection": inspection,
        "freshness": RESULTS["freshness"],
    }
    RESULTS["status"] = "passed"
    RESULTS["finished_utc"] = utc()
    save()
    print(
        "PASS: t41 ordinary reference preparation prerequisites; "
        "no reference generation yet.",
        flush=True,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        RESULTS["status"] = "stopped"
        RESULTS["stopped_utc"] = utc()
        RESULTS["failure"] = str(error)
        save()
        print(f"STOP: {error}", file=sys.stderr)
        sys.exit(1)

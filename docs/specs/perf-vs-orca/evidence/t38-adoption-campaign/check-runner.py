"""Read-only campaign input/runner checks; no builds, slices or timing."""

import json
import os
from pathlib import Path
import subprocess
import sys

EVIDENCE = Path(__file__).resolve().parent
ROOT = EVIDENCE.parents[4]
RESULTS = []


def run(name, command, expected=0, env=None):
    result = subprocess.run(command, cwd=ROOT, capture_output=True, env=env)
    (EVIDENCE / f"{name}.stdout.log").write_bytes(result.stdout)
    (EVIDENCE / f"{name}.stderr.log").write_bytes(result.stderr)
    RESULTS.append({"name": name, "command": command, "exit_code": result.returncode})
    (EVIDENCE / "runner-results.json").write_text(json.dumps(RESULTS, indent=2) + "\n")
    if result.returncode != expected:
        raise RuntimeError(f"{name}: unexpected exit {result.returncode}")
    return result.stdout.decode("utf-8-sig")


def main():
    if (EVIDENCE / "runner-results.json").exists():
        raise RuntimeError("existing campaign check evidence; do not overwrite/retry")
    freeze = json.loads(
        run(
            "freeze-before",
            [
                sys.executable,
                "docs/specs/perf-vs-orca/evidence/t40-reference-preparation/verify-freeze.py",
                "docs/specs/perf-vs-orca/evidence/t40-reference-preparation/attempt-2",
            ],
        )
    )
    run(
        "harness-copy",
        [
            "cmp",
            "resources/perimeter-acceptance/run_bench.ps1",
            "docs/specs/perf-vs-orca/evidence/alloc-bench/run_bench.ps1",
        ],
    )
    run(
        "status-roundtrip",
        [
            "pwsh",
            "-NoProfile",
            "-File",
            "resources/perimeter-acceptance/test-status-roundtrip.ps1",
        ],
    )
    dry = json.loads(
        run(
            "dry-run",
            [
                "pwsh",
                "-NoProfile",
                "-File",
                "resources/perimeter-acceptance/run-acceptance.ps1",
                "-DryRun",
            ],
        )
    )
    assert dry["automatic_commit"] is False and len(dry["cells"]) == 6
    assert dry["status"] == "DROP"
    assert dry["cells"][0]["decision"] == "inconclusive"
    assert dry["cells"][1]["decision"] == "DROP"
    absent = ROOT / "target/perimeter-acceptance/t38-absent-corpus"
    assert not absent.exists(), "negative-control corpus must be absent"
    missing = run(
        "missing-corpus",
        [
            "pwsh",
            "-NoProfile",
            "-File",
            "resources/perimeter-acceptance/run-acceptance.ps1",
            "-Workload",
            "supports-off-benchy",
            "-ExpectedGenerator",
            "classic",
            "-CorpusRoot",
            str(absent),
        ],
        expected=1,
    )
    assert missing.startswith("missing-artifact: ")
    environment = {
        key: value
        for key, value in os.environ.items()
        if key.startswith(("CARGO", "RUST", "PNP_", "SLICER_"))
    }
    (EVIDENCE / "environment.json").write_text(json.dumps(environment, indent=2) + "\n")
    assert not os.environ.get("SLICER_MODULE_PATH"), "unexpected module environment"
    snapshot = Path(freeze["ordinary_snapshot_root"])
    env = dict(os.environ, SLICER_DEBUG_PATHS="1")
    for name, flags in (
        ("runner-default-discovery", []),
        (
            "isolated-discovery",
            ["--no-default-module-paths", "--no-integrated-modules"],
        ),
    ):
        run(
            name,
            [
                str(snapshot / "pnp_cli.exe"),
                "module",
                "diagnose",
                "--module-dir",
                str(snapshot / "modules"),
                *flags,
            ],
            env=env,
        )
    print("PASS: frozen identity, runner controls and discovery probes; no timing")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"STOP: {error}", file=sys.stderr)
        sys.exit(1)

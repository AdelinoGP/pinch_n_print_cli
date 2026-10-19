"""One runner preflight: controls and ordinary module discovery, never timing."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

EVIDENCE = Path(__file__).resolve().parent / "runner"
ROOT = Path(__file__).resolve().parents[5]
RESULTS = []


def save():
    (EVIDENCE / "results.json").write_text(json.dumps(RESULTS, indent=2) + "\n")


def run(name, command, expected=0, env=None):
    result = subprocess.run(command, cwd=ROOT, capture_output=True, env=env)
    (EVIDENCE / f"{name}.stdout.log").write_bytes(result.stdout)
    (EVIDENCE / f"{name}.stderr.log").write_bytes(result.stderr)
    RESULTS.append({"name": name, "argv": command, "exit_code": result.returncode})
    save()
    if result.returncode != expected:
        raise RuntimeError(f"{name} failed: exit {result.returncode}")
    return result.stdout.decode("utf-8-sig")


def main():
    EVIDENCE.mkdir(exist_ok=False)
    bench = ROOT / "resources/perimeter-acceptance/run_bench.ps1"
    original = ROOT / "docs/specs/perf-vs-orca/evidence/alloc-bench/run_bench.ps1"
    if bench.read_bytes() != original.read_bytes():
        raise RuntimeError("benchmark copy mismatch")
    RESULTS.append({"name": "benchmark-copy", "byte_identical": True})
    save()
    run(
        "status-roundtrip",
        [
            "pwsh",
            "-NoProfile",
            "-File",
            "resources/perimeter-acceptance/test-status-roundtrip.ps1",
        ],
    )
    summary = ROOT / "target/perimeter-acceptance/summary.json"
    if summary.exists():
        shutil.copy2(summary, EVIDENCE / "summary-before-controls.json")
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
    if not (
        dry["status"] == "DROP"
        and dry["automatic_commit"] is False
        and len(dry["cells"]) == 6
        and dry["threads"] == 12
        and dry["samples_per_cell"] == 4
        and dry["warmup_per_cell"] == 1
        and dry["cells"][0]["decision"] == "inconclusive"
        and dry["cells"][1]["decision"] == "DROP"
    ):
        raise RuntimeError("dry-run protocol disagreement")
    shutil.copy2(summary, EVIDENCE / "dry-run.synthetic.json")
    missing = ROOT / "target/perimeter-acceptance/t38-resumed-absent-corpus"
    if missing.exists():
        raise RuntimeError("negative-control corpus unexpectedly exists")
    text = run(
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
            str(missing),
        ],
        expected=1,
    )
    if not text.startswith("missing-artifact: "):
        raise RuntimeError("missing-corpus control did not reject as documented")
    if os.environ.get("SLICER_MODULE_PATH"):
        raise RuntimeError("unexpected module environment")
    (EVIDENCE / "module-environment.json").write_text(
        json.dumps(
            {
                key: value
                for key, value in os.environ.items()
                if key.startswith(("SLICER_", "PNP_"))
            },
            indent=2,
        )
        + "\n"
    )
    manifest = json.loads(
        (
            ROOT
            / "docs/specs/perf-vs-orca/evidence/t41-reference-repreparation/attempt-1/frozen-manifest.json"
        ).read_text()
    )
    snapshot = Path(manifest["snapshot_root"])
    env = dict(os.environ, SLICER_DEBUG_PATHS="1")
    reports = []
    for name, flags in (
        ("ordinary-runner-discovery", []),
        (
            "ordinary-isolated-discovery",
            ["--no-default-module-paths", "--no-integrated-modules"],
        ),
    ):
        report = json.loads(
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
        )
        if not report["pass"] or not report["modules"]:
            raise RuntimeError("empty or invalid module discovery")
        if any(module["provenance"] != "external" for module in report["modules"]):
            raise RuntimeError("unexpected integrated module in ordinary snapshot")
        reports.append(report)
    if reports[0]["modules"] != reports[1]["modules"]:
        raise RuntimeError("runner's default discovery differs from isolated snapshot")
    RESULTS.append(
        {
            "name": "ordinary-discovery-equivalence",
            "passed": True,
            "modules_loaded": reports[0]["modules_loaded"],
        }
    )
    save()
    print("PASS: runner controls and ordinary discovery; no acceptance timing")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"STOP: {error}", file=sys.stderr)
        sys.exit(1)

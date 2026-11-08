"""Audit completed work and check freshness; never build, slice, restore or retry."""

import json
from pathlib import Path
import shutil
import subprocess

import corpus
import manage

HERE, REPO, RAW = corpus.HERE, corpus.REPO, corpus.RAW


def audit():
    identity = json.loads((HERE / "identity.json").read_text())
    before = json.loads((HERE / "before-inventory.json").read_text())
    archive = json.loads((HERE / "archive-audit.json").read_text())
    for table in (
        identity["inputs"],
        identity["sources"],
        before,
        archive["build_inputs"],
    ):
        assert all(corpus.sha(REPO / p) == h for p, h in table.items())
    assert (
        manage.base.inv(HERE.parent / "t53-union-simplify")
        == archive["prior_evidence_inventory"]
    )
    for mode, _ in manage.base.MODES:
        root = manage.base.dist(mode)
        prefix = root.relative_to(REPO).as_posix() + "/"
        assert manage.base.inv(root) == {
            p[len(prefix) :]: h for p, h in before.items() if p.startswith(prefix)
        }
        assert (
            manage.base.inv(RAW / (mode + "-baseline")) == identity["snapshots"][mode]
        )
        assert manage.base.inv(RAW / (mode + "-probe")) == json.loads(
            (HERE / (mode + "-probe-inventory.json")).read_text()
        )
    provenance = json.loads((HERE / "harness-provenance.json").read_text())
    assert (
        manage.base.inv(Path(provenance["clipper_original"]))
        == provenance["clipper_original_inventory"]
    )
    assert not subprocess.check_output(
        ["git", "diff", "--", *identity["sources"]], cwd=REPO
    )


def equal_files(a, b):
    with a.open("rb") as left, b.open("rb") as right:
        while True:
            x, y = left.read(1024 * 1024), right.read(1024 * 1024)
            assert x == y, "actual output byte drift"
            if not x:
                return True


def main():
    audit()
    result = {"streamed_output_comparisons": {}, "final_freshness": {}}
    for mode, _ in manage.base.MODES:
        for job in ("benchy", "base"):
            name = mode + "-" + job
            paths = [
                RAW / (name + "-" + arm + ".gcode") for arm in ("baseline", "probe")
            ]
            for arm, path in zip(("baseline", "probe"), paths):
                saved = json.loads(
                    (HERE / (name + "-" + arm + "-output.json")).read_text()
                )
                assert (
                    corpus.sha(path) == saved["sha256"]
                    and path.stat().st_size == saved["bytes"]
                )
            result["streamed_output_comparisons"][name] = {
                "equal": equal_files(*paths),
                "bytes": paths[0].stat().st_size,
            }
    logs = HERE / "logs"
    logs.mkdir(exist_ok=True)
    for mode, flags in manage.base.MODES:
        command = ["cargo", "xtask", "build-guests", *flags, "--check"]
        path = logs / (mode + "-final-freshness.log")
        with path.open("wb") as stream:
            run = subprocess.run(
                command, cwd=REPO, stdout=stream, stderr=subprocess.STDOUT
            )
        result["final_freshness"][mode] = {
            "command": command,
            "exit_code": run.returncode,
        }
        print(mode + " final freshness: exit " + str(run.returncode), flush=True)
        assert run.returncode == 0, "STOP: unqualified final freshness"
    audit()
    result["restoration_reverified"] = {
        "original_sources_artifact_bytes_and_distribution_file_sets": True,
        "tracked_build_inputs_and_t53_evidence": True,
        "baseline_probe_snapshot_inventories": True,
        "cargo_registry_unchanged": True,
    }
    result["raw_logs"] = {
        p.name: {"sha256": corpus.sha(p), "bytes": p.stat().st_size}
        for p in RAW.glob("*.log")
    }
    for p in RAW.glob("*.log"):
        if any(tag in p.name for tag in ("build", "freshness", "diagnose")):
            shutil.copy2(p, logs / p.name)
    result["sealed_evidence"] = {
        p.relative_to(HERE).as_posix(): corpus.sha(p)
        for p in HERE.rglob("*")
        if p.is_file()
        and "__pycache__" not in p.parts
        and p.name
        not in ("validation.json", "FINDINGS.md", "verifier-negative-controls.json")
    }
    (HERE / "validation.json").write_text(json.dumps(result, indent=2) + "\n")
    print(
        "PASS: actual streamed outputs, restored inventories, independent final freshness"
    )


if __name__ == "__main__":
    main()

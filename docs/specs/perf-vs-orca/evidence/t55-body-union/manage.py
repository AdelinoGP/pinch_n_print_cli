"""Bounded t55 driver; reuse audited t53 mechanics, not its output references."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "t53_manage", HERE.parent / "t53-union-simplify/manage.py"
)
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)
base.HERE = HERE
base.RAW = base.REPO / ".local-artifacts/perimeter-reference-preparation/t55-body-union"
base.SOURCES = ["modules/core-modules/tree-support-planner/src/lib.rs"]


def audit():
    prior = json.loads((HERE.parent / "t53-union-simplify/identity.json").read_text())
    before = json.loads(
        (HERE.parent / "t53-union-simplify/before-inventory.json").read_text()
    )
    assert all(base.sha(base.REPO / p) == h for p, h in prior["inputs"].items())
    assert all(base.sha(base.REPO / p) == h for p, h in prior["sources"].items())
    assert all(base.sha(base.REPO / p) == h for p, h in before.items())
    oldraw = (
        base.REPO
        / ".local-artifacts/perimeter-reference-preparation/t53-union-simplify"
    )
    assert all(
        base.inv(oldraw / f"{mode}-baseline") == prior["snapshots"][mode]
        for mode, _ in base.MODES
    )
    files = subprocess.check_output(["git", "ls-files", "-z"]).decode().split("\0")
    build_inputs = {
        p: base.sha(base.REPO / p)
        for p in files
        if p and (p.endswith((".rs", ".wit", ".toml")) or p.endswith("Cargo.lock"))
    }
    base.write(
        HERE / "archive-audit.json",
        {
            "t53_inputs_sources_artifacts_and_archived_baselines_match": True,
            "build_inputs": build_inputs,
            "t53_identity_sha256": base.sha(
                HERE.parent / "t53-union-simplify/identity.json"
            ),
            "prior_evidence_inventory": base.inv(HERE.parent / "t53-union-simplify"),
        },
    )


def prepare():
    audit()
    base.prepare()


def restore():
    base.restore()
    audit_before = json.loads((HERE / "archive-audit.json").read_text())
    assert all(
        base.sha(base.REPO / p) == h for p, h in audit_before["build_inputs"].items()
    )
    assert (
        base.inv(HERE.parent / "t53-union-simplify")
        == audit_before["prior_evidence_inventory"]
    )
    restored = json.loads((HERE / "restoration.json").read_text())
    inventory = json.loads((HERE / "before-inventory.json").read_text())
    for mode, _ in base.MODES:
        root = base.dist(mode)
        prefix = root.relative_to(base.REPO).as_posix() + "/"
        assert base.inv(root) == {
            k[len(prefix) :]: v for k, v in inventory.items() if k.startswith(prefix)
        }
    restored["build_inputs_unchanged"] = True
    restored["prior_t53_evidence_unchanged"] = True
    restored["distribution_file_sets_match_before"] = True
    base.write(HERE / "restoration.json", restored)


if __name__ == "__main__":
    {
        "prepare": prepare,
        "baseline": lambda: base.capture("baseline"),
        "build-probe": base.build_probe,
        "probe": lambda: base.capture("probe"),
        "restore": restore,
    }[sys.argv[1]]()

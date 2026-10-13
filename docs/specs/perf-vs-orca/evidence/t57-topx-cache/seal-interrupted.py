"""Audit a stopped qualification and restoration; never resume, slice or time."""

import json
from pathlib import Path
import manage as m
import outputs
import verify

assert not (m.HERE / "qualification.json").exists(), "not an interrupted take"
assert not (m.HERE / "campaign.json").exists(), "timing unexpectedly started"
assert not (m.RAW / "measure").exists(), "unexpected measurement captures"
verify.exactness()
outputs.audit_snapshots()
identity = json.loads((m.HERE / "identity.json").read_text())
progress = json.loads((m.HERE / "qualification-progress.json").read_text())
frozen = json.loads((m.HERE / "qualification-inputs.json").read_text())
assert progress["pass"] is False and len(progress["cells"]) == 7
assert all(m.sha(m.REPO / p) == h for p, h in frozen["inputs"].items())
for key, name in [
    ("protocol_sha256", "protocol.json"),
    ("exactness_sha256", "exactness.json"),
    ("identity_sha256", "identity.json"),
    ("candidate_builds_sha256", "candidate-builds.json"),
    ("runner_sha256", "run-cell.ps1"),
    ("output_verifier_sha256", "outputs.py"),
]:
    assert frozen[key] == m.sha(m.HERE / name), "frozen qualification identity changed"
for cell in progress["cells"]:
    root = m.RAW / "proof" / cell["cell"]
    rows = outputs.rows(root)
    assert rows == cell["rows"] and len(rows) == 2
    assert cell["streamed_byte_identical"]
    outputs.equal_files(*(root / (r["run_id"] + ".gcode") for r in rows))
    for key in (
        "sha256",
        "bytes",
        "type_counts",
        "extruding_xy_moves",
        "warnings",
        "completion",
    ):
        assert rows[0]["output"][key] == rows[1]["output"][key]
interrupted = m.RAW / "proof/ordinary-base-arachne-on"
assert interrupted.is_dir() and not (interrupted / "rows.csv").exists()
assert not list(interrupted.glob("*.gcode")), (
    "interrupted output unexpectedly completed"
)
assert all(m.sha(m.REPO / p) == h for p, h in identity["build_inputs"].items())
assert all(m.sha(m.REPO / p) == h for p, h in identity["before_artifacts"].items())
assert m.inv(Path(identity["registry_path"])) == identity["registry_inventory"]
restored = json.loads((m.HERE / "restoration.json").read_text())
assert restored["freshness"] == {"ordinary": 0, "accelerated": 0}
for mode, _ in m.MODES:
    dist = m.REPO / (
        "target/dist/developer"
        if mode == "ordinary"
        else "target/dist-accelerated/developer"
    )
    assert m.inv(dist) == identity["baseline_snapshots"][mode]
blocker = {
    "status": "blocked",
    "qualification_pass": False,
    "timing_started": False,
    "completed_cells": [c["cell"] for c in progress["cells"]],
    "interrupted_cell": "ordinary-base-arachne-on",
    "interrupted_arm": "baseline",
    "cause": "outer shell command exceeded its configured aggregate timeout; remaining qualification not executed",
    "timeout_ms": 2400000,
    "timeout_authority": "shell sh_1168dc419001Z1rvDkA5RePRQT completion notification",
    "no_performance_verdict": True,
    "recommendation": "do not adopt; request human authorization before resuming qualification",
    "automatic_retry": False,
    "restoration": restored,
    "coverage": "seven ordinary-mode qualification pairs passed; eighth baseline interrupted; accelerated qualification and all paired timing absent",
}
m.write(m.HERE / "interruption.json", blocker)
seal = {
    "outcome": "BLOCKED_NO_PERFORMANCE_VERDICT",
    "production_build_inputs_unchanged": True,
    "active_artifact_bytes_and_distribution_file_sets_restored": True,
    "registry_unchanged": True,
    "snapshot_inventories_reverified": True,
    "final_production_freshness": restored["freshness"],
    "raw_manifest": {
        p.relative_to(m.RAW).as_posix(): {"sha256": m.sha(p), "bytes": p.stat().st_size}
        for p in sorted((m.RAW / "proof").rglob("*"))
        if p.is_file()
    },
    "sealed_evidence": {
        p.relative_to(m.HERE).as_posix(): m.sha(p)
        for p in sorted(m.HERE.rglob("*"))
        if p.is_file()
        and "__pycache__" not in p.parts
        and p.name not in ("validation.json", "FINDINGS.md")
    },
}
m.write(m.HERE / "validation.json", seal)
print(
    "PASS: partial output proof revalidated and restored state sealed; full qualification/timing remain BLOCKED"
)

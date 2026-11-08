"""Audit restored state and seal measured evidence; never build, slice or retry."""

import json
from pathlib import Path
import manage as m
import outputs
import verify

identity = json.loads((m.HERE / "identity.json").read_text())
restored = json.loads((m.HERE / "restoration.json").read_text())
assert restored["freshness"] == {"ordinary": 0, "accelerated": 0}
assert all(m.sha(m.REPO / p) == h for p, h in identity["build_inputs"].items())
assert all(m.sha(m.REPO / p) == h for p, h in identity["before_artifacts"].items())
assert m.inv(Path(identity["registry_path"])) == identity["registry_inventory"]
outputs.audit_snapshots()
verify.exactness()
verify.qualification()
verify.campaign.verify()
qualification = json.loads((m.HERE / "qualification.json").read_text())
campaign = json.loads((m.HERE / "campaign.json").read_text())
for p, digest in qualification["inputs"]["inputs"].items():
    assert m.sha(m.REPO / p) == digest
result = {
    "outcome": campaign["recommendation"],
    "production_build_inputs_unchanged": True,
    "active_artifact_bytes_and_distribution_file_sets_restored": True,
    "registry_unchanged": True,
    "frozen_inputs_unchanged": True,
    "snapshot_inventories_reverified": True,
    "final_production_freshness": {},
    "raw_manifest": {},
}
for mode, flags in m.MODES:
    result["final_production_freshness"][mode] = m.run(
        ["cargo", "xtask", "build-guests", *flags, "--check"],
        mode + "-independent-final-production-freshness",
    )
    dist = m.REPO / (
        "target/dist/developer"
        if mode == "ordinary"
        else "target/dist-accelerated/developer"
    )
    assert m.inv(dist) == identity["baseline_snapshots"][mode]
for phase in ("proof", "measure"):
    for p in sorted((m.RAW / phase).rglob("*")):
        if p.is_file():
            result["raw_manifest"][p.relative_to(m.RAW).as_posix()] = {
                "sha256": m.sha(p),
                "bytes": p.stat().st_size,
            }
result["sealed_evidence"] = {
    p.relative_to(m.HERE).as_posix(): m.sha(p)
    for p in sorted(m.HERE.rglob("*"))
    if p.is_file()
    and "__pycache__" not in p.parts
    and p.name not in ("validation.json", "FINDINGS.md")
}
m.write(m.HERE / "validation.json", result)
print(
    "PASS: restored production, independent freshness, raw identities and sealed measured evidence"
)

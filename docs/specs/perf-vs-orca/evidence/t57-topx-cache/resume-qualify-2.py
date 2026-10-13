"""Authorized resumption 2: revalidate 15 cells, rerun cell 16, seal qualification."""

import json
import manage as m
import outputs

identity, candidate = outputs.audit_snapshots()
assert not (m.HERE / "qualification.json").exists(), "qualification already complete"
frozen = json.loads((m.HERE / "qualification-inputs.json").read_text())
for key, name in [
    ("protocol_sha256", "protocol.json"),
    ("exactness_sha256", "exactness.json"),
    ("identity_sha256", "identity.json"),
    ("candidate_builds_sha256", "candidate-builds.json"),
    ("runner_sha256", "run-cell.ps1"),
    ("output_verifier_sha256", "outputs.py"),
]:
    assert frozen[key] == m.sha(m.HERE / name), "frozen identity changed: " + name
auth = json.loads((m.HERE / "resume-authorization-2.json").read_text())
assert auth["runner_unchanged"] is True
progress = json.loads((m.HERE / "qualification-progress.json").read_text())
assert progress["pass"] is False and len(progress["cells"]) == 15

# Re-prove all completed cells from raw bytes before reuse.
cells = []
for entry in progress["cells"]:
    root = m.RAW / "proof" / entry["cell"]
    rows = outputs.rows(root)
    assert rows == entry["rows"], "completed cell drift: " + entry["cell"]
    outputs.equal_files(*(root / (r["run_id"] + ".gcode") for r in rows))
    cells.append(entry)
print("PASS: 15 completed cells revalidated from raw outputs", flush=True)

cell = "accelerated-base-arachne-on"
assert not (m.RAW / "proof" / cell).exists(), "cell 16 not cleared"
mode, fixture, generator, supports = cell.split("-")
m.run(
    [
        "pwsh",
        "-NoProfile",
        "-File",
        m.HERE / "run-cell.ps1",
        "-Mode",
        mode,
        "-Fixture",
        fixture,
        "-Generator",
        generator,
        "-Supports",
        supports,
        "-Phase",
        "proof",
    ],
    cell + "-qualification",
)
root = m.RAW / "proof" / cell
rows = outputs.rows(root)
assert len(rows) == 2 and [r["arm"] for r in rows] == ["baseline", "candidate"]
outputs.equal_files(*(root / (r["run_id"] + ".gcode") for r in rows))
assert rows[0]["output"]["warnings"] == rows[1]["output"]["warnings"], "warning drift"
original = next(c for c in cells if c["cell"] == "ordinary-base-arachne-on")
outputs.equal_files(
    m.RAW / "proof" / original["cell"] / (original["rows"][0]["run_id"] + ".gcode"),
    root / (rows[0]["run_id"] + ".gcode"),
)
assert rows[0]["sha256"] == original["rows"][0]["sha256"], "mode equality"
cells.append({"cell": cell, "streamed_byte_identical": True, "rows": rows})
m.write(m.HERE / "qualification-progress.json", {"pass": False, "cells": cells})
print("PASS full-matrix cell: " + cell, flush=True)

assert len(cells) == 16
outputs.audit_snapshots()
assert all(m.sha(m.REPO / p) == h for p, h in frozen["inputs"].items()), (
    "input drift during qualification"
)
m.write(
    m.HERE / "qualification.json",
    {
        "pass": True,
        "inputs": frozen,
        "cells": cells,
        "reference_authority": "fresh original-engine current-source baseline per cell, mode equality and archived post-repair t55 classic-on identity",
        "resumption": {
            "run1": "stopped by outer command timeout after 7 cells",
            "run2": "completed cells 8-15; harness session eviction mid cell-16 candidate warmup",
            "run3": "cell 16 rerun whole under resume-authorization-2.json; earlier cells revalidated, never re-sliced",
        },
    },
)
print(
    "PASS: entire fresh full matrix, clean nonvacuous output and streamed byte equality; timing now eligible",
    flush=True,
)

"""Authorized resumption: revalidate completed cells, finish matrix, seal the gate."""

import json
import manage as m
import outputs

identity, candidate = outputs.audit_snapshots()
assert not (m.HERE / "qualification.json").exists(), "qualification already complete"
frozen = json.loads((m.HERE / "qualification-inputs.json").read_text())
assert m.sha(m.HERE / "run-cell.ps1") == frozen["runner_sha256"]
assert m.sha(m.HERE / "outputs.py") == frozen["output_verifier_sha256"]
auth = json.loads((m.HERE / "resume-authorization.json").read_text())
assert auth["human_request"].startswith("Continue until completion")
progress = json.loads((m.HERE / "qualification-progress.json").read_text())
assert progress["pass"] is False and len(progress["cells"]) == 7

# The seven completed cells are re-proved from raw bytes before reuse.
cells = []
for entry in progress["cells"]:
    root = m.RAW / "proof" / entry["cell"]
    rows = outputs.rows(root)
    assert rows == entry["rows"], "completed cell drift: " + entry["cell"]
    outputs.equal_files(*(root / (r["run_id"] + ".gcode") for r in rows))
    cells.append(entry)
print("PASS: seven completed cells revalidated from raw outputs", flush=True)

order = [
    f"{mode}-{fixture}-{generator}-{supports}"
    for mode, _ in m.MODES
    for fixture in ("benchy", "base")
    for generator in ("classic", "arachne")
    for supports in ("off", "on")
]
remaining = order[len(cells) :]
assert remaining[0] == "ordinary-base-arachne-on" and len(remaining) == 9
for cell in remaining:
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
    files = [root / (r["run_id"] + ".gcode") for r in rows]
    outputs.equal_files(*files)
    assert rows[0]["output"]["warnings"] == rows[1]["output"]["warnings"], (
        "warning drift"
    )
    if generator == "classic" and supports == "on":
        prior = json.loads(
            (
                m.HERE.parent
                / "t55-body-union"
                / (mode + "-" + fixture + "-baseline-output.json")
            ).read_text()
        )
        assert (
            rows[0]["sha256"] == prior["sha256"] and rows[0]["bytes"] == prior["bytes"]
        ), "post-repair reference drift"
    if mode == "accelerated":
        original = next(
            c
            for c in cells
            if c["cell"] == f"ordinary-{fixture}-{generator}-{supports}"
        )
        outputs.equal_files(
            m.RAW
            / "proof"
            / original["cell"]
            / (original["rows"][0]["run_id"] + ".gcode"),
            files[0],
        )
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
            "authorized_by": "resume-authorization.json",
            "human_request": auth["human_request"],
            "revalidated_completed_cells": 7,
            "rerun_cells": remaining,
        },
    },
)
print(
    "PASS: entire fresh full matrix, clean nonvacuous output and streamed byte equality; timing now eligible",
    flush=True,
)

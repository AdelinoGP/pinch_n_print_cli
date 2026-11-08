"""One-shot full fresh matrix; never time, retry or rebuild."""

import json
import manage as m
import outputs

identity, candidate = outputs.audit_snapshots()
assert not (m.HERE / "qualification.json").exists(), "qualification one-shot guard"
checks = json.loads((m.HERE / "exactness.json").read_text())
assert checks["pass"], "exactness/code gates before qualification"
config_dir = m.HERE.parent / "matched-pair/configs"
inputs = {
    p.relative_to(m.REPO).as_posix(): m.sha(p) for p in config_dir.glob("pnp-*.json")
}
assert len(inputs) == 4
inputs.update(identity["inputs"])
frozen = {
    "inputs": inputs,
    "protocol_sha256": m.sha(m.HERE / "protocol.json"),
    "exactness_sha256": m.sha(m.HERE / "exactness.json"),
    "identity_sha256": m.sha(m.HERE / "identity.json"),
    "candidate_builds_sha256": m.sha(m.HERE / "candidate-builds.json"),
    "runner_sha256": m.sha(m.HERE / "run-cell.ps1"),
    "output_verifier_sha256": m.sha(m.HERE / "outputs.py"),
}
assert not (m.HERE / "qualification-inputs.json").exists()
m.write(m.HERE / "qualification-inputs.json", frozen)
cells = []
for mode, _ in m.MODES:
    for fixture in ("benchy", "base"):
        for generator in ("classic", "arachne"):
            for supports in ("off", "on"):
                cell = f"{mode}-{fixture}-{generator}-{supports}"
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
                assert len(rows) == 2 and [r["arm"] for r in rows] == [
                    "baseline",
                    "candidate",
                ]
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
                        rows[0]["sha256"] == prior["sha256"]
                        and rows[0]["bytes"] == prior["bytes"]
                    ), "post-repair reference drift"
                if mode == "accelerated":
                    original = next(
                        c
                        for c in cells
                        if c["cell"] == f"ordinary-{fixture}-{generator}-{supports}"
                    )
                    original_root = m.RAW / "proof" / original["cell"]
                    outputs.equal_files(
                        original_root / (original["rows"][0]["run_id"] + ".gcode"),
                        files[0],
                    )
                cells.append(
                    {"cell": cell, "streamed_byte_identical": True, "rows": rows}
                )
                m.write(
                    m.HERE / "qualification-progress.json",
                    {"pass": False, "cells": cells},
                )
                print("PASS full-matrix cell: " + cell, flush=True)
assert len(cells) == 16
outputs.audit_snapshots()
assert all(m.sha(m.REPO / p) == h for p, h in inputs.items()), (
    "input drift during qualification"
)
m.write(
    m.HERE / "qualification.json",
    {
        "pass": True,
        "inputs": frozen,
        "cells": cells,
        "reference_authority": "fresh original-engine current-source baseline per cell, mode equality and archived post-repair t55 classic-on identity",
    },
)
print(
    "PASS: entire fresh full matrix, clean nonvacuous output and streamed byte equality; timing now eligible"
)

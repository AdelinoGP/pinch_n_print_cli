"""One approved paired take; resume-safe, deterministic reducer works without raw payloads.

Three-state outcome taxonomy (never conflated):
- KEEP:          all four timed cells pass the paired gate.
- DROP:          a cell ran and failed the gate (stop; no automatic retry).
- INCONCLUSIVE:  a cell could not be evaluated cleanly (e.g. starvation discards);
                 returned to the human, never reported as a measured DROP.
"""

import copy
import json
import math
import shutil
from statistics import median
import sys
import manage as m
import outputs

CELLS = [
    f"{mode}-{fixture}-classic-on"
    for mode, _ in m.MODES
    for fixture in ("benchy", "base")
]


def summarize(rows, proof):
    assert len(rows) == 14, "two excluded warmups plus six complete pairs"
    assert [r["arm"] for r in rows[:2]] == ["baseline", "candidate"]
    assert all(r["warmup"] == 1 and r["pair"] == 0 for r in rows[:2])
    expected_order = [
        arm
        for i in range(1, 7)
        for arm in (["baseline", "candidate"] if i % 2 else ["candidate", "baseline"])
    ]
    assert [r["arm"] for r in rows[2:]] == expected_order, "alternating paired order"
    for i in range(1, 7):
        assert all(r["pair"] == i and r["warmup"] == 0 for r in rows[2 * i : 2 * i + 2])
    for row in rows:
        assert row["sha256"] == proof["sha256"] and row["bytes"] == proof["bytes"], (
            "timed output drift"
        )
        assert row["type_counts"] == proof["type_counts"]
        assert row["output"]["warnings"] == proof["warnings"], "timed warning drift"
        assert row["threads"] == 12 and row["status"] == "ok" and not row["degraded"]
        assert row["exit_code"] == row["fatal"] == row["non_fatal"] == 0
        for key in ("wall_seconds", "cpu_seconds", "cpu_wall_ratio"):
            assert math.isfinite(row[key]) and row[key] > 0
        assert math.isclose(
            row["cpu_wall_ratio"],
            row["cpu_seconds"] / row["wall_seconds"],
            rel_tol=1e-12,
        )
    measured = rows[2:]
    cutoffs = {
        arm: 0.75 * max(r["cpu_wall_ratio"] for r in measured if r["arm"] == arm)
        for arm in ("baseline", "candidate")
    }
    retained, excluded = [], []
    for i in range(1, 7):
        pair = {r["arm"]: r for r in measured if r["pair"] == i}
        assert set(pair) == {"baseline", "candidate"}
        starving = [
            arm for arm, row in pair.items() if row["cpu_wall_ratio"] < cutoffs[arm]
        ]
        if starving:
            excluded.append({"pair": i, "starved_arms": starving})
        else:
            retained.append(pair)
    result = {
        "retained_pairs": len(retained),
        "excluded_pairs": excluded,
        "starvation_cutoffs": cutoffs,
        "gate_passed": False,
    }
    if len(retained) < 5:
        result.update(
            status="inconclusive",
            reason="fewer than five retained pairs; no resampling",
        )
        return result
    metrics = ("wall_seconds", "cpu_seconds")
    medians = {
        arm: {key: median(p[arm][key] for p in retained) for key in metrics}
        for arm in ("baseline", "candidate")
    }
    deltas = {
        key: median(p["candidate"][key] - p["baseline"][key] for p in retained)
        for key in metrics
    }
    passed = all(
        medians["candidate"][key] < medians["baseline"][key] and deltas[key] < 0
        for key in metrics
    )
    result.update(
        {
            "status": "pass" if passed else "fail",
            "gate_passed": passed,
            "medians": medians,
            "paired_median_deltas": deltas,
            "reduction_percent": {
                key: 100 * (1 - medians["candidate"][key] / medians["baseline"][key])
                for key in metrics
            },
        }
    )
    return result


def decide(statuses):
    """Legal completed-campaign shapes only: pass*; or pass* then one stop."""
    values = [statuses[cell] for cell in CELLS]
    if all(v == "pass" for v in values):
        return "KEEP"
    for index, value in enumerate(values):
        if value == "pass":
            continue
        if value in ("fail", "inconclusive"):
            assert all(x == "pass" for x in values[:index]), "pass after stop"
            assert all(x == "not_run" for x in values[index + 1 :]), "run after stop"
            return "DROP" if value == "fail" else "INCONCLUSIVE"
        raise AssertionError("not_run without a recorded stop: " + repr(values))
    raise AssertionError("unreachable")


def validate_cell(cell, qualification, root):
    """Re-derive one cell's saved summary from its raw rows; never trust memory."""
    proof = next(c for c in qualification["cells"] if c["cell"] == cell)
    proof_file = m.RAW / "proof" / cell / (proof["rows"][0]["run_id"] + ".gcode")
    rows = outputs.rows(root)
    for row in rows:
        outputs.equal_files(proof_file, root / (row["run_id"] + ".gcode"))
    summary = summarize(rows, proof["rows"][0]["output"])
    return {"summary": summary, "rows": rows, "status": summary["status"]}


def run():
    assert not (m.HERE / "campaign.json").exists(), "one-shot campaign guard"
    resume_authorized = sys.argv[2:] == ["--resume-authorized"]
    assert not sys.argv[2:] or resume_authorized, "unknown campaign option"
    qualification = json.loads((m.HERE / "qualification.json").read_text())
    assert qualification["pass"] and len(qualification["cells"]) == 16
    frozen = qualification["inputs"]
    assert m.sha(m.HERE / "protocol.json") == frozen["protocol_sha256"]
    assert m.sha(m.HERE / "run-cell.ps1") == frozen["runner_sha256"]
    assert m.sha(m.HERE / "outputs.py") == frozen["output_verifier_sha256"]
    assert all(m.sha(m.REPO / p) == h for p, h in frozen["inputs"].items())
    outputs.audit_snapshots()
    progress_path = m.HERE / "campaign-progress.json"
    progress = (
        json.loads(progress_path.read_text())
        if progress_path.exists()
        else {"cells": {}, "interrupted": []}
    )
    # A stop can only leave an in-flight cell: archive it and rerun that cell alone.
    # Rerunning an archived in-flight cell requires explicit human authorization
    # (--resume-authorized); it is never an automatic retry.
    for cell in CELLS:
        if cell in progress["cells"]:
            continue
        stale = m.RAW / "measure" / cell
        if stale.exists():
            assert resume_authorized, (
                "in-flight measured cell without human resume authorization: " + cell
            )
            archived = m.RAW / "measure-interrupted" / cell
            archived.parent.mkdir(parents=True, exist_ok=True)
            attempt = 1
            candidate = archived
            while candidate.exists():
                attempt += 1
                candidate = archived.with_name(f"{cell}-stop{attempt}")
            shutil.move(str(stale), str(candidate))
            progress["interrupted"].append(
                {"cell": cell, "archived_as": str(candidate.relative_to(m.REPO))}
            )
    stopped = False
    for cell in CELLS:
        if cell in progress["cells"]:
            data = validate_cell(cell, qualification, m.RAW / "measure" / cell)
            assert data == progress["cells"][cell], "measured cell drift: " + cell
            stopped = data["status"] != "pass"
            if stopped:
                break
            continue
        assert not stopped, "no cell may run after a recorded stop"
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
                "measure",
            ],
            cell + "-measurement",
        )
        data = validate_cell(cell, qualification, m.RAW / "measure" / cell)
        progress["cells"][cell] = data
        m.write(progress_path, progress)
        print(json.dumps({"cell": cell, **data["summary"]}, indent=2), flush=True)
        if data["status"] != "pass":
            stopped = True
            break
    statuses = {
        cell: progress["cells"].get(cell, {}).get("status", "not_run") for cell in CELLS
    }
    recommendation = decide(statuses)
    outputs.audit_snapshots()
    assert all(m.sha(m.REPO / p) == h for p, h in frozen["inputs"].items())
    report = {
        "recommendation": recommendation,
        "automatic_retry": False,
        "automatic_commit": False,
        "protocol_sha256": frozen["protocol_sha256"],
        "qualification_sha256": m.sha(m.HERE / "qualification.json"),
        "campaign_reducer_sha256": m.sha(m.HERE / "campaign.py"),
        "interrupted": progress["interrupted"],
        "cells": {
            cell: progress["cells"].get(cell, {"status": "not_run"}) for cell in CELLS
        },
    }
    m.write(m.HERE / "campaign.json", report)
    print(
        "Bounded recommendation: "
        + recommendation
        + "; no automatic retry/adoption/commit"
    )


def verify():
    report = json.loads((m.HERE / "campaign.json").read_text())
    qualification = json.loads((m.HERE / "qualification.json").read_text())
    assert report["qualification_sha256"] == m.sha(m.HERE / "qualification.json")
    assert report["protocol_sha256"] == m.sha(m.HERE / "protocol.json")
    assert report["campaign_reducer_sha256"] == m.sha(m.HERE / "campaign.py"), (
        "reducer changed after the campaign was reduced"
    )
    statuses = {}
    for cell in CELLS:
        data = report["cells"][cell]
        statuses[cell] = data["status"]
        if data["status"] == "not_run":
            assert data == {"status": "not_run"}, "not_run cell carries data"
            continue
        proof = next(c for c in qualification["cells"] if c["cell"] == cell)["rows"][0][
            "output"
        ]
        summary = summarize(data["rows"], proof)
        assert summary == data["summary"] and data["status"] == summary["status"]
        bad = copy.deepcopy(data["rows"])
        bad[2]["sha256"] = "0" * 64
        try:
            summarize(bad, proof)
        except AssertionError:
            pass
        else:
            raise AssertionError("timed output sabotage accepted")
    assert report["recommendation"] == decide(statuses)
    assert report["automatic_retry"] is False and report["automatic_commit"] is False
    print(
        "PASS: checkout-only paired reduction, output sabotage rejection and stop discipline"
    )


if __name__ == "__main__":
    {"run": run, "verify": verify}[sys.argv[1]]()

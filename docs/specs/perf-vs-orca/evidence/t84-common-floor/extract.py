#!/usr/bin/env python3
"""Reduce the t84 raw capture tree into committed per-run capture JSON.

The raw stderr streams live under the gitignored
``target/t84-common-floor/<batch>/`` tree (see ``run_capture.ps1``).  This
script keeps only the terms the evidence needs, so ``verify.py`` can run from a
plain checkout without the raw tree.

    python extract.py --raw target/t84-common-floor

Batches:

* ``t84-head``    — HEAD, profiling off. Cell wall + stage walls + output SHA.
* ``t84-probe2``  — probe build, profiling on. Term split on both cells.
* ``t84-probe3``  — probe build with the operand-window counter (one cell).
"""

import argparse
import csv
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent

PROBE_RE = re.compile(r"^\[t84\] stage=(\w+) (.*)$")


def _kv(text):
    out = {}
    for token in text.split():
        key, _, value = token.partition("=")
        if not key:
            continue
        try:
            out[key] = int(value)
        except ValueError:
            try:
                out[key] = float(value)
            except ValueError:
                out[key] = value
    return out


def _read_rows(batch_dir):
    rows = {}
    path = batch_dir / "rows.csv"
    if not path.exists():
        return rows
    with path.open(encoding="utf-8-sig", newline="") as handle:
        for row in csv.DictReader(handle):
            rows[row["run_id"]] = row
    return rows


def _events(path):
    if not path.exists():
        return
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            try:
                yield json.loads(line)
            except json.JSONDecodeError:
                continue


def _stage_walls(path):
    walls = {}
    total = None
    for event in _events(path):
        kind = event.get("event")
        if kind == "stage_complete":
            walls[event.get("stage")] = walls.get(event.get("stage"), 0) + event.get(
                "elapsed_ms", 0
            )
        elif kind == "slice_complete":
            total = event.get("elapsed_ms")
    return walls, total


def _probe_terms(path):
    """Probe lines are plain ``[t84]`` eprintln lines interleaved with the JSON
    instrumentation stream, so they are matched on the raw text."""
    terms = {}
    if not path.exists():
        return terms
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            match = PROBE_RE.match(line.strip())
            if not match:
                continue
            stage, rest = match.groups()
            terms[stage] = _kv(rest)
    return terms


def _profile(path):
    for event in _events(path):
        if event.get("event") != "profile_summary":
            continue
        modules = {m["module_id"]: m for m in event["profile"]["modules"]}
        seam = modules.get("com.core.seam-planner-default", {})
        overhang = modules.get("host:overhang_annotation", {})
        return {
            "seam": {
                "module_wall_ms": round(seam.get("total_wall_ns", 0) / 1e6, 3),
                "module_fuel": seam.get("total_fuel", 0),
                "scopes": [
                    {
                        "scope": s["scope"],
                        "fuel": s["total_fuel"],
                        "calls": s["calls"],
                        "wall_ms": round(s["total_wall_ns"] / 1e6, 3),
                    }
                    for s in seam.get("scopes", [])
                ],
            },
            "overhang_host": {
                "wall_ms": round(overhang.get("total_wall_ns", 0) / 1e6, 3),
                "scopes": [
                    {
                        "scope": s["scope"],
                        "calls": s["calls"],
                        "wall_ms": round(s["total_wall_ns"] / 1e6, 3),
                    }
                    for s in overhang.get("scopes", [])
                ],
            },
        }
    return {}


def capture(batch_dir, run_id, cell):
    rows = _read_rows(batch_dir)
    row = rows[run_id]
    stderr = batch_dir / "stderr" / f"{run_id}.jsonl"
    stages, slice_ms = _stage_walls(stderr)
    terms = _probe_terms(stderr)
    payload = {
        "cell": cell,
        "run_id": run_id,
        "label": row["label"],
        "probe": row["probe"].lower() == "true",
        "profile": row["profile"].lower() == "true",
        "wall_seconds": float(row["wall_seconds"]),
        "cpu_seconds": float(row["cpu_seconds"]),
        "slice_complete_ms": slice_ms,
        "bytes": int(row["bytes"]),
        "sha256": row["sha256"],
        "status": row["status"],
        "degraded": row["degraded"].lower() == "true",
        "fatal": int(row["fatal"]),
        "non_fatal": int(row["non_fatal"]),
        "stages_ms": {
            "PrePass::OverhangAnnotation": stages.get("PrePass::OverhangAnnotation"),
            "PrePass::SeamPlanning": stages.get("PrePass::SeamPlanning"),
        },
        "terms": {
            "overhang": terms.get("overhang", {}),
            "seam_project": terms.get("seam_project", {}),
            "seam_call": terms.get("seam_call", {}),
            "seam_harvest": terms.get("seam_harvest", {}),
            "overhang_window": terms.get("overhang_window", {}),
        },
        "profile": _profile(stderr),
    }
    return payload


PLAN = [
    ("t84-head", "base-classic-off-head", "base-classic-off"),
    ("t84-head", "base-classic-on-head", "base-classic-on"),
    ("t84-probe2", "base-classic-off-probe", "base-classic-off"),
    ("t84-probe2", "base-classic-on-probe", "base-classic-on"),
    ("t84-probe3", "base-classic-off-probe3", "base-classic-off"),
    ("t84-restore", "base-classic-off-head2", "base-classic-off"),
]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw", required=True, help="raw capture root")
    args = parser.parse_args()

    raw = Path(args.raw)
    if not raw.is_absolute():
        raw = (HERE / ".." / ".." / ".." / ".." / ".." / raw).resolve()
    out_dir = HERE / "captures"
    out_dir.mkdir(exist_ok=True)

    for batch, run_id, cell in PLAN:
        payload = capture(raw / batch, run_id, cell)
        path = out_dir / f"{run_id}.json"
        path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {path.name}")


if __name__ == "__main__":
    main()

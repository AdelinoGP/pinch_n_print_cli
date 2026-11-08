#!/usr/bin/env python3
"""t67 qualification checker.

Reads every proof cell's rows.csv under
.local-artifacts/perimeter-reference-preparation/t67-region-prep-sharing/proof/,
requires per cell: exactly 2 rows (baseline + candidate), byte-identical
outputs (sha256 equality), clean status (ok / not degraded / zero fatal /
zero non-fatal), non-vacuous role counts. Writes qualification.json with
pass=false on the first violated cell; exit code mirrors pass.
"""

from __future__ import annotations

import csv
import json
import sys
from pathlib import Path


def _repo_root() -> Path:
    p = Path(__file__).resolve().parent  # .../evidence/t67-region-prep-sharing
    for ancestor in [p, *p.parents]:
        if (ancestor / ".local-artifacts").is_dir():
            return ancestor
    raise SystemExit("could not locate repo root (.local-artifacts)")


ROOT = (
    _repo_root()
    / ".local-artifacts"
    / "perimeter-reference-preparation"
    / "t67-region-prep-sharing"
)

CELLS = [
    f"{mode}-{fixture}-{gen}-{sup}"
    for mode in ("ordinary", "accelerated")
    for fixture in ("benchy", "base")
    for gen in ("classic", "arachne")
    for sup in ("off", "on")
]


def main() -> int:
    cells = {}
    problems = []
    for cell in CELLS:
        rows_path = ROOT / "proof" / cell / "rows.csv"
        if not rows_path.exists():
            problems.append(f"{cell}: proof rows absent")
            continue
        rows = list(csv.DictReader(rows_path.open(encoding="utf-8-sig", newline="")))
        if len(rows) != 2:
            problems.append(f"{cell}: expected 2 proof rows, found {len(rows)}")
            continue
        arms = {r["arm"] for r in rows}
        if arms != {"baseline", "candidate"}:
            problems.append(f"{cell}: arms {sorted(arms)} != baseline/candidate")
            continue
        if rows[0]["sha256"] != rows[1]["sha256"]:
            problems.append(
                f"{cell}: output drift baseline {rows[0]['sha256'][:8]} vs candidate {rows[1]['sha256'][:8]}"
            )
            continue
        for r in rows:
            if r["status"] != "ok" or r["degraded"] != "False":
                problems.append(
                    f"{cell}: unclean {r['arm']} status={r['status']} degraded={r['degraded']}"
                )
            if int(r["fatal"]) != 0 or int(r["non_fatal"]) != 0:
                problems.append(f"{cell}: fatal/non-fatal not zero on {r['arm']}")
        types = json.loads(rows[0]["type_counts"])
        for role in ("Outer wall", "Sparse infill", "Inner wall"):
            if role not in types:
                problems.append(f"{cell}: vacuous output (missing {role})")
        if "on" in cell.split("-")[-1] and (
            "Support" not in types or "Support interface" not in types
        ):
            problems.append(f"{cell}: vacuous support roles")
        cells[cell] = {
            "sha256": rows[0]["sha256"],
            "bytes": int(rows[0]["bytes"]),
            "wall_seconds": {r["arm"]: float(r["wall_seconds"]) for r in rows},
            "cpu_seconds": {r["arm"]: float(r["cpu_seconds"]) for r in rows},
            "type_counts": types,
            "status": rows[0]["status"],
            "degraded": rows[0]["degraded"],
        }
    passed = not problems and len(cells) == 16
    cells_array = [{"cell": name, **record} for name, record in cells.items()]
    out = {"pass": passed, "cells": cells_array, "problems": problems}
    (Path(__file__).parent / "qualification.json").write_text(
        json.dumps(out, indent=1), encoding="utf-8"
    )
    print(f"qualified cells: {len(cells)}/16; pass={passed}")
    for p in problems:
        print("PROBLEM:", p)
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())

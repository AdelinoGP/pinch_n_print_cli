#!/usr/bin/env python3
"""t102: raw capture rows -> the committed reduced rows.

Reads this machine's `.local-artifacts` captures and writes `proof-rows.json`,
`pregate-rows.json` and `measure-rows.json` next to this script, with the
per-run warning map reduced to counts (the raw map is ~100 KB per row and only
the counts are gated). `reduce.py` and `verify.py` then work from the committed
rows alone, so a plain checkout reproduces the take.

Usage:
    python docs/specs/perf-vs-orca/evidence/t102-allocator/extract.py
"""

from __future__ import annotations

import csv
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
ROOT = REPO / ".local-artifacts" / "perimeter-reference-preparation" / "t102-allocator"

MODES = ("ordinary", "accelerated")
FIXTURES = ("benchy", "base")
GENERATORS = ("classic", "arachne")
SUPPORTS = ("off", "on")
PROOF_CELLS = [
    f"{mode}-{fixture}-{generator}-{supports}"
    for mode in MODES
    for fixture in FIXTURES
    for generator in GENERATORS
    for supports in SUPPORTS
]
MEASURE_CELLS = [
    f"{mode}-base-{generator}-on" for mode in MODES for generator in GENERATORS
]
PREGATE_CELLS = ["ordinary-base-classic-on"]

ROW_FIELDS = (
    "run_id",
    "phase",
    "mode",
    "fixture",
    "generator",
    "supports",
    "arm",
    "pair",
    "warmup",
    "threads",
    "exit_code",
    "exe_sha256",
    "wall_seconds",
    "cpu_seconds",
    "cpu_wall_ratio",
    "bytes",
    "sha256",
    "type_counts",
    "warnings",
    "status",
    "degraded",
    "fatal",
    "non_fatal",
)

INT_FIELDS = ("pair", "warmup", "threads", "exit_code", "bytes", "fatal", "non_fatal")
FLOAT_FIELDS = ("wall_seconds", "cpu_seconds", "cpu_wall_ratio")


def normalise_row(raw: dict) -> dict:
    row = {key: raw[key] for key in ROW_FIELDS if key in raw}
    for key in INT_FIELDS:
        row[key] = int(row[key])
    for key in FLOAT_FIELDS:
        row[key] = float(row[key])
    for key in ("type_counts", "warnings"):
        value = row.get(key)
        if isinstance(value, str):
            row[key] = json.loads(value) if value else {}
        elif value is None:
            row[key] = {}
    if "degraded" in row:
        row["degraded"] = str(row["degraded"]).strip().lower() in ("true", "1")
    warnings = row.get("warnings") or {}
    if isinstance(warnings, dict) and set(warnings) != {"lines", "kinds"}:
        row["warnings"] = {
            "lines": sum(int(count) for count in warnings.values()),
            "kinds": len(warnings),
        }
    return row


def read_cell(phase: str, cell: str) -> list[dict]:
    path = ROOT / phase / cell / "rows.csv"
    if not path.exists():
        return []
    with path.open(newline="", encoding="utf-8-sig") as handle:
        return [normalise_row(raw) for raw in csv.DictReader(handle)]


def main() -> int:
    for phase, cells, name in (
        ("proof", PROOF_CELLS, "proof-rows.json"),
        ("pregate", PREGATE_CELLS, "pregate-rows.json"),
        ("measure", MEASURE_CELLS, "measure-rows.json"),
    ):
        data = {cell: read_cell(phase, cell) for cell in cells}
        missing = [cell for cell, rows in data.items() if not rows]
        if missing:
            print(f"warning: {phase} cells with no rows: {missing}")
        out = HERE / name
        out.write_text(json.dumps(data, indent=1) + "\n")
        print(f"wrote {out.name}: {sum(len(rows) for rows in data.values())} rows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

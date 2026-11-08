#!/usr/bin/env python3
"""Reduce the t80 raw campaign CSVs into the committed evidence set.

Writes, beside this script:

- `rows.csv`    — the curated measured + warmup rows from the campaign batches.
- `summary.json`— per-cell per-arm medians and paired deltas.

Usage:
  python reduce.py target/t80-resolution/classic-20261022-benchy/rows.csv \
                   target/t80-resolution/classic-20261022-base/rows.csv
"""

from __future__ import annotations

import csv
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

FIELDS = [
    "run_id",
    "batch_id",
    "cell",
    "fixture",
    "generator",
    "supports",
    "arm",
    "run_index",
    "warmup",
    "wall_seconds",
    "cpu_seconds",
    "cpu_wall_ratio",
    "peak_workingset_bytes",
    "gcode_bytes",
    "sha256_of_gcode",
    "feature_counts_json",
    "resolution_effective",
    "config_path",
    "output_path",
    "exit_code",
    "timestamp_utc",
]

EXPECTED_RESOLUTION = {"default": "0.01", "floor": "0.001"}


def read_rows(paths: list[Path]) -> list[dict]:
    rows: list[dict] = []
    for path in paths:
        with path.open(newline="", encoding="utf-8-sig") as handle:
            for row in csv.DictReader(handle):
                if not row.get("run_id"):
                    continue
                rows.append({k: row[k] for k in FIELDS})
    return rows


def median(values: list[float]) -> float | None:
    return statistics.median(values) if values else None


def pct(new: float, old: float) -> float:
    return (new - old) / old * 100.0


def build_summary(rows: list[dict]) -> dict:
    cells: list[dict] = []
    keys = sorted({(r["fixture"], r["cell"]) for r in rows})
    for fixture, cell in keys:
        measured = [
            r
            for r in rows
            if r["fixture"] == fixture and r["cell"] == cell and r["warmup"] == "0"
        ]
        arms: dict[str, dict] = {}
        for arm in ("default", "floor"):
            subset = [r for r in measured if r["arm"] == arm]
            subset.sort(key=lambda r: int(r["run_index"]))
            arms[arm] = {
                "n": len(subset),
                "wall_median_s": median([float(r["wall_seconds"]) for r in subset]),
                "cpu_median_s": median([float(r["cpu_seconds"]) for r in subset]),
                "bytes_median": median([float(r["gcode_bytes"]) for r in subset]),
                "wall_samples": [float(r["wall_seconds"]) for r in subset],
                "cpu_samples": [float(r["cpu_seconds"]) for r in subset],
                "bytes_samples": [int(r["gcode_bytes"]) for r in subset],
            }
        paired = []
        by_index = {
            arm: {int(r["run_index"]): r for r in measured if r["arm"] == arm}
            for arm in ("default", "floor")
        }
        for index in sorted(set(by_index["default"]) & set(by_index["floor"])):
            d = by_index["default"][index]
            f = by_index["floor"][index]
            paired.append(
                {
                    "run_index": index,
                    "wall_delta_pct": pct(
                        float(f["wall_seconds"]), float(d["wall_seconds"])
                    ),
                    "cpu_delta_pct": pct(
                        float(f["cpu_seconds"]), float(d["cpu_seconds"])
                    ),
                }
            )
        cells.append(
            {
                "fixture": fixture,
                "cell": cell,
                "arms": arms,
                "paired": paired,
                "median_wall_delta_pct": median([p["wall_delta_pct"] for p in paired]),
                "median_cpu_delta_pct": median([p["cpu_delta_pct"] for p in paired]),
                "byte_delta_pct": pct(
                    arms["floor"]["bytes_median"], arms["default"]["bytes_median"]
                ),
            }
        )
    return {
        "take": "t80-resolution-fairness",
        "note": (
            "'floor' is OrcaSlicer's canonical minimum resolution (0.001 mm); "
            "the matched job inherits 0.01. Deltas are (floor - default) / default, "
            "so a positive delta is the wall Orca gains from its slicing-time "
            "contour simplification. PnP implements no equivalent pass."
        ),
        "arms": {
            "default": {"resolution_mm": "0.01"},
            "floor": {"resolution_mm": "0.001"},
        },
        "cells": cells,
    }


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    paths = [Path(a) for a in argv[1:]]
    rows = read_rows(paths)
    if not rows:
        print("no rows read")
        return 1
    out = HERE / "rows.csv"
    with out.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(rows)
    summary = build_summary(rows)
    (HERE / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    print(
        f"wrote {out} ({len(rows)} rows) and summary.json ({len(summary['cells'])} cells)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

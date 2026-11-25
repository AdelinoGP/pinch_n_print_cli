#!/usr/bin/env python3
"""Ticket 139 scoreboard reducer: 8 cells x 3 tools, 3 measured runs each.

Reads `target/matched-pair/t139-scoreboard/scoreboard-rows.csv` (gitignored)
and writes `reduced/scoreboard.json`. The verifier reads only the reduced file.

Run from the repository root:

    python docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/reduce_scoreboard.py
"""

from __future__ import annotations

import csv
import json
import statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
ROWS = ROOT / "target/matched-pair/t139-scoreboard/scoreboard-rows.csv"
OUT = Path(__file__).resolve().parent / "reduced" / "scoreboard.json"

CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
    "base-classic-off",
    "base-classic-on",
    "base-arachne-off",
    "base-arachne-on",
]
TOOLS = ["orca", "pnp-ordinary", "pnp-accelerated"]

# t135 Stage-1 ratios of record (ordinary / accelerated), superseded by this
# ticket's re-derivation.
T135_RATIOS = {
    "benchy-classic-off": (2.445, 2.420),
    "benchy-classic-on": (2.8, 2.78),
    "benchy-arachne-off": (2.57, 2.55),
    "benchy-arachne-on": (2.99, 2.97),
    "base-classic-off": (3.24, 3.22),
    "base-classic-on": (3.77, 3.75),
    "base-arachne-off": (3.30, 3.28),
    "base-arachne-on": (3.77, 3.75),
}


def main() -> int:
    rows = list(csv.DictReader(ROWS.open(encoding="utf-8")))
    for row in rows:
        row["cell_key"] = f"{row['fixture']}-{row['generator']}-{row['supports']}"
    cells: dict = {}
    for cell in CELLS:
        cells[cell] = {}
        for tool in TOOLS:
            runs = [
                r
                for r in rows
                if r["cell_key"] == cell and r["tool"] == tool and r["warmup"] == "0"
            ]
            if len(runs) != 3:
                raise SystemExit(
                    f"{cell}/{tool}: expected 3 measured runs, got {len(runs)}"
                )
            if any(r["exit_code"] != "0" for r in runs):
                raise SystemExit(f"{cell}/{tool}: a measured run did not exit 0")
            if any(r["completion_status"] not in ("ok", "n/a") for r in runs):
                raise SystemExit(f"{cell}/{tool}: completion status not ok")
            wall = [float(r["wall_seconds"]) for r in runs]
            cpu = [float(r["cpu_seconds"]) for r in runs]
            cells[cell][tool] = {
                "runs": 3,
                "wall_median_s": round(statistics.median(wall), 4),
                "wall_runs_s": [round(v, 4) for v in wall],
                "cpu_median_s": round(statistics.median(cpu), 4),
                "cpu_runs_s": [round(v, 4) for v in cpu],
                "gcode_sha256": runs[0]["sha256_of_gcode"],
            }
        orca = cells[cell]["orca"]["wall_median_s"]
        for tool in ("pnp-ordinary", "pnp-accelerated"):
            ratio = cells[cell][tool]["wall_median_s"] / orca
            cells[cell][tool]["ratio_vs_orca"] = round(ratio, 4)
            cells[cell][tool]["required_cut_pct"] = round(
                (1.0 - 1.0 / ratio) * 100.0, 2
            )

    summary = {
        "cells": cells,
        "ratios": {
            cell: {
                "ordinary": cells[cell]["pnp-ordinary"]["ratio_vs_orca"],
                "accelerated": cells[cell]["pnp-accelerated"]["ratio_vs_orca"],
                "t135_ordinary": T135_RATIOS[cell][0],
                "t135_accelerated": T135_RATIOS[cell][1],
            }
            for cell in CELLS
        },
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUT}")
    for cell in CELLS:
        rec = summary["ratios"][cell]
        print(
            f"{cell}: orca {cells[cell]['orca']['wall_median_s']:.2f}s | "
            f"pnp {cells[cell]['pnp-ordinary']['wall_median_s']:.2f}s ordinary "
            f"({rec['ordinary']:.3f}x), {cells[cell]['pnp-accelerated']['wall_median_s']:.2f}s "
            f"accelerated ({rec['accelerated']:.3f}x) | t135 {rec['t135_ordinary']:.2f}x"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

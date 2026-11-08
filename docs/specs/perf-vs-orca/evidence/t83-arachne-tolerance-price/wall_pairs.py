#!/usr/bin/env python3
"""Reduce the uninstrumented wall-pair A/B for ticket 83.

    python wall_pairs.py --rows target/t83-price/wall-1/rows.csv
    python wall_pairs.py            # re-derive from the committed wall-pairs.json

Writes/reads `wall-pairs.json`: paired deltas per cell, wall and CPU clauses,
and the per-arm output SHA (determinism check). The pre-declared pricing gate
is decided on the instrumented evidence; this file is the direct wall check.
"""

import argparse
import csv
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
CPU_CEILING_PCT = 3.0


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=1) + "\n", encoding="utf-8")


def reduce_rows(path):
    rows = list(csv.DictReader(path.open(encoding="utf-8-sig")))
    out = []
    for row in rows:
        out.append(
            {
                "run_id": row["run_id"],
                "fixture": row["fixture"],
                "cell": row["cell"],
                "arm": row["arm"],
                "pair": int(row["pair"]),
                "order_index": int(row["order_index"]),
                "wall_seconds": float(row["wall_seconds"]),
                "cpu_seconds": float(row["cpu_seconds"]),
                "cpu_wall_ratio": float(row["cpu_wall_ratio"]),
                "bytes": int(row["bytes"]),
                "sha256": row["sha256"],
                "status": row["status"],
                "degraded": row["degraded"] == "True",
                "fatal": int(row["fatal"]),
                "non_fatal": int(row["non_fatal"]),
            }
        )
    return out


def summarize(rows):
    cells = {}
    by_cell = {}
    for row in rows:
        by_cell.setdefault(f"{row['fixture']}-{row['cell']}", []).append(row)
    for cell, samples in sorted(by_cell.items()):
        pairs = sorted({r["pair"] for r in samples})
        wall_deltas, cpu_deltas = [], []
        for pair in pairs:
            default = [
                r for r in samples if r["pair"] == pair and r["arm"] == "default"
            ]
            canonical = [
                r for r in samples if r["pair"] == pair and r["arm"] == "canonical"
            ]
            if len(default) != 1 or len(canonical) != 1:
                raise SystemExit(f"{cell} pair {pair}: expected one run per arm")
            d, c = default[0], canonical[0]
            wall_deltas.append(
                100 * (c["wall_seconds"] - d["wall_seconds"]) / d["wall_seconds"]
            )
            cpu_deltas.append(
                100 * (c["cpu_seconds"] - d["cpu_seconds"]) / d["cpu_seconds"]
            )
        shas = {}
        for arm in ("default", "canonical"):
            arm_shas = {r["sha256"] for r in samples if r["arm"] == arm}
            if len(arm_shas) != 1:
                raise SystemExit(
                    f"{cell}/{arm}: output not deterministic ({len(arm_shas)} shas)"
                )
            shas[arm] = arm_shas.pop()
        if shas["default"] == shas["canonical"]:
            raise SystemExit(f"{cell}: arms produced identical G-code (vacuous)")
        cells[cell] = {
            "pairs": len(pairs),
            "default_wall_median_s": statistics.median(
                [r["wall_seconds"] for r in samples if r["arm"] == "default"]
            ),
            "canonical_wall_median_s": statistics.median(
                [r["wall_seconds"] for r in samples if r["arm"] == "canonical"]
            ),
            "default_cpu_median_s": statistics.median(
                [r["cpu_seconds"] for r in samples if r["arm"] == "default"]
            ),
            "canonical_cpu_median_s": statistics.median(
                [r["cpu_seconds"] for r in samples if r["arm"] == "canonical"]
            ),
            "wall_deltas_pct": wall_deltas,
            "cpu_deltas_pct": cpu_deltas,
            "wall_delta_pct_median": statistics.median(wall_deltas),
            "cpu_delta_pct_median": statistics.median(cpu_deltas),
            "wall_clause": statistics.median(wall_deltas) < 0,
            "cpu_clause": statistics.median(cpu_deltas) <= CPU_CEILING_PCT,
            "output_sha256": shas,
            "output_bytes": {
                arm: {r["bytes"] for r in samples if r["arm"] == arm}.pop()
                for arm in ("default", "canonical")
            },
            "clean": all(
                r["status"] == "ok" and not r["degraded"] and r["fatal"] == 0
                for r in samples
            ),
            "non_fatal_total": sum(r["non_fatal"] for r in samples),
        }
    return {
        "protocol": {
            "cpu_ceiling_pct": CPU_CEILING_PCT,
            "alternating_pairs": True,
            "instrumentation": "none",
        },
        "cells": cells,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rows", type=Path)
    args = parser.parse_args()
    if args.rows:
        rows = reduce_rows(args.rows)
        write(HERE / "wall-pairs.json", {"rows": rows, **summarize(rows)})
    data = json.loads((HERE / "wall-pairs.json").read_text(encoding="utf-8-sig"))
    for cell, summary in data["cells"].items():
        print(
            f"{cell}: {summary['pairs']} pairs, wall median "
            f"{summary['wall_delta_pct_median']:+.2f}% (clause {summary['wall_clause']}), "
            f"CPU median {summary['cpu_delta_pct_median']:+.2f}% "
            f"(clause {summary['cpu_clause']}), clean={summary['clean']}"
        )


if __name__ == "__main__":
    main()

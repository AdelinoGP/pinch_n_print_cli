#!/usr/bin/env python3
"""Restate the scoreboard as a fairness counterfactual (perf-vs-orca ticket 81).

The map's fairness contract (disclosed 2026-10-22 by ticket 80) says the matched
job is **not** resolution-matched: the Orca arm inherits `resolution = 0.01` and
spends the slicing-time contour simplification PnP never implemented, while PnP
generates walls from full-detail contours. Ticket 80 measured what that lever is
worth on Orca's side:

    delta_cell = (wall at 0.001 mm - wall at 0.01 mm) / wall at 0.01 mm

so an Orca doing PnP's geometric work would cost `wall * (1 + delta)`. The
counterfactual ratio is therefore

    corrected = raw_ratio / (1 + delta)

This is a fairness counterfactual, **not** a scoreboard: the raw table stays the
like-for-like measurement of both tools as configured. Nothing here is a new
measurement — every input is a committed number.

Usage:
    python docs/specs/perf-vs-orca/evidence/t81-resolution-port/corrected_scoreboard.py
"""

from __future__ import annotations

import csv
import json
import statistics
from pathlib import Path

EVIDENCE = Path(__file__).resolve().parents[1]
SCOREBOARD = EVIDENCE / "scoreboard-refresh-20261017"
T80 = EVIDENCE / "t80-resolution-fairness" / "summary.json"

CELLS = [
    ("benchy", "classic", "off"),
    ("benchy", "arachne", "off"),
    ("benchy", "classic", "on"),
    ("benchy", "arachne", "on"),
    ("base", "classic", "off"),
    ("base", "arachne", "off"),
    ("base", "classic", "on"),
    ("base", "arachne", "on"),
]

FILES = {
    "benchy": ["rows-benchy.csv"],
    "base": [
        "rows-base-classic-off.csv",
        "rows-base-arachne-off.csv",
        "rows-base-classic-on-ord.csv",
        "rows-base-classic-on-orca.csv",
        "rows-base-arachne-on-ord.csv",
        "rows-base-classic-on-acc.csv",
        "rows-base-arachne-on-acc.csv",
        "rows-base-arachne-on-orca.csv",
    ],
}


def medians() -> dict[tuple[str, str, str], dict[str, float]]:
    """Median uninstrumented wall per (fixture, generator, supports, tool)."""
    samples: dict[tuple[str, str, str, str], list[float]] = {}
    for fixture, names in FILES.items():
        for name in names:
            path = SCOREBOARD / name
            if not path.exists():
                raise SystemExit(f"missing committed rows: {path}")
            with path.open(newline="", encoding="utf-8") as handle:
                for row in csv.DictReader(handle):
                    if row.get("warmup") in ("1", "true", "True"):
                        continue
                    tool = row["tool"]
                    key = (row["fixture"], row["generator"], row["supports"], tool)
                    samples.setdefault(key, []).append(float(row["wall_seconds"]))
    out = {}
    for (fixture, generator, supports, tool), values in samples.items():
        out.setdefault((fixture, generator, supports), {})[tool] = statistics.median(
            values
        )
    return out


def main() -> int:
    walls = medians()
    t80 = json.loads(T80.read_text(encoding="utf-8"))
    delta = {
        (cell["fixture"], cell["cell"].split("-")[0], cell["cell"].split("-")[1]): cell[
            "median_wall_delta_pct"
        ]
        / 100.0
        for cell in t80["cells"]
    }

    rows = []
    print(
        f"{'cell':32s} {'Orca':>8s} {'PnP ord':>8s} {'raw':>6s} {'dW':>7s} {'corr':>6s} "
        f"{'PnP acc':>8s} {'raw':>6s} {'corr':>6s}"
    )
    for fixture, generator, supports in CELLS:
        cell = walls[(fixture, generator, supports)]
        d = delta[(fixture, generator, supports)]
        orca = cell["orca"]
        entry = {
            "fixture": fixture,
            "cell": f"{generator}-{supports}",
            "orca_wall_s": orca,
            "orca_delta_pct": round(d * 100, 2),
            "pnp": {},
        }
        for tool in ("pnp-ordinary", "pnp-accelerated"):
            wall = cell[tool]
            raw = wall / orca
            corrected = raw / (1.0 + d)
            entry["pnp"][tool] = {
                "wall_s": wall,
                "raw_ratio": round(raw, 2),
                "corrected_ratio": round(corrected, 2),
            }
        rows.append(entry)
        ord_ = entry["pnp"]["pnp-ordinary"]
        acc_ = entry["pnp"]["pnp-accelerated"]
        print(
            f"{fixture + ' ' + generator + '-' + supports:32s} {orca:8.2f} "
            f"{ord_['wall_s']:8.2f} {ord_['raw_ratio']:6.2f} {d * 100:+6.1f}% "
            f"{ord_['corrected_ratio']:6.2f} {acc_['wall_s']:8.2f} {acc_['raw_ratio']:6.2f} "
            f"{acc_['corrected_ratio']:6.2f}"
        )

    best_raw = min(r["pnp"]["pnp-ordinary"]["raw_ratio"] for r in rows)
    best_corrected = min(r["pnp"]["pnp-ordinary"]["corrected_ratio"] for r in rows)
    verdict = {
        "note": (
            "corrected = raw / (1 + delta) is a fairness counterfactual against an "
            "Orca doing PnP's geometric work, never a scoreboard"
        ),
        "cells": rows,
        "best_raw_ratio_ordinary": best_raw,
        "best_corrected_ratio_ordinary": best_corrected,
        "all_cells_lost_raw": all(
            r["pnp"]["pnp-ordinary"]["raw_ratio"] > 1.0 for r in rows
        ),
        "all_cells_lost_corrected": all(
            r["pnp"]["pnp-ordinary"]["corrected_ratio"] > 1.0 for r in rows
        ),
        "park_condition_1_5x_still_binding": best_corrected > 1.5,
    }
    out = Path(__file__).resolve().parent / "corrected-scoreboard.json"
    out.write_text(json.dumps(verdict, indent=2), encoding="utf-8")
    print(
        f"\nbest raw {best_raw:.2f}x -> best corrected {best_corrected:.2f}x; "
        f"every cell still lost: raw={verdict['all_cells_lost_raw']} "
        f"corrected={verdict['all_cells_lost_corrected']}; "
        f"1.5x park still binding={verdict['park_condition_1_5x_still_binding']}"
    )
    print(f"-> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

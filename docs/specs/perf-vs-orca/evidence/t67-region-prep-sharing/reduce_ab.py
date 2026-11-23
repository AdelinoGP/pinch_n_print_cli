#!/usr/bin/env python3
"""t67 paired A/B reducer.

Same reduction semantics as t65/t58's reduce_ab.py (the standing paired
ordinary+accelerated gate), retargeted to t67's artifacts root and its
pre-declared timing cells: the standing four-cell set — classic supports-on
(benchy + base), both modes, 6 alternating baseline/candidate pairs plus 2
warmups per arm. Reports, per cell/mode:
  - paired median wall / CPU deltas (candidate vs baseline, per-pair ratio of
    medians over retained pairs),
  - per-arm medians and min-max ranges,
  - per-sample cpu/wall ratios with starvation handling: a pair is excluded
    (whole pair, never resampled) when either arm's cpu/wall ratio falls below
    0.5x that arm's own median ratio. Cells losing pairs to exclusion are
    reported with their retained-pair count (< 6 = inconclusive, not a pass).
Verdict per the pre-declared standing paired gate: candidate median wall win
AND CPU not contradicting in EVERY timed cell and BOTH modes.
"""

from __future__ import annotations

import csv
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
ROOT = REPO / ".local-artifacts" / "perimeter-reference-preparation" / "t67-region-prep-sharing"
MEASURE_CELLS = [
    f"{mode}-{fixture}-classic-on"
    for mode in ("ordinary", "accelerated")
    for fixture in ("benchy", "base")
]


def median(xs: list[float]) -> float:
    return statistics.median(xs)


def stats(rows: list[dict], arm: str) -> dict:
    sel = [
        float(r["wall_seconds"]) for r in rows if r["warmup"] == "0" and r["arm"] == arm
    ]
    cpu = [
        float(r["cpu_seconds"]) for r in rows if r["warmup"] == "0" and r["arm"] == arm
    ]
    ratios = [
        float(r["cpu_wall_ratio"])
        for r in rows
        if r["warmup"] == "0" and r["arm"] == arm
    ]
    return {
        "n": len(sel),
        "wall_median": median(sel),
        "wall_min": min(sel),
        "wall_max": max(sel),
        "cpu_median": median(cpu),
        "ratio_median": median(ratios),
    }


def reduce_cell(cell: str) -> dict:
    rows_path = ROOT / "measure" / cell / "rows.csv"
    if not rows_path.exists():
        return {"cell": cell, "status": "not_run"}
    all_rows = list(csv.DictReader(rows_path.open(encoding="utf-8-sig", newline="")))
    non_warm = [r for r in all_rows if r["warmup"] == "0"]
    base = [r for r in non_warm if r["arm"] == "baseline"]
    cand = [r for r in non_warm if r["arm"] == "candidate"]
    if not base or not cand:
        return {"cell": cell, "status": "no_samples"}
    starved = []
    ratio_base = median(float(r["cpu_wall_ratio"]) for r in base)
    ratio_cand = median(float(r["cpu_wall_ratio"]) for r in cand)
    retained_pairs = []
    for i in range(1, 7):
        b = next(
            (r for r in non_warm if r["arm"] == "baseline" and int(r["pair"]) == i),
            None,
        )
        c = next(
            (r for r in non_warm if r["arm"] == "candidate" and int(r["pair"]) == i),
            None,
        )
        if b is None or c is None:
            continue
        rb, rc = float(b["cpu_wall_ratio"]), float(c["cpu_wall_ratio"])
        if rb < 0.5 * ratio_base or rc < 0.5 * ratio_cand:
            starved.append(i)
            continue
        retained_pairs.append((b, c, i))
    if not retained_pairs:
        return {"cell": cell, "status": "all_pairs_starved", "excluded_pairs": starved}
    wall_ratios = [
        float(c["wall_seconds"]) / float(b["wall_seconds"])
        for b, c, _ in retained_pairs
    ]
    cpu_ratios = [
        float(c["cpu_seconds"]) / float(b["cpu_seconds"]) for b, c, _ in retained_pairs
    ]
    base_s = stats(all_rows, "baseline")
    cand_s = stats(all_rows, "candidate")
    return {
        "cell": cell,
        "status": "reduced",
        "retained_pairs": len(retained_pairs),
        "excluded_pairs": starved,
        "wall_ratio_paired_median": median(wall_ratios),
        "wall_delta_pct_paired_median": (median(wall_ratios) - 1.0) * 100.0,
        "cpu_ratio_paired_median": median(cpu_ratios),
        "cpu_delta_pct_paired_median": (median(cpu_ratios) - 1.0) * 100.0,
        "baseline": {
            "n": base_s["n"],
            "wall_median_s": base_s["wall_median"],
            "wall_range": [base_s["wall_min"], base_s["wall_max"]],
            "cpu_median_s": base_s["cpu_median"],
            "cpu_wall_ratio_median": base_s["ratio_median"],
        },
        "candidate": {
            "n": cand_s["n"],
            "wall_median_s": cand_s["wall_median"],
            "wall_range": [cand_s["wall_min"], cand_s["wall_max"]],
            "cpu_median_s": cand_s["cpu_median"],
            "cpu_wall_ratio_median": cand_s["ratio_median"],
        },
        "pairs": [
            {
                "pair": i,
                "baseline_wall_s": float(b["wall_seconds"]),
                "candidate_wall_s": float(c["wall_seconds"]),
                "baseline_cpu_s": float(b["cpu_seconds"]),
                "candidate_cpu_s": float(c["cpu_seconds"]),
                "baseline_ratio": float(b["cpu_wall_ratio"]),
                "candidate_ratio": float(c["cpu_wall_ratio"]),
                "sha_equal": b["sha256"] == c["sha256"],
            }
            for b, c, i in retained_pairs
        ],
        "unreduced_wall_medians": {
            "baseline_wall_median_s": median(
                float(r["wall_seconds"]) for r in non_warm if r["arm"] == "baseline"
            ),
            "candidate_wall_median_s": median(
                float(r["wall_seconds"]) for r in non_warm if r["arm"] == "candidate"
            ),
        },
    }


def main() -> int:
    results = {}
    for cell in MEASURE_CELLS:
        results[cell] = reduce_cell(cell)
    passes = []
    for cell, r in results.items():
        if r.get("status") != "reduced":
            passes.append(False)
            continue
        wall_ok = r["wall_delta_pct_paired_median"] < 0.0
        cpu_ok = r["cpu_delta_pct_paired_median"] <= 0.0
        r["wall_clause"] = "pass" if wall_ok else "fail"
        r["cpu_corroboration_clause"] = "pass" if cpu_ok else "fail"
        passes.append(wall_ok and cpu_ok and r["retained_pairs"] >= 6)
        if r["retained_pairs"] < 6:
            r["note"] = (
                "retained pairs below 6 — treat as inconclusive, not a measured pass"
            )
    strict_pass = all(passes)
    verdict = (
        "PASS (both clauses in every pre-declared cell, both modes)"
        if strict_pass
        else "GATE NOT MET — measured recommendation returns to the human"
    )
    out = {"verdict": verdict, "cells": results}
    (HERE / "ab-reduction.json").write_text(json.dumps(out, indent=1), encoding="utf-8")
    for cell in MEASURE_CELLS:
        r = results[cell]
        if r.get("status") == "reduced":
            if r.get("note"):
                print(f"{cell}: {r['note']}")
            print(
                f"{cell}: wall {r['wall_delta_pct_paired_median']:+.2f}% cpu {r['cpu_delta_pct_paired_median']:+.2f}%"
                f" pairs {r['retained_pairs']}/6 excluded {r['excluded_pairs']} "
                f"({r['wall_clause']}/{r['cpu_corroboration_clause']})"
            )
        else:
            print(f"{cell}: {r}")
    print("VERDICT:", verdict)
    return 0


if __name__ == "__main__":
    sys.exit(main())

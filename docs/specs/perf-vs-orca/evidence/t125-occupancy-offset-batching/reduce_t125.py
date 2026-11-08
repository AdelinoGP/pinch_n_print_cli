#!/usr/bin/env python3
"""Reduce ticket 125's raw output and paired A/B rows into committed evidence."""

import argparse
import csv
import json
import statistics
from collections import defaultdict
from pathlib import Path

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
ARMS = [
    "baseline-ordinary",
    "baseline-accelerated",
    "candidate-ordinary",
    "candidate-accelerated",
]
FLOOR_FRACTION = 0.008


def read_rows(path):
    with open(path, newline="", encoding="utf-8-sig") as handle:
        return list(csv.DictReader(handle))


def build_output(rows):
    out = {}
    for row in rows:
        out.setdefault(row["cell"], {})[row["arm"]] = {
            "wall_seconds": float(row["wall_seconds"]),
            "cpu_seconds": float(row["cpu_seconds"]),
            "gcode_bytes": int(row["gcode_bytes"]),
            "sha256": row["sha256"],
            "exit_code": int(row["exit_code"]),
            "completion_status": row["completion_status"],
            "non_fatal_error_count": int(row["non_fatal_error_count"]),
            "degraded": row["degraded"] == "true",
        }
    missing = [
        f"{cell}/{arm}"
        for cell in CELLS
        for arm in ARMS
        if arm not in out.get(cell, {})
    ]
    if missing:
        raise SystemExit(f"output rows missing: {missing}")
    return out


def build_reference(output, reference_path):
    with open(reference_path, encoding="utf-8") as handle:
        reference = json.load(handle)["output"]
    return {
        cell: {
            "t123_candidate_sha256": reference[cell]["candidate-ordinary"]["sha256"],
            "baseline_ordinary_sha256": output[cell]["baseline-ordinary"]["sha256"],
            "reproduces": output[cell]["baseline-ordinary"]["sha256"]
            == reference[cell]["candidate-ordinary"]["sha256"],
        }
        for cell in CELLS
    }


def build_equality(output):
    rows = {}
    for cell in CELLS:
        out = output[cell]
        base_ord = out["baseline-ordinary"]["sha256"]
        base_acc = out["baseline-accelerated"]["sha256"]
        cand_ord = out["candidate-ordinary"]["sha256"]
        cand_acc = out["candidate-accelerated"]["sha256"]
        rows[cell] = {
            "baseline_arms_identical": base_ord == base_acc,
            "candidate_arms_identical": cand_ord == cand_acc,
            "candidate_equals_baseline": cand_ord == base_ord,
            "baseline_ordinary_sha256": base_ord,
            "baseline_accelerated_sha256": base_acc,
            "candidate_ordinary_sha256": cand_ord,
            "candidate_accelerated_sha256": cand_acc,
        }
    return rows


def build_ab(rows):
    per_cell = defaultdict(lambda: defaultdict(list))
    for row in rows:
        per_cell[row["cell"]][row["arm"]].append(
            {
                "pair": int(row["pair"]),
                "order": row["order"],
                "wall_seconds": float(row["wall_seconds"]),
                "cpu_seconds": float(row["cpu_seconds"]),
                "sha256": row["sha256"],
                "gcode_bytes": int(row["gcode_bytes"]),
                "exit_code": int(row["exit_code"]),
                "completion_status": row["completion_status"],
                "non_fatal_error_count": int(row["non_fatal_error_count"]),
                "degraded": row["degraded"] == "true",
            }
        )

    out = {}
    for cell in CELLS:
        base = sorted(per_cell[cell]["baseline-ordinary"], key=lambda r: r["pair"])
        cand = sorted(per_cell[cell]["candidate-ordinary"], key=lambda r: r["pair"])
        if len(base) != 3 or len(cand) != 3:
            raise SystemExit(f"ab rows must contain three pairs for {cell}")
        base_wall = [r["wall_seconds"] for r in base]
        cand_wall = [r["wall_seconds"] for r in cand]
        base_cpu = [r["cpu_seconds"] for r in base]
        cand_cpu = [r["cpu_seconds"] for r in cand]
        base_median = statistics.median(base_wall)
        cand_median = statistics.median(cand_wall)
        wall_delta = base_median - cand_median
        floor = FLOOR_FRACTION * base_median
        cpu_delta = statistics.median(base_cpu) - statistics.median(cand_cpu)
        supports_on = cell.endswith("-on")
        failures = []
        if supports_on:
            if wall_delta < floor:
                failures.append("wall improvement below 0.80% floor")
            if cpu_delta <= 0:
                failures.append("median process CPU did not decrease")
            scope = "supports-on improvement"
        else:
            if wall_delta < 0:
                failures.append("median wall regressed")
            if cpu_delta < 0:
                failures.append("median process CPU regressed")
            scope = "supports-off no-regression"
        out[cell] = {
            "gate_scope": scope,
            "gate_pass": not failures,
            "failure_reasons": failures,
            "runs_per_arm": len(base_wall),
            "baseline_wall_seconds": base_wall,
            "candidate_wall_seconds": cand_wall,
            "baseline_cpu_seconds": base_cpu,
            "candidate_cpu_seconds": cand_cpu,
            "baseline_median_wall_seconds": base_median,
            "candidate_median_wall_seconds": cand_median,
            "median_wall_delta_seconds": wall_delta,
            "median_wall_delta_fraction": wall_delta / base_median,
            "floor_seconds": floor,
            "floor_multiple": wall_delta / floor,
            "baseline_median_cpu_seconds": statistics.median(base_cpu),
            "candidate_median_cpu_seconds": statistics.median(cand_cpu),
            "cpu_delta_median_seconds": cpu_delta,
            "cpu_corroborates": cpu_delta > 0,
            "pairs": [
                {
                    "pair": b["pair"],
                    "order": b["order"],
                    "wall_delta_seconds": b["wall_seconds"] - c["wall_seconds"],
                    "cpu_delta_seconds": b["cpu_seconds"] - c["cpu_seconds"],
                }
                for b, c in zip(base, cand)
            ],
            "baseline_sha256": sorted({r["sha256"] for r in base}),
            "candidate_sha256": sorted({r["sha256"] for r in cand}),
            "gcode_bytes": sorted({r["gcode_bytes"] for r in base + cand}),
            "exit_codes": sorted({r["exit_code"] for r in base + cand}),
            "completion_status": sorted({r["completion_status"] for r in base + cand}),
            "non_fatal_error_count": sorted(
                {r["non_fatal_error_count"] for r in base + cand}
            ),
            "degraded": sorted({r["degraded"] for r in base + cand}),
        }
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-rows", default="target/t125/output/rows.csv")
    parser.add_argument("--ab-rows", default="target/t125/ab/rows.csv")
    parser.add_argument(
        "--reference",
        default="docs/specs/perf-vs-orca/evidence/t123-facet-retirement/reduced/summary.json",
    )
    parser.add_argument(
        "--out",
        default="docs/specs/perf-vs-orca/evidence/t125-occupancy-offset-batching/reduced/summary.json",
    )
    parser.add_argument("--baseline-binary-sha256", required=True)
    parser.add_argument("--candidate-binary-sha256", required=True)
    parser.add_argument("--baseline-module-sha256", required=True)
    parser.add_argument("--candidate-module-sha256", required=True)
    parser.add_argument("--baseline-accelerated-binary-sha256", required=True)
    parser.add_argument("--candidate-accelerated-binary-sha256", required=True)
    parser.add_argument("--baseline-accelerated-module-sha256", required=True)
    parser.add_argument("--candidate-accelerated-module-sha256", required=True)
    args = parser.parse_args()

    output = build_output(read_rows(args.output_rows))
    ab = build_ab(read_rows(args.ab_rows))
    verdict = "KEEP" if all(row["gate_pass"] for row in ab.values()) else "DROP"
    summary = {
        "ticket": "125",
        "verdict": verdict,
        "converted_call_family": "per-origin role carve only; per-layer emit check remains singular",
        "floor_fraction": FLOOR_FRACTION,
        "cells": CELLS,
        "arms": ARMS,
        "binaries": {
            "baseline_ordinary_sha256": args.baseline_binary_sha256,
            "candidate_ordinary_sha256": args.candidate_binary_sha256,
            "baseline_accelerated_sha256": args.baseline_accelerated_binary_sha256,
            "candidate_accelerated_sha256": args.candidate_accelerated_binary_sha256,
        },
        "planner_modules": {
            "baseline_ordinary_sha256": args.baseline_module_sha256,
            "candidate_ordinary_sha256": args.candidate_module_sha256,
            "baseline_accelerated_sha256": args.baseline_accelerated_module_sha256,
            "candidate_accelerated_sha256": args.candidate_accelerated_module_sha256,
        },
        "output": output,
        "reference_check": build_reference(output, Path(args.reference)),
        "arm_equality": build_equality(output),
        "ab": ab,
    }
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=2, sort_keys=True)
        handle.write("\n")
    print(f"wrote {out_path}: {verdict}")
    for cell in CELLS:
        row = ab[cell]
        print(
            f"  {cell}: {'PASS' if row['gate_pass'] else 'FAIL'} "
            f"wall {row['median_wall_delta_fraction'] * 100:+.2f}% "
            f"({row['floor_multiple']:+.2f}x floor), "
            f"CPU {row['cpu_delta_median_seconds']:+.4f}s"
        )


if __name__ == "__main__":
    main()

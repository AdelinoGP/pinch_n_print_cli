#!/usr/bin/env python3
"""Reduce the ticket-116 raw campaign output into the committed evidence set.

Inputs (all gitignored, produced by run_t116.ps1 and audit_delta.py):
    target/t116/output/rows.csv      -Mode output rows (4 arms x 8 cells)
    target/t116/ab/rows.csv          -Mode ab rows (3 interleaved pairs x 8 cells)
    target/t116/audit/<cell>.json    audit_delta.py output per cell

Output:
    docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/reduced/summary.json

The reducer is deterministic: same inputs -> byte-identical summary.
"""

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
    with open(path, newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def median(values):
    return statistics.median(values)


def build_output(rows):
    out = {}
    for row in rows:
        cell = row["cell"]
        arm = row["arm"]
        out.setdefault(cell, {})[arm] = {
            "wall_seconds": float(row["wall_seconds"]),
            "cpu_seconds": float(row["cpu_seconds"]),
            "gcode_bytes": int(row["gcode_bytes"]),
            "sha256": row["sha256"],
            "exit_code": int(row["exit_code"]),
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


def build_ab(rows):
    """Per-cell paired statistics on the ordinary arms."""
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
            }
        )

    out = {}
    for cell in CELLS:
        base = sorted(per_cell[cell]["baseline-ordinary"], key=lambda r: r["pair"])
        cand = sorted(per_cell[cell]["candidate-ordinary"], key=lambda r: r["pair"])
        if not base or not cand:
            raise SystemExit(f"ab rows missing for {cell}")
        base_wall = [r["wall_seconds"] for r in base]
        cand_wall = [r["wall_seconds"] for r in cand]
        base_cpu = [r["cpu_seconds"] for r in base]
        cand_cpu = [r["cpu_seconds"] for r in cand]
        base_median = median(base_wall)
        cand_median = median(cand_wall)
        delta_s = base_median - cand_median
        floor_s = FLOOR_FRACTION * base_median
        pair_deltas = [
            {
                "pair": b["pair"],
                "wall_delta_seconds": b["wall_seconds"] - c["wall_seconds"],
                "cpu_delta_seconds": b["cpu_seconds"] - c["cpu_seconds"],
            }
            for b, c in zip(base, cand)
        ]
        out[cell] = {
            "runs_per_arm": len(base_wall),
            "baseline_wall_seconds": base_wall,
            "candidate_wall_seconds": cand_wall,
            "baseline_cpu_seconds": base_cpu,
            "candidate_cpu_seconds": cand_cpu,
            "baseline_median_wall_seconds": base_median,
            "candidate_median_wall_seconds": cand_median,
            "median_wall_delta_seconds": delta_s,
            "median_wall_delta_fraction": delta_s / base_median,
            "floor_seconds": floor_s,
            "floor_multiple": delta_s / floor_s,
            "cpu_delta_median_seconds": median(base_cpu) - median(cand_cpu),
            "cpu_corroborates": median(cand_cpu) < median(base_cpu),
            "pairs": pair_deltas,
            "baseline_sha256": sorted({r["sha256"] for r in base}),
            "candidate_sha256": sorted({r["sha256"] for r in cand}),
            "gcode_bytes": sorted({r["gcode_bytes"] for r in base + cand}),
            "exit_codes": sorted({r["exit_code"] for r in base + cand}),
        }
    return out


def build_delta(audit_dir):
    out = {}
    for cell in CELLS:
        path = audit_dir / f"{cell}.json"
        if not path.exists():
            raise SystemExit(f"audit row missing: {path}")
        with open(path, encoding="utf-8") as handle:
            out[cell] = json.load(handle)
    return out


def build_reference(output, reference_path):
    """Compare the baseline ordinary arm with the t115 post-merge reference."""
    with open(reference_path, encoding="utf-8") as handle:
        reference = json.load(handle)["output_audit"]
    rows = {}
    for cell in CELLS:
        expected = reference[cell]["post_sha256"]
        got = output[cell]["baseline-ordinary"]["sha256"]
        rows[cell] = {
            "t115_post_sha256": expected,
            "baseline_ordinary_sha256": got,
            "reproduces": got == expected,
        }
    return rows


def build_equality(output):
    rows = {}
    for cell in CELLS:
        base_ord = output[cell]["baseline-ordinary"]["sha256"]
        base_acc = output[cell]["baseline-accelerated"]["sha256"]
        cand_ord = output[cell]["candidate-ordinary"]["sha256"]
        cand_acc = output[cell]["candidate-accelerated"]["sha256"]
        rows[cell] = {
            "baseline_arms_identical": base_ord == base_acc,
            "candidate_arms_identical": cand_ord == cand_acc,
            "candidate_moved_from_baseline": cand_ord != base_ord,
            "baseline_ordinary_sha256": base_ord,
            "baseline_accelerated_sha256": base_acc,
            "candidate_ordinary_sha256": cand_ord,
            "candidate_accelerated_sha256": cand_acc,
        }
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output-rows",
        default="target/t116/output/rows.csv",
        help="rows.csv from run_t116.ps1 -Mode output",
    )
    parser.add_argument(
        "--ab-rows",
        default="target/t116/ab/rows.csv",
        help="rows.csv from run_t116.ps1 -Mode ab",
    )
    parser.add_argument(
        "--audit-dir",
        default="target/t116/audit",
        help="audit_delta.py output directory",
    )
    parser.add_argument(
        "--reference",
        default="docs/specs/perf-vs-orca/evidence/t115-post-merge/summary.json",
        help="t115 post-merge summary.json (in-repo reference set)",
    )
    parser.add_argument(
        "--out",
        default="docs/specs/perf-vs-orca/evidence/t116-prep-slice-canonical-bridge/reduced/summary.json",
    )
    parser.add_argument("--candidate-binary-sha256", default="")
    parser.add_argument("--baseline-binary-sha256", default="")
    args = parser.parse_args()

    output = build_output(read_rows(args.output_rows))
    ab = build_ab(read_rows(args.ab_rows))
    delta = build_delta(Path(args.audit_dir))
    reference = build_reference(output, args.reference)
    equality = build_equality(output)

    summary = {
        "ticket": "116",
        "floor_fraction": FLOOR_FRACTION,
        "cells": CELLS,
        "arms": ARMS,
        "binaries": {
            "baseline_ordinary_sha256": args.baseline_binary_sha256,
            "candidate_ordinary_sha256": args.candidate_binary_sha256,
        },
        "output": output,
        "reference_check": reference,
        "arm_equality": equality,
        "delta_audit": delta,
        "ab": ab,
    }

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=2, sort_keys=True)
        handle.write("\n")
    print(f"wrote {out_path}")


if __name__ == "__main__":
    main()

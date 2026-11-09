#!/usr/bin/env python3
"""Reduce the ticket-132 raw campaign output into the committed evidence set.

Inputs (gitignored, produced by run_t132.ps1):
    target/t132/output/rows.csv      -Mode output rows (4 arms x 8 cells)
    target/t132/ab/rows.csv          -Mode ab rows (3 interleaved pairs x 8 cells)
    target/t132/manifest.json        -Mode manifest (arm binary + planner hashes)

Output:
    docs/specs/perf-vs-orca/evidence/t132-planner-occupancy-memo/reduced/summary.json

The reducer is deterministic: same inputs -> byte-identical summary. The
reference set is ticket 123's candidate-ordinary G-code, the set every PnP arm
has reproduced since (t124, t131); this take changes no geometry, so all four
arms must reproduce it cell by cell.

The gate is the standing paired rule: the 0.80 % wall floor with process-CPU
corroboration. This candidate is supports-on only, so the four supports-on
cells must clear the floor and the four supports-off cells must show no
regression (the amended gate of ticket 117).
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

SUPPORTS_ON = {cell for cell in CELLS if cell.endswith("-on")}

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
        if len(base) != len(cand):
            raise SystemExit(
                f"ab pair count mismatch for {cell}: {len(base)} vs {len(cand)}"
            )
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
                "order": b["order"],
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
            "supports_on": cell in SUPPORTS_ON,
        }
    return out


def build_reference(output, reference_path):
    with open(reference_path, encoding="utf-8") as handle:
        reference = json.load(handle)["output"]
    rows = {}
    for cell in CELLS:
        expected = reference[cell]["candidate-ordinary"]["sha256"]
        got = {arm: output[cell][arm]["sha256"] for arm in ARMS}
        rows[cell] = {
            "t123_candidate_sha256": expected,
            "arm_sha256": got,
            "reproduces": all(sha == expected for sha in got.values()),
        }
    return rows


def build_equality(output):
    rows = {}
    for cell in CELLS:
        shas = {arm: output[cell][arm]["sha256"] for arm in ARMS}
        rows[cell] = {
            "baseline_arms_identical": shas["baseline-ordinary"]
            == shas["baseline-accelerated"],
            "candidate_arms_identical": shas["candidate-ordinary"]
            == shas["candidate-accelerated"],
            "all_arms_identical": len(set(shas.values())) == 1,
            "sha256": sorted(set(shas.values())),
        }
    return rows


def build_gate(ab):
    rows = {}
    for cell in CELLS:
        row = ab[cell]
        clears_floor = (
            row["floor_multiple"] >= 1.0
            and row["median_wall_delta_seconds"] > 0.0
            and row["cpu_delta_median_seconds"] >= 0.0
        )
        no_regression = (
            row["median_wall_delta_seconds"] >= 0.0
            and row["cpu_delta_median_seconds"] >= 0.0
        )
        if cell in SUPPORTS_ON:
            verdict = "PASS" if clears_floor else "FAIL"
            rule = (
                "supports-on: median wall delta >= 0.80 % of baseline median, "
                "process CPU corroborating"
            )
        else:
            verdict = "PASS" if no_regression else "FAIL"
            rule = "supports-off: no median wall or CPU regression"
        rows[cell] = {
            "rule": rule,
            "clears_floor": clears_floor,
            "no_regression": no_regression,
            "cpu_corroborates": row["cpu_corroborates"],
            "verdict": verdict,
        }
    return {
        "floor_fraction": FLOOR_FRACTION,
        "cells": rows,
        "passed": sum(1 for row in rows.values() if row["verdict"] == "PASS"),
        "total": len(rows),
    }


def build_manifest(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output-rows",
        default="target/t132/output/rows.csv",
    )
    parser.add_argument(
        "--ab-rows",
        default="target/t132/ab/rows.csv",
    )
    parser.add_argument(
        "--manifest",
        default="target/t132/manifest.json",
    )
    parser.add_argument(
        "--reference",
        default="docs/specs/perf-vs-orca/evidence/t123-facet-retirement/reduced/summary.json",
    )
    parser.add_argument(
        "--out",
        default="docs/specs/perf-vs-orca/evidence/t132-planner-occupancy-memo/reduced/summary.json",
    )
    args = parser.parse_args()

    output = build_output(read_rows(args.output_rows))
    ab = build_ab(read_rows(args.ab_rows))
    summary = {
        "ticket": "132",
        "arms": build_manifest(args.manifest),
        "floor_fraction": FLOOR_FRACTION,
        "output": output,
        "equality": build_equality(output),
        "reference_check": build_reference(output, args.reference),
        "ab": ab,
        "gate": build_gate(ab),
    }

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=2, sort_keys=True)
        handle.write("\n")
    print(f"wrote {out_path}")
    print(
        "gate: {passed}/{total} cells PASS".format(
            passed=summary["gate"]["passed"], total=summary["gate"]["total"]
        )
    )
    for cell in CELLS:
        gate = summary["gate"]["cells"][cell]
        row = ab[cell]
        print(
            "{cell:24s} {verdict:4s} wall {base:8.2f} -> {cand:8.2f} s "
            "delta {delta:+7.2f} s ({frac:+.3%}) floor x{multiple:5.2f} cpu {cpu:+.2f} s".format(
                cell=cell,
                verdict=gate["verdict"],
                base=row["baseline_median_wall_seconds"],
                cand=row["candidate_median_wall_seconds"],
                delta=row["median_wall_delta_seconds"],
                frac=row["median_wall_delta_fraction"],
                multiple=row["floor_multiple"],
                cpu=row["cpu_delta_median_seconds"],
            )
        )


if __name__ == "__main__":
    main()

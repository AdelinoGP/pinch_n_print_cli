#!/usr/bin/env python3
"""Ticket 139 reducer: reference set, paired A/B, and adhesion-oracle verdicts.

Reads the raw t139 campaign tree (`target/t139/`, gitignored) and writes
`reduced/campaign.json`. The verifier reads only the reduced file.

Run from the repository root:

    python docs/specs/perf-vs-orca/evidence/t139-layer-plane-parity/reduce_t139.py
"""

from __future__ import annotations

import csv
import json
import statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
T139 = ROOT / "target/t139"
OUT = Path(__file__).resolve().parent / "reduced" / "campaign.json"

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


def read_rows(path: Path) -> list[dict]:
    with path.open(encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def main() -> int:
    output = read_rows(T139 / "output-final2" / "rows.csv")
    ab = read_rows(T139 / "ab2" / "rows.csv")

    reference = {}
    for row in output:
        if row["arm"] == "candidate-ordinary":
            reference[row["cell"]] = {
                "sha256": row["sha256"],
                "bytes": int(row["gcode_bytes"]),
                "wall_seconds": float(row["wall_seconds"]),
                "cpu_seconds": float(row["cpu_seconds"]),
            }
    accelerated = {
        row["cell"]: row["sha256"]
        for row in output
        if row["arm"] == "candidate-accelerated"
    }

    paired = {}
    for cell in CELLS:
        base = [r for r in ab if r["cell"] == cell and r["arm"] == "baseline-ordinary"]
        cand = [r for r in ab if r["cell"] == cell and r["arm"] == "candidate-ordinary"]
        if not base or not cand:
            raise SystemExit(f"missing A/B rows for {cell}")
        wall_base = statistics.median(float(r["wall_seconds"]) for r in base)
        wall_cand = statistics.median(float(r["wall_seconds"]) for r in cand)
        cpu_base = statistics.median(float(r["cpu_seconds"]) for r in base)
        cpu_cand = statistics.median(float(r["cpu_seconds"]) for r in cand)
        paired[cell] = {
            "pairs": len(base),
            "baseline_wall_median_s": round(wall_base, 4),
            "candidate_wall_median_s": round(wall_cand, 4),
            "wall_ratio_candidate_over_baseline": round(wall_cand / wall_base, 4),
            "baseline_cpu_median_s": round(cpu_base, 4),
            "candidate_cpu_median_s": round(cpu_cand, 4),
            "cpu_ratio_candidate_over_baseline": round(cpu_cand / cpu_base, 4),
        }

    oracle = {}
    for cell in CELLS:
        path = T139 / "oracle-final" / f"{cell}.json"
        data = json.loads(path.read_text(encoding="utf-8"))
        checks = data["checks"]
        # Only the gate keys are verdicts; presence flags (`*_both_absent`) are
        # records that are legitimately false when the feature exists.
        fails = sorted(
            key
            for key, value in checks.items()
            if value is False and (key.endswith("_pass") or key == "pass")
        )
        oracle[cell] = {
            "pass": checks["pass"],
            "failing_checks": fails,
            "pnp": {
                feature: {
                    "loop_count": data["pnp"][feature]["loop_count"],
                    "swept_area_mm2": data["pnp"][feature]["swept_area_mm2"],
                    "outer_loop_area_mm2": data["pnp"][feature]["outer_loop_area_mm2"],
                    "growth_mm": data["pnp"][feature]["growth_mm"],
                }
                for feature in ("Brim", "Skirt")
                if feature in data["pnp"]
            },
            "orca": {
                feature: {
                    "loop_count": data["orca"][feature]["loop_count"],
                    "swept_area_mm2": data["orca"][feature]["swept_area_mm2"],
                    "outer_loop_area_mm2": data["orca"][feature]["outer_loop_area_mm2"],
                    "growth_mm": data["orca"][feature]["growth_mm"],
                }
                for feature in ("Brim", "Skirt")
                if feature in data["orca"]
            },
            "relative": {
                key: round(value, 4)
                for key, value in checks.items()
                if isinstance(value, float)
            },
        }

    summary = {
        "reference_set": reference,
        "candidate_arms_identical": all(
            reference[cell]["sha256"] == accelerated[cell] for cell in CELLS
        ),
        "ab": paired,
        "adhesion_oracle": {
            "cells": oracle,
            "pass_count": sum(1 for cell in CELLS if oracle[cell]["pass"]),
        },
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUT}")
    for cell in CELLS:
        row = paired[cell]
        print(
            f"{cell}: wall {row['baseline_wall_median_s']} -> "
            f"{row['candidate_wall_median_s']} ({row['wall_ratio_candidate_over_baseline']:.3f}x), "
            f"cpu {row['cpu_ratio_candidate_over_baseline']:.3f}x"
        )
    print(f"oracle pass {summary['adhesion_oracle']['pass_count']}/8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

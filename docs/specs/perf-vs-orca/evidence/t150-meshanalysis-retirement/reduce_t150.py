#!/usr/bin/env python3
"""Reduce the t150 campaign into census.json.

Reads (all under the repo root):
  target/t150/output/rows.csv   - 8 cells x {baseline, candidate} identity runs
  target/t150/t150/rows.csv     - the paired A/B rows and the identical-arm null control
  target/t150/stage/*.stderr.log - instrumented MeshAnalysis stage rows (stage reading only)
  docs/specs/perf-vs-orca/evidence/t148-print-plane-carve/reference.json

The raw CSV tree is gitignored. census.json is the in-repo reduced set that
verify.py re-derives every published number from.
"""

from __future__ import annotations

import csv
import json
import re
import statistics
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
EVIDENCE = Path(__file__).resolve().parent
OUTPUT_CSV = ROOT / "target/t150/output/rows.csv"
AB_CSV = ROOT / "target/t150/t150/rows.csv"
STAGE_DIR = ROOT / "target/t150/stage"
REFERENCE = (
    ROOT / "docs/specs/perf-vs-orca/evidence/t148-print-plane-carve/reference.json"
)

FLOOR_PCT = 0.80

# Ratios of record at the t148 active reference set. Supports-off cells are
# t145's; supports-on cells are t148's. Both are cited in FINDINGS.md; they are
# applied here only to compose an indicative post-t150 ratio (see "route").
RATIO_OF_RECORD = {
    "benchy-classic-off": 2.043,
    "benchy-classic-on": 2.705,
    "benchy-arachne-off": 1.853,
    "benchy-arachne-on": 2.829,
    "base-classic-off": 2.078,
    "base-classic-on": 3.356,
    "base-arachne-off": 1.984,
    "base-arachne-on": 3.248,
}

STAGE_CELLS = {
    "base-classic-off": ("base-classic-off", "base-classic-off"),
    "base-classic-on": ("base-classic-supports-on", "base-classic-supports-on"),
    "benchy-classic-off": (
        "benchy-classic-supports-off",
        "benchy-classic-supports-off",
    ),
}


def read_rows(path: Path) -> list[dict]:
    with path.open(encoding="utf-8-sig", newline="") as fh:
        return [r for r in csv.DictReader(fh) if r.get("cell")]


def stage_ms(log: Path) -> int:
    text = log.read_text(encoding="utf-8", errors="replace")
    hits = re.findall(
        r'"stage":"PrePass::MeshAnalysis","module_id":"host:mesh_analysis".*?"elapsed_ms":(\d+)',
        text,
    )
    if not hits:
        raise SystemExit(f"no MeshAnalysis elapsed_ms in {log}")
    return int(hits[-1])


def main() -> None:
    reference = json.loads(REFERENCE.read_text(encoding="utf-8"))
    active = reference["active_set"]

    output_rows = [r for r in read_rows(OUTPUT_CSV) if r["mode"] == "output"]
    identity = []
    for r in output_rows:
        identity.append(
            {
                "cell": r["cell"],
                "arm": r["arm"],
                "sha256": r["sha256"],
                "reference_sha256": active[r["cell"]],
                "matches_reference": r["sha256"] == active[r["cell"]],
                "gcode_bytes": int(r["gcode_bytes"]),
                "exit_code": int(r["exit_code"]),
                "degraded": r["degraded"] == "true",
                "exact_z_rejections": int(r["exact_z_rejections"]),
                "wall_seconds": float(r["wall_seconds"]),
                "cpu_seconds": float(r["cpu_seconds"]),
            }
        )

    rows = read_rows(AB_CSV)
    pairs: dict[str, dict[str, dict[str, float]]] = defaultdict(dict)
    for r in rows:
        if r["mode"] != "ab":
            continue
        pairs[r["cell"]].setdefault(r["pair"], {})[r["arm"]] = {
            "wall": float(r["wall_seconds"]),
            "cpu": float(r["cpu_seconds"]),
            "sha256": r["sha256"],
        }

    ab = {}
    for cell in sorted(pairs):
        per_pair = []
        for pair in sorted(pairs[cell], key=int):
            b = pairs[cell][pair]["baseline"]
            c = pairs[cell][pair]["candidate"]
            per_pair.append(
                {
                    "pair": int(pair),
                    "baseline_wall_s": b["wall"],
                    "candidate_wall_s": c["wall"],
                    "wall_delta_pct": round(
                        (c["wall"] - b["wall"]) / b["wall"] * 100, 4
                    ),
                    "baseline_cpu_s": b["cpu"],
                    "candidate_cpu_s": c["cpu"],
                    "cpu_delta_pct": round((c["cpu"] - b["cpu"]) / b["cpu"] * 100, 4),
                    "baseline_sha256": b["sha256"],
                    "candidate_sha256": c["sha256"],
                }
            )
        wall_median = round(statistics.median(p["wall_delta_pct"] for p in per_pair), 4)
        cpu_median = round(statistics.median(p["cpu_delta_pct"] for p in per_pair), 4)
        ab[cell] = {
            "pairs": per_pair,
            "pair_count": len(per_pair),
            "candidate_faster_pairs": sum(
                1 for p in per_pair if p["wall_delta_pct"] < 0
            ),
            "wall_delta_median_pct": wall_median,
            "cpu_delta_median_pct": cpu_median,
            "floor_multiple": round(-wall_median / FLOOR_PCT, 4),
            "sha_identical_in_all_pairs": all(
                p["baseline_sha256"] == p["candidate_sha256"] for p in per_pair
            ),
        }

    null_pairs = defaultdict(dict)
    for r in rows:
        if r["mode"] != "null":
            continue
        null_pairs[r["pair"]][r["sequence"]] = {
            "wall": float(r["wall_seconds"]),
            "cpu": float(r["cpu_seconds"]),
        }
    null = []
    for pair in sorted(null_pairs, key=int):
        a = null_pairs[pair]["1"]
        b = null_pairs[pair]["2"]
        null.append(
            {
                "pair": int(pair),
                "first_wall_s": a["wall"],
                "second_wall_s": b["wall"],
                "wall_delta_pct": round((b["wall"] - a["wall"]) / a["wall"] * 100, 4),
                "first_cpu_s": a["cpu"],
                "second_cpu_s": b["cpu"],
                "cpu_delta_pct": round((b["cpu"] - a["cpu"]) / a["cpu"] * 100, 4),
            }
        )
    null_wall = [p["wall_delta_pct"] for p in null]
    null_cpu = [p["cpu_delta_pct"] for p in null]

    stages = {}
    for cell, (base_name, cand_name) in STAGE_CELLS.items():
        base_ms = stage_ms(STAGE_DIR / f"{base_name}-baseline.stderr.log")
        cand_ms = stage_ms(STAGE_DIR / f"{cand_name}-candidate.stderr.log")
        stages[cell] = {
            "baseline_mesh_analysis_ms": base_ms,
            "candidate_mesh_analysis_ms": cand_ms,
            "delta_ms": cand_ms - base_ms,
            "delta_pct": round((cand_ms - base_ms) / base_ms * 100, 4),
        }

    # Indicative composition: apply each cell's paired wall delta to its ratio
    # of record. The ratio is from another window, so this is not a scoreboard.
    route = {}
    for cell, ratio in RATIO_OF_RECORD.items():
        delta = ab[cell]["wall_delta_median_pct"] / 100.0
        composed = ratio * (1.0 + delta)
        route[cell] = {
            "ratio_of_record": ratio,
            "post_t150_ratio_indicative": round(composed, 4),
            "required_cut_pct_indicative": round((1.0 - 1.0 / composed) * 100, 4),
            "required_cut_pct_of_record": round((1.0 - 1.0 / ratio) * 100, 4),
        }

    census = {
        "ticket": 150,
        "title": "MeshAnalysis bridge retirement",
        "date": "2026-11-13",
        "floor_pct": FLOOR_PCT,
        "identity": {
            "runs": identity,
            "cells_identical": len(
                {r["cell"] for r in identity if r["matches_reference"]}
            ),
            "all_match": all(r["matches_reference"] for r in identity),
            "degraded_cells": [
                r["cell"] + "/" + r["arm"] for r in identity if r["degraded"]
            ],
            "exact_z_rejections_total": sum(r["exact_z_rejections"] for r in identity),
            "reference_source": "docs/specs/perf-vs-orca/evidence/t148-print-plane-carve/reference.json",
        },
        "ab": ab,
        "null_control": {
            "cell": "base-classic-off",
            "arm": "candidate vs candidate",
            "pairs": null,
            "wall_band_pct": [min(null_wall), max(null_wall)],
            "wall_median_pct": round(statistics.median(null_wall), 4),
            "cpu_band_pct": [min(null_cpu), max(null_cpu)],
        },
        "stage": stages,
        "route": route,
        "disclosures": [
            "Stage times come from --instrument-stderr runs and are not wall claims.",
            "Ratios of record are t145 (supports-off) and t148 (supports-on); the "
            "post-t150 columns apply this take's paired deltas to them and are indicative, "
            "not a re-taken scoreboard.",
            "The accelerated timing arm is not taken; the accelerated host does not build "
            "(libmimalloc-sys, unchanged since t142). Output identity is measured on the "
            "ordinary arm only.",
            "The campaign ran on one box in one window; the identical-arm null control is "
            "the only noise bound quoted.",
        ],
    }

    out = EVIDENCE / "census.json"
    out.write_text(json.dumps(census, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {out}")
    print(
        f"  identity: {census['identity']['cells_identical']}/8 cells match, all={census['identity']['all_match']}"
    )
    for cell, data in ab.items():
        print(
            f"  {cell:22} n={data['pair_count']} wall {data['wall_delta_median_pct']:+.2f}% "
            f"({data['floor_multiple']:.2f}x floor) cpu {data['cpu_delta_median_pct']:+.2f}%"
        )


if __name__ == "__main__":
    main()

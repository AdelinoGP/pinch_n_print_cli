#!/usr/bin/env python3
"""Reduce criterion's on-disk artifacts to `baselines.json` (t29).

Reads every `target/criterion/**/new/{estimates,sample,tukey,benchmark}.json`
left by `cargo bench -p slicer-helpers --bench mesh_ops` and emits the reduced
per-leaf table quoted by `FINDINGS.md`. The raw tree is gitignored; this script
and `baselines.json` are tracked, so the reduced set is re-derivable from a
plain checkout after re-running the bench.

Usage:
    python docs/specs/perf-vs-orca/evidence/t29-mesh-ops-fixtures/reduce-baselines.py
"""

from __future__ import annotations

import json
import pathlib
import sys

EVIDENCE = pathlib.Path(__file__).resolve().parent
# docs/specs/perf-vs-orca/evidence/t29-mesh-ops-fixtures -> repository root
REPO_ROOT = EVIDENCE.parents[4]
CRITERION = REPO_ROOT / "target" / "criterion"

# Leaves this ticket owns; anything else in target/criterion is ignored.
WANTED = {
    "mesh_ops/import_step/cube",
    "mesh_ops/import_step/assembly",
    "mesh_ops/repair/wedge",
    "mesh_ops/decimate/calicat_default",
}

# Pre-ticket leaves, quoted from the t15 evidence set for the not-comparable
# note. Values are t15's on-disk means/medians at HEAD 3f51b7b9.
PRETICKET = {
    "mesh_ops/repair/cube": {"mean_ns": 2947.0, "median_ns": 2903.0},
    "mesh_ops/decimate/cube_default": {"mean_ns": 3745.0, "median_ns": 3692.9},
}


def leaf_dirs() -> list[pathlib.Path]:
    return sorted(p.parent for p in CRITERION.glob("**/new/estimates.json"))


def count_outliers(
    times: list[float], iters: list[int], fences: list[float] | None
) -> dict:
    """Reproduce criterion's Tukey outlier buckets from raw sample times.

    criterion 0.5.1 persists `tukey.json` as the four fence values
    `[low_severe, low_mild, high_mild, high_severe]` and does not persist the
    bucket counts; t15 recomputed them from the raw times and this mirrors
    that discipline. The fences are in per-iteration units, while
    `sample.json`'s `times` are per-sample totals across that sample's
    `iters`, so normalize before comparing.
    """
    if not fences or len(fences) != 4:
        return {"total": 0, "recomputed": False}
    low_severe, low_mild, high_mild, high_severe = fences
    buckets = {
        "low_severe": 0,
        "low_mild": 0,
        "high_mild": 0,
        "high_severe": 0,
    }
    for t, n in zip(times, iters):
        per_iter = t / n
        if per_iter < low_severe:
            buckets["low_severe"] += 1
        elif per_iter < low_mild:
            buckets["low_mild"] += 1
        elif per_iter > high_severe:
            buckets["high_severe"] += 1
        elif per_iter > high_mild:
            buckets["high_mild"] += 1
    buckets["total"] = sum(v for k, v in buckets.items() if k != "total")
    buckets["recomputed"] = True
    return buckets


def read_leaf(new_dir: pathlib.Path) -> dict | None:
    benchmark = json.loads((new_dir / "benchmark.json").read_text(encoding="utf-8"))
    full_id = benchmark.get("full_id") or benchmark.get("directory_name")
    if full_id not in WANTED:
        return None
    estimates = json.loads((new_dir / "estimates.json").read_text(encoding="utf-8"))
    sample = json.loads((new_dir / "sample.json").read_text(encoding="utf-8"))
    tukey = json.loads((new_dir / "tukey.json").read_text(encoding="utf-8"))
    mean = estimates["mean"]
    median = estimates["median"]
    slope = estimates.get("slope")
    times = sample["times"]
    fences = tukey if isinstance(tukey, list) else None

    base_dir = new_dir.parent / "base"
    base_present = (base_dir / "estimates.json").is_file()
    base_identical = False
    if base_present:
        base_estimates = json.loads(
            (base_dir / "estimates.json").read_text(encoding="utf-8")
        )
        base_identical = base_estimates == estimates

    return {
        "full_id": full_id,
        "group_id": benchmark.get("group_id"),
        "sampling_mode": sample.get("sampling_mode"),
        "samples": len(times),
        "iters_per_sample_range": [min(sample["iters"]), max(sample["iters"])],
        "mean_ns": mean["point_estimate"],
        "mean_ci95_ns": [
            mean["confidence_interval"]["lower_bound"],
            mean["confidence_interval"]["upper_bound"],
        ],
        "median_ns": median["point_estimate"],
        "median_ci95_ns": [
            median["confidence_interval"]["lower_bound"],
            median["confidence_interval"]["upper_bound"],
        ],
        "std_dev_ns": estimates.get("std_dev", {}).get("point_estimate"),
        "cv_pct": (
            100.0 * mean["standard_error"] / mean["point_estimate"]
            if mean["point_estimate"]
            else None
        ),
        "slope_ns": slope["point_estimate"] if slope else None,
        "console_typical_vs_mean_pct": (
            100.0
            * (slope["point_estimate"] - mean["point_estimate"])
            / mean["point_estimate"]
            if slope and mean["point_estimate"]
            else None
        ),
        "outliers": count_outliers(times, sample["iters"], fences),
        "tukey_fences_ns": fences,
        "base_present": base_present,
        "base_identical_to_new": base_identical,
    }


def main() -> int:
    if not CRITERION.is_dir():
        print(f"error: {CRITERION} not found; run the bench first", file=sys.stderr)
        return 1
    rows = []
    seen: set[str] = set()
    for new_dir in leaf_dirs():
        row = read_leaf(new_dir)
        if row is None or row["full_id"] in seen:
            continue
        seen.add(row["full_id"])
        rows.append(row)
    missing = WANTED - seen
    if missing:
        print(
            f"error: no criterion data for leaves: {sorted(missing)}", file=sys.stderr
        )
        return 1

    doc = {
        "source": (
            "target/criterion/**/new/*.json (criterion 0.5.1), reduced by "
            "reduce-baselines.py"
        ),
        "bench": "cargo bench -p slicer-helpers --bench mesh_ops",
        "note": (
            "First run on the t29 fixture change, so base == new for every "
            "leaf here. The pre-ticket mesh_ops/repair/cube and "
            "mesh_ops/decimate/cube_default leaves were replaced; their t15 "
            "values are recorded under pre_ticket_leaves_superseded and are "
            "not comparable to these leaves."
        ),
        "leaves": rows,
        "pre_ticket_leaves_superseded": PRETICKET,
    }
    out = EVIDENCE / "baselines.json"
    out.write_text(json.dumps(doc, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {out} ({len(rows)} leaves)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

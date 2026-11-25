#!/usr/bin/env python3
"""Verify every number t150 publishes, from census.json alone.

Usage:
  python verify.py            # 0 on success, 1 on the first failed claim group
  python verify.py --self-test  # negative controls: each mutation must be rejected

census.json is produced by reduce_t150.py from the gitignored campaign CSV tree.
This script is stdlib-only and reads no raw capture.
"""

from __future__ import annotations

import copy
import json
import statistics
import sys
from pathlib import Path

CENSUS = Path(__file__).resolve().parent / "census.json"

FLOOR_PCT = 0.80

# Published values (FINDINGS.md). The verifier re-derives each from census.json.
PUBLISHED = {
    "identity_cells": 8,
    "identity_runs": 16,
    "identity_bytes": {
        "benchy-classic-off": 4_085_300,
        "benchy-classic-on": 8_673_104,
        "benchy-arachne-off": 4_227_096,
        "benchy-arachne-on": 8_815_235,
        "base-classic-off": 21_186_923,
        "base-classic-on": 39_732_305,
        "base-arachne-off": 21_046_723,
        "base-arachne-on": 39_594_635,
    },
    "pair_counts": {
        "base-classic-off": 3,
        "base-classic-on": 6,
        "base-arachne-off": 3,
        "base-arachne-on": 3,
        "benchy-classic-off": 3,
        "benchy-classic-on": 3,
        "benchy-arachne-off": 3,
        "benchy-arachne-on": 3,
    },
    "wall_median_pct": {
        "benchy-classic-off": -1.77,
        "benchy-classic-on": -4.95,
        "benchy-arachne-off": -4.87,
        "benchy-arachne-on": -3.20,
        "base-classic-off": -12.08,
        "base-classic-on": -4.06,
        "base-arachne-off": -9.37,
        "base-arachne-on": -5.78,
    },
    "cpu_median_pct": {
        "benchy-classic-off": -0.41,
        "benchy-classic-on": -1.29,
        "benchy-arachne-off": -0.32,
        "benchy-arachne-on": -0.19,
        "base-classic-off": -1.77,
        "base-classic-on": -0.10,
        "base-arachne-off": -1.67,
        "base-arachne-on": -0.46,
    },
    "null_pairs": 6,
    "null_wall_band_pct": [-3.6715, 2.1991],
    "null_wall_median_pct": 1.0631,
    "null_cpu_band_pct": [-0.1725, 0.5002],
    "stage": {
        "base-classic-off": {"baseline": 4445, "candidate": 732, "delta_pct": -83.53},
        "base-classic-on": {"baseline": 4616, "candidate": 582, "delta_pct": -87.39},
        "benchy-classic-off": {"baseline": 168, "candidate": 95, "delta_pct": -43.45},
    },
    "stage_share_of_identity_wall_pct": 13.48,
    "route": {
        "benchy-classic-off": [2.007, 50.17],
        "benchy-classic-on": [2.571, 61.11],
        "benchy-arachne-off": [1.763, 43.27],
        "benchy-arachne-on": [2.739, 63.48],
        "base-classic-off": [1.827, 45.27],
        "base-classic-on": [3.220, 68.94],
        "base-arachne-off": [1.798, 44.38],
        "base-arachne-on": [3.060, 67.32],
    },
}


def close(a: float, b: float, tol: float = 0.011) -> bool:
    return abs(a - b) <= tol


def check_identity(c: dict) -> list[tuple[str, bool, str]]:
    out = []
    runs = c["identity"]["runs"]
    cells = {r["cell"] for r in runs}
    out.append(
        ("identity run count", len(runs) == PUBLISHED["identity_runs"], f"{len(runs)}")
    )
    out.append(
        (
            "identity cell count",
            len(cells) == PUBLISHED["identity_cells"],
            f"{len(cells)}",
        )
    )
    out.append(
        (
            "every run matches the t148 reference set",
            all(
                r["sha256"] == r["reference_sha256"] and r["matches_reference"]
                for r in runs
            ),
            f"{sum(1 for r in runs if r['matches_reference'])}/{len(runs)}",
        )
    )
    out.append(
        (
            "supports-on byte sizes publish t148's",
            all(
                r["gcode_bytes"] == PUBLISHED["identity_bytes"][r["cell"]] for r in runs
            ),
            "checked",
        )
    )
    out.append(
        (
            "no degraded runs",
            not c["identity"]["degraded_cells"]
            and all(not r["degraded"] for r in runs),
            str(c["identity"]["degraded_cells"]),
        )
    )
    out.append(
        (
            "no exact-Z rejections",
            c["identity"]["exact_z_rejections_total"] == 0
            and all(r["exact_z_rejections"] == 0 for r in runs),
            str(c["identity"]["exact_z_rejections_total"]),
        )
    )
    return out


def check_ab(c: dict) -> list[tuple[str, bool, str]]:
    out = []
    ab = c["ab"]
    out.append(("all eight cells measured", len(ab) == 8, f"{len(ab)}"))
    for cell, published_median in PUBLISHED["wall_median_pct"].items():
        data = ab[cell]
        per = data["pairs"]
        recomputed_median = statistics.median(p["wall_delta_pct"] for p in per)
        delta_ok = close(recomputed_median, published_median, 0.011)
        floor = -recomputed_median / FLOOR_PCT
        out.append(
            (
                f"{cell}: {data['pair_count']} pairs, wall median {published_median:+.2f}%",
                data["pair_count"] == PUBLISHED["pair_counts"][cell]
                and delta_ok
                and close(floor, data["floor_multiple"], 0.011),
                f"pairs={data['pair_count']} median={recomputed_median:+.4f} floor={floor:.2f}x",
            )
        )
        out.append(
            (
                f"{cell}: CPU median {PUBLISHED['cpu_median_pct'][cell]:+.2f}%",
                close(
                    statistics.median(p["cpu_delta_pct"] for p in per),
                    PUBLISHED["cpu_median_pct"][cell],
                    0.011,
                ),
                f"{statistics.median(p['cpu_delta_pct'] for p in per):+.4f}",
            )
        )
        out.append(
            (
                f"{cell}: output identical in every A/B pair",
                data["sha_identical_in_all_pairs"]
                and all(p["baseline_sha256"] == p["candidate_sha256"] for p in per),
                f"{sum(1 for p in per if p['baseline_sha256'] == p['candidate_sha256'])}/{len(per)}",
            )
        )
        # Every A/B run must still carry a reference-set hash.
        ref = {r["cell"]: r["reference_sha256"] for r in c["identity"]["runs"]}
        out.append(
            (
                f"{cell}: every A/B run carries the reference hash",
                all(
                    p["baseline_sha256"] == ref[cell]
                    and p["candidate_sha256"] == ref[cell]
                    for p in per
                ),
                ref[cell][:12],
            )
        )
    # The gate's literal form: a positive median improvement on the base cells.
    for cell in ("base-classic-off", "base-classic-on"):
        median = ab[cell]["wall_delta_median_pct"]
        out.append(
            (
                f"gate: {cell} median beats the {FLOOR_PCT}% floor",
                median < -FLOOR_PCT,
                f"{median:+.2f}% ({-median / FLOOR_PCT:.2f}x)",
            )
        )
    for cell in (
        "benchy-classic-off",
        "benchy-classic-on",
        "benchy-arachne-off",
        "benchy-arachne-on",
    ):
        median = ab[cell]["wall_delta_median_pct"]
        out.append(
            (f"gate: {cell} median does not regress", median <= 0.0, f"{median:+.2f}%")
        )
    # Disclosure: the benchy classic-off median sits inside the null wall band.
    band = c["null_control"]["wall_band_pct"]
    out.append(
        (
            "disclosure: benchy classic-off median is inside the null wall band",
            band[0] <= ab["benchy-classic-off"]["wall_delta_median_pct"] <= band[1],
            f"{ab['benchy-classic-off']['wall_delta_median_pct']:+.2f}% in [{band[0]:+.2f},{band[1]:+.2f}]",
        )
    )
    return out


def check_null(c: dict) -> list[tuple[str, bool, str]]:
    out = []
    null = c["null_control"]
    per = null["pairs"]
    wall = [p["wall_delta_pct"] for p in per]
    cpu = [p["cpu_delta_pct"] for p in per]
    out.append(("null pair count", len(per) == PUBLISHED["null_pairs"], f"{len(per)}"))
    out.append(
        (
            "null wall band",
            close(min(wall), PUBLISHED["null_wall_band_pct"][0], 0.011)
            and close(max(wall), PUBLISHED["null_wall_band_pct"][1], 0.011),
            f"[{min(wall):+.4f},{max(wall):+.4f}]",
        )
    )
    out.append(
        (
            "null wall median",
            close(statistics.median(wall), PUBLISHED["null_wall_median_pct"], 0.011),
            f"{statistics.median(wall):+.4f}",
        )
    )
    out.append(
        (
            "null CPU band",
            close(min(cpu), PUBLISHED["null_cpu_band_pct"][0], 0.011)
            and close(max(cpu), PUBLISHED["null_cpu_band_pct"][1], 0.011),
            f"[{min(cpu):+.4f},{max(cpu):+.4f}]",
        )
    )
    # Read the CPU clause against the measured band, not against zero.
    strict = 0
    for cell, data in c["ab"].items():
        cpu_median = data["cpu_delta_median_pct"]
        if cpu_median < min(cpu):
            strict += 1
        out.append(
            (
                f"CPU clause: {cell} is not contradicted",
                cpu_median <= max(cpu) and cpu_median <= 0.0,
                f"{cpu_median:+.2f}% vs null max {max(cpu):+.2f}%",
            )
        )
    out.append(
        (
            "disclosure: strict CPU-lower-than-null-edge holds in 7/8 cells",
            strict == 7,
            f"{strict}/8",
        )
    )
    return out


def check_stage(c: dict) -> list[tuple[str, bool, str]]:
    out = []
    for cell, published in PUBLISHED["stage"].items():
        stage = c["stage"][cell]
        pct = (
            (stage["candidate_mesh_analysis_ms"] - stage["baseline_mesh_analysis_ms"])
            / stage["baseline_mesh_analysis_ms"]
            * 100
        )
        out.append(
            (
                f"stage {cell}: {published['baseline']} -> {published['candidate']} ms",
                stage["baseline_mesh_analysis_ms"] == published["baseline"]
                and stage["candidate_mesh_analysis_ms"] == published["candidate"]
                and close(pct, published["delta_pct"], 0.011)
                and stage["delta_ms"]
                == stage["candidate_mesh_analysis_ms"]
                - stage["baseline_mesh_analysis_ms"],
                f"{pct:+.2f}%",
            )
        )
    base_wall = next(
        r["wall_seconds"]
        for r in c["identity"]["runs"]
        if r["cell"] == "base-classic-off" and r["arm"] == "baseline"
    )
    share = -c["stage"]["base-classic-off"]["delta_ms"] / 1000.0 / base_wall * 100
    out.append(
        (
            "stage saving as a share of the base classic-off identity wall",
            close(share, PUBLISHED["stage_share_of_identity_wall_pct"], 0.011),
            f"{share:.2f}% of {base_wall:.2f}s",
        )
    )
    return out


def check_route(c: dict) -> list[tuple[str, bool, str]]:
    out = []
    for cell, (ratio, cut) in PUBLISHED["route"].items():
        route = c["route"][cell]
        delta = c["ab"][cell]["wall_delta_median_pct"] / 100.0
        composed = route["ratio_of_record"] * (1.0 + delta)
        composed_cut = (1.0 - 1.0 / composed) * 100
        out.append(
            (
                f"route {cell}: {ratio:.3f}x / cut {cut:.2f}%",
                close(composed, ratio, 0.0011)
                and close(composed_cut, cut, 0.011)
                and close(route["post_t150_ratio_indicative"], ratio, 0.0011)
                and close(route["required_cut_pct_indicative"], cut, 0.011),
                f"{composed:.4f}x / {composed_cut:.3f}%",
            )
        )
    return out


def check(c: dict) -> list[tuple[str, bool, str]]:
    return (
        check_identity(c)
        + check_ab(c)
        + check_null(c)
        + check_stage(c)
        + check_route(c)
    )


def run(census: dict) -> bool:
    results = check(census)
    failed = [r for r in results if not r[1]]
    for name, ok, detail in results:
        print(f"{'PASS' if ok else 'FAIL'}  {name}  [{detail}]")
    print(f"\n{len(results) - len(failed)}/{len(results)} checks passed")
    return not failed


def self_test(census: dict) -> int:
    mutations = []

    def mutate(description, fn):
        c = copy.deepcopy(census)
        fn(c)
        mutations.append((description, c))

    mutate(
        "flip one identity hash",
        lambda c: c["identity"]["runs"][0].__setitem__("sha256", "00" * 32),
    )

    def shift_pair(c):
        c["ab"]["base-classic-off"]["pairs"][0]["candidate_wall_s"] *= 1.5
        pairs = c["ab"]["base-classic-off"]["pairs"]
        base = c["ab"]["base-classic-off"]
        per = []
        for p in pairs:
            per.append(
                (p["candidate_wall_s"] - p["baseline_wall_s"])
                / p["baseline_wall_s"]
                * 100
            )
        base["wall_delta_median_pct"] = round(statistics.median(per), 4)
        base["floor_multiple"] = round(-base["wall_delta_median_pct"] / FLOOR_PCT, 4)

    mutate("move a pair so the base classic-off median crosses zero", shift_pair)

    def break_null(c):
        c["null_control"]["pairs"][2]["wall_delta_pct"] = -12.0

    mutate("make one null pair a 12% outlier", break_null)

    mutation_stage = lambda c: c["stage"]["base-classic-on"].__setitem__(
        "candidate_mesh_analysis_ms", 1
    )
    mutate("change the base classic-on candidate stage time", mutation_stage)

    mutation_route = lambda c: c["route"]["base-classic-off"].__setitem__(
        "post_t150_ratio_indicative", 1.0
    )
    mutate("change the base classic-off composed ratio", mutation_route)

    rejected = 0
    for description, mutated in mutations:
        ok = run_quiet(mutated)
        status = "REJECTED" if not ok else "NOT DETECTED"
        if not ok:
            rejected += 1
        print(f"{status}: {description}")
    print(f"\n{rejected}/{len(mutations)} negative controls rejected")
    return 0 if rejected == len(mutations) else 1


def run_quiet(census: dict) -> bool:
    return all(ok for _, ok, _ in check(census))


def main(argv: list[str]) -> int:
    census = json.loads(CENSUS.read_text(encoding="utf-8"))
    if "--self-test" in argv:
        return self_test(census)
    return 0 if run(census) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

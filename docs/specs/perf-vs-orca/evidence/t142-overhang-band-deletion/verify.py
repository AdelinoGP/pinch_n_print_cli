#!/usr/bin/env python3
"""Verify the committed t142 evidence without the gitignored raw tree.

Re-derives every headline number from ``reduced/campaign.json``:

  * output byte-equality in all 8 cells, and the t139 reference set unmoved;
  * paired medians, wall saving %, CPU medians and the CPU corroboration;
  * the 0.80 % paired floor in all 8 cells;
  * the recorded gates.

``--self-test`` runs four negative controls against mutated copies of the
payload and requires each one to be rejected.
"""

from __future__ import annotations

import argparse
import copy
import json
import statistics
from pathlib import Path
from typing import Callable

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

T139_REFERENCE = {
    "benchy-classic-off": "93e21fee",
    "benchy-classic-on": "b3f1eccc",
    "benchy-arachne-off": "3510e876",
    "benchy-arachne-on": "96cdac85",
    "base-classic-off": "ca14d055",
    "base-classic-on": "585c6a8e",
    "base-arachne-off": "d8fdc1d9",
    "base-arachne-on": "c9fee59a",
}


def median(values: list[float]) -> float:
    return statistics.median(values)


def check(payload: dict) -> list[str]:
    """Return a list of failures; an empty list means the evidence holds."""
    failures: list[str] = []
    ab = payload.get("ab", {})
    output = payload.get("output", {})
    gates = payload.get("gates", {})
    floor = float(payload.get("floor_pct", 0.80))

    for cell in CELLS:
        if cell not in ab:
            failures.append(f"{cell}: missing A/B evidence")
            continue
        if cell not in output:
            failures.append(f"{cell}: missing output evidence")
            continue

        arms = {"baseline-ordinary": [], "candidate-ordinary": []}
        for row in ab[cell]["rows"]:
            arms[row["arm"]].append(row)
        if not arms["baseline-ordinary"] or not arms["candidate-ordinary"]:
            failures.append(f"{cell}: missing one arm in the paired rows")
            continue
        baseline_wall = median(
            [row["wall_seconds"] for row in arms["baseline-ordinary"]]
        )
        candidate_wall = median(
            [row["wall_seconds"] for row in arms["candidate-ordinary"]]
        )
        baseline_cpu = median([row["cpu_seconds"] for row in arms["baseline-ordinary"]])
        candidate_cpu = median(
            [row["cpu_seconds"] for row in arms["candidate-ordinary"]]
        )

        saving_pct = (1.0 - candidate_wall / baseline_wall) * 100.0
        recorded = float(ab[cell]["wall_saving_pct"])
        if abs(saving_pct - recorded) > 1e-3:
            failures.append(
                f"{cell}: recorded saving {recorded} != re-derived {saving_pct:.4f}"
            )
        if saving_pct < floor:
            failures.append(
                f"{cell}: saving {saving_pct:.3f}% below the {floor}% floor"
            )
        if candidate_cpu >= baseline_cpu:
            failures.append(f"{cell}: CPU does not corroborate the wall saving")
        if any(
            row["wall_seconds"] <= 0.0 or row["cpu_seconds"] <= 0.0
            for row in ab[cell]["rows"]
        ):
            failures.append(f"{cell}: non-positive timing row")

        cell_output = output[cell]
        if not cell_output.get("identical"):
            failures.append(f"{cell}: baseline/candidate G-code differs")
        sha = cell_output.get("candidate_sha256", "")
        if not sha.startswith(T139_REFERENCE[cell]):
            failures.append(
                f"{cell}: reference hash {sha[:12]} != t139 {T139_REFERENCE[cell]}"
            )
        if int(cell_output.get("gcode_bytes", 0)) <= 0:
            failures.append(f"{cell}: empty G-code")

    accelerated = payload.get("accelerated_output") or {}
    for cell in CELLS:
        if not accelerated:
            break
        entry = accelerated.get(cell, {})
        if entry.get("sha256") != output[cell].get("candidate_sha256"):
            failures.append(f"{cell}: accelerated output differs from the ordinary candidate")
        if not entry.get("matches_candidate_ordinary"):
            failures.append(f"{cell}: accelerated parity flag is false")

    for name, passed in gates.items():
        if passed is not True:
            failures.append(f"gate {name} is not true")
    if len(ab) != len(CELLS) or len(output) != len(CELLS):
        failures.append("cell count is not 8")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--evidence",
        type=Path,
        default=Path(
            "docs/specs/perf-vs-orca/evidence/t142-overhang-band-deletion/reduced/campaign.json"
        ),
    )
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    payload = json.loads(args.evidence.read_text(encoding="utf-8"))
    failures = check(payload)
    if failures:
        for failure in failures:
            print(f"FAIL {failure}")
        return 1
    print(
        "PASS t142 evidence: 8/8 byte-identical, 8/8 above the 0.80% floor, CPU corroborates"
    )

    if not args.self_test:
        return 0

    controls: list[tuple[str, Callable[[dict], None]]] = [
        (
            "flipped candidate hash",
            lambda p: p["output"]["base-classic-off"].__setitem__(
                "candidate_sha256", "0" * 64
            ),
        ),
        (
            "candidate slower than baseline",
            lambda p: p["ab"]["base-classic-off"]["rows"].append(
                {
                    "pair": 99,
                    "arm": "candidate-ordinary",
                    "wall_seconds": p["ab"]["base-classic-off"][
                        "baseline_wall_median_s"
                    ]
                    * 1.5,
                    "cpu_seconds": p["ab"]["base-classic-off"][
                        "candidate_cpu_median_s"
                    ],
                    "sha256": "0" * 64,
                }
            ),
        ),
        (
            "CPU inversion",
            lambda p: p["ab"]["base-classic-on"].__setitem__(
                "rows",
                [
                    dict(row, cpu_seconds=row["cpu_seconds"] * 10.0)
                    if row["arm"] == "candidate-ordinary"
                    else row
                    for row in p["ab"]["base-classic-on"]["rows"]
                ],
            ),
        ),
        (
            "missing cell",
            lambda p: p["ab"].pop("benchy-classic-off"),
        ),
    ]
    rejected = 0
    for label, mutate in controls:
        mutated = copy.deepcopy(payload)
        mutate(mutated)
        if check(mutated):
            print(f"SELFTEST PASS (rejected) {label}")
            rejected += 1
        else:
            print(f"SELFTEST FAIL (accepted) {label}")
    if rejected != len(controls):
        return 1
    print(f"SELFTEST PASS {rejected}/{len(controls)} negative controls rejected")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

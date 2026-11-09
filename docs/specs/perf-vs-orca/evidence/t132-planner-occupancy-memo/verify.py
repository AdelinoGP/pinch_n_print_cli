#!/usr/bin/env python3
"""Verify the ticket-132 evidence set from the committed summary alone.

Every check is a claim the ticket makes. The verifier re-derives the claims
from `reduced/summary.json`; it never re-runs the campaign.

    python verify.py            # exit 0 on PASS
    python verify.py --self-test  # exit 0 only if the negative controls are rejected
"""

import argparse
import copy
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "reduced" / "summary.json"

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


class Checker:
    def __init__(self):
        self.checks = 0
        self.failures = []

    def check(self, condition, message):
        self.checks += 1
        if not condition:
            self.failures.append(message)


def verify(summary):
    c = Checker()
    cells = summary["cells"] if "cells" in summary else CELLS
    output = summary["output"]
    ab = summary["ab"]
    gate = summary["gate"]
    equality = summary["equality"]
    reference = summary["reference_check"]
    manifest = summary["arms"]
    floor_fraction = summary["floor_fraction"]

    c.check(floor_fraction == 0.008, "floor fraction must be 0.80 %")
    c.check(set(cells) == set(CELLS), "summary must cover all eight cells")

    # ── Output identity ──────────────────────────────────────────────────
    for cell in CELLS:
        arms = output[cell]
        c.check(set(arms) == set(ARMS), f"{cell}: four arms required")
        shas = {arm: arms[arm]["sha256"] for arm in ARMS}
        c.check(
            len(set(shas.values())) == 1,
            f"{cell}: all four arms must emit one G-code hash, got {sorted(set(shas.values()))}",
        )
        bytes_set = {arms[arm]["gcode_bytes"] for arm in ARMS}
        c.check(len(bytes_set) == 1, f"{cell}: arms must emit one byte count")
        c.check(
            all(arms[arm]["exit_code"] == 0 for arm in ARMS),
            f"{cell}: every arm must exit 0",
        )
        row = reference[cell]
        c.check(
            row["t123_candidate_sha256"] == shas["candidate-ordinary"],
            f"{cell}: reference must equal the candidate arm hash",
        )
        c.check(
            row["reproduces"], f"{cell}: every arm must reproduce t123's reference hash"
        )
        c.check(
            equality[cell]["all_arms_identical"], f"{cell}: equality entry must agree"
        )
        c.check(
            equality[cell]["sha256"] == [row["t123_candidate_sha256"]],
            f"{cell}: equality sha set must be the reference singleton",
        )

    # ── Manifest ─────────────────────────────────────────────────────────
    c.check(
        manifest["baseline-ordinary"]["exe_sha256"]
        == manifest["candidate-ordinary"]["exe_sha256"],
        "ordinary arms must share one host binary (guest-only change)",
    )
    c.check(
        manifest["baseline-accelerated"]["exe_sha256"]
        == manifest["candidate-accelerated"]["exe_sha256"],
        "accelerated arms must share one host binary (guest-only change)",
    )
    c.check(
        manifest["baseline-ordinary"]["planner_wasm_sha256"]
        != manifest["candidate-ordinary"]["planner_wasm_sha256"],
        "the ordinary arms must carry different planner guests",
    )
    c.check(
        manifest["baseline-accelerated"]["planner_wasm_sha256"]
        != manifest["candidate-accelerated"]["planner_wasm_sha256"],
        "the accelerated arms must carry different planner guests",
    )

    # ── Paired statistics and gate ───────────────────────────────────────
    for cell in CELLS:
        row = ab[cell]
        c.check(
            row["runs_per_arm"] >= 3,
            f"{cell}: at least three pairs per arm",
        )
        c.check(
            len(row["baseline_wall_seconds"]) == row["runs_per_arm"]
            and len(row["candidate_wall_seconds"]) == row["runs_per_arm"],
            f"{cell}: one wall sample per arm per recorded pair",
        )
        c.check(
            len(row["baseline_cpu_seconds"]) == row["runs_per_arm"]
            and len(row["candidate_cpu_seconds"]) == row["runs_per_arm"],
            f"{cell}: one CPU sample per arm per recorded pair",
        )
        c.check(
            row["baseline_sha256"]
            == row["candidate_sha256"]
            == [reference[cell]["t123_candidate_sha256"]],
            f"{cell}: paired arms must reproduce the reference hash",
        )
        c.check(
            row["exit_codes"] == [0],
            f"{cell}: paired runs must all exit 0",
        )
        # Re-derive the median delta and the floor from the raw samples.
        import statistics

        base_median = statistics.median(row["baseline_wall_seconds"])
        cand_median = statistics.median(row["candidate_wall_seconds"])
        c.check(
            abs(base_median - row["baseline_median_wall_seconds"]) < 1e-9,
            f"{cell}: baseline median must re-derive from the samples",
        )
        c.check(
            abs(cand_median - row["candidate_median_wall_seconds"]) < 1e-9,
            f"{cell}: candidate median must re-derive from the samples",
        )
        delta = base_median - cand_median
        c.check(
            abs(delta - row["median_wall_delta_seconds"]) < 1e-9,
            f"{cell}: median wall delta must re-derive from the medians",
        )
        floor = floor_fraction * base_median
        c.check(
            abs(floor - row["floor_seconds"]) < 1e-9,
            f"{cell}: floor must be 0.80 % of the baseline median",
        )
        c.check(
            abs(row["floor_multiple"] - delta / floor) < 1e-6,
            f"{cell}: floor multiple must re-derive from delta / floor",
        )
        c.check(
            abs(row["median_wall_delta_fraction"] - delta / base_median) < 1e-12,
            f"{cell}: delta fraction must re-derive from the medians",
        )
        c.check(
            row["supports_on"] == (cell in SUPPORTS_ON),
            f"{cell}: supports-on flag must match the cell name",
        )

        gate_row = gate["cells"][cell]
        if cell in SUPPORTS_ON:
            expected = (
                "PASS"
                if (
                    row["floor_multiple"] >= 1.0
                    and delta > 0.0
                    and row["cpu_delta_median_seconds"] >= 0.0
                )
                else "FAIL"
            )
        else:
            expected = (
                "PASS"
                if (delta >= 0.0 and row["cpu_delta_median_seconds"] >= 0.0)
                else "FAIL"
            )
        c.check(
            gate_row["verdict"] == expected,
            f"{cell}: gate verdict must follow the amended supports-on / supports-off rule",
        )

    passed = sum(1 for cell in CELLS if gate["cells"][cell]["verdict"] == "PASS")
    c.check(gate["passed"] == passed, "gate passed count must re-derive")
    c.check(gate["total"] == len(CELLS), "gate total must cover all eight cells")

    return c


def negative_controls(summary):
    """Each mutation must make the verifier fail."""
    mutations = []

    def mutate(name, fn):
        bad = copy.deepcopy(summary)
        fn(bad)
        result = verify(bad)
        mutations.append((name, len(result.failures) > 0))

    def flip_arm_hash(s):
        s["output"]["base-classic-on"]["candidate-accelerated"]["sha256"] = "0" * 64

    def flip_reference(s):
        s["reference_check"]["benchy-classic-off"]["t123_candidate_sha256"] = "0" * 64

    def flip_gate(s):
        s["gate"]["cells"]["base-classic-on"]["verdict"] = "FAIL"

    def flip_floor_multiple(s):
        s["ab"]["base-arachne-on"]["floor_multiple"] += 1.0

    def flip_median(s):
        s["ab"]["benchy-arachne-off"]["candidate_median_wall_seconds"] += 1.0

    def flip_exit(s):
        s["output"]["base-classic-off"]["baseline-accelerated"]["exit_code"] = 1

    def flip_same_guest(s):
        s["arms"]["candidate-ordinary"]["planner_wasm_sha256"] = s["arms"][
            "baseline-ordinary"
        ]["planner_wasm_sha256"]

    def flip_pairs(s):
        s["ab"]["base-classic-on"]["runs_per_arm"] = 2

    def flip_floor_fraction(s):
        s["floor_fraction"] = 0.005

    def flip_equality(s):
        s["equality"]["benchy-classic-on"]["all_arms_identical"] = False

    def flip_ab_hash(s):
        s["ab"]["base-arachne-off"]["candidate_sha256"] = ["0" * 64]

    for name, fn in [
        ("arm hash", flip_arm_hash),
        ("reference hash", flip_reference),
        ("gate verdict", flip_gate),
        ("floor multiple", flip_floor_multiple),
        ("median", flip_median),
        ("exit code", flip_exit),
        ("same planner guest", flip_same_guest),
        ("pair count", flip_pairs),
        ("floor fraction", flip_floor_fraction),
        ("equality flag", flip_equality),
        ("paired hash", flip_ab_hash),
    ]:
        mutate(name, fn)
    return mutations


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--summary", default=str(SUMMARY))
    args = parser.parse_args()

    summary = json.loads(Path(args.summary).read_text(encoding="utf-8"))
    result = verify(summary)
    for failure in result.failures:
        print(f"FAIL: {failure}")

    if args.self_test:
        mutations = negative_controls(summary)
        rejected = [name for name, ok in mutations if ok]
        survived = [name for name, ok in mutations if not ok]
        for name in survived:
            print(f"SELF-TEST SURVIVED: {name}")
        if survived or result.failures:
            print(f"FAIL: {len(rejected)}/{len(mutations)} negative controls rejected")
            return 1
        print(
            f"PASS: {result.checks} checks, {len(rejected)}/{len(mutations)} negative controls rejected"
        )
        return 0

    if result.failures:
        print(f"FAIL: {len(result.failures)} of {result.checks} checks failed")
        return 1
    print(f"PASS: {result.checks} checks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

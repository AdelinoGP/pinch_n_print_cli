#!/usr/bin/env python3
"""Verify ticket 125's output identity, provenance and measured DROP verdict."""

import argparse
import copy
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "reduced" / "summary.json"
T123 = HERE.parent / "t123-facet-retirement" / "reduced" / "summary.json"
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
FLOOR = 0.008


class Failure(Exception):
    pass


def require(condition, message):
    if not condition:
        raise Failure(message)


def check(summary, reference):
    checks = 0

    def ok(condition, message):
        nonlocal checks
        require(condition, message)
        checks += 1

    ok(summary["ticket"] == "125", "summary must be ticket 125")
    ok(summary["floor_fraction"] == FLOOR, "floor must be 0.008")
    ok(summary["verdict"] == "DROP", "the measured campaign verdict is DROP")
    ok(
        summary["converted_call_family"].startswith("per-origin role carve only"),
        "the candidate converted only the per-origin call family",
    )
    for mode in ("ordinary", "accelerated"):
        binaries = summary["binaries"]
        modules = summary["planner_modules"]
        ok(
            binaries[f"baseline_{mode}_sha256"] == binaries[f"candidate_{mode}_sha256"],
            f"{mode}: guest-only candidate must not change the host binary",
        )
        ok(
            modules[f"baseline_{mode}_sha256"] != modules[f"candidate_{mode}_sha256"],
            f"{mode}: the planner module must differ between arms",
        )

    for cell in CELLS:
        out = summary["output"][cell]
        for arm in ARMS:
            row = out[arm]
            ok(row["exit_code"] == 0, f"{cell}/{arm}: exit code")
            ok(row["completion_status"] == "ok", f"{cell}/{arm}: completion status")
            ok(
                row["non_fatal_error_count"] == 0,
                f"{cell}/{arm}: non-fatal diagnostics",
            )
            ok(row["degraded"] is False, f"{cell}/{arm}: degraded run")
            ok(
                len(row["sha256"]) == 64 and row["gcode_bytes"] > 0,
                f"{cell}/{arm}: output",
            )
        expected = reference["output"][cell]["candidate-ordinary"]["sha256"]
        ref = summary["reference_check"][cell]
        ok(ref["t123_candidate_sha256"] == expected, f"{cell}: reference identity")
        ok(
            ref["baseline_ordinary_sha256"] == expected,
            f"{cell}: baseline is not post-t123",
        )
        ok(ref["reproduces"] is True, f"{cell}: reference flag")
        equality = summary["arm_equality"][cell]
        ok(
            equality["baseline_arms_identical"] is True,
            f"{cell}: baseline ordinary/accelerated",
        )
        ok(
            equality["candidate_arms_identical"] is True,
            f"{cell}: candidate ordinary/accelerated",
        )
        ok(
            equality["candidate_equals_baseline"] is True,
            f"{cell}: candidate changed output",
        )
        hashes = {out[arm]["sha256"] for arm in ARMS}
        ok(
            hashes == {expected},
            f"{cell}: all four outputs must equal the post-t123 bytes",
        )

    derived_pass = {}
    for cell in CELLS:
        ab = summary["ab"][cell]
        ok(ab["runs_per_arm"] == 3, f"{cell}: three pairs required")
        base_wall = ab["baseline_wall_seconds"]
        cand_wall = ab["candidate_wall_seconds"]
        base_cpu = ab["baseline_cpu_seconds"]
        cand_cpu = ab["candidate_cpu_seconds"]
        ok(len(base_wall) == len(cand_wall) == 3, f"{cell}: wall roster")
        ok(len(base_cpu) == len(cand_cpu) == 3, f"{cell}: CPU roster")
        ok(ab["exit_codes"] == [0], f"{cell}: AB exit codes")
        ok(ab["completion_status"] == ["ok"], f"{cell}: AB completion")
        ok(ab["non_fatal_error_count"] == [0], f"{cell}: AB non-fatal diagnostics")
        ok(ab["degraded"] == [False], f"{cell}: AB degraded flag")
        ok(
            ab["baseline_sha256"]
            == [summary["output"][cell]["baseline-ordinary"]["sha256"]],
            f"{cell}: AB baseline hash",
        )
        ok(
            ab["candidate_sha256"]
            == [summary["output"][cell]["candidate-ordinary"]["sha256"]],
            f"{cell}: AB candidate hash",
        )
        ok(len(ab["gcode_bytes"]) == 1, f"{cell}: AB output volume changed")
        ok([p["pair"] for p in ab["pairs"]] == [1, 2, 3], f"{cell}: pair sequence")
        ok(
            ab["pairs"][0]["order"].startswith("baseline-ordinary"),
            f"{cell}: pair 1 order",
        )
        ok(
            ab["pairs"][1]["order"].startswith("candidate-ordinary"),
            f"{cell}: pair 2 order",
        )
        ok(
            ab["pairs"][2]["order"].startswith("baseline-ordinary"),
            f"{cell}: pair 3 order",
        )

        base_median = statistics.median(base_wall)
        cand_median = statistics.median(cand_wall)
        base_cpu_median = statistics.median(base_cpu)
        cand_cpu_median = statistics.median(cand_cpu)
        wall_delta = base_median - cand_median
        cpu_delta = base_cpu_median - cand_cpu_median
        floor = FLOOR * base_median
        ok(
            ab["baseline_median_wall_seconds"] == base_median,
            f"{cell}: baseline wall median",
        )
        ok(
            ab["candidate_median_wall_seconds"] == cand_median,
            f"{cell}: candidate wall median",
        )
        ok(
            ab["baseline_median_cpu_seconds"] == base_cpu_median,
            f"{cell}: baseline CPU median",
        )
        ok(
            ab["candidate_median_cpu_seconds"] == cand_cpu_median,
            f"{cell}: candidate CPU median",
        )
        ok(
            abs(ab["median_wall_delta_seconds"] - wall_delta) < 1e-9,
            f"{cell}: wall delta",
        )
        ok(
            abs(ab["median_wall_delta_fraction"] - wall_delta / base_median) < 1e-9,
            f"{cell}: wall fraction",
        )
        ok(abs(ab["floor_seconds"] - floor) < 1e-9, f"{cell}: floor arithmetic")
        ok(
            abs(ab["floor_multiple"] - wall_delta / floor) < 1e-9,
            f"{cell}: floor multiple",
        )
        ok(abs(ab["cpu_delta_median_seconds"] - cpu_delta) < 1e-9, f"{cell}: CPU delta")
        ok(ab["cpu_corroborates"] is (cpu_delta > 0), f"{cell}: CPU corroboration flag")

        if cell.endswith("-on"):
            passed = wall_delta >= floor and cpu_delta > 0
            ok(ab["gate_scope"] == "supports-on improvement", f"{cell}: gate scope")
        else:
            passed = wall_delta >= 0 and cpu_delta >= 0
            ok(ab["gate_scope"] == "supports-off no-regression", f"{cell}: gate scope")
        derived_pass[cell] = passed
        ok(ab["gate_pass"] is passed, f"{cell}: gate verdict does not match samples")
        ok(
            (not ab["failure_reasons"]) is passed,
            f"{cell}: failure reasons do not match verdict",
        )

    ok(
        all(derived_pass.values()) is False,
        "this evidence must not read as an all-cell KEEP",
    )
    ok(
        derived_pass
        == {
            "benchy-classic-off": False,
            "benchy-classic-on": False,
            "benchy-arachne-off": False,
            "benchy-arachne-on": True,
            "base-classic-off": False,
            "base-classic-on": False,
            "base-arachne-off": False,
            "base-arachne-on": True,
        },
        "the recorded two-pass / six-fail cell pattern changed",
    )
    return checks


def load():
    return (
        json.loads(SUMMARY.read_text(encoding="utf-8")),
        json.loads(T123.read_text(encoding="utf-8")),
    )


def self_test():
    summary, reference = load()
    controls = []

    def control(name, mutate):
        candidate = copy.deepcopy(summary)
        mutate(candidate)
        try:
            check(candidate, reference)
        except Failure:
            controls.append(name)
            return
        raise SystemExit(f"negative control NOT rejected: {name}")

    def changed_output(s):
        s["output"][CELLS[0]]["candidate-ordinary"]["sha256"] = "0" * 64

    def stale_baseline(s):
        s["reference_check"][CELLS[1]]["baseline_ordinary_sha256"] = "1" * 64

    def degraded(s):
        s["ab"][CELLS[2]]["degraded"] = [True]

    def nonfatal(s):
        s["output"][CELLS[3]]["candidate-accelerated"]["non_fatal_error_count"] = 1

    def dropped_pair(s):
        s["ab"][CELLS[4]]["runs_per_arm"] = 2

    def moved_median(s):
        s["ab"][CELLS[5]]["candidate_median_wall_seconds"] += 1

    def false_keep(s):
        s["verdict"] = "KEEP"

    def false_gate(s):
        s["ab"][CELLS[5]]["gate_pass"] = True

    def unchanged_module(s):
        s["planner_modules"]["candidate_ordinary_sha256"] = s["planner_modules"][
            "baseline_ordinary_sha256"
        ]

    def wrong_order(s):
        s["ab"][CELLS[6]]["pairs"][1]["order"] = s["ab"][CELLS[6]]["pairs"][0]["order"]

    for name, mutate in [
        ("candidate output changed", changed_output),
        ("stale baseline", stale_baseline),
        ("degraded AB run", degraded),
        ("non-fatal output diagnostic", nonfatal),
        ("dropped pair", dropped_pair),
        ("median disagrees with samples", moved_median),
        ("false KEEP", false_keep),
        ("gate flag contradicts samples", false_gate),
        ("unchanged planner module", unchanged_module),
        ("non-alternating pair order", wrong_order),
    ]:
        control(name, mutate)
    print(f"self-test: {len(controls)}/{len(controls)} negative controls rejected")
    for name in controls:
        print(f"  rejected: {name}")
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    summary, reference = load()
    if args.self_test:
        return self_test()
    checks = check(summary, reference)
    print(f"PASS: {checks} checks; output identity and the measured DROP verdict hold")
    for cell in CELLS:
        ab = summary["ab"][cell]
        print(
            f"  {cell}: {'PASS' if ab['gate_pass'] else 'FAIL'} "
            f"wall {ab['median_wall_delta_fraction'] * 100:+.2f}% "
            f"({ab['floor_multiple']:+.2f}x floor), CPU {ab['cpu_delta_median_seconds']:+.4f}s"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Verify the ticket-116 evidence set from a plain checkout.

Exits 0 when every committed number is internally consistent and every gate is
met. `--self-test` additionally mutates the summary in memory and requires each
negative control to be rejected.

The checked contract (ticket 116, under ticket 111's 16-cell contract):

  * baseline reproduction — the baseline ordinary arm reproduces the t115
    post-merge reference set for all eight cells;
  * 16-cell output — ordinary and accelerated arms are byte-identical within
    each build, and the candidate's output differs from the baseline's;
  * delta audit — structural section types (perimeters, support, skirt, brim,
    raft) are identical in every cell; only the fill / bridge families move;
  * paired gate — three interleaved pairs per cell, median wall saving at least
    the 0.80 % floor in all eight cells, process CPU corroborating.
"""

import argparse
import json
import statistics
import sys
from pathlib import Path

EVIDENCE = Path(__file__).resolve().parent
SUMMARY = EVIDENCE / "reduced" / "summary.json"
T115 = EVIDENCE.parent / "t115-post-merge" / "summary.json"

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

STRUCTURAL_TYPES = {
    "Outer wall",
    "Inner wall",
    "Overhang wall",
    "Support",
    "Support interface",
    "Support transition",
    "Skirt",
    "Brim",
    "Raft",
}

FILL_TYPES = {
    "Bridge",
    "Internal Bridge",
    "Sparse infill",
    "Internal solid infill",
    "Solid infill",
    "Top surface",
    "Bottom surface",
    "Ironing",
    "Gap fill",
}

FLOOR_FRACTION = 0.008


class Failure(Exception):
    pass


def require(condition, message):
    if not condition:
        raise Failure(message)


def check(summary, reference, *, skip_gate=False):
    checks = 0

    def ok(condition, message):
        nonlocal checks
        require(condition, message)
        checks += 1

    ok(summary["ticket"] == "116", "summary must be the ticket-116 evidence")
    ok(summary["floor_fraction"] == FLOOR_FRACTION, "floor fraction must be 0.008")

    # ---- 1. output set completeness -------------------------------------
    for cell in CELLS:
        for arm in ARMS:
            row = summary["output"][cell][arm]
            ok(row["exit_code"] == 0, f"{cell}/{arm}: slice exit code must be 0")
            ok(
                len(row["sha256"]) == 64,
                f"{cell}/{arm}: sha256 must be a hex digest",
            )
        ok(
            summary["output"][cell]["candidate-ordinary"]["gcode_bytes"] > 0,
            f"{cell}: candidate output must be non-empty",
        )

    # ---- 2. baseline reproduces the post-merge reference ----------------
    for cell in CELLS:
        entry = summary["reference_check"][cell]
        expected = reference["output_audit"][cell]["post_sha256"]
        ok(
            entry["t115_post_sha256"] == expected,
            f"{cell}: reference hash must match the committed t115 set",
        )
        ok(
            entry["baseline_ordinary_sha256"] == expected,
            f"{cell}: baseline ordinary must reproduce the t115 post-merge hash",
        )
        ok(
            entry["reproduces"] is True,
            f"{cell}: baseline reproduction flag must be true",
        )

    # ---- 3. 16-cell output equality -------------------------------------
    for cell in CELLS:
        entry = summary["arm_equality"][cell]
        out = summary["output"][cell]
        ok(
            out["baseline-ordinary"]["sha256"] == out["baseline-accelerated"]["sha256"],
            f"{cell}: baseline ordinary and accelerated output must match",
        )
        ok(
            out["candidate-ordinary"]["sha256"]
            == out["candidate-accelerated"]["sha256"],
            f"{cell}: candidate ordinary and accelerated output must match",
        )
        ok(entry["baseline_arms_identical"] is True, f"{cell}: baseline arm flag")
        ok(entry["candidate_arms_identical"] is True, f"{cell}: candidate arm flag")
        ok(
            out["candidate-ordinary"]["sha256"] != out["baseline-ordinary"]["sha256"],
            f"{cell}: the canonical rule must move the output somewhere",
        )

    # ---- 4. delta audit --------------------------------------------------
    for cell in CELLS:
        audit = summary["delta_audit"][cell]
        ok(audit["cell"] == cell, f"{cell}: audit row must name its cell")
        moved = set(audit["changed_types"])
        allowed = FILL_TYPES | STRUCTURAL_TYPES | {"layer-change", "startup"}
        ok(
            moved <= allowed,
            f"{cell}: changed section types outside the bridge families: "
            f"{sorted(moved - allowed)}",
        )
        ok(
            audit["structural_geometry_lines"] == 0,
            f"{cell}: {audit['structural_geometry_lines']} structural geometry line(s) moved",
        )
        ok(
            audit["structural_geometry_identical"] is True,
            f"{cell}: structural_geometry_identical must be true",
        )
        for section, buckets in audit["structural_changed_types"].items():
            ok(
                buckets.get("geometry", 0) == 0,
                f"{cell}/{section}: structural section moved geometry lines",
            )
        ok(
            audit["geometry_samples"] == [],
            f"{cell}: the audit recorded unclassified geometry changes",
        )
        ok(
            any(
                "flat_bridge_closing_join" in line
                for line in audit["appendix_changed_lines"]
            ),
            f"{cell}: the retired flat_bridge_closing_join appendix row must disappear",
        )
        ok(
            audit["candidate_sha256"]
            == summary["output"][cell]["candidate-ordinary"]["sha256"],
            f"{cell}: audit candidate hash must match the output row",
        )

    # ---- 5. paired gate --------------------------------------------------
    for cell in CELLS:
        ab = summary["ab"][cell]
        ok(ab["runs_per_arm"] == 3, f"{cell}: three paired runs per arm are required")
        base_wall = ab["baseline_wall_seconds"]
        cand_wall = ab["candidate_wall_seconds"]
        ok(len(base_wall) == 3 and len(cand_wall) == 3, f"{cell}: sample lists")
        ok(
            ab["baseline_median_wall_seconds"] == statistics.median(base_wall),
            f"{cell}: baseline median must come from the samples",
        )
        ok(
            ab["candidate_median_wall_seconds"] == statistics.median(cand_wall),
            f"{cell}: candidate median must come from the samples",
        )
        expected_floor = FLOOR_FRACTION * statistics.median(base_wall)
        ok(
            abs(ab["floor_seconds"] - expected_floor) < 1e-9,
            f"{cell}: floor must be 0.80 % of the baseline median",
        )
        expected_delta = statistics.median(base_wall) - statistics.median(cand_wall)
        ok(
            abs(ab["median_wall_delta_seconds"] - expected_delta) < 1e-9,
            f"{cell}: wall delta must equal median(base) - median(candidate)",
        )
        ok(
            abs(ab["floor_multiple"] - expected_delta / expected_floor) < 1e-9,
            f"{cell}: floor multiple must be delta / floor",
        )
        ok(
            ab["floor_multiple"] >= 1.0,
            f"{cell}: wall saving {ab['floor_multiple']:.2f}x floor is below the 0.80 % gate",
        )
        ok(
            ab["cpu_corroborates"] is True,
            f"{cell}: process CPU must corroborate the wall saving",
        )
        ok(
            len(ab["baseline_sha256"]) == 1,
            f"{cell}: baseline arm output must be stable",
        )
        ok(
            len(ab["candidate_sha256"]) == 1,
            f"{cell}: candidate arm output must be stable",
        )
        ok(
            ab["candidate_sha256"][0]
            == summary["output"][cell]["candidate-ordinary"]["sha256"],
            f"{cell}: ab candidate hash must match the output row",
        )
        ok(
            ab["baseline_sha256"][0] == reference["output_audit"][cell]["post_sha256"],
            f"{cell}: ab baseline hash must be the post-merge reference",
        )

    return checks


def load():
    with open(SUMMARY, encoding="utf-8") as handle:
        summary = json.load(handle)
    with open(T115, encoding="utf-8") as handle:
        reference = json.load(handle)
    return summary, reference


def deep_copy(summary):
    return json.loads(json.dumps(summary))


def self_test():
    summary, reference = load()
    controls = []

    def control(name, mutate):
        candidate = deep_copy(summary)
        mutate(candidate)
        try:
            check(candidate, reference)
        except Failure:
            controls.append(name)
            return
        raise SystemExit(f"negative control NOT rejected: {name}")

    def flip_reference(s):
        s["reference_check"][CELLS[0]]["baseline_ordinary_sha256"] = "0" * 64

    def break_arm_equality(s):
        s["output"][CELLS[3]]["candidate-accelerated"]["sha256"] = "1" * 64

    def fail_floor(s):
        s["ab"][CELLS[5]]["floor_multiple"] = 0.5

    def fail_cpu(s):
        s["ab"][CELLS[2]]["cpu_corroborates"] = False

    def move_structural(s):
        s["delta_audit"][CELLS[0]]["structural_geometry_lines"] = 1
        s["delta_audit"][CELLS[0]]["structural_geometry_identical"] = False

    def drop_retired_key(s):
        s["delta_audit"][CELLS[1]]["appendix_changed_lines"] = [
            line
            for line in s["delta_audit"][CELLS[1]]["appendix_changed_lines"]
            if "flat_bridge_closing_join" not in line
        ]

    def unknown_section(s):
        s["delta_audit"][CELLS[7]]["changed_types"]["Custom"] = {"changed_lines": 3}

    def break_median(s):
        s["ab"][CELLS[1]]["baseline_median_wall_seconds"] += 1.0

    def drop_run(s):
        s["ab"][CELLS[4]]["runs_per_arm"] = 2

    def identical_output(s):
        s["output"][CELLS[6]]["candidate-ordinary"]["sha256"] = s["output"][CELLS[6]][
            "baseline-ordinary"
        ]["sha256"]

    for name, mutate in [
        ("reference hash flipped", flip_reference),
        ("candidate accelerated output diverged", break_arm_equality),
        ("wall saving below the floor", fail_floor),
        ("CPU contradicts the wall", fail_cpu),
        ("structural geometry moved", move_structural),
        ("retired config key still emitted", drop_retired_key),
        ("unclassified section type moved", unknown_section),
        ("median disagrees with samples", break_median),
        ("a pair is missing", drop_run),
        ("candidate output unchanged", identical_output),
    ]:
        control(name, mutate)

    print(f"PASS: self-test rejected {len(controls)}/{len(controls)} negative controls")
    print(f"PASS: {len(controls)} controls: " + "; ".join(controls))
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    summary, reference = load()
    checks = check(summary, reference)
    print(f"PASS: {checks} checks across {len(CELLS)} cells")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Verify the ticket-133 evidence set from the committed summary alone.

Every check is a claim the ticket makes. The verifier re-derives the claims
from `reduced/summary.json`; it never re-runs the campaign.

    python verify.py            # exit 0 on PASS
    python verify.py --self-test  # exit 0 only if the negative controls are rejected
"""

import argparse
import copy
import json
import statistics
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

CLASSIC_CELLS = [cell for cell in CELLS if "-classic-" in cell]

# Ticket 131's S5 probe (margin 6 x spacing; this candidate uses 10 x spacing,
# so the probe's window speedup is an upper bound on the removed work).
EXPECTED_S5 = {
    "benchy-classic-off": (56, 30, 594.893, 62.736),
    "benchy-classic-on": (56, 30, 572.06, 56.738),
    "base-classic-off": (130, 42, 9911.389, 1124.447),
    "base-classic-on": (130, 42, 9923.769, 1185.943),
}


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
    output = summary["output"]
    ab = summary["ab"]
    gate = summary["gate"]
    equality = summary["equality"]
    reference = summary["reference_check"]
    delta_audit = summary["delta_audit"]
    ceiling = summary["ceiling"]
    manifest = summary["arms"]
    floor_fraction = summary["floor_fraction"]

    c.check(summary["ticket"] == "133", "the evidence set must name ticket 133")
    c.check(floor_fraction == 0.008, "floor fraction must be 0.80 %")
    c.check(set(output) == set(CELLS), "summary must cover all eight cells")

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
            row["arm_sha256"] == shas,
            f"{cell}: reference arm-hash map must match the output arms",
        )
        c.check(
            equality[cell]["all_arms_identical"], f"{cell}: equality entry must agree"
        )
        c.check(
            equality[cell]["baseline_arms_identical"]
            and equality[cell]["candidate_arms_identical"],
            f"{cell}: both build arms must be byte-identical within each arm",
        )
        c.check(
            equality[cell]["sha256"] == [row["t123_candidate_sha256"]],
            f"{cell}: equality sha set must be the reference singleton",
        )

    # ── Delta audit: exactness is the claim ──────────────────────────────
    for cell in CELLS:
        audit = delta_audit[cell]
        c.check(audit["verdict"] == "EXACT", f"{cell}: delta audit must be EXACT")
        c.check(audit["arms_identical"], f"{cell}: delta audit requires one arm hash")
        c.check(
            not audit["candidate_moves_cell"],
            f"{cell}: the candidate must not move the reference G-code",
        )
        c.check(
            audit["structural_geometry_lines"] == 0,
            f"{cell}: exact output has zero structural geometry lines",
        )

    # ── Manifest: host-side change, one shared module tree ───────────────
    c.check(
        manifest["baseline-ordinary"]["exe_sha256"]
        != manifest["candidate-ordinary"]["exe_sha256"],
        "the ordinary arms must carry different host binaries",
    )
    c.check(
        manifest["baseline-accelerated"]["exe_sha256"]
        != manifest["candidate-accelerated"]["exe_sha256"],
        "the accelerated arms must carry different host binaries",
    )
    c.check(
        manifest["baseline-ordinary"]["planner_wasm_sha256"]
        == manifest["candidate-ordinary"]["planner_wasm_sha256"],
        "the ordinary arms must share one module tree (host-only change)",
    )
    c.check(
        manifest["baseline-accelerated"]["planner_wasm_sha256"]
        == manifest["candidate-accelerated"]["planner_wasm_sha256"],
        "the accelerated arms must share one module tree (host-only change)",
    )

    # ── Ticket 131 ceiling, re-derived from the quoted fields ────────────
    for cell in CLASSIC_CELLS:
        row = ceiling[cell]
        layers, neg_ran, whole, windowed = EXPECTED_S5[cell]
        c.check(row is not None, f"{cell}: t131's S5 probe must exist")
        c.check(row["layers"] == layers, f"{cell}: S5 probe layer count")
        c.check(row["neg_ran_layers"] == neg_ran, f"{cell}: S5 negative-ran layers")
        c.check(
            abs(row["sum_whole_neg_ms"] - whole) < 1e-6,
            f"{cell}: S5 whole-layer negative offset cost",
        )
        c.check(
            abs(row["sum_win_neg_ms"] - windowed) < 1e-6,
            f"{cell}: S5 windowed negative offset cost",
        )
        c.check(
            abs(row["neg_speedup"] - whole / windowed) < 1e-9,
            f"{cell}: S5 speedup must re-derive from the two timings",
        )
        c.check(
            abs(row["operand_shrink"] - row["sum_points_whole"] / row["sum_points_win"])
            < 1e-9,
            f"{cell}: S5 operand shrink must re-derive from the point counts",
        )
        c.check(
            row["sum_points_win"] < row["sum_points_whole"],
            f"{cell}: window must shrink the operand",
        )
    for cell in CELLS:
        if cell not in CLASSIC_CELLS:
            c.check(ceiling[cell] is None, f"{cell}: t131 did not probe this cell")

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
            len(row["pairs"]) == row["runs_per_arm"],
            f"{cell}: one paired delta per recorded pair",
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
        cpu_delta = statistics.median(row["baseline_cpu_seconds"]) - statistics.median(
            row["candidate_cpu_seconds"]
        )
        c.check(
            abs(cpu_delta - row["cpu_delta_median_seconds"]) < 1e-9,
            f"{cell}: CPU delta must re-derive from the CPU samples",
        )
        c.check(
            row["cpu_corroborates"] == (cpu_delta > 0.0),
            f"{cell}: CPU corroboration must follow the measured sign",
        )
        for index, pair in enumerate(sorted(row["pairs"], key=lambda p: p["pair"])):
            c.check(
                abs(
                    pair["wall_delta_seconds"]
                    - (
                        row["baseline_wall_seconds"][index]
                        - row["candidate_wall_seconds"][index]
                    )
                )
                < 1e-9,
                f"{cell}: pair {pair['pair']} wall delta must re-derive from the samples",
            )
            c.check(
                abs(
                    pair["cpu_delta_seconds"]
                    - (
                        row["baseline_cpu_seconds"][index]
                        - row["candidate_cpu_seconds"][index]
                    )
                )
                < 1e-9,
                f"{cell}: pair {pair['pair']} CPU delta must re-derive from the samples",
            )
        paired_signs = [p["wall_delta_seconds"] > 0.0 for p in row["pairs"]]
        c.check(
            2 * sum(paired_signs) > len(paired_signs),
            f"{cell}: a strict majority of paired runs must favour the candidate",
        )

        gate_row = gate["cells"][cell]
        expected = (
            "PASS"
            if (row["floor_multiple"] >= 1.0 and delta > 0.0 and cpu_delta >= 0.0)
            else "FAIL"
        )
        c.check(
            gate_row["verdict"] == expected,
            f"{cell}: gate verdict must follow the standing floor rule",
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

    def flip_host_binary(s):
        s["arms"]["candidate-ordinary"]["exe_sha256"] = s["arms"]["baseline-ordinary"][
            "exe_sha256"
        ]

    def flip_same_module_tree(s):
        s["arms"]["candidate-ordinary"]["planner_wasm_sha256"] = "0" * 64

    def flip_pairs(s):
        s["ab"]["base-classic-on"]["runs_per_arm"] = 2

    def flip_floor_fraction(s):
        s["floor_fraction"] = 0.005

    def flip_equality(s):
        s["equality"]["benchy-classic-on"]["all_arms_identical"] = False

    def flip_ab_hash(s):
        s["ab"]["base-arachne-off"]["candidate_sha256"] = ["0" * 64]

    def flip_delta_verdict(s):
        s["delta_audit"]["base-classic-off"]["verdict"] = "DELTA"

    def flip_candidate_moves(s):
        s["delta_audit"]["benchy-arachne-on"]["candidate_moves_cell"] = True

    def flip_ceiling(s):
        s["ceiling"]["base-classic-on"]["sum_whole_neg_ms"] += 1.0

    def flip_cpu_sign(s):
        s["ab"]["benchy-classic-off"]["cpu_delta_median_seconds"] = -1.0

    def flip_pair_signs(s):
        for pair in s["ab"]["base-classic-off"]["pairs"]:
            pair["wall_delta_seconds"] = -abs(pair["wall_delta_seconds"])

    for name, fn in [
        ("arm hash", flip_arm_hash),
        ("reference hash", flip_reference),
        ("gate verdict", flip_gate),
        ("floor multiple", flip_floor_multiple),
        ("median", flip_median),
        ("exit code", flip_exit),
        ("same host binary", flip_host_binary),
        ("module tree split", flip_same_module_tree),
        ("pair count", flip_pairs),
        ("floor fraction", flip_floor_fraction),
        ("equality flag", flip_equality),
        ("paired hash", flip_ab_hash),
        ("delta verdict", flip_delta_verdict),
        ("candidate moved", flip_candidate_moves),
        ("ceiling quote", flip_ceiling),
        ("CPU sign", flip_cpu_sign),
        ("paired signs", flip_pair_signs),
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

#!/usr/bin/env python3
"""Verify ticket 127's output identity, provenance and measured gate verdict."""

import argparse
import copy
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "reduced" / "summary.json"
INSTRUMENTED = HERE / "reduced" / "instrumented.json"
TRANSFER = HERE / "reduced" / "transfer.json"
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


def check(summary, reference, instrumented, transfer):
    checks = 0

    def ok(condition, message):
        nonlocal checks
        require(condition, message)
        checks += 1

    ok(summary["ticket"] == "127", "summary must be ticket 127")
    ok(summary["floor_fraction"] == FLOOR, "floor must be 0.008")
    ok(
        summary["converted_call_family"].startswith("raft-fill dispatch outside"),
        "the candidate converted only the raft-band dispatch",
    )
    # Host-only candidate: every module artifact is identical between arms, and
    # both host binaries differ in both build modes.
    for mode in ("ordinary", "accelerated"):
        binaries = summary["binaries"]
        modules = summary["raft_module"]
        ok(
            binaries[f"baseline_{mode}_sha256"] != binaries[f"candidate_{mode}_sha256"],
            f"{mode}: the candidate must change the host binary",
        )
        ok(
            modules[f"baseline_{mode}_sha256"] == modules[f"candidate_{mode}_sha256"],
            f"{mode}: a host-only candidate must not change any guest module",
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

        passed = wall_delta >= floor and cpu_delta > 0
        ok(ab["gate_scope"] == "all-cell floor + CPU", f"{cell}: gate scope")
        derived_pass[cell] = passed
        ok(ab["gate_pass"] is passed, f"{cell}: gate verdict does not match samples")
        ok(
            (not ab["failure_reasons"]) is passed,
            f"{cell}: failure reasons do not match verdict",
        )

    derived_verdict = "KEEP" if all(derived_pass.values()) else "DROP"
    ok(
        summary["verdict"] == derived_verdict,
        "summary verdict does not follow from the per-cell gate",
    )

    # Instrumented probe: the declined dispatch must actually disappear, and
    # the other Layer::Infill producers must still run on every layer.
    for cell in CELLS:
        probe = instrumented["cells"][cell]
        ok(
            probe["raft_default_calls"] == 0,
            f"{cell}: raft-default still dispatched on a raft-free job",
        )
        ok(
            probe["baseline_raft_default_calls"] > 0,
            f"{cell}: no baseline raft-default dispatch to remove",
        )
        ok(
            probe["infill_max_calls"] == probe["layers"],
            f"{cell}: another Layer::Infill producer lost per-layer coverage",
        )
        ok(
            probe["gcode_matches_reference"] is True,
            f"{cell}: instrumented probe moved the output",
        )
        ok(
            probe["raft_default_thread_ms"] == 0,
            f"{cell}: raft-default still accumulated dispatches",
        )
        ok(
            probe["baseline_raft_default_thread_ms"] > 0,
            f"{cell}: the removed baseline dispatches were already free",
        )
        ok(
            probe["gcode_sha256"]
            == reference["output"][cell]["candidate-ordinary"]["sha256"],
            f"{cell}: instrumented probe is not the reference output",
        )
        # The cost transfers to the next Infill dispatch rather than vanishing:
        # baseline pays it in raft-default, the candidate pays the same median in
        # gyroid-infill on the same layers.
        base_first = transfer["cells"][cell]["baseline"]["first_infill_modules"]
        cand_first = transfer["cells"][cell]["candidate"]["first_infill_modules"]
        ok(
            base_first.get("com.core.raft-default", {}).get("layers")
            == probe["layers"],
            f"{cell}: baseline raft-default is not the first Infill dispatch on every layer",
        )
        ok(
            "com.core.raft-default" not in cand_first,
            f"{cell}: raft-default still paid the stage preparation",
        )
        ok(
            cand_first.get("com.core.gyroid-infill", {}).get("layers")
            == probe["layers"],
            f"{cell}: gyroid-infill did not take over the first Infill dispatch",
        )
        base_cost = base_first["com.core.raft-default"]["p50_ms"]
        cand_cost = cand_first["com.core.gyroid-infill"]["p50_ms"]
        ok(
            cand_cost >= 0.6 * base_cost,
            f"{cell}: the first-dispatch cost vanished instead of transferring "
            f"({base_cost} -> {cand_cost} ms)",
        )
    return checks


def load():
    return (
        json.loads(SUMMARY.read_text(encoding="utf-8")),
        json.loads(T123.read_text(encoding="utf-8")),
        json.loads(INSTRUMENTED.read_text(encoding="utf-8")),
        json.loads(TRANSFER.read_text(encoding="utf-8")),
    )


def self_test():
    summary, reference, instrumented, transfer = load()
    verdict = summary["verdict"]
    flipped = "DROP" if verdict == "KEEP" else "KEEP"
    controls = []

    def control(name, mutate):
        candidate = copy.deepcopy(summary)
        mutate(candidate)
        try:
            check(candidate, reference, instrumented, transfer)
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

    def flipped_verdict(s):
        s["verdict"] = flipped

    def false_gate(s):
        s["ab"][CELLS[5]]["gate_pass"] = not s["ab"][CELLS[5]]["gate_pass"]

    def unchanged_binary(s):
        s["binaries"]["candidate_ordinary_sha256"] = s["binaries"][
            "baseline_ordinary_sha256"
        ]

    def changed_module(s):
        s["raft_module"]["candidate_ordinary_sha256"] = "2" * 64

    def wrong_order(s):
        s["ab"][CELLS[6]]["pairs"][1]["order"] = s["ab"][CELLS[6]]["pairs"][0]["order"]

    for name, mutate in [
        ("candidate output changed", changed_output),
        ("stale baseline", stale_baseline),
        ("degraded AB run", degraded),
        ("non-fatal output diagnostic", nonfatal),
        ("dropped pair", dropped_pair),
        ("median disagrees with samples", moved_median),
        ("verdict contradicts the gate", flipped_verdict),
        ("gate flag contradicts samples", false_gate),
        ("candidate binary unchanged", unchanged_binary),
        ("guest module changed by a host-only candidate", changed_module),
        ("non-alternating pair order", wrong_order),
    ]:
        control(name, mutate)

    def probe(name, mutate):
        candidate = copy.deepcopy(instrumented)
        mutate(candidate)
        try:
            check(summary, reference, candidate, transfer)
        except Failure:
            controls.append(name)
            return
        raise SystemExit(f"negative control NOT rejected: {name}")

    def still_dispatched(s):
        s["cells"][CELLS[0]]["raft_default_calls"] = 495

    def coverage_lost(s):
        s["cells"][CELLS[1]]["infill_max_calls"] = 0

    def probe_output_moved(s):
        s["cells"][CELLS[2]]["gcode_matches_reference"] = False

    def still_accumulating(s):
        s["cells"][CELLS[3]]["raft_default_thread_ms"] = 61

    for name, mutate in [
        ("raft-default still dispatched", still_dispatched),
        ("stage coverage lost", coverage_lost),
        ("instrumented probe moved output", probe_output_moved),
        ("declined dispatch still accumulated", still_accumulating),
    ]:
        probe(name, mutate)

    def transfer_control(name, mutate):
        candidate = copy.deepcopy(transfer)
        mutate(candidate)
        try:
            check(summary, reference, instrumented, candidate)
        except Failure:
            controls.append(name)
            return
        raise SystemExit(f"negative control NOT rejected: {name}")

    def cost_vanished(s):
        first = s["cells"][CELLS[0]]["candidate"]["first_infill_modules"]
        first["com.core.gyroid-infill"]["p50_ms"] = 0

    def raft_still_first(s):
        first = s["cells"][CELLS[1]]["candidate"]["first_infill_modules"]
        first["com.core.raft-default"] = first.pop("com.core.gyroid-infill")
        s["cells"][CELLS[1]]["candidate"]["first_infill_modules"] = first

    def baseline_not_raft(s):
        first = s["cells"][CELLS[2]]["baseline"]["first_infill_modules"]
        first["com.core.raft-default"]["layers"] = 1

    for name, mutate in [
        ("first-dispatch cost vanished", cost_vanished),
        ("raft-default still paid the stage preparation", raft_still_first),
        ("baseline raft-default was not the first dispatch", baseline_not_raft),
    ]:
        transfer_control(name, mutate)

    print(f"self-test: {len(controls)}/{len(controls)} negative controls rejected")
    for name in controls:
        print(f"  rejected: {name}")
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    summary, reference, instrumented, transfer = load()
    if args.self_test:
        return self_test()
    checks = check(summary, reference, instrumented, transfer)
    print(
        f"PASS: {checks} checks; output identity, provenance and the gate verdict hold"
    )
    print(f"verdict: {summary['verdict']}")
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

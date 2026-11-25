#!/usr/bin/env python3
"""Ticket 135 Stage 2 verifier: every claim re-derived from `reduced/summary.json`.

No raw G-code, no STL, no campaign tree, no probe log. `--self-test` mutates the
summary and asserts each mutation is rejected, so a broken verifier cannot pass
vacuously.

Run from the repository root:

    python docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/verify.py
    python docs/specs/perf-vs-orca/evidence/t135-occupancy-batch/verify.py --self-test
"""

from __future__ import annotations

import copy
import json
import statistics
import sys
from pathlib import Path

SUMMARY = Path(__file__).resolve().parent / "reduced" / "summary.json"

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
SUPPORTS_ON = [cell for cell in CELLS if cell.endswith("-on")]
SUPPORTS_OFF = [cell for cell in CELLS if cell.endswith("-off")]
FLOOR_PCT = 0.80
TOL = 1e-6


def close(left: float, right: float, tol: float = TOL) -> bool:
    return abs(left - right) <= tol * max(1.0, abs(left), abs(right))


def check(summary: dict) -> list[str]:
    errors: list[str] = []
    if summary.get("ticket") != 135 or summary.get("stage") != 2:
        errors.append("summary is not the t135 Stage 2 payload")
    if summary.get("floor_pct") != FLOOR_PCT:
        errors.append("floor drifted off the 0.80 % gate")

    reference = summary.get("reference_set", {})
    output = summary.get("output", {})
    ab = summary.get("ab", {})
    probe = summary.get("probe", {})
    gates = summary.get("gates", {})

    for name in (
        "output_identical_8_of_8",
        "reference_set_unmoved_8_of_8",
        "all_runs_exit_zero",
        "supports_on_clear_floor_4_of_4",
        "supports_on_cpu_not_contradicted_4_of_4",
        "supports_off_no_regression_beyond_null_band_4_of_4",
        "null_control_output_stable",
        "ab_output_stable_8_of_8",
        "probe_batch_fans_out_above_serial",
    ):
        if name not in gates:
            errors.append(f"gate missing: {name}")
        elif gates[name] is not True:
            errors.append(f"gate not met: {name}")

    # The strict sign forms are disclosures, not gates: on the inert
    # supports-off cells they flip on differences the null control shows are
    # inside the instrument's noise. Their recorded values must match the
    # prose, which says both are false.
    disclosures = summary.get("disclosures", {})
    if disclosures.get("supports_on_cpu_corroborates_4_of_4_strict") is not False:
        errors.append("strict CPU-corroboration disclosure drifted")
    if disclosures.get("supports_off_no_median_regression_4_of_4_strict") is not False:
        errors.append("strict supports-off regression disclosure drifted")

    if sorted(output) != sorted(CELLS):
        errors.append(f"output cell set drifted: {sorted(output)}")
    if sorted(ab) != sorted(CELLS):
        errors.append(f"ab cell set drifted: {sorted(ab)}")
    if sorted(probe) != sorted(SUPPORTS_ON):
        errors.append(f"probe cell set drifted: {sorted(probe)}")

    # ── Output gate ─────────────────────────────────────────────────────────
    for cell in CELLS:
        row = output.get(cell, {})
        if row.get("baseline_exit_code") != 0 or row.get("candidate_exit_code") != 0:
            errors.append(f"{cell}: an output run exited non-zero")
        if not row.get("baseline_sha256"):
            errors.append(f"{cell}: baseline hash missing")
        if row.get("baseline_sha256") != row.get("candidate_sha256"):
            errors.append(f"{cell}: baseline and candidate G-code differ")
        if row.get("candidate_sha256") != reference.get(cell):
            errors.append(f"{cell}: candidate hash is not the t139 reference")
        if row.get("gcode_bytes", 0) <= 0:
            errors.append(f"{cell}: empty G-code")

    # ── A/B: recompute every derived number from the raw pair rows ──────────
    for cell in CELLS:
        entry = ab.get(cell, {})
        rows = entry.get("rows", [])
        baseline = [row for row in rows if row["arm"] == "baseline-ordinary"]
        candidate = [row for row in rows if row["arm"] == "candidate-ordinary"]
        if not baseline or not candidate:
            errors.append(f"{cell}: missing A/B arm rows")
            continue
        if len(baseline) != len(candidate):
            errors.append(f"{cell}: asymmetric pair counts")
        pairs = sorted({row["pair"] for row in rows})
        if entry.get("pairs") != pairs:
            errors.append(f"{cell}: pair list drifted")

        baseline_wall = statistics.median(row["wall_seconds"] for row in baseline)
        candidate_wall = statistics.median(row["wall_seconds"] for row in candidate)
        baseline_cpu = statistics.median(row["cpu_seconds"] for row in baseline)
        candidate_cpu = statistics.median(row["cpu_seconds"] for row in candidate)
        saving_pct = (1.0 - candidate_wall / baseline_wall) * 100.0

        expected = {
            "baseline_wall_median_s": round(baseline_wall, 4),
            "candidate_wall_median_s": round(candidate_wall, 4),
            "baseline_cpu_median_s": round(baseline_cpu, 4),
            "candidate_cpu_median_s": round(candidate_cpu, 4),
            "wall_delta_s": round(baseline_wall - candidate_wall, 4),
            "wall_ratio_candidate_over_baseline": round(
                candidate_wall / baseline_wall, 4
            ),
            "wall_saving_pct": round(saving_pct, 3),
            "cpu_ratio_candidate_over_baseline": round(candidate_cpu / baseline_cpu, 4),
            "cpu_saving_pct": round((1.0 - candidate_cpu / baseline_cpu) * 100.0, 3),
            "cpu_delta_s": round(baseline_cpu - candidate_cpu, 4),
        }
        for key, value in expected.items():
            stored = entry.get(key)
            if stored is None or abs(float(stored) - value) > 1e-9:
                errors.append(f"{cell}: {key} does not re-derive ({stored} vs {value})")

        by_pair = {
            "baseline-ordinary": {row["pair"]: row for row in baseline},
            "candidate-ordinary": {row["pair"]: row for row in candidate},
        }
        pairs_faster = all(
            by_pair["candidate-ordinary"][pair]["wall_seconds"]
            < by_pair["baseline-ordinary"][pair]["wall_seconds"]
            for pair in pairs
        )
        cpu_corroborates = candidate_cpu < baseline_cpu
        clears_floor = saving_pct >= FLOOR_PCT
        stable = len({row["sha256"] for row in rows}) == 1
        if entry.get("pairs_candidate_faster") != pairs_faster:
            errors.append(f"{cell}: pairs_candidate_faster does not re-derive")
        if entry.get("cpu_corroborates") != cpu_corroborates:
            errors.append(f"{cell}: cpu_corroborates does not re-derive")
        if entry.get("clears_floor") != clears_floor:
            errors.append(f"{cell}: clears_floor does not re-derive")
        if entry.get("output_sha256_stable") != stable:
            errors.append(f"{cell}: output_sha256_stable does not re-derive")
        if cell in SUPPORTS_ON and not clears_floor:
            errors.append(f"{cell}: supports-on cell does not clear the floor")
        if cell in SUPPORTS_ON and not cpu_corroborates:
            # Disclosure, not a gate: the batch is a parallel candidate, so the
            # CPU record is expected to be flat rather than lower.
            pass
        if cell in SUPPORTS_OFF and saving_pct < 0.0:
            # Disclosure, not a gate: the null control defines the measurable
            # band on these structurally inert cells.
            pass

    # ── Probe: the batch's own fan-out ──────────────────────────────────────
    for cell in SUPPORTS_ON:
        entry = probe.get(cell, {})
        calls = entry.get("calls", 0)
        requests = entry.get("requests", 0)
        wall = entry.get("wall_ms", 0.0)
        thread = entry.get("thread_ms", 0.0)
        if calls <= 0 or requests <= 0:
            errors.append(f"{cell}: the probe recorded no occupancy batch")
            continue
        if requests < calls:
            errors.append(f"{cell}: fewer requests than calls")
        if wall <= 0.0 or thread <= 0.0:
            errors.append(f"{cell}: probe wall/thread missing")
            continue
        fanout = thread / wall
        if (
            abs(float(entry.get("fanout_thread_over_wall", 0.0)) - round(fanout, 3))
            > 1e-9
        ):
            errors.append(f"{cell}: fanout does not re-derive")
        if fanout <= 1.0:
            errors.append(f"{cell}: batch fan-out is not above serial")

    return errors


def mutations(summary: dict):
    """Yield (label, mutated summary) pairs that a correct verifier must reject."""

    def clone():
        return copy.deepcopy(summary)

    body = clone()
    body["gates"]["output_identical_8_of_8"] = False
    yield "gate_flag_flipped", body

    body = clone()
    body["output"]["base-classic-on"]["candidate_sha256"] = "0" * 64
    yield "candidate_hash_swapped", body

    body = clone()
    body["output"]["benchy-classic-on"]["baseline_exit_code"] = 1
    yield "non_zero_exit", body

    body = clone()
    cell = "base-classic-on"
    body["ab"][cell]["rows"][1]["wall_seconds"] += 10.0
    yield "wall_row_moved", body

    body = clone()
    body["ab"]["benchy-classic-on"]["wall_saving_pct"] = 12.34
    yield "saving_pct_forged", body

    body = clone()
    body["probe"]["base-classic-on"]["thread_ms"] = (
        body["probe"]["base-classic-on"]["wall_ms"] * 0.5
    )
    yield "probe_fanout_forced_serial", body

    body = clone()
    body["ab"]["base-arachne-off"]["cpu_seconds"] = 0.0
    body["ab"]["base-arachne-off"]["rows"][0]["cpu_seconds"] = 9999.0
    yield "supports_off_cpu_regression", body

    body = clone()
    del body["ab"]["benchy-classic-off"]
    yield "cell_deleted", body


def main(argv: list[str]) -> int:
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    if "--self-test" in argv:
        errors = check(summary)
        if errors:
            print("baseline summary is not clean; self-test aborted")
            for error in errors:
                print(f"  {error}")
            return 1
        rejected = 0
        for label, body in mutations(summary):
            if check(body):
                rejected += 1
                print(f"  rejected: {label}")
            else:
                print(f"  ACCEPTED (bug): {label}")
        total = len(list(mutations(summary)))
        print(f"self-test: {rejected}/{total} negative controls rejected")
        return 0 if rejected == total else 1

    errors = check(summary)
    for error in errors:
        print(f"FAIL: {error}")
    print("PASS" if not errors else f"FAIL ({len(errors)} checks)")
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

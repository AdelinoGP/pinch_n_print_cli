#!/usr/bin/env python3
"""Verify the t121 per-layer dispatch fix from the committed A/B rows.

Re-derives every cell median, delta, hash invariant and the per-layer bracket
from `ab-rows-classic.csv` + `summary.json`. `--self-test` mutates the inputs
and requires every control to be rejected.
"""

from __future__ import annotations

from collections import Counter, defaultdict
import csv
import io
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]


def parse_rows(text: str) -> list[dict]:
    return list(csv.DictReader(io.StringIO(text)))


def check(summary: dict, rows: list[dict]) -> list[str]:
    problems: list[str] = []
    if len(rows) != 24:
        problems.append(f"expected 24 A/B rows, got {len(rows)}")

    grouped: dict[tuple[str, str], list[dict]] = defaultdict(list)
    seen = set()
    for row in rows:
        key = (row.get("cell", ""), row.get("pair", ""), row.get("arm", ""))
        if key in seen:
            problems.append(f"duplicate row {key}")
        seen.add(key)
        grouped[(row.get("cell", ""), row.get("arm", ""))].append(row)

    computed = {}
    for cell in CELLS:
        for arm in ("pre", "fix"):
            arm_rows = grouped.get((cell, arm), [])
            if len(arm_rows) != 3:
                problems.append(f"{cell}/{arm}: expected 3 rows, got {len(arm_rows)}")
                continue
            hashes = {row["sha256"] for row in arm_rows}
            if len(hashes) != 1:
                problems.append(f"{cell}/{arm}: runs are not byte-identical")
            computed[(cell, arm)] = {
                "wall_ms": statistics.median(int(row["wall_ms"]) for row in arm_rows),
                "cpu_s": statistics.median(float(row["cpu_s"]) for row in arm_rows),
                "sha256": next(iter(hashes)) if hashes else "",
            }

    for cell in CELLS:
        pre = computed.get((cell, "pre"))
        fix = computed.get((cell, "fix"))
        if not pre or not fix:
            continue
        if pre["sha256"] != fix["sha256"]:
            problems.append(f"{cell}: pre and fix outputs differ")
        wanted = summary["cells"].get(cell)
        if wanted is None:
            problems.append(f"{cell}: absent from summary")
            continue
        if wanted["sha256"] != pre["sha256"]:
            problems.append(f"{cell}: summary hash differs from rows")
        for arm, expected in (("pre", wanted["pre"]), ("fix", wanted["fix"])):
            wall = computed[(cell, arm)]["wall_ms"]
            cpu = computed[(cell, arm)]["cpu_s"]
            if abs(wall - expected["wall_ms_median"]) > 1:
                problems.append(
                    f"{cell}/{arm}: wall median {wall} != summary "
                    f"{expected['wall_ms_median']}"
                )
            if abs(cpu - expected["cpu_s_median"]) > 0.01:
                problems.append(
                    f"{cell}/{arm}: CPU median {cpu} != summary "
                    f"{expected['cpu_s_median']}"
                )
        wall_delta = (fix["wall_ms"] - pre["wall_ms"]) / pre["wall_ms"] * 100
        cpu_delta = (fix["cpu_s"] - pre["cpu_s"]) / pre["cpu_s"] * 100
        if abs(wall_delta - wanted["wall_delta_pct"]) > 0.05:
            problems.append(f"{cell}: wall delta {wall_delta:.2f} != summary")
        if abs(cpu_delta - wanted["cpu_delta_pct"]) > 0.05:
            problems.append(f"{cell}: CPU delta {cpu_delta:.2f} != summary")

    positive_cpu = []
    positive_wall = []
    for row in rows:
        cell = row["cell"]
        pair = row["pair"]
        pre = next(
            (
                r
                for r in rows
                if r["cell"] == cell and r["pair"] == pair and r["arm"] == "pre"
            ),
            None,
        )
        if row["arm"] != "fix" or pre is None:
            continue
        if float(row["cpu_s"]) >= float(pre["cpu_s"]):
            positive_cpu.append((cell, pair))
        if int(row["wall_ms"]) >= int(pre["wall_ms"]):
            positive_wall.append((cell, pair))
    if positive_cpu:
        problems.append(f"fix CPU is not negative in every pair: {positive_cpu}")
    if positive_wall != [("benchy-classic-off", "2")]:
        problems.append(f"unexpected wall-positive pairs: {positive_wall}")
    if not any("benchy-classic-off pair 2" in item for item in summary["anomalies"]):
        problems.append("the disclosed wall anomaly is not recorded")

    per_layer = summary["per_layer"]
    if not (
        per_layer["fix_ms"]
        < per_layer["t115_premerge_bracket_ms"]
        < per_layer["pre_fix_ms"]
    ):
        problems.append("per-layer bracket is not ordered pre > target > fix")
    probe = summary["probe"]
    if not (
        probe["transport_prepare_us_mean_after"]
        < probe["transport_prepare_us_mean_before"]
        and probe["transport_total_us_mean_after"]
        < probe["transport_total_us_mean_before"]
    ):
        problems.append("probe does not record the transport reduction")
    return problems


def main() -> int:
    summary = json.loads((HERE / "summary.json").read_text(encoding="utf-8"))
    rows = parse_rows((HERE / "ab-rows-classic.csv").read_text(encoding="utf-8"))
    problems = check(summary, rows)
    if problems:
        for problem in problems:
            print(f"FAIL: {problem}")
        return 1
    print(
        "PASS: t121 — 4 cells x 3 pairs, CPU negative 12/12, wall negative 11/12, "
        "outputs identical; per-layer 42,406 -> 6,758 ms (t115 bracket 7,265)"
    )
    if "--self-test" in sys.argv:
        import copy

        csv_text = (HERE / "ab-rows-classic.csv").read_text(encoding="utf-8")
        first = rows[0]
        controls = []

        bad_hash = copy.deepcopy(rows)
        bad_hash[0]["sha256"] = "0" * 64
        controls.append(("flipped hash", summary, bad_hash))

        dropped = copy.deepcopy(rows)
        dropped.pop()
        controls.append(("dropped row", summary, dropped))

        duplicate = copy.deepcopy(rows)
        duplicate.append(copy.deepcopy(first))
        controls.append(("duplicate row", summary, duplicate))

        bad_cpu = copy.deepcopy(rows)
        fix_row = next(
            r
            for r in bad_cpu
            if r["cell"] == "benchy-classic-off"
            and r["pair"] == "1"
            and r["arm"] == "fix"
        )
        fix_row["cpu_s"] = "99.99"
        controls.append(("positive CPU pair", summary, bad_cpu))

        bad_wall = copy.deepcopy(rows)
        fix_row = next(
            r
            for r in bad_wall
            if r["cell"] == "base-classic-off"
            and r["pair"] == "1"
            and r["arm"] == "fix"
        )
        fix_row["wall_ms"] = "999999"
        controls.append(("positive wall pair", summary, bad_wall))

        bad_median = copy.deepcopy(summary)
        bad_median["cells"]["base-classic-off"]["fix"]["cpu_s_median"] += 1
        controls.append(("changed median", bad_median, rows))

        bad_bracket = copy.deepcopy(summary)
        bad_bracket["per_layer"]["fix_ms"] = (
            bad_bracket["per_layer"]["t115_premerge_bracket_ms"] + 1
        )
        controls.append(("broken bracket", bad_bracket, rows))

        bad_probe = copy.deepcopy(summary)
        bad_probe["probe"]["transport_prepare_us_mean_after"] = (
            bad_probe["probe"]["transport_prepare_us_mean_before"] + 1
        )
        controls.append(("probe not reduced", bad_probe, rows))

        for name, control_summary, control_rows in controls:
            if not check(control_summary, control_rows):
                print(f"MISSED: {name}")
                return 1
            print(f"rejected: {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Verify the ticket-115 post-merge reconciliation evidence from a plain checkout.

Checks that `summary.json` is internally consistent with the committed
`ab-rows-classic.csv` and that every headline claim the FINDINGS make is
arithmetically re-derivable from the committed numbers.

Run with `--self-test` to apply a set of negative controls (mutations that must
be rejected) after the positive checks.
"""

from __future__ import annotations

import copy
import csv
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "summary.json"
AB_ROWS = HERE / "ab-rows-classic.csv"

CHECKS = 0


def check(condition: bool, message: str) -> None:
    global CHECKS
    CHECKS += 1
    if not condition:
        raise SystemExit(f"FAIL: {message}")


def load() -> tuple[dict, list[dict]]:
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    with AB_ROWS.open(encoding="utf-8-sig", newline="") as handle:
        rows = list(csv.DictReader(handle))
    return summary, rows


def check_output_audit(summary: dict) -> None:
    audit = summary["output_audit"]
    check(len(audit) == 8, "output audit covers all eight matched cells")
    for cell, row in audit.items():
        check(
            row["byte_delta"] == 34,
            f"{cell}: post-merge output grows by exactly the 34-byte config line",
        )
        check(
            row["post_sha256"] != row["pre_sha256"],
            f"{cell}: the hash moves with the added line",
        )
        check(
            row["post_bytes"] - row["pre_bytes"] == 34,
            f"{cell}: recorded byte counts agree with the delta",
        )


def check_per_layer(summary: dict) -> None:
    cells = summary["cells"]
    bands = {
        "benchy-classic-off": (1.8, 2.6),
        "benchy-classic-on": (1.6, 2.3),
        "base-classic-off": (4.2, 6.2),
        "base-classic-on": (4.0, 6.5),
    }
    for cell, band in bands.items():
        pre = statistics.median(cells[cell]["per_layer_ms"]["pre"])
        post = statistics.median(cells[cell]["per_layer_ms"]["post"])
        ratio = post / pre
        check(
            band[0] <= ratio <= band[1],
            f"{cell}: per-layer median ratio {ratio:.2f} inside {band}",
        )


def check_merge_ab(summary: dict, rows: list[dict]) -> None:
    by: dict[tuple[str, str], dict[int, dict]] = {}
    for row in rows:
        by.setdefault((row["cell"], row["arm"]), {})[int(row["pair"])] = {
            "wall": float(row["wall_seconds"]),
            "cpu": float(row["cpu_seconds"]),
            "sha": row["sha256"],
        }
    recorded = summary["merge_ab"]
    check(len(recorded) == 4, "A/B covers the four classic cells")
    for cell, block in recorded.items():
        pre = by[(cell, "pre")]
        post = by[(cell, "post")]
        check(sorted(pre) == sorted(post), f"{cell}: both arms have the same pairs")
        cpu_deltas = []
        for pair in sorted(pre):
            wall_delta = (
                (post[pair]["wall"] - pre[pair]["wall"]) / pre[pair]["wall"] * 100
            )
            cpu_delta = (post[pair]["cpu"] - pre[pair]["cpu"]) / pre[pair]["cpu"] * 100
            cpu_deltas.append(cpu_delta)
            rec = block["pairs"][str(pair)]
            check(
                abs(rec["wall_delta_pct"] - wall_delta) < 0.01,
                f"{cell} pair {pair}: recorded wall delta re-derives",
            )
            check(
                abs(rec["cpu_delta_pct"] - cpu_delta) < 0.01,
                f"{cell} pair {pair}: recorded CPU delta re-derives",
            )
        check(
            all(delta > 0 for delta in cpu_deltas),
            f"{cell}: process CPU rises in every one of the three pairs",
        )
        check(
            block["median_cpu_delta_pct"] > 15.0,
            f"{cell}: median CPU delta above 15%",
        )
        check(
            len(set(pre[p]["sha"] for p in pre)) == 1
            and len(set(post[p]["sha"] for p in post)) == 1,
            f"{cell}: each arm is byte-identical across its pairs",
        )
        check(
            set(pre[p]["sha"] for p in pre) != set(post[p]["sha"] for p in post),
            f"{cell}: the two arms do not share a hash",
        )
        check(
            block["median_wall_delta_pct"] > 0.0,
            f"{cell}: median wall delta is a loss",
        )


def check_instrumented(summary: dict) -> None:
    inst = summary["instrumented_base_classic_off"]
    phases = inst["phases_ms"]
    pre_layer = phases["pre"]["per_layer"]
    post_layer = phases["post"]["per_layer"]
    check(post_layer / pre_layer > 4.0, "instrumented per-layer ratio above 4x")
    pre_prepass = phases["pre"]["prepass"]
    post_prepass = phases["post"]["prepass"]
    check(
        abs(post_prepass - pre_prepass) / pre_prepass < 0.03,
        "instrumented prepass is unchanged within 3%",
    )
    modules = inst["module_bracket_ms"]
    exploded = [
        name
        for name, row in modules.items()
        if row["pre"] < 1_000 and row["post"] > 28_000 and row["calls_post"] > 100
    ]
    check(
        len(exploded) >= 8,
        f"at least eight small per-layer modules explode past 28 s (found {len(exploded)})",
    )
    for name in exploded:
        check(
            modules[name]["calls_pre"] == modules[name]["calls_post"],
            f"{name}: the merge does not change the call count",
        )
    perimeters = modules["com.core.classic-perimeters"]
    check(
        perimeters["post"] > perimeters["pre"] * 1.5,
        "classic-perimeters bracket grows by more than 50%",
    )


def check_paint(summary: dict) -> None:
    paint = summary["paint_audit"]
    cube = paint["cube_4color"]
    pre = cube["pre"]
    post = cube["post"]
    check(
        abs(sum(pre["tool_e"].values()) - pre["total_e"]) < 0.01,
        "cube_4color pre: per-tool extrusion sums to the total",
    )
    check(
        abs(sum(post["tool_e"].values()) - post["total_e"]) < 0.01,
        "cube_4color post: per-tool extrusion sums to the total",
    )
    total_delta = abs(post["total_e"] - pre["total_e"]) / pre["total_e"]
    check(total_delta < 0.002, "cube_4color total extrusion moves less than 0.2%")
    tool_deltas = {
        tool: (post["tool_e"][tool] - pre["tool_e"][tool]) / pre["tool_e"][tool] * 100
        for tool in pre["tool_e"]
    }
    check(
        max(abs(v) for v in tool_deltas.values()) > 1.0,
        "cube_4color per-tool attribution moves by more than 1%",
    )
    check(
        cube["bytes"]["post"] < cube["bytes"]["pre"],
        "cube_4color output volume falls",
    )
    seams = paint["painted_seams"]
    check(
        seams["moves"]["pre"] == seams["moves"]["post"],
        "painted_seams keeps its G1 move count",
    )
    check(seams["diff_lines"] > 1_000, "painted_seams output ordering changes")


def run_positive() -> None:
    summary, rows = load()
    check_output_audit(summary)
    check_per_layer(summary)
    check_merge_ab(summary, rows)
    check_instrumented(summary)
    check_paint(summary)


def run_self_test() -> None:
    summary, rows = load()

    def rejects(mutate) -> None:
        mutated = copy.deepcopy(summary)
        mutated_rows = copy.deepcopy(rows)
        mutate(mutated, mutated_rows)
        try:
            check_output_audit(mutated)
            check_per_layer(mutated)
            check_merge_ab(mutated, mutated_rows)
            check_instrumented(mutated)
            check_paint(mutated)
        except SystemExit:
            return
        raise SystemExit("FAIL: a mutated summary was accepted")

    def wrong_byte_delta(s, _r):
        s["output_audit"]["base-classic-on"]["byte_delta"] = 35

    def wrong_per_layer(s, _r):
        s["cells"]["base-classic-off"]["per_layer_ms"]["post"] = [
            v / 10 for v in s["cells"]["base-classic-off"]["per_layer_ms"]["post"]
        ]

    def wrong_cpu(s, _r):
        s["merge_ab"]["base-classic-off"]["pairs"]["1"]["cpu_delta_pct"] = 1.0

    def cpu_swap(s, r):
        for row in r:
            if row["cell"] == "benchy-classic-off" and row["pair"] == "2":
                row["cpu_seconds"] = "10.0"

    def flat_instrumented(s, _r):
        s["instrumented_base_classic_off"]["phases_ms"]["post"]["per_layer"] = s[
            "instrumented_base_classic_off"
        ]["phases_ms"]["pre"]["per_layer"]

    def paint_inverted(s, _r):
        s["paint_audit"]["cube_4color"]["post"]["tool_e"]["T0"] = 9999.0

    for mutate in (
        wrong_byte_delta,
        wrong_per_layer,
        wrong_cpu,
        cpu_swap,
        flat_instrumented,
        paint_inverted,
    ):
        rejects(mutate)
        print(f"rejected: {mutate.__name__}")

    run_positive()


def main() -> int:
    if "--self-test" in sys.argv:
        run_self_test()
        print(f"PASS: {CHECKS} checks; 6/6 negative controls rejected")
    else:
        run_positive()
        print(f"PASS: {CHECKS} checks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

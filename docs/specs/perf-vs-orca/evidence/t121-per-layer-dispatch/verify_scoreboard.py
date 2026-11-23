#!/usr/bin/env python3
"""Verify the t121 post-merge Orca scoreboard from the committed rows.

Re-derives the 8 x 3 x (1 warmup + 3 measured) roster, the PnP output identity,
the standing starvation rule, the per-cell medians and the Orca ratios. The
Orca G-code hash is not an identity (its appendix embeds timing); only the
PnP ordinary/accelerated hashes are compared.
"""

from __future__ import annotations

from collections import defaultdict
import csv
import io
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOLS = ("orca", "pnp-ordinary", "pnp-accelerated")


def load_rows(text: str) -> list[dict]:
    return list(csv.DictReader(io.StringIO(text)))


def check(summary: dict, rows: list[dict]) -> list[str]:
    problems: list[str] = []
    if len(rows) != 96:
        problems.append(f"expected 96 rows, got {len(rows)}")

    groups: dict[tuple[str, str, str, str], list[dict]] = defaultdict(list)
    for row in rows:
        groups[(row["fixture"], row["generator"], row["supports"], row["tool"])].append(
            row
        )

    cells = sorted({(k[0], k[1], k[2]) for k in groups})
    if len(cells) != 8:
        problems.append(f"expected 8 cells, got {len(cells)}")

    computed: dict[str, dict] = {}
    for cell in cells:
        key = "/".join(cell)
        per_tool = {}
        for tool in TOOLS:
            arm = groups.get((*cell, tool), [])
            warmups = [r for r in arm if r["warmup"] != "0"]
            measured = [r for r in arm if r["warmup"] == "0"]
            if len(warmups) != 1 or len(measured) != 3:
                problems.append(
                    f"{key}/{tool}: roster warmups={len(warmups)} measured={len(measured)}"
                )
                continue
            if tool != "orca":
                hashes = {r["sha256_of_gcode"] for r in measured}
                if len(hashes) != 1:
                    problems.append(f"{key}/{tool}: PnP samples are not byte-identical")
                for row in measured:
                    if row["degraded"] != "false":
                        problems.append(f"{key}/{tool}: degraded={row['degraded']}")
                    if row["non_fatal_error_count"] != "0":
                        problems.append(
                            f"{key}/{tool}: non_fatal={row['non_fatal_error_count']}"
                        )
            ratios = [
                float(r["cpu_seconds"]) / float(r["wall_seconds"]) for r in measured
            ]
            best = max(ratios)
            retained = [
                row for row, ratio in zip(measured, ratios) if ratio >= 0.75 * best
            ]
            per_tool[tool] = {
                "wall_s": statistics.median(float(r["wall_seconds"]) for r in retained),
                "cpu_s": statistics.median(float(r["cpu_seconds"]) for r in retained),
                "n": len(retained),
                "excluded": len(measured) - len(retained),
            }
        if set(per_tool) != set(TOOLS):
            continue
        if per_tool["pnp-ordinary"]["wall_s"] and per_tool["pnp-accelerated"]["wall_s"]:
            ordinary_hash = {
                r["sha256_of_gcode"]
                for r in groups[(*cell, "pnp-ordinary")]
                if r["warmup"] == "0"
            }
            accelerated_hash = {
                r["sha256_of_gcode"]
                for r in groups[(*cell, "pnp-accelerated")]
                if r["warmup"] == "0"
            }
            if ordinary_hash != accelerated_hash:
                problems.append(f"{key}: ordinary and accelerated outputs differ")
        per_tool["ordinary_ratio"] = (
            per_tool["pnp-ordinary"]["wall_s"] / per_tool["orca"]["wall_s"]
        )
        per_tool["accelerated_ratio"] = (
            per_tool["pnp-accelerated"]["wall_s"] / per_tool["orca"]["wall_s"]
        )
        computed[key] = per_tool

    wanted = summary.get("cells", {})
    if set(computed) != set(wanted):
        problems.append(f"summary cells differ: {sorted(set(computed) ^ set(wanted))}")
    for key, derived in computed.items():
        expected = wanted.get(key)
        if expected is None:
            continue
        for tool in TOOLS:
            for field in ("wall_s", "cpu_s"):
                if abs(derived[tool][field] - expected[tool][field]) > 0.01:
                    problems.append(f"{key}/{tool}: {field} differs from summary")
            if derived[tool]["n"] != expected[tool]["n"]:
                problems.append(f"{key}/{tool}: retained sample count differs")
        for field in ("ordinary_ratio", "accelerated_ratio"):
            if abs(derived[field] - expected[field]) > 0.01:
                problems.append(f"{key}: {field} differs from summary")

    if computed:
        ordinary = [v["ordinary_ratio"] for v in computed.values()]
        accelerated = [v["accelerated_ratio"] for v in computed.values()]
        if min(ordinary) < 1.0 or min(accelerated) < 1.0:
            problems.append("a PnP arm is recorded as faster than Orca; check the sign")
        if not (2.78 <= min(ordinary) <= 2.79 and 5.10 <= max(ordinary) <= 5.12):
            problems.append("ordinary/Orca range is not the recorded 2.78x-5.11x")
        if not (2.75 <= min(accelerated) <= 2.77 and 5.08 <= max(accelerated) <= 5.11):
            problems.append("accelerated/Orca range is not the recorded 2.76x-5.10x")
    return problems


def main() -> int:
    summary = json.loads((HERE / "scoreboard-summary.json").read_text(encoding="utf-8"))
    rows = load_rows((HERE / "scoreboard.csv").read_text(encoding="utf-8"))
    problems = check(summary, rows)
    if problems:
        for problem in problems:
            print(f"FAIL: {problem}")
        return 1
    print(
        "PASS: t121 scoreboard — 8 cells x 3 tools x [1 warmup + 3 measured]; "
        "PnP ordinary == accelerated bytes in 8/8; ordinary/Orca 2.78x-5.11x"
    )
    if "--self-test" in sys.argv:
        import copy

        controls = []

        bad_hash = copy.deepcopy(rows)
        for row in bad_hash:
            if row["tool"] == "pnp-accelerated" and row["warmup"] == "0":
                row["sha256_of_gcode"] = "0" * 64
                break
        controls.append(("flipped PnP hash", summary, bad_hash))

        dropped = copy.deepcopy(rows)
        dropped.pop()
        controls.append(("dropped row", summary, dropped))

        duplicate = copy.deepcopy(rows)
        duplicate.append(copy.deepcopy(rows[0]))
        controls.append(("duplicate row", summary, duplicate))

        bad_median = copy.deepcopy(summary)
        bad_median["cells"]["base/classic/on"]["pnp-ordinary"]["wall_s"] += 1
        controls.append(("changed median", bad_median, rows))

        bad_ratio = copy.deepcopy(summary)
        bad_ratio["cells"]["benchy/classic/off"]["ordinary_ratio"] += 0.1
        controls.append(("changed ratio", bad_ratio, rows))

        bad_degraded = copy.deepcopy(rows)
        for row in bad_degraded:
            if row["tool"] == "pnp-ordinary" and row["warmup"] == "0":
                row["degraded"] = "true"
                break
        controls.append(("degraded PnP run", summary, bad_degraded))

        for name, control_summary, control_rows in controls:
            if not check(control_summary, control_rows):
                print(f"MISSED: {name}")
                return 1
            print(f"rejected: {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

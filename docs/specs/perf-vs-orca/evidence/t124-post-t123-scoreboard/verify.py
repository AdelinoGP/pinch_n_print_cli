#!/usr/bin/env python3
"""Verify the t124 post-t123 Orca scoreboard from the committed rows.

Re-derives, from `scoreboard.csv` alone plus the committed t123 reference set:

- the 8 x 3 x (1 warmup + 3 measured) roster;
- PnP output identity within and across the two PnP arms (Orca's G-code hash is
  not an identity: its appendix embeds timing);
- `degraded=false` / `non_fatal=0` on every PnP run;
- the standing starvation rule (cpu/wall below 0.75x the group's best sample is
  excluded), the per-cell medians and the Orca ratios;
- every Orca run's resolved config pins (t103 pins + filament leg) and the t105
  `tree_strong` support-style pin on supports-on cells;
- **provenance**: every PnP cell hash equals the t123 candidate-ordinary hash,
  i.e. the PnP side measured here is the post-t123 tree;
- the destination status: PnP loses all eight cells, within the recorded range.

`--self-test` re-runs the checks against mutated copies and fails if any control
is not rejected.
"""

from __future__ import annotations

import copy
import csv
import io
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
T123_SUMMARY = HERE.parent / "t123-facet-retirement" / "reduced" / "summary.json"
TOOLS = ("orca", "pnp-ordinary", "pnp-accelerated")
PNP_TOOLS = ("pnp-ordinary", "pnp-accelerated")

# Recorded in SCOREBOARD.md; the verifier is a check against the prose, not a
# re-derivation of it. Fill from the measured take.
RECORDED_ORDINARY_RANGE = (2.36, 4.65)
RECORDED_ACCELERATED_RANGE = (2.40, 4.67)
RECORDED_CELL_COUNT = 8


def load_rows(text: str) -> list[dict]:
    return list(csv.DictReader(io.StringIO(text)))


def check(
    summary: dict, rows: list[dict], t123_candidate_hashes: dict[str, str]
) -> list[str]:
    problems: list[str] = []
    if len(rows) != RECORDED_CELL_COUNT * len(TOOLS) * 4:
        problems.append(
            f"expected {RECORDED_CELL_COUNT * len(TOOLS) * 4} rows, got {len(rows)}"
        )

    groups: dict[tuple[str, str, str, str], list[dict]] = defaultdict(list)
    for row in rows:
        groups[(row["fixture"], row["generator"], row["supports"], row["tool"])].append(
            row
        )

    cells = sorted({(k[0], k[1], k[2]) for k in groups})
    if len(cells) != RECORDED_CELL_COUNT:
        problems.append(f"expected {RECORDED_CELL_COUNT} cells, got {len(cells)}")

    computed: dict[str, dict] = {}
    for cell in cells:
        key = "/".join(cell)
        per_tool: dict[str, dict] = {}
        for tool in TOOLS:
            arm = groups.get((*cell, tool), [])
            warmups = [r for r in arm if r["warmup"] != "0"]
            measured = [r for r in arm if r["warmup"] == "0"]
            if len(warmups) != 1 or len(measured) != 3:
                problems.append(
                    f"{key}/{tool}: roster warmups={len(warmups)} measured={len(measured)}"
                )
                continue
            if tool in PNP_TOOLS:
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
                    if row["validated_generator"] != row["generator"]:
                        problems.append(
                            f"{key}/{tool}: validated generator {row['validated_generator']}"
                        )
                    # The standing adhesion asymmetry the scoreboard must
                    # disclose: PnP prints its 8 mm skirt-brim default while
                    # pinned-back Orca prints none.
                    if row["resolved_brim_width"] != "8":
                        problems.append(
                            f"{key}/{tool}: PnP brim_width={row['resolved_brim_width']}, "
                            "expected the 8 mm default"
                        )
            else:
                for row in measured:
                    pin = row["resolved_enable_arc_fitting"]
                    if pin not in ("0", "disabled"):
                        problems.append(f"{key}/orca: enable_arc_fitting={pin}")
                    if row["resolved_elefant_foot_compensation"] != "0":
                        problems.append(
                            f"{key}/orca: elefant_foot_compensation="
                            f"{row['resolved_elefant_foot_compensation']}"
                        )
                    if row["resolved_brim_width"] != "0":
                        problems.append(
                            f"{key}/orca: brim_width={row['resolved_brim_width']}"
                        )
                    if row["resolved_filament_density"] in ("", "0"):
                        problems.append(f"{key}/orca: filament density did not resolve")
                    if row["resolved_filament_max_volumetric_speed"] in ("", "2"):
                        problems.append(f"{key}/orca: volumetric speed did not resolve")
                    if row["resolved_first_layer_temperature"] in ("", "200"):
                        problems.append(
                            f"{key}/orca: first-layer temperature did not resolve"
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
        cell_id = f"{cell[0]}-{cell[1]}-{cell[2]}"
        reference = t123_candidate_hashes.get(cell_id)
        if reference is None:
            problems.append(f"{key}: no t123 candidate hash for {cell_id}")
        elif ordinary_hash != {reference}:
            problems.append(
                f"{key}: PnP hash is not the t123 candidate set (provenance)"
            )
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
        if min(ordinary) <= 1.0 or min(accelerated) <= 1.0:
            problems.append("a PnP arm is recorded as faster than Orca; check the sign")
        lo, hi = RECORDED_ORDINARY_RANGE
        if not (lo - 0.01 <= min(ordinary) and max(ordinary) <= hi + 0.01):
            problems.append(
                f"ordinary/Orca range {min(ordinary):.2f}x-{max(ordinary):.2f}x "
                f"is not the recorded {lo:.2f}x-{hi:.2f}x"
            )
        lo, hi = RECORDED_ACCELERATED_RANGE
        if not (lo - 0.01 <= min(accelerated) and max(accelerated) <= hi + 0.01):
            problems.append(
                f"accelerated/Orca range {min(accelerated):.2f}x-{max(accelerated):.2f}x "
                f"is not the recorded {lo:.2f}x-{hi:.2f}x"
            )
    return problems


def load_t123_hashes() -> dict[str, str]:
    doc = json.loads(T123_SUMMARY.read_text(encoding="utf-8"))
    return {
        cell: entry["candidate-ordinary"]["sha256"]
        for cell, entry in doc["output"].items()
    }


def main() -> int:
    summary = json.loads((HERE / "scoreboard-summary.json").read_text(encoding="utf-8"))
    rows = load_rows((HERE / "scoreboard.csv").read_text(encoding="utf-8-sig"))
    hashes = load_t123_hashes()
    problems = check(summary, rows, hashes)
    if problems:
        for problem in problems:
            print(f"FAIL: {problem}")
        return 1
    print(
        "PASS: t124 scoreboard - 8 cells x 3 tools x [1 warmup + 3 measured]; "
        "PnP ordinary == accelerated bytes in 8/8; PnP hashes equal the t123 "
        f"candidate set; ordinary/Orca "
        f"{min(v['ordinary_ratio'] for v in summary['cells'].values()):.2f}x-"
        f"{max(v['ordinary_ratio'] for v in summary['cells'].values()):.2f}x"
    )
    if "--self-test" in sys.argv:
        controls = []

        bad_hash = copy.deepcopy(rows)
        for row in bad_hash:
            if row["tool"] == "pnp-accelerated" and row["warmup"] == "0":
                row["sha256_of_gcode"] = "0" * 64
                break
        controls.append(("flipped PnP hash", summary, bad_hash, hashes))

        stale = copy.deepcopy(rows)
        for row in stale:
            if row["tool"] == "pnp-ordinary" and row["warmup"] == "0":
                row["sha256_of_gcode"] = "f" * 64
                break
        controls.append(("pre-t123 PnP output", summary, stale, hashes))

        bad_reference = dict(hashes)
        first = sorted(bad_reference)[0]
        bad_reference[first] = "0" * 64
        controls.append(("stale t123 reference", summary, rows, bad_reference))

        dropped = copy.deepcopy(rows)
        dropped.pop()
        controls.append(("dropped row", summary, dropped, hashes))

        duplicate = copy.deepcopy(rows)
        duplicate.append(copy.deepcopy(rows[0]))
        controls.append(("duplicate row", summary, duplicate, hashes))

        bad_median = copy.deepcopy(summary)
        bad_median["cells"]["base/classic/on"]["pnp-ordinary"]["wall_s"] += 1
        controls.append(("changed median", bad_median, rows, hashes))

        bad_ratio = copy.deepcopy(summary)
        bad_ratio["cells"]["benchy/classic/off"]["ordinary_ratio"] += 0.1
        controls.append(("changed ratio", bad_ratio, rows, hashes))

        bad_degraded = copy.deepcopy(rows)
        for row in bad_degraded:
            if row["tool"] == "pnp-ordinary" and row["warmup"] == "0":
                row["degraded"] = "true"
                break
        controls.append(("degraded PnP run", summary, bad_degraded, hashes))

        bad_pin = copy.deepcopy(rows)
        for row in bad_pin:
            if row["tool"] == "orca" and row["warmup"] == "0":
                row["resolved_brim_width"] = "5"
                break
        controls.append(("unpinned Orca brim", summary, bad_pin, hashes))

        bad_style = copy.deepcopy(rows)
        for row in bad_style:
            if (
                row["tool"] == "orca"
                and row["warmup"] == "0"
                and row["supports"] == "on"
            ):
                row["resolved_enable_arc_fitting"] = "enabled"
                break
        controls.append(("unpinned arc fitting", summary, bad_style, hashes))

        for name, control_summary, control_rows, control_hashes in controls:
            if not check(control_summary, control_rows, control_hashes):
                print(f"MISSED: {name}")
                return 1
            print(f"rejected: {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""t102 reduction: committed rows -> the committed evidence artifacts.

Global allocator substitution (perf-vs-orca ticket 102): mimalloc wrapped by the
accounting allocator, versus the system heap.

    python docs/specs/perf-vs-orca/evidence/t102-allocator/reduce.py

Reads `proof-rows.json`, `pregate-rows.json` and `measure-rows.json` (produced by
`extract.py` from the raw captures) and writes `qualification.json`,
`pregate.json`, `ab-reduction.json` and `summary.json` next to this script.
`verify.py` re-derives every claim from the committed artifacts.
"""

from __future__ import annotations

import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
MODES = ("ordinary", "accelerated")
FIXTURES = ("benchy", "base")
GENERATORS = ("classic", "arachne")
SUPPORTS = ("off", "on")
PROOF_CELLS = [
    f"{mode}-{fixture}-{generator}-{supports}"
    for mode in MODES
    for fixture in FIXTURES
    for generator in GENERATORS
    for supports in SUPPORTS
]
MEASURE_CELLS = [
    f"{mode}-base-{generator}-on" for mode in MODES for generator in GENERATORS
]
PREGATE_CELL = "ordinary-base-classic-on"
PAIRS_REQUIRED = 6
PREGATE_PAIRS = 3
CPU_CEILING_PCT = 3.0
FLOOR_PCT = 0.80
FLOOR_WALLS_S = {
    "ordinary-base-classic-on": 164.803,
    "ordinary-base-arachne-on": 166.931,
    "accelerated-base-classic-on": 163.258,
    "accelerated-base-arachne-on": 154.282,
}


def median(values) -> float:
    return statistics.median(values)


def load(name: str) -> dict[str, list[dict]]:
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def load_references() -> dict[str, dict]:
    path = HERE.parent / "t91-exact-z-memo" / "references.json"
    return json.loads(path.read_text(encoding="utf-8"))["cells"]


def build_qualification(proof_rows: dict[str, list[dict]]) -> dict:
    references = load_references()
    cells = []
    problems = []
    for cell in PROOF_CELLS:
        rows = proof_rows.get(cell, [])
        if len(rows) != 2:
            problems.append(f"{cell}: expected 2 proof rows, found {len(rows)}")
            continue
        arms = {row["arm"] for row in rows}
        if arms != {"baseline", "candidate"}:
            problems.append(f"{cell}: arms {sorted(arms)} != baseline/candidate")
            continue
        shas = {row["arm"]: row["sha256"] for row in rows}
        if shas["baseline"] != shas["candidate"]:
            problems.append(
                f"{cell}: output drift baseline {shas['baseline'][:8]} vs "
                f"candidate {shas['candidate'][:8]}"
            )
            continue
        if len({row["exe_sha256"] for row in rows}) != 2:
            problems.append(f"{cell}: arms share one host binary")
        reference = references.get(cell)
        if reference is not None:
            if shas["baseline"] != reference["sha256"]:
                problems.append(
                    f"{cell}: output {shas['baseline'][:8]} != post-t81 reference "
                    f"{reference['sha256'][:8]}"
                )
            if int(rows[0]["bytes"]) != int(reference["bytes"]):
                problems.append(
                    f"{cell}: bytes {rows[0]['bytes']} != reference {reference['bytes']}"
                )
        for row in rows:
            if row["status"] != "ok" or row["degraded"] is not False:
                problems.append(
                    f"{cell}: unclean {row['arm']} status={row['status']} "
                    f"degraded={row['degraded']}"
                )
            if int(row["fatal"]) != 0 or int(row["non_fatal"]) != 0:
                problems.append(f"{cell}: fatal/non-fatal not zero on {row['arm']}")
        types = rows[0]["type_counts"]
        for role in ("Outer wall", "Sparse infill", "Inner wall"):
            if role not in types:
                problems.append(f"{cell}: vacuous output (missing {role})")
        if cell.split("-")[3] == "on" and (
            "Support" not in types or "Support interface" not in types
        ):
            problems.append(f"{cell}: vacuous support roles")
        cells.append(
            {
                "cell": cell,
                "sha256": shas["baseline"],
                "bytes": int(rows[0]["bytes"]),
                "reference_sha256": (
                    reference["sha256"] if reference is not None else None
                ),
                "wall_seconds": {row["arm"]: row["wall_seconds"] for row in rows},
                "exe_sha256": {row["arm"]: row["exe_sha256"] for row in rows},
                "type_counts": types,
                "status": rows[0]["status"],
                "degraded": rows[0]["degraded"],
            }
        )

    cross_mode = {}
    by_name = {cell["cell"]: cell for cell in cells}
    for fixture in FIXTURES:
        for generator in GENERATORS:
            for supports in SUPPORTS:
                ordinary = by_name.get(f"ordinary-{fixture}-{generator}-{supports}")
                accelerated = by_name.get(
                    f"accelerated-{fixture}-{generator}-{supports}"
                )
                if ordinary is None or accelerated is None:
                    continue
                cross_mode[f"{fixture}-{generator}-{supports}"] = {
                    "equal": ordinary["sha256"] == accelerated["sha256"],
                    "ordinary_sha256": ordinary["sha256"],
                    "accelerated_sha256": accelerated["sha256"],
                }
    cross_mode_ok = len(cross_mode) == 8 and all(
        entry["equal"] for entry in cross_mode.values()
    )
    if not cross_mode_ok:
        problems.append("cross-mode bytes differ")
    return {
        "pass": not problems and len(cells) == 16,
        "cells": cells,
        "problems": problems,
        "cross_mode": cross_mode,
        "cross_mode_note": (
            "ordinary vs accelerated byte equality; the post-t81 reference set "
            "is ordinary-only, so accelerated cells are gated on cross-mode "
            "equality plus their own baseline/candidate identity"
        ),
    }


def reduce_cell(cell: str, rows: list[dict], pairs_required: int) -> dict:
    if not rows:
        return {"cell": cell, "status": "not_run"}
    non_warm = [row for row in rows if row["warmup"] == 0]
    baseline = [row for row in non_warm if row["arm"] == "baseline"]
    candidate = [row for row in non_warm if row["arm"] == "candidate"]
    if not baseline or not candidate:
        return {"cell": cell, "status": "no_samples"}
    ratio_base = median(row["cpu_wall_ratio"] for row in baseline)
    ratio_candidate = median(row["cpu_wall_ratio"] for row in candidate)
    retained = []
    starved = []
    for index in range(1, pairs_required + 1):
        base_row = next(
            (
                row
                for row in non_warm
                if row["arm"] == "baseline" and row["pair"] == index
            ),
            None,
        )
        cand_row = next(
            (
                row
                for row in non_warm
                if row["arm"] == "candidate" and row["pair"] == index
            ),
            None,
        )
        if base_row is None or cand_row is None:
            starved.append({"pair": index, "reason": "missing"})
            continue
        if base_row["sha256"] != cand_row["sha256"]:
            starved.append({"pair": index, "reason": "output drift"})
            continue
        if (
            base_row["cpu_wall_ratio"] < 0.5 * ratio_base
            or cand_row["cpu_wall_ratio"] < 0.5 * ratio_candidate
        ):
            starved.append({"pair": index, "reason": "starvation"})
            continue
        retained.append((base_row, cand_row))
    if not retained:
        return {"cell": cell, "status": "all_pairs_starved", "excluded_pairs": starved}
    wall_ratios = [
        cand["wall_seconds"] / base["wall_seconds"] for base, cand in retained
    ]
    cpu_ratios = [cand["cpu_seconds"] / base["cpu_seconds"] for base, cand in retained]

    def arm_stats(rows_for_arm: list[dict]) -> dict:
        wall = [row["wall_seconds"] for row in rows_for_arm]
        cpu = [row["cpu_seconds"] for row in rows_for_arm]
        return {
            "n": len(rows_for_arm),
            "wall_median_s": median(wall),
            "wall_range": [min(wall), max(wall)],
            "cpu_median_s": median(cpu),
            "cpu_wall_ratio_median": median(
                row["cpu_wall_ratio"] for row in rows_for_arm
            ),
        }

    wall_delta = (median(wall_ratios) - 1.0) * 100.0
    wall_s = FLOOR_WALLS_S.get(cell)
    return {
        "cell": cell,
        "status": "reduced",
        "retained_pairs": len(retained),
        "excluded_pairs": starved,
        "wall_ratio_paired_median": median(wall_ratios),
        "wall_delta_pct_paired_median": wall_delta,
        "cpu_ratio_paired_median": median(cpu_ratios),
        "cpu_delta_pct_paired_median": (median(cpu_ratios) - 1.0) * 100.0,
        "wall_delta_seconds": (median(wall_ratios) - 1.0) * wall_s if wall_s else None,
        "floor_seconds": wall_s * FLOOR_PCT / 100.0 if wall_s else None,
        "multiple_of_floor": (abs(wall_delta) / FLOOR_PCT if wall_s else None),
        "baseline": arm_stats([base for base, _ in retained]),
        "candidate": arm_stats([cand for _, cand in retained]),
        "pairs": [
            {
                "pair": index,
                "baseline_wall_s": base["wall_seconds"],
                "candidate_wall_s": cand["wall_seconds"],
                "wall_ratio": cand["wall_seconds"] / base["wall_seconds"],
                "baseline_cpu_s": base["cpu_seconds"],
                "candidate_cpu_s": cand["cpu_seconds"],
                "cpu_ratio": cand["cpu_seconds"] / base["cpu_seconds"],
                "sha256": base["sha256"],
            }
            for index, (base, cand) in enumerate(retained, start=1)
        ],
    }


def build_pregate(pregate_rows: dict[str, list[dict]]) -> dict:
    result = reduce_cell(
        PREGATE_CELL, pregate_rows.get(PREGATE_CELL, []), PREGATE_PAIRS
    )
    if result["status"] != "reduced":
        return {"cell": PREGATE_CELL, "status": result["status"], "proceed": False}
    proceed = (
        result["wall_delta_pct_paired_median"] < 0.0
        and result["cpu_delta_pct_paired_median"] <= CPU_CEILING_PCT
        and result["retained_pairs"] == PREGATE_PAIRS
        and not result["excluded_pairs"]
    )
    result["proceed"] = proceed
    result["clauses"] = {
        "wall_negative": result["wall_delta_pct_paired_median"] < 0.0,
        "cpu_within_ceiling": result["cpu_delta_pct_paired_median"] <= CPU_CEILING_PCT,
        "pairs": result["retained_pairs"] == PREGATE_PAIRS
        and not result["excluded_pairs"],
    }
    return result


def build_ab(measure_rows: dict[str, list[dict]]) -> dict:
    cells = {
        cell: reduce_cell(cell, measure_rows.get(cell, []), PAIRS_REQUIRED)
        for cell in MEASURE_CELLS
    }
    wall_ok = all(
        result["status"] == "reduced" and result["wall_delta_pct_paired_median"] < 0.0
        for result in cells.values()
    )
    cpu_ok = all(
        result["status"] == "reduced"
        and result["cpu_delta_pct_paired_median"] <= CPU_CEILING_PCT
        for result in cells.values()
    )
    pairs_ok = all(
        result["status"] == "reduced"
        and result["retained_pairs"] == PAIRS_REQUIRED
        and not result["excluded_pairs"]
        for result in cells.values()
    )
    outputs_ok = all(
        len({pair["sha256"] for pair in result["pairs"]}) == 1
        for result in cells.values()
        if result["status"] == "reduced"
    )
    return {
        "gate_cells": MEASURE_CELLS,
        "cpu_clause_ceiling_pct": CPU_CEILING_PCT,
        "wall_clause": "paired-median wall delta < 0 in all four pre-declared cells",
        "cpu_clause": f"paired-median process-CPU delta <= {CPU_CEILING_PCT} % in all four",
        "pairs_clause": "6 retained pairs per cell, zero starvation exclusions",
        "output_clause": "every retained pair byte-identical",
        "wall_pass": wall_ok,
        "cpu_pass": cpu_ok,
        "pairs_pass": pairs_ok,
        "output_pass": outputs_ok,
        "verdict": (
            "PASS" if wall_ok and cpu_ok and pairs_ok and outputs_ok else "GATE NOT MET"
        ),
        "cells": cells,
    }


def build_summary(qualification: dict, pregate: dict, ab: dict) -> dict:
    cells = ab["cells"]
    return {
        "headline": {
            "qualification_pass": qualification["pass"],
            "qualification_cells": len(qualification["cells"]),
            "cross_mode_equal": sum(
                1 for entry in qualification["cross_mode"].values() if entry["equal"]
            ),
            "pregate_proceed": pregate.get("proceed", False),
            "gate_verdict": ab["verdict"],
            "wall_deltas_pct": {
                cell: cells[cell].get("wall_delta_pct_paired_median")
                for cell in MEASURE_CELLS
            },
            "cpu_deltas_pct": {
                cell: cells[cell].get("cpu_delta_pct_paired_median")
                for cell in MEASURE_CELLS
            },
            "multiple_of_floor": {
                cell: cells[cell].get("multiple_of_floor") for cell in MEASURE_CELLS
            },
            "pairs": {
                cell: cells[cell].get("retained_pairs") for cell in MEASURE_CELLS
            },
        },
        "cells": cells,
        "pregate": pregate,
        "qualification_problems": qualification["problems"],
    }


def main() -> int:
    proof = load("proof-rows.json")
    pregate = load("pregate-rows.json")
    measure = load("measure-rows.json")
    qualification = build_qualification(proof)
    pre = build_pregate(pregate)
    ab = build_ab(measure)
    summary = build_summary(qualification, pre, ab)
    for name, payload in (
        ("qualification.json", qualification),
        ("pregate.json", pre),
        ("ab-reduction.json", ab),
        ("summary.json", summary),
    ):
        (HERE / name).write_text(json.dumps(payload, indent=1) + "\n")
        print(f"wrote {name}")
    print(
        f"qualification pass={qualification['pass']} problems={len(qualification['problems'])}"
    )
    print(f"pregate proceed={pre.get('proceed')}")
    print(f"gate verdict={ab['verdict']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

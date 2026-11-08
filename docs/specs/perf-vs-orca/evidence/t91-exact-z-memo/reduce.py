#!/usr/bin/env python3
"""t91 reduction: raw capture rows -> the committed evidence artifacts.

Per-object memo for the z-independent exact-Z query terms
(perf-vs-orca ticket 91). Two modes:

  python reduce.py            # read the committed reduced rows (checkout-only)
  python reduce.py --raw      # read this machine's .local-artifacts captures

Both write `qualification.json`, `ab-reduction.json` and `summary.json` next to
this script. `extract.py` produces the committed reduced rows from `--raw`
captures; `verify.py` re-derives from the committed rows and checks that the
committed JSONs match, so a plain checkout can reproduce every headline claim.
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
ROOT = (
    REPO
    / ".local-artifacts"
    / "perimeter-reference-preparation"
    / "t91-exact-z-memo"
)

MODES = ("ordinary", "accelerated")
FIXTURES = ("benchy", "base")
GENERATORS = ("classic", "arachne")
SUPPORTS = ("off", "on")
PROOF_CELLS = [
    f"{mode}-{fixture}-{gen}-{sup}"
    for mode in MODES
    for fixture in FIXTURES
    for gen in GENERATORS
    for sup in SUPPORTS
]
MEASURE_CELLS = [f"{mode}-base-{gen}-on" for mode in MODES for gen in GENERATORS]
CPU_CEILING_PCT = 0.0
PAIRS_REQUIRED = 6

ROW_FIELDS = (
    "arm",
    "pair",
    "warmup",
    "exe_sha256",
    "wall_seconds",
    "cpu_seconds",
    "cpu_wall_ratio",
    "bytes",
    "sha256",
    "status",
    "degraded",
    "fatal",
    "non_fatal",
    "type_counts",
    "mode",
    "fixture",
    "generator",
    "supports",
)


# ── loading ────────────────────────────────────────────────────────────────


def _row(record: dict) -> dict:
    return {key: record[key] for key in ROW_FIELDS}


def load_raw_proof() -> dict[str, list[dict]]:
    rows = {}
    for cell in PROOF_CELLS:
        path = ROOT / "proof" / cell / "rows.csv"
        if not path.exists():
            rows[cell] = []
            continue
        with path.open(encoding="utf-8-sig", newline="") as handle:
            rows[cell] = [_row(row) for row in csv.DictReader(handle)]
    return rows


def load_raw_measure() -> dict[str, list[dict]]:
    rows = {}
    for cell in MEASURE_CELLS:
        path = ROOT / "measure" / cell / "rows.csv"
        if not path.exists():
            rows[cell] = []
            continue
        with path.open(encoding="utf-8-sig", newline="") as handle:
            rows[cell] = [_row(row) for row in csv.DictReader(handle)]
    return rows


def load_committed_rows(name: str) -> dict[str, list[dict]]:
    path = HERE / name
    if not path.exists():
        raise SystemExit(f"missing committed reduced rows: {path.name}")
    return json.loads(path.read_text(encoding="utf-8"))


def load_references() -> dict:
    return json.loads((HERE / "references.json").read_text(encoding="utf-8"))["cells"]


def load_attribution() -> dict:
    path = HERE / "attribution.json"
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


# ── qualification ──────────────────────────────────────────────────────────


def build_qualification(proof_rows: dict[str, list[dict]]) -> dict:
    references = load_references()
    cells = []
    problems = []
    for cell in PROOF_CELLS:
        rows = proof_rows[cell]
        if len(rows) != 2:
            problems.append(f"{cell}: expected 2 proof rows, found {len(rows)}")
            continue
        arms = {row["arm"] for row in rows}
        if arms != {"baseline", "candidate"}:
            problems.append(f"{cell}: arms {sorted(arms)} != baseline/candidate")
            continue
        if rows[0]["sha256"] != rows[1]["sha256"]:
            problems.append(
                f"{cell}: output drift baseline {rows[0]['sha256'][:8]} vs "
                f"candidate {rows[1]['sha256'][:8]}"
            )
            continue
        reference = references.get(cell)
        if reference is not None:
            if rows[0]["sha256"] != reference["sha256"]:
                problems.append(
                    f"{cell}: output {rows[0]['sha256'][:8]} != t81 reference "
                    f"{reference['sha256'][:8]}"
                )
            if int(rows[0]["bytes"]) != int(reference["bytes"]):
                problems.append(
                    f"{cell}: bytes {rows[0]['bytes']} != reference "
                    f"{reference['bytes']}"
                )
        for row in rows:
            if row["status"] != "ok" or row["degraded"] != "False":
                problems.append(
                    f"{cell}: unclean {row['arm']} status={row['status']} "
                    f"degraded={row['degraded']}"
                )
            if int(row["fatal"]) != 0 or int(row["non_fatal"]) != 0:
                problems.append(f"{cell}: fatal/non-fatal not zero on {row['arm']}")
        types = json.loads(rows[0]["type_counts"])
        for role in ("Outer wall", "Sparse infill", "Inner wall"):
            if role not in types:
                problems.append(f"{cell}: vacuous output (missing {role})")
        if cell.endswith("-on") and (
            "Support" not in types or "Support interface" not in types
        ):
            problems.append(f"{cell}: vacuous support roles")
        cells.append(
            {
                "cell": cell,
                "sha256": rows[0]["sha256"],
                "bytes": int(rows[0]["bytes"]),
                "reference_sha256": (
                    reference["sha256"] if reference is not None else None
                ),
                "wall_seconds": {
                    row["arm"]: float(row["wall_seconds"]) for row in rows
                },
                "exe_sha256": {row["arm"]: row["exe_sha256"] for row in rows},
                "cpu_seconds": {row["arm"]: float(row["cpu_seconds"]) for row in rows},
                "type_counts": types,
                "status": rows[0]["status"],
                "degraded": rows[0]["degraded"],
            }
        )

    cross_mode = {}
    by_name = {cell["cell"]: cell for cell in cells}
    for fixture in FIXTURES:
        for gen in GENERATORS:
            for sup in SUPPORTS:
                ordinary = by_name.get(f"ordinary-{fixture}-{gen}-{sup}")
                accelerated = by_name.get(f"accelerated-{fixture}-{gen}-{sup}")
                if ordinary is None or accelerated is None:
                    continue
                cross_mode[f"{fixture}-{gen}-{sup}"] = {
                    "equal": ordinary["sha256"] == accelerated["sha256"],
                    "ordinary_sha256": ordinary["sha256"],
                    "accelerated_sha256": accelerated["sha256"],
                }
    return {
        "pass": not problems and len(cells) == 16,
        "cells": cells,
        "problems": problems,
        "cross_mode": cross_mode,
        "cross_mode_note": (
            "ordinary vs accelerated byte equality; the accelerated artifacts "
            "have no post-t81 reference set, so this is reported, not gated"
        ),
    }


# ── paired A/B ─────────────────────────────────────────────────────────────


def median(values: list[float]) -> float:
    return statistics.median(values)


def reduce_cell(cell: str, rows: list[dict]) -> dict:
    if not rows:
        return {"cell": cell, "status": "not_run"}
    non_warm = [row for row in rows if row["warmup"] == "0"]
    baseline = [row for row in non_warm if row["arm"] == "baseline"]
    candidate = [row for row in non_warm if row["arm"] == "candidate"]
    if not baseline or not candidate:
        return {"cell": cell, "status": "no_samples"}
    ratio_base = median(float(row["cpu_wall_ratio"]) for row in baseline)
    ratio_cand = median(float(row["cpu_wall_ratio"]) for row in candidate)
    retained = []
    starved = []
    for index in range(1, PAIRS_REQUIRED + 1):
        base_row = next(
            (
                row
                for row in non_warm
                if row["arm"] == "baseline" and int(row["pair"]) == index
            ),
            None,
        )
        cand_row = next(
            (
                row
                for row in non_warm
                if row["arm"] == "candidate" and int(row["pair"]) == index
            ),
            None,
        )
        if base_row is None or cand_row is None:
            continue
        if (
            float(base_row["cpu_wall_ratio"]) < 0.5 * ratio_base
            or float(cand_row["cpu_wall_ratio"]) < 0.5 * ratio_cand
        ):
            starved.append(index)
            continue
        retained.append((base_row, cand_row, index))
    if not retained:
        return {"cell": cell, "status": "all_pairs_starved", "excluded_pairs": starved}
    wall_ratios = [
        float(cand["wall_seconds"]) / float(base["wall_seconds"])
        for base, cand, _ in retained
    ]
    cpu_ratios = [
        float(cand["cpu_seconds"]) / float(base["cpu_seconds"])
        for base, cand, _ in retained
    ]

    def arm_stats(arm_rows: list[dict]) -> dict:
        wall = [float(row["wall_seconds"]) for row in arm_rows]
        cpu = [float(row["cpu_seconds"]) for row in arm_rows]
        return {
            "n": len(wall),
            "wall_median_s": median(wall),
            "wall_range": [min(wall), max(wall)],
            "cpu_median_s": median(cpu),
            "cpu_wall_ratio_median": median(
                float(row["cpu_wall_ratio"]) for row in arm_rows
            ),
        }

    arm_shas = {
        arm: sorted({row["sha256"] for row in non_warm if row["arm"] == arm})
        for arm in ("baseline", "candidate")
    }
    arm_exes = {
        arm: sorted({row["exe_sha256"] for row in non_warm if row["arm"] == arm})
        for arm in ("baseline", "candidate")
    }
    return {
        "cell": cell,
        "status": "reduced",
        "arm_sha_sets": arm_shas,
        "arm_exe_sha_sets": arm_exes,
        "deterministic": all(len(shas) == 1 for shas in arm_shas.values()),
        "arms_distinct_binaries": (
            len(arm_exes["baseline"]) == 1
            and len(arm_exes["candidate"]) == 1
            and arm_exes["baseline"] != arm_exes["candidate"]
        ),
        "retained_pairs": len(retained),
        "excluded_pairs": starved,
        "wall_ratio_paired_median": median(wall_ratios),
        "wall_delta_pct_paired_median": (median(wall_ratios) - 1.0) * 100.0,
        "cpu_ratio_paired_median": median(cpu_ratios),
        "cpu_delta_pct_paired_median": (median(cpu_ratios) - 1.0) * 100.0,
        "baseline": arm_stats(baseline),
        "candidate": arm_stats(candidate),
        "pairs": [
            {
                "pair": index,
                "baseline_wall_s": float(base["wall_seconds"]),
                "candidate_wall_s": float(cand["wall_seconds"]),
                "baseline_cpu_s": float(base["cpu_seconds"]),
                "candidate_cpu_s": float(cand["cpu_seconds"]),
                "baseline_ratio": float(base["cpu_wall_ratio"]),
                "candidate_ratio": float(cand["cpu_wall_ratio"]),
                "sha_equal": base["sha256"] == cand["sha256"],
            }
            for base, cand, index in retained
        ],
    }


def build_ab(measure_rows: dict[str, list[dict]]) -> dict:
    cells = {cell: reduce_cell(cell, measure_rows[cell]) for cell in MEASURE_CELLS}
    passes = []
    for cell, result in cells.items():
        if result.get("status") != "reduced":
            passes.append(False)
            continue
        wall_ok = result["wall_delta_pct_paired_median"] < 0.0
        cpu_ok = result["cpu_delta_pct_paired_median"] <= CPU_CEILING_PCT
        sha_ok = all(pair["sha_equal"] for pair in result["pairs"])
        deterministic = result["deterministic"]
        result["wall_clause"] = "pass" if wall_ok else "fail"
        result["cpu_corroboration_clause"] = "pass" if cpu_ok else "fail"
        result["output_identity_clause"] = "pass" if sha_ok else "fail"
        result["determinism_clause"] = "pass" if deterministic else "fail"
        if result["retained_pairs"] < PAIRS_REQUIRED:
            result["note"] = (
                "retained pairs below 6 - inconclusive, not a measured pass"
            )
        passes.append(
            wall_ok
            and cpu_ok
            and sha_ok
            and deterministic
            and result["arms_distinct_binaries"]
            and result["retained_pairs"] >= PAIRS_REQUIRED
        )
    strict_pass = all(passes) and len(passes) == len(MEASURE_CELLS)
    cpu_flat_or_lower = all(
        cells[cell].get("status") != "reduced"
        or cells[cell]["cpu_delta_pct_paired_median"] <= 0.0
        for cell in MEASURE_CELLS
    )
    verdict = (
        "PASS (wall falls, process CPU flat or lower, every pair byte-identical "
        "and one SHA per arm, in all four pre-declared base supports-on cells)"
        if strict_pass
        else "GATE NOT MET - measured recommendation returns to the human"
    )
    return {
        "verdict": verdict,
        "gate_cells": MEASURE_CELLS,
        "cpu_clause_ceiling_pct": CPU_CEILING_PCT,
        "cpu_flat_or_lower_in_all_cells": cpu_flat_or_lower,
        "wall_clause": "paired-median wall delta < 0 in all four pre-declared cells",
        "cpu_clause": (
            "paired-median process-CPU delta <= 0.0 % in all four pre-declared cells "
            "(the ticket's 'flat or lower' clause: this candidate removes work, "
            "so a CPU rise is a red flag, not a ceiling to spend)"
        ),
        "output_clause": "every retained pair byte-identical, one SHA per arm",
        "cells": cells,
    }


# ── summary ────────────────────────────────────────────────────────────────


def build_summary(qualification: dict, ab: dict, attribution: dict) -> dict:
    """Headline numbers, all derived from the two artifacts above."""
    headline = {}
    for cell in ab["gate_cells"]:
        result = ab["cells"].get(cell, {})
        if result.get("status") != "reduced":
            headline[cell] = {"status": result.get("status")}
            continue
        headline[cell] = {
            "wall_delta_pct": result["wall_delta_pct_paired_median"],
            "cpu_delta_pct": result["cpu_delta_pct_paired_median"],
            "wall_delta_s": (
                result["candidate"]["wall_median_s"]
                - result["baseline"]["wall_median_s"]
            ),
            "retained_pairs": result["retained_pairs"],
            "excluded_pairs": result["excluded_pairs"],
            "baseline_wall_median_s": result["baseline"]["wall_median_s"],
        }
    return {
        "ticket": 91,
        "verdict": ab["verdict"],
        "cpu_flat_or_lower_in_all_cells": ab.get(
            "cpu_flat_or_lower_in_all_cells", False
        ),
        "qualification_pass": qualification["pass"],
        "qualified_cells": len(qualification["cells"]),
        "cross_mode_mismatches": [
            key
            for key, value in qualification["cross_mode"].items()
            if not value["equal"]
        ],
        "paired_gate": headline,
        "attribution": attribution.get("cells", {}),
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--raw",
        action="store_true",
        help="read this machine's .local-artifacts captures instead of the committed rows",
    )
    args = parser.parse_args(argv[1:])
    if args.raw:
        proof_rows = load_raw_proof()
        measure_rows = load_raw_measure()
    else:
        proof_rows = load_committed_rows("proof-rows.json")
        measure_rows = load_committed_rows("measure-rows.json")
    qualification = build_qualification(proof_rows)
    ab = build_ab(measure_rows)
    summary = build_summary(qualification, ab, load_attribution())
    (HERE / "qualification.json").write_text(
        json.dumps(qualification, indent=1), encoding="utf-8"
    )
    (HERE / "ab-reduction.json").write_text(json.dumps(ab, indent=1), encoding="utf-8")
    (HERE / "summary.json").write_text(json.dumps(summary, indent=1), encoding="utf-8")

    print(
        f"qualified cells: {len(qualification['cells'])}/16 pass={qualification['pass']}"
    )
    for problem in qualification["problems"]:
        print("PROBLEM:", problem)
    for cell in ab["gate_cells"]:
        result = ab["cells"][cell]
        if result.get("status") != "reduced":
            print(f"{cell}: {result}")
            continue
        print(
            f"{cell}: wall {result['wall_delta_pct_paired_median']:+.2f}% "
            f"cpu {result['cpu_delta_pct_paired_median']:+.2f}% "
            f"pairs {result['retained_pairs']}/6 excluded {result['excluded_pairs']} "
            f"({result['wall_clause']}/{result['cpu_corroboration_clause']}/"
            f"{result['output_identity_clause']}/{result['determinism_clause']})"
        )
    print("VERDICT:", ab["verdict"])
    return 0 if qualification["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

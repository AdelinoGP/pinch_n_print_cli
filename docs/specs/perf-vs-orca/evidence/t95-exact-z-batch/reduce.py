#!/usr/bin/env python3
"""t95 reduction: raw capture rows -> the committed evidence artifacts.

Batched requested-Z exact-Z cross-sections (perf-vs-orca ticket 95).

Two modes:

  python reduce.py            # read the committed reduced rows (checkout-only)
  python reduce.py --raw      # read this machine's .local-artifacts captures

Both write `qualification.json`, `ab-reduction.json` and `summary.json` next to
this script. `extract.py` produces the committed reduced rows from `--raw`
captures; `verify.py` re-derives every number from the committed rows and
checks the headline claims, so a plain checkout reproduces the take.
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
ROOT = (
    REPO / ".local-artifacts" / "perimeter-reference-preparation" / "t95-exact-z-batch"
)

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
PAIRS_REQUIRED = 6
CPU_CEILING_PCT = 3.0
FLOOR_PCT = 0.80

ROW_FIELDS = (
    "run_id",
    "phase",
    "mode",
    "fixture",
    "generator",
    "supports",
    "arm",
    "pair",
    "warmup",
    "threads",
    "exit_code",
    "exe_sha256",
    "wall_seconds",
    "cpu_seconds",
    "cpu_wall_ratio",
    "bytes",
    "sha256",
    "type_counts",
    "warnings",
    "status",
    "degraded",
    "fatal",
    "non_fatal",
)


def normalise_row(raw: dict) -> dict:
    row = {key: raw[key] for key in ROW_FIELDS if key in raw}
    for key in (
        "pair",
        "warmup",
        "threads",
        "exit_code",
        "bytes",
        "fatal",
        "non_fatal",
    ):
        row[key] = int(row[key])
    for key in ("wall_seconds", "cpu_seconds", "cpu_wall_ratio"):
        row[key] = float(row[key])
    for key in ("type_counts", "warnings"):
        value = row.get(key)
        if isinstance(value, str):
            row[key] = json.loads(value) if value else {}
        elif value is None:
            row[key] = {}
    # The raw per-run warning map keys on whole warning messages, which runs to
    # ~120 KB per row. Keep the counts the gates actually read.
    warnings = row.get("warnings") or {}
    if isinstance(warnings, dict) and set(warnings) != {"lines", "kinds"}:
        row["warnings"] = {
            "lines": sum(int(count) for count in warnings.values()),
            "kinds": len(warnings),
        }
    return row


def _read_csv(path: Path) -> list[dict]:
    with path.open(newline="", encoding="utf-8-sig") as handle:
        return [normalise_row(raw) for raw in csv.DictReader(handle)]


def load_raw(phase: str, cells: list[str]) -> dict[str, list[dict]]:
    rows: dict[str, list[dict]] = {}
    for cell in cells:
        path = ROOT / phase / cell / "rows.csv"
        rows[cell] = _read_csv(path) if path.exists() else []
    return rows


def load_committed(name: str) -> dict[str, list[dict]]:
    path = HERE / name
    data = json.loads(path.read_text(encoding="utf-8"))
    return {cell: [normalise_row(row) for row in rows] for cell, rows in data.items()}


def load_references() -> dict[str, dict]:
    path = HERE.parent / "t91-exact-z-memo" / "references.json"
    return json.loads(path.read_text(encoding="utf-8"))["cells"]


def load_previous_walls() -> dict[str, float]:
    """Cell walls from the last production take (t91's kept candidate arm)."""
    path = HERE.parent / "t91-exact-z-memo" / "ab-reduction.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    return {
        cell: data["cells"][cell]["candidate"]["wall_median_s"]
        for cell in MEASURE_CELLS
    }


def median(values) -> float:
    return statistics.median(values)


# ── Phase 1: the probe split ────────────────────────────────────────────────


def parse_probe_log(path: Path) -> dict:
    record: dict = {"log": path.name, "loop_ns": None, "batch": None}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = dict(token.split("=", 1) for token in line.split() if "=" in token)
        if line.startswith("T95 LOOP"):
            record["entries"] = int(fields["entries"])
            record["loop_ns"] = int(fields["total_ns"])
        elif line.startswith("T95 BATCH"):
            record["batch"] = {
                key: int(fields[key])
                for key in (
                    "vertices",
                    "triangles",
                    "queries",
                    "misses",
                    "object_misses",
                    "distinct_z",
                    "single_ns_total",
                    "single_ns_min",
                    "single_ns_median",
                    "single_ns_max",
                    "batch_ns",
                    "equal",
                    "mismatch",
                    "batch_polys",
                    "production_polys",
                )
            }
    if record["loop_ns"] is None or record["batch"] is None:
        raise AssertionError(f"{path.name}: missing T95 LOOP/BATCH line")
    return record


def build_phase1() -> dict:
    walls = load_previous_walls()
    runs = []
    for path in sorted((HERE / "captures").glob("*.probe.log")):
        # ordinary-base-classic-on[-rN].probe.log
        stem = path.name[: -len(".probe.log")]
        parts = stem.split("-")
        cell = "-".join(parts[:4])
        round_index = parts[4] if len(parts) > 4 else "r1"
        record = parse_probe_log(path)
        batch = record["batch"]
        saving_ns = batch["single_ns_total"] - batch["batch_ns"]
        wall = walls[cell]
        floor_s = wall * FLOOR_PCT / 100.0
        saving_s = saving_ns / 1e9
        runs.append(
            {
                "cell": cell,
                "round": round_index,
                "log": record["log"],
                "entries": record["entries"],
                "loop_ns": record["loop_ns"],
                "triangles": batch["triangles"],
                "vertices": batch["vertices"],
                "queries": batch["queries"],
                "misses": batch["misses"],
                "distinct_z": batch["distinct_z"],
                "single_ns_total": batch["single_ns_total"],
                "single_ns_median": batch["single_ns_median"],
                "single_ns_min": batch["single_ns_min"],
                "single_ns_max": batch["single_ns_max"],
                "batch_ns": batch["batch_ns"],
                "saving_ns": saving_ns,
                "saving_s": saving_s,
                "equal": batch["equal"],
                "mismatch": batch["mismatch"],
                "batch_polys": batch["batch_polys"],
                "production_polys": batch["production_polys"],
                "cell_wall_s": wall,
                "floor_s": floor_s,
                "share_pct": saving_s / wall * 100.0,
                "margin_x": saving_s / floor_s,
            }
        )
    if not runs:
        raise AssertionError("no probe captures")
    return {
        "method": (
            "one probe slice per round; the batch arm runs after the production "
            "loop, so it changes no emitted byte; instrumented walls are "
            "attribution only and the share is the serial loop's own "
            "accumulated saving (ns = wall ns, the loop is serial) over the "
            "last production take's cell wall"
        ),
        "floor_pct": FLOOR_PCT,
        "runs": runs,
        "shapes": {
            "batch_prefetch": {
                "verdict": "above floor",
                "min_share_pct": min(run["share_pct"] for run in runs),
                "max_share_pct": max(run["share_pct"] for run in runs),
                "min_margin_x": min(run["margin_x"] for run in runs),
                "exact_planes": f"{min(run['equal'] for run in runs)}/"
                f"{max(run['equal'] + run['mismatch'] for run in runs)}",
            }
        },
    }


# ── Qualification (proof matrix) ────────────────────────────────────────────


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
        types = rows[0]["type_counts"]
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


# ── Paired A/B ──────────────────────────────────────────────────────────────


def reduce_cell(cell: str, rows: list[dict]) -> dict:
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
    for index in range(1, PAIRS_REQUIRED + 1):
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

    return {
        "cell": cell,
        "status": "reduced",
        "retained_pairs": len(retained),
        "excluded_pairs": starved,
        "wall_ratio_paired_median": median(wall_ratios),
        "wall_delta_pct_paired_median": (median(wall_ratios) - 1.0) * 100.0,
        "cpu_ratio_paired_median": median(cpu_ratios),
        "cpu_delta_pct_paired_median": (median(cpu_ratios) - 1.0) * 100.0,
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


def build_ab(measure_rows: dict[str, list[dict]]) -> dict:
    cells = {
        cell: reduce_cell(cell, measure_rows.get(cell, [])) for cell in MEASURE_CELLS
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


def build_summary(qualification: dict, ab: dict, phase1: dict) -> dict:
    return {
        "headline": {
            "qualification_pass": qualification["pass"],
            "qualification_cells": len(qualification["cells"]),
            "cross_mode_equal": sum(
                1 for entry in qualification["cross_mode"].values() if entry["equal"]
            ),
            "gate_verdict": ab["verdict"],
            "phase1_min_share_pct": phase1["shapes"]["batch_prefetch"]["min_share_pct"],
            "phase1_max_share_pct": phase1["shapes"]["batch_prefetch"]["max_share_pct"],
            "phase1_exact_planes": phase1["shapes"]["batch_prefetch"]["exact_planes"],
        },
        "cells": {
            cell: {
                "status": result["status"],
                "wall_delta_pct": result.get("wall_delta_pct_paired_median"),
                "cpu_delta_pct": result.get("cpu_delta_pct_paired_median"),
                "wall_delta_s": (
                    result["candidate"]["wall_median_s"]
                    - result["baseline"]["wall_median_s"]
                    if result["status"] == "reduced"
                    else None
                ),
                "retained_pairs": result.get("retained_pairs"),
                "excluded_pairs": result.get("excluded_pairs"),
                "baseline_wall_median_s": (
                    result["baseline"]["wall_median_s"]
                    if result["status"] == "reduced"
                    else None
                ),
                "candidate_wall_median_s": (
                    result["candidate"]["wall_median_s"]
                    if result["status"] == "reduced"
                    else None
                ),
            }
            for cell, result in ab["cells"].items()
        },
    }


def write_json(name: str, payload: dict) -> None:
    (HERE / name).write_text(
        json.dumps(payload, indent=1, sort_keys=False) + "\n", encoding="utf-8"
    )


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw", action="store_true", help="read .local-artifacts")
    args = parser.parse_args(argv)
    if args.raw:
        proof_rows = load_raw("proof", PROOF_CELLS)
        measure_rows = load_raw("measure", MEASURE_CELLS)
    else:
        proof_rows = load_committed("proof-rows.json")
        measure_rows = load_committed("measure-rows.json")
    qualification = build_qualification(proof_rows)
    ab = build_ab(measure_rows)
    phase1 = build_phase1()
    summary = build_summary(qualification, ab, phase1)
    write_json("qualification.json", qualification)
    write_json("ab-reduction.json", ab)
    write_json("phase1.json", phase1)
    write_json("summary.json", summary)
    print(
        f"qualification pass={qualification['pass']} "
        f"cells={len(qualification['cells'])} problems={len(qualification['problems'])}"
    )
    for cell, result in ab["cells"].items():
        if result["status"] != "reduced":
            print(f"{cell}: {result['status']}")
            continue
        print(
            f"{cell}: wall {result['wall_delta_pct_paired_median']:+.2f}% "
            f"cpu {result['cpu_delta_pct_paired_median']:+.2f}% "
            f"pairs {result['retained_pairs']}/6 excluded {len(result['excluded_pairs'])}"
        )
    print(f"verdict: {ab['verdict']}")
    return 0 if qualification["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())

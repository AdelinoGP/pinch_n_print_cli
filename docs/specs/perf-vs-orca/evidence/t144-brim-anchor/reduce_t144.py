#!/usr/bin/env python3
"""t144 reducer: campaign rows + audit + oracle -> reduced/summary.json.

Inputs (all under the repository root):

  target/t144/manifest.json        arm manifest (run_t144.ps1 -Mode manifest)
  target/t144/output/rows.csv      baseline/candidate/accelerated output rows
  target/t144/ab/rows.csv          interleaved paired rows (attribution only)
  target/t144/null/rows.csv        same-arm null control rows
  target/t144/audit.json           audit_delta.py output (skirt/brim blast radius)
  target/t144/oracle/<cell>-<arm>.json   oracle_skirt_brim.py outputs

Only stdlib. The summary is the in-repo artifact `verify.py` reads; raw G-code,
stderr and the Orca vendor tree stay gitignored. The active reference set is
read from `../t143-union-grouping/reference.json` so there is one home for it.
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path

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
SUPPORTS_OFF = [
    "benchy-classic-off",
    "benchy-arachne-off",
    "base-classic-off",
    "base-arachne-off",
]
SUPPORTS_ON = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]
ARMS_OUTPUT = ("baseline-ordinary", "candidate-ordinary", "candidate-accelerated")
HERE = Path(__file__).resolve().parent


def load_json(path: Path) -> dict | None:
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def load_csv(path: Path) -> list[dict]:
    if not path.exists():
        return []
    with path.open(encoding="utf-8", newline="") as handle:
        rows = []
        for row in csv.DictReader(handle):
            if not row.get("cell"):
                continue
            row["wall_seconds"] = float(row["wall_seconds"])
            row["cpu_seconds"] = float(row["cpu_seconds"])
            row["pair"] = int(row["pair"])
            row["gcode_bytes"] = int(row["gcode_bytes"])
            row["exit_code"] = int(row["exit_code"])
            rows.append(row)
        return rows


def active_reference() -> tuple[dict, str]:
    data = load_json(HERE.parent / "t143-union-grouping" / "reference.json") or {}
    merged: dict[str, str] = {}
    merged.update(data.get("t139_base", {}))
    merged.update(data.get("benchy_replacement_fixture", {}))
    return merged, data.get("note", "")


def output_summary(rows: list[dict]) -> dict:
    reference, reference_note = active_reference()
    cells: dict[str, dict] = {}
    for row in rows:
        if row["arm"] not in ARMS_OUTPUT:
            continue
        entry = cells.setdefault(row["cell"], {})
        entry[row["arm"]] = {
            "sha256": row["sha256"],
            "gcode_bytes": row["gcode_bytes"],
            "exit_code": row["exit_code"],
            "wall_seconds": row["wall_seconds"],
        }
    for cell, entry in cells.items():
        baseline = entry.get("baseline-ordinary")
        ordinary = entry.get("candidate-ordinary")
        accelerated = entry.get("candidate-accelerated")
        entry["baseline_matches_active_reference"] = bool(
            baseline and baseline["sha256"] == reference.get(cell)
        )
        entry["candidate_equal_across_arms"] = bool(
            ordinary and accelerated and ordinary["sha256"] == accelerated["sha256"]
        )
        entry["candidate_moved_from_baseline"] = bool(
            baseline and ordinary and baseline["sha256"] != ordinary["sha256"]
        )
    candidate_set = {
        cell: entry["candidate-ordinary"]["sha256"]
        for cell, entry in cells.items()
        if "candidate-ordinary" in entry
    }
    inert_cells = sorted(
        cell
        for cell, entry in cells.items()
        if entry.get("candidate_moved_from_baseline") is False
    )
    return {
        "reference_kind": {
            cell: (
                "t139_base"
                if cell.startswith("base-")
                else "benchy_replacement_fixture"
            )
            for cell in CELLS
        },
        "reference_note": reference_note,
        "active_reference": reference,
        "cells": cells,
        "candidate_set": candidate_set,
        "moved_cells": sum(
            1 for entry in cells.values() if entry.get("candidate_moved_from_baseline")
        ),
        "inert_cells": inert_cells,
        "inert_disclosure": (
            "base classic-on / base arachne-on are byte-identical to the baseline: "
            "the support footprint clips the whole brim area, so no brim path (and "
            "therefore no brim point in the skirt hull) reaches the output; the "
            "support clip is out of scope (t138/t139) and unchanged."
        ),
    }


def paired_summary(rows: list[dict]) -> dict:
    by_cell: dict[str, dict[int, dict[str, dict]]] = {}
    for row in rows:
        if row["arm"] not in ("baseline-ordinary", "candidate-ordinary"):
            continue
        by_cell.setdefault(row["cell"], {}).setdefault(row["pair"], {})[row["arm"]] = (
            row
        )

    out: dict[str, dict] = {}
    for cell, pairs in by_cell.items():
        records = []
        for pair in sorted(pairs):
            arms = pairs[pair]
            if "baseline-ordinary" not in arms or "candidate-ordinary" not in arms:
                continue
            base = arms["baseline-ordinary"]
            cand = arms["candidate-ordinary"]
            records.append(
                {
                    "pair": pair,
                    "order": base.get("order", ""),
                    "baseline_wall_s": base["wall_seconds"],
                    "candidate_wall_s": cand["wall_seconds"],
                    "baseline_cpu_s": base["cpu_seconds"],
                    "candidate_cpu_s": cand["cpu_seconds"],
                    "wall_delta_pct": round(
                        (cand["wall_seconds"] - base["wall_seconds"])
                        / base["wall_seconds"]
                        * 100.0,
                        4,
                    ),
                    "cpu_delta_pct": round(
                        (cand["cpu_seconds"] - base["cpu_seconds"])
                        / base["cpu_seconds"]
                        * 100.0,
                        4,
                    ),
                }
            )
        if not records:
            continue
        out[cell] = {
            "pairs": records,
            "pair_count": len(records),
            "wall_delta_pct_median": round(
                statistics.median(r["wall_delta_pct"] for r in records), 4
            ),
            "cpu_delta_pct_median": round(
                statistics.median(r["cpu_delta_pct"] for r in records), 4
            ),
            "baseline_wall_median_s": round(
                statistics.median(r["baseline_wall_s"] for r in records), 4
            ),
            "candidate_wall_median_s": round(
                statistics.median(r["candidate_wall_s"] for r in records), 4
            ),
        }
    return out


def null_summary(rows: list[dict]) -> dict:
    grouped: dict[tuple[str, int], list[dict]] = {}
    for row in rows:
        grouped.setdefault((row["cell"], row["pair"]), []).append(row)
    per_cell: dict[str, list[dict]] = {}
    for (cell, pair), entries in grouped.items():
        if len(entries) != 2:
            continue
        first, second = entries
        per_cell.setdefault(cell, []).append(
            {
                "pair": pair,
                "order": first.get("order", ""),
                "wall_delta_pct": round(
                    (second["wall_seconds"] - first["wall_seconds"])
                    / first["wall_seconds"]
                    * 100.0,
                    4,
                ),
                "cpu_delta_pct": round(
                    (second["cpu_seconds"] - first["cpu_seconds"])
                    / first["cpu_seconds"]
                    * 100.0,
                    4,
                ),
            }
        )
    cells: dict[str, dict] = {}
    for cell, records in per_cell.items():
        cells[cell] = {
            "pairs": records,
            "pair_count": len(records),
            "wall_delta_pct_median": round(
                statistics.median(r["wall_delta_pct"] for r in records), 4
            ),
            "cpu_delta_pct_median": round(
                statistics.median(r["cpu_delta_pct"] for r in records), 4
            ),
            "wall_delta_pct_abs_max": round(
                max(abs(r["wall_delta_pct"]) for r in records), 4
            ),
            "cpu_delta_pct_abs_max": round(
                max(abs(r["cpu_delta_pct"]) for r in records), 4
            ),
        }
    band = {
        "wall_abs_max_pct": round(
            max((c["wall_delta_pct_abs_max"] for c in cells.values()), default=0.0), 4
        ),
        "cpu_abs_max_pct": round(
            max((c["cpu_delta_pct_abs_max"] for c in cells.values()), default=0.0), 4
        ),
    }
    return {"cells": cells, "band_pct": band}


def oracle_summary() -> dict:
    cells: dict[str, dict] = {}
    for cell in SUPPORTS_OFF:
        entry: dict[str, dict] = {}
        for arm in ("baseline", "candidate"):
            data = load_json(
                HERE.parents[4] / "target" / "t144" / "oracle" / f"{cell}-{arm}.json"
            )
            if data is None:
                continue
            checks = data.get("checks", {})

            def pct(key: str):
                value = checks.get(key)
                return None if value is None else round(value * 100.0, 4)

            entry[arm] = {
                "pnp_swept_area_mm2": data["pnp"]["Brim"]["swept_area_mm2"],
                "pnp_brim_outer_area_mm2": data["pnp"]["Brim"]["outer_loop_area_mm2"],
                "pnp_skirt_swept_area_mm2": data["pnp"]["Skirt"]["swept_area_mm2"],
                "pnp_skirt_outer_area_mm2": data["pnp"]["Skirt"]["outer_loop_area_mm2"],
                "orca_swept_area_mm2": data["orca"]["Brim"]["swept_area_mm2"],
                "orca_skirt_outer_area_mm2": data["orca"]["Skirt"][
                    "outer_loop_area_mm2"
                ],
                "brim_swept_area_rel_diff": pct("brim_swept_area_rel_diff"),
                "skirt_swept_area_rel_diff": pct("skirt_swept_area_rel_diff"),
                "skirt_outer_loop_area_rel_diff": pct("skirt_outer_loop_area_rel_diff"),
                # t146 added these: the brim->skirt radial step is derived from
                # them, and the step is what attributes the skirt-hull input
                # defect without needing the gitignored raw capture tree.
                "pnp_brim_growth_mm": data["pnp"]["Brim"].get("growth_mm"),
                "pnp_skirt_growth_mm": data["pnp"]["Skirt"].get("growth_mm"),
                "orca_brim_growth_mm": data["orca"]["Brim"].get("growth_mm"),
                "orca_skirt_growth_mm": data["orca"]["Skirt"].get("growth_mm"),
                "pnp_brim_path_length_mm": data["pnp"]["Brim"].get("path_length_mm"),
                "pnp_skirt_path_length_mm": data["pnp"]["Skirt"].get("path_length_mm"),
                "orca_brim_path_length_mm": data["orca"]["Brim"].get("path_length_mm"),
                "orca_skirt_path_length_mm": data["orca"]["Skirt"].get(
                    "path_length_mm"
                ),
                "pnp_brim_loop_count": data["pnp"]["Brim"].get("loop_count"),
                "pnp_skirt_loop_count": data["pnp"]["Skirt"].get("loop_count"),
                "brim_swept_area_pass": checks.get("brim_swept_area_pass"),
                "skirt_area_pass": checks.get("skirt_area_pass"),
                "pass": checks.get("pass"),
            }
        if entry:
            cells[cell] = entry
    for cell in SUPPORTS_ON:
        cells[cell] = {"status": "support-engine-limited (out of scope, t138/t139)"}
    return {
        "cells": cells,
        "scope": "four supports-off cells vs the Orca reference; supports-on reported as support-engine-limited",
        "bands": {"brim_swept_area_pct": 12.0, "skirt_outer_loop_area_pct": 3.0},
    }


def audit_summary() -> dict | None:
    data = load_json(HERE.parents[4] / "target" / "t144" / "audit.json")
    if data is None:
        return None
    return data


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo-root", type=Path, default=HERE.parents[4], help="repository root"
    )
    parser.add_argument("--out", type=Path, default=HERE / "reduced" / "summary.json")
    args = parser.parse_args()
    root = args.repo_root
    base = root / "target" / "t144"

    manifest = load_json(base / "manifest.json")
    output = output_summary(load_csv(base / "output" / "rows.csv"))
    ab = paired_summary(load_csv(base / "ab" / "rows.csv"))
    null = null_summary(load_csv(base / "null" / "rows.csv"))
    oracle = oracle_summary()
    audit = audit_summary()

    summary = {
        "ticket": 144,
        "manifest": manifest,
        "output": output,
        "ab": ab,
        "null": null,
        "oracle": oracle,
        "audit": audit,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Reduce the ticket 135 raw campaign tree to ``reduced/summary.json``.

Inputs (gitignored raw tree):
  target/t135/output-baseline-relink/rows.csv   baseline arms on the final host binary
  target/t135/output-closing/rows.csv           candidate ordinary (final port)
  target/t135/output-closing-acc/rows.csv       candidate accelerated (final port)
  target/t135/ab3/rows.csv                      interleaved paired A/B (final arms)
  target/t135/manifest.json                     arm exe / skirt-brim wasm hashes
  target/t135/oracle-closing/*.json             parity-oracle results per cell
  target/t135/audit.json                        delta-audit result

Output: ``reduced/summary.json`` (committed; ``verify.py`` needs only this file).
"""

from __future__ import annotations

import csv
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / "target" / "t135"

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


def read_rows(path: Path) -> list[dict]:
    if not path.exists():
        raise SystemExit(f"reduce: missing {path}")
    with path.open(encoding="utf-8") as fh:
        return list(csv.DictReader(fh))


def medians(rows: list[dict]) -> dict:
    wall = [float(r["wall_seconds"]) for r in rows]
    cpu = [float(r["cpu_seconds"]) for r in rows]
    return {
        "n": len(rows),
        "wall_median": round(statistics.median(wall), 4),
        "cpu_median": round(statistics.median(cpu), 4),
        "wall_values": [round(w, 4) for w in wall],
        "cpu_values": [round(c, 4) for c in cpu],
    }


def main() -> None:
    baseline_rows = read_rows(RAW / "output-baseline-relink" / "rows.csv")
    candidate_rows = read_rows(RAW / "output-closing" / "rows.csv") + read_rows(
        RAW / "output-closing-acc" / "rows.csv"
    )
    ab_rows = read_rows(RAW / "ab3" / "rows.csv")
    manifest = json.loads((RAW / "manifest.json").read_text(encoding="utf-8"))

    output: dict[str, dict] = {}
    for cell in CELLS:
        output[cell] = {}
        for arm, rows in (
            ("baseline-ordinary", baseline_rows),
            ("baseline-accelerated", baseline_rows),
            ("candidate-ordinary", candidate_rows),
            ("candidate-accelerated", candidate_rows),
        ):
            row = next((r for r in rows if r["cell"] == cell and r["arm"] == arm), None)
            if row is None:
                raise SystemExit(f"reduce: missing {cell} / {arm}")
            output[cell][arm] = {
                "sha256": row["sha256"],
                "gcode_bytes": int(row["gcode_bytes"]),
                "exit_code": int(row["exit_code"]),
            }

    ab: dict[str, dict] = {}
    for cell in CELLS:
        cell_rows = [r for r in ab_rows if r["cell"] == cell]
        per_arm = {}
        for arm in ("baseline-ordinary", "candidate-ordinary"):
            per_arm[arm] = medians([r for r in cell_rows if r["arm"] == arm])
        pairs = {}
        for pair in sorted({r["pair"] for r in cell_rows}, key=int):
            pair_rows = {r["arm"]: r for r in cell_rows if r["pair"] == pair}
            if len(pair_rows) == 2:
                pairs[pair] = {
                    "wall_baseline": float(
                        pair_rows["baseline-ordinary"]["wall_seconds"]
                    ),
                    "wall_candidate": float(
                        pair_rows["candidate-ordinary"]["wall_seconds"]
                    ),
                    "cpu_baseline": float(
                        pair_rows["baseline-ordinary"]["cpu_seconds"]
                    ),
                    "cpu_candidate": float(
                        pair_rows["candidate-ordinary"]["cpu_seconds"]
                    ),
                    "order": pair_rows["candidate-ordinary"]["order"],
                }
        base = per_arm["baseline-ordinary"]
        cand = per_arm["candidate-ordinary"]
        ab[cell] = {
            "baseline": base,
            "candidate": cand,
            "wall_ratio": round(cand["wall_median"] / base["wall_median"], 4)
            if base["wall_median"]
            else None,
            "cpu_ratio": round(cand["cpu_median"] / base["cpu_median"], 4)
            if base["cpu_median"]
            else None,
            "delta_wall_median_s": round(cand["wall_median"] - base["wall_median"], 4),
            "cpu_signs": sum(
                1 for p in pairs.values() if p["cpu_candidate"] > p["cpu_baseline"]
            ),
            "wall_signs": sum(
                1 for p in pairs.values() if p["wall_candidate"] > p["wall_baseline"]
            ),
            "pairs": pairs,
        }

    oracle = {}
    for cell in CELLS:
        path = RAW / "oracle-closing" / f"{cell}.json"
        if not path.exists():
            raise SystemExit(f"reduce: missing oracle {path}")
        oracle[cell] = json.loads(path.read_text(encoding="utf-8"))

    audit = json.loads((RAW / "audit.json").read_text(encoding="utf-8"))

    scoreboard_rows = read_rows(
        REPO / "target" / "matched-pair" / "t135-scoreboard2" / "scoreboard-rows.csv"
    )
    measured = [r for r in scoreboard_rows if r["warmup"] == "0"]
    scoreboard: dict = {"batch": "t135-scoreboard2", "cells": {}, "resolved": {}, "clean": True}
    for cell in CELLS:
        fixture, generator, supports = cell.split("-")
        entry: dict = {}
        for tool in ("orca", "pnp-ordinary", "pnp-accelerated"):
            rows = [
                r
                for r in measured
                if r["fixture"] == fixture
                and r["generator"] == generator
                and r["supports"] == supports
                and r["tool"] == tool
            ]
            if len(rows) != 3:
                raise SystemExit(f"reduce: scoreboard {cell}/{tool} has {len(rows)} measured rows")
            wall = [float(r["wall_seconds"]) for r in rows]
            cpu = [float(r["cpu_seconds"]) for r in rows]
            entry[tool] = {
                "wall_median": round(statistics.median(wall), 4),
                "cpu_median": round(statistics.median(cpu), 4),
                "wall_values": [round(w, 4) for w in wall],
                "sha256": sorted({r["sha256_of_gcode"] for r in rows}),
                "gcode_bytes": sorted({int(r["gcode_bytes"]) for r in rows}),
                "exit_codes": sorted({int(r["exit_code"]) for r in rows}),
                "degraded": sorted({r["degraded"] for r in rows}),
                "fatal": sorted({int(r["fatal_error_count"]) for r in rows if r["fatal_error_count"] != "n/a"}),
                "resolved": {
                    "brim_width": sorted({r["resolved_brim_width"] for r in rows}),
                    "brim_type": sorted({r["resolved_brim_type"] for r in rows}),
                    "skirt_loops": sorted({r["resolved_skirt_loops"] for r in rows}),
                    "skirt_distance": sorted({r["resolved_skirt_distance"] for r in rows}),
                    "skirt_height": sorted({r["resolved_skirt_height"] for r in rows}),
                },
            }
        entry["ratio_ordinary"] = round(
            entry["pnp-ordinary"]["wall_median"] / entry["orca"]["wall_median"], 4
        )
        entry["ratio_accelerated"] = round(
            entry["pnp-accelerated"]["wall_median"] / entry["orca"]["wall_median"], 4
        )
        entry["required_cut_ordinary"] = round(1.0 - 1.0 / entry["ratio_ordinary"], 4)
        entry["required_cut_accelerated"] = round(1.0 - 1.0 / entry["ratio_accelerated"], 4)
        scoreboard["cells"][cell] = entry
    scoreboard["clean"] = all(
        c[t]["exit_codes"] == [0]
        and c[t]["degraded"] == ["false"]
        and c[t]["fatal"] in ([], [0])
        and len(c[t]["sha256"]) == 1
        for c in scoreboard["cells"].values()
        for t in ("pnp-ordinary", "pnp-accelerated")
    )

    summary = {
        "ticket": 135,
        "stage": 1,
        "manifest": manifest,
        "output": output,
        "ab": ab,
        "oracle": oracle,
        "audit": audit,
        "scoreboard": scoreboard,
    }
    out_dir = HERE / "reduced"
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    print(f"wrote {out_dir / 'summary.json'}")
    print(
        f"cells={len(output)} ab_pairs={sum(len(c['pairs']) for c in ab.values())} "
        f"oracle_cells={len(oracle)} audit_cells={len(audit.get('cells', []))} "
        f"scoreboard_cells={len(scoreboard['cells'])}"
    )


if __name__ == "__main__":
    main()

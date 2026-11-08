#!/usr/bin/env python3
"""t104 phase-split take -- counterfactual arithmetic.

Three committed inputs:

* `phase-split.json` -- phase and stage shares from today's instrumented runs.
* ticket 103's `summary.json` -- the **paired, quiet, today-vs-today** PnP and
  Orca medians that the map's ratios of record come from.
* `walls.csv` -- today's uninstrumented runs, taken under disclosed external
  load. **Corroboration only**: they are never the ratio of record and never
  the counterfactual denominator. Their job is to give the load-inflation
  factor beside the shares and to prove the invocation reproduces the
  reference outputs.

The two measurement regimes are never subtracted from each other. A phase's
implied wall is its **share** of the instrumented `slice_complete` elapsed
applied to the ratio-of-record median, so a counterfactual ratio is always
`(1 - share) x ratio_of_record`.

Usage:
  python arithmetic.py
"""

from __future__ import annotations

import csv
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
T103 = (
    REPO
    / "docs"
    / "specs"
    / "perf-vs-orca"
    / "evidence"
    / "t103-orca-rebaseline"
    / "summary.json"
)


def load_walls(path: Path) -> dict[str, dict]:
    rows = list(csv.DictReader(path.open(encoding="utf-8")))
    by_cell: dict[str, list[float]] = {}
    for r in rows:
        if r["warmup"] == "true" or r["exit_code"] != "0":
            continue
        by_cell.setdefault(r["cell"], []).append(float(r["wall_s"]))
    return {
        cell: {
            "measured": len(v),
            "wall_s": round(statistics.median(v), 3),
            "spread_s": round(max(v) - min(v), 3),
            "runs_s": [round(x, 3) for x in v],
        }
        for cell, v in by_cell.items()
    }


def t103_key(cell: str) -> str:
    fixture, rest = cell.split("-", 1)
    return f"{fixture}/{rest}"


def main() -> int:
    walls = load_walls(HERE / "walls.csv")
    split = json.loads((HERE / "phase-split.json").read_text(encoding="utf-8"))["cells"]
    t103 = json.loads(T103.read_text(encoding="utf-8"))
    park = t103["park_ratio"]

    cells = {}
    for cell, s in sorted(split.items()):
        key = t103_key(cell)
        entry = t103["cells"][key]
        orca = entry["tools"]["orca"]["wall_s"]
        pnp = entry["tools"]["pnp-ordinary"]["wall_s"]
        ratio = pnp / orca
        elapsed = s["slice_complete_ms"]
        phases = {k: (v or 0) for k, v in s["phases_ms"].items()}
        phases["outside_phases"] = s["outside_phases_ms"]
        shares = {k: round(v / elapsed, 6) for k, v in phases.items()}
        implied = {k: round(shares[k] * pnp, 3) for k in phases}
        target = park * orca
        loaded = walls.get(cell)

        out = {
            "pnp_wall_of_record_s": round(pnp, 4),
            "orca_wall_of_record_s": round(orca, 4),
            "ratio_of_record": round(ratio, 2),
            "park_ratio": park,
            "target_s": round(target, 3),
            "required_cut_s": round(pnp - target, 3),
            "required_cut_pct": round(100.0 * (pnp - target) / pnp, 2),
            "phase_shares": shares,
            "phase_implied_wall_s": implied,
            "instrumented_slice_complete_ms": elapsed,
            "layer_count": s["layer_count"],
            "gcode_sha256": s["gcode_sha256"],
            "gcode_bytes": s["gcode_bytes"],
            "degraded": s["degraded"],
            "non_fatal_error_count": s["non_fatal_error_count"],
            "loaded_wall_s": loaded["wall_s"] if loaded else None,
            "loaded_runs_s": loaded["runs_s"] if loaded else None,
            "loaded_measured_runs": loaded["measured"] if loaded else 0,
            "load_inflation": round(loaded["wall_s"] / pnp, 3) if loaded else None,
        }
        for phase in ("prepass", "per_layer", "postpass", "outside_phases"):
            out[f"ratio_if_{phase}_zero"] = round((pnp - implied[phase]) / orca, 2)
        dom = list(s["prepass_stages_ms"].items())[:2]
        out["prepass_dominators"] = [
            {
                "stage": name,
                "ms": ms,
                "share_of_cell": round(ms / elapsed, 6),
                "implied_wall_s": round(ms / elapsed * pnp, 3),
                "ratio_if_zero": round((pnp - ms / elapsed * pnp) / orca, 2),
            }
            for name, ms in dom
        ]
        both = sum(d["implied_wall_s"] for d in out["prepass_dominators"])
        out["ratio_if_both_dominators_zero"] = round((pnp - both) / orca, 2)
        # Floor of any phase-elimination route: every phase removed, only the
        # startup / mesh-IO / teardown residual left.
        out["ratio_if_all_phases_zero"] = round(shares["outside_phases"] * ratio, 2)
        cells[cell] = out

    result = {
        "note": "Derived by arithmetic.py. Phase shares from today's instrumented runs; "
        "ratios and counterfactual denominators from ticket 103's paired quiet "
        "today-vs-today medians; today's uninstrumented walls are corroboration only "
        "(external load, disclosed) and enter as load_inflation.",
        "park_ratio": park,
        "cells": cells,
    }
    (HERE / "counterfactuals.json").write_text(
        json.dumps(result, indent=1) + "\n", encoding="utf-8"
    )
    for cell, c in cells.items():
        print(
            f"{cell}: {c['ratio_of_record']}x (pnp {c['pnp_wall_of_record_s']}s / "
            f"orca {c['orca_wall_of_record_s']}s), cut {c['required_cut_pct']}%; "
            f"prepass {100 * c['phase_shares']['prepass']:.1f}% -> {c['ratio_if_prepass_zero']}x if zero; "
            f"per-layer {100 * c['phase_shares']['per_layer']:.1f}% -> "
            f"{c['ratio_if_per_layer_zero']}x if zero; load x{c['load_inflation']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

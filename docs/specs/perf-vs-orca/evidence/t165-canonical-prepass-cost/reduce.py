#!/usr/bin/env python3
"""t165 reduce: runs.csv + t164's board/split -> reduced.json.

Canonical prepass per class = median(t_tmp over the 3 measured slice runs)
                             - median(--info wall over the 3 measured info runs).
A value below 3x the class's worst measured poll gap is a BOUND, not a point
(t110's rule).

The slack table re-runs t164's own substitution (`arithmetic.py`): every phase
other than the prepass stays at t164's measured share x t164's wall of record,
and only the canonical prepass input changes. That is an ESTIMATE, not a ratio of
record. A window-scaled sensitivity column multiplies the new input by
(t164 Orca classic wall / t165 Orca classic wall) for the class.

Usage: python reduce.py [--out reduced.json]
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
T164 = HERE.parent / "t164-phase-re-attribution" / "reduced"
T110_S = {"benchy-off": 0.0505, "benchy-on": 0.5498, "base-off": 2.1030, "base-on": 10.4448}
CLASSES = ["benchy-off", "benchy-on", "base-off", "base-on"]
CELLS = [f"{f}-{g}-{s}" for f in ("benchy", "base") for g in ("classic", "arachne") for s in ("off", "on")]


def load_rows(path: Path) -> list[dict]:
    with path.open(encoding="utf-8") as fh:
        return list(csv.DictReader(fh))


def f(x: str) -> float:
    return float(x)


def reduce(rows: list[dict], board: dict, split: dict) -> dict:
    meas = [r for r in rows if r["measured"] == "1"]
    agree = [abs(f(r["wall_agree_pct"])) for r in rows]
    exe = sorted({r["exe_sha256"] for r in rows})
    info = {m: [f(r["tap_wall_ms"]) for r in meas if r["cls"] == f"info-{m}"] for m in ("benchy", "base")}
    info_cpu = {m: [f(r["cpu_final_s"]) for r in meas if r["cls"] == f"info-{m}"] for m in ("benchy", "base")}
    classes = {}
    for c in CLASSES:
        model = c.split("-")[0]
        rs = [r for r in meas if r["cls"] == c]
        tmp = [f(r["t_tmp_ms"]) for r in rs]
        ren = [f(r["t_rename_ms"]) for r in rs]
        wall = [f(r["tap_wall_ms"]) for r in rs]
        cpu_tmp = [f(r["cpu_at_tmp_s"]) for r in rs]
        gap = max(f(r["poll_max_gap_ms"]) for r in rs)
        info_med = statistics.median(info[model])
        prepass = statistics.median(tmp) - info_med
        per_round = [round(t - i, 1) for t, i in zip(tmp, info[model])]
        classes[c] = {
            "runs": len(rs),
            "t_tmp_ms": tmp,
            "info_ms": info[model],
            "info_median_ms": info_med,
            "t_tmp_median_ms": statistics.median(tmp),
            "prepass_ms": round(prepass, 1),
            "prepass_per_round_ms": per_round,
            "prepass_per_round_range_ms": round(max(per_round) - min(per_round), 1),
            "emit_median_ms": round(statistics.median([b - a for a, b in zip(tmp, ren)]), 1),
            "wall_median_ms": statistics.median(wall),
            "prepass_cpu_s": round(statistics.median(cpu_tmp) - statistics.median(info_cpu[model]), 3),
            "worst_poll_gap_ms": gap,
            "label": "point" if prepass >= 3 * gap else "bound",
            "t110_prepass_ms": round(T110_S[c] * 1000, 1),
            "vs_t110": round(prepass / (T110_S[c] * 1000), 4),
        }
    cells = {}
    for cell in CELLS:
        fx, gen, sup = cell.split("-")
        cls = f"{fx}-{sup}"
        b = board["cells"][cell]
        shares = {k: round(v, 4) for k, v in split["cells"][cell]["median"]["shares"].items()}
        orca_s, pnp_s = b["orca"]["wall_median_s"], b["pnp_ordinary"]["wall_median_s"]
        other_s = pnp_s - shares["prepass"] * pnp_s
        budget = orca_s - other_s
        canon = classes[cls]["prepass_ms"] / 1000.0
        classic_t164 = board["cells"][f"{fx}-classic-{sup}"]["orca"]["wall_median_s"]
        scale = classic_t164 / (classes[cls]["wall_median_ms"] / 1000.0)
        cells[cell] = {
            "estimate": True,
            "orca_s_t164": orca_s,
            "prepass_budget_s": round(budget, 4),
            "canonical_prepass_s_t110": T110_S[cls],
            "canonical_prepass_s_t165": round(canon, 4),
            "slack_t110": round(budget / T110_S[cls], 4),
            "slack_t165": round(budget / canon, 4),
            "result_t110": round((T110_S[cls] + other_s) / orca_s, 4),
            "result_t165": round((canon + other_s) / orca_s, 4),
            "window_scale": round(scale, 4),
            "result_t165_window_scaled": round((canon * scale + other_s) / orca_s, 4),
        }
    binding = max(cells, key=lambda c: cells[c]["result_t165"])
    return {
        "note": "ESTIMATE, not a ratio of record. Prepass = median t_tmp - median --info wall; "
                "substitution = t164's arithmetic with only the canonical prepass input changed.",
        "exe_sha256": exe,
        "rows": len(rows),
        "measured_rows": len(meas),
        "max_abs_wall_agree_pct": round(max(agree), 3),
        "info_median_ms": {m: statistics.median(v) for m, v in info.items()},
        "classes": classes,
        "cells": cells,
        "summary": {
            "cells_winning": sum(1 for c in cells.values() if c["result_t165"] < 1.0),
            "cells_winning_window_scaled": sum(1 for c in cells.values() if c["result_t165_window_scaled"] < 1.0),
            "binding_cell": binding,
            "binding_result": cells[binding]["result_t165"],
            "binding_slack": cells[binding]["slack_t165"],
            "min_slack": min(c["slack_t165"] for c in cells.values()),
        },
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(HERE / "reduced.json"))
    a = ap.parse_args()
    rows = load_rows(HERE / "runs.csv")
    board = json.loads((T164 / "board.json").read_text(encoding="utf-8"))
    split = json.loads((T164 / "instrumented.json").read_text(encoding="utf-8"))
    out = reduce(rows, board, split)
    Path(a.out).write_text(json.dumps(out, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(out["summary"], indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

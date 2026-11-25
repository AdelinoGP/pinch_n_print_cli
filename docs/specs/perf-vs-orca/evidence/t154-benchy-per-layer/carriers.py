#!/usr/bin/env python3
"""t154 per-layer carrier table -- from the committed instrumented split.

Per cell it ranks the per-layer carriers by thread-accumulated module time and
prices each as a **wall-equivalent ceiling**: `thread_sum / measured effective
parallelism`. That charges 100 % of the carrier's parallel work to the critical
path, so it is an upper bound, never a price. It also states each carrier's
share of the cell's implied wall of record (from `arithmetic.json`) and the
carrier's multiple of the 0.80 % floor.

A carrier that is a *stage-shared host cost* is not a module's own work: t127
proved by declining `raft-default` that the first-dispatched `Layer::Infill`
module's bracket pays the stage's preparation. This table therefore labels the
first Infill-stage module's bracket as `stage_shared_preparation` on the
strength of that measured transfer, and never claims the split inside it.

Usage:
  python carriers.py            # writes carriers.json
"""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
FLOOR = 0.008  # 0.80 % of the cell wall

def main() -> int:
    summary = json.loads((HERE / "summary.json").read_text(encoding="utf-8"))["cells"]
    arith = json.loads((HERE / "arithmetic.json").read_text(encoding="utf-8"))["cells"]
    out = {}
    for cell, red in sorted(summary.items()):
        wall = red["phases_ms"]["per_layer"]
        mods = red["module_thread_sum_ms"]["per_layer"]
        thread_sum = sum(mods.values())
        parallel = thread_sum / wall if wall else None
        cell_wall = arith[cell]["pnp_s"]
        rows = []
        for module_id, thread_ms in mods.items():
            ceiling_ms = thread_ms / parallel if parallel else None
            rows.append(
                {
                    "module_id": module_id,
                    "thread_sum_ms": thread_ms,
                    "share_of_per_layer_thread": round(thread_ms / thread_sum, 4),
                    "wall_ceiling_ms": round(ceiling_ms, 1),
                    "share_of_cell_wall": round(ceiling_ms / (cell_wall * 1000), 4),
                    "floor_multiple": round(
                        (ceiling_ms / (cell_wall * 1000)) / FLOOR, 2
                    ),
                    "stage_shared_preparation": module_id == "com.core.raft-default",
                }
            )
        out[cell] = {
            "per_layer_wall_ms": wall,
            "per_layer_thread_sum_ms": thread_sum,
            "effective_parallelism": round(parallel, 2),
            "cell_wall_s_estimate": cell_wall,
            "carriers": sorted(rows, key=lambda r: -r["thread_sum_ms"]),
        }
    payload = {
        "note": (
            "Wall-equivalent ceilings are thread_sum / measured effective parallelism: "
            "they charge 100 % of a carrier's parallel work to the critical path, so they "
            "are upper bounds, never prices. Cell walls are the arithmetic.json estimates."
        ),
        "cells": out,
    }
    (HERE / "carriers.json").write_text(
        json.dumps(payload, indent=1) + "\n", encoding="utf-8"
    )
    for cell, r in out.items():
        top = r["carriers"][0]
        print(
            f"{cell:20s} wall {r['per_layer_wall_ms']:5d} ms  parallel {r['effective_parallelism']:5.2f}x  "
            f"top {top['module_id'].replace('com.core.', '')} {100 * top['share_of_per_layer_thread']:.1f}%  "
            f"ceiling {top['share_of_cell_wall'] * 100:.1f}% of cell ({top['floor_multiple']:.1f}x floor)"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

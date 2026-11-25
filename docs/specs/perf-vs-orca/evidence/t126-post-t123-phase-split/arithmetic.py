#!/usr/bin/env python3
"""t126 -- counterfactual arithmetic against ticket 124's ratios of record.

Inputs (both committed):
  summary.json                              reduced instrumented shares (this take)
  ../t124-post-t123-scoreboard/scoreboard-summary.json   paired medians of record

Output: counterfactuals.json

Every denominator is t124's ordinary PnP median wall and its Orca median; the
shares come from this take's instrumented passes. An instrumented wall is never
used as a wall, and a per-layer thread sum is never used as wall time.

Usage:
  python arithmetic.py [--summary summary.json] [--out counterfactuals.json]
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
T124 = HERE.parent / "t124-post-t123-scoreboard" / "scoreboard-summary.json"
FLOOR = 0.008  # 0.80 % of the cell's own median wall


def cell_key(name: str) -> str:
    fixture, generator, supports = name.split("-", 2)
    return f"{fixture}/{generator}/{supports}"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--summary", default=str(HERE / "summary.json"))
    ap.add_argument("--t124", default=str(T124))
    ap.add_argument("--out", default=str(HERE / "counterfactuals.json"))
    a = ap.parse_args()

    summary = json.loads(Path(a.summary).read_text(encoding="utf-8"))["cells"]
    record = json.loads(Path(a.t124).read_text(encoding="utf-8"))["cells"]

    cells = {}
    for name, cell in summary.items():
        key = cell_key(name)
        rec = record[key]
        pnp = rec["pnp-ordinary"]["wall_s"]
        orca = rec["orca"]["wall_s"]
        ratio = rec["ordinary_ratio"]
        complete = cell["slice_complete_ms"]
        shares = {
            phase: (ms or 0) / complete for phase, ms in cell["phases_ms"].items()
        }
        shares["outside"] = cell["outside_phases_ms"] / complete
        floor_ms = FLOOR * pnp * 1000.0

        def if_zero(*phases: str) -> float:
            return ratio * (1.0 - sum(shares[p] for p in phases))

        detail = cell["phase_detail"].get("per_layer", {})
        per_layer_gap_share = shares.get("per_layer", 0.0) * (
            detail.get("host_gap_fraction") or 0.0
        )
        per_layer_thread_sum = sum(cell["per_layer_stages_thread_sum_ms"].values())
        per_layer_wall = cell["phases_ms"].get("per_layer") or 0
        parallelism = per_layer_thread_sum / per_layer_wall if per_layer_wall else None
        postpass_thread_sum = sum(
            cell["module_thread_sum_ms"].get("postpass", {}).values()
        )
        top_modules = {}
        for mid, thread_sum in list(
            cell["module_thread_sum_ms"].get("per_layer", {}).items()
        )[:12]:
            wall_equivalent = thread_sum / parallelism if parallelism else None
            top_modules[mid] = {
                "calls": cell["module_calls"]["per_layer"].get(mid),
                "thread_sum_ms": thread_sum,
                "wall_equivalent_ceiling_ms": round(wall_equivalent, 1)
                if wall_equivalent is not None
                else None,
                "floor_multiple_ceiling": round(wall_equivalent / floor_ms, 3)
                if wall_equivalent is not None and floor_ms
                else None,
            }
        cells[name] = {
            "fixture": name.split("-")[0],
            "generator": name.split("-")[1],
            "supports": name.split("-")[2],
            "pnp_ordinary_median_s": pnp,
            "orca_median_s": orca,
            "ratio_of_record": ratio,
            "cut_to_beat": 1.0 - 1.0 / ratio,
            "cut_to_1_5x": 1.0 - 1.5 / ratio,
            "share": {k: round(v, 6) for k, v in shares.items()},
            "implied_wall_s": {k: round(v * pnp, 3) for k, v in shares.items()},
            "ratio_if_prepass_zero": if_zero("prepass"),
            "ratio_if_per_layer_zero": if_zero("per_layer"),
            "ratio_if_prepass_and_per_layer_zero": if_zero("prepass", "per_layer"),
            "floor_ms": round(floor_ms, 1),
            "per_layer_thread_sum_ms": per_layer_thread_sum,
            "per_layer_parallelism": round(parallelism, 3) if parallelism else None,
            "per_layer_host_gap_share": round(per_layer_gap_share, 6),
            "per_layer_host_gap_implied_ms": round(
                per_layer_gap_share * pnp * 1000.0, 1
            ),
            "per_layer_host_gap_floor_multiple": round(
                (per_layer_gap_share * pnp * 1000.0) / floor_ms, 3
            )
            if floor_ms
            else None,
            "postpass_thread_sum_ms": postpass_thread_sum,
            "top_per_layer_modules": top_modules,
            "top_postpass_modules_thread_sum_ms": dict(
                list(cell["module_thread_sum_ms"].get("postpass", {}).items())[:8]
            ),
            "per_layer_stages_thread_sum_ms": cell["per_layer_stages_thread_sum_ms"],
            "postpass_stages_ms": cell["postpass_stages_ms"],
        }

    out = {
        "note": (
            "Shares from t126's instrumented passes; denominators are ticket "
            "124's paired medians of record. Instrumented walls are not wall "
            "claims and per-layer thread sums are not wall time."
        ),
        "floor_fraction": FLOOR,
        "cells": cells,
    }
    Path(a.out).write_text(json.dumps(out, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {a.out}: {len(cells)} cells")
    for name, c in cells.items():
        print(
            f"{name}: ratio {c['ratio_of_record']:.2f}x, cut {100 * c['cut_to_beat']:.0f}%, "
            f"prepass {100 * c['share']['prepass']:.1f}%, per-layer {100 * c['share']['per_layer']:.1f}%, "
            f"if prepass 0 -> {c['ratio_if_prepass_zero']:.2f}x, "
            f"per-layer host gap {c['per_layer_host_gap_implied_ms']:.0f} ms "
            f"({c['per_layer_host_gap_floor_multiple']:.2f}x floor)"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

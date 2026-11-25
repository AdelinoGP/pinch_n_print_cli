#!/usr/bin/env python3
"""t131 -- derive the report's arithmetic from `summary.json`.

Every number quoted in FINDINGS.md is printed here from the reduced set, so a
reader can re-derive it without the probe build. `verify.py` re-runs the same
functions and asserts the FINDINGS prose against them.

Usage:
  python arithmetic.py [--summary summary.json]
"""

from __future__ import annotations

import argparse
import collections
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]
OP_ORDER = ["clip", "offset", "offset2_p1", "offset2_p2"]


def ms(value_ns: float) -> float:
    return value_ns / 1e6


def stage_rows(summary: dict, cell: str) -> list[dict]:
    rec = summary["cells"][cell]
    rows = []
    for stage, ops in rec["t131_stage"].items():
        if stage.startswith("PHASE"):
            continue
        calls = sum(p["calls"] for p in ops.values())
        wit = sum(p["wit_in_ns"] + p["wit_out_ns"] for p in ops.values())
        conv = sum(p["conv_in_ns"] for p in ops.values())
        sweep = sum(p["sweep_ns"] for p in ops.values())
        rebuild = sum(p["rebuild_ns"] for p in ops.values())
        in_points = sum(p["in_points"] for p in ops.values())
        out_points = sum(p["out_points"] for p in ops.values())
        overhead = wit + conv + rebuild
        wall = rec["stage_walls_ms"].get(stage, {}).get("sum_ms", 0.0)
        wall_calls = rec["stage_walls_ms"].get(stage, {}).get("calls", 0)
        if wall_calls > 1:  # per-layer stage: sum is the thread-sum, max is wall
            wall = rec["stage_walls_ms"][stage]["max_ms"]
        rows.append(
            {
                "stage": stage,
                "wall_ms": wall,
                "calls": calls,
                "sweep_ms": ms(sweep),
                "overhead_ms": ms(overhead),
                "wit_ms": ms(wit),
                "conv_ms": ms(conv),
                "rebuild_ms": ms(rebuild),
                "thread_sum_ms": ms(sweep + overhead),
                "sweep_share": sweep / (sweep + overhead) if sweep + overhead else 0.0,
                "points_per_call": in_points / calls if calls else 0.0,
                "in_points": in_points,
                "out_points": out_points,
                "parallelism": (ms(sweep + overhead) / wall) if wall else 0.0,
            }
        )
    return rows


def phase_rows(summary: dict, cell: str) -> list[dict]:
    rec = summary["cells"][cell]
    rows = []
    for stage in ("PHASE::PerLayer", "PHASE::PostPass"):
        ops = rec["t131_stage"].get(stage, {})
        calls = sum(p["calls"] for p in ops.values())
        if not calls:
            continue
        sweep = sum(p["sweep_ns"] for p in ops.values())
        overhead = sum(
            p["wit_in_ns"] + p["conv_in_ns"] + p["rebuild_ns"] + p["wit_out_ns"]
            for p in ops.values()
        )
        rows.append(
            {
                "stage": stage,
                "calls": calls,
                "sweep_ms": ms(sweep),
                "overhead_ms": ms(overhead),
                "sweep_share": sweep / (sweep + overhead) if sweep + overhead else 0.0,
            }
        )
    return rows


def s5_rows(summary: dict, cell: str) -> dict | None:
    rec = summary["cells"].get(f"{cell}@s5")
    return rec["s5"] if rec else None


def s5_wall_saving_ms(summary: dict, cell: str, parallelism: float) -> float:
    """S5 thread-time saving divided by the stage's measured parallelism."""
    s5 = s5_rows(summary, cell)
    if not s5:
        return 0.0
    saved = (s5["sum_whole_pos_ms"] - s5["sum_win_pos_ms"]) + (
        s5["sum_whole_neg_ms"] - s5["sum_win_neg_ms"]
    )
    return saved / parallelism if parallelism else 0.0


def s6_family(summary: dict, cell: str) -> dict:
    """Content-census families of the singular-offset imports.

    `occupancy` is the +0.35 mm Miter/no-miter family (`support_object_xy_distance`
    at the matched job); `collision` aggregates the paired -0.6 / -0.8 mm family.
    """
    fams = summary["cells"][cell].get("s6_families") or {}
    out = {}
    occ = fams.get("0.350000|none|0.000000")
    if occ:
        out["occupancy"] = occ
    collision = {"calls": 0, "distinct_payloads": 0, "repeat_calls": 0, "total_ms": 0.0}
    for key in ("-0.800000|none|0.000000", "-0.600000|none|0.000000"):
        f = fams.get(key)
        if not f:
            continue
        for field in collision:
            collision[field] += f[field]
    if collision["calls"]:
        out["collision"] = collision
    return out


def s6_saving_ms(family: dict) -> float:
    """Memo saving: (calls - distinct) / calls * total, split evenly per call."""
    calls = family.get("calls", 0)
    distinct = family.get("distinct_payloads", 0)
    total = family.get("total_ms", 0.0)
    if not calls:
        return 0.0
    return total * (calls - distinct) / calls


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--summary", default=str(HERE / "summary.json"))
    a = ap.parse_args()
    summary = json.loads(Path(a.summary).read_text(encoding="utf-8"))

    for cell in CELLS:
        print(f"\n=== {cell}")
        print(
            f"  phase walls: "
            + ", ".join(
                f"{k}={v:.0f}ms"
                for k, v in summary["cells"][cell]["phase_walls_ms"].items()
            )
        )
        for row in stage_rows(summary, cell):
            print(
                f"  {row['stage']:30s} wall={row['wall_ms']:9.0f}  calls={row['calls']:7.0f}  "
                f"sweep={row['sweep_ms']:9.1f}  overhead={row['overhead_ms']:8.1f}  "
                f"sweep_share={100 * row['sweep_share']:6.2f}%  pts/call={row['points_per_call']:8.1f}  "
                f"par={row['parallelism']:5.2f}x"
            )
        for row in phase_rows(summary, cell):
            print(
                f"  {row['stage']:30s} calls={row['calls']:7.0f}  sweep={row['sweep_ms']:9.1f}  "
                f"overhead={row['overhead_ms']:8.1f}  sweep_share={100 * row['sweep_share']:6.2f}%"
            )
        s5 = s5_rows(summary, cell)
        if s5:
            par = next(
                (
                    r["parallelism"]
                    for r in stage_rows(summary, cell)
                    if r["stage"] == "PrePass::ShellClassification"
                ),
                1.0,
            )
            pos_ratio = s5["sum_whole_pos_ms"] / s5["sum_win_pos_ms"]
            neg_ratio = (
                s5["sum_whole_neg_ms"] / s5["sum_win_neg_ms"]
                if s5["sum_win_neg_ms"]
                else 0.0
            )
            print(
                f"  S5 layers={s5['layers']} (neg ran {s5['neg_ran_layers']})  "
                f"pos {s5['sum_whole_pos_ms']:.1f} -> {s5['sum_win_pos_ms']:.1f} ms ({pos_ratio:.2f}x)  "
                f"neg {s5['sum_whole_neg_ms']:.1f} -> {s5['sum_win_neg_ms']:.1f} ms ({neg_ratio:.2f}x)  "
                f"wall saving at {par:.2f}x = {s5_wall_saving_ms(summary, cell, par):.0f} ms  "
                f"exact pos {s5['exact_pos_layers']}/{s5['layers']} neg {s5['exact_neg_layers']}/{s5['neg_ran_layers']}  "
                f"max_abs_delta pos {s5['max_abs_pos_area_delta_mm2']:.6f} neg {s5['max_abs_neg_area_delta_mm2']:.6f} mm2"
            )
        s6 = summary["cells"][cell].get("s6")
        if s6:
            print(
                f"  S6 all singular offsets: calls={s6['calls']} distinct={s6['distinct_payloads']} "
                f"repeat={s6['repeat_calls']} total={s6['total_ms']:.1f} ms"
            )
        fam = s6_family(summary, cell)
        for name in ("occupancy", "collision"):
            if name in fam:
                f = fam[name]
                print(
                    f"  S6 {name}: calls={f['calls']} distinct={f['distinct_payloads']} "
                    f"repeat={f['repeat_calls']} total={f['total_ms']:.1f} ms "
                    f"memo saving={s6_saving_ms(f):.1f} ms"
                )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

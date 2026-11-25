#!/usr/bin/env python3
"""t164 -- compare this take's split with t154's, cell by cell.

Inputs (both committed):
  ../t154-benchy-per-layer/summary.json   t154's instrumented split (2026-11-14)
  reduced/instrumented.json               this take's split

Shares are within-cell ratios, so this comparison is an attribution diff, not a
wall comparison: the two takes were measured on different trees (t154 predates
t158) and in different windows. A share movement is evidence that the phase mix
changed; it is not a claim about either cell's wall.

Usage:
  python compare_t154.py [--out reduced/split-delta.json]
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
T154 = HERE.parent / "t154-benchy-per-layer" / "summary.json"
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
PHASES = ["prepass", "per_layer", "postpass", "validation", "outside"]


def shares_of_t154(row: dict) -> dict[str, float]:
    total = row["slice_complete_ms"]
    out = {k: v / total for k, v in row["phases_ms"].items()}
    out["outside"] = row["outside_phases_ms"] / total
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(HERE / "reduced" / "split-delta.json"))
    a = ap.parse_args()

    old = json.loads(T154.read_text(encoding="utf-8"))["cells"]
    new = json.loads(
        (HERE / "reduced" / "instrumented.json").read_text(encoding="utf-8")
    )["cells"]

    out: dict[str, dict] = {}
    for cell in CELLS:
        old_shares = shares_of_t154(old[cell])
        new_shares = new[cell]["median"]["shares"]
        delta = {
            p: round(100.0 * (new_shares.get(p, 0.0) - old_shares.get(p, 0.0)), 2)
            for p in PHASES
        }
        out[cell] = {
            "t154_slice_complete_ms": old[cell]["slice_complete_ms"],
            "t164_slice_complete_ms": new[cell]["median"]["slice_complete_ms"],
            "t154_shares": {p: round(old_shares.get(p, 0.0), 4) for p in PHASES},
            "t164_shares": {p: round(new_shares.get(p, 0.0), 4) for p in PHASES},
            "delta_pp": delta,
        }

    payload = {
        "note": (
            "Share deltas in percentage points, t164 minus t154. Attribution only: "
            "the takes are on different trees and in different windows, so a share "
            "movement is not a wall claim and the instrumented walls are not "
            "comparable."
        ),
        "cells": out,
    }
    out_path = Path(a.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {a.out}")
    print(f"  {'cell':20s} " + " ".join(f"{p:>9s}" for p in PHASES))
    for cell in CELLS:
        d = out[cell]["delta_pp"]
        print(f"  {cell:20s} " + " ".join(f"{d[p]:+8.2f}" for p in PHASES))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

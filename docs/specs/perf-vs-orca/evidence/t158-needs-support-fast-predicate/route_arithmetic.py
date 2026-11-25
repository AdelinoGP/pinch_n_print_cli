#!/usr/bin/env python3
"""Compose t158's measured wall deltas with t154's route ledger.

The result is an indicative cross-window composition, not a fresh Orca
scoreboard. The canonical-prepass projection subtracts the whole measured paired
wall saving from t154's projected PnP wall; t157's estimate is not added again.
"""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
T154 = HERE.parent / "t154-benchy-per-layer" / "arithmetic.json"
CAMPAIGN = HERE / "reduced" / "campaign.json"
OUTPUT = HERE / "route_arithmetic.json"


def main() -> int:
    prior = json.loads(T154.read_text(encoding="utf-8"))["cells"]
    campaign = json.loads(CAMPAIGN.read_text(encoding="utf-8"))
    cells = {}
    for cell in sorted(prior):
        old = prior[cell]
        measured = campaign["ab"][cell]
        saving_pct = float(measured["wall_saving_pct"])
        saving_s = -float(measured["wall_delta_s"])
        old_ratio = float(old["ratio_of_record"])
        new_ratio = old_ratio * (1.0 - saving_pct / 100.0)
        old_canonical = float(old["prepass_at_canonical_result"])
        orca_s = float(old["orca_s"])
        new_canonical = (old_canonical * orca_s - saving_s) / orca_s
        cells[cell] = {
            "estimate": True,
            "orca_s": orca_s,
            "pre_t158_ratio": round(old_ratio, 6),
            "t158_wall_saving_pct": round(saving_pct, 4),
            "t158_wall_saving_s": round(saving_s, 6),
            "post_t158_ratio": round(new_ratio, 6),
            "pre_t158_required_cut_pct": round((1.0 - 1.0 / old_ratio) * 100.0, 2),
            "post_t158_required_cut_pct": round((1.0 - 1.0 / new_ratio) * 100.0, 2),
            "pre_t158_canonical_prepass_result": round(old_canonical, 6),
            "post_t158_canonical_prepass_result": round(new_canonical, 6),
        }
    payload = {
        "note": (
            "ESTIMATE, not a new scoreboard of record. Ratios compose t154's "
            "cross-window ledger with t158's measured separate-arm median wall "
            "deltas. Canonical-prepass results subtract the whole measured wall "
            "saving from t154's projected PnP wall; no t157 estimate is added."
        ),
        "cells": cells,
    }
    OUTPUT.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT.name}: {len(cells)} cells")
    for cell, row in cells.items():
        print(
            f"  {cell:20s} ratio {row['pre_t158_ratio']:.4f}x -> "
            f"{row['post_t158_ratio']:.4f}x; canonical prepass "
            f"{row['pre_t158_canonical_prepass_result']:.4f}x -> "
            f"{row['post_t158_canonical_prepass_result']:.4f}x"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

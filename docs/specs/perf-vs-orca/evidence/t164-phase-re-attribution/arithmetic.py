#!/usr/bin/env python3
"""t164 phase re-attribution -- the destination arithmetic on one window's inputs.

Inputs (both committed):
  - `reduced/board.json`: this take's paired scoreboard, 8 cells x {Orca, PnP
    ordinary}, 1 warmup + 3 measured, interleaved. Walls of record.
  - `reduced/instrumented.json`: this take's instrumented split, three passes per
    cell, reduced to a per-phase median share. Attribution only.

Fixed input (not a wall, held at its source value so the substitution is the
same one t154 performed):
  - canonical prepass cost, from t110's tap-derived phase split. `benchy-off` is
    a bound, not a point.

The substitution holds every phase other than the prepass at this take's
measured share and replaces the prepass with canonical's cost. The result is an
ESTIMATE of that substitution, not a measurement of a candidate; nothing here is
a ratio of record.

Usage:
  python arithmetic.py [--board ...] [--split ...] [--out reduced/arithmetic.json]
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
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

# Canonical prepass wall (seconds) by fixture/supports class, from t110's
# tap-derived split. Held fixed: this is the substitution's input, not a
# measurement of this window.
CANONICAL_PREPASS_S = {
    "benchy-off": 0.0505,
    "benchy-on": 0.5498,
    "base-off": 2.1030,
    "base-on": 10.4448,
}


def canonical_prepass(cell: str) -> float:
    fixture, _generator, supports = cell.split("-", 2)
    return CANONICAL_PREPASS_S[f"{fixture}-{supports}"]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--board", default=str(HERE / "reduced" / "board.json"))
    ap.add_argument("--split", default=str(HERE / "reduced" / "instrumented.json"))
    ap.add_argument("--out", default=str(HERE / "reduced" / "arithmetic.json"))
    a = ap.parse_args()

    board = json.loads(Path(a.board).read_text(encoding="utf-8"))
    split = json.loads(Path(a.split).read_text(encoding="utf-8"))

    out: dict[str, dict] = {}
    for cell in CELLS:
        b = board["cells"][cell]
        m = split["cells"][cell]["median"]
        orca_s = b["orca"]["wall_median_s"]
        pnp_s = b["pnp_ordinary"]["wall_median_s"]
        # All arithmetic is derived from the 4-decimal shares that are written
        # out, so verify.py can reproduce every figure from the artifact itself.
        shares = {k: round(v, 4) for k, v in m["shares"].items()}
        prepass_share = shares["prepass"]
        prepass_s = prepass_share * pnp_s
        # Everything the prepass does not own, at today's measured share.
        other_s = pnp_s - prepass_s
        canonical_s = canonical_prepass(cell)
        result = (canonical_s + other_s) / orca_s
        # The prepass budget that would exactly tie Orca.
        prepass_budget_s = orca_s - other_s
        required_cut = 1.0 - (orca_s / pnp_s)
        required_prepass_cut = (
            1.0 - prepass_budget_s / prepass_s if prepass_s > 0 else None
        )
        needed_from_other = max(0.0, (canonical_s + other_s) - orca_s)
        out[cell] = {
            "estimate": True,
            "orca_s": orca_s,
            "pnp_s": pnp_s,
            "ratio_of_record": round(b["ratio_ordinary_over_orca"], 4),
            "required_cut": round(required_cut, 4),
            "phase_shares": shares,
            "implied_phase_walls_s": {
                k: round(v * pnp_s, 4) for k, v in shares.items()
            },
            "prepass_s": round(prepass_s, 4),
            "other_than_prepass_s": round(other_s, 4),
            "canonical_prepass_s": canonical_s,
            "prepass_budget_s": round(prepass_budget_s, 4),
            "required_prepass_cut": round(required_prepass_cut, 4)
            if required_prepass_cut is not None
            else None,
            "prepass_at_canonical_result": round(result, 4),
            "margin_at_canonical": round(1.0 - result, 4),
            "needed_from_other_than_prepass_s": round(needed_from_other, 4),
            "needed_from_other_fraction_of_per_layer": (
                round(needed_from_other / (shares["per_layer"] * pnp_s), 4)
                if shares.get("per_layer")
                else None
            ),
            "per_layer_s": round(shares.get("per_layer", 0.0) * pnp_s, 4),
            "postpass_s": round(shares.get("postpass", 0.0) * pnp_s, 4),
            "outside_s": round(shares.get("outside", 0.0) * pnp_s, 4),
            "repeat_max_share_delta_pp": split["cells"][cell][
                "repeat_max_share_delta_pp"
            ],
        }

    winners = [c for c, r in out.items() if r["prepass_at_canonical_result"] < 1.0]
    losers = [c for c, r in out.items() if r["prepass_at_canonical_result"] >= 1.0]
    binding = max(out, key=lambda c: out[c]["prepass_at_canonical_result"])
    payload = {
        "note": (
            "ESTIMATE, not a ratio of record. Shares are this take's instrumented "
            "split (per-phase median over three passes per cell); walls are this "
            "take's paired scoreboard medians (one window, Orca 2.5.1, PnP "
            "ordinary); canonical prepass costs are t110's tap split held fixed. "
            "`prepass_at_canonical` replaces the prepass with canonical's cost and "
            "holds every other phase at this take's measured share. No instrumented "
            "wall is a wall claim."
        ),
        "canonical_prepass_s": CANONICAL_PREPASS_S,
        "cells": out,
        "summary": {
            "cells_winning_at_canonical_prepass": len(winners),
            "winning_cells": winners,
            "losing_cells": losers,
            "binding_cell": binding,
            "binding_result": out[binding]["prepass_at_canonical_result"],
            "binding_needs_from_other_s": out[binding][
                "needed_from_other_than_prepass_s"
            ],
            "required_prepass_cut_range": [
                min(
                    r["required_prepass_cut"]
                    for r in out.values()
                    if r["required_prepass_cut"] is not None
                ),
                max(
                    r["required_prepass_cut"]
                    for r in out.values()
                    if r["required_prepass_cut"] is not None
                ),
            ],
            "prepass_share_range": [
                min(r["phase_shares"]["prepass"] for r in out.values()),
                max(r["phase_shares"]["prepass"] for r in out.values()),
            ],
            "per_layer_share_range": [
                min(r["phase_shares"]["per_layer"] for r in out.values()),
                max(r["phase_shares"]["per_layer"] for r in out.values()),
            ],
        },
    }
    out_path = Path(a.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {a.out}: {len(winners)}/8 win at canonical prepass")
    for cell in sorted(out, key=lambda c: out[c]["prepass_at_canonical_result"]):
        r = out[cell]
        print(
            f"  {cell:20s} {r['ratio_of_record']:5.3f}x  "
            f"prepass {100 * r['phase_shares']['prepass']:5.1f}%  "
            f"-> {r['prepass_at_canonical_result']:5.3f}x  "
            f"cut {100 * (r['required_prepass_cut'] or 0):5.1f}%  "
            f"needs other {r['needed_from_other_than_prepass_s']:6.3f}s"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

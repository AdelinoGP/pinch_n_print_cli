#!/usr/bin/env python3
"""t154 destination arithmetic -- the per-layer phase's share of each cell's cut.

Inputs (all committed):
  - `summary.json`: this take's instrumented phase split, eight cells.
  - the denominators of record, encoded below with their provenance:
      * supports-off Orca medians: t145's paired scoreboard (one window,
        1 warmup + 3 measured runs per tool, interleaved).
      * supports-on Orca medians: t145's benchy arms and t148's measured base
        arms (t148 re-measured base classic-on's Orca arm 23.414 -> 21.598 s).
      * PnP ratios of record: t145 (supports-off) and t148 (supports-on),
        carried forward by t150's measured per-cell A/B wall deltas.
  - canonical prepass costs: t110's tap-derived phase split
    (benchy-off 50.5 ms is a bound, not a point).

The arithmetic is an ESTIMATE, not a ratio of record: it chains two
measurement windows (the phase shares are today's; the walls are t145/t148
carried by t150's deltas). Every number below is therefore labelled
`estimate` and the ticket discloses it. The per-layer decision it drives is
insensitive to the ledger: the benchy cells clear the line by 10-33 %, which is
larger than the chained-ledger error the map has measured on this rig.

Usage:
  python arithmetic.py            # writes arithmetic.json
"""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent

# Orca medians (seconds) and the ratio of record per cell, with provenance.
DENOMINATORS = {
    "benchy-classic-off": {
        "orca_s": 1.915,
        "ratio_of_record": 2.043 * (1 - 0.0177),
        "provenance": "t145 paired scoreboard; ratio carried by t150's -1.77% PnP delta",
    },
    "benchy-classic-on": {
        "orca_s": 2.442,
        "ratio_of_record": 2.705 * (1 - 0.0495),
        "provenance": "t145 Orca arm + t148 supports-on ratio; t150's -4.95% delta",
    },
    "benchy-arachne-off": {
        "orca_s": 1.983,
        "ratio_of_record": 1.853 * (1 - 0.0487),
        "provenance": "t145 paired scoreboard; ratio carried by t150's -4.87% PnP delta",
    },
    "benchy-arachne-on": {
        "orca_s": 2.591,
        "ratio_of_record": 2.829 * (1 - 0.0487),
        "provenance": "t145 Orca arm + t148 supports-on ratio; t150's -4.87% delta",
    },
    "base-classic-off": {
        "orca_s": 13.414,
        "ratio_of_record": 2.078 * (1 - 0.1208),
        "provenance": "t145 paired scoreboard; ratio carried by t150's -12.08% PnP delta",
    },
    "base-classic-on": {
        "orca_s": 21.598,
        "ratio_of_record": 3.356 * (1 - 0.0406),
        "provenance": "t148 re-measured Orca arm (23.414 -> 21.598) + t148 ratio; t150's -4.06% delta",
    },
    "base-arachne-off": {
        "orca_s": 14.920,
        "ratio_of_record": 1.984 * (1 - 0.0937),
        "provenance": "t145 paired scoreboard; ratio carried by t150's -9.37% PnP delta",
    },
    "base-arachne-on": {
        "orca_s": 25.822,
        "ratio_of_record": 3.248 * (1 - 0.0578),
        "provenance": "t148 ratio on t145's Orca arm; t150's -5.78% delta",
    },
}

# Canonical prepass cost (seconds) by fixture/generator/supports class, from
# t110's tap-derived split. benchy-off is a bound (below 3x the tap's worst gap).
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
    summary = json.loads((HERE / "summary.json").read_text(encoding="utf-8"))
    out = {}
    for cell, red in sorted(summary["cells"].items()):
        den = DENOMINATORS[cell]
        total = red["slice_complete_ms"]
        phases = red["phases_ms"]
        pnp_s = den["ratio_of_record"] * den["orca_s"]
        shares = {name: ms / total for name, ms in phases.items()}
        other_s = (
            (
                (phases["per_layer"] + phases["postpass"] + phases["validation"])
                + red["outside_phases_ms"]
            )
            / total
            * pnp_s
        )
        prepass_s = phases["prepass"] / total * pnp_s
        canonical_s = canonical_prepass(cell)
        # Prepass at canonical cost, holding every other phase at today's share.
        result = (canonical_s + other_s) / den["orca_s"]
        required_cut = 1 - (den["orca_s"] - other_s) / prepass_s
        needed_from_other = max(0.0, (canonical_s + other_s) - den["orca_s"])
        out[cell] = {
            "estimate": True,
            "provenance": den["provenance"],
            "orca_s": den["orca_s"],
            "ratio_of_record": round(den["ratio_of_record"], 6),
            "pnp_s": round(pnp_s, 4),
            "phase_shares": {k: round(v, 4) for k, v in shares.items()},
            "outside_share": round(red["outside_phases_ms"] / total, 4),
            "prepass_s": round(prepass_s, 4),
            "other_than_prepass_s": round(other_s, 4),
            "per_layer_s": round(shares["per_layer"] * pnp_s, 4),
            "canonical_prepass_s": canonical_s,
            "prepass_at_canonical_result": round(result, 4),
            "margin_at_canonical": round(1 - result, 4),
            "required_prepass_cut": round(required_cut, 4),
            "needed_from_other_than_prepass_s": round(needed_from_other, 4),
            "needed_from_other_fraction_of_per_layer": (
                round(needed_from_other / (shares["per_layer"] * pnp_s), 4)
                if shares["per_layer"] > 0
                else None
            ),
        }
    payload = {
        "note": (
            "ESTIMATE, not a ratio of record. Phase shares are t154's instrumented "
            "split (HEAD, 2026-11-14); walls are t145/t148 medians carried by t150's "
            "measured per-cell deltas; canonical prepass costs are t110's tap split. "
            "Prepass-at-canonical holds every other phase at today's measured share. "
            "An instrumented share is attribution only; no instrumented wall is a "
            "wall claim."
        ),
        "cells": out,
    }
    (HERE / "arithmetic.json").write_text(
        json.dumps(payload, indent=1) + "\n", encoding="utf-8"
    )
    winners = [c for c, r in out.items() if r["prepass_at_canonical_result"] < 1.0]
    print(
        f"wrote arithmetic.json: {len(out)} cells, {len(winners)} win at canonical prepass"
    )
    for cell, r in sorted(
        out.items(), key=lambda kv: kv[1]["prepass_at_canonical_result"]
    ):
        print(
            f"  {cell:20s} result {r['prepass_at_canonical_result']:.3f}x  "
            f"required prepass cut {100 * r['required_prepass_cut']:5.1f}%  "
            f"needed from other {r['needed_from_other_than_prepass_s']:.3f} s"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

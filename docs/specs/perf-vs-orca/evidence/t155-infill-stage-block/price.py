#!/usr/bin/env python3
"""t155 -- price the measured block and write `summary.json`.

Every input is either this take's committed `probe.json` or a carried ledger
constant (t154's effective parallelism and walls, t148/t151's reference set).
The carried constants are encoded here with their provenance so the verifier can
re-derive the same arithmetic checkout-only.

Nothing here is a wall claim: a ceiling is `thread_sum / effective_parallelism`
and charges 100 % of the carrier's parallel work to the critical path.

Usage:
  python price.py
"""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent

# t154's measured per-layer effective parallelism (thread / wall) and the cell
# walls of record from its destination arithmetic (`evidence/t154-benchy-per-layer/`,
# `arithmetic.json`). Carried, not re-measured.
CELLS = {
    "base-classic-off": {
        "effective_parallelism": 10.91,
        "pnp_wall_of_record_s": 24.51,
        "orca_wall_s": 13.41,
    },
    "base-classic-on": {
        "effective_parallelism": 11.18,
        "pnp_wall_of_record_s": 69.54,
        "orca_wall_s": 21.598,
    },
}

# t154's destination arithmetic leaves base classic-on needing this much from a
# source other than the prepass, at canonical prepass cost.
BASE_ON_PER_LAYER_REQUIREMENT_S = 1.096
BASE_ON_CANONICAL_PREPASS_RATIO = 1.051
FLOOR_FRACTION = 0.008


def main() -> int:
    probe = json.loads((HERE / "probe.json").read_text(encoding="utf-8"))

    def prep_sum(cell: str, stage: str, module: str) -> int:
        return (
            probe["cells"][cell]["prep_views_by_stage_module"]
            .get(f"{stage}|{module}", {})
            .get("sum_us", 0)
        )

    def site_sum(cell: str, stage: str, module: str, site: str) -> int:
        return (
            probe["cells"][cell]["module_sites"]
            .get(f"{stage}|{module}", {})
            .get(site, {})
            .get("sum_us", 0)
        )

    cells = {}
    for cell, ledger in CELLS.items():
        redundant = prep_sum(cell, "Layer::Infill", "com.core.raft-default")
        bracket = (
            redundant
            + site_sum(cell, "Layer::Infill", "com.core.raft-default", "input_ctor")
            + site_sum(cell, "Layer::Infill", "com.core.raft-default", "run_stage")
        )
        first_derivation = prep_sum(
            cell, "Layer::Perimeters", "com.core.classic-perimeters"
        )
        other_infill = sum(
            value["sum_us"]
            for key, value in probe["cells"][cell]["prep_views_by_stage_module"].items()
            if key.startswith("Layer::Infill|") and "raft-default" not in key
        )
        per_layer_wall_ms = probe["cells"][cell]["phase_elapsed_ms"]["per_layer"]
        per_layer_thread_s = (
            per_layer_wall_ms / 1000.0 * ledger["effective_parallelism"]
        )
        ceiling_s = redundant / 1_000_000.0 / ledger["effective_parallelism"]
        floor_s = FLOOR_FRACTION * ledger["pnp_wall_of_record_s"]
        required_s = ledger["pnp_wall_of_record_s"] - ledger["orca_wall_s"]

        segments = probe["cells"][cell]["prepare_regions_segments"]
        segment_total = sum(
            v["sum_us"]
            for k, v in segments.items()
            if k.startswith("pr_") and k != "pr_region_start"
        )
        needs_support = segments.get("pr_needs_support", {}).get("sum_us", 0)

        cells[cell] = {
            "provenance": (
                "thread sums: this take's probe-v3 pass (probe.json); "
                "effective parallelism and walls: t154's carrier table / destination arithmetic (carried)"
            ),
            "first_perimeters_derivation_thread_s": round(first_derivation / 1e6, 3),
            "redundant_infill_derivation_thread_s": round(redundant / 1e6, 3),
            "infill_first_dispatch_bracket_thread_s": round(bracket / 1e6, 3),
            "prep_views_share_of_bracket": round(redundant / bracket, 4)
            if bracket
            else 0.0,
            "other_infill_modules_prep_views_thread_s": round(other_infill / 1e6, 3),
            "per_layer_thread_s": round(per_layer_thread_s, 3),
            "redundant_share_of_per_layer_thread": round(
                redundant / 1e6 / per_layer_thread_s, 4
            ),
            "ceiling_wall_s": round(ceiling_s, 3),
            "floor_s": round(floor_s, 4),
            "floor_multiple": round(ceiling_s / floor_s, 2),
            "cell_share_of_pnp_wall": round(
                ceiling_s / ledger["pnp_wall_of_record_s"], 4
            ),
            "required_wall_cut_s": round(required_s, 3),
            "share_of_required_cut": round(ceiling_s / required_s, 4),
            "needs_support_share_of_prepare_regions": round(
                needs_support / segment_total, 4
            )
            if segment_total
            else 0.0,
        }

    on = cells["base-classic-on"]
    on["base_on_requirement_s"] = BASE_ON_PER_LAYER_REQUIREMENT_S
    on["ceiling_over_requirement"] = round(
        on["ceiling_wall_s"] / BASE_ON_PER_LAYER_REQUIREMENT_S, 2
    )
    on["indicative_canonical_prepass_ratio"] = BASE_ON_CANONICAL_PREPASS_RATIO
    on["indicative_ratio_after_ceiling"] = round(
        (
            BASE_ON_CANONICAL_PREPASS_RATIO * CELLS["base-classic-on"]["orca_wall_s"]
            - on["ceiling_wall_s"]
        )
        / CELLS["base-classic-on"]["orca_wall_s"],
        3,
    )

    summary = {
        "ticket": "155-infill-stage-block-base-on-residual",
        "headline": (
            "The first Layer::Infill dispatch block is the prepared-region derivation "
            "(prepare_slice_regions -> derive_needs_support), paid twice per layer because "
            "the fill-polygon sync clears the arena memo; the second derivation is redundant "
            "and its ceiling exceeds base classic-on's 1.096 s requirement."
        ),
        "ledger_constants": CELLS,
        "cells": cells,
    }
    (HERE / "summary.json").write_text(
        json.dumps(summary, indent=1) + "\n", encoding="utf-8"
    )
    print(json.dumps(summary["cells"], indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

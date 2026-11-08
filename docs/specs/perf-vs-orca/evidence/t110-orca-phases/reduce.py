#!/usr/bin/env python3
"""t110 reducer: runs.csv -> phase-split.json.

Joins the Orca phase tap against ticket 104's committed PnP phase table for the same
four classic cells. All arithmetic is over medians of retained runs; the PnP side is
read from ../t104-phase-split/phase-split.json and never re-measured here.

Phases:
  startup+import  = `--info` median for the same model (no slicing; upper bound on
                    the slice path's startup + vendor + import)
  prepass         = t_tmp - startup+import   (Orca's PrintObject::slice phase)
  emit            = t_rename - t_tmp          (per-layer paths + G-code emission)
  tail            = t_exit - t_rename         (teardown / result bookkeeping)
"""

import csv
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
T104 = os.path.join(HERE, "..", "t104-phase-split", "phase-split.json")
CONTROLS = os.path.join(HERE, "controls.json")
CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]
MODEL_OF = {
    "benchy-classic-off": "benchy",
    "benchy-classic-on": "benchy",
    "base-classic-off": "base",
    "base-classic-on": "base",
}


def med(values):
    return statistics.median(values) if values else None


def load_rows(path):
    with open(path, newline="", encoding="utf-8") as fh:
        return list(csv.DictReader(fh))


def num(row, key):
    v = row.get(key)
    if v is None or v == "":
        return None
    return float(v)


def main():
    runs_csv = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "runs.csv")
    out_path = (
        sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, "phase-split.json")
    )
    rows = load_rows(runs_csv)
    t104 = json.load(open(T104, encoding="utf-8"))
    controls = (
        json.load(open(CONTROLS, encoding="utf-8")) if os.path.exists(CONTROLS) else {}
    )
    trivial = {a["model"]: a for a in controls.get("trivial_arms", [])}

    info = {}
    for model in ("calicat", "benchy", "base"):
        vals = [
            num(r, "wall_ms")
            for r in rows
            if r["kind"] == "info" and r["model"] == model
        ]
        info[model] = {
            "runs": len(vals),
            "wall_ms_median": med(vals),
            "wall_ms_values": vals,
        }

    cells = {}
    for cell in CELLS:
        rs = [r for r in rows if r["kind"] == "slice" and r["cell"] == cell]
        walls = [num(r, "wall_ms") for r in rs]
        pres = [num(r, "prephase_ms") for r in rs]
        emits = [num(r, "emit_ms") for r in rs]
        tails = [num(r, "tail_ms") for r in rs]
        cpu_tmp = [num(r, "cpu_at_tmp_s") for r in rs]
        cpu_ren = [num(r, "cpu_at_rename_s") for r in rs]
        cpu_fin = [num(r, "cpu_final_s") for r in rs]
        bytes_ = [num(r, "gcode_bytes") for r in rs]
        util = [num(r, "sys_cpu_util") for r in rs]
        max_gap = [num(r, "poll_max_gap_ms") for r in rs]

        startup = info[MODEL_OF[cell]]["wall_ms_median"]
        wall = med(walls)
        prephase = med(pres)
        emit = med(emits)
        tail = med(tails)
        prepass = None if prephase is None else prephase - startup

        pnp = t104["cells"][cell]
        pnp_phases = pnp["phases_ms"]
        pnp_prepass = pnp_phases["prepass"]
        pnp_per_layer_plus_post = pnp_phases["per_layer"] + pnp_phases["postpass"]
        pnp_outside = pnp["outside_phases_ms"]
        pnp_slicing = pnp["slice_complete_ms"] - pnp_outside
        orca_slicing = None if wall is None else wall - startup

        def share(x, total):
            return None if (x is None or not total) else round(100.0 * x / total, 2)

        cells[cell] = {
            "orca": {
                "runs": len(rs),
                "wall_ms_median": wall,
                "wall_ms_values": walls,
                "wall_ms_spread_pct": None
                if not wall
                else round(100.0 * (max(walls) - min(walls)) / wall, 2),
                "startup_import_ms": startup,
                "prephase_ms_median": prephase,
                "prepass_ms": None if prepass is None else round(prepass, 1),
                "emit_ms_median": emit,
                "tail_ms_median": tail,
                "shares_pct": {
                    "startup_import": share(startup, wall),
                    "prepass": share(prepass, wall),
                    "emit": share(emit, wall),
                    "tail": share(tail, wall),
                },
                "cpu_s_median": {
                    "at_prepass_end": med(cpu_tmp),
                    "at_emit_end": med(cpu_ren),
                    "at_exit": med(cpu_fin),
                },
                "cpu_s_by_phase": {
                    "startup_import_and_prepass": None
                    if med(cpu_tmp) is None
                    else round(med(cpu_tmp), 3),
                    "emit": None
                    if (med(cpu_ren) is None or med(cpu_tmp) is None)
                    else round(med(cpu_ren) - med(cpu_tmp), 3),
                    "tail": None
                    if (med(cpu_fin) is None or med(cpu_ren) is None)
                    else round(med(cpu_fin) - med(cpu_ren), 3),
                },
                "gcode_bytes": med(bytes_),
                "sys_cpu_util_median": med(util),
                "poll_max_gap_ms": max(g for g in max_gap if g is not None),
                "slicing_ms": None if orca_slicing is None else round(orca_slicing, 1),
                "slicing_share_pct": share(orca_slicing, wall),
                "trivial_arm": trivial.get(MODEL_OF[cell]),
            },
            "pnp_t104": {
                "slice_complete_ms": pnp["slice_complete_ms"],
                "prepass_ms": pnp_prepass,
                "per_layer_ms": pnp_phases["per_layer"],
                "postpass_ms": pnp_phases["postpass"],
                "outside_phases_ms": pnp_outside,
                "layer_count": pnp["layer_count"],
                "gcode_bytes": pnp.get("gcode_bytes"),
                "prepass_share_pct": round(
                    100.0 * pnp_prepass / pnp["slice_complete_ms"], 2
                ),
                "per_layer_plus_postpass_share_pct": round(
                    100.0
                    * (pnp_phases["per_layer"] + pnp_phases["postpass"])
                    / pnp["slice_complete_ms"],
                    2,
                ),
                "slicing_ms": pnp_slicing,
            },
            "compare": {
                "prepass_multiple_pnp_over_orca": None
                if not prepass
                else round(pnp_prepass / prepass, 2),
                "emit_multiple_pnp_over_orca": None
                if not emit
                else round(pnp_per_layer_plus_post / emit, 2),
                "slicing_multiple_pnp_over_orca": None
                if not orca_slicing
                else round(pnp_slicing / orca_slicing, 2),
                "import_multiple_orca_over_pnp": None
                if not pnp_outside
                else round(startup / pnp_outside, 2),
                "pnp_prepass_over_orca_whole_cell": None
                if not wall
                else round(pnp_prepass / wall, 2),
                "whole_slice_multiple_pnp_over_orca": None
                if not wall
                else round(pnp["slice_complete_ms"] / wall, 2),
                "note": (
                    "Orca prepass = prephase - startup_import, where startup_import is the "
                    "--info median for the same model (an upper bound, since --info also "
                    "prints a manifold check), so the Orca prepass and slicing figures are "
                    "LOWER bounds and every PnP/Orca multiple here is an UPPER bound. The "
                    "trivial-feature arm (controls.json) corroborates the import term: with "
                    "every per-layer feature turned down the cell wall moves by ~0.1 s. PnP "
                    "side is t104's instrumented phase table; its 'outside_phases' is "
                    "startup + mesh IO + teardown inside the instrumented span, so the import "
                    "multiple overstates Orca/PnP by whatever share of 'outside' is not import."
                ),
            },
        }

    doc = {
        "note": (
            "t110 reducer output. Orca side: phase tap medians over retained runs "
            "(runs.csv); PnP side: ticket 104's committed phase-split.json. Wall "
            "numbers are attribution only (tap adds a 2 ms poller)."
        ),
        "info_controls": info,
        "cells": cells,
    }
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1)
    print(json.dumps(doc["info_controls"], indent=1))
    for cell, v in cells.items():
        o = v["orca"]
        c = v["compare"]
        print(
            f"{cell:20s} wall={o['wall_ms_median']:8.1f} startup={o['startup_import_ms']:7.1f} "
            f"prepass={o['prepass_ms']:8.1f} emit={o['emit_ms_median']:7.1f} tail={o['tail_ms_median']:6.1f} | "
            f"PnP prepass={v['pnp_t104']['prepass_ms']:7d} x{c['prepass_multiple_pnp_over_orca']} | "
            f"emit x{c['emit_multiple_pnp_over_orca']}"
        )
    print(f"\nwrote {out_path}")


if __name__ == "__main__":
    main()

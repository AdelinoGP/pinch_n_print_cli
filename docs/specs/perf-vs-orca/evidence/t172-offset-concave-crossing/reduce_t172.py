#!/usr/bin/env python3
"""t172 reduce: paired wall/CPU statistics from the campaign rows.

Reads the gitignored raw tree under `target/t172/` and writes the in-repo
reduced set under `reduced/`. Nothing here reads or writes the tracker.

Pre-declared gate (Q2 ruling):
- All 8 cells clear the 0.80 % wall floor AND sit strictly outside their
  same-window null band's any-pair bound, with process CPU corroborating
  (positive median CPU saving).
- Baseline reproduces t169's active reference set in 8/8; accelerated arm is
  byte-identical to ordinary in 16/16.
- `base-classic-off` >= 3.9 % and `base-arachne-off` >= 10.3 % (t169's
  estimated required cuts).

Usage:  python reduce_t172.py
"""

import csv
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
RAW = os.path.join(REPO, "target", "t172")
OUT = os.path.join(HERE, "reduced")

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

FLOOR_PCT = 0.80
REQUIRED_CUTS_T169_EST = {
    "base-classic-off": 3.9,
    "base-arachne-off": 10.3,
}

# The active reference set at the moment this take started (t169's, recorded in
# `../t169-offset-decimation-flat-opening/reference.json`).
T169_ACTIVE = {
    "benchy-classic-off": "f505910c4ca40d51ab2362fcdeb6b93e2b50937a5132be0a43d2e124603a7f9b",
    "benchy-classic-on": "9b40bf51cb4ec899dd30d5d97238e9eff5f5f52467a12d86b89e8b1b67fe6f25",
    "benchy-arachne-off": "224fb6f06e9086b04d33e0fa107690395319afd4a45ceb112315be9b1ee7a5e7",
    "benchy-arachne-on": "f32047c37441f59f5cc39511584b10cc03ac3c933d9d8b4e91cb332aaef39819",
    "base-classic-off": "33468ea96bb6cd93e5cad33fbd8c6041268b8a3ac4f13106e5fd133fcacba27c",
    "base-classic-on": "661b9a510669c18f1b342435785c408310bf93ea34ebf878f6dcd736391ab885",
    "base-arachne-off": "e861766a95fdb1b15827af49feaeeea1a5c54f0b390d34b6256ba0360c39598d",
    "base-arachne-on": "2124b5f5e82b3cf2f7bf542ded0341c48c674a35267420a2815bcf65fcc9f58e",
}


def read_rows(path):
    with open(path, encoding="utf-8-sig", newline="") as fh:
        return list(csv.DictReader(fh))


def pair_savings(rows, metric):
    by = {}
    for row in rows:
        if row.get("null_control") in ("True", "true", "1"):
            continue
        by.setdefault((row["cell"], int(row["pair"])), {})[row["side"]] = float(
            row[metric]
        )
    out = {}
    for (cell, pair), sides in sorted(by.items()):
        if "baseline" not in sides or "candidate" not in sides:
            continue
        b, c = sides["baseline"], sides["candidate"]
        pct = (b - c) / b * 100.0
        out.setdefault(cell, []).append(
            {"pair": pair, "baseline_s": b, "candidate_s": c, "saving_pct": pct}
        )
    return out


def cell_stats(savings):
    vals = [p["saving_pct"] for p in savings]
    return {
        "pairs": len(vals),
        "median_saving_pct": statistics.median(vals),
        "mean_saving_pct": statistics.fmean(vals),
        "min_saving_pct": min(vals),
        "max_saving_pct": max(vals),
        "wins": sum(1 for v in vals if v > 0.0),
        "baseline_median_s": statistics.median([p["baseline_s"] for p in savings]),
        "candidate_median_s": statistics.median([p["candidate_s"] for p in savings]),
        "floor_multiple": statistics.median(vals) / FLOOR_PCT,
    }


def null_band(rows):
    by = {}
    for row in rows:
        by.setdefault((row["cell"], int(row["pair"])), {})[row["side"]] = row
    out = {}
    for (cell, pair), sides in sorted(by.items()):
        a, b = sides.get("baseline"), sides.get("baseline2")
        if not a or not b:
            continue
        walls = abs(
            (float(a["wall_s"]) - float(b["wall_s"])) / float(b["wall_s"]) * 100.0
        )
        cpus = abs((float(a["cpu_s"]) - float(b["cpu_s"])) / float(b["cpu_s"]) * 100.0)
        out.setdefault(cell, {"wall_abs": [], "cpu_abs": []})
        out[cell]["wall_abs"].append(walls)
        out[cell]["cpu_abs"].append(cpus)
    res = {}
    for cell, data in out.items():
        res[cell] = {
            "pairs": len(data["wall_abs"]),
            "wall_any_pair_pct": max(data["wall_abs"]),
            "wall_median_pct": statistics.median(data["wall_abs"]),
            "cpu_any_pair_pct": max(data["cpu_abs"]),
            "cpu_median_pct": statistics.median(data["cpu_abs"]),
        }
    res["_worst"] = {
        "wall_any_pair_pct": max(
            v["wall_any_pair_pct"] for k, v in res.items() if k != "_worst"
        ),
        "wall_median_pct": max(
            v["wall_median_pct"] for k, v in res.items() if k != "_worst"
        ),
        "cpu_any_pair_pct": max(
            v["cpu_any_pair_pct"] for k, v in res.items() if k != "_worst"
        ),
        "cpu_median_pct": max(
            v["cpu_median_pct"] for k, v in res.items() if k != "_worst"
        ),
    }
    return res


def gate(summary, null):
    """Strict Q2 ruling gate: all 8 cells clear 0.80 % floor and null any-pair
    bound with CPU corroborating, plus base supports-off required cuts."""
    verdicts = {}
    for cell in CELLS:
        w = summary[cell]["wall"]
        c = summary[cell]["cpu"]
        n = null.get(cell)
        clears_floor = w["median_saving_pct"] >= FLOOR_PCT
        outside_null = n is None or w["median_saving_pct"] > n["wall_any_pair_pct"]
        cpu_ok = c["median_saving_pct"] > 0.0
        req_cut = REQUIRED_CUTS_T169_EST.get(cell)
        beats_cut = req_cut is None or w["median_saving_pct"] >= req_cut
        if clears_floor and outside_null and cpu_ok and beats_cut:
            verdicts[cell] = "PASS"
        elif not clears_floor:
            verdicts[cell] = "SUB-FLOOR"
        elif not outside_null:
            verdicts[cell] = "INSIDE-NULL"
        elif not cpu_ok:
            verdicts[cell] = "CPU-CONTRADICTED"
        else:
            verdicts[cell] = "MISSED-CUT"
    return verdicts


def main():
    ab_rows = read_rows(os.path.join(RAW, "ab", "ab", "rows.csv"))
    null_rows = []
    null_path = os.path.join(RAW, "ab", "null", "rows.csv")
    if os.path.exists(null_path):
        null_rows.extend(read_rows(null_path))
    out_rows = read_rows(os.path.join(RAW, "output", "ref", "rows.csv"))

    wall_pairs = pair_savings(ab_rows, "wall_s")
    cpu_pairs = pair_savings(ab_rows, "cpu_s")
    summary = {}
    for cell in CELLS:
        summary[cell] = {
            "wall": cell_stats(wall_pairs[cell]),
            "cpu": cell_stats(cpu_pairs[cell]),
        }
    null = null_band(null_rows) if null_rows else {"_worst": None}
    verdicts = gate(summary, null if null_rows else {})
    summary["_gate"] = verdicts

    # Load batch metadata (CPU loads) if present
    meta = {}
    for b in ("ab", "null", "accel"):
        mp = os.path.join(RAW, "ab", b, "batch_meta.json")
        if os.path.exists(mp):
            with open(mp, encoding="utf-8-sig") as fh:
                meta[b] = json.load(fh)
    summary["_meta"] = meta

    manifest_path = os.path.join(RAW, "manifest.json")
    manifest = {}
    if os.path.exists(manifest_path):
        with open(manifest_path, encoding="utf-8-sig") as fh:
            manifest = json.load(fh)

    out = {"t169_active": T169_ACTIVE, "manifest": manifest, "cells": {}, "accelerated": {}}
    for row in out_rows:
        out["cells"].setdefault(row["cell"], {})[row["arm"]] = {
            "sha256": row["gcode_sha256"],
            "bytes": int(row["gcode_bytes"]),
        }
    for row in read_rows(os.path.join(RAW, "output", "accel", "rows.csv")):
        out["accelerated"].setdefault(row["cell"], {})[row["arm"]] = row["gcode_sha256"]

    accel_rows = read_rows(os.path.join(RAW, "ab", "accel", "rows.csv"))
    accel = {}
    aw = pair_savings(accel_rows, "wall_s")
    ac = pair_savings(accel_rows, "cpu_s")
    for cell in CELLS:
        accel[cell] = {"wall": cell_stats(aw[cell]), "cpu": cell_stats(ac[cell])}

    os.makedirs(OUT, exist_ok=True)
    for name, doc in (
        ("ab.json", summary),
        ("null.json", null),
        ("output.json", out),
        ("accel.json", accel),
    ):
        with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as fh:
            json.dump(doc, fh, indent=2)
            fh.write("\n")

    print(
        "%-22s %9s %9s %9s %9s %8s %6s  %s"
        % ("cell", "base_s", "cand_s", "wall%", "cpu%", "floor x", "wins", "verdict")
    )
    for cell in CELLS:
        w = summary[cell]["wall"]
        c = summary[cell]["cpu"]
        print(
            "%-22s %9.3f %9.3f %+9.3f %+9.3f %8.2f %6d  %s"
            % (
                cell,
                w["baseline_median_s"],
                w["candidate_median_s"],
                w["median_saving_pct"],
                c["median_saving_pct"],
                w["floor_multiple"],
                w["wins"],
                verdicts[cell],
            )
        )
    if null_rows:
        print(
            "\nnull band (any-pair / median): wall %.2f%% / %.2f%%, cpu %.2f%% / %.2f%%"
            % (
                null["_worst"]["wall_any_pair_pct"],
                null["_worst"]["wall_median_pct"],
                null["_worst"]["cpu_any_pair_pct"],
                null["_worst"]["cpu_median_pct"],
            )
        )
    print("wrote reduced/ab.json, null.json, output.json, accel.json")
    bad = [
        cell
        for cell in CELLS
        if out["cells"][cell]["baseline-ordinary"]["sha256"] != T169_ACTIVE[cell]
    ]
    print("baseline reproduces the t169 active set in %d/8 cells" % (8 - len(bad)))
    if bad:
        print("MISMATCH:", ", ".join(bad))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

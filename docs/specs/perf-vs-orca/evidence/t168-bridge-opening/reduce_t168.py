#!/usr/bin/env python3
"""t168 reduce: paired wall/CPU statistics from the campaign rows.

Reads the gitignored raw tree under `target/t168/ab/` and writes the in-repo
reduced set under `reduced/`. Nothing here reads or writes the tracker.

Definitions (the standing gate's, unchanged from t158/t166):
- per-pair saving = (baseline - candidate) / baseline * 100, so a positive
  number means the candidate finished sooner;
- a cell's statistic is the median over pairs, with the win count beside it;
- the floor is 0.80 % and its multiple is `median_saving / 0.80`;
- the same-arm null band is measured in the same window, from
  `-Mode ab -NullControl`, as the any-pair absolute saving and its median.

Usage:  python reduce_t168.py
"""

import csv
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
RAW = os.path.join(REPO, "target", "t168")
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

# The active reference set at the moment this take started (t166's, recorded
# in `../t166-shell-opening-parity/reference.json`). The baseline arm must
# reproduce every one of these; a mismatch invalidates the take's control.
T166_REFERENCE = {
    "benchy-classic-off": "276782a7",
    "benchy-classic-on": "44564d29",
    "benchy-arachne-off": "46111bb0",
    "benchy-arachne-on": "12c5e45c",
    "base-classic-off": "0a1ee3df",
    "base-classic-on": "bc5cb5e1",
    "base-arachne-off": "cadebffa",
    "base-arachne-on": "77f2b655",
}


def read_rows(path):
    with open(path, encoding="utf-8-sig", newline="") as fh:
        return list(csv.DictReader(fh))


def pair_savings(rows, metric):
    """{(cell): [(pair, pct_saving), ...]} from an A/B rows.csv."""
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
    """{(cell): {...}} for the same-arm control (both sides are the baseline)."""
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
    """The pre-declared gate: base cells clear the floor and the null band; the
    benchy cells show no median regression."""
    verdicts = {}
    for cell in CELLS:
        s = summary[cell]["wall"]
        n = null.get(cell)
        base = cell.startswith("base")
        if base:
            clears_floor = s["median_saving_pct"] >= FLOOR_PCT
            # The strict reading: the median must clear the null band's
            # **any-pair** bound, not merely its median. A median that sits
            # inside the instrument's own spread is not a measured effect.
            outside_null = n is None or s["median_saving_pct"] > n["wall_any_pair_pct"]
            verdicts[cell] = (
                "PASS"
                if (clears_floor and outside_null)
                else ("SUB-FLOOR" if not clears_floor else "INSIDE-NULL")
            )
        else:
            verdicts[cell] = "PASS" if s["median_saving_pct"] >= 0.0 else "REGRESSION"
    return verdicts


def main():
    ab_rows = read_rows(os.path.join(RAW, "ab", "ab", "rows.csv"))
    # The null control is the same instrument measured in the same windows:
    # `null` bounds the supports-off/benchy cells, `null2` the two base
    # supports-on cells that the first pass could not separate from noise.
    null_rows = []
    for batch in ("null", "null2"):
        path = os.path.join(RAW, "ab", batch, "rows.csv")
        if os.path.exists(path):
            null_rows.extend(read_rows(path))
    out_rows = read_rows(os.path.join(RAW, "output", "ref", "rows.csv"))

    wall_pairs = pair_savings(ab_rows, "wall_s")
    cpu_pairs = pair_savings(ab_rows, "cpu_s")
    summary = {}
    for cell in CELLS:
        summary[cell] = {
            "wall": cell_stats(wall_pairs[cell]),
            "cpu": cell_stats(cpu_pairs[cell]),
        }
    null = null_band(null_rows)
    verdicts = gate(summary, null)
    summary["_gate"] = verdicts

    # Output currency: the baseline must reproduce t166's active set; the
    # candidate records the new one. Hashes are read, never re-derived.
    out = {"t166_reference": T166_REFERENCE, "cells": {}, "accelerated": {}}
    for row in out_rows:
        out["cells"].setdefault(row["cell"], {})[row["arm"]] = {
            "sha256": row["gcode_sha256"],
            "bytes": int(row["gcode_bytes"]),
        }
    for row in read_rows(os.path.join(RAW, "output", "accel", "rows.csv")):
        out["accelerated"].setdefault(row["cell"], {})[row["arm"]] = row["gcode_sha256"]

    # Accelerated-arm corroboration: the same paired statistics, three pairs a
    # cell, no null band (disclosed: the accelerated arm was not null-controlled).
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
        "%-22s %9s %9s %9s %8s %6s  %s"
        % ("cell", "base_s", "cand_s", "wall%", "floor x", "wins", "verdict")
    )
    for cell in CELLS:
        w = summary[cell]["wall"]
        print(
            "%-22s %9.3f %9.3f %+9.3f %8.2f %6d  %s"
            % (
                cell,
                w["baseline_median_s"],
                w["candidate_median_s"],
                w["median_saving_pct"],
                w["floor_multiple"],
                w["wins"],
                verdicts[cell],
            )
        )
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
        if not out["cells"][cell]["baseline-ordinary"]["sha256"].startswith(
            T166_REFERENCE[cell]
        )
    ]
    print("baseline reproduces t166 reference in %d/8 cells" % (8 - len(bad)))
    if bad:
        print("MISMATCH:", ", ".join(bad))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

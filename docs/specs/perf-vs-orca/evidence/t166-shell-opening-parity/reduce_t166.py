#!/usr/bin/env python3
"""Reduce the t166 campaign CSVs into reduced/campaign.json.

Inputs (all under target/t166/, gitignored):

    output/ref/rows.csv    8 cells x {baseline, candidate}, one run each
    ab/ab/rows.csv         5 interleaved pairs per cell
    ab/null/rows.csv       null control: baseline arm on both sides

The reducer only reshapes and reduces; verify.py re-derives every claim from
`reduced/campaign.json` and never reads the raw tree.

Definitions used throughout:

  pair ratio      candidate wall / baseline wall inside one interleaved pair
  wall saving %   (1 - median(pair ratios)) * 100      positive = candidate faster
  floor multiple  wall saving % / 0.80                  the standing gate floor
"""

import csv
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
RAW = os.path.join(REPO, "target", "t166")
OUT = os.path.join(HERE, "reduced")

# t151's active reference set (the currency before this take), by cell id.
T151_REFERENCE = {
    "benchy-classic-off": "3d28d1b3",
    "benchy-classic-on": "eb202973",
    "benchy-arachne-off": "39ad6289",
    "benchy-arachne-on": "77c3c578",
    "base-classic-off": "9bb18f0e",
    "base-classic-on": "eccbd523",
    "base-arachne-off": "2f249cdb",
    "base-arachne-on": "1811425f",
}

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


def read_rows(path):
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8-sig", newline="") as fh:
        return list(csv.DictReader(fh))


def f(row, key):
    return float(row[key])


def output_section(rows):
    by = {}
    for row in rows:
        by.setdefault(row["cell"], {})[row["arm"]] = row
    cells = {}
    for cell in CELLS:
        arms = by.get(cell, {})
        base, cand = arms.get("baseline-ordinary"), arms.get("candidate-ordinary")
        if not base or not cand:
            continue
        for row in (base, cand):
            if int(row["exit_code"]) != 0:
                raise SystemExit("non-zero exit in output campaign: %s" % row)
        baseline_hash, candidate_hash = base["gcode_sha256"], cand["gcode_sha256"]
        cells[cell] = {
            "baseline_sha256": baseline_hash,
            "baseline_prefix16": baseline_hash[:16],
            "candidate_sha256": candidate_hash,
            "candidate_prefix16": candidate_hash[:16],
            "baseline_reproduces_t151": baseline_hash.startswith(T151_REFERENCE[cell]),
            "output_moved": baseline_hash != candidate_hash,
            "baseline_bytes": int(base["gcode_bytes"]),
            "candidate_bytes": int(cand["gcode_bytes"]),
            "byte_delta": int(cand["gcode_bytes"]) - int(base["gcode_bytes"]),
        }
    return cells


def pair_section(rows):
    cells = {}
    for cell in CELLS:
        by_pair = {}
        for row in rows:
            if row["cell"] != cell:
                continue
            by_pair.setdefault(int(row["pair"]), {})[row["side"]] = row
        pairs = []
        for p in sorted(by_pair):
            sides = by_pair[p]
            if "baseline" not in sides or "candidate" not in sides:
                continue
            b, c = sides["baseline"], sides["candidate"]
            if int(b["exit_code"]) != 0 or int(c["exit_code"]) != 0:
                raise SystemExit("non-zero exit in pair %d / %s" % (p, cell))
            pairs.append(
                {
                    "pair": p,
                    "order": b["order"],
                    "baseline_wall_s": f(b, "wall_s"),
                    "candidate_wall_s": f(c, "wall_s"),
                    "baseline_cpu_s": f(b, "cpu_s"),
                    "candidate_cpu_s": f(c, "cpu_s"),
                    "wall_ratio": f(c, "wall_s") / f(b, "wall_s"),
                    "cpu_ratio": f(c, "cpu_s") / f(b, "cpu_s"),
                    "baseline_hash": b["gcode_sha256"],
                    "candidate_hash": c["gcode_sha256"],
                }
            )
        if not pairs:
            continue
        wall_ratios = [p["wall_ratio"] for p in pairs]
        cpu_ratios = [p["cpu_ratio"] for p in pairs]
        wall_saving = (1.0 - statistics.median(wall_ratios)) * 100.0
        cpu_saving = (1.0 - statistics.median(cpu_ratios)) * 100.0
        cells[cell] = {
            "pairs": pairs,
            "n_pairs": len(pairs),
            "wall_ratio_median": statistics.median(wall_ratios),
            "cpu_ratio_median": statistics.median(cpu_ratios),
            "wall_saving_pct": wall_saving,
            "cpu_saving_pct": cpu_saving,
            "floor_multiple": wall_saving / FLOOR_PCT,
            "pairs_candidate_faster": sum(1 for r in wall_ratios if r < 1.0),
            "pairs_cpu_lower": sum(1 for r in cpu_ratios if r < 1.0),
            "baseline_wall_median_s": statistics.median(
                p["baseline_wall_s"] for p in pairs
            ),
            "candidate_wall_median_s": statistics.median(
                p["candidate_wall_s"] for p in pairs
            ),
            "wall_saving_s": statistics.median(p["baseline_wall_s"] for p in pairs)
            - statistics.median(p["candidate_wall_s"] for p in pairs),
            "hashes_identical_within_pairs": all(
                p["baseline_hash"] == p["candidate_hash"] for p in pairs
            ),
        }
    return cells


def null_section(rows):
    """Null control: the baseline arm on both sides. Bands are the any-pair
    absolute deviation and the median deviation, both in percent."""
    cells = {}
    for cell in sorted({row["cell"] for row in rows}):
        by_pair = {}
        for row in rows:
            if row["cell"] != cell:
                continue
            by_pair.setdefault(int(row["pair"]), {})[row["side"]] = row
        pairs = []
        for p in sorted(by_pair):
            sides = by_pair[p]
            if "baseline" not in sides or "baseline2" not in sides:
                continue
            a, b = sides["baseline"], sides["baseline2"]
            pairs.append(
                {
                    "pair": p,
                    "wall_ratio": f(b, "wall_s") / f(a, "wall_s"),
                    "cpu_ratio": f(b, "cpu_s") / f(a, "cpu_s"),
                }
            )
        if not pairs:
            continue
        wall_dev = [abs(p["wall_ratio"] - 1.0) * 100.0 for p in pairs]
        cpu_dev = [abs(p["cpu_ratio"] - 1.0) * 100.0 for p in pairs]
        cells[cell] = {
            "n_pairs": len(pairs),
            "wall_band_pct_any_pair": max(wall_dev),
            "wall_band_pct_median": (
                abs(statistics.median([p["wall_ratio"] for p in pairs]) - 1.0)
            )
            * 100.0,
            "cpu_band_pct_any_pair": max(cpu_dev),
            "cpu_band_pct_median": (
                abs(statistics.median([p["cpu_ratio"] for p in pairs]) - 1.0)
            )
            * 100.0,
        }
    return cells


def main():
    output = read_rows(os.path.join(RAW, "output", "ref", "rows.csv"))
    ab = read_rows(os.path.join(RAW, "ab", "ab", "rows.csv"))
    null = read_rows(os.path.join(RAW, "ab", "null", "rows.csv"))

    manifest_path = os.path.join(RAW, "manifest.json")
    manifest = {}
    if os.path.exists(manifest_path):
        with open(manifest_path, encoding="utf-8-sig") as fh:
            manifest = json.load(fh)

    meta = {}
    meta_path = os.path.join(RAW, "ab", "ab", "batch_meta.json")
    if os.path.exists(meta_path):
        with open(meta_path, encoding="utf-8-sig") as fh:
            meta = json.load(fh)

    doc = {
        "schema": 1,
        "take": "2026-11-19",
        "floor_pct": FLOOR_PCT,
        "threads": 12,
        "manifest": manifest,
        "ab_batch_meta": meta,
        "output": output_section(output),
        "pairs": pair_section(ab),
        "null": null_section(null),
    }
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, "campaign.json")
    with open(path, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(doc, fh, indent=2)
        fh.write("\n")
    print("wrote", path)
    for cell in CELLS:
        if cell in doc["pairs"]:
            p = doc["pairs"][cell]
            print(
                "%-22s n=%d wall %+7.3f%% (%5.2fx floor) cpu %+7.3f%% %d/%d faster"
                % (
                    cell,
                    p["n_pairs"],
                    p["wall_saving_pct"],
                    p["floor_multiple"],
                    p["cpu_saving_pct"],
                    p["pairs_candidate_faster"],
                    p["n_pairs"],
                )
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())

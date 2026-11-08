#!/usr/bin/env python3
"""Reduce the t103 re-baseline ledger into the SCOREBOARD tables and summary.json.

Reads the five ledger CSVs committed beside this file (never the raw
target/matched-pair capture tree), applies the standing protocol rules, and
prints the per-cell table, CPU corroboration, the brim asymmetry, the pins and
the park bound. `verify.py` re-derives the same facts independently and fails
on any mismatch.

Rules mirrored from summarize_scoreboard.ps1:
  - measured rows only (warmup == 0);
  - starvation (PERF-HANDOFF 10.3): within a (fixture, cell, tool) group a
    sample is excluded when cpu/wall < 0.75 * the group's best cpu/wall;
  - median of retained wall/cpu samples.

Floor: the map's per-candidate 0.80 % of the cell wall (the smallest effect a
timing take is allowed to campaign on). "Floor multiples to park" is the
fraction of PnP wall that must disappear to reach the park's ~1.5x condition,
divided by 0.008.
"""

import csv
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LEDGER_CSVS = [
    "rows-benchy.csv",
    "rows-base-classic-off.csv",
    "rows-base-arachne-off.csv",
    "rows-base-classic-on.csv",
    "rows-base-arachne-on.csv",
]
TOOLS = ("orca", "pnp-ordinary", "pnp-accelerated")
FLOOR = 0.008
PARK_RATIO = 1.5
STARVATION = 0.75


def load_rows():
    rows = []
    for name in LEDGER_CSVS:
        path = HERE / name
        if not path.exists():
            raise SystemExit(f"missing ledger csv: {name}")
        with path.open(newline="", encoding="utf-8-sig") as fh:
            rows.extend(csv.DictReader(fh))
    return rows


def groups(rows):
    out = {}
    for row in rows:
        key = (row["fixture"], f"{row['generator']}-{row['supports']}", row["tool"])
        out.setdefault(key, []).append(row)
    return out


def retained(group):
    measured = [r for r in group if r["warmup"] == "0"]
    if not measured:
        return [], []
    best = max(float(r["cpu_wall_ratio"]) for r in measured)
    kept = [r for r in measured if float(r["cpu_wall_ratio"]) >= STARVATION * best]
    return kept, [r for r in measured if r not in kept]


def medians(rows):
    return (
        statistics.median(float(r["wall_seconds"]) for r in rows),
        statistics.median(float(r["cpu_seconds"]) for r in rows),
    )


def main():
    rows = load_rows()
    by_group = groups(rows)
    cells = sorted({(r["fixture"], f"{r['generator']}-{r['supports']}") for r in rows})

    summary = {"cells": {}, "floor": FLOOR, "park_ratio": PARK_RATIO}
    table_rows = []
    for fixture, cell in cells:
        entry = {"fixture": fixture, "cell": cell, "tools": {}}
        for tool in TOOLS:
            group = by_group.get((fixture, cell, tool), [])
            kept, starved = retained(group)
            w, c = medians(kept)
            entry["tools"][tool] = {
                "warmups": len([r for r in group if r["warmup"] == "1"]),
                "measured": len([r for r in group if r["warmup"] == "0"]),
                "retained": len(kept),
                "starved": [r["run_id"] for r in starved],
                "wall_s": round(w, 4),
                "cpu_s": round(c, 4),
                "cpu_wall": round(c / w, 4) if w else 0.0,
                "gcode_bytes": sorted({int(r["gcode_bytes"]) for r in kept}),
                "sha256": sorted({r["sha256_of_gcode"] for r in kept}),
                "degraded": sorted({r["degraded"] for r in kept}),
                "non_fatal": sorted({r["non_fatal_error_count"] for r in kept}),
                "resolved_brim_width": sorted(
                    {r.get("resolved_brim_width", "") for r in kept}
                ),
                "resolved_enable_arc_fitting": sorted(
                    {r.get("resolved_enable_arc_fitting", "") for r in kept}
                ),
                "resolved_elefant_foot_compensation": sorted(
                    {r.get("resolved_elefant_foot_compensation", "") for r in kept}
                ),
                "resolved_filament_density": sorted(
                    {r.get("resolved_filament_density", "") for r in kept}
                ),
                "resolved_filament_max_volumetric_speed": sorted(
                    {r.get("resolved_filament_max_volumetric_speed", "") for r in kept}
                ),
                "resolved_first_layer_temperature": sorted(
                    {r.get("resolved_first_layer_temperature", "") for r in kept}
                ),
            }
        t = entry["tools"]
        entry["gap_ordinary"] = round(
            t["pnp-ordinary"]["wall_s"] / t["orca"]["wall_s"], 4
        )
        entry["gap_accelerated"] = round(
            t["pnp-accelerated"]["wall_s"] / t["orca"]["wall_s"], 4
        )
        entry["cpu_gap_ordinary"] = round(
            t["pnp-ordinary"]["cpu_s"] / t["orca"]["cpu_s"], 3
        )
        # Park bound: PnP wall must fall to PARK_RATIO * Orca wall.
        needed = 1.0 - (PARK_RATIO * t["orca"]["wall_s"]) / t["pnp-ordinary"]["wall_s"]
        entry["park"] = {
            "required_wall_reduction_fraction_ordinary": round(needed, 4),
            "floor_multiples_ordinary": round(needed / FLOOR, 1),
            "within_park": needed <= 0.0,
        }
        summary["cells"][f"{fixture}/{cell}"] = entry
        table_rows.append(entry)

    print("# t103 Orca re-baseline: median uninstrumented wall (s)")
    print()
    print(
        "| Cell | Orca | PnP ordinary | PnP accelerated | gap ord | gap acc | park: reduction needed | floor multiples |"
    )
    print("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
    for e in table_rows:
        t = e["tools"]
        print(
            f"| {e['fixture']} {e['cell']} | {t['orca']['wall_s']:.2f} | "
            f"{t['pnp-ordinary']['wall_s']:.2f} | {t['pnp-accelerated']['wall_s']:.2f} | "
            f"**{e['gap_ordinary']:.2f}x** | **{e['gap_accelerated']:.2f}x** | "
            f"{100 * e['park']['required_wall_reduction_fraction_ordinary']:.1f} % | "
            f"{e['park']['floor_multiples_ordinary']:.0f}x |"
        )

    print()
    print("## Pins and filament leg (every retained Orca row)")
    for e in table_rows:
        o = e["tools"]["orca"]
        print(
            f"- {e['fixture']}/{e['cell']}: arc={o['resolved_enable_arc_fitting']} "
            f"elefant={o['resolved_elefant_foot_compensation']} "
            f"brim={o['resolved_brim_width']} filament={o['resolved_filament_density']}/"
            f"{o['resolved_filament_max_volumetric_speed']}/"
            f"{o['resolved_first_layer_temperature']}"
        )

    print()
    print("## Brim asymmetry (PnP prints what pinned-back Orca does not)")
    for e in table_rows:
        print(
            f"- {e['fixture']}/{e['cell']}: PnP brim={e['tools']['pnp-ordinary']['resolved_brim_width']} "
            f"vs Orca brim={e['tools']['orca']['resolved_brim_width']}"
        )

    print()
    print("## Starvation and output flags")
    for e in table_rows:
        for tool in TOOLS:
            t = e["tools"][tool]
            print(
                f"- {e['fixture']}/{e['cell']}/{tool}: warmups={t['warmups']} "
                f"measured={t['measured']} retained={t['retained']} "
                f"starved={t['starved']} degraded={t['degraded']} non_fatal={t['non_fatal']}"
            )

    out = HERE / "summary.json"
    out.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print()
    print(f"wrote {out.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

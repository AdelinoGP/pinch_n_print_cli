#!/usr/bin/env python3
"""t106 post-pin supports-on scoreboard -- reduce the raw rig batches.

Reads the standing rig's per-batch CSVs and stats JSONs under
`target/matched-pair/t106-<tag>-<fixture>-<cell>/` and writes the committed
reduced set:

  runs.csv      one row per Orca run (measured and warmup): wall/CPU, the
                resolved `support_style` / `enable_support`, output bytes/SHA
                and the support section counts
  summary.json  per-cell median, spread, ticket-103 PnP ratio, cut-to-park, the
                pre-pin ticket-103 Orca comparison and the ticket-105 transfer
                check (pre-registered in REPRODUCE.md)

Usage:
  python reduce.py [--raw <matched-pair dir>]

The raw tree is gitignored; `verify.py` never reads it.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
DEFAULT_RAW = REPO / "target/matched-pair"
EVIDENCE = REPO / "docs/specs/perf-vs-orca/evidence"
T103 = EVIDENCE / "t103-orca-rebaseline/summary.json"
T103_ROWS = EVIDENCE / "t103-orca-rebaseline"
T105 = EVIDENCE / "t105-tree-strong/summary.json"

BATCH_RE = re.compile(
    r"^t106-(?P<tag>[rw])(?P<index>\d+)-"
    r"(?P<fixture>benchy|base)-(?P<generator>classic|arachne)-on$"
)

RUN_COLUMNS = [
    "batch",
    "fixture",
    "cell",
    "warmup",
    "round",
    "run_index",
    "wall_s",
    "cpu_s",
    "cpu_wall",
    "gcode_bytes",
    "sha256",
    "support_style",
    "enable_support",
    "support_sections",
    "support_interface_sections",
    "type_counts_json",
]


def load_batches(raw: Path) -> list[dict]:
    rows: list[dict] = []
    for batch_dir in sorted(raw.glob("t106-*")):
        m = BATCH_RE.match(batch_dir.name)
        if not m:
            continue
        csv_path = batch_dir / "scoreboard-rows.csv"
        if not csv_path.exists():
            continue
        warmup = m.group("tag") == "w"
        round_index = 0 if warmup else int(m.group("index"))
        for r in csv.DictReader(csv_path.open(encoding="utf-8")):
            run_id = r["run_id"]
            stats_path = batch_dir / "stats" / (run_id + ".json")
            stats = (
                json.loads(stats_path.read_text(encoding="utf-8"))
                if stats_path.exists()
                else {}
            )
            appendix = stats.get("appendix", {})
            counts = stats.get("type_counts", {})
            rows.append(
                {
                    "batch": batch_dir.name,
                    "fixture": m.group("fixture"),
                    "cell": m.group("generator") + "-on",
                    "warmup": warmup,
                    "round": round_index,
                    "run_index": int(r["run_index"]),
                    "wall_s": float(r["wall_seconds"]),
                    "cpu_s": float(r["cpu_seconds"]),
                    "cpu_wall": float(r["cpu_wall_ratio"]),
                    "gcode_bytes": int(r["gcode_bytes"]),
                    "sha256": r["sha256_of_gcode"],
                    "support_style": appendix.get("support_style", ""),
                    "enable_support": appendix.get("enable_support", ""),
                    "support_sections": int(counts.get("Support", 0)),
                    "support_interface_sections": int(
                        counts.get("Support interface", 0)
                    ),
                    "type_counts_json": json.dumps(counts, sort_keys=True),
                }
            )
    return rows


def starve_filter(ratios: list[float]) -> tuple[list[int], list[int]]:
    """Standing starvation rule (PERF-HANDOFF 10.3): cpu/wall below 0.75x the
    group's best ratio could have held its CPU and did not. Return retained and
    excluded indices; excluded samples stay in runs.csv, never deleted."""
    if not ratios:
        return [], []
    threshold = 0.75 * max(ratios)
    retained = [i for i, q in enumerate(ratios) if q >= threshold]
    excluded = [i for i, q in enumerate(ratios) if q < threshold]
    return retained, excluded


# Ticket 103's canonical ledger files, exactly as its own `analyze.py` names
# them. The `-r2/-r3/-r4` files beside them are extra takes of one cell that
# ticket 103's summary does NOT use; globbing would mix them in and inflate the
# prior spread several-fold.
T103_LEDGER_CSVS = [
    "rows-benchy.csv",
    "rows-base-classic-off.csv",
    "rows-base-arachne-off.csv",
    "rows-base-classic-on.csv",
    "rows-base-arachne-on.csv",
]


def t103_orca_samples() -> dict[str, list[dict]]:
    """Ticket 103's retained Orca supports-on samples, from its canonical CSVs.

    Applies ticket 103's own starvation rule (cpu/wall below 0.75x the group's
    best), because the comparison baseline must be the numbers its summary
    reported, not every row that happens to be on disk.
    """
    groups: dict[str, list[dict]] = {}
    for name in T103_LEDGER_CSVS:
        path = T103_ROWS / name
        with path.open(encoding="utf-8-sig") as fh:
            for r in csv.DictReader(fh):
                if r["tool"] != "orca" or r["supports"] != "on":
                    continue
                if str(r["warmup"]) != "0":
                    continue
                key = r["fixture"] + "/" + r["cell"]
                groups.setdefault(key, []).append(
                    {
                        "wall_s": float(r["wall_seconds"]),
                        "cpu_wall": float(r["cpu_wall_ratio"]),
                        "gcode_bytes": int(r["gcode_bytes"]),
                        "sha256": r["sha256_of_gcode"],
                        "support_sections": int(
                            json.loads(r["type_counts_json"]).get("Support", 0)
                        ),
                        "support_interface_sections": int(
                            json.loads(r["type_counts_json"]).get(
                                "Support interface", 0
                            )
                        ),
                    }
                )
    by_cell: dict[str, list[dict]] = {}
    for key, group in groups.items():
        kept, _ = starve_filter([s["cpu_wall"] for s in group])
        by_cell[key] = [group[i] for i in kept]
    return by_cell


def t103_pnp_wall(fixture: str, cell: str) -> float:
    t103 = json.loads(T103.read_text(encoding="utf-8"))
    return float(t103["cells"][fixture + "/" + cell]["tools"]["pnp-ordinary"]["wall_s"])


def t105_transfer(fixture: str, cell: str) -> dict:
    t105 = json.loads(T105.read_text(encoding="utf-8"))
    c = t105["cells"][fixture + "/" + cell]
    return {
        "style_wall_delta_pct": float(c["style_wall_delta_pct"]),
        "derived_ratio_vs_pnp": float(c["arms"]["strong"]["ratio_vs_pnp"]),
    }


def summarize(rows: list[dict]) -> dict:
    measured = [r for r in rows if not r["warmup"]]
    t103_orca = t103_orca_samples()
    cells: dict[str, dict] = {}
    keys = sorted({(r["fixture"], r["cell"]) for r in measured})
    for fixture, cell in keys:
        cell_key = fixture + "/" + cell
        group = [r for r in measured if r["fixture"] == fixture and r["cell"] == cell]
        group.sort(key=lambda r: r["round"])
        retained, excluded = starve_filter([r["cpu_wall"] for r in group])
        kept = [group[i] for i in retained]
        walls = [r["wall_s"] for r in kept]
        pnp_wall = t103_pnp_wall(fixture, cell)

        prior = t103_orca.get(cell_key, [])
        prior_walls = [s["wall_s"] for s in prior]
        prior_median = statistics.median(prior_walls) if prior_walls else None

        transfer = t105_transfer(fixture, cell)
        predicted_wall = (
            prior_median * (1.0 + transfer["style_wall_delta_pct"] / 100.0)
            if prior_median is not None
            else None
        )

        out: dict = {
            "samples": len(group),
            "retained": len(kept),
            "excluded_rounds": [group[i]["round"] for i in excluded],
            "resolved_styles": sorted({r["support_style"] for r in group}),
            "resolved_enable_support": sorted({r["enable_support"] for r in group}),
            "median_wall_s": round(statistics.median(walls), 4) if walls else None,
            "median_cpu_s": round(statistics.median([r["cpu_s"] for r in kept]), 4)
            if kept
            else None,
            "cpu_wall_median": round(
                statistics.median([r["cpu_wall"] for r in kept]), 4
            )
            if kept
            else None,
            "wall_range_s": [round(min(walls), 4), round(max(walls), 4)]
            if walls
            else None,
            "spread_s": round(max(walls) - min(walls), 4) if walls else None,
            "gcode_bytes": sorted({r["gcode_bytes"] for r in kept}),
            "sha256": sorted({r["sha256"] for r in kept}),
            "support_sections": sorted({r["support_sections"] for r in kept}),
            "support_interface_sections": sorted(
                {r["support_interface_sections"] for r in kept}
            ),
            "pnp_ordinary_wall_s": round(pnp_wall, 4),
            "t103_orca_wall_s": round(prior_median, 4)
            if prior_median is not None
            else None,
            "t103_orca_range_s": [
                round(min(prior_walls), 4),
                round(max(prior_walls), 4),
            ]
            if prior_walls
            else None,
            "t103_orca_spread_s": round(max(prior_walls) - min(prior_walls), 4)
            if prior_walls
            else None,
            "t103_orca_gcode_bytes": sorted({s["gcode_bytes"] for s in prior}),
            "t103_orca_support_sections": sorted(
                {s["support_sections"] for s in prior}
            ),
            "t103_orca_support_interface_sections": sorted(
                {s["support_interface_sections"] for s in prior}
            ),
            "t103_orca_sha256": sorted({s["sha256"] for s in prior}),
            "t105_transfer": transfer,
        }

        if out["median_wall_s"] is not None:
            out["ratio_vs_pnp"] = round(pnp_wall / out["median_wall_s"], 4)
            out["cut_to_park_pct"] = round(
                100.0 * (1.0 - 1.5 * out["median_wall_s"] / pnp_wall), 2
            )
        if prior_median is not None and out["median_wall_s"] is not None:
            delta = out["median_wall_s"] - prior_median
            out["post_pin_vs_t103_orca"] = {
                "delta_s": round(delta, 4),
                "delta_pct": round(100.0 * delta / prior_median, 2),
            }
        if predicted_wall is not None:
            transfer["predicted_wall_s"] = round(predicted_wall, 4)
            transfer["predicted_ratio_vs_pnp"] = round(pnp_wall / predicted_wall, 4)
            measured_wall = out["median_wall_s"]
            # Pre-registered agreement rule (REPRODUCE.md): the measured median
            # agrees with the transfer prediction when the absolute difference
            # fits inside the larger of the two takes' own measured ranges.
            tol = max(
                out["spread_s"] or 0.0,
                out["t103_orca_spread_s"] or 0.0,
            )
            out["transfer_check"] = {
                "measured_minus_predicted_s": round(measured_wall - predicted_wall, 4)
                if measured_wall is not None
                else None,
                "measured_minus_predicted_pct": round(
                    100.0 * (measured_wall - predicted_wall) / predicted_wall, 2
                )
                if measured_wall is not None
                else None,
                "tolerance_s": round(tol, 4),
                "tolerance_basis": "max(t106 measured range, t103 measured range)",
                "agrees": abs(measured_wall - predicted_wall) <= tol
                if measured_wall is not None
                else None,
            }

        # Non-vacuity: the pin must have changed the job. Orca's g-code SHA
        # changes every run (the appendix embeds timing), so SHA can never
        # witness a job change -- output VOLUME is the stable signal, and the
        # t105 ruling measured it moving +23.5..35.3 %.
        prior_bytes = sorted({s["gcode_bytes"] for s in prior})
        measured_bytes = out["gcode_bytes"]
        if prior_bytes and len(prior_bytes) == 1 and len(measured_bytes) == 1:
            delta_pct = 100.0 * (measured_bytes[0] - prior_bytes[0]) / prior_bytes[0]
            out["output_bytes_delta_pct_vs_t103_orca"] = round(delta_pct, 2)
            out["non_vacuous_vs_t103_orca"] = abs(delta_pct) >= 5.0
        else:
            out["output_bytes_delta_pct_vs_t103_orca"] = None
            out["non_vacuous_vs_t103_orca"] = False
        cells[cell_key] = out

    provenance = {}
    prov_path = HERE / "provenance.json"
    if prov_path.exists():
        provenance = json.loads(prov_path.read_text(encoding="utf-8"))
    return {
        "take": "t106-post-pin-supports-on-scoreboard",
        "park_ratio": 1.5,
        "support_style_pin": "tree_strong",
        "provenance": provenance,
        "cells": cells,
    }


def write_runs(rows: list[dict], path: Path) -> None:
    with path.open("w", encoding="utf-8", newline="") as f:
        w = csv.DictWriter(f, fieldnames=RUN_COLUMNS)
        w.writeheader()
        for r in rows:
            w.writerow({k: r[k] for k in RUN_COLUMNS})


def print_tables(summary: dict) -> None:
    print(
        "cell,median wall s,ratio,cut %,cpu s,cpu/wall,bytes,"
        "t103 orca s,delta %,predicted s,transfer agrees"
    )
    for cell, c in sorted(summary["cells"].items()):
        t = c.get("t105_transfer", {})
        tc = c.get("transfer_check", {})
        print(
            f"{cell},{c['median_wall_s']},{c.get('ratio_vs_pnp')},"
            f"{c.get('cut_to_park_pct')},{c['median_cpu_s']},"
            f"{c['cpu_wall_median']},{c['gcode_bytes']},"
            f"{c['t103_orca_wall_s']},"
            f"{c.get('post_pin_vs_t103_orca', {}).get('delta_pct')},"
            f"{t.get('predicted_wall_s')},{tc.get('agrees')}"
        )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=DEFAULT_RAW)
    args = ap.parse_args()
    rows = load_batches(args.raw)
    if not rows:
        raise SystemExit(f"no t106 batches found under {args.raw}")
    write_runs(rows, HERE / "runs.csv")
    summary = summarize(rows)
    (HERE / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    print_tables(summary)
    print(f"wrote {HERE / 'runs.csv'} ({len(rows)} rows) and summary.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

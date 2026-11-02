#!/usr/bin/env python3
"""t105 supports-on engine-style take -- reduce the raw rig batches.

Reads the standing rig's per-batch CSVs and stats JSONs under
`target/matched-pair/t105-<arm>-<tag>-<fixture>-<cell>/` and writes the
committed reduced set:

  runs.csv      one row per Orca run (measured and warmup), with the arm, the
                resolved `support_style`, wall/CPU, output bytes/SHA and the
                support section counts
  summary.json  per-cell, per-arm medians, the paired per-round deltas, the
                ratio against ticket 103's PnP ordinary median, and the output
                disclosure

Usage:
  python reduce.py [--raw <matched-pair dir>] [--rounds 4]

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
T103 = REPO / "docs/specs/perf-vs-orca/evidence/t103-orca-rebaseline/summary.json"

BATCH_RE = re.compile(
    r"^t105-(organic|strong)-(?P<tag>[rw])(?P<index>\d+)-"
    r"(?P<fixture>benchy|base)-(?P<generator>classic|arachne)-on$"
)
ARM_STYLE = {"organic": "default", "strong": "tree_strong"}

RUN_COLUMNS = [
    "batch",
    "fixture",
    "cell",
    "arm",
    "warmup",
    "round",
    "run_index",
    "wall_s",
    "cpu_s",
    "cpu_wall",
    "gcode_bytes",
    "sha256",
    "support_style",
    "support_sections",
    "support_interface_sections",
    "type_counts_json",
]


def load_batches(raw: Path) -> list[dict]:
    rows: list[dict] = []
    for batch_dir in sorted(raw.glob("t105-*")):
        m = BATCH_RE.match(batch_dir.name)
        if not m:
            continue
        csv_path = batch_dir / "scoreboard-rows.csv"
        if not csv_path.exists():
            continue
        arm = m.group(1)
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
                    "arm": arm,
                    "warmup": warmup,
                    "round": round_index,
                    "run_index": int(r["run_index"]),
                    "wall_s": float(r["wall_seconds"]),
                    "cpu_s": float(r["cpu_seconds"]),
                    "cpu_wall": float(r["cpu_wall_ratio"]),
                    "gcode_bytes": int(r["gcode_bytes"]),
                    "sha256": r["sha256_of_gcode"],
                    "support_style": appendix.get("support_style", ""),
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
    excluded indices; excluded samples are kept in runs.csv, never deleted."""
    if not ratios:
        return [], []
    threshold = 0.75 * max(ratios)
    retained = [i for i, q in enumerate(ratios) if q >= threshold]
    excluded = [i for i, q in enumerate(ratios) if q < threshold]
    return retained, excluded


def t103_pnp_wall(fixture: str, cell: str) -> float:
    t103 = json.loads(T103.read_text(encoding="utf-8"))
    return float(t103["cells"][fixture + "/" + cell]["tools"]["pnp-ordinary"]["wall_s"])


def summarize(rows: list[dict]) -> dict:
    measured = [r for r in rows if not r["warmup"]]
    cells: dict[str, dict] = {}
    keys = sorted({(r["fixture"], r["cell"]) for r in measured})
    for fixture, cell in keys:
        cell_key = fixture + "/" + cell
        arms: dict[str, dict] = {}
        kept_by_round: dict[str, dict[int, dict]] = {}
        for arm in ("organic", "strong"):
            group = [
                r
                for r in measured
                if r["fixture"] == fixture and r["cell"] == cell and r["arm"] == arm
            ]
            group.sort(key=lambda r: r["round"])
            retained, excluded = starve_filter([r["cpu_wall"] for r in group])
            kept = [group[i] for i in retained]
            kept_by_round[arm] = {r["round"]: r for r in kept}
            styles = sorted({r["support_style"] for r in group})
            arms[arm] = {
                "samples": len(group),
                "retained": len(kept),
                "excluded_rounds": [group[i]["round"] for i in excluded],
                "resolved_styles": styles,
                "median_wall_s": round(
                    statistics.median([r["wall_s"] for r in kept]), 4
                )
                if kept
                else None,
                "median_cpu_s": round(statistics.median([r["cpu_s"] for r in kept]), 4)
                if kept
                else None,
                "cpu_wall_median": round(
                    statistics.median([r["cpu_wall"] for r in kept]), 4
                )
                if kept
                else None,
                "gcode_bytes": sorted({r["gcode_bytes"] for r in kept}),
                "sha256": sorted({r["sha256"] for r in kept}),
                "support_sections": sorted({r["support_sections"] for r in kept}),
                "support_interface_sections": sorted(
                    {r["support_interface_sections"] for r in kept}
                ),
            }

        # Paired per-round deltas only where BOTH arms' samples survived the
        # starvation rule: a starved sample's wall is inflated by the stall, so
        # pairing it would attribute the stall to the style.
        rounds_seen = sorted(
            {
                r["round"]
                for r in measured
                if r["fixture"] == fixture and r["cell"] == cell
            }
        )
        paired_rounds = [
            rnd
            for rnd in rounds_seen
            if rnd in kept_by_round["organic"] and rnd in kept_by_round["strong"]
        ]
        deltas = [
            round(
                kept_by_round["strong"][rnd]["wall_s"]
                - kept_by_round["organic"][rnd]["wall_s"],
                4,
            )
            for rnd in paired_rounds
        ]

        pnp_wall = t103_pnp_wall(fixture, cell)
        out = {
            "pnp_ordinary_wall_s": round(pnp_wall, 4),
            "arms": arms,
            "paired_rounds_used": paired_rounds,
            "paired_rounds_excluded": [
                rnd for rnd in rounds_seen if rnd not in paired_rounds
            ],
            "paired_round_deltas_s": deltas,
            "paired_round_delta_median_s": round(statistics.median(deltas), 4)
            if deltas
            else None,
        }
        for arm in ("organic", "strong"):
            wall = arms[arm]["median_wall_s"]
            if wall is None:
                continue
            out["arms"][arm]["ratio_vs_pnp"] = round(pnp_wall / wall, 4)
            out["arms"][arm]["cut_to_park_pct"] = round(
                100.0 * (1.0 - 1.5 * wall / pnp_wall), 2
            )
        organic = out["arms"]["organic"]["median_wall_s"]
        strong = out["arms"]["strong"]["median_wall_s"]
        if organic and strong:
            out["style_wall_delta_s"] = round(strong - organic, 4)
            out["style_wall_delta_pct"] = round(100.0 * (strong - organic) / organic, 2)
            out["ratio_move_pct"] = round(
                100.0
                * (
                    out["arms"]["strong"]["ratio_vs_pnp"]
                    - out["arms"]["organic"]["ratio_vs_pnp"]
                )
                / out["arms"]["organic"]["ratio_vs_pnp"],
                2,
            )
        out["non_vacuous"] = arms["organic"]["sha256"] != arms["strong"]["sha256"]
        cells[cell_key] = out

    provenance = {}
    prov_path = HERE / "provenance.json"
    if prov_path.exists():
        provenance = json.loads(prov_path.read_text(encoding="utf-8"))
    return {
        "take": "t105-supports-on-engine-style-fairness",
        "park_ratio": 1.5,
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
    print("cell,arm,median wall s,ratio,cpu s,cpu/wall,bytes,support sections")
    for cell, c in sorted(summary["cells"].items()):
        for arm in ("organic", "strong"):
            a = c["arms"][arm]
            print(
                f"{cell},{arm},{a['median_wall_s']},{a.get('ratio_vs_pnp')},"
                f"{a['median_cpu_s']},{a['cpu_wall_median']},"
                f"{a['gcode_bytes']},{a['support_sections']}"
            )
        print(
            f"{cell},delta,{c.get('style_wall_delta_s')} s / "
            f"{c.get('style_wall_delta_pct')} %,ratio move {c.get('ratio_move_pct')} %"
        )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=DEFAULT_RAW)
    args = ap.parse_args()
    rows = load_batches(args.raw)
    if not rows:
        raise SystemExit(f"no t105 batches found under {args.raw}")
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

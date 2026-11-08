#!/usr/bin/env python3
"""Reduce the t122 probe CSVs into the committed summary JSON.

Raw input (gitignored, produced by the probe in probe.patch):
    target/t122/benchy.csv
    target/t122/base.csv

The reducer is checkout-reproducible only when the probe has been run; the
committed `reduced/` files are the evidence of record and are what `verify.py`
reads. The four per-cell walls are the t121 post-merge ordinary medians from
`evidence/t121-per-layer-dispatch/SCOREBOARD.md`; the reducer recomputes the
0.80 % floor from them rather than quoting a floor.
"""

from __future__ import annotations

import csv
import json
from pathlib import Path

EVIDENCE_DIR = Path(__file__).resolve().parent
REPO_ROOT = EVIDENCE_DIR.parents[4]
RAW_DIR = REPO_ROOT / "target" / "t122"
OUT_DIR = EVIDENCE_DIR / "reduced"

# t121 post-merge scoreboard, ordinary arm (median wall, seconds):
# evidence/t121-per-layer-dispatch/SCOREBOARD.md
CELLS = {
    "benchy": [
        ("benchy classic-off", 5.73, "classic", False),
        ("benchy classic-on", 10.64, "classic", True),
        ("benchy arachne-off", 6.01, "arachne", False),
        ("benchy arachne-on", 11.26, "arachne", True),
    ],
    "base": [
        ("base classic-off", 56.08, "classic", False),
        ("base classic-on", 132.24, "classic", True),
        ("base arachne-off", 55.43, "arachne", False),
        ("base arachne-on", 124.36, "arachne", True),
    ],
}

FLOOR_FRACTION = 0.008

INT_COLUMNS = {
    "layer",
}

FLOAT_COLUMNS = [
    "z",
    "current_area",
    "canonical_area",
    "facet_area",
    "flat_area",
    "added",
    "removed",
    "moved_solid",
    "moved_sparse",
    "moved_solid_zone",
    "moved_sparse_zone",
    "closing_area",
    "closing_bound",
    "classify_ms",
    "facet_ms",
    "flat_ms",
    "canon_class_ms",
    "expand_solid_ms",
    "expand_sparse_ms",
    "trim_ms",
    "closing_ms",
    "expand_solid_zone_ms",
    "expand_sparse_zone_ms",
    "trim_zone_ms",
    "closing_zone_ms",
]


def read_rows(path: Path) -> list[dict]:
    rows: list[dict] = []
    with path.open(newline="", encoding="utf-8") as handle:
        for raw in csv.DictReader(handle):
            row: dict = {}
            for key, value in raw.items():
                if key in INT_COLUMNS:
                    row[key] = int(value)
                elif key in FLOAT_COLUMNS:
                    row[key] = float(value)
                else:
                    raise SystemExit(f"unknown column {key!r} in {path}")
            rows.append(row)
    if not rows:
        raise SystemExit(f"no rows in {path}")
    return rows


def total(rows: list[dict], key: str) -> float:
    return sum(row[key] for row in rows)


def summarize(fixture: str, rows: list[dict]) -> dict:
    ms_classify = total(rows, "classify_ms")
    ms_facet = total(rows, "facet_ms")
    ms_flat = total(rows, "flat_ms")
    ms_canon_class = total(rows, "canon_class_ms")
    ms_expand_solid_zone = total(rows, "expand_solid_zone_ms")
    ms_expand_sparse_zone = total(rows, "expand_sparse_zone_ms")
    ms_trim_zone = total(rows, "trim_zone_ms")
    ms_closing_zone = total(rows, "closing_zone_ms")
    ms_expand_solid_whole = total(rows, "expand_solid_ms")
    ms_expand_sparse_whole = total(rows, "expand_sparse_ms")
    ms_trim_whole = total(rows, "trim_ms")
    ms_closing_whole = total(rows, "closing_ms")

    current_classification_ms = ms_classify + ms_facet + ms_flat
    expansion_only_zone_ms = (
        ms_expand_solid_zone + ms_expand_sparse_zone + ms_trim_zone + ms_closing_zone
    )
    classification_only_net_ms = current_classification_ms - ms_canon_class
    combined_net_zone_ms = classification_only_net_ms - expansion_only_zone_ms

    whole_region_added_ms = (
        ms_expand_solid_whole
        + ms_expand_sparse_whole
        + ms_trim_whole
        + ms_closing_whole
    )
    combined_net_whole_region_ms = (
        current_classification_ms - ms_canon_class - whole_region_added_ms
    )

    cells = []
    for name, wall_s, generator, supports in CELLS[fixture]:
        floor_ms = wall_s * 1000.0 * FLOOR_FRACTION
        cells.append(
            {
                "cell": name,
                "generator": generator,
                "supports": supports,
                "wall_s": wall_s,
                "floor_ms": floor_ms,
                "classification_only_multiple": classification_only_net_ms / floor_ms,
                "combined_multiple": combined_net_zone_ms / floor_ms,
            }
        )

    return {
        "fixture": fixture,
        "layer_count": len(rows),
        "first_layer": rows[0]["layer"],
        "last_layer": rows[-1]["layer"],
        "area_mm2": {
            "facet_polygon_sum": total(rows, "facet_area")
            if "facet_area" in rows[0]
            else None,
            "flat_family": total(rows, "flat_area") if "flat_area" in rows[0] else None,
            "current_bridge": total(rows, "current_area"),
            "canonical_bridge": total(rows, "canonical_area"),
            "classification_added": total(rows, "added"),
            "classification_removed": total(rows, "removed"),
            "classification_total_moved": total(rows, "added") + total(rows, "removed"),
            "expansion_moved_solid_zone": total(rows, "moved_solid_zone"),
            "expansion_moved_sparse_zone": total(rows, "moved_sparse_zone"),
            "expansion_moved_solid_whole_region": total(rows, "moved_solid"),
            "expansion_moved_sparse_whole_region": total(rows, "moved_sparse"),
            "closing_measured": total(rows, "closing_area"),
            "closing_steiner_bound": total(rows, "closing_bound"),
            "combined_role_movement_bound": (
                total(rows, "added")
                + total(rows, "removed")
                + total(rows, "moved_solid_zone")
                + max(0.0, total(rows, "closing_area"))
            ),
        },
        "ms": {
            "current_classify": ms_classify,
            "current_facet": ms_facet,
            "current_flat": ms_flat,
            "current_classification": current_classification_ms,
            "canonical_classification": ms_canon_class,
            "expansion_solid_zone": ms_expand_solid_zone,
            "expansion_sparse_zone": ms_expand_sparse_zone,
            "trim_zone": ms_trim_zone,
            "closing_zone": ms_closing_zone,
            "expansion_only_zone": expansion_only_zone_ms,
            "classification_only_net": classification_only_net_ms,
            "combined_net_zone": combined_net_zone_ms,
            "expansion_solid_whole_region": ms_expand_solid_whole,
            "expansion_sparse_whole_region": ms_expand_sparse_whole,
            "trim_whole_region": ms_trim_whole,
            "closing_whole_region": ms_closing_whole,
            "combined_net_whole_region": combined_net_whole_region_ms,
        },
        "ceilings": cells,
        "layers": rows,
    }


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    all_cells = []
    for fixture in ("benchy", "base"):
        raw = RAW_DIR / f"{fixture}.csv"
        if not raw.exists():
            raise SystemExit(
                f"missing raw input {raw}; apply probe.patch and run the probe"
            )
        summary = summarize(fixture, read_rows(raw))
        (OUT_DIR / f"summary-{fixture}.json").write_text(
            json.dumps(summary, indent=2) + "\n", encoding="utf-8"
        )
        all_cells.extend(summary["ceilings"])

    ceilings = {
        "floor_fraction": FLOOR_FRACTION,
        "wall_source": "t121 post-merge scoreboard (ordinary arm, median wall)",
        "cells": all_cells,
        "all_eight_clear": all(cell["combined_multiple"] >= 1.0 for cell in all_cells),
        "benchy_combined_negative": all(
            cell["combined_multiple"] < 0.0
            for cell in all_cells
            if cell["cell"].startswith("benchy")
        ),
        "base_combined_clears": all(
            cell["combined_multiple"] >= 1.0
            for cell in all_cells
            if cell["cell"].startswith("base")
        ),
    }
    (OUT_DIR / "ceilings.json").write_text(
        json.dumps(ceilings, indent=2) + "\n", encoding="utf-8"
    )

    for fixture in ("benchy", "base"):
        summary = json.loads(
            (OUT_DIR / f"summary-{fixture}.json").read_text(encoding="utf-8")
        )
        print(f"=== {fixture}: {summary['layer_count']} layers ===")
        print(json.dumps(summary["area_mm2"], indent=2))
        print(json.dumps(summary["ms"], indent=2))
        for cell in summary["ceilings"]:
            print(
                f"{cell['cell']}: floor {cell['floor_ms']:.2f} ms, "
                f"classification-only {cell['classification_only_multiple']:.2f}x, "
                f"combined {cell['combined_multiple']:.2f}x"
            )


if __name__ == "__main__":
    main()

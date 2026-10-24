#!/usr/bin/env python3
"""Merge the curated adjudication into `key-table.csv` and fail on gaps.

`classify.py` is mechanical and makes no judgement. This script adds three
columns — `scope`, `bucket`, `verdict` — and refuses to finish while any row is
still `bucket=review`. That is the audit's completeness gate: every one of the
resolved keys gets a classification, not just the interesting ones.

Buckets
-------
  matched           resolved value is the same work as the matched Orca job
  equal-by-rename   PnP's counterpart exists under a different key name
  equal-by-units    values differ only by unit/format (percent, 100nm units, enums)
  equal-by-clamp    values differ but canonical clamps them to the same behaviour
  emission          changes only the emitted G-code, not pre-emission geometry work
  absent-inert      PnP has no counterpart; the resolved value disables its feature
  absent-material   PnP has no counterpart and the resolved value enables geometry work
  absent-non-geometry  PnP has no counterpart and the key is not geometry work at all
  differs-inert     PnP has the counterpart, the values differ, but the consuming
                    stage is not exercised on this matched job
  geometry-differs  PnP has the counterpart, both run it, and the values differ

`scope=geometry` marks every key whose canonical role is pre-emission geometry
work. `verify.py` re-derives the counts and cross-checks the curated claims.
"""

from __future__ import annotations

import csv
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
TABLE = HERE / "key-table.csv"
VERDICTS = HERE / "verdicts.json"

# Reference tabs whose keys never change pre-emission geometry work.
EMISSION_TABS = {
    "Speed",
    "Cooling",
    "Filament",
    "Machine",
    "Calibration",
    "Extruder",
    "Printer",
    "Cross-Reference Index (alphabetical)",
}


def main() -> int:
    curated = json.loads(VERDICTS.read_text(encoding="utf-8"))
    rows = list(csv.DictReader(TABLE.open(encoding="utf-8")))

    remaining = 0
    for row in rows:
        key = row["key"]
        tab = (row["reference_tab"] or "").split(" / ")[0]
        entry = curated.get(key)

        if entry is None and row["value_relation"] in ("equal", "equal-per-cell"):
            entry = {"bucket": "matched", "scope": classify_scope(row)}
        if entry is None and row["value_relation"] == "equal-some":
            entry = {"bucket": "equal-by-units", "scope": classify_scope(row)}
        if entry is None and row["pnp_status"] == "absent":
            scope = classify_scope(row)
            if scope != "geometry":
                entry = {"bucket": "absent-non-geometry", "scope": scope}
            else:
                entry = {
                    "bucket": "absent-inert" if is_neutral(row) else "absent-material",
                    "scope": scope,
                }
        if entry is None and tab in EMISSION_TABS:
            entry = {"bucket": "emission", "scope": "emission"}
        if entry is None:
            entry = {"bucket": "review", "scope": classify_scope(row)}
            remaining += 1

        row["scope"] = entry["scope"]
        row["bucket"] = entry["bucket"]
        row["verdict"] = entry.get("note", "")

    fields = list(rows[0].keys())
    with TABLE.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)

    counts: dict[str, int] = {}
    for row in rows:
        counts[row["bucket"]] = counts.get(row["bucket"], 0) + 1
    geometry = sum(1 for row in rows if row["scope"] == "geometry")

    summary = {
        "rows": len(rows),
        "geometry_scope": geometry,
        "buckets": counts,
        "geometry_differs": sorted(
            row["key"] for row in rows if row["bucket"] == "geometry-differs"
        ),
        "resolved_not_in_reference": sum(1 for row in rows if not row["reference_tab"]),
        "absent_material": sorted(
            row["key"] for row in rows if row["bucket"] == "absent-material"
        ),
    }
    (HERE / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps(summary, indent=2))
    if remaining:
        print(f"UNCLASSIFIED: {remaining} rows still bucket=review")
        for row in rows:
            if row["bucket"] == "review":
                print(
                    f"  {row['key']:42s} orca={row['orca_value'][:24]:24s} pnp={row['pnp_defaults'][:24]}"
                )
        return 1
    return 0


def is_neutral(row: dict[str, str]) -> bool:
    value = (row["orca_value"] or row["orca_values_by_cell"]).strip().lower()
    if value.startswith(("disabled", "no ", "none", "nowhere")):
        return True
    try:
        return float(value.rstrip("%")) == 0.0
    except ValueError:
        return value in ("", "false", "default", "auto", "type2")


def classify_scope(row: dict[str, str]) -> str:
    tab = (row["reference_tab"] or "").split(" / ")[0]
    section = row["reference_section"] or row["inventory_section"]
    if tab in EMISSION_TABS:
        return "emission"
    if tab in ("Quality", "Strength", "Support", "Others"):
        if section.startswith(
            (
                "Seam",
                "Ironing",
                "Special mode",
                "G-code output",
                "Notes",
                "Post-processing",
            )
        ):
            return "emission"
        return "geometry"
    return "other"


if __name__ == "__main__":
    raise SystemExit(main())

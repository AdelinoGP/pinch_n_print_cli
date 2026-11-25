#!/usr/bin/env python3
"""Ticket 135 Stage 1 delta audit: the port's blast radius is skirt/brim only.

Compares each cell's pre-port g-code against the ported g-code. The meaningful
claim is narrower and stronger than a raw line diff:

  **every motion line outside the `Skirt` and `Brim` sections is identical once
  the extrusion field is removed, and its extrusion differs by at most
  1e-3 mm.**

Why the E caveat: PnP derives each move's relative `E` from a running extrusion
accumulator, so changing the adhesion volume shifts the accumulated total and
re-rounds later moves by <=1e-3 mm. Coordinates, feedrates, Z values, travels
and feature order are byte-identical; the printed geometry does not move.

Travels *inside* the adhesion sections, and the resolved-config block (where the
module's two new keys appear), are excluded by construction: a travel's start
point legitimately moves when the preceding adhesion loop ends elsewhere.

Usage:
    python audit_delta.py --old-dir <dir> --new-dir <dir> [--json-out audit.json]

Each directory holds its arm's `<cell><suffix>.gcode` files.
"""

from __future__ import annotations

import argparse
import collections
import json
import re
import sys
from pathlib import Path

FEATURE = re.compile(r"^;TYPE:(.+?)\s*$")
MOTION = re.compile(r"^G[01] ")
EXTRUSION = re.compile(r"\sE(-?(?:\d+(?:\.\d+)?|\.\d+))")
CONFIG_START = "; CONFIG_BLOCK_START"
CONFIG_END = "; CONFIG_BLOCK_END"
ADHESION = {"Skirt", "Brim"}
E_TOLERANCE_MM = 1e-3


def parse(path: Path) -> tuple[dict[str, list[str]], list[str], collections.Counter]:
    """Returns (feature -> motion lines, feature order, raw-line multiset)."""
    sections: dict[str, list[str]] = {}
    order: list[str] = []
    raw: collections.Counter = collections.Counter()
    current = "<none>"
    in_config = False
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith(CONFIG_START):
            in_config = True
        if in_config:
            current = "<config>"
            if current not in sections:
                sections[current] = []
                order.append(current)
            raw[("<config>", line)] += 1
            if line.startswith(CONFIG_END):
                in_config = False
            continue
        m = FEATURE.match(line)
        if m:
            current = m.group(1).strip()
            if current not in sections:
                sections[current] = []
                order.append(current)
        if MOTION.match(line):
            sections[current].append(line)
        raw[(current, line)] += 1
    return sections, order, raw


def strip_e(line: str) -> str:
    return EXTRUSION.sub("", line)


def audit_cell(old: Path, new: Path) -> dict:
    old_sections, old_order, old_raw = parse(old)
    new_sections, new_order, new_raw = parse(new)

    failures: list[dict] = []
    geometry_sections: list[dict] = []
    max_de = 0.0
    e_changed = 0
    e_total = 0
    for feature in old_order:
        if feature in ADHESION or feature == "<config>":
            continue
        old_motion = old_sections.get(feature, [])
        new_motion = new_sections.get(feature, [])
        identical_motion = [strip_e(a) for a in old_motion] == [
            strip_e(b) for b in new_motion
        ]
        geometry_sections.append(
            {
                "feature": feature,
                "motion_lines": len(old_motion),
                "identical": identical_motion,
            }
        )
        if not identical_motion:
            first = next(
                (
                    i
                    for i, (a, b) in enumerate(zip(old_motion, new_motion))
                    if strip_e(a) != strip_e(b)
                ),
                min(len(old_motion), len(new_motion)),
            )
            failures.append(
                {
                    "feature": feature,
                    "old_count": len(old_motion),
                    "new_count": len(new_motion),
                    "first_diff_index": first,
                    "old_sample": old_motion[first : first + 3],
                    "new_sample": new_motion[first : first + 3],
                }
            )
        for a, b in zip(old_motion, new_motion):
            ea, eb = EXTRUSION.search(a), EXTRUSION.search(b)
            if ea is None or eb is None:
                if (ea is None) != (eb is None):
                    failures.append(
                        {"feature": feature, "detail": "E presence differs", "line": a}
                    )
                continue
            e_total += 1
            delta = abs(float(ea.group(1)) - float(eb.group(1)))
            if delta > 0:
                e_changed += 1
                max_de = max(max_de, delta)
            if delta > E_TOLERANCE_MM:
                failures.append(
                    {
                        "feature": feature,
                        "detail": "E delta above tolerance",
                        "line": a,
                        "delta": delta,
                    }
                )

    removed = old_raw - new_raw
    added = new_raw - old_raw
    census: dict[str, dict[str, int]] = {}
    for (feature, _), count in removed.items():
        census.setdefault(feature, {"removed": 0, "added": 0})["removed"] += count
    for (feature, _), count in added.items():
        census.setdefault(feature, {"removed": 0, "added": 0})["added"] += count

    return {
        "cell": old.stem,
        "geometry_sections": geometry_sections,
        "e_stats": {
            "motion_lines_with_e": e_total,
            "e_changed_lines": e_changed,
            "max_abs_delta_mm": round(max_de, 6),
            "tolerance_mm": E_TOLERANCE_MM,
        },
        "line_census": dict(sorted(census.items())),
        "failures": failures[:5],
        "pass": not failures,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--old-dir", type=Path, required=True, help="pre-port gcode directory"
    )
    ap.add_argument(
        "--new-dir", type=Path, required=True, help="ported gcode directory"
    )
    ap.add_argument("--old-suffix", default="-baseline-ordinary")
    ap.add_argument("--new-suffix", default="-candidate-ordinary")
    ap.add_argument(
        "--cells",
        default="benchy-classic-off,benchy-classic-on,benchy-arachne-off,"
        "benchy-arachne-on,base-classic-off,base-classic-on,base-arachne-off,"
        "base-arachne-on",
    )
    ap.add_argument("--json-out", type=Path)
    args = ap.parse_args()

    cells = [c for c in args.cells.split(",") if c]
    pairs = []
    for cell in cells:
        old = args.old_dir / f"{cell}{args.old_suffix}.gcode"
        new = args.new_dir / f"{cell}{args.new_suffix}.gcode"
        if not old.exists() or not new.exists():
            raise SystemExit(f"audit: missing pair for {cell}: {old} / {new}")
        pairs.append((cell, old, new))
    if not pairs:
        raise SystemExit("audit: no matching cell pairs")

    results = [{**audit_cell(old, new), "cell": cell} for cell, old, new in pairs]
    payload = {"cells": results, "pass": all(r["pass"] for r in results)}
    print(json.dumps(payload, indent=2))
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return 0 if payload["pass"] else 1


if __name__ == "__main__":
    sys.exit(main())

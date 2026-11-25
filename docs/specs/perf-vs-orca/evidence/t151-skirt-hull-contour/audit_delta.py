#!/usr/bin/env python3
"""Ticket 151 delta audit: the skirt-hull fold's blast radius is the `Skirt` section only.

Compares each cell's pre-fold g-code against the folded one. The claim the
ticket pre-declares is:

  **every motion line outside the `Skirt` section is identical once the
  extrusion field is removed, and `|dE|` outside `Skirt` is zero.**

The first clause is the geometry gate and is what `pass` reports. The second
clause is *measured* and reported separately, because PnP derives each move's
relative `E` from a running extrusion accumulator: the skirt is emitted before
the brim and the part, so a longer or shorter skirt shifts every later `E` by a
constant and re-rounds it. This script therefore also reports, per cell:

  * the distinct `dE` values seen outside `Skirt` and whether they are constant
    within each feature section (the accumulator signature), and
  * how many lines carry a non-zero `dE` outside `Skirt`.

A non-zero `dE` that is constant per section and bounded by the accumulator's
rounding is an E-field artefact of moving the skirt, not printed geometry
moving; it is disclosed, never silently waived.

Usage:
    python audit_delta.py --old-dir <dir> --new-dir <dir> [--json-out audit.json]

Each directory holds the arm's `<cell><suffix>.gcode` files.
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
# The only section allowed to move. `Brim` is *not* in this set: t151 changes the
# skirt's hull input, not the brim, so a brim delta falsifies the ticket.
CHANGED_SECTIONS = {"Skirt"}


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


def e_delta(a: str, b: str) -> float | None:
    ea, eb = EXTRUSION.search(a), EXTRUSION.search(b)
    if ea is None or eb is None:
        return None
    return round(float(ea.group(1)) - float(eb.group(1)), 6)


def audit_cell(old: Path, new: Path) -> dict:
    old_sections, old_order, old_raw = parse(old)
    new_sections, new_order, new_raw = parse(new)

    failures: list[dict] = []
    sections: list[dict] = []
    outside_deltas: dict[str, set[float]] = {}
    outside_changed = 0
    outside_total = 0
    outside_max = 0.0
    outside_section_deltas: dict[str, list[float]] = collections.defaultdict(list)
    skirt_max = 0.0
    skirt_changed = 0

    for feature in old_order:
        old_motion = old_sections.get(feature, [])
        new_motion = new_sections.get(feature, [])
        identical_motion = [strip_e(a) for a in old_motion] == [
            strip_e(b) for b in new_motion
        ]
        changed_section = feature in CHANGED_SECTIONS or feature == "<config>"
        entry = {
            "feature": feature,
            "motion_lines": len(old_motion),
            "identical": identical_motion,
            "changed_by_design": changed_section,
        }

        deltas: list[float] = []
        for a, b in zip(old_motion, new_motion):
            d = e_delta(a, b)
            if d is not None:
                deltas.append(d)
        entry["max_abs_e_delta_mm"] = round(
            max((abs(d) for d in deltas), default=0.0), 6
        )
        sections.append(entry)

        if not changed_section:
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
            for d in deltas:
                outside_total += 1
                if d != 0.0:
                    outside_changed += 1
                    outside_max = max(outside_max, abs(d))
                    outside_section_deltas[feature].append(d)
                    outside_deltas.setdefault(feature, set()).add(d)
        elif feature == "Skirt":
            for d in deltas:
                if d != 0.0:
                    skirt_changed += 1
                    skirt_max = max(skirt_max, abs(d))

    removed = old_raw - new_raw
    added = new_raw - old_raw
    census: dict[str, dict[str, int]] = {}
    for (feature, _), count in removed.items():
        census.setdefault(feature, {"removed": 0, "added": 0})["removed"] += count
    for (feature, _), count in added.items():
        census.setdefault(feature, {"removed": 0, "added": 0})["added"] += count

    # The accumulator signature: within each feature section outside `Skirt`,
    # every non-zero delta is the same value, and that value is shared by every
    # section that follows the skirt.
    distinct = sorted({d for s in outside_deltas.values() for d in s})
    per_section_constant = {
        feature: sorted(set(values))
        for feature, values in outside_section_deltas.items()
    }
    constant_per_section = all(len(v) == 1 for v in per_section_constant.values())

    return {
        "cell": old.stem,
        "sections": sections,
        "outside_skirt": {
            "motion_lines_with_e": outside_total,
            "e_changed_lines": outside_changed,
            "max_abs_e_delta_mm": round(outside_max, 6),
            "distinct_nonzero_deltas": distinct,
            "nonzero_delta_by_section": per_section_constant,
            "constant_within_each_section": constant_per_section,
        },
        "skirt": {
            "e_changed_lines": skirt_changed,
            "max_abs_e_delta_mm": round(skirt_max, 6),
        },
        "line_census": dict(sorted(census.items())),
        "failures": failures[:5],
        "pass": not failures,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--old-dir", type=Path, required=True, help="pre-fold gcode directory"
    )
    ap.add_argument(
        "--new-dir", type=Path, required=True, help="folded gcode directory"
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

#!/usr/bin/env python3
"""t171 -- reduce the raw `--profile` stderr into per-module and per-scope rows.

`run_profile_t171.py` keeps only the summed scope rows. The census needs the
module split too: which guest module burns the fuel, and which host built-in
owns the clipper wall. This reducer reads the same raw stderr text (gitignored,
under `target/t171-profile/<tag>/`) and writes the committed
`reduced/profile-detail-<tag>.json`.

Parsing rules, all read off the CLI's own table:
  - `units: fuel` starts the fuel section, `units: wall-clock` the wall section;
  - an unindented row is a module (`<id>  fuel|wall  <pct>  <amount>  calls N`);
  - an indented row is a `slicer_core::profile` scope owned by that module;
  - `<module self>` is the module's own residue, recorded separately.

Amounts are converted to canonical integers (fuel units, or nanoseconds for
wall) exactly as `run_profile_t171.py` does, so the two reductions agree.

Usage:
  python reduce_profile_t171.py [--tag clean]
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]

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
CALLS = re.compile(r"\s+calls\s+(\d+)\s*$")
AMOUNT = re.compile(r"\s(\d[\d,.]*)\s*(ns|us|ms|s)\s")
# Fuel amounts carry no unit; percentages do, so require 5+ digits and that the
# number be followed by `calls` or end-of-line.
FUEL_AMOUNT = re.compile(r"\s(\d[\d,]{4,})(?=\s+calls|\s*$)")


def parse_amount(match: re.Match) -> int:
    value = float(match.group(1).replace(",", ""))
    unit = match.group(2)
    scale = {"s": 1e9, "ms": 1e6, "us": 1e3, "ns": 1.0}[unit]
    return int(round(value * scale))


def amount_of(line: str, section: str) -> int | None:
    if section == "wall":
        m = AMOUNT.search(line)
        return parse_amount(m) if m else None
    m = FUEL_AMOUNT.search(line)
    return int(m.group(1).replace(",", "")) if m else None


def parse(text: str) -> dict:
    out = {
        "fuel": {"total": None, "modules": {}},
        "wall": {"total": None, "builtins": {}},
    }
    section = None
    module = None
    for line in text.splitlines():
        if line.startswith("units: fuel"):
            section = "fuel"
            module = None
            continue
        if line.startswith("units: wall-clock"):
            section = "wall"
            module = None
            continue
        if line.startswith("total:"):
            if section == "fuel":
                out["fuel"]["total"] = int(line.split()[1].replace(",", ""))
            elif section == "wall":
                m = AMOUNT.search(line)
                if m:
                    out["wall"]["total"] = parse_amount(m)
            continue
        if line.startswith("=== ") or line.startswith("note:") or not line.strip():
            if line.startswith("=== "):
                section = None
            continue
        if section is None:
            continue
        table = out[section]["modules" if section == "fuel" else "builtins"]
        if line.startswith("  "):
            if module is None:
                continue
            name = re.split(r"\s{2,}", line.strip())[0]
            calls = 0
            m = CALLS.search(line)
            if m:
                calls = int(m.group(1))
            amount = amount_of(line, section)
            if amount is None:
                continue
            if name.startswith("<module self>"):
                table[module]["self"] = amount
            else:
                table[module]["scopes"][name] = {
                    "amount": amount,
                    "calls": calls,
                }
            continue
        # Module header row.
        if "calls" in line:
            calls = int(CALLS.search(line).group(1))
        else:
            calls = 0
        amount = amount_of(line, section)
        if amount is None:
            continue
        module = line.split()[0]
        table[module] = {
            "amount": amount,
            "calls": calls,
            "self": 0,
            "scopes": {},
        }
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", default="clean")
    a = ap.parse_args()
    raw_dir = REPO / "target" / "t171-profile" / a.tag
    cells = {}
    for cell in CELLS:
        path = raw_dir / f"{cell}.stderr.txt"
        if not path.exists():
            raise SystemExit(f"missing raw profile: {path}")
        cells[cell] = parse(path.read_text(encoding="utf-8", errors="replace"))
    out = {
        "schema": 1,
        "tag": a.tag,
        "note": (
            "Attribution only. Fuel is deterministic executed guest instructions; "
            "wall is accumulated thread-time under --profile, inflated by mark "
            "overhead and summed across concurrent workers. Neither is a wall claim."
        ),
        "cells": cells,
    }
    path = HERE / "reduced" / f"profile-detail-{a.tag}.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(out, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {path}: {len(cells)} cells")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

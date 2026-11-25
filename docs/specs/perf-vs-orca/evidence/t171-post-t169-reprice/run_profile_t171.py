#!/usr/bin/env python3
"""t171 attribution: per-scope totals from `--profile` on the current tree.

The instrumented passes give the phase and stage brackets; this script measures
*where inside the prepass* the surviving work sits. `pnp_cli slice --profile`
prints a ranked table whose indented rows are `slicer_core::profile` scopes
(`polygon_ops::offset`, `polygon_ops::offset2_ex`, `polygon_ops::opening`, ...),
separately for the fuel section (guest modules) and the wall-clock section
(native host built-ins). Each scope row is self-cost, so the sum over modules is
the scope's run-wide self-time.

Everything here is attribution: `--profile` costs throughput, so no wall or CPU
number from this script is a speed claim.

Usage:
  python run_profile_t171.py [--cells a,b] [--tag clean|probe]
Writes: target/t171-profile/<tag>/<cell>.stderr.txt and reduced/profile-<tag>.json
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
EXE = REPO / "target" / "release" / "pnp_cli.exe"
MODULES = REPO / "modules" / "core-modules"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {"benchy": REPO / "tmp" / "3dbenchy.stl", "base": REPO / "tmp" / "base.stl"}
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
PCT = re.compile(r"\s(\s*n/a|\s*\d+\.\d+)%\s+(.+?)\s*(?:calls \d+)?$")


def parse_amount(text: str) -> int:
    """Amount cell -> canonical integer: nanoseconds for wall, units for fuel."""
    t = text.strip()
    if t.endswith(" s"):
        return int(round(float(t[:-2]) * 1e9))
    if t.endswith(" ms"):
        return int(round(float(t[:-3]) * 1e6))
    if t.endswith(" us"):
        return int(round(float(t[:-3]) * 1e3))
    if t.endswith(" ns"):
        return int(round(float(t[:-3])))
    return int(t.replace(",", ""))


def parse_profile(text: str) -> dict:
    """{section: {scope: self_cost}}, section in {"fuel", "wall"}.

    Fuel is per-module (guest) and carries `<module self>` rows; wall is
    per-host-built-in. Scope rows are summed across modules for the run-wide
    self-cost, which is the quantity t131's census and t169's §7 quote.
    """
    section = None
    out = {"fuel": {}, "wall": {}}
    for line in text.splitlines():
        if line.startswith("units: fuel"):
            section = "fuel"
            continue
        if line.startswith("units: wall-clock"):
            section = "wall"
            continue
        if line.startswith("=== ") or line.startswith("note:"):
            section = None
            continue
        if section is None or not line.startswith("  "):
            continue
        stripped = line[2:]
        if stripped.startswith("<module self>"):
            continue
        m = PCT.search(line)
        if not m:
            continue
        name = stripped[: m.start(1)].strip()
        if not name or name.startswith("<"):
            continue
        out[section][name] = out[section].get(name, 0) + parse_amount(m.group(2))
    return out


def run(cell: str, tag: str) -> dict:
    fixture, generator, supports = cell.split("-", 2)
    cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
    out_dir = REPO / "target" / "t171-profile" / tag
    out_dir.mkdir(parents=True, exist_ok=True)
    gcode = out_dir / f"{cell}.gcode"
    stderr = out_dir / f"{cell}.stderr.txt"
    args = [
        str(EXE),
        "slice",
        "--model",
        str(FIXTURES[fixture]),
        "--output",
        str(gcode),
        "--config",
        str(cfg),
        "--module-dir",
        str(MODULES),
        "--no-default-module-paths",
        "--profile",
        "--no-progress-events",
    ]
    env = dict(os.environ, RAYON_NUM_THREADS="12")
    with stderr.open("wb") as errf:
        rc = subprocess.run(
            args, stdout=subprocess.DEVNULL, stderr=errf, env=env
        ).returncode
    text = stderr.read_text(encoding="utf-8", errors="replace")
    gcode.unlink(missing_ok=True)
    return {"exit_code": rc, "scopes": parse_profile(text)}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cells", default=",".join(CELLS))
    ap.add_argument("--tag", default="clean")
    a = ap.parse_args()
    cells = [c.strip() for c in a.cells.split(",") if c.strip()]
    doc = {
        "schema": 1,
        "tag": a.tag,
        "exe": str(EXE),
        "cells": {},
        "notes": "attribution only; --profile inflates wall and sums concurrent spans",
    }
    failures = 0
    for cell in cells:
        got = run(cell, a.tag)
        doc["cells"][cell] = got
        print(
            f"{a.tag} {cell}: exit={got['exit_code']} "
            f"scopes={len(got['scopes']['fuel'])} fuel / {len(got['scopes']['wall'])} wall",
            flush=True,
        )
        if got["exit_code"] != 0:
            failures += 1
    out = HERE / "reduced" / f"profile-{a.tag}.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print("wrote", out)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())

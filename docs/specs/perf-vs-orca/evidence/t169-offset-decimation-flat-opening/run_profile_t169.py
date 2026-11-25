#!/usr/bin/env python3
"""t169 attribution: per-scope totals from `--profile` on both arms.

The paired campaign measures wall; this script measures *where* the work moved.
`pnp_cli slice --profile` prints a ranked table whose indented rows are
`slicer_core::profile` scopes (`polygon_ops::offset`, `polygon_ops::offset2_ex`,
`polygon_ops::opening`, ...), separately for the fuel section (guest modules)
and the wall-clock section (host built-ins). Each scope row is self-cost, so the
sum over modules is the scope's run-wide self-time.

Everything here is attribution: `--profile` costs throughput, so no wall or CPU
number from this script is a speed claim.

Usage:
  python run_profile_t169.py [--cells a,b] [--arms baseline,candidate]
Writes: target/t169-profile/<arm>/<cell>.stderr.txt and reduced/profile.json
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
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {"benchy": REPO / "tmp" / "3dbenchy.stl", "base": REPO / "tmp" / "base.stl"}
ARMS = {
    "baseline": (
        REPO / "target" / "t169-baseline" / "pnp_cli-ordinary.exe",
        REPO / "target" / "t169-baseline" / "modules",
    ),
    "candidate": (
        REPO / "target" / "t169-candidate" / "pnp_cli-ordinary.exe",
        REPO / "target" / "t169-candidate" / "modules",
    ),
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
PCT = re.compile(r"\s(\s*n/a|\s*\d+\.\d+)%\s+(.+?)\s*(?:calls \d+)?$")
SCOPE = re.compile(r"^  (\S.*?)\s{2,}")


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
    """{section: {scope: self_cost}}, section in {"fuel", "wall"}."""
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


def run(arm: str, cell: str, out_dir: Path) -> dict:
    exe, modules = ARMS[arm]
    fixture, generator, supports = cell.split("-", 2)
    cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
    gcode = out_dir / f"{cell}.gcode"
    stderr = out_dir / f"{cell}.stderr.txt"
    args = [
        str(exe),
        "slice",
        "--model",
        str(FIXTURES[fixture]),
        "--output",
        str(gcode),
        "--config",
        str(cfg),
        "--module-dir",
        str(modules),
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
    ap.add_argument("--arms", default="baseline,candidate")
    a = ap.parse_args()
    cells = [c.strip() for c in a.cells.split(",") if c.strip()]
    arms = [x.strip() for x in a.arms.split(",") if x.strip()]
    doc = {
        "schema": 1,
        "arms": {},
        "notes": "attribution only; --profile inflates wall",
    }
    failures = 0
    for arm in arms:
        doc["arms"][arm] = {}
        out_dir = REPO / "target" / "t169-profile" / arm
        out_dir.mkdir(parents=True, exist_ok=True)
        for cell in cells:
            got = run(arm, cell, out_dir)
            doc["arms"][arm][cell] = got
            print(
                f"{arm} {cell}: exit={got['exit_code']} scopes={len(got['scopes']['fuel'])} fuel / {len(got['scopes']['wall'])} wall",
                flush=True,
            )
            if got["exit_code"] != 0:
                failures += 1
    out = HERE / "reduced" / "profile.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print("wrote", out)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())

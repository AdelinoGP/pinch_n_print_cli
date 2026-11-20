#!/usr/bin/env python3
"""t173: run the boolean-volume probe on the five ticket cells.

Attribution only: the probe records per-call thread-time and operand counts;
`--profile` inflates wall and the probe adds bookkeeping, so no number here is a
wall claim or a ratio of record.

Usage:
  python run_probe_t173.py [--cells a,b] [--tag probe]
Writes: target/t173-probe-runs/<tag>/<cell>.stderr.txt and reduced/probe-<tag>.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
EXE = REPO / "target" / "release" / "pnp_cli.exe"
MODULES = REPO / "modules" / "core-modules"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {"benchy": REPO / "tmp" / "3dbenchy.stl", "base": REPO / "tmp" / "base.stl"}
REFERENCE = HERE.parent / "t172-offset-concave-crossing" / "reference.json"
CELLS = [
    "base-classic-on",
    "base-arachne-on",
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-off",
]


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def run(cell: str, tag: str, refs: dict[str, str]) -> dict:
    fixture, generator, supports = cell.split("-", 2)
    cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
    out_dir = REPO / "target" / "t173-probe-runs" / tag
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
    env = dict(os.environ, RAYON_NUM_THREADS="12", PNP_T173_PROBE="1")
    with stderr.open("wb") as errf:
        rc = subprocess.run(
            args, stdout=subprocess.DEVNULL, stderr=errf, env=env
        ).returncode
    got = sha256(gcode) if gcode.exists() else ""
    want = refs.get(cell)
    ok = want is not None and got == want
    gcode.unlink(missing_ok=True)
    return {
        "cell": cell,
        "exit_code": rc,
        "gcode_sha256": got,
        "reference_sha256": want,
        "reference_match": ok,
        "stderr": str(stderr.relative_to(REPO)),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cells", default=",".join(CELLS))
    ap.add_argument("--tag", default="probe")
    a = ap.parse_args()
    refs = json.loads(REFERENCE.read_text(encoding="utf-8"))["active_set"]
    doc = {
        "schema": 1,
        "tag": a.tag,
        "exe": str(EXE),
        "exe_sha256": sha256(EXE),
        "cells": {},
        "notes": "probe + --profile attribution only; neither instrument is a wall claim",
    }
    failures = 0
    for cell in [c.strip() for c in a.cells.split(",") if c.strip()]:
        row = run(cell, a.tag, refs)
        doc["cells"][cell] = row
        print(
            f"{cell}: exit={row['exit_code']} ref_match={row['reference_match']} "
            f"sha={row['gcode_sha256'][:12]}",
            flush=True,
        )
        if row["exit_code"] != 0 or not row["reference_match"]:
            failures += 1
    out = HERE / "reduced" / f"runs-{a.tag}.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(doc, indent=1) + "\n", encoding="utf-8")
    print("wrote", out)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())

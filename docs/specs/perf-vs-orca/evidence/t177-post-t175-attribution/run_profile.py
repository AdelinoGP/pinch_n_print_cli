#!/usr/bin/env python3
"""t177 attribution: per-scope totals from `--profile` on the post-t175 tree.

Every `--profile` pass is checked for exit_code == 0, code-1200 count == 0,
and byte-identical G-code SHA-256 + byte size against t175's active reference
set BEFORE removing the temporary `.gcode` file.

Everything here is attribution only: `--profile` inflates wall and sums
concurrent spans across worker threads, so no wall or CPU number from this
script is a speed claim.

Usage:
  python run_profile.py [--cells a,b] [--tag clean]
Writes:
  target/t177-profile/<tag>/<cell>.stderr.txt
  reduced/profile-<tag>.json
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
EXE = REPO / "target" / "release" / "pnp_cli.exe"
MODULES = REPO / "modules" / "core-modules"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {"benchy": REPO / "tmp" / "3dbenchy.stl", "base": REPO / "tmp" / "base.stl"}
REFERENCE = HERE.parent / "t175-tiled-collision-carve" / "reference.json"
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


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


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
        name = line[2 : m.start(1)].strip()
        if not name or name.startswith("<"):
            continue
        out[section][name] = out[section].get(name, 0) + parse_amount(m.group(2))
    return out


def count_code_1200(text: str) -> int:
    hits = 0
    for line in text.splitlines():
        if (
            '"code":1200' in line
            or '"code": 1200' in line
            or "exact-Z occupancy" in line
        ):
            hits += 1
    return hits


def run(cell: str, tag: str, refs: dict[str, str], ref_bytes: dict[str, int]) -> dict:
    fixture, generator, supports = cell.split("-", 2)
    cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
    out_dir = REPO / "target" / "t177-profile" / tag
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
    t0 = time.perf_counter()
    with stderr.open("wb") as errf:
        rc = subprocess.run(
            args, stdout=subprocess.DEVNULL, stderr=errf, env=env
        ).returncode
    wall = time.perf_counter() - t0
    got_sha = sha256_file(gcode) if gcode.exists() else ""
    got_bytes = gcode.stat().st_size if gcode.exists() else 0
    gcode.unlink(missing_ok=True)
    want_sha = refs.get(cell)
    want_bytes = ref_bytes.get(cell)
    ref_ok = (want_sha is not None and got_sha == want_sha) and (
        want_bytes is None or got_bytes == want_bytes
    )
    text = stderr.read_text(encoding="utf-8", errors="replace")
    c1200 = count_code_1200(text)
    return {
        "exit_code": rc,
        "profile_wall_seconds": round(wall, 3),
        "gcode_sha256": got_sha,
        "gcode_bytes": got_bytes,
        "reference_sha256": want_sha,
        "reference_bytes": want_bytes,
        "reference_match": ref_ok,
        "code_1200_count": c1200,
        "scopes": parse_profile(text),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cells", default=",".join(CELLS))
    ap.add_argument("--tag", default="clean")
    a = ap.parse_args()
    ref_doc = json.loads(REFERENCE.read_text(encoding="utf-8"))
    refs = ref_doc["active_set"]
    ref_bytes = ref_doc["bytes"]
    cells = [c.strip() for c in a.cells.split(",") if c.strip()]
    doc = {
        "schema": 1,
        "tag": a.tag,
        "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "exe": EXE.relative_to(REPO).as_posix(),
        "exe_sha256": sha256_file(EXE),
        "reference_artifact": REFERENCE.relative_to(REPO).as_posix(),
        "cells": {},
        "notes": "attribution only; --profile inflates wall and sums concurrent spans",
    }
    failures = 0
    for cell in cells:
        got = run(cell, a.tag, refs, ref_bytes)
        doc["cells"][cell] = got
        print(
            f"{a.tag} {cell}: exit={got['exit_code']} wall={got['profile_wall_seconds']:.3f}s "
            f"sha={got['gcode_sha256'][:12]} ref={got['reference_match']} "
            f"c1200={got['code_1200_count']} "
            f"scopes={len(got['scopes']['fuel'])} fuel / {len(got['scopes']['wall'])} wall",
            flush=True,
        )
        if got["exit_code"] != 0 or not got["reference_match"] or got["code_1200_count"] != 0:
            failures += 1
    out = HERE / "reduced" / f"profile-{a.tag}.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print("wrote", out)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())

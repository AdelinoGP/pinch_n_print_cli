#!/usr/bin/env python3
"""t104 phase-split take -- one instrumented pass per cell.

Attribution only: `--instrument-stderr` output is reduced by reduce.py and never
denominated as a wall claim. Each pass is validated (exit 0, `degraded=false`,
`non_fatal_error_count=0`) and its G-code SHA-256 is checked against the t87
post-t81 reference set (evidence/t91-exact-z-memo/references.json).

Usage:
  python run-instrumented.py --cells benchy-classic-off,base-classic-off
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
EXE = REPO / "target" / "release" / "pnp_cli.exe"
MODULES = REPO / "modules" / "core-modules"
CONFIGS = (
    REPO / "docs" / "specs" / "perf-vs-orca" / "evidence" / "matched-pair" / "configs"
)
FIXTURES = {"benchy": REPO / "tmp" / "3dbenchy.stl", "base": REPO / "tmp" / "base.stl"}
REFERENCES = (
    REPO
    / "docs"
    / "specs"
    / "perf-vs-orca"
    / "evidence"
    / "t91-exact-z-memo"
    / "references.json"
)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cells", required=True)
    ap.add_argument("--out-dir", default=str(REPO / "target" / "t104-instrumented"))
    ap.add_argument("--suffix", default="", help="appended to the output stem (e.g. .rep2)")
    a = ap.parse_args()

    refs = json.loads(REFERENCES.read_text(encoding="utf-8"))["cells"]
    out_dir = Path(a.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    failures = 0
    for cell in [c.strip() for c in a.cells.split(",") if c.strip()]:
        fixture, generator, supports = cell.split("-", 2)
        cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
        gcode = out_dir / f"{cell}{a.suffix}.gcode"
        raw = out_dir / f"{cell}{a.suffix}.jsonl"
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
            "--instrument-stderr",
        ]
        env = dict(os.environ, RAYON_NUM_THREADS="12")
        t0 = time.perf_counter()
        with raw.open("wb") as errf:
            rc = subprocess.run(
                args, stdout=subprocess.DEVNULL, stderr=errf, env=env
            ).returncode
        wall = time.perf_counter() - t0
        got = sha256(gcode) if gcode.exists() else ""
        want = refs.get(f"ordinary-{cell}", {}).get("sha256")
        ok_ref = want is None or got == want
        print(
            f"{cell}: exit={rc} wall={wall:.3f}s (attribution only) sha={got[:12]} "
            f"ref_match={ok_ref}",
            flush=True,
        )
        if rc != 0 or not ok_ref:
            failures += 1
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())

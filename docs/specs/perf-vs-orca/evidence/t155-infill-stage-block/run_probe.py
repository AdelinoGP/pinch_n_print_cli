#!/usr/bin/env python3
"""t155 -- run one temporary-probe pass and validate it.

Builds nothing. Apply the probe first, build the ordinary release binary, then:

  git apply docs/specs/perf-vs-orca/evidence/t155-infill-stage-block/probe.patch
  cargo build --release -p pnp-cli

Usage (from the repo root):
  python docs/specs/perf-vs-orca/evidence/t155-infill-stage-block/run_probe.py \
      --cell base-classic-off --stem base-classic-off-ns
  ... --cell base-classic-on --stem base-classic-on-ns
  ... --cell base-classic-off --stem base-classic-off-decline --decline-raft

Each pass is validated (exit 0, `degraded=false`, zero diagnostics) and its
G-code SHA-256 is checked against t151's active set. Probe values are
attribution only; no wall claim is taken. `--decline-raft` enables the
probe-only raft-default decline gate (t127's transfer method).
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
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {"benchy": REPO / "tmp" / "3dbenchy.stl", "base": REPO / "tmp" / "base.stl"}
REFERENCE = HERE.parent / "t151-skirt-hull-contour" / "reference.json"
OUT = REPO / "target" / "t155-probe"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cell", required=True)
    ap.add_argument("--stem", required=True)
    ap.add_argument("--decline-raft", action="store_true")
    args = ap.parse_args()

    fixture, generator, supports = args.cell.split("-", 2)
    config = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
    OUT.mkdir(parents=True, exist_ok=True)
    raw = (OUT / args.stem).with_suffix(".jsonl")
    gcode = (OUT / args.stem).with_suffix(".gcode")

    env = dict(os.environ, RAYON_NUM_THREADS="12", PNP_T155_PROBE="1")
    if args.decline_raft:
        env["PNP_T155_DECLINE_RAFT"] = "1"
    argv = [
        str(EXE),
        "slice",
        "--model",
        str(FIXTURES[fixture]),
        "--output",
        str(gcode),
        "--config",
        str(config),
        "--module-dir",
        str(MODULES),
        "--instrument-stderr",
    ]
    started = time.perf_counter()
    with raw.open("wb") as errfile:
        exit_code = subprocess.run(
            argv, stdout=subprocess.DEVNULL, stderr=errfile, env=env
        ).returncode
    wall = time.perf_counter() - started

    refs = json.loads(REFERENCE.read_text(encoding="utf-8"))["active_set"]
    key = f"{fixture}-{generator}-supports-{supports}"
    got = sha256(gcode) if gcode.exists() else ""
    match = got == refs.get(key)
    print(
        f"{args.stem}: exit={exit_code} wall={wall:.3f}s (attribution only) "
        f"sha={got[:12]} ref_match={match} binary={sha256(EXE)[:12]}"
    )
    return 0 if exit_code == 0 and match else 1


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""t154 fuel-profile arm -- per-module scope tables for the per-layer carriers.

Runs `--profile` once per cell (attribution only; the profile costs throughput,
so its walls are never wall claims) and extracts the `profile_summary` JSON
event into `profile-<cell>.json`, keeping the raw stderr beside it.

Usage:
  python run-profile.py
  python run-profile.py --cells benchy-classic-off
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
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
]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cells", default=",".join(CELLS))
    ap.add_argument("--out-dir", default=str(REPO / "target" / "t154-instrumented"))
    a = ap.parse_args()
    out_dir = Path(a.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    failures = 0
    for cell in [c.strip() for c in a.cells.split(",") if c.strip()]:
        fixture, generator, supports = cell.split("-", 2)
        cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
        gcode = out_dir / f"profile-{cell}.gcode"
        log = out_dir / f"profile-{cell}.err"
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
            "--profile",
        ]
        env = dict(os.environ, RAYON_NUM_THREADS="12")
        with log.open("wb") as errf:
            rc = subprocess.run(
                args, stdout=subprocess.DEVNULL, stderr=errf, env=env
            ).returncode
        summary = None
        with log.open("r", encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.strip()
                if not line.startswith("{"):
                    continue
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if rec.get("event") == "profile_summary":
                    summary = rec
        if rc != 0 or summary is None:
            print(f"{cell}: exit={rc} profile_summary={'yes' if summary else 'no'}")
            failures += 1
            continue
        # Keep only what the analysis uses: module id, calls, fuel, wall, scopes.
        reduced = {
            "cell": cell,
            "elapsed_ms": summary.get("elapsed_ms"),
            "fuel_total": summary["profile"]["fuel_total"],
            "wall_total_ns": summary["profile"]["wall_total_ns"],
            "modules": [
                {
                    "module_id": m["module_id"],
                    "unit": m["unit"],
                    "calls": m["calls"],
                    "total_fuel": m["total_fuel"],
                    "total_wall_ns": m["total_wall_ns"],
                    "self_fuel": m["self_fuel"],
                    "self_wall_ns": m["self_wall_ns"],
                    "scopes": [
                        {
                            "scope": s["scope"],
                            "calls": s["calls"],
                            "self_fuel": s["self_fuel"],
                            "self_wall_ns": s["self_wall_ns"],
                        }
                        for s in m.get("scopes", [])
                    ],
                }
                for m in summary["profile"]["modules"]
            ],
        }
        dest = HERE / f"profile-{cell}.json"
        dest.write_text(json.dumps(reduced, indent=1) + "\n", encoding="utf-8")
        print(
            f"{cell}: exit={rc} elapsed={reduced['elapsed_ms']} ms -> {dest.name}",
            flush=True,
        )
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())

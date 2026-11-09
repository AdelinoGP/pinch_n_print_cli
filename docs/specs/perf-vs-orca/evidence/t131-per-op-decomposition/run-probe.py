#!/usr/bin/env python3
"""t131 per-op decomposition -- probe runs.

Runs one matched cell with the temporary t131 probe binary and captures the
probe's stderr stream. Attribution only: the probe adds clock reads around
every clipper primitive, so no wall claim is taken from these runs; the G-code
hash must still equal ticket 123's candidate-ordinary reference, which is the
output-neutrality proof for the probe build.

Usage:
  python run-probe.py --flags probe --cells benchy-classic-off,base-classic-off
  python run-probe.py --flags probe+s5 --cells base-classic-off
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
EXE = REPO / "target" / "release" / "pnp_cli.exe"
MODULES = REPO / "modules" / "core-modules"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {"benchy": REPO / "tmp" / "3dbenchy.stl", "base": REPO / "tmp" / "base.stl"}
T123 = HERE.parent / "t123-facet-retirement" / "reduced" / "summary.json"
CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def reference_hashes() -> dict[str, str]:
    cells = json.loads(T123.read_text(encoding="utf-8"))["output"]
    return {
        cell: payload["candidate-ordinary"]["sha256"] for cell, payload in cells.items()
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cells", default=",".join(CELLS))
    ap.add_argument("--flags", default="probe", choices=["probe", "probe+s5"])
    ap.add_argument("--out-dir", default=str(REPO / "target" / "t131-runs"))
    a = ap.parse_args()

    refs = reference_hashes()
    out_dir = Path(a.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, PNP_T131_PROBE="1", RAYON_NUM_THREADS="12")
    if a.flags == "probe+s5":
        env["PNP_T131_S5"] = "1"
    failures = 0
    rows = []
    for cell in [c.strip() for c in a.cells.split(",") if c.strip()]:
        fixture, generator, supports = cell.split("-", 2)
        cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
        tag = f"{cell}.{a.flags}"
        gcode = out_dir / f"{tag}.gcode"
        raw = out_dir / f"{tag}.jsonl"
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
        t0 = time.perf_counter()
        with raw.open("wb") as errf:
            rc = subprocess.run(
                args, stdout=subprocess.DEVNULL, stderr=errf, env=env
            ).returncode
        wall = time.perf_counter() - t0
        got = sha256(gcode) if gcode.exists() else ""
        want = refs.get(cell)
        ok_ref = want is not None and got == want
        rows.append(
            {
                "cell": cell,
                "flags": a.flags,
                "exit_code": rc,
                "instrumented_wall_seconds": round(wall, 3),
                "gcode_sha256": got,
                "reference_sha256": want,
                "reference_match": ok_ref,
                "jsonl": raw.name,
            }
        )
        print(
            f"{cell} [{a.flags}]: exit={rc} wall={wall:.3f}s "
            f"sha={got[:12]} ref_match={ok_ref} raw={raw.name}",
            flush=True,
        )
        if rc != 0 or not ok_ref:
            failures += 1
    name = "runs.json" if a.flags == "probe" else "runs-s5.json"
    (HERE / name).write_text(json.dumps(rows, indent=1) + "\n", encoding="utf-8")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())

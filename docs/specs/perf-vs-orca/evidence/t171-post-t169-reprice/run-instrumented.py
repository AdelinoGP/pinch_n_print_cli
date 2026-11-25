#!/usr/bin/env python3
"""t171 post-t169 re-price -- instrumented passes, three per matched cell.

Attribution only: `--instrument-stderr` inflates wall, so nothing here is a
wall claim. Every pass is validated (exit 0) and its G-code SHA-256 is checked
against t169's active reference set, which is the matched job's currency after
the edge-decimation port.

Usage:
  python run-instrumented.py                 # 8 cells x 3 passes
  python run-instrumented.py --passes 1      # one pass per cell (smoke)
  python run-instrumented.py --cells base-classic-on
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
FIXTURES = {
    "benchy": REPO / "tmp" / "3dbenchy.stl",
    "base": REPO / "tmp" / "base.stl",
}
REFERENCE = HERE.parent / "t169-offset-decimation-flat-opening" / "reference.json"
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


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def reference_hashes() -> dict[str, str]:
    return json.loads(REFERENCE.read_text(encoding="utf-8"))["active_set"]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cells", default=",".join(CELLS))
    ap.add_argument("--passes", type=int, default=3)
    ap.add_argument("--out-dir", default=str(REPO / "target" / "t171-instrumented"))
    a = ap.parse_args()

    refs = reference_hashes()
    out_dir = Path(a.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    failures = 0
    rows = []
    for cell in [c.strip() for c in a.cells.split(",") if c.strip()]:
        fixture, generator, supports = cell.split("-", 2)
        cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
        for p in range(1, a.passes + 1):
            gcode = out_dir / f"{cell}.pass{p}.gcode"
            raw = out_dir / f"{cell}.pass{p}.jsonl"
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
            want = refs.get(cell)
            ok_ref = want is not None and got == want
            rows.append(
                {
                    "cell": cell,
                    "pass": p,
                    "exit_code": rc,
                    "instrumented_wall_seconds": round(wall, 3),
                    "gcode_sha256": got,
                    "reference_sha256": want,
                    "reference_match": ok_ref,
                }
            )
            print(
                f"{cell} pass{p}: exit={rc} wall={wall:.3f}s (attribution only) "
                f"sha={got[:12]} ref_match={ok_ref}",
                flush=True,
            )
            if rc != 0 or not ok_ref:
                failures += 1
    (HERE / "runs.json").write_text(json.dumps(rows, indent=1) + "\n", encoding="utf-8")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())

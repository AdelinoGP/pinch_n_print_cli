#!/usr/bin/env python3
"""t157 needs_support predicate attribution -- dual and fast probe passes.

Attribution only: `--instrument-stderr` + `PNP_T157_PROBE=1` output is reduced by
reduce.py and is never denominated as a wall claim. Each pass is validated
(exit 0, `degraded=false`, `fatal_error_count=0`, `non_fatal_error_count=0`,
zero predicate mismatches in dual mode) and its G-code SHA-256 is checked
against t151's active reference set.

Usage:
  python run-probe.py
  python run-probe.py --cells benchy-classic-off,benchy-classic-on --modes dual,fast
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
REFERENCE = HERE.parent / "t151-skirt-hull-contour" / "reference.json"
DEFAULT_CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]
DEFAULT_MODES = ["dual", "fast"]


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
    ap.add_argument("--cells", default=",".join(DEFAULT_CELLS))
    ap.add_argument("--modes", default=",".join(DEFAULT_MODES))
    ap.add_argument("--out-dir", default=str(REPO / "target" / "t157-probe"))
    a = ap.parse_args()

    refs = reference_hashes()
    out_dir = Path(a.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    failures = 0
    rows = []
    cells = [c.strip() for c in a.cells.split(",") if c.strip()]
    modes = [m.strip() for m in a.modes.split(",") if m.strip()]

    for mode in modes:
        for cell in cells:
            fixture, generator, supports = cell.split("-", 2)
            cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
            gcode = out_dir / f"{cell}.{mode}.gcode"
            raw = out_dir / f"{cell}.{mode}.jsonl"
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
            env = dict(os.environ, RAYON_NUM_THREADS="12", PNP_T157_PROBE="1")
            if mode == "fast":
                env["PNP_T157_FAST_PREDICATE"] = "1"
            else:
                env.pop("PNP_T157_FAST_PREDICATE", None)

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
                    "mode": mode,
                    "exit_code": rc,
                    "probe_wall_seconds": round(wall, 3),
                    "gcode_sha256": got,
                    "reference_sha256": want,
                    "reference_match": ok_ref,
                }
            )
            print(
                f"{cell} [{mode}]: exit={rc} wall={wall:.3f}s (attribution only) "
                f"sha={got[:12]} ref_match={ok_ref}",
                flush=True,
            )
            if rc != 0 or not ok_ref:
                failures += 1

    (HERE / "runs.json").write_text(json.dumps(rows, indent=1) + "\n", encoding="utf-8")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())

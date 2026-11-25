#!/usr/bin/env python3
"""t134 probe campaign driver.

Runs the eight matched cells once each on the ordinary host binary with the
temporary `PNP_T134_PROBE` instrumentation enabled and `--instrument-stderr`
on. Every run must reproduce ticket 123's candidate-ordinary G-code hash;
a mismatch fails the run. The probe is attribution-only: wall timings here
are recorded for disclosure, never claimed as a benchmark.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import os
import subprocess
import sys
import time
from pathlib import Path

CELLS = {
    "benchy-classic-off": ("tmp/3dbenchy.stl", "pnp-classic-supports-off.json"),
    "benchy-classic-on": ("tmp/3dbenchy.stl", "pnp-classic-supports-on.json"),
    "benchy-arachne-off": ("tmp/3dbenchy.stl", "pnp-arachne-supports-off.json"),
    "benchy-arachne-on": ("tmp/3dbenchy.stl", "pnp-arachne-supports-on.json"),
    "base-classic-off": ("tmp/base.stl", "pnp-classic-supports-off.json"),
    "base-classic-on": ("tmp/base.stl", "pnp-classic-supports-on.json"),
    "base-arachne-off": ("tmp/base.stl", "pnp-arachne-supports-off.json"),
    "base-arachne-on": ("tmp/base.stl", "pnp-arachne-supports-on.json"),
}

# Ticket 123 candidate-ordinary reference set.
EXPECTED = {
    "benchy-classic-off": "90638bf3ea9e592d573b223c3eb9af64473efe804578d2478adfbbc5f0e67008",
    "benchy-classic-on": "711c6334e397e5ae6911b15bfc71d42add9e93822cddd0a29391db6ef531d556",
    "benchy-arachne-off": "2ba5d84ccf3c8deea6f39f4977dd71221faaecde9987d1699b361a5366e86380",
    "benchy-arachne-on": "a972b0f2c25a988e667ecf5a5b3d6c3f5ba87f5c8872733c2a4b4e5d51a9695e",
    "base-classic-off": "b3b4ac5e885c7f637b98f79eca21901a41d85739627d4dcb40820663c15e90ef",
    "base-classic-on": "4e31a2db368ff80f388512211ae824b9660342a465c87f093f6182023e34eb4e",
    "base-arachne-off": "6b865d4a2cb892daa3172add3e3677e0dc35192bcedd0acbc463abca7064d715",
    "base-arachne-on": "62d6eedc02e5db972928f830104aab95c3598f6df6407159cb5438adb35bef13",
}


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--repo-root", type=Path, default=Path(__file__).resolve().parents[5]
    )
    ap.add_argument("--cells", nargs="*", default=list(CELLS))
    ap.add_argument("--threads", type=int, default=12)
    args = ap.parse_args()

    root = args.repo_root.resolve()
    out_dir = root / "target" / "t134" / "runs"
    gcode_dir = out_dir / "gcode"
    gcode_dir.mkdir(parents=True, exist_ok=True)
    exe = root / "target" / "release" / "pnp_cli.exe"
    config_dir = (
        root
        / "docs"
        / "specs"
        / "perf-vs-orca"
        / "evidence"
        / "matched-pair"
        / "configs"
    )
    module_dir = root / "modules" / "core-modules"
    if not exe.exists():
        print(f"missing probe binary: {exe}", file=sys.stderr)
        return 2

    rows_path = out_dir / "rows.csv"
    with rows_path.open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(
            [
                "cell",
                "wall_seconds",
                "gcode_bytes",
                "sha256",
                "exit_code",
                "probe_env",
                "threads",
            ]
        )
        for cell in args.cells:
            model_rel, cfg_rel = CELLS[cell]
            model = root / model_rel
            cfg = config_dir / cfg_rel
            gcode = gcode_dir / f"{cell}.gcode"
            stderr = out_dir / f"{cell}.stderr.jsonl"
            env = dict(os.environ)
            env["PNP_T134_PROBE"] = "1"
            env["RAYON_NUM_THREADS"] = str(args.threads)
            cmd = [
                str(exe),
                "slice",
                "--model",
                str(model),
                "--output",
                str(gcode),
                "--config",
                str(cfg),
                "--module-dir",
                str(module_dir),
                "--no-default-module-paths",
                "--instrument-stderr",
            ]
            print(f"[t134] START {cell}", flush=True)
            t0 = time.perf_counter()
            with stderr.open("wb") as err:
                proc = subprocess.run(
                    cmd, stdout=subprocess.DEVNULL, stderr=err, env=env
                )
            wall = time.perf_counter() - t0
            digest = sha256(gcode) if gcode.exists() else ""
            size = gcode.stat().st_size if gcode.exists() else 0
            print(
                f"[t134] DONE  {cell} wall={wall:.2f}s bytes={size} sha={digest[:12]} exit={proc.returncode}",
                flush=True,
            )
            w.writerow(
                [
                    cell,
                    f"{wall:.4f}",
                    size,
                    digest,
                    proc.returncode,
                    "PNP_T134_PROBE=1",
                    args.threads,
                ]
            )
            f.flush()
            if proc.returncode != 0:
                return 1
            if digest != EXPECTED[cell]:
                print(
                    f"[t134] HASH MISMATCH {cell}: {digest} != {EXPECTED[cell]}",
                    file=sys.stderr,
                )
                return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

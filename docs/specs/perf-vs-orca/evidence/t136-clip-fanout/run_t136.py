#!/usr/bin/env python3
"""t136 probe campaign driver.

Runs the matched cells on the ordinary probe binary with `PNP_T136_PROBE=1` and
`--instrument-stderr`. Every run must reproduce the active post-t139 G-code
reference; a mismatch fails the run. The probe is attribution-only: the walls
recorded here are probe passes, never claimed as benchmark cells.
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

# Active post-t139 reference set (t142 / t135 Stage 2 hash the same eight cells).
EXPECTED = {
    "benchy-classic-off": "93e21fee6d757d37b3ae3b4efbfa2b24da178004f9a568d6647e2343ae005123",
    "benchy-classic-on": "b3f1ecccc5385f1b807c3418c271bcf57761a183e60c05eefb3294195df3d7a2",
    "benchy-arachne-off": "3510e8761d65cf3da702b9aa04dd7b0fec64ba2b16996e423bfcd46b72c1fd37",
    "benchy-arachne-on": "96cdac85437082377b351640019b3b1efe7208f6b77ada0f5b9de87035b07697",
    "base-classic-off": "ca14d0554e37eddf27c80b5052b583d6fc6eba5cf7bf551d6bb419fd45a78769",
    "base-classic-on": "585c6a8edc2fd1de72dd2a86933fdad25c7ed144662dd80bbc68fb7856e4f629",
    "base-arachne-off": "d8fdc1d990b7763dd592311c333e1f2b0cd7f7beb9885b99abb5c95a7db2cd98",
    "base-arachne-on": "c9fee59a234212e7868bc600572213c47d6961de1f8963bfe1a34a25cf1f7d06",
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
    ap.add_argument(
        "--cells",
        nargs="*",
        default=["benchy-classic-on", "base-classic-on"],
    )
    ap.add_argument("--passes", type=int, default=3)
    ap.add_argument("--threads", type=int, default=12)
    ap.add_argument("--tag", default="campaign")
    args = ap.parse_args()

    root = args.repo_root.resolve()
    run_dir = root / "target" / "t136" / args.tag
    gcode_dir = run_dir / "gcode"
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

    rows_path = run_dir / "rows.csv"
    with rows_path.open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(
            [
                "cell",
                "pass",
                "wall_seconds",
                "gcode_bytes",
                "sha256",
                "exit_code",
                "probe_env",
                "threads",
                "stderr",
            ]
        )
        for pass_no in range(1, args.passes + 1):
            # Alternate the cell order each pass so a warming box does not
            # favour one model.
            cells = args.cells if pass_no % 2 == 1 else list(reversed(args.cells))
            for cell in cells:
                model_rel, cfg_rel = CELLS[cell]
                model = root / model_rel
                cfg = config_dir / cfg_rel
                gcode = gcode_dir / f"{cell}-p{pass_no}.gcode"
                stderr_name = f"{cell}-p{pass_no}.stderr.jsonl"
                stderr = run_dir / stderr_name
                env = dict(os.environ)
                env["PNP_T136_PROBE"] = "1"
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
                print(f"[t136] START {cell} p{pass_no}", flush=True)
                t0 = time.perf_counter()
                with stderr.open("wb") as err:
                    proc = subprocess.run(
                        cmd, stdout=subprocess.DEVNULL, stderr=err, env=env
                    )
                wall = time.perf_counter() - t0
                digest = sha256(gcode) if gcode.exists() else ""
                size = gcode.stat().st_size if gcode.exists() else 0
                print(
                    f"[t136] DONE  {cell} p{pass_no} wall={wall:.2f}s bytes={size} "
                    f"sha={digest[:12]} exit={proc.returncode}",
                    flush=True,
                )
                w.writerow(
                    [
                        cell,
                        pass_no,
                        f"{wall:.4f}",
                        size,
                        digest,
                        proc.returncode,
                        "PNP_T136_PROBE=1",
                        args.threads,
                        stderr_name,
                    ]
                )
                f.flush()
                if proc.returncode != 0:
                    print(f"[t136] NONZERO EXIT {cell}", file=sys.stderr)
                    return 1
                if digest != EXPECTED[cell]:
                    print(
                        f"[t136] HASH MISMATCH {cell}: {digest} != {EXPECTED[cell]}",
                        file=sys.stderr,
                    )
                    return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

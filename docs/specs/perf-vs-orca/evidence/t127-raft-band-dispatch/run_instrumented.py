#!/usr/bin/env python3
"""Ticket 127 instrumented probe -- one candidate pass per matched cell.

Measures the raft-default dispatch decline directly: the committed campaign
runs emit no `--instrument-stderr` stream, so this probe runs the candidate
ordinary arm once per cell and reduces the per-module event counts. It is
attribution only: no wall or CPU claim is taken from an instrumented pass.
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
ARMS = {
    "baseline": REPO / "target" / "t127-baseline" / "ordinary",
    "candidate": REPO / "target" / "t127-candidate" / "ordinary",
}
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {"benchy": REPO / "tmp" / "3dbenchy.stl", "base": REPO / "tmp" / "base.stl"}
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


def events(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return rows


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cells", default=",".join(CELLS))
    ap.add_argument("--arm", default="candidate", choices=sorted(ARMS))
    ap.add_argument("--out-dir", default=None)
    args = ap.parse_args()
    arm = ARMS[args.arm]
    exe = arm / "pnp_cli.exe"
    modules = arm / "modules"
    out_default = REPO / "target" / "t127" / ("instrumented" if args.arm == "candidate" else "instrumented-baseline")

    out_dir = Path(args.out_dir) if args.out_dir else out_default
    out_dir.mkdir(parents=True, exist_ok=True)
    rows = []
    for cell in [c.strip() for c in args.cells.split(",") if c.strip()]:
        fixture, generator, supports = cell.split("-", 2)
        raw = out_dir / f"{cell}.jsonl"
        gcode = out_dir / f"{cell}.gcode"
        command = [
            str(exe),
            "slice",
            "--model",
            str(FIXTURES[fixture]),
            "--output",
            str(gcode),
            "--config",
            str(CONFIGS / f"pnp-{generator}-supports-{supports}.json"),
            "--module-dir",
            str(modules),
            "--no-default-module-paths",
            "--instrument-stderr",
        ]
        env = dict(os.environ, RAYON_NUM_THREADS="12")
        started = time.perf_counter()
        with raw.open("wb") as errf:
            rc = subprocess.run(
                command, stdout=subprocess.DEVNULL, stderr=errf, env=env
            ).returncode
        wall = time.perf_counter() - started
        stream = events(raw)
        layers = sum(1 for event in stream if event.get("event") == "layer_start")
        completes = [
            event
            for event in stream
            if event.get("event") == "module_complete"
            and event.get("phase") == "per_layer"
        ]
        by_module: dict[str, list[float]] = {}
        by_stage: dict[str, set[str]] = {}
        for event in completes:
            module = event.get("module_id")
            if not module:
                continue
            by_module.setdefault(module, []).append(float(event.get("elapsed_ms", 0)))
            by_stage.setdefault(event.get("stage", ""), set()).add(module)
        raft = by_module.get("com.core.raft-default", [])
        infill = [
            len(by_module[module])
            for module in by_stage.get("Layer::Infill", set())
            if module != "com.core.raft-default"
        ]
        row = {
            "cell": cell,
            "exit_code": rc,
            "instrumented_wall_seconds": round(wall, 3),
            "layers": layers,
            "raft_default_calls": len(raft),
            "raft_default_thread_ms": round(sum(raft), 1),
            "infill_max_calls": max(infill) if infill else 0,
            "infill_modules_observed": sorted(
                module
                for module in by_stage.get("Layer::Infill", set())
                if module != "com.core.raft-default"
            ),
            "gcode_sha256": sha256(gcode) if gcode.exists() else "",
        }
        rows.append(row)
        print(
            f"{cell}: exit={rc} layers={layers} raft_calls={row['raft_default_calls']} "
            f"infill_max_calls={row['infill_max_calls']} sha={row['gcode_sha256'][:12]}",
            flush=True,
        )
    (out_dir / "rows.json").write_text(
        json.dumps(rows, indent=1) + "\n", encoding="utf-8"
    )
    return 0 if all(row["exit_code"] == 0 for row in rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())

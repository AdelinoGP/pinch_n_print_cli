#!/usr/bin/env python3
"""t180 ShellClassification post-t172 residual attribution probe runner.

Runs `pnp_cli.exe` with `PNP_T180_PROBE_OUT` set (no `--instrument-stderr` on
timing passes to avoid stderr observer distortion) on the matched-job cells,
verifies every output SHA-256 and byte count against `t175/reference.json`,
and also runs qualification `--instrument-stderr` passes to verify clean
`degraded=false`, `fatal=0`, `non_fatal=0`, `code_1200=0`.

Usage:
  python run_probe.py
  python run_probe.py --cells benchy-classic-off,base-classic-off --passes 1
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import statistics
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
REFERENCE = HERE.parent / "t175-tiled-collision-carve" / "reference.json"

DEFAULT_PRIMARY = ["benchy-classic-off", "benchy-classic-on", "base-classic-off"]
DEFAULT_INVARIANCE = ["benchy-arachne-off", "base-classic-on", "base-arachne-off"]


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def inspect_jsonl(raw: Path) -> dict:
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    slice_complete_ms = None
    shell_stage_ms = None
    shell_start = None
    with raw.open("r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if '"code":1200' in line or '"code": 1200' in line or "exact-Z occupancy" in line:
                code_1200 += 1
            s = line.strip()
            if not s.startswith("{"):
                continue
            try:
                obj = json.loads(s)
            except json.JSONDecodeError:
                continue
            ev = obj.get("event")
            if ev == "slice_complete":
                degraded = obj.get("degraded")
                fatal = obj.get("fatal_error_count")
                non_fatal = obj.get("non_fatal_error_count")
                slice_complete_ms = obj.get("elapsed_ms")
            elif ev == "stage_start" and obj.get("stage") == "PrePass::ShellClassification":
                shell_start = obj.get("elapsed_ms")
            elif ev == "stage_complete" and obj.get("stage") == "PrePass::ShellClassification":
                if obj.get("duration_ms") is not None:
                    shell_stage_ms = obj.get("duration_ms")
                elif shell_start is not None and obj.get("elapsed_ms") is not None:
                    shell_stage_ms = obj.get("elapsed_ms") - shell_start
    return {
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
        "slice_complete_ms": slice_complete_ms,
        "shell_stage_ms": shell_stage_ms,
    }


def run_one(cell: str, tag: str, out_dir: Path, refs: dict, ref_bytes: dict, instrument: bool) -> dict:
    fixture, generator, supports = cell.split("-", 2)
    cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
    gcode = out_dir / f"{cell}.{tag}.gcode"
    probe_json = out_dir / f"{cell}.{tag}.probe.json"
    stderr_log = out_dir / f"{cell}.{tag}.stderr.jsonl"
    if probe_json.exists():
        probe_json.unlink()

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
    ]
    if instrument:
        args.append("--instrument-stderr")

    env = dict(os.environ, RAYON_NUM_THREADS="12", PNP_T180_PROBE_OUT=str(probe_json))
    t0 = time.perf_counter()
    if instrument:
        with stderr_log.open("wb") as errf:
            rc = subprocess.run(args, stdout=subprocess.DEVNULL, stderr=errf, env=env).returncode
    else:
        rc = subprocess.run(args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env).returncode
    wall_s = time.perf_counter() - t0

    got_sha = sha256_file(gcode) if gcode.exists() else ""
    got_bytes = gcode.stat().st_size if gcode.exists() else 0
    ok_ref = (got_sha == refs.get(cell)) and (got_bytes == ref_bytes.get(cell))
    probe_data = json.loads(probe_json.read_text(encoding="utf-8")) if probe_json.exists() else None
    ev = inspect_jsonl(stderr_log) if instrument and stderr_log.exists() else None

    stage_ms = probe_data["counters"]["stage_wall_ns"] / 1e6 if probe_data else None
    print(
        f"{cell:20s} {tag:8s} rc={rc} wall={wall_s:6.3f}s stage={stage_ms or 0:7.1f}ms "
        f"ref={ok_ref} sha={got_sha[:10]}"
        + (f" degraded={ev['degraded']} c1200={ev['code_1200_count']} inst_stage={ev['shell_stage_ms']}ms" if ev else ""),
        flush=True,
    )
    return {
        "cell": cell,
        "tag": tag,
        "instrumented": instrument,
        "exit_code": rc,
        "wall_seconds": round(wall_s, 4),
        "stage_wall_ms": round(stage_ms, 3) if stage_ms is not None else None,
        "gcode_sha256": got_sha,
        "gcode_bytes": got_bytes,
        "reference_match": ok_ref,
        "instrument_events": ev,
        "probe_file": probe_json.name if probe_data else None,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--primary", default=",".join(DEFAULT_PRIMARY))
    ap.add_argument("--invariance", default=",".join(DEFAULT_INVARIANCE))
    ap.add_argument("--passes", type=int, default=3)
    ap.add_argument("--out-dir", default=str(REPO / "target" / "t180-probe"))
    ap.add_argument("--skip-qual", action="store_true")
    a = ap.parse_args()

    ref_doc = json.loads(REFERENCE.read_text(encoding="utf-8"))
    refs = ref_doc["active_set"]
    ref_bytes = ref_doc["bytes"]

    out_dir = Path(a.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    runs = []
    failures = 0

    primary_cells = [c.strip() for c in a.primary.split(",") if c.strip()]
    inv_cells = [c.strip() for c in a.invariance.split(",") if c.strip()]

    for cell in primary_cells:
        for p in range(1, a.passes + 1):
            r = run_one(cell, f"pass{p}", out_dir, refs, ref_bytes, instrument=False)
            runs.append(r)
            if r["exit_code"] != 0 or not r["reference_match"]:
                failures += 1
        if not a.skip_qual:
            r = run_one(cell, "qual", out_dir, refs, ref_bytes, instrument=True)
            runs.append(r)
            ev = r["instrument_events"] or {}
            if (
                r["exit_code"] != 0
                or not r["reference_match"]
                or ev.get("degraded") is not False
                or ev.get("fatal_error_count") != 0
                or ev.get("non_fatal_error_count") != 0
                or ev.get("code_1200_count") != 0
            ):
                failures += 1

    for cell in inv_cells:
        r = run_one(cell, "inv1", out_dir, refs, ref_bytes, instrument=False)
        runs.append(r)
        if r["exit_code"] != 0 or not r["reference_match"]:
            failures += 1

    summary = {
        "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "exe_sha256": sha256_file(EXE),
        "rayon_num_threads": 12,
        "runs": runs,
    }
    (out_dir / "probe_runs.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())

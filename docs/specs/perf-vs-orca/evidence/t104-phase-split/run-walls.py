#!/usr/bin/env python3
"""t104 phase-split take -- uninstrumented wall runner.

Measures process creation-to-exit wall clock (perf_counter) and process CPU
(GetProcessTimes, kernel+user) for one cell, 1 warmup + N measured runs, in the
matched-pair rig's invocation shape (RAYON_NUM_THREADS=12, same config, same
module dir). Wall rows are the only wall claims; CPU is recorded as
corroboration and never gated here.

Usage:
  python run-walls.py --cells benchy-classic-off,base-classic-off [--runs 3]
"""

from __future__ import annotations

import argparse
import csv
import ctypes
import ctypes.wintypes as wt
import hashlib
import os
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[5]
EXE = REPO / "target" / "release" / "pnp_cli.exe"
MODULES = REPO / "modules" / "core-modules"
CONFIGS = (
    REPO / "docs" / "specs" / "perf-vs-orca" / "evidence" / "matched-pair" / "configs"
)
FIXTURES = {"benchy": REPO / "tmp" / "3dbenchy.stl", "base": REPO / "tmp" / "base.stl"}

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", wt.DWORD), ("dwHighDateTime", wt.DWORD)]


def _ft(v: FILETIME) -> int:
    return (v.dwHighDateTime << 32) | v.dwLowDateTime


def process_cpu_s(pid: int) -> float | None:
    """kernel+user CPU seconds for a (just-exited) process, via GetProcessTimes."""
    h = k32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
    if not h:
        return None
    try:
        created, exited, kern, user = FILETIME(), FILETIME(), FILETIME(), FILETIME()
        ok = k32.GetProcessTimes(
            h,
            ctypes.byref(created),
            ctypes.byref(exited),
            ctypes.byref(kern),
            ctypes.byref(user),
        )
        if not ok:
            return None
        return (_ft(kern) + _ft(user)) / 1e7
    finally:
        k32.CloseHandle(h)


def system_cpu_window(seconds: float = 1.0) -> float:
    """Machine-wide busy fraction over a short window (GetSystemTimes)."""
    idle, kern, user = FILETIME(), FILETIME(), FILETIME()
    k32.GetSystemTimes(ctypes.byref(idle), ctypes.byref(kern), ctypes.byref(user))
    i0, k0, u0 = _ft(idle), _ft(kern), _ft(user)
    time.sleep(seconds)
    k32.GetSystemTimes(ctypes.byref(idle), ctypes.byref(kern), ctypes.byref(user))
    i1, k1, u1 = _ft(idle), _ft(kern), _ft(user)
    total = (k1 - k0) + (u1 - u0)
    if total <= 0:
        return float("nan")
    return 100.0 * (1.0 - (i1 - i0) / total)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def split_cell(cell: str) -> tuple[str, str, str]:
    fixture, generator, supports = cell.split("-", 2)
    return fixture, generator, supports


def run_once(cell: str, out_dir: Path, warmup: bool, index: int) -> dict:
    fixture, generator, supports = split_cell(cell)
    cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
    out = out_dir / f"{cell}-r{index}{'-warmup' if warmup else ''}.gcode"
    args = [
        str(EXE),
        "slice",
        "--model",
        str(FIXTURES[fixture]),
        "--output",
        str(out),
        "--config",
        str(cfg),
        "--module-dir",
        str(MODULES),
        "--no-progress-events",
    ]
    env = dict(os.environ, RAYON_NUM_THREADS="12")
    t0 = time.perf_counter()
    proc = subprocess.Popen(
        args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env
    )
    proc.wait()
    wall = time.perf_counter() - t0
    cpu = process_cpu_s(proc.pid)
    return {
        "cell": cell,
        "run_index": index,
        "warmup": "true" if warmup else "false",
        "wall_s": f"{wall:.3f}",
        "cpu_s": f"{cpu:.3f}" if cpu is not None else "",
        "exit_code": proc.returncode,
        "gcode_bytes": out.stat().st_size if out.exists() else 0,
        "sha256": sha256(out) if out.exists() else "",
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cells", required=True)
    ap.add_argument("--runs", type=int, default=3)
    ap.add_argument("--out", default=str(Path(__file__).with_name("walls.csv")))
    ap.add_argument("--gcode-dir", default=str(REPO / "target" / "t104-gcode"))
    a = ap.parse_args()

    cells = [c.strip() for c in a.cells.split(",") if c.strip()]
    out_dir = Path(a.gcode_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    rows: list[dict] = []
    load_before = system_cpu_window()
    for cell in cells:
        r = run_once(cell, out_dir, True, 0)
        rows.append(r)
        print(f"{cell} warmup: {r['wall_s']}s cpu={r['cpu_s']}", flush=True)
        if r["exit_code"] != 0:
            print("warmup failed", file=sys.stderr)
            return 1
        for i in range(1, a.runs + 1):
            r = run_once(cell, out_dir, False, i)
            rows.append(r)
            print(
                f"{cell} r{i} measured: {r['wall_s']}s cpu={r['cpu_s']} "
                f"exit={r['exit_code']} bytes={r['gcode_bytes']}",
                flush=True,
            )
            if r["exit_code"] != 0:
                print("run failed", file=sys.stderr)
                return 1
    load_after = system_cpu_window()
    with open(a.out, "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    with open(Path(a.out).with_name("load.txt"), "w", encoding="utf-8") as f:
        f.write(
            f"load_before_percent {load_before:.1f}\nload_after_percent {load_after:.1f}\n"
        )
    print(
        f"wrote {a.out} ({len(rows)} rows); load {load_before:.1f}% -> {load_after:.1f}%"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

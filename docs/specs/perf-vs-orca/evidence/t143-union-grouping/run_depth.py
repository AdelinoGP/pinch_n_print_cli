#!/usr/bin/env python3
"""t143 buffering-depth probe driver.

Runs one matched cell on the probe build (t136 instrumentation applied) at the
planner's current `EMIT_BATCH_LAYERS` depth and records:

  - process wall and CPU,
  - peak working set (Windows `PROCESS_MEMORY_COUNTERS.PeakWorkingSetSize`),
  - every `t136-segment name=... wall_sum_ns=...` line, so `union_run`'s host
    wall and the clip core's guest span are re-derivable,
  - the G-code hash, checked against the active post-t139 reference set.

The probe is attribution-only; the walls recorded here are never claimed as
benchmark cells.
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from ctypes import wintypes

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

# t139's base reference set. The four benchy hashes of that set are retired by
# the 2026-11-12 fixture swap; benchy cells read `reference.json`'s
# `benchy_replacement_fixture` entry instead (never t139).
T139_BASE = {
    "base-classic-off": "ca14d0554e37eddf27c80b5052b583d6fc6eba5cf7bf551d6bb419fd45a78769",
    "base-classic-on": "585c6a8edc2fd1de72dd2a86933fdad25c7ed144662dd80bbc68fb7856e4f629",
    "base-arachne-off": "d8fdc1d990b7763dd592311c333e1f2b0cd7f7beb9885b99abb5c95a7db2cd98",
    "base-arachne-on": "c9fee59a234212e7868bc600572213c47d6961de1f8963bfe1a34a25cf1f7d06",
}


def expected_hash(root: Path, cell: str) -> str | None:
    """t139's hash for a base cell; the replacement-fixture baseline for a benchy cell.

    The 2026-11-12 benchy export is a different tessellation of the same model,
    so t139's four benchy hashes are retired. `reference.json` records the
    baseline arm on the replacement fixture; a `null` means "record it first".
    """
    reference_path = Path(__file__).resolve().parent / "reference.json"
    reference = json.loads(reference_path.read_text(encoding="utf-8"))
    return reference["t139_base"].get(cell) or reference[
        "benchy_replacement_fixture"
    ].get(cell)


SEGMENT_RE = re.compile(
    r"t136-segment name=(?P<name>\S+) calls=(?P<calls>\d+) requests=(?P<requests>\d+) "
    r"vertices=(?P<vertices>\d+) wall_sum_ns=(?P<wall>\d+) thread_ns=(?P<thread>\d+) "
    r"max_ns=(?P<max_ns>\d+) ideal_ns=(?P<ideal>\d+) guest_wall_ns=(?P<guest>\d+)"
)


class PMC(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("PageFaultCount", wintypes.DWORD),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
    ]


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", wintypes.DWORD), ("dwHighDateTime", wintypes.DWORD)]


PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
PROCESS_QUERY_INFORMATION = 0x0400
PROCESS_VM_READ = 0x0010


def _open(pid: int):
    kernel32 = ctypes.windll.kernel32
    handle = kernel32.OpenProcess(
        PROCESS_QUERY_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
        False,
        pid,
    )
    return handle or None


def peak_working_set(pid: int) -> int | None:
    kernel32 = ctypes.windll.kernel32
    psapi = ctypes.windll.psapi
    handle = _open(pid)
    if not handle:
        return None
    try:
        counters = PMC()
        counters.cb = ctypes.sizeof(PMC)
        ok = psapi.GetProcessMemoryInfo(
            handle, ctypes.byref(counters), ctypes.sizeof(counters)
        )
        if not ok:
            return None
        return int(counters.PeakWorkingSetSize)
    finally:
        kernel32.CloseHandle(handle)


def process_cpu_seconds(pid: int) -> float | None:
    kernel32 = ctypes.windll.kernel32
    handle = _open(pid)
    if not handle:
        return None
    try:
        created, exited, kernel, user = FILETIME(), FILETIME(), FILETIME(), FILETIME()
        if not kernel32.GetProcessTimes(
            handle,
            ctypes.byref(created),
            ctypes.byref(exited),
            ctypes.byref(kernel),
            ctypes.byref(user),
        ):
            return None

        def to_ns(ft: FILETIME) -> int:
            return (ft.dwHighDateTime << 32) | ft.dwLowDateTime

        return (to_ns(kernel) + to_ns(user)) / 1e7
    finally:
        kernel32.CloseHandle(handle)


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
    ap.add_argument("--cell", required=True, choices=sorted(CELLS))
    ap.add_argument(
        "--depth", type=int, required=True, help="EMIT_BATCH_LAYERS of the probe build"
    )
    ap.add_argument("--tag", default="depth")
    ap.add_argument("--threads", type=int, default=12)
    ap.add_argument("--exe", type=Path, default=None)
    ap.add_argument("--module-dir", type=Path, default=None)
    args = ap.parse_args()

    root = args.repo_root.resolve()
    run_dir = root / "target" / "t143" / args.tag
    gcode_dir = run_dir / "gcode"
    gcode_dir.mkdir(parents=True, exist_ok=True)
    exe = args.exe or (root / "target" / "release" / "pnp_cli.exe")
    module_dir = args.module_dir or (root / "modules" / "core-modules")
    config_dir = root / "docs/specs/perf-vs-orca/evidence/matched-pair/configs"
    fixture, config = CELLS[args.cell]
    out = gcode_dir / f"{args.cell}-d{args.depth}.gcode"
    stderr_path = run_dir / f"{args.cell}-d{args.depth}.stderr.log"

    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = str(args.threads)
    env["PNP_T136_PROBE"] = "1"

    cmd = [
        str(exe),
        "slice",
        "--model",
        str(root / fixture),
        "--output",
        str(out),
        "--config",
        str(config_dir / config),
        "--module-dir",
        str(module_dir),
        "--no-default-module-paths",
        "--instrument-stderr",
    ]

    start = time.perf_counter()
    launch_epoch_ms = time.time() * 1000.0
    with stderr_path.open("w", encoding="utf-8", errors="replace") as err:
        proc = subprocess.Popen(
            cmd, cwd=str(root), env=env, stdout=subprocess.DEVNULL, stderr=err
        )
        peak = 0
        while proc.poll() is None:
            value = peak_working_set(proc.pid)
            if value is not None:
                peak = max(peak, value)
            time.sleep(0.05)
        end = time.perf_counter()
    final_peak = peak_working_set(proc.pid)
    if final_peak is not None:
        peak = max(peak, final_peak)
    wall = end - start
    cpu = process_cpu_seconds(proc.pid)

    digest = sha256(out) if out.exists() else ""
    segments = {}
    first_per_layer_epoch_ms = None
    first_support_epoch_ms = None
    for line in stderr_path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = SEGMENT_RE.search(line)
        if match:
            name = match.group("name")
            entry = segments.setdefault(
                name,
                {
                    "calls": 0,
                    "requests": 0,
                    "vertices": 0,
                    "wall_sum_ns": 0,
                    "thread_ns": 0,
                    "guest_wall_ns": 0,
                },
            )
            entry["calls"] += int(match.group("calls"))
            entry["requests"] += int(match.group("requests"))
            entry["vertices"] += int(match.group("vertices"))
            entry["wall_sum_ns"] += int(match.group("wall"))
            entry["thread_ns"] += int(match.group("thread"))
            entry["guest_wall_ns"] += int(match.group("guest"))
            continue
        if '"event":"stage_start"' not in line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        ts = event.get("timestamp_ms")
        if ts is None:
            continue
        if event.get("phase") == "per_layer":
            if first_per_layer_epoch_ms is None or ts < first_per_layer_epoch_ms:
                first_per_layer_epoch_ms = ts
        if event.get("stage") == "Layer::Support":
            if first_support_epoch_ms is None or ts < first_support_epoch_ms:
                first_support_epoch_ms = ts

    clip_core_ns = sum(seg["wall_sum_ns"] for seg in segments.values())
    union = segments.get("union_run", {})
    expected = expected_hash(root, args.cell)
    if expected is None:
        print(
            f"[t143-depth] no reference recorded for {args.cell}; record the baseline arm first",
            file=sys.stderr,
        )
        return 2
    row = {
        "cell": args.cell,
        "depth": args.depth,
        "wall_seconds": round(wall, 4),
        "peak_working_set_bytes": peak,
        "gcode_bytes": out.stat().st_size if out.exists() else 0,
        "sha256": digest,
        "exit_code": proc.returncode,
        "reference_match": digest == expected,
        "clip_core_wall_ms": round(clip_core_ns / 1e6, 3),
        "union": {
            k: union.get(k, 0)
            for k in ("calls", "requests", "wall_sum_ns", "thread_ns")
        },
        "segments": segments,
        "stderr": str(stderr_path.relative_to(root)),
        "cpu_seconds": round(cpu, 4) if cpu is not None else None,
        "first_per_layer_ms": (
            round(first_per_layer_epoch_ms - launch_epoch_ms, 1)
            if first_per_layer_epoch_ms is not None
            else None
        ),
        "first_support_layer_ms": (
            round(first_support_epoch_ms - launch_epoch_ms, 1)
            if first_support_epoch_ms is not None
            else None
        ),
    }
    rows_path = run_dir / "rows.jsonl"
    with rows_path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(row) + "\n")

    print(
        f"[t143-depth] {args.cell} d{args.depth} wall={wall:.3f}s cpu={cpu if cpu is not None else -1:.1f}s "
        f"peak_rss={peak / (1 << 20):.1f}MiB clip_core={row['clip_core_wall_ms']:.1f}ms "
        f"union_wall={(union.get('wall_sum_ns', 0)) / 1e6:.1f}ms "
        f"union_reqs={union.get('requests', 0)} first_layer={row['first_per_layer_ms']}ms "
        f"sha={digest[:12]} ref={row['reference_match']}"
    )
    if proc.returncode != 0 or not row["reference_match"]:
        print(
            f"[t143-depth] FAIL exit={proc.returncode} reference_match={row['reference_match']}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

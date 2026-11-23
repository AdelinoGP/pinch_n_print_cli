#!/usr/bin/env python3
"""t110 controls: is the import term real, and what does the tap itself cost?

  A. trivial-feature arm — slice the same model with every per-layer feature turned
     down (1 wall loop, 1 top/bottom shell, 0 % infill, no support), tapped like a
     campaign run. If the cell is import-bound, this arm lands close to the full cell
     and its prepass+emit collapses; that is the corroboration for treating
     `--info` as the import proxy.
  B. tap cost — run the tap against a 5 s no-op child and read the tap process's own
     CPU, so the poller's price is disclosed rather than assumed.
  C. (failed, recorded) binary-STL prefix linearity — `--info` on truncated prefixes
     of base.stl. A truncated binary STL is an open mesh, and Orca's repair path does
     not return on it (killed at 300 s on the 25 % / 615k-facet prefix). Abandoned;
     the failure is recorded in controls.json rather than silently dropped.

Usage: python controls.py --out controls.json
"""

import ctypes
import ctypes.wintypes as wt
import json
import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import run_campaign as rc  # noqa: E402

PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
TRIVIAL_KEYS = [
    "--sparse-infill-density=0%",
    "--wall-loops=1",
    "--top-shell-layers=1",
    "--bottom-shell-layers=1",
]


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", wt.DWORD), ("dwHighDateTime", wt.DWORD)]


def _ft(ft):
    return ((ft.dwHighDateTime << 32) + ft.dwLowDateTime) / 10_000_000.0


def cpu_of(pid):
    h = ctypes.windll.kernel32.OpenProcess(
        PROCESS_QUERY_LIMITED_INFORMATION, False, pid
    )
    if not h:
        return None
    try:
        c, e, k, u = FILETIME(), FILETIME(), FILETIME(), FILETIME()
        if not ctypes.windll.kernel32.GetProcessTimes(
            h, ctypes.byref(c), ctypes.byref(e), ctypes.byref(k), ctypes.byref(u)
        ):
            return None
        return _ft(k) + _ft(u)
    finally:
        ctypes.windll.kernel32.CloseHandle(h)


def trivial_run(rawdir, model):
    """Trivial-feature slice, tapped for the same boundaries as a campaign run."""
    outdir = os.path.join(rawdir, "trivial-" + model)
    os.makedirs(outdir, exist_ok=True)
    tmp = os.path.join(outdir, "plate_1.gcode.tmp")
    final = os.path.join(outdir, "plate_1.gcode")
    for p in (tmp, final):
        if os.path.exists(p):
            os.remove(p)
    cell = {
        "cell": model + "-trivial",
        "model": model,
        "generator": "classic",
        "support": "0",
        "style": None,
    }
    cmd = rc.build_cmd(cell, outdir, os.path.join(outdir, "orca-cli.log"))
    at = cmd.index("--debug")
    cmd = cmd[:at] + TRIVIAL_KEYS + cmd[at:]

    t0 = time.perf_counter_ns()
    p = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    t_tmp = t_ren = None
    while True:
        now = (time.perf_counter_ns() - t0) / 1e6
        if t_tmp is None and os.path.exists(tmp):
            t_tmp = now
        if t_tmp is not None and t_ren is None and os.path.exists(final):
            t_ren = now
        if p.poll() is not None:
            break
        time.sleep(0.002)
    wall = (time.perf_counter_ns() - t0) / 1e6
    return {
        "model": model,
        "wall_ms": round(wall, 1),
        "prephase_ms": None if t_tmp is None else round(t_tmp, 1),
        "emit_ms": None
        if (t_tmp is None or t_ren is None)
        else round(t_ren - t_tmp, 1),
        "tail_ms": None if t_ren is None else round(wall - t_ren, 1),
        "gcode_bytes": os.path.getsize(final) if os.path.exists(final) else 0,
        "args": TRIVIAL_KEYS,
    }


def tap_cost():
    child = ["cmd", "/c", "ping", "-n", "5", "127.0.0.1"]
    t0 = time.perf_counter_ns()
    before = cpu_of(os.getpid())
    p = subprocess.Popen(child, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    polls = 0
    while p.poll() is None:
        polls += 1
        time.sleep(0.002)
    after = cpu_of(os.getpid())
    wall = (time.perf_counter_ns() - t0) / 1e6
    return {
        "child_wall_ms": round(wall, 1),
        "polls": polls,
        "tap_cpu_s": None
        if (before is None or after is None)
        else round(after - before, 4),
        "tap_cpu_pct_of_core": None
        if (before is None or after is None)
        else round(100.0 * (after - before) / (wall / 1000.0), 2),
    }


def main():
    out = (
        sys.argv[sys.argv.index("--out") + 1]
        if "--out" in sys.argv
        else os.path.join(os.path.dirname(os.path.abspath(__file__)), "controls.json")
    )
    rawdir = os.path.join(rc.REPO, "target", "t110-controls")
    os.makedirs(rawdir, exist_ok=True)

    arms = []
    for model in ("base", "benchy"):
        arm = trivial_run(rawdir, model)
        arms.append(arm)
        print(
            f"trivial {model}: wall={arm['wall_ms']} pre={arm['prephase_ms']} "
            f"emit={arm['emit_ms']} bytes={arm['gcode_bytes']}",
            flush=True,
        )

    cost = tap_cost()
    print(f"tap cost: {cost}", flush=True)

    doc = {
        "note": (
            "t110 controls. A: trivial-feature slices of base and benchy, tapped for the same "
            "boundaries as a campaign run (every per-layer feature turned down). B: the tap's "
            "own CPU against a 5 s no-op child. C: the abandoned STL-prefix linearity test is "
            "recorded with its failure, because a truncated binary STL is a non-manifold mesh "
            "and Orca's repair path did not return on the 615k-facet prefix (killed at 300 s)."
        ),
        "trivial_arms": arms,
        "tap_cost": cost,
        "abandoned_import_linearity": {
            "attempt": "--info on binary-STL prefixes of base.stl at 25 / 50 / 100 % facets",
            "result": "hung on the 25 % prefix (615,308 facets); killed after 300 s",
            "reason": "a truncated binary STL is an open mesh, so TriangleMesh::repair() and the manifold check do not terminate in bounded time",
            "replaced_by": "the trivial-feature slice arm, which uses a closed mesh and the real slice code path",
        },
    }
    with open(out, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1)
    print(f"wrote {out}")


if __name__ == "__main__":
    main()

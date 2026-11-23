#!/usr/bin/env python3
"""t110 campaign runner: OrcaSlicer phase tap.

Measures three hard boundaries per Orca slice without any Orca cooperation:

  t_tmp     `<outdir>/plate_1.gcode.tmp` first appears  -> closes startup+vendor+import+prepass
  t_rename  `plate_1.gcode` appears                      -> closes per-layer paths + G-code emission
  t_exit    process exit                                 -> closes the teardown tail

Cumulative process CPU (GetProcessTimes) is read at every temp-file growth change,
so each phase also carries CPU seconds. An `--info` arm on the same model measures
startup + vendor + import with no slicing.

Rows go to runs.csv (in-repo evidence); per-run output/log dirs stay under
target/t110-runs (gitignored).

Usage:
  python run_campaign.py --repeats 3 --out <evidence>/runs.csv --rawdir target/t110-runs
"""

import argparse
import ctypes
import ctypes.wintypes as wt
import csv
import json
import os
import statistics
import subprocess
import sys
import time

PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
ORCA_EXE = r"C:\Program Files\OrcaSlicer\orca-slicer.exe"
POLL_S = 0.002

REPO = os.path.abspath(
    os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", "..")
)
VENDOR = os.path.join(REPO, "target", "orca-vendor")
MACHINE = os.path.join(VENDOR, "BBL", "machine", "Bambu Lab X1 Carbon 0.4 nozzle.json")
PROCESS = os.path.join(VENDOR, "BBL", "process", "0.20mm Standard @BBL X1C.json")
FILAMENT = os.path.join(VENDOR, "BBL", "filament", "Generic PLA.json")
MODELS = {
    "benchy": os.path.join(REPO, "tmp", "3dbenchy.stl"),
    "base": os.path.join(REPO, "tmp", "base.stl"),
    "calicat": os.path.join(REPO, "resources", "calicat.stl"),
}

# t103 matched keys + t105 pin on the supports-on cells.
MATCHED_KEYS = [
    "--wall-loops=2",
    "--sparse-infill-density=20%",
    "--sparse-infill-pattern=gyroid",
    "--layer-height=0.2",
    "--initial-layer-print-height=0.2",
    "--enable-arc-fitting=0",
    "--elefant-foot-compensation=0",
    "--brim-width=0",
]

CELLS = [
    {
        "cell": "benchy-classic-off",
        "model": "benchy",
        "generator": "classic",
        "support": "0",
        "style": None,
    },
    {
        "cell": "benchy-classic-on",
        "model": "benchy",
        "generator": "classic",
        "support": "1",
        "style": "tree_strong",
    },
    {
        "cell": "base-classic-off",
        "model": "base",
        "generator": "classic",
        "support": "0",
        "style": None,
    },
    {
        "cell": "base-classic-on",
        "model": "base",
        "generator": "classic",
        "support": "1",
        "style": "tree_strong",
    },
]


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", wt.DWORD), ("dwHighDateTime", wt.DWORD)]


def _ft(ft):
    return ((ft.dwHighDateTime << 32) + ft.dwLowDateTime) / 10_000_000.0


def proc_cpu(pid):
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


def sys_cpu_seconds():
    idle, kernel, user = FILETIME(), FILETIME(), FILETIME()
    if not ctypes.windll.kernel32.GetSystemTimes(
        ctypes.byref(idle), ctypes.byref(kernel), ctypes.byref(user)
    ):
        return None
    return _ft(idle), _ft(kernel), _ft(user)


def build_cmd(cell, outdir, logpath):
    model = MODELS[cell["model"]]
    args = [
        ORCA_EXE,
        "--slice",
        "0",
        "--load-settings=" + MACHINE + ";" + PROCESS,
        "--load-filaments=" + FILAMENT,
        "--wall-generator=" + cell["generator"],
        "--enable-support=" + cell["support"],
    ] + MATCHED_KEYS
    if cell.get("style") and cell["support"] == "1":
        args.append("--support-style=" + cell["style"])
    args += ["--debug", "3", "--logfile", logpath, "--outputdir", outdir, model]
    return args


def build_info_cmd(model, outdir, logpath):
    return [
        ORCA_EXE,
        "--info",
        "--load-settings=" + MACHINE + ";" + PROCESS,
        "--load-filaments=" + FILAMENT,
        "--debug",
        "3",
        "--logfile",
        logpath,
        "--outputdir",
        outdir,
        MODELS[model],
    ]


def run_slice(cell, run_id, rawdir):
    outdir = os.path.join(rawdir, run_id)
    os.makedirs(outdir, exist_ok=False)
    tmp = os.path.join(outdir, "plate_1.gcode.tmp")
    final = os.path.join(outdir, "plate_1.gcode")
    logpath = os.path.join(outdir, "orca-cli.log")
    cmd = build_cmd(cell, outdir, logpath)

    load0 = sys_cpu_seconds()
    t0 = time.perf_counter_ns()
    proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    t_tmp = t_rename = t_exit = None
    cpu_at_tmp = cpu_at_rename = cpu_final = None
    last_size = -1
    growth = 0
    t_poll_max_gap = 0.0
    last_loop = t0
    while True:
        now_ns = time.perf_counter_ns()
        gap = (now_ns - last_loop) / 1e6
        if gap > t_poll_max_gap:
            t_poll_max_gap = gap
        last_loop = now_ns
        elapsed = (now_ns - t0) / 1e6
        if t_tmp is None and os.path.exists(tmp):
            t_tmp = elapsed
            cpu_at_tmp = proc_cpu(proc.pid)
        size = os.path.getsize(tmp) if os.path.exists(tmp) else -1
        if size != last_size:
            growth += 1
            last_size = size
        if t_tmp is not None and t_rename is None and os.path.exists(final):
            t_rename = elapsed
            cpu_at_rename = proc_cpu(proc.pid)
        if proc.poll() is not None:
            t_exit = (time.perf_counter_ns() - t0) / 1e6
            cpu_final = proc_cpu(proc.pid)
            break
        time.sleep(POLL_S)
    load1 = sys_cpu_seconds()

    util = None
    if load0 and load1:
        di = load1[0] - load0[0]
        dt = (load1[1] - load0[1]) + (load1[2] - load0[2])
        if dt > 0:
            util = 1.0 - di / dt
    return {
        "run_id": run_id,
        "cell": cell["cell"],
        "kind": "slice",
        "model": cell["model"],
        "generator": cell["generator"],
        "support": cell["support"],
        "style": cell.get("style") or "",
        "exit_code": proc.returncode,
        "wall_ms": round(t_exit, 1),
        "t_tmp_ms": None if t_tmp is None else round(t_tmp, 1),
        "t_rename_ms": None if t_rename is None else round(t_rename, 1),
        "prephase_ms": None if t_tmp is None else round(t_tmp, 1),
        "emit_ms": None
        if (t_tmp is None or t_rename is None)
        else round(t_rename - t_tmp, 1),
        "tail_ms": None
        if (t_rename is None or t_exit is None)
        else round(t_exit - t_rename, 1),
        "cpu_at_tmp_s": None if cpu_at_tmp is None else round(cpu_at_tmp, 3),
        "cpu_at_rename_s": None if cpu_at_rename is None else round(cpu_at_rename, 3),
        "cpu_final_s": None if cpu_final is None else round(cpu_final, 3),
        "gcode_bytes": os.path.getsize(final) if os.path.exists(final) else 0,
        "growth_events": growth,
        "poll_max_gap_ms": round(t_poll_max_gap, 1),
        "sys_cpu_util": None if util is None else round(util, 4),
        "log_bytes": os.path.getsize(logpath) if os.path.exists(logpath) else 0,
    }


def run_info(model, run_id, rawdir):
    outdir = os.path.join(rawdir, run_id)
    os.makedirs(outdir, exist_ok=False)
    logpath = os.path.join(outdir, "orca-cli.log")
    cmd = build_info_cmd(model, outdir, logpath)
    load0 = sys_cpu_seconds()
    t0 = time.perf_counter_ns()
    proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    cpu_at_exit = None
    while True:
        if proc.poll() is not None:
            cpu_at_exit = proc_cpu(proc.pid)
            break
        time.sleep(POLL_S)
    t_exit = (time.perf_counter_ns() - t0) / 1e6
    load1 = sys_cpu_seconds()
    util = None
    if load0 and load1:
        di = load1[0] - load0[0]
        dt = (load1[1] - load0[1]) + (load1[2] - load0[2])
        if dt > 0:
            util = 1.0 - di / dt
    return {
        "run_id": run_id,
        "cell": "info-" + model,
        "kind": "info",
        "model": model,
        "generator": "",
        "support": "",
        "style": "",
        "exit_code": proc.returncode,
        "wall_ms": round(t_exit, 1),
        "t_tmp_ms": None,
        "t_rename_ms": None,
        "prephase_ms": None,
        "emit_ms": None,
        "tail_ms": None,
        "cpu_at_tmp_s": None,
        "cpu_at_rename_s": None,
        "cpu_final_s": None if cpu_at_exit is None else round(cpu_at_exit, 3),
        "gcode_bytes": 0,
        "growth_events": 0,
        "poll_max_gap_ms": 0.0,
        "sys_cpu_util": None if util is None else round(util, 4),
        "log_bytes": os.path.getsize(logpath) if os.path.exists(logpath) else 0,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repeats", type=int, default=3)
    ap.add_argument("--out", required=True)
    ap.add_argument("--rawdir", required=True)
    args = ap.parse_args()
    os.makedirs(args.rawdir, exist_ok=True)

    rows = []
    # --- info controls: two per model, interleaved before the slice rounds.
    for r in range(2):
        for model in ("calicat", "benchy", "base"):
            rid = f"info-{model}-r{r + 1}"
            row = run_info(model, rid, args.rawdir)
            rows.append(row)
            print(
                f"[info] {model} r{r + 1} wall={row['wall_ms']} ms util={row['sys_cpu_util']}",
                flush=True,
            )

    # --- one discarded warmup per cell, then `repeats` measured rounds, round-robin.
    for cell in CELLS:
        rid = f"{cell['cell']}-warmup"
        row = run_slice(cell, rid, args.rawdir)
        print(
            f"[warmup] {cell['cell']} wall={row['wall_ms']} ms pre={row['prephase_ms']} emit={row['emit_ms']}",
            flush=True,
        )

    for r in range(args.repeats):
        for cell in CELLS:
            rid = f"{cell['cell']}-m{r + 1}"
            row = run_slice(cell, rid, args.rawdir)
            rows.append(row)
            print(
                f"[slice] {cell['cell']} m{r + 1} wall={row['wall_ms']} ms "
                f"pre={row['prephase_ms']} emit={row['emit_ms']} tail={row['tail_ms']} util={row['sys_cpu_util']}",
                flush=True,
            )

    fields = list(rows[0].keys())
    with open(args.out, "w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=fields)
        w.writeheader()
        w.writerows(rows)
    print(f"\nwrote {len(rows)} rows -> {args.out}", flush=True)


if __name__ == "__main__":
    main()

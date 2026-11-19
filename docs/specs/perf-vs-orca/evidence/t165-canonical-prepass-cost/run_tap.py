#!/usr/bin/env python3
"""t165: re-run t110's Orca phase tap on the installed OrcaSlicer 2.5.1.

Differences from t110's run_campaign.py (each disclosed in FINDINGS.md):
  * the current matched job: the scoreboard rig's override set
    (`New-OrcaOverrideArgs` in matched-pair/run_scoreboard.ps1), with the
    resolved appendix asserted per run (the rig's `Test-OrcaEvidence` keys);
  * no `--debug` (the rig's default log level), `--logfile` beside the run;
  * timeBeginPeriod(1) so the poll gap is ~1-2 ms rather than ~15 ms;
  * a process handle held from launch so GetProcessTimes(creation, exit)
    gives the OS wall of every run, the 1 % accuracy control;
  * the executable SHA-256 is stamped on every row;
  * `--info` controls run inside each round (3 measured per model), not
    only before the slice rounds.

Usage:
  python run_tap.py --repeats 3 --out <evidence>/runs.csv --rawdir target/t165-runs
"""

import argparse
import csv
import ctypes
import ctypes.wintypes as wt
import hashlib
import os
import subprocess
import time

ORCA_EXE = r"C:\Program Files\OrcaSlicer\orca-slicer.exe"
POLL_S = 0.001
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
SYNCHRONIZE = 0x00100000

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), *[".."] * 5))
VENDOR = os.path.join(REPO, "target", "orca-vendor")
MACHINE = os.path.join(VENDOR, "BBL", "machine", "Bambu Lab X1 Carbon 0.4 nozzle.json")
PROCESS = os.path.join(VENDOR, "BBL", "process", "0.20mm Standard @BBL X1C.json")
FILAMENT = os.path.join(VENDOR, "BBL", "filament", "Generic PLA.json")
MODELS = {
    "benchy": os.path.join(REPO, "tmp", "3dbenchy.stl"),
    "base": os.path.join(REPO, "tmp", "base.stl"),
}

# run_scoreboard.ps1 `$Job` + `New-OrcaOverrideArgs`, classic generator.
JOB = {
    "wall_loops": "2",
    "sparse_infill_density": "20%",
    "sparse_infill_pattern": "gyroid",
    "layer_height": "0.2",
    "brim_width": "8",
    "brim_type": "outer_only",
    "skirt_loops": "6",
    "skirt_distance": "3",
    "skirt_height": "1",
    "initial_layer_line_width": "0.4",
    "tree_support_brim_width": "0",
    "support_type": "tree(auto)",
    "elefant_foot_compensation": "0",
}


def overrides(support):
    a = [
        "--wall-generator=classic",
        "--wall-loops=2",
        "--sparse-infill-density=20%",
        "--sparse-infill-pattern=gyroid",
        "--enable-support=" + support,
        "--layer-height=0.2",
        "--initial-layer-print-height=0.2",
        "--enable-arc-fitting=0",
        "--elefant-foot-compensation=0",
        "--brim-width=8",
        "--brim-type=outer_only",
        "--skirt-loops=6",
        "--skirt-distance=3",
        "--skirt-height=1",
        "--initial-layer-line-width=0.4",
        "--tree-support-brim-width=0",
    ]
    if support == "1":
        a.append("--support-style=tree_strong")
    return a


CLASSES = [
    {"cls": "benchy-off", "model": "benchy", "support": "0"},
    {"cls": "benchy-on", "model": "benchy", "support": "1"},
    {"cls": "base-off", "model": "base", "support": "0"},
    {"cls": "base-on", "model": "base", "support": "1"},
]


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", wt.DWORD), ("dwHighDateTime", wt.DWORD)]


def _ft(ft):
    return ((ft.dwHighDateTime << 32) + ft.dwLowDateTime) / 10_000_000.0


k32 = ctypes.windll.kernel32


def open_proc(pid):
    return k32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, False, pid)


def times(h):
    c, e, k, u = FILETIME(), FILETIME(), FILETIME(), FILETIME()
    if not k32.GetProcessTimes(h, ctypes.byref(c), ctypes.byref(e), ctypes.byref(k), ctypes.byref(u)):
        return None
    return _ft(c), _ft(e), _ft(k) + _ft(u)


def sys_times():
    i, k, u = FILETIME(), FILETIME(), FILETIME()
    k32.GetSystemTimes(ctypes.byref(i), ctypes.byref(k), ctypes.byref(u))
    return _ft(i), _ft(k), _ft(u)


def util(a, b):
    di = b[0] - a[0]
    dt = (b[1] - a[1]) + (b[2] - a[2])
    return None if dt <= 0 else round(1.0 - di / dt, 4)


def appendix(path):
    cfg, inside = {}, False
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if line.startswith("; CONFIG_BLOCK_START"):
                inside = True
                continue
            if line.startswith("; CONFIG_BLOCK_END"):
                break
            if inside and line.startswith("; ") and " = " in line:
                k, v = line[2:].rstrip("\n").split(" = ", 1)
                cfg[k.strip()] = v.strip()
    return cfg


def assert_job(cfg, support, run_id):
    if len(cfg) < 50:
        raise SystemExit(f"rejected {run_id}: appendix missing ({len(cfg)} keys)")
    want = dict(JOB, wall_generator="classic", enable_support=support,
                support_style="tree_strong" if support == "1" else "default")
    for k, v in want.items():
        if cfg.get(k) != v:
            raise SystemExit(f"rejected {run_id}: {k}={cfg.get(k)!r}, want {v!r}")
    if cfg.get("enable_arc_fitting") not in ("0", "disabled"):
        raise SystemExit(f"rejected {run_id}: enable_arc_fitting={cfg.get('enable_arc_fitting')!r}")
    for k, bad in (("filament_density", "0"), ("filament_max_volumetric_speed", "2"),
                   ("first_layer_temperature", "200")):
        if not cfg.get(k) or cfg.get(k) == bad:
            raise SystemExit(f"rejected {run_id}: {k}={cfg.get(k)!r} (t98 defect signature)")


def run(kind, spec, run_id, rawdir, exe_sha):
    outdir = os.path.join(rawdir, run_id)
    os.makedirs(outdir, exist_ok=False)
    log = os.path.join(outdir, "orca-cli.log")
    common = ["--load-settings=" + MACHINE + ";" + PROCESS, "--load-filaments=" + FILAMENT]
    if kind == "info":
        cmd = [ORCA_EXE, "--info"] + common + ["--logfile", log, "--outputdir", outdir, MODELS[spec]]
    else:
        cmd = ([ORCA_EXE, "--slice", "0"] + common + overrides(spec["support"])
               + ["--logfile", log, "--outputdir", outdir, MODELS[spec["model"]]])
    tmp = os.path.join(outdir, "plate_1.gcode.tmp")
    final = os.path.join(outdir, "plate_1.gcode")
    s0 = sys_times()
    t0 = time.perf_counter_ns()
    p = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    h = open_proc(p.pid)
    t_tmp = t_ren = None
    cpu_tmp = cpu_ren = None
    gap_max, last = 0.0, t0
    while True:
        now = time.perf_counter_ns()
        gap_max = max(gap_max, (now - last) / 1e6)
        last = now
        el = (now - t0) / 1e6
        if kind == "slice":
            if t_tmp is None and os.path.exists(tmp):
                t_tmp, cpu_tmp = el, times(h)[2]
            if t_tmp is not None and t_ren is None and os.path.exists(final):
                t_ren, cpu_ren = el, times(h)[2]
        if p.poll() is not None:
            t_exit = (time.perf_counter_ns() - t0) / 1e6
            break
        time.sleep(POLL_S)
    c, e, cpu = times(h)
    k32.CloseHandle(h)
    s1 = sys_times()
    os_wall_ms = (e - c) * 1000.0
    row = {
        "run_id": run_id, "kind": kind,
        "cls": ("info-" + spec) if kind == "info" else spec["cls"],
        "exe_sha256": exe_sha, "exit_code": p.returncode,
        "tap_wall_ms": round(t_exit, 1), "os_wall_ms": round(os_wall_ms, 1),
        "wall_agree_pct": round(100.0 * (t_exit - os_wall_ms) / os_wall_ms, 3),
        "t_tmp_ms": None if t_tmp is None else round(t_tmp, 1),
        "t_rename_ms": None if t_ren is None else round(t_ren, 1),
        "cpu_at_tmp_s": None if cpu_tmp is None else round(cpu_tmp, 3),
        "cpu_at_rename_s": None if cpu_ren is None else round(cpu_ren, 3),
        "cpu_final_s": round(cpu, 3),
        "gcode_bytes": os.path.getsize(final) if os.path.exists(final) else 0,
        "poll_max_gap_ms": round(gap_max, 2),
        "sys_cpu_util": util(s0, s1),
    }
    if p.returncode != 0:
        raise SystemExit(f"rejected {run_id}: exit {p.returncode}")
    if kind == "slice":
        if t_tmp is None or t_ren is None:
            raise SystemExit(f"rejected {run_id}: tap boundary missing (tmp={t_tmp}, rename={t_ren})")
        assert_job(appendix(final), spec["support"], run_id)
    return row


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repeats", type=int, default=3)
    ap.add_argument("--out", required=True)
    ap.add_argument("--rawdir", required=True)
    a = ap.parse_args()
    os.makedirs(a.rawdir, exist_ok=True)
    ctypes.windll.winmm.timeBeginPeriod(1)
    exe_sha = hashlib.sha256(open(ORCA_EXE, "rb").read()).hexdigest()
    print("orca-slicer.exe sha256", exe_sha, flush=True)
    rows = []
    try:
        for r in range(a.repeats + 1):  # round 0 = discarded warmup
            tag = "w" if r == 0 else f"m{r}"
            for model in ("benchy", "base"):
                row = run("info", model, f"info-{model}-{tag}", a.rawdir, exe_sha)
                row["measured"] = int(r > 0)
                rows.append(row)
                print(f"[{tag}] info-{model} wall={row['tap_wall_ms']} os={row['os_wall_ms']} util={row['sys_cpu_util']}", flush=True)
            for spec in CLASSES:
                row = run("slice", spec, f"{spec['cls']}-{tag}", a.rawdir, exe_sha)
                row["measured"] = int(r > 0)
                rows.append(row)
                print(f"[{tag}] {spec['cls']} wall={row['tap_wall_ms']} os={row['os_wall_ms']} tmp={row['t_tmp_ms']} "
                      f"ren={row['t_rename_ms']} gap={row['poll_max_gap_ms']} util={row['sys_cpu_util']}", flush=True)
    finally:
        ctypes.windll.winmm.timeEndPeriod(1)
    with open(a.out, "w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(f"wrote {len(rows)} rows -> {a.out}", flush=True)


if __name__ == "__main__":
    main()

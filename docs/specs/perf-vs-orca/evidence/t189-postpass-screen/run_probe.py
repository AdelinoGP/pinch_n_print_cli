#!/usr/bin/env python3
"""t189 observation-only PostPass substage attribution runner.

Runs 3 instrumented passes + 1 --profile pass on all 8 matrix cells using the
probe-built pnp_cli.exe (RAYON_NUM_THREADS=12, unsandboxed, T189_PROBE=1).
Verifies every single run is 100% byte-identical (SHA-256 + bytes) to t182/reference.json.
"""
from __future__ import annotations

import ctypes
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
EXE = REPO / "target" / "t189" / "probe" / "pnp_cli.exe"
MODS = REPO / "modules" / "core-modules"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {
    "benchy": REPO / "tmp" / "3dbenchy.stl",
    "base": REPO / "tmp" / "base.stl",
}
REFERENCE = HERE.parent / "t182-spatial-hierarchical-phase3-union" / "reference.json"
OUT_DIR = REPO / "target" / "t189" / "probe" / "runs"

CELLS = [
    ("benchy-classic-off", "benchy", "pnp-classic-supports-off.json"),
    ("benchy-classic-on", "benchy", "pnp-classic-supports-on.json"),
    ("benchy-arachne-off", "benchy", "pnp-arachne-supports-off.json"),
    ("benchy-arachne-on", "benchy", "pnp-arachne-supports-on.json"),
    ("base-classic-off", "base", "pnp-classic-supports-off.json"),
    ("base-classic-on", "base", "pnp-classic-supports-on.json"),
    ("base-arachne-off", "base", "pnp-arachne-supports-off.json"),
    ("base-arachne-on", "base", "pnp-arachne-supports-on.json"),
]


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", ctypes.c_uint32), ("dwHighDateTime", ctypes.c_uint32)]


def ft_to_sec(ft: FILETIME) -> float:
    return ((ft.dwHighDateTime << 32) | ft.dwLowDateTime) * 1e-7


def run_with_cpu(args: list[str], env: dict, stderr_path: Path) -> tuple[int, float, float]:
    t0 = time.perf_counter()
    with stderr_path.open("wb") as fh:
        proc = subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=fh, env=env)
        rc = proc.wait()
        wall = time.perf_counter() - t0
        cpu = 0.0
        if os.name == "nt":
            c, e, k, u = FILETIME(), FILETIME(), FILETIME(), FILETIME()
            if ctypes.windll.kernel32.GetProcessTimes(
                int(proc._handle),
                ctypes.byref(c),
                ctypes.byref(e),
                ctypes.byref(k),
                ctypes.byref(u),
            ):
                cpu = ft_to_sec(k) + ft_to_sec(u)
    return rc, wall, cpu


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_stderr(raw: Path) -> dict:
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    slice_complete_ms = None
    stage_starts: dict[str, int] = {}
    stage_ms: dict[str, int] = {}
    module_starts: dict[tuple[str, str], int] = {}
    module_ms: dict[str, int] = {}
    phase_starts: dict[str, int] = {}
    phase_ms: dict[str, int] = {}
    probes: list[dict] = []
    profile_summary = None

    with raw.open("r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if '"code":1200' in line or '"code": 1200' in line or "exact-Z occupancy" in line:
                code_1200 += 1
            s = line.strip()
            if s.startswith("T189_PROBE "):
                try:
                    probes.append(json.loads(s[len("T189_PROBE "):]))
                except json.JSONDecodeError:
                    pass
                continue
            if not s.startswith("{"):
                continue
            try:
                obj = json.loads(s)
            except json.JSONDecodeError:
                continue
            ev = obj.get("event")
            el = obj.get("elapsed_ms")
            if ev == "slice_complete":
                degraded = obj.get("degraded")
                fatal = obj.get("fatal_error_count")
                non_fatal = obj.get("non_fatal_error_count")
                slice_complete_ms = el
            elif ev == "phase_complete":
                ph = obj.get("phase")
                if ph is not None and el is not None:
                    phase_ms[ph] = el
            elif ev == "stage_complete":
                st = obj.get("stage") or obj.get("stage_id")
                if st and el is not None and str(st).startswith("PostPass::"):
                    stage_ms[st] = el
            elif ev == "module_complete":
                st = obj.get("stage") or obj.get("stage_id") or ""
                mod = obj.get("module_id") or obj.get("module")
                if str(st).startswith("PostPass::") and mod and el is not None:
                    module_ms[f"{st}/{mod}"] = el
            elif ev == "profile_summary":
                profile_summary = obj

    return {
        "degraded": degraded,
        "fatal": fatal,
        "non_fatal": non_fatal,
        "code_1200": code_1200,
        "slice_complete_ms": slice_complete_ms,
        "phase_ms": phase_ms,
        "postpass_stage_ms": stage_ms,
        "postpass_module_ms": module_ms,
        "probes": probes,
        "profile_summary": profile_summary,
    }


def main() -> None:
    if not EXE.exists():
        raise SystemExit(f"missing probe binary: {EXE}")
    ref = json.loads(REFERENCE.read_text(encoding="utf-8"))
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = "12"
    env["T189_PROBE"] = "1"

    out_summary: dict = {
        "timestamp_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "exe": str(EXE),
        "exe_sha256": sha256_file(EXE),
        "rayon_num_threads": 12,
        "cells": {},
    }

    for cell, model_key, cfg_name in CELLS:
        stl = FIXTURES[model_key]
        cfg = CONFIGS / cfg_name
        exp_sha = ref["active_set"][cell]
        exp_bytes = ref["bytes"][cell]
        passes = []

        for p_idx in range(1, 4):
            gcode = OUT_DIR / f"{cell}_p{p_idx}.gcode"
            stderr_log = OUT_DIR / f"{cell}_p{p_idx}.stderr.jsonl"
            args = [
                str(EXE),
                "slice",
                "--model", str(stl),
                "--output", str(gcode),
                "--config", str(cfg),
                "--module-dir", str(MODS),
                "--no-default-module-paths",
                "--instrument-stderr",
            ]
            rc, wall, cpu = run_with_cpu(args, env, stderr_log)
            sha = sha256_file(gcode)
            nbytes = gcode.stat().st_size
            parsed = parse_stderr(stderr_log)
            ok = (
                rc == 0
                and sha == exp_sha
                and nbytes == exp_bytes
                and parsed["degraded"] is False
                and parsed["fatal"] == 0
                and parsed["non_fatal"] == 0
                and parsed["code_1200"] == 0
            )
            if not ok:
                raise SystemExit(
                    f"PROBE INTEGRITY FAIL {cell} p{p_idx}: rc={rc} sha={sha[:12]} vs {exp_sha[:12]} "
                    f"bytes={nbytes} vs {exp_bytes} degraded={parsed['degraded']} "
                    f"fatal={parsed['fatal']} non_fatal={parsed['non_fatal']} code1200={parsed['code_1200']}"
                )
            passes.append({
                "pass": p_idx,
                "wall_s": wall,
                "cpu_s": cpu,
                "sha256": sha,
                "bytes": nbytes,
                "slice_complete_ms": parsed["slice_complete_ms"],
                "phase_ms": parsed["phase_ms"],
                "postpass_stage_ms": parsed["postpass_stage_ms"],
                "postpass_module_ms": parsed["postpass_module_ms"],
                "probes": parsed["probes"],
            })
            gcode.unlink(missing_ok=True)
            print(
                f"[probe] {cell} p{p_idx}: wall={wall:.3f}s cpu={cpu:.3f}s "
                f"postpass_stages={parsed['postpass_stage_ms']} OK",
                flush=True,
            )

        # 1 --profile pass (without T189_PROBE env var to keep profile clean)
        env_prof = os.environ.copy()
        env_prof["RAYON_NUM_THREADS"] = "12"
        env_prof.pop("T189_PROBE", None)
        gcode = OUT_DIR / f"{cell}_profile.gcode"
        stderr_log = OUT_DIR / f"{cell}_profile.stderr.jsonl"
        args_prof = [
            str(EXE),
            "slice",
            "--model", str(stl),
            "--output", str(gcode),
            "--config", str(cfg),
            "--module-dir", str(MODS),
            "--no-default-module-paths",
            "--instrument-stderr",
            "--profile",
        ]
        rc, wall, cpu = run_with_cpu(args_prof, env_prof, stderr_log)
        sha = sha256_file(gcode)
        nbytes = gcode.stat().st_size
        parsed = parse_stderr(stderr_log)
        if rc != 0 or sha != exp_sha or nbytes != exp_bytes or parsed["degraded"] is not False:
            raise SystemExit(f"PROFILE INTEGRITY FAIL {cell}: rc={rc} sha={sha[:12]}")
        gcode.unlink(missing_ok=True)

        postpass_fuel = {}
        ps = parsed["profile_summary"]
        prof = ps.get("profile", {}) if ps else {}
        if prof:
            for row in prof.get("modules", []) or []:
                mid = row.get("module_id") or row.get("module")
                if mid in (
                    "com.core.skirt-brim",
                    "com.core.overhang-classifier-default",
                    "com.core.part-cooling",
                    "com.core.wipe-tower",
                    "com.core.machine-gcode-emit",
                ):
                    postpass_fuel[mid] = row

        out_summary["cells"][cell] = {
            "median_wall_s": statistics.median(p["wall_s"] for p in passes),
            "median_cpu_s": statistics.median(p["cpu_s"] for p in passes),
            "passes": passes,
            "profile_pass": {
                "wall_s": wall,
                "cpu_s": cpu,
                "sha256": sha,
                "bytes": nbytes,
                "postpass_module_fuel": postpass_fuel,
            },
        }
        print(f"[profile] {cell}: wall={wall:.3f}s sha={sha[:12]} OK", flush=True)

    out_path = OUT_DIR / "probe_raw.json"
    out_path.write_text(json.dumps(out_summary, indent=2), encoding="utf-8")
    print(f"Wrote {out_path} (32/32 runs byte-identical to t182/reference.json)", flush=True)


if __name__ == "__main__":
    main()

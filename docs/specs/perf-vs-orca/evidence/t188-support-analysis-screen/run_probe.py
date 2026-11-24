#!/usr/bin/env python3
"""t188 observation-only PrePass::SupportAnalysis substage attribution runner.

Runs 3 instrumented passes + 1 --profile pass on all 4 supports-on cells
using the probe-built pnp_cli.exe (RAYON_NUM_THREADS=12, unsandboxed).
Verifies every single run is 100% byte-identical (SHA-256 + bytes) to t182/reference.json.
"""
from __future__ import annotations

import ctypes
import datetime
import hashlib
import json
import os
import re
import statistics
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
EXE = REPO / "target" / "t188" / "probe" / "pnp_cli.exe"
MODS = REPO / "modules" / "core-modules"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {
    "benchy": REPO / "tmp" / "3dbenchy.stl",
    "base": REPO / "tmp" / "base.stl",
}
REFERENCE = HERE.parent / "t182-spatial-hierarchical-phase3-union" / "reference.json"
OUT_DIR = REPO / "target" / "t188" / "probe" / "runs"


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


def inspect_jsonl(raw: Path) -> dict:
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    slice_complete_ms = None
    sa_start = None
    sa_end = None
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
            elif ev == "stage_start" and obj.get("stage_id") == "PrePass::SupportAnalysis":
                if sa_start is None:
                    sa_start = obj.get("elapsed_ms")
            elif ev == "stage_end" and obj.get("stage_id") == "PrePass::SupportAnalysis":
                sa_end = obj.get("elapsed_ms")
    return {
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
        "slice_complete_ms": slice_complete_ms,
        "sa_stage_ms": (sa_end - sa_start) if (sa_start is not None and sa_end is not None) else None,
    }


def parse_profile_sa(stderr_path: Path) -> dict:
    txt = stderr_path.read_text(encoding="utf-8", errors="replace")
    lines = [ln.strip() for ln in txt.splitlines() if "support_analysis" in ln]
    return {"sa_lines": lines}


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    ref_doc = json.loads(REFERENCE.read_text(encoding="utf-8"))
    refs = ref_doc["active_set"]
    ref_bytes = ref_doc["bytes"]
    exe_path = EXE if EXE.exists() else (REPO / "target" / "release" / "pnp_cli.exe")

    cells = ["benchy-classic-on", "benchy-arachne-on", "base-classic-on", "base-arachne-on"]
    rows = []

    for cell in cells:
        fixture, generator, supports = cell.split("-", 2)
        cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
        for rnd in (1, 2, 3):
            gcode = OUT_DIR / f"{cell}.probe.r{rnd}.gcode"
            stderr_p = OUT_DIR / f"{cell}.probe.r{rnd}.stderr.txt"
            probe_json = OUT_DIR / f"{cell}.probe.r{rnd}.json"
            args = [
                str(exe_path),
                "slice",
                "--model",
                str(FIXTURES[fixture]),
                "--output",
                str(gcode),
                "--config",
                str(cfg),
                "--module-dir",
                str(MODS),
                "--no-default-module-paths",
                "--instrument-stderr",
            ]
            env = dict(os.environ, RAYON_NUM_THREADS="12", T188_PROBE_OUT=str(probe_json))
            rc, wall, cpu = run_with_cpu(args, env, stderr_p)
            got_sha = sha256_file(gcode) if gcode.exists() else ""
            got_bytes = gcode.stat().st_size if gcode.exists() else 0
            ok_ref = got_sha == refs[cell] and got_bytes == ref_bytes[cell]
            inst = inspect_jsonl(stderr_p)
            pdata = json.loads(probe_json.read_text(encoding="utf-8")) if probe_json.exists() else None
            row = {
                "cell": cell,
                "kind": "probe",
                "round": rnd,
                "exit_code": rc,
                "wall_s": round(wall, 4),
                "cpu_s": round(cpu, 4),
                "gcode_sha256": got_sha,
                "gcode_bytes": got_bytes,
                "reference_match": ok_ref,
                "inst": inst,
                "probe": pdata,
            }
            rows.append(row)
            tag = "OK" if (rc == 0 and ok_ref and inst["degraded"] is False and inst["code_1200_count"] == 0) else "FAIL"
            sa_ms = pdata["total_wall_ns"] / 1e6 if pdata else -1
            print(f"[{tag}] {cell:20s} r{rnd} wall={wall:6.3f}s cpu={cpu:6.3f}s sa={sa_ms:7.2f}ms inst_sa={inst['sa_stage_ms']}ms ref={ok_ref}", flush=True)

        # 1 profile pass (with T188_PROBE_OUT unset to avoid probe overhead on profile)
        gcode = OUT_DIR / f"{cell}.profile.r1.gcode"
        stderr_p = OUT_DIR / f"{cell}.profile.r1.stderr.txt"
        args = [
            str(exe_path),
            "slice",
            "--model",
            str(FIXTURES[fixture]),
            "--output",
            str(gcode),
            "--config",
            str(cfg),
            "--module-dir",
            str(MODS),
            "--no-default-module-paths",
            "--profile",
        ]
        env = dict(os.environ, RAYON_NUM_THREADS="12")
        env.pop("T188_PROBE_OUT", None)
        rc, wall, cpu = run_with_cpu(args, env, stderr_p)
        got_sha = sha256_file(gcode) if gcode.exists() else ""
        got_bytes = gcode.stat().st_size if gcode.exists() else 0
        ok_ref = got_sha == refs[cell] and got_bytes == ref_bytes[cell]
        prof = parse_profile_sa(stderr_p)
        rows.append({
            "cell": cell,
            "kind": "profile",
            "round": 1,
            "exit_code": rc,
            "wall_s": round(wall, 4),
            "cpu_s": round(cpu, 4),
            "gcode_sha256": got_sha,
            "gcode_bytes": got_bytes,
            "reference_match": ok_ref,
            "profile": prof,
        })
        print(f"[PROF] {cell:20s} ref={ok_ref} lines={prof['sa_lines']}", flush=True)

    # Print readable median breakdown per cell
    print("\n=== t188 SUPPORT ANALYSIS PROBE SUMMARY (median of 3 passes) ===", flush=True)
    for cell in cells:
        rs = [r for r in rows if r["cell"] == cell and r["kind"] == "probe"]
        tot = statistics.median(r["probe"]["total_wall_ns"] for r in rs) / 1e6
        print(f"\n--- {cell}: total_wall={tot:.2f} ms (slice_complete={statistics.median(r['inst']['slice_complete_ms'] for r in rs):.0f} ms) ---")
        phases = rs[0]["probe"]["phases"].keys()
        for pk in phases:
            med = statistics.median(r["probe"]["phases"][pk] for r in rs) / 1e6
            print(f"  phase {pk:20s}: {med:8.2f} ms ({100.0 * med / tot:5.1f}%)")
        print(f"  counts: {rs[0]['probe']['counts']}")
        print(f"  layer_summary: {rs[0]['probe']['layer_summary']}")
        all_ops = sorted({k for r in rs for k in r["probe"]["ops"].keys()})
        for ok in all_ops:
            med_ms = statistics.median(r["probe"]["ops"][ok]["ns"] for r in rs) / 1e6
            max_ms = statistics.median(r["probe"]["ops"][ok]["max_ns"] for r in rs) / 1e6
            c = rs[0]["probe"]["ops"][ok]["count"]
            vin = rs[0]["probe"]["ops"][ok]["vin"]
            vout = rs[0]["probe"]["ops"][ok]["vout"]
            print(f"  op    {ok:30s}: sum={med_ms:9.2f} ms | cnt={c:5d} | max={max_ms:7.2f} ms | vin={vin:8d} | vout={vout:8d}")

    out = {
        "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "exe_sha256": sha256_file(exe_path),
        "rows": rows,
    }
    (OUT_DIR / "probe_raw.json").write_text(json.dumps(out, indent=2), encoding="utf-8")
    failures = sum(1 for r in rows if r["exit_code"] != 0 or not r["reference_match"])
    print(f"\nDONE: {len(rows)} runs, {failures} failures", flush=True)
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())

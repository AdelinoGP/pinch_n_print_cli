#!/usr/bin/env python3
"""t187 exact zero-drift candidate mechanism screen (interleaved A/B + fuel + stage).

Compares staged clean baseline-modules vs candidate-full-modules using the SAME
clean HEAD target/release/pnp_cli.exe, RAYON_NUM_THREADS=12, unsandboxed:
  1. 3 interleaved A/B rounds across all 4 supports-on cells (uninstrumented wall + process CPU)
     + 1 supports-off inertness check on benchy-classic-off & base-classic-off.
  2. 1 --profile pass per arm on all 4 supports-on cells (exact deterministic guest fuel + host thread-time).
  3. 1 --instrument-stderr pass per arm on all 4 supports-on cells (PrePass::SupportGeometry & tree-support-planner stage/module wall + qual).

Every single run is verified 100% byte-identical (SHA-256 + bytes) to t182/reference.json.
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
EXE = REPO / "target" / "release" / "pnp_cli.exe"
BASE_MODS = REPO / "target" / "t187-screen" / "baseline-modules"
CAND_MODS = REPO / "target" / "t187-screen" / "candidate-full-modules"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {
    "benchy": REPO / "tmp" / "3dbenchy.stl",
    "base": REPO / "tmp" / "base.stl",
}
REFERENCE = HERE.parent / "t182-spatial-hierarchical-phase3-union" / "reference.json"
OUT_DIR = REPO / "target" / "t187-screen" / "runs"


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", ctypes.c_uint32), ("dwHighDateTime", ctypes.c_uint32)]


def ft_to_sec(ft: FILETIME) -> float:
    return ((ft.dwHighDateTime << 32) | ft.dwLowDateTime) * 1e-7


def run_with_cpu(args: list[str], env: dict, stderr_path: Path | None) -> tuple[int, float, float]:
    t0 = time.perf_counter()
    if stderr_path is not None:
        fh = stderr_path.open("wb")
        stderr_arg = fh
    else:
        fh = None
        stderr_arg = subprocess.DEVNULL
    try:
        proc = subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=stderr_arg, env=env)
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
    finally:
        if fh is not None:
            fh.close()


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_profile_planner(stderr_path: Path) -> dict:
    txt = stderr_path.read_text(encoding="utf-8", errors="replace")
    out = {"planner_fuel": None, "total_guest_fuel": None, "host_native_clip_s": None, "host_native_offset_s": None}
    m = re.search(r"com\.core\.tree-support-planner\s+\d+\s+([\d,]+)\s+fuel", txt)
    if m:
        out["planner_fuel"] = int(m.group(1).replace(",", ""))
    m_tot = re.search(r"Guest fuel total:\s+([\d,]+)", txt)
    if m_tot:
        out["total_guest_fuel"] = int(m_tot.group(1).replace(",", ""))
    # Also capture host:native clip and offset lines
    for line in txt.splitlines():
        if "com.core.tree-support-planner" in line and "fuel" in line and out["planner_fuel"] is None:
            nums = re.findall(r"\b\d[\d,]{5,}\b", line)
            if nums:
                out["planner_fuel"] = int(nums[0].replace(",", ""))
    return out


def inspect_jsonl(raw: Path) -> dict:
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    slice_complete_ms = None
    sg_start = None
    sg_end = None
    tsp_ms = None
    trad_ms = None
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
            elif ev == "stage_start" and obj.get("stage_id") == "PrePass::SupportGeometry":
                if sg_start is None:
                    sg_start = obj.get("elapsed_ms")
            elif ev == "stage_end" and obj.get("stage_id") == "PrePass::SupportGeometry":
                sg_end = obj.get("elapsed_ms")
            elif ev == "module_complete" and obj.get("stage_id") == "PrePass::SupportGeometry":
                mid = obj.get("module_id")
                if mid == "com.core.tree-support-planner":
                    tsp_ms = obj.get("duration_ms")
                elif mid == "com.core.traditional-support-planner":
                    trad_ms = obj.get("duration_ms")
    return {
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
        "slice_complete_ms": slice_complete_ms,
        "sg_stage_ms": (sg_end - sg_start) if (sg_start is not None and sg_end is not None) else None,
        "tsp_ms": tsp_ms,
        "trad_ms": trad_ms,
    }


def run_one(cell: str, arm: str, kind: str, rnd: int, refs: dict, ref_bytes: dict) -> dict:
    fixture, generator, supports = cell.split("-", 2)
    cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
    mod_dir = BASE_MODS if arm == "baseline" else CAND_MODS
    gcode = OUT_DIR / f"{cell}.{arm}.{kind}.r{rnd}.gcode"
    stderr_path = OUT_DIR / f"{cell}.{arm}.{kind}.r{rnd}.stderr.txt" if kind != "ab" else None
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
        str(mod_dir),
        "--no-default-module-paths",
    ]
    if kind == "profile":
        args.append("--profile")
    elif kind == "inst":
        args.append("--instrument-stderr")

    env = dict(os.environ, RAYON_NUM_THREADS="12")
    env.pop("T187_PROBE_OUT", None)
    rc, wall, cpu = run_with_cpu(args, env, stderr_path)
    got_sha = sha256_file(gcode) if gcode.exists() else ""
    got_bytes = gcode.stat().st_size if gcode.exists() else 0
    ok_ref = got_sha == refs[cell] and got_bytes == ref_bytes[cell]
    row = {
        "cell": cell,
        "arm": arm,
        "kind": kind,
        "round": rnd,
        "exit_code": rc,
        "wall_s": round(wall, 4),
        "cpu_s": round(cpu, 4),
        "gcode_sha256": got_sha,
        "gcode_bytes": got_bytes,
        "reference_match": ok_ref,
    }
    if kind == "inst" and stderr_path is not None:
        row["inst"] = inspect_jsonl(stderr_path)
    elif kind == "profile" and stderr_path is not None:
        row["profile"] = parse_profile_planner(stderr_path)
        row["stderr_file"] = stderr_path.name
    tag = "OK" if (rc == 0 and ok_ref) else "FAIL"
    print(f"[{tag}] {cell:20s} {arm:9s} {kind:7s} r{rnd} wall={wall:7.3f}s cpu={cpu:7.3f}s ref={ok_ref}", flush=True)
    return row


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    ref_doc = json.loads(REFERENCE.read_text(encoding="utf-8"))
    refs = ref_doc["active_set"]
    ref_bytes = ref_doc["bytes"]

    rows = []
    # Quick byte-identity check first on all 4 supports-on cells + 2 supports-off cells
    on_cells = ["benchy-classic-on", "benchy-arachne-on", "base-classic-on", "base-arachne-on"]
    off_cells = ["benchy-classic-off", "base-classic-off"]

    # Warmup
    _ = run_one("benchy-classic-on", "baseline", "ab", 0, refs, ref_bytes)
    _ = run_one("benchy-classic-on", "candidate", "ab", 0, refs, ref_bytes)

    # Interleaved 3-round uninstrumented A/B on all 4 supports-on cells
    for cell in on_cells:
        for rnd in (1, 2, 3):
            order = ("baseline", "candidate") if rnd % 2 == 1 else ("candidate", "baseline")
            for arm in order:
                rows.append(run_one(cell, arm, "ab", rnd, refs, ref_bytes))

    # Supports-off byte-identity inertness check
    for cell in off_cells:
        rows.append(run_one(cell, "candidate", "ab", 1, refs, ref_bytes))

    # Instrumented stage/module comparison (1 pass/arm/cell)
    for cell in on_cells:
        for arm in ("baseline", "candidate"):
            rows.append(run_one(cell, arm, "inst", 1, refs, ref_bytes))

    # Deterministic --profile fuel comparison (1 pass/arm/cell)
    for cell in on_cells:
        for arm in ("baseline", "candidate"):
            rows.append(run_one(cell, arm, "profile", 1, refs, ref_bytes))

    # Summary print
    print("\n=== INTERLEAVED 3-ROUND SCREEN SUMMARY ===", flush=True)
    summary = {}
    for cell in on_cells:
        b_ab = [r for r in rows if r["cell"] == cell and r["arm"] == "baseline" and r["kind"] == "ab"]
        c_ab = [r for r in rows if r["cell"] == cell and r["arm"] == "candidate" and r["kind"] == "ab"]
        bw = statistics.median(r["wall_s"] for r in b_ab)
        cw = statistics.median(r["wall_s"] for r in c_ab)
        bc = statistics.median(r["cpu_s"] for r in b_ab)
        cc = statistics.median(r["cpu_s"] for r in c_ab)
        dw_pct = 100.0 * (bw - cw) / bw
        dc_pct = 100.0 * (bc - cc) / bc
        b_inst = next(r for r in rows if r["cell"] == cell and r["arm"] == "baseline" and r["kind"] == "inst")["inst"]
        c_inst = next(r for r in rows if r["cell"] == cell and r["arm"] == "candidate" and r["kind"] == "inst")["inst"]
        summary[cell] = {
            "base_wall_med_s": round(bw, 4),
            "cand_wall_med_s": round(cw, 4),
            "wall_saved_s": round(bw - cw, 4),
            "wall_saved_pct": round(dw_pct, 2),
            "base_cpu_med_s": round(bc, 4),
            "cand_cpu_med_s": round(cc, 4),
            "cpu_saved_pct": round(dc_pct, 2),
            "base_tsp_ms": b_inst["tsp_ms"],
            "cand_tsp_ms": c_inst["tsp_ms"],
            "base_sg_ms": b_inst["sg_stage_ms"],
            "cand_sg_ms": c_inst["sg_stage_ms"],
        }
        print(
            f"  {cell:20s} wall: {bw:6.3f} -> {cw:6.3f}s ({dw_pct:+6.2f}%) | "
            f"cpu: {bc:6.3f} -> {cc:6.3f}s ({dc_pct:+6.2f}%) | "
            f"tsp: {b_inst['tsp_ms']} -> {c_inst['tsp_ms']}ms | sg: {b_inst['sg_stage_ms']} -> {c_inst['sg_stage_ms']}ms",
            flush=True,
        )

    out = {
        "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "exe_sha256": sha256_file(EXE),
        "baseline_planner_sha256": sha256_file(BASE_MODS / "tree-support-planner" / "tree-support-planner.wasm"),
        "candidate_planner_sha256": sha256_file(CAND_MODS / "tree-support-planner" / "tree-support-planner.wasm"),
        "summary": summary,
        "rows": rows,
    }
    (OUT_DIR / "screen.json").write_text(json.dumps(out, indent=2), encoding="utf-8")
    failures = sum(1 for r in rows if r["exit_code"] != 0 or not r["reference_match"])
    print(f"DONE: {len(rows)} runs, {failures} failures", flush=True)
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())

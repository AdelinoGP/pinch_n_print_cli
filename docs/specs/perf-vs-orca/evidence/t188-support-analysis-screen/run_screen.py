#!/usr/bin/env python3
"""t188 exact zero-drift PrePass::SupportAnalysis candidate mechanism screen.

Compares clean HEAD baseline binary (target/t188/baseline/pnp_cli.exe) vs
candidate binary (target/t188/candidate/pnp_cli.exe) using the SAME clean
modules/core-modules tree, RAYON_NUM_THREADS=12, unsandboxed:
  1. 3 interleaved A/B rounds across all 4 supports-on cells (uninstrumented wall + process CPU)
     + supports-off inertness check on benchy-classic-off & base-classic-off.
  2. 1 --instrument-stderr pass per arm on all 4 supports-on cells (PrePass::SupportAnalysis stage wall + qual).
  3. 1 --profile pass per arm on all 4 supports-on cells (host:support_analysis offset/clip counts & thread-time).
  4. 1 --instrument-stderr ablation pass per mode (sa1, sa123, sa4, sa1234) on benchy-classic-on & base-classic-on.

Every single run is verified 100% byte-identical (SHA-256 + bytes) to t182/reference.json.
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
BASE_EXE = REPO / "target" / "t188" / "baseline" / "pnp_cli.exe"
CAND_EXE = REPO / "target" / "t188" / "candidate" / "pnp_cli.exe"
MODS = REPO / "modules" / "core-modules"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {
    "benchy": REPO / "tmp" / "3dbenchy.stl",
    "base": REPO / "tmp" / "base.stl",
}
REFERENCE = HERE.parent / "t182-spatial-hierarchical-phase3-union" / "reference.json"
OUT_DIR = REPO / "target" / "t188" / "screen" / "runs"


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", ctypes.c_uint32), ("dwHighDateTime", ctypes.c_uint32)]


def ft_to_sec(ft: FILETIME) -> float:
    return ((ft.dwHighDateTime << 32) | ft.dwLowDateTime) * 1e-7


def run_with_cpu(args: list[str], env: dict, stderr_path: Path | None) -> tuple[int, float, float]:
    t0 = time.perf_counter()
    fh = stderr_path.open("wb") if stderr_path is not None else None
    stderr_arg = fh if fh is not None else subprocess.DEVNULL
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


def inspect_jsonl(raw: Path) -> dict:
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    slice_complete_ms = None
    sa_ms = None
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
            elif ev == "stage_complete" and obj.get("stage") == "PrePass::SupportAnalysis":
                sa_ms = obj.get("elapsed_ms")
            elif ev == "module_complete" and obj.get("stage") == "PrePass::SupportAnalysis":
                if sa_ms is None:
                    sa_ms = obj.get("elapsed_ms")
    if sa_ms is None and sa_start is not None and sa_end is not None:
        sa_ms = sa_end - sa_start
    return {
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
        "slice_complete_ms": slice_complete_ms,
        "sa_stage_ms": sa_ms,
    }


def parse_profile_sa(stderr_path: Path) -> dict:
    txt = stderr_path.read_text(encoding="utf-8", errors="replace")
    for line in txt.splitlines():
        s = line.strip()
        if s.startswith("{") and '"profile_summary"' in s:
            obj = json.loads(s)
            prof = obj.get("profile", {})
            fuel_total = prof.get("fuel_total")
            wall_total_ns = prof.get("wall_total_ns")
            sa_entry = None
            for m in prof.get("modules", []):
                if m.get("module_id") == "host:support_analysis":
                    sa_entry = m
                    break
            scopes = {}
            if sa_entry:
                for sc in sa_entry.get("scopes", []):
                    scopes[sc["scope"]] = {
                        "calls": sc["calls"],
                        "wall_ns": sc["total_wall_ns"],
                    }
            return {
                "fuel_total": fuel_total,
                "wall_total_ns": wall_total_ns,
                "sa_total_calls": sa_entry["calls"] if sa_entry else 0,
                "sa_total_wall_ns": sa_entry["total_wall_ns"] if sa_entry else 0,
                "scopes": scopes,
            }
    return {}


def run_one(
    cell: str,
    arm: str,
    kind: str,
    rnd: int,
    refs: dict,
    ref_bytes: dict,
    mode: str | None = None,
) -> dict:
    fixture, generator, supports = cell.split("-", 2)
    cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
    exe = BASE_EXE if arm == "baseline" else CAND_EXE
    tag_mode = f".{mode}" if mode else ""
    gcode = OUT_DIR / f"{cell}.{arm}{tag_mode}.{kind}.r{rnd}.gcode"
    stderr_path = (
        OUT_DIR / f"{cell}.{arm}{tag_mode}.{kind}.r{rnd}.stderr.txt"
        if kind != "ab"
        else None
    )
    args = [
        str(exe),
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
    ]
    if kind == "profile":
        args.append("--profile")
    elif kind in ("inst", "ablation"):
        args.append("--instrument-stderr")

    env = dict(os.environ, RAYON_NUM_THREADS="12")
    env.pop("T188_PROBE_OUT", None)
    env.pop("T188_CENSUS", None)
    if mode is not None:
        env["T188_MODE"] = mode
    else:
        env.pop("T188_MODE", None)

    rc, wall, cpu = run_with_cpu(args, env, stderr_path)
    got_sha = sha256_file(gcode) if gcode.exists() else ""
    got_bytes = gcode.stat().st_size if gcode.exists() else 0
    ok_ref = got_sha == refs[cell] and got_bytes == ref_bytes[cell]
    row = {
        "cell": cell,
        "arm": arm,
        "mode": mode or ("baseline" if arm == "baseline" else "full"),
        "kind": kind,
        "round": rnd,
        "exit_code": rc,
        "wall_s": round(wall, 4),
        "cpu_s": round(cpu, 4),
        "gcode_sha256": got_sha,
        "gcode_bytes": got_bytes,
        "reference_match": ok_ref,
    }
    if kind in ("inst", "ablation") and stderr_path is not None:
        row["inst"] = inspect_jsonl(stderr_path)
    elif kind == "profile" and stderr_path is not None:
        row["profile"] = parse_profile_sa(stderr_path)
    tag = "OK" if (rc == 0 and ok_ref) else "FAIL"
    extra = ""
    if "inst" in row:
        extra = f" sa={row['inst']['sa_stage_ms']}ms sc={row['inst']['slice_complete_ms']}ms"
    print(
        f"[{tag}] {cell:20s} {row['mode']:9s} {kind:8s} r{rnd} wall={wall:7.3f}s cpu={cpu:7.3f}s ref={ok_ref}{extra}",
        flush=True,
    )
    return row


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    ref_doc = json.loads(REFERENCE.read_text(encoding="utf-8"))
    refs = ref_doc["active_set"]
    ref_bytes = ref_doc["bytes"]

    on_cells = ["benchy-classic-on", "benchy-arachne-on", "base-classic-on", "base-arachne-on"]
    off_cells = ["benchy-classic-off", "base-classic-off"]
    rows = []

    # Warmup
    _ = run_one("benchy-classic-on", "baseline", "ab", 0, refs, ref_bytes)
    _ = run_one("benchy-classic-on", "candidate", "ab", 0, refs, ref_bytes)

    # 3-round interleaved uninstrumented A/B on all 4 supports-on cells
    for cell in on_cells:
        for rnd in (1, 2, 3):
            order = ("baseline", "candidate") if rnd % 2 == 1 else ("candidate", "baseline")
            for arm in order:
                rows.append(run_one(cell, arm, "ab", rnd, refs, ref_bytes))

    # Supports-off byte-identity inertness check
    for cell in off_cells:
        rows.append(run_one(cell, "candidate", "ab", 1, refs, ref_bytes))

    # Instrumented stage comparison (1 pass/arm/cell)
    for cell in on_cells:
        for arm in ("baseline", "candidate"):
            rows.append(run_one(cell, arm, "inst", 1, refs, ref_bytes))

    # Deterministic --profile host:support_analysis comparison (1 pass/arm/cell)
    for cell in on_cells:
        for arm in ("baseline", "candidate"):
            rows.append(run_one(cell, arm, "profile", 1, refs, ref_bytes))

    # Mechanism ablation on benchy-classic-on and base-classic-on
    for cell in ("benchy-classic-on", "base-classic-on"):
        for mode in ("sa1", "sa123", "sa4", "sa1234"):
            rows.append(run_one(cell, "candidate", "ablation", 1, refs, ref_bytes, mode=mode))

    print("\n=== t188 INTERLEAVED 3-ROUND SCREEN SUMMARY ===", flush=True)
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
        b_prof = next(r for r in rows if r["cell"] == cell and r["arm"] == "baseline" and r["kind"] == "profile")["profile"]
        c_prof = next(r for r in rows if r["cell"] == cell and r["arm"] == "candidate" and r["kind"] == "profile")["profile"]
        summary[cell] = {
            "base_wall_med_s": round(bw, 4),
            "cand_wall_med_s": round(cw, 4),
            "wall_saved_s": round(bw - cw, 4),
            "wall_saved_pct": round(dw_pct, 2),
            "base_cpu_med_s": round(bc, 4),
            "cand_cpu_med_s": round(cc, 4),
            "cpu_saved_pct": round(dc_pct, 2),
            "base_sa_ms": b_inst["sa_stage_ms"],
            "cand_sa_ms": c_inst["sa_stage_ms"],
            "base_prof_sa_calls": b_prof.get("sa_total_calls"),
            "cand_prof_sa_calls": c_prof.get("sa_total_calls"),
            "base_prof_sa_s": round(b_prof.get("sa_total_wall_ns", 0) / 1e9, 4),
            "cand_prof_sa_s": round(c_prof.get("sa_total_wall_ns", 0) / 1e9, 4),
        }
        print(
            f"  {cell:20s} wall: {bw:6.3f} -> {cw:6.3f}s ({dw_pct:+6.2f}%) | "
            f"cpu: {bc:6.3f} -> {cc:6.3f}s ({dc_pct:+6.2f}%) | "
            f"sa: {b_inst['sa_stage_ms']} -> {c_inst['sa_stage_ms']}ms | "
            f"host_sa: {b_prof.get('sa_total_wall_ns',0)/1e9:.3f} -> {c_prof.get('sa_total_wall_ns',0)/1e9:.3f}s "
            f"(calls {b_prof.get('sa_total_calls')} -> {c_prof.get('sa_total_calls')})",
            flush=True,
        )

    out = {
        "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "baseline_exe_sha256": sha256_file(BASE_EXE),
        "candidate_exe_sha256": sha256_file(CAND_EXE),
        "summary": summary,
        "rows": rows,
    }
    (OUT_DIR / "screen_raw.json").write_text(json.dumps(out, indent=2), encoding="utf-8")
    failures = sum(1 for r in rows if r["exit_code"] != 0 or not r["reference_match"])
    print(f"DONE: {len(rows)} runs, {failures} failures", flush=True)
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())

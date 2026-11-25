#!/usr/bin/env python3
"""t189 exact PostPass acceleration screen runner.

Runs:
1. Full 8-cell interleaved uninstrumented A/B (3 rounds x 2 arms x 8 cells = 48 runs + 1 warmup)
   comparing clean HEAD baseline (target/t189/baseline/pnp_cli.exe + modules/core-modules)
   against Full Zero-WIT Exact Candidate PP-1+PP-2+PP-3 (target/t189/candidate/pnp_cli.exe +
   target/t189/candidate-mods).
2. Instrumented stage-breakdown comparison (baseline vs candidate on all 8 cells = 16 runs).
3. Monotonic ablation ladder (baseline, PP-2, PP-3, PP-2+PP-3 host-only, PP-1, Full) on 4
   representative cells (benchy/base x supports-off/on = 24 runs).
4. Deterministic --profile guest-fuel check on benchy-classic-on and base-classic-on (baseline
   vs candidate = 4 runs) to quantify PostPass guest fuel savings from lazy LayerCollectionView.

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
BASE_EXE = REPO / "target" / "t189" / "baseline" / "pnp_cli.exe"
CAND_EXE = REPO / "target" / "t189" / "candidate" / "pnp_cli.exe"
BASE_MODS = REPO / "modules" / "core-modules"
CAND_MODS = REPO / "target" / "t189" / "candidate-mods"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {
    "benchy": REPO / "tmp" / "3dbenchy.stl",
    "base": REPO / "tmp" / "base.stl",
}
REFERENCE = HERE.parent / "t182-spatial-hierarchical-phase3-union" / "reference.json"
OUT_DIR = REPO / "target" / "t189" / "screen"

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

ABLATION_CELLS = [
    ("benchy-classic-off", "benchy", "pnp-classic-supports-off.json"),
    ("benchy-classic-on", "benchy", "pnp-classic-supports-on.json"),
    ("base-classic-off", "base", "pnp-classic-supports-off.json"),
    ("base-classic-on", "base", "pnp-classic-supports-on.json"),
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


def parse_inst(raw: Path) -> dict:
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    slice_complete_ms = None
    phase_ms: dict[str, int] = {}
    stage_ms: dict[str, int] = {}
    module_ms: dict[str, int] = {}
    profile_summary = None
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
        "profile_summary": profile_summary,
    }


def run_one(
    exe: Path,
    mods: Path,
    stl: Path,
    cfg: Path,
    tag: str,
    exp_sha: str,
    exp_bytes: int,
    flags: str | None = None,
    instrument: bool = False,
    profile: bool = False,
) -> dict:
    gcode = OUT_DIR / f"{tag}.gcode"
    stderr_p = OUT_DIR / f"{tag}.stderr.log"
    args = [
        str(exe),
        "slice",
        "--model", str(stl),
        "--output", str(gcode),
        "--config", str(cfg),
        "--module-dir", str(mods),
        "--no-default-module-paths",
    ]
    if instrument or profile:
        args.append("--instrument-stderr")
    if profile:
        args.append("--profile")
    env = os.environ.copy()
    env["RAYON_NUM_THREADS"] = "12"
    if flags is not None:
        env["T189_FLAGS"] = flags
    else:
        env.pop("T189_FLAGS", None)
    rc, wall, cpu = run_with_cpu(args, env, stderr_p)
    sha = sha256_file(gcode) if gcode.exists() else ""
    nbytes = gcode.stat().st_size if gcode.exists() else 0
    gcode.unlink(missing_ok=True)
    ok = rc == 0 and sha == exp_sha and nbytes == exp_bytes
    inst = parse_inst(stderr_p) if (instrument or profile) else None
    if inst is not None:
        ok = ok and inst["degraded"] is False and inst["fatal"] == 0 and inst["non_fatal"] == 0 and inst["code_1200"] == 0
    if not ok:
        raise SystemExit(
            f"SCREEN INTEGRITY FAIL {tag}: rc={rc} sha={sha[:12]} vs {exp_sha[:12]} "
            f"bytes={nbytes} vs {exp_bytes} inst={inst}"
        )
    res = {
        "tag": tag,
        "wall_s": round(wall, 5),
        "cpu_s": round(cpu, 5),
        "sha256": sha,
        "bytes": nbytes,
    }
    if inst is not None:
        res["inst"] = {k: v for k, v in inst.items() if k != "profile_summary"}
        if profile and inst["profile_summary"]:
            prof = inst["profile_summary"].get("profile", {})
            lf_fuel = {}
            for row in prof.get("modules", []) or []:
                mid = row.get("module_id") or row.get("module")
                if mid in (
                    "com.core.skirt-brim",
                    "com.core.overhang-classifier-default",
                    "com.core.part-cooling",
                    "com.core.wipe-tower",
                    "com.core.machine-gcode-emit",
                ):
                    lf_fuel[mid] = row
            res["postpass_fuel"] = lf_fuel
            res["total_fuel"] = prof.get("fuel_total")
    return res


def main() -> None:
    for p in (BASE_EXE, CAND_EXE, BASE_MODS, CAND_MODS):
        if not p.exists():
            raise SystemExit(f"missing required path: {p}")
    ref = json.loads(REFERENCE.read_text(encoding="utf-8"))
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    # 1 warmup run (untimed, still reference-checked)
    print("[warmup] benchy-classic-off ...", flush=True)
    run_one(
        BASE_EXE, BASE_MODS, FIXTURES["benchy"], CONFIGS / "pnp-classic-supports-off.json",
        "warmup_base", ref["active_set"]["benchy-classic-off"], ref["bytes"]["benchy-classic-off"],
    )
    run_one(
        CAND_EXE, CAND_MODS, FIXTURES["benchy"], CONFIGS / "pnp-classic-supports-off.json",
        "warmup_cand", ref["active_set"]["benchy-classic-off"], ref["bytes"]["benchy-classic-off"],
        flags="all",
    )

    out: dict = {
        "timestamp_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "baseline_exe_sha256": sha256_file(BASE_EXE),
        "candidate_exe_sha256": sha256_file(CAND_EXE),
        "rayon_num_threads": 12,
        "ab": {},
        "instrumented": {},
        "ablation": {},
        "profile_fuel": {},
    }

    # 1. Full 8-cell interleaved uninstrumented A/B (3 pairs/cell, alternating order)
    print("\n=== 1. FULL 8-CELL INTERLEAVED A/B (3 pairs/cell, uninstrumented) ===", flush=True)
    for cell, model_key, cfg_name in CELLS:
        stl = FIXTURES[model_key]
        cfg = CONFIGS / cfg_name
        exp_sha = ref["active_set"][cell]
        exp_bytes = ref["bytes"][cell]
        pairs = []
        for r in range(1, 4):
            if r % 2 == 1:
                b = run_one(BASE_EXE, BASE_MODS, stl, cfg, f"ab_{cell}_r{r}_base", exp_sha, exp_bytes)
                c = run_one(CAND_EXE, CAND_MODS, stl, cfg, f"ab_{cell}_r{r}_cand", exp_sha, exp_bytes, flags="all")
            else:
                c = run_one(CAND_EXE, CAND_MODS, stl, cfg, f"ab_{cell}_r{r}_cand", exp_sha, exp_bytes, flags="all")
                b = run_one(BASE_EXE, BASE_MODS, stl, cfg, f"ab_{cell}_r{r}_base", exp_sha, exp_bytes)
            pairs.append({"round": r, "baseline": b, "candidate": c})
            dw = (b["wall_s"] - c["wall_s"]) / b["wall_s"] * 100.0
            dc = (b["cpu_s"] - c["cpu_s"]) / b["cpu_s"] * 100.0
            print(f"  [{cell:18s} r{r}] wall {b['wall_s']:.3f}->{c['wall_s']:.3f}s ({dw:+.2f}%) cpu {b['cpu_s']:.3f}->{c['cpu_s']:.3f}s ({dc:+.2f}%) OK", flush=True)

        b_w = statistics.median(p["baseline"]["wall_s"] for p in pairs)
        c_w = statistics.median(p["candidate"]["wall_s"] for p in pairs)
        b_c = statistics.median(p["baseline"]["cpu_s"] for p in pairs)
        c_c = statistics.median(p["candidate"]["cpu_s"] for p in pairs)
        wall_saved_pct = (b_w - c_w) / b_w * 100.0
        cpu_saved_pct = (b_c - c_c) / b_c * 100.0
        wins = sum(1 for p in pairs if p["candidate"]["wall_s"] < p["baseline"]["wall_s"])
        cpu_wins = sum(1 for p in pairs if p["candidate"]["cpu_s"] < p["baseline"]["cpu_s"])
        out["ab"][cell] = {
            "baseline_median_wall_s": round(b_w, 5),
            "candidate_median_wall_s": round(c_w, 5),
            "wall_saved_pct": round(wall_saved_pct, 3),
            "floor_multiple": round(wall_saved_pct / 0.80, 2),
            "wall_wins": wins,
            "baseline_median_cpu_s": round(b_c, 5),
            "candidate_median_cpu_s": round(c_c, 5),
            "cpu_saved_pct": round(cpu_saved_pct, 3),
            "cpu_wins": cpu_wins,
            "pairs": pairs,
        }
        print(
            f"  ==> {cell:18s} MEDIAN wall={wall_saved_pct:+.2f}% ({wall_saved_pct/0.80:.2f}x floor, {wins}/3) "
            f"cpu={cpu_saved_pct:+.2f}% ({cpu_wins}/3)",
            flush=True,
        )

    # 2. Instrumented stage comparison across all 8 cells
    print("\n=== 2. INSTRUMENTED POSTPASS STAGE COMPARISON (8 cells) ===", flush=True)
    for cell, model_key, cfg_name in CELLS:
        stl = FIXTURES[model_key]
        cfg = CONFIGS / cfg_name
        exp_sha = ref["active_set"][cell]
        exp_bytes = ref["bytes"][cell]
        b = run_one(BASE_EXE, BASE_MODS, stl, cfg, f"inst_{cell}_base", exp_sha, exp_bytes, instrument=True)
        c = run_one(CAND_EXE, CAND_MODS, stl, cfg, f"inst_{cell}_cand", exp_sha, exp_bytes, flags="all", instrument=True)
        out["instrumented"][cell] = {"baseline": b, "candidate": c}
        print(
            f"  [inst {cell:18s}] postpass {b['inst']['phase_ms'].get('postpass')}->{c['inst']['phase_ms'].get('postpass')}ms "
            f"stages {b['inst']['postpass_stage_ms']} -> {c['inst']['postpass_stage_ms']}",
            flush=True,
        )

    # 3. Monotonic ablation ladder on 4 representative cells
    print("\n=== 3. MONOTONIC ABLATION LADDER (4 cells x 6 configs) ===", flush=True)
    configs = [
        ("baseline", BASE_EXE, BASE_MODS, None),
        ("pp2_only", CAND_EXE, BASE_MODS, "pp2"),
        ("pp3_only", CAND_EXE, BASE_MODS, "pp3"),
        ("pp2_pp3_host_only", CAND_EXE, BASE_MODS, "pp2,pp3"),
        ("pp1_only", CAND_EXE, CAND_MODS, "pp1"),
        ("full_pp1_pp2_pp3", CAND_EXE, CAND_MODS, "all"),
    ]
    for cell, model_key, cfg_name in ABLATION_CELLS:
        stl = FIXTURES[model_key]
        cfg = CONFIGS / cfg_name
        exp_sha = ref["active_set"][cell]
        exp_bytes = ref["bytes"][cell]
        out["ablation"][cell] = {}
        for cname, exe, mods, flg in configs:
            r = run_one(exe, mods, stl, cfg, f"abl_{cell}_{cname}", exp_sha, exp_bytes, flags=flg, instrument=True)
            out["ablation"][cell][cname] = r
            print(
                f"  [abl {cell:18s} {cname:18s}] wall={r['wall_s']:.3f}s postpass={r['inst']['phase_ms'].get('postpass')}ms "
                f"stages={r['inst']['postpass_stage_ms']}",
                flush=True,
            )

    # 4. Deterministic --profile guest-fuel check on benchy-classic-on & base-classic-on
    print("\n=== 4. DETERMINISTIC GUEST FUEL CHECK (--profile) ===", flush=True)
    for cell, model_key, cfg_name in [
        ("benchy-classic-on", "benchy", "pnp-classic-supports-on.json"),
        ("base-classic-on", "base", "pnp-classic-supports-on.json"),
    ]:
        stl = FIXTURES[model_key]
        cfg = CONFIGS / cfg_name
        exp_sha = ref["active_set"][cell]
        exp_bytes = ref["bytes"][cell]
        b = run_one(BASE_EXE, BASE_MODS, stl, cfg, f"prof_{cell}_base", exp_sha, exp_bytes, profile=True)
        c = run_one(CAND_EXE, CAND_MODS, stl, cfg, f"prof_{cell}_cand", exp_sha, exp_bytes, flags="all", profile=True)
        out["profile_fuel"][cell] = {"baseline": b, "candidate": c}
        print(f"  [prof {cell:18s}] total_fuel {b.get('total_fuel')} -> {c.get('total_fuel')}", flush=True)

    out_path = OUT_DIR / "screen_raw.json"
    out_path.write_text(json.dumps(out, indent=2), encoding="utf-8")
    print(f"\nWrote {out_path} — ALL runs 100% byte-identical to t182/reference.json!", flush=True)


if __name__ == "__main__":
    main()

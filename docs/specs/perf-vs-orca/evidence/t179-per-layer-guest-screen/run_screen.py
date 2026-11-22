#!/usr/bin/env python3
"""Unsandboxed attribution & candidate screen for t179.

Runs TWO clean unsandboxed experiments against the untouched production
`target/release/pnp_cli.exe` and the four isolated scratch module dirs:
  - baseline : modules/core-modules
  - arm_ts1  : target/t179-arm-ts1  (TS-1: paint_policy_for + skeleton_wall_count)
  - arm_ts2  : target/t179-arm-ts2  (TS-2: TS-1 + redundant role_union bypass)
  - arm_il   : target/t179-arm-il   (IL-ALL: IL-1 Clipper64 open-subject tag + IL-2 meta/source_segments + IL-3 BoundaryInfillGraph/connect)
  - arm_both : target/t179-arm-both (TS-2 + IL-ALL combined)

Part 1 (--instrument-stderr, 1 pass per cell/arm):
  Measures per_layer phase wall, module_thread_sum_ms, L2 infill-linker elapsed_ms,
  and single_layer_straggler_tail_ms across arms.

Part 2 (uninstrumented interleaved wall + GetProcessTimes CPU, 1 warmup + 3 rounds):
  Measures screen-grade paired wall & CPU impact on all 8 cells with byte-identical
  SHA-256 verification against t175's active reference.json on every single run.
  NOT a formal landing gate (graduated tickets run the full standing 5-pair ordinary +
  accelerated + null-band gate).

Writes:
  docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/reduced/screen_runs.json
"""

from __future__ import annotations

import ctypes
import ctypes.wintypes as wt
import datetime
import hashlib
import json
import os
import statistics
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
EXE = REPO / "target" / "release" / "pnp_cli.exe"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {"benchy": REPO / "tmp" / "3dbenchy.stl", "base": REPO / "tmp" / "base.stl"}
REFERENCE = HERE.parent / "t175-tiled-collision-carve" / "reference.json"

ARMS = {
    "baseline": REPO / "modules" / "core-modules",
    "arm_ts1": REPO / "target" / "t179-arm-ts1",
    "arm_ts2": REPO / "target" / "t179-arm-ts2",
    "arm_il": REPO / "target" / "t179-arm-il",
    "arm_both": REPO / "target" / "t179-arm-both",
}

CELL_ARMS_INSTR = {
    "benchy-classic-off": ["baseline", "arm_il"],
    "benchy-arachne-off": ["baseline", "arm_il"],
    "benchy-classic-on": ["baseline", "arm_ts1", "arm_ts2", "arm_il", "arm_both"],
    "benchy-arachne-on": ["baseline", "arm_ts2", "arm_il", "arm_both"],
    "base-classic-off": ["baseline", "arm_il"],
    "base-arachne-off": ["baseline", "arm_il"],
    "base-classic-on": ["baseline", "arm_ts1", "arm_ts2", "arm_il", "arm_both"],
    "base-arachne-on": ["baseline", "arm_ts2", "arm_il", "arm_both"],
}

CELL_ARMS_WALL = {
    "benchy-classic-off": ["baseline", "arm_il"],
    "benchy-arachne-off": ["baseline", "arm_il"],
    "benchy-classic-on": ["baseline", "arm_ts1", "arm_ts2", "arm_il", "arm_both"],
    "benchy-arachne-on": ["baseline", "arm_ts2", "arm_il", "arm_both"],
    "base-classic-off": ["baseline", "arm_il"],
    "base-arachne-off": ["baseline", "arm_il"],
    "base-classic-on": ["baseline", "arm_ts1", "arm_ts2", "arm_il", "arm_both"],
    "base-arachne-on": ["baseline", "arm_ts2", "arm_il", "arm_both"],
}

kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", wt.DWORD), ("dwHighDateTime", wt.DWORD)]


def ft_to_sec(ft: FILETIME) -> float:
    val = (ft.dwHighDateTime << 32) | ft.dwLowDateTime
    return val * 1e-7


def run_with_cpu(args: list[str], env: dict[str, str], stderr_path: Path | None) -> tuple[int, float, float]:
    t0 = time.perf_counter()
    if stderr_path is not None:
        errf = stderr_path.open("wb")
    else:
        errf = subprocess.DEVNULL
    try:
        proc = subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=errf, env=env)
        rc = proc.wait()
        wall = time.perf_counter() - t0
        c_ft, e_ft, k_ft, u_ft = FILETIME(), FILETIME(), FILETIME(), FILETIME()
        ok = kernel32.GetProcessTimes(
            wt.HANDLE(int(proc._handle)),
            ctypes.byref(c_ft),
            ctypes.byref(e_ft),
            ctypes.byref(k_ft),
            ctypes.byref(u_ft),
        )
        cpu = (ft_to_sec(k_ft) + ft_to_sec(u_ft)) if ok else 0.0
    finally:
        if stderr_path is not None:
            errf.close()
    return rc, wall, cpu


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_instrumented(path: Path) -> dict:
    p_start = None
    p_end = None
    phases = {}
    slice_complete_ms = None
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    layer_ends: list[tuple[int, int]] = []
    mod_thread: dict[str, int] = {}
    l2_il_ms = None
    l2_dur_ms = None
    l2_start_rel = None
    l2_end_rel = None
    top_mod_by_layer: dict[int, tuple[str, int]] = {}

    with path.open("r", encoding="utf-8", errors="replace") as fh:
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
            if ev == "phase_start" and obj.get("phase") == "per_layer":
                p_start = obj.get("timestamp_ms")
            elif ev == "phase_complete":
                ph = obj.get("phase")
                phases[ph] = obj.get("elapsed_ms")
                if ph == "per_layer":
                    p_end = obj.get("timestamp_ms")
            elif ev == "layer_start" and obj.get("layer_index") == 2 and p_start is not None:
                l2_start_rel = obj.get("timestamp_ms") - p_start
            elif ev == "layer_complete":
                li = obj.get("layer_index")
                ts = obj.get("timestamp_ms")
                if p_start is not None and ts is not None and li is not None:
                    rel = ts - p_start
                    layer_ends.append((rel, int(li)))
                    if li == 2:
                        l2_end_rel = rel
                        l2_dur_ms = obj.get("elapsed_ms")
            elif ev == "module_complete" and obj.get("phase") == "per_layer":
                mid = obj.get("module_id")
                el = int(obj.get("elapsed_ms", 0))
                mod_thread[mid] = mod_thread.get(mid, 0) + el
                li = obj.get("layer_index")
                if li is not None:
                    prev = top_mod_by_layer.get(int(li))
                    if prev is None or el > prev[1]:
                        top_mod_by_layer[int(li)] = (mid, el)
                if li == 2 and mid == "com.core.infill-linker":
                    l2_il_ms = el
            elif ev == "slice_complete":
                slice_complete_ms = obj.get("elapsed_ms")
                degraded = obj.get("degraded")
                fatal = obj.get("fatal_error_count")
                non_fatal = obj.get("non_fatal_error_count")

    layer_ends.sort(reverse=True)
    straggler_tail_ms = 0
    last_layer = None
    second_last = None
    last_top_mod = None
    if len(layer_ends) >= 2:
        last_rel, last_layer = layer_ends[0]
        sec_rel, second_last = layer_ends[1]
        straggler_tail_ms = max(0, last_rel - sec_rel)
        if last_layer in top_mod_by_layer:
            last_top_mod = {"module": top_mod_by_layer[last_layer][0], "ms": top_mod_by_layer[last_layer][1]}

    return {
        "slice_complete_ms": slice_complete_ms,
        "phases_ms": phases,
        "per_layer_ms": phases.get("per_layer"),
        "tree_support_thread_ms": mod_thread.get("com.core.tree-support", 0),
        "infill_linker_thread_ms": mod_thread.get("com.core.infill-linker", 0),
        "per_layer_module_thread_sum_ms": sum(mod_thread.values()),
        "l2_start_rel_ms": l2_start_rel,
        "l2_end_rel_ms": l2_end_rel,
        "l2_dur_ms": l2_dur_ms,
        "l2_infill_linker_ms": l2_il_ms,
        "last_layer_index": last_layer,
        "second_last_layer_index": second_last,
        "straggler_tail_ms": straggler_tail_ms,
        "last_layer_top_module": last_top_mod,
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
    }


def main() -> int:
    ref_doc = json.loads(REFERENCE.read_text(encoding="utf-8"))
    refs = ref_doc["active_set"]
    ref_bytes = ref_doc["bytes"]
    out_dir = REPO / "target" / "t179-screen-raw"
    out_dir.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, RAYON_NUM_THREADS="12")

    doc = {
        "schema": 1,
        "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "note": (
            "t179 unsandboxed screen only (NOT a formal landing gate). Part 1 runs 1 --instrument-stderr "
            "pass per cell/arm to verify per-layer phase wall, module thread-sums, and L2 straggler tail. "
            "Part 2 runs 1 warmup + 3 interleaved uninstrumented rounds per cell with GetProcessTimes CPU "
            "and byte-identical SHA-256 verification against t175/reference.json."
        ),
        "exe_sha256": sha256_file(EXE),
        "arm_wasm_sha256": {
            arm: {
                "tree-support.wasm": sha256_file(mdir / "tree-support" / "tree-support.wasm"),
                "infill-linker.wasm": sha256_file(mdir / "infill-linker" / "infill-linker.wasm"),
            }
            for arm, mdir in ARMS.items()
        },
        "reference_artifact": REFERENCE.relative_to(REPO).as_posix(),
        "instrumented": {},
        "uninstrumented_wall": {},
    }

    failures = 0

    print("=== PART 1: UNSANDBOXED INSTRUMENTED PHASE/STRAGGLER SCREEN ===", flush=True)
    for cell, arm_list in CELL_ARMS_INSTR.items():
        fixture, gen, sup = cell.split("-", 2)
        cfg = CONFIGS / f"pnp-{gen}-supports-{sup}.json"
        doc["instrumented"][cell] = {}
        for arm in arm_list:
            gc = out_dir / f"instr-{cell}-{arm}.gcode"
            err = out_dir / f"instr-{cell}-{arm}.jsonl"
            args = [
                str(EXE), "slice",
                "--model", str(FIXTURES[fixture]),
                "--output", str(gc),
                "--config", str(cfg),
                "--module-dir", str(ARMS[arm]),
                "--no-default-module-paths",
                "--instrument-stderr",
            ]
            rc, wall, cpu = run_with_cpu(args, env, err)
            sha = sha256_file(gc) if gc.exists() else ""
            sz = gc.stat().st_size if gc.exists() else 0
            gc.unlink(missing_ok=True)
            ref_ok = (sha == refs[cell] and sz == ref_bytes[cell])
            parsed = parse_instrumented(err)
            parsed.update({
                "exit_code": rc,
                "wall_seconds": round(wall, 4),
                "cpu_seconds": round(cpu, 4),
                "gcode_sha256": sha,
                "gcode_bytes": sz,
                "reference_match": ref_ok,
            })
            doc["instrumented"][cell][arm] = parsed
            ok = (rc == 0 and ref_ok and parsed["degraded"] is False and parsed["code_1200_count"] == 0)
            if not ok:
                failures += 1
            print(
                f"  INSTR {cell:18s} {arm:8s} ref={ref_ok} total={parsed['slice_complete_ms']}ms "
                f"pl={parsed['per_layer_ms']}ms ts_th={parsed['tree_support_thread_ms']}ms "
                f"il_th={parsed['infill_linker_thread_ms']}ms L2_il={parsed['l2_infill_linker_ms']}ms "
                f"tail={parsed['straggler_tail_ms']}ms(L{parsed['last_layer_index']})",
                flush=True,
            )

    print("=== PART 2: UNSANDBOXED INTERLEAVED WALL/CPU SCREEN (1 warmup + 3 rounds) ===", flush=True)
    for cell, arm_list in CELL_ARMS_WALL.items():
        fixture, gen, sup = cell.split("-", 2)
        cfg = CONFIGS / f"pnp-{gen}-supports-{sup}.json"
        # Warmup on baseline
        gc_w = out_dir / f"warm-{cell}.gcode"
        run_with_cpu(
            [
                str(EXE), "slice",
                "--model", str(FIXTURES[fixture]),
                "--output", str(gc_w),
                "--config", str(cfg),
                "--module-dir", str(ARMS["baseline"]),
                "--no-default-module-paths",
                "--no-progress-events",
            ],
            env,
            None,
        )
        gc_w.unlink(missing_ok=True)

        rows = []
        by_arm_wall: dict[str, list[float]] = {a: [] for a in arm_list}
        by_arm_cpu: dict[str, list[float]] = {a: [] for a in arm_list}
        for r in range(1, 4):
            order = arm_list if (r % 2 == 1) else list(reversed(arm_list))
            for arm in order:
                gc = out_dir / f"wall-{cell}-r{r}-{arm}.gcode"
                args = [
                    str(EXE), "slice",
                    "--model", str(FIXTURES[fixture]),
                    "--output", str(gc),
                    "--config", str(cfg),
                    "--module-dir", str(ARMS[arm]),
                    "--no-default-module-paths",
                    "--no-progress-events",
                ]
                rc, wall, cpu = run_with_cpu(args, env, None)
                sha = sha256_file(gc) if gc.exists() else ""
                sz = gc.stat().st_size if gc.exists() else 0
                gc.unlink(missing_ok=True)
                ref_ok = (sha == refs[cell] and sz == ref_bytes[cell])
                if rc != 0 or not ref_ok:
                    failures += 1
                by_arm_wall[arm].append(wall)
                by_arm_cpu[arm].append(cpu)
                rows.append({
                    "round": r,
                    "arm": arm,
                    "exit_code": rc,
                    "wall_seconds": round(wall, 4),
                    "cpu_seconds": round(cpu, 4),
                    "cpu_over_wall": round(cpu / wall, 3) if wall > 0 else 0.0,
                    "gcode_sha256": sha,
                    "gcode_bytes": sz,
                    "reference_match": ref_ok,
                })

        base_med_w = statistics.median(by_arm_wall["baseline"])
        base_med_c = statistics.median(by_arm_cpu["baseline"])
        summary = {}
        for arm in arm_list:
            mw = statistics.median(by_arm_wall[arm])
            mc = statistics.median(by_arm_cpu[arm])
            dw_pct = 100.0 * (base_med_w - mw) / base_med_w if base_med_w > 0 else 0.0
            dc_pct = 100.0 * (base_med_c - mc) / base_med_c if base_med_c > 0 else 0.0
            summary[arm] = {
                "median_wall_seconds": round(mw, 4),
                "median_cpu_seconds": round(mc, 4),
                "median_cpu_over_wall": round(mc / mw, 3) if mw > 0 else 0.0,
                "wall_runs": [round(x, 4) for x in by_arm_wall[arm]],
                "cpu_runs": [round(x, 4) for x in by_arm_cpu[arm]],
                "wall_saved_pct_vs_baseline": round(dw_pct, 2),
                "cpu_saved_pct_vs_baseline": round(dc_pct, 2),
                "wall_saved_ms_vs_baseline": round((base_med_w - mw) * 1000.0, 1),
            }
            print(
                f"  WALL  {cell:18s} {arm:8s} med={mw:7.3f}s (dW={dw_pct:+6.2f}% / {(base_med_w-mw)*1000:+7.1f}ms) "
                f"cpu={mc:7.2f}s (dC={dc_pct:+6.2f}%) c/w={mc/mw:.2f}x",
                flush=True,
            )

        doc["uninstrumented_wall"][cell] = {"summary": summary, "rows": rows}

    out_path = HERE / "reduced" / "screen_runs.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(doc, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {out_path} (failures={failures})", flush=True)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())

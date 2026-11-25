#!/usr/bin/env python3
"""t187 tree-support-planner residual attribution — probe capture runner.

Captures:
  1. Clean wall passes with T187_PROBE_OUT (no --profile, no --instrument-stderr):
     3 passes each on benchy-classic-on, base-classic-on;
     1 pass each on benchy-arachne-on, base-arachne-on (classic == arachne invariance).
  2. Deterministic fuel passes (--profile + T187_PROBE_OUT):
     1 pass each on benchy-classic-on, benchy-arachne-on, base-classic-on, base-arachne-on.
  3. Qualification passes (--instrument-stderr, T187_PROBE_OUT unset):
     1 pass each on all 4 supports-on cells (degraded=false, fatal=0, non_fatal=0, code-1200=0).

Every single run is verified byte-identical (SHA-256 + bytes) against t182's active reference.json.
"""
from __future__ import annotations

import datetime
import hashlib
import json
import os
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
EXE = REPO / "target" / "release" / "pnp_cli.exe"
MODULES = REPO / "modules" / "core-modules"
CONFIGS = HERE.parent / "matched-pair" / "configs"
FIXTURES = {
    "benchy": REPO / "tmp" / "3dbenchy.stl",
    "base": REPO / "tmp" / "base.stl",
}
REFERENCE = HERE.parent / "t182-spatial-hierarchical-phase3-union" / "reference.json"
OUT_DIR = REPO / "target" / "t187-probe"


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
    sg_stage_ms = None
    tsp_mod_ms = None
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
            elif ev == "stage_complete" and obj.get("stage_id") == "PrePass::SupportGeometry":
                sg_stage_ms = obj.get("duration_ms")
            elif ev == "module_complete" and obj.get("module_id") == "com.core.tree-support-planner":
                tsp_mod_ms = obj.get("duration_ms")
    return {
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
        "slice_complete_ms": slice_complete_ms,
        "sg_stage_ms": sg_stage_ms,
        "tsp_mod_ms": tsp_mod_ms,
    }


def run_one(cell: str, kind: str, idx: int, refs: dict, ref_bytes: dict) -> dict:
    fixture, generator, supports = cell.split("-", 2)
    cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
    gcode = OUT_DIR / f"{cell}.{kind}.p{idx}.gcode"
    stderr_path = OUT_DIR / f"{cell}.{kind}.p{idx}.stderr.txt"
    probe_json = OUT_DIR / f"{cell}.{kind}.p{idx}.probe.json"
    if probe_json.exists():
        probe_json.unlink()

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
        str(MODULES),
        "--no-default-module-paths",
    ]
    env = dict(os.environ, RAYON_NUM_THREADS="12")
    if kind == "wall":
        env["T187_PROBE_OUT"] = str(probe_json)
    elif kind == "fuel":
        env["T187_PROBE_OUT"] = str(probe_json)
        args.append("--profile")
    elif kind == "qual":
        env.pop("T187_PROBE_OUT", None)
        args.append("--instrument-stderr")

    t0 = time.perf_counter()
    with stderr_path.open("wb") as errf:
        rc = subprocess.run(args, stdout=subprocess.DEVNULL, stderr=errf, env=env).returncode
    wall_s = time.perf_counter() - t0
    got_sha = sha256_file(gcode) if gcode.exists() else ""
    got_bytes = gcode.stat().st_size if gcode.exists() else 0
    ok_ref = got_sha == refs[cell] and got_bytes == ref_bytes[cell]
    row = {
        "cell": cell,
        "kind": kind,
        "pass": idx,
        "exit_code": rc,
        "wall_s": round(wall_s, 4),
        "gcode_sha256": got_sha,
        "gcode_bytes": got_bytes,
        "reference_match": ok_ref,
        "probe_file": str(probe_json.name) if probe_json.exists() else None,
    }
    if kind == "qual":
        row["qual"] = inspect_jsonl(stderr_path)
    status = "OK" if (rc == 0 and ok_ref) else "FAIL"
    print(f"[{status}] {cell:20s} {kind:4s} p{idx} wall={wall_s:7.3f}s bytes={got_bytes} ref={ok_ref}", flush=True)
    return row


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    ref_doc = json.loads(REFERENCE.read_text(encoding="utf-8"))
    refs = ref_doc["active_set"]
    ref_bytes = ref_doc["bytes"]

    rows = []
    # Warmup (discarded) on benchy-classic-on
    _ = run_one("benchy-classic-on", "wall", 0, refs, ref_bytes)

    # 1. Clean wall passes (3 each on classic-on, 1 each on arachne-on)
    for p in (1, 2, 3):
        rows.append(run_one("benchy-classic-on", "wall", p, refs, ref_bytes))
    rows.append(run_one("benchy-arachne-on", "wall", 1, refs, ref_bytes))
    for p in (1, 2, 3):
        rows.append(run_one("base-classic-on", "wall", p, refs, ref_bytes))
    rows.append(run_one("base-arachne-on", "wall", 1, refs, ref_bytes))

    # 2. Deterministic fuel passes (--profile)
    for cell in ("benchy-classic-on", "benchy-arachne-on", "base-classic-on", "base-arachne-on"):
        rows.append(run_one(cell, "fuel", 1, refs, ref_bytes))

    # 3. Qualification passes (--instrument-stderr)
    for cell in ("benchy-classic-on", "benchy-arachne-on", "base-classic-on", "base-arachne-on"):
        rows.append(run_one(cell, "qual", 1, refs, ref_bytes))

    manifest = {
        "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "exe_sha256": sha256_file(EXE),
        "planner_wasm_sha256": sha256_file(MODULES / "tree-support-planner" / "tree-support-planner.wasm"),
        "rows": rows,
    }
    (OUT_DIR / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    failures = sum(1 for r in rows if r["exit_code"] != 0 or not r["reference_match"])
    print(f"DONE: {len(rows)} runs, {failures} failures", flush=True)
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())

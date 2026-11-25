#!/usr/bin/env python3
"""t178 Phase-3 batched union capture runner.

Runs:
  - Qualification + binary corpus dump pass (with `--instrument-stderr`, checking
    exit=0, degraded=false, fatal=0, non_fatal=0, code-1200=0, and byte-identical
    G-code SHA-256 + bytes against t175's active reference.json).
  - Clean uninstrumented Phase-3 timing passes (no dump I/O, no stderr event
    stream; only in-memory per-request clip/total timers + batch wall written
    outside the batch timer, verifying exit=0 and byte-identical G-code).

Attribution & screen only (probe is reverted before ticket closure).
"""

from __future__ import annotations

import argparse
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
REFERENCE = HERE.parent / "t175-tiled-collision-carve" / "reference.json"
DEFAULT_CELLS = [
    "benchy-classic-on",
    "base-classic-on",
    "benchy-arachne-on",
    "base-arachne-on",
]
DUMP_CELLS = {"benchy-classic-on", "base-classic-on"}


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
    support_geom_ms = None
    planner_ms = None
    with raw.open("r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if (
                '"code":1200' in line
                or '"code": 1200' in line
                or "exact-Z occupancy" in line
            ):
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
            elif ev == "stage_complete" and obj.get("stage") == "PrePass::SupportGeometry":
                support_geom_ms = obj.get("elapsed_ms")
            elif (
                ev == "module_complete"
                and obj.get("module_id") == "com.core.tree-support-planner"
            ):
                planner_ms = obj.get("elapsed_ms")
    return {
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
        "slice_complete_ms": slice_complete_ms,
        "support_geometry_ms": support_geom_ms,
        "planner_ms": planner_ms,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cells", default=",".join(DEFAULT_CELLS))
    ap.add_argument("--timing-passes", type=int, default=3)
    ap.add_argument("--out-dir", default=str(REPO / "target" / "t178-capture"))
    ap.add_argument("--skip-dump", action="store_true")
    a = ap.parse_args()

    ref_doc = json.loads(REFERENCE.read_text(encoding="utf-8"))
    refs = ref_doc["active_set"]
    ref_bytes = ref_doc["bytes"]

    out_dir = Path(a.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    runs = []
    failures = 0
    cells = [c.strip() for c in a.cells.split(",") if c.strip()]

    for cell in cells:
        fixture, generator, supports = cell.split("-", 2)
        cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"

        # 1) Qualification + optional binary dump pass
        gcode_q = out_dir / f"{cell}.qual.gcode"
        stderr_q = out_dir / f"{cell}.qual.stderr.jsonl"
        timing_q = out_dir / f"{cell}.qual.phase3.jsonl"
        if timing_q.exists():
            timing_q.unlink()
        dump_path = out_dir / f"{cell}.phase3.bin"
        do_dump = (not a.skip_dump) and (cell in DUMP_CELLS)
        if do_dump and dump_path.exists():
            dump_path.unlink()

        env = dict(os.environ, RAYON_NUM_THREADS="12", PNP_T178_TIMING_OUT=str(timing_q))
        if do_dump:
            env["PNP_T178_DUMP_OUT"] = str(dump_path)
        else:
            env.pop("PNP_T178_DUMP_OUT", None)

        args = [
            str(EXE),
            "slice",
            "--model",
            str(FIXTURES[fixture]),
            "--output",
            str(gcode_q),
            "--config",
            str(cfg),
            "--module-dir",
            str(MODULES),
            "--no-default-module-paths",
            "--instrument-stderr",
        ]
        t0 = time.perf_counter()
        with stderr_q.open("wb") as errf:
            rc = subprocess.run(args, stdout=subprocess.DEVNULL, stderr=errf, env=env).returncode
        wall = time.perf_counter() - t0
        got_sha = sha256_file(gcode_q) if gcode_q.exists() else ""
        got_bytes = gcode_q.stat().st_size if gcode_q.exists() else 0
        diag = inspect_jsonl(stderr_q)
        ok_ref = (got_sha == refs[cell]) and (got_bytes == ref_bytes[cell])
        ok = (
            rc == 0
            and ok_ref
            and diag["degraded"] is False
            and diag["fatal_error_count"] == 0
            and diag["non_fatal_error_count"] == 0
            and diag["code_1200_count"] == 0
        )
        if not ok:
            failures += 1
        dump_bytes = dump_path.stat().st_size if do_dump and dump_path.exists() else 0
        row = {
            "cell": cell,
            "kind": "qual_dump" if do_dump else "qual",
            "pass": 0,
            "exit_code": rc,
            "wall_s": round(wall, 4),
            "gcode_sha256": got_sha,
            "gcode_bytes": got_bytes,
            "matches_t175_ref": ok_ref,
            "dump_bytes": dump_bytes,
            **diag,
            "ok": ok,
        }
        runs.append(row)
        print(
            f"[qual] {cell}: rc={rc} wall={wall:.3f}s ref={ok_ref} "
            f"degraded={diag['degraded']} code1200={diag['code_1200_count']} "
            f"sg={diag['support_geometry_ms']}ms planner={diag['planner_ms']}ms "
            f"dump={dump_bytes/(1<<20):.1f}MiB",
            flush=True,
        )

        # 2) Clean uninstrumented timing passes (classic cells + arachne single check)
        n_passes = a.timing_passes if cell in DUMP_CELLS else 1
        for p in range(1, n_passes + 1):
            gcode_t = out_dir / f"{cell}.t{p}.gcode"
            timing_t = out_dir / f"{cell}.t{p}.phase3.jsonl"
            if timing_t.exists():
                timing_t.unlink()
            env_t = dict(os.environ, RAYON_NUM_THREADS="12", PNP_T178_TIMING_OUT=str(timing_t))
            env_t.pop("PNP_T178_DUMP_OUT", None)
            args_t = [
                str(EXE),
                "slice",
                "--model",
                str(FIXTURES[fixture]),
                "--output",
                str(gcode_t),
                "--config",
                str(cfg),
                "--module-dir",
                str(MODULES),
                "--no-default-module-paths",
            ]
            t0 = time.perf_counter()
            rc = subprocess.run(
                args_t, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=env_t
            ).returncode
            wall = time.perf_counter() - t0
            got_sha = sha256_file(gcode_t) if gcode_t.exists() else ""
            got_bytes = gcode_t.stat().st_size if gcode_t.exists() else 0
            ok_ref = (got_sha == refs[cell]) and (got_bytes == ref_bytes[cell])
            ok = rc == 0 and ok_ref
            if not ok:
                failures += 1
            row = {
                "cell": cell,
                "kind": "timing",
                "pass": p,
                "exit_code": rc,
                "wall_s": round(wall, 4),
                "gcode_sha256": got_sha,
                "gcode_bytes": got_bytes,
                "matches_t175_ref": ok_ref,
                "ok": ok,
            }
            runs.append(row)
            print(
                f"[time] {cell} pass {p}/{n_passes}: rc={rc} wall={wall:.3f}s ref={ok_ref}",
                flush=True,
            )

    summary = {
        "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "exe_sha256": sha256_file(EXE),
        "planner_wasm_sha256": sha256_file(
            MODULES / "tree-support-planner" / "tree-support-planner.wasm"
        ),
        "rayon_num_threads": 12,
        "failures": failures,
        "runs": runs,
    }
    (out_dir / "capture_runs.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    print(f"DONE: {len(runs)} runs, failures={failures}", flush=True)
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())

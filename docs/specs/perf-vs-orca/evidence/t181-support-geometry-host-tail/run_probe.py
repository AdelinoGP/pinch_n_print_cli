#!/usr/bin/env python3
"""t181 SupportGeometry post-planner host tail attribution probe runner.

Runs `pnp_cli.exe` with `PNP_T181_PROBE_OUT` set on the matched-job cells,
verifies every output SHA-256 and byte count against `t175/reference.json`,
runs `--instrument-stderr` qualification passes to reconcile against `t177`'s
`stage_complete - tree-support-planner::module_complete` bracket, and runs
separate `PNP_T181_VERIFY_PPO=1` + `PNP_T181_DUMP_IO` passes on
`benchy-classic-on` and `base-classic-on` (never mixed with timing passes).
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

SUPPORTS_ON = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]
SUPPORTS_OFF_CONTROLS = [
    "benchy-classic-off",
    "base-classic-off",
]


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
    sg_stage_start = None
    sg_stage_end = None
    sg_stage_dur = None
    sg_tree_end = None
    sg_tree_dur = None
    sg_trad_dur = None
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
            elif obj.get("stage") == "PrePass::SupportGeometry":
                if ev == "stage_start":
                    sg_stage_start = obj.get("timestamp_ms")
                elif ev == "module_complete" and obj.get("module_id") == "com.core.tree-support-planner":
                    sg_tree_end = obj.get("timestamp_ms")
                    sg_tree_dur = obj.get("elapsed_ms")
                elif ev == "module_complete" and obj.get("module_id") == "com.core.traditional-support-planner":
                    sg_trad_dur = obj.get("elapsed_ms")
                elif ev == "stage_complete" and sg_tree_end is not None:
                    sg_stage_end = obj.get("timestamp_ms")
                    sg_stage_dur = obj.get("duration_ms")
                    if sg_stage_dur is None and sg_stage_start is not None and sg_stage_end is not None:
                        sg_stage_dur = sg_stage_end - sg_stage_start
    tail_ms = (
        max(0, sg_stage_end - sg_tree_end)
        if sg_tree_end is not None and sg_stage_end is not None
        else 0
    )
    return {
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
        "slice_complete_ms": slice_complete_ms,
        "sg_stage_ms": sg_stage_dur,
        "sg_tree_planner_ms": sg_tree_dur,
        "sg_trad_planner_ms": sg_trad_dur,
        "sg_post_planner_tail_ms": tail_ms,
    }


def run_one(
    cell: str,
    tag: str,
    out_dir: Path,
    refs: dict,
    ref_bytes: dict,
    instrument: bool,
    verify_ppo: bool = False,
    dump_io: Path | None = None,
) -> dict:
    fixture, generator, supports = cell.split("-", 2)
    cfg = CONFIGS / f"pnp-{generator}-supports-{supports}.json"
    gcode = out_dir / f"{cell}.{tag}.gcode"
    probe_json = out_dir / f"{cell}.{tag}.probe.json"
    stderr_log = out_dir / f"{cell}.{tag}.stderr.jsonl"
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
    if instrument:
        args.append("--instrument-stderr")

    env = dict(os.environ, RAYON_NUM_THREADS="12", PNP_T181_PROBE_OUT=str(probe_json))
    if verify_ppo:
        env["PNP_T181_VERIFY_PPO"] = "1"
    else:
        env.pop("PNP_T181_VERIFY_PPO", None)
    if dump_io is not None:
        env["PNP_T181_DUMP_IO"] = str(dump_io)
    else:
        env.pop("PNP_T181_DUMP_IO", None)

    t0 = time.perf_counter()
    if instrument:
        with stderr_log.open("wb") as errf:
            proc = subprocess.run(args, env=env, stdout=subprocess.PIPE, stderr=errf, check=False)
    else:
        proc = subprocess.run(args, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    wall_s = time.perf_counter() - t0

    if proc.returncode != 0:
        raise RuntimeError(f"{cell} ({tag}) failed exit={proc.returncode}")

    actual_sha = sha256_file(gcode)
    actual_bytes = gcode.stat().st_size
    if actual_sha != refs[cell] or actual_bytes != ref_bytes[cell]:
        raise RuntimeError(
            f"{cell} ({tag}) hash/bytes mismatch: got {actual_sha} ({actual_bytes}) "
            f"expected {refs[cell]} ({ref_bytes[cell]})"
        )

    probe = json.loads(probe_json.read_text(encoding="utf-8"))
    rec = {
        "cell": cell,
        "tag": tag,
        "instrumented": instrument,
        "verify_ppo": verify_ppo,
        "dump_io": str(dump_io.name) if dump_io is not None else None,
        "exit_code": proc.returncode,
        "wall_s": round(wall_s, 4),
        "sha256": actual_sha,
        "bytes": actual_bytes,
        "matches_reference": True,
        "probe_file": probe_json.name,
        "post_planner_tail_ms": round(probe["post_planner_tail_ns"] / 1e6, 3),
        "stage_total_ms": round(probe["stage_total_ns"] / 1e6, 3),
        "ppo_verify": probe.get("ppo_verify"),
    }
    if instrument:
        qual = inspect_jsonl(stderr_log)
        rec["qual"] = qual
        if (
            qual["degraded"] is not False
            or qual["fatal_error_count"] != 0
            or qual["non_fatal_error_count"] != 0
            or qual["code_1200_count"] != 0
        ):
            raise RuntimeError(f"{cell} ({tag}) qual failed: {qual}")
    print(
        f"[{cell:18s} {tag:8s}] wall={wall_s:6.3f}s stage={rec['stage_total_ms']:8.2f}ms "
        f"tail={rec['post_planner_tail_ms']:7.2f}ms sha={actual_sha[:8]} OK",
        flush=True,
    )
    return rec


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out-dir", default=str(REPO / "target" / "t181-probe"))
    ap.add_argument("--passes", type=int, default=3)
    ap.add_argument("--cells", default=None, help="Comma-separated subset for quick test")
    ap.add_argument("--skip-qual", action="store_true")
    ap.add_argument("--skip-dump", action="store_true")
    args = ap.parse_args()

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    ref_doc = json.loads(REFERENCE.read_text(encoding="utf-8"))
    refs = ref_doc["active_set"]
    ref_bytes = ref_doc["bytes"]

    exe_sha = sha256_file(EXE)
    runs = []

    if args.cells:
        cells = [c.strip() for c in args.cells.split(",") if c.strip()]
        for cell in cells:
            for p in range(1, args.passes + 1):
                runs.append(run_one(cell, f"pass{p}", out_dir, refs, ref_bytes, instrument=False))
    else:
        for cell in SUPPORTS_ON:
            for p in range(1, args.passes + 1):
                runs.append(run_one(cell, f"pass{p}", out_dir, refs, ref_bytes, instrument=False))
        for cell in SUPPORTS_OFF_CONTROLS:
            runs.append(run_one(cell, "ctrl1", out_dir, refs, ref_bytes, instrument=False))
        if not args.skip_qual:
            for cell in SUPPORTS_ON + SUPPORTS_OFF_CONTROLS:
                runs.append(run_one(cell, "qual", out_dir, refs, ref_bytes, instrument=True))
        if not args.skip_dump:
            for cell, fname in [
                ("benchy-classic-on", "benchy.agg_io.postcard"),
                ("base-classic-on", "base.agg_io.postcard"),
            ]:
                runs.append(
                    run_one(
                        cell,
                        "ppo_dump",
                        out_dir,
                        refs,
                        ref_bytes,
                        instrument=False,
                        verify_ppo=True,
                        dump_io=out_dir / fname,
                    )
                )

    summary = {
        "captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "exe_path": str(EXE),
        "exe_sha256": exe_sha,
        "rayon_num_threads": 12,
        "reference": str(REFERENCE.relative_to(REPO)).replace("\\", "/"),
        "runs": runs,
    }
    (out_dir / "probe_runs.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(f"Wrote {out_dir / 'probe_runs.json'} ({len(runs)} runs, all reference-identical)")


if __name__ == "__main__":
    main()

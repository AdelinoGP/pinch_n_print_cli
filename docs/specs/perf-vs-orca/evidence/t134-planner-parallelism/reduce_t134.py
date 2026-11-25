#!/usr/bin/env python3
"""Reduce the t134 probe stderr streams into a checkout-only summary.

The raw streams live under `target/t134/runs/` (gitignored). This script keeps
only the measured counters and phase walls the ticket's claims rest on, so
`verify.py` can run against the committed `reduced/summary.json` alone.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import re
from pathlib import Path

CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
    "base-classic-off",
    "base-classic-on",
    "base-arachne-off",
    "base-arachne-on",
]
PHASES = [
    "p0_volumes",
    "p2_contacts",
    "p3_drop",
    "p5_smooth",
    "p6d_occupancy",
    "p6e_node_roles",
    "p6f_emit_geometry",
    "p6a_clip_core",
]
ROLLUPS = {
    "drop_loop": ["p3_drop"],
    "emit_pass": [
        "p6d_occupancy",
        "p6e_node_roles",
        "p6f_emit_geometry",
        "p6a_clip_core",
    ],
    "pre_loop": ["p0_volumes", "p2_contacts", "p5_smooth"],
}
NUMERIC_KEYS = re.compile(
    r"^(clip|offset|offset2_p1|offset2_p2)_(calls|sweep_ns|in_points|in_paths|max_sweep_ns|max_in_points)$"
    r"|^batch_(serial|parallel)_(calls|requests)$"
)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def parse_guest(message: str) -> dict[str, dict[str, int]]:
    out: dict[str, dict[str, int]] = {}
    for field in message.split()[1:]:
        name, value = field.split("=", 1)
        us, calls = value.split("us/")
        out[name] = {"wall_us": int(us), "spans": int(calls)}
    return out


def parse_cell(path: Path) -> dict:
    stage_walls: dict[str, list[int]] = {}
    module_walls: dict[str, int] = {}
    stage_thread: dict[str, dict[str, int]] = {}
    module_thread: dict[str, dict[str, int]] = {}
    slice_wall_ms = None
    phase_wall: dict[str, dict[str, int]] = {}
    open_spans: dict[str, dict[str, int]] = {}
    phase_thread: dict[str, dict[str, int]] = {}

    def vals(payload: dict) -> dict[str, int]:
        return {k: int(v) for k, v in payload.items() if NUMERIC_KEYS.match(k)}

    def add_into(
        dst: dict[str, dict[str, int]], key: str, payload: dict[str, int]
    ) -> None:
        row = dst.setdefault(key, {})
        for k, v in payload.items():
            row[k] = row.get(k, 0) + v

    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if line.startswith("{"):
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            kind = event.get("event")
            if kind == "stage_complete" and event.get("phase") == "prepass":
                stage_walls.setdefault(event["stage"], []).append(
                    int(event["elapsed_ms"])
                )
            elif kind == "module_complete":
                module_walls[event["module_id"]] = int(event["elapsed_ms"])
            elif kind == "slice_complete":
                slice_wall_ms = int(event["elapsed_ms"])
            elif kind == "module_log" and event.get("message", "").startswith(
                "t134-guest"
            ):
                phase_wall = parse_guest(event["message"])
            continue
        if not line.startswith("t134-snap "):
            continue
        payload = dict(re.findall(r"(\w+)=([^\s]+)", line))
        tag = payload["tag"]
        if tag.startswith("S:"):
            open_spans[tag[2:]] = vals(payload)
        elif tag.startswith("E:"):
            name = tag[2:]
            before = open_spans.pop(name)
            after = vals(payload)
            add_into(
                phase_thread, name, {k: after[k] - before.get(k, 0) for k in after}
            )
        elif tag.startswith("stage:"):
            add_into(stage_thread, tag[6:], vals(payload))
        elif tag.startswith("module:"):
            add_into(module_thread, tag[7:], vals(payload))

    phases: dict[str, dict] = {}
    for name in PHASES:
        thread = phase_thread.get(name, {})
        wall_us = phase_wall.get(name, {}).get("wall_us", 0)
        spans = phase_wall.get(name, {}).get("spans", 0)
        sweep_ms = (
            thread.get("clip_sweep_ns", 0)
            + thread.get("offset_sweep_ns", 0)
            + thread.get("offset2_p1_sweep_ns", 0)
            + thread.get("offset2_p2_sweep_ns", 0)
        ) / 1e6
        calls = sum(
            thread.get(f"{op}_calls", 0)
            for op in ("clip", "offset", "offset2_p1", "offset2_p2")
        )
        wall_ms = wall_us / 1000.0
        phases[name] = {
            "wall_ms": round(wall_ms, 3),
            "thread_ms": round(sweep_ms, 3),
            "calls": calls,
            "spans": spans,
            "parallelism": round(sweep_ms / wall_ms, 3) if wall_ms > 0 else None,
            "guest_cpu": calls == 0,
            "clip_calls": thread.get("clip_calls", 0),
            "offset_calls": thread.get("offset_calls", 0),
            "in_points": thread.get("clip_in_points", 0)
            + thread.get("offset_in_points", 0),
            "batch_parallel_calls": thread.get("batch_parallel_calls", 0),
            "batch_serial_calls": thread.get("batch_serial_calls", 0),
            "batch_parallel_requests": thread.get("batch_parallel_requests", 0),
            "batch_serial_requests": thread.get("batch_serial_requests", 0),
        }

    module_wall_ms = module_walls.get("com.core.tree-support-planner")
    module_thr = module_thread.get("com.core.tree-support-planner", {})
    module_thread_ms = (
        module_thr.get("clip_sweep_ns", 0)
        + module_thr.get("offset_sweep_ns", 0)
        + module_thr.get("offset2_p1_sweep_ns", 0)
        + module_thr.get("offset2_p2_sweep_ns", 0)
    ) / 1e6

    phase_wall_sum = sum(p["wall_ms"] for p in phases.values())
    phase_thread_sum = sum(p["thread_ms"] for p in phases.values())
    rollup = {}
    for name, members in ROLLUPS.items():
        wall = sum(phases[m]["wall_ms"] for m in members)
        thread = sum(phases[m]["thread_ms"] for m in members)
        rollup[name] = {
            "wall_ms": round(wall, 3),
            "thread_ms": round(thread, 3),
            "wall_share": round(wall / module_wall_ms, 4) if module_wall_ms else None,
            "thread_share": round(thread / module_thread_ms, 4)
            if module_thread_ms
            else None,
            "parallelism": round(thread / wall, 3) if wall > 0 else None,
        }
    rollup["residual"] = {
        "wall_ms": round(module_wall_ms - phase_wall_sum, 3)
        if module_wall_ms
        else None,
        "thread_ms": round(module_thread_ms - phase_thread_sum, 3)
        if module_thread_ms
        else None,
    }

    census = []
    for stage, walls in stage_walls.items():
        entry = {
            "stage": stage,
            "wall_ms": max(walls),
            "stage_events_ms": walls,
            "thread_ms": round(
                sum(
                    row.get("clip_sweep_ns", 0)
                    + row.get("offset_sweep_ns", 0)
                    + row.get("offset2_p1_sweep_ns", 0)
                    + row.get("offset2_p2_sweep_ns", 0)
                    for row in [stage_thread.get(stage, {})]
                )
                / 1e6,
                3,
            ),
            "calls": sum(
                stage_thread.get(stage, {}).get(f"{op}_calls", 0)
                for op in ("clip", "offset", "offset2_p1", "offset2_p2")
            ),
        }
        entry["parallelism"] = (
            round(entry["thread_ms"] / entry["wall_ms"], 3)
            if entry["wall_ms"] > 0
            else None
        )
        if stage == "PrePass::SupportGeometry":
            entry["producer_wall_ms"] = walls[0]
            entry["planner_stage_wall_ms"] = walls[-1]
            entry["planner_module_wall_ms"] = module_wall_ms
            entry["planner_module_thread_ms"] = round(module_thread_ms, 3)
            entry["planner_module_parallelism"] = (
                round(module_thread_ms / module_wall_ms, 3) if module_wall_ms else None
            )
        census.append(entry)

    prepass_ms = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if '"event":"phase_complete"' in line:
            try:
                e = json.loads(line)
            except json.JSONDecodeError:
                continue
            if e.get("phase") == "prepass":
                prepass_ms = int(e["elapsed_ms"])

    return {
        "slice_wall_ms": slice_wall_ms,
        "prepass_ms": prepass_ms,
        "planner": {
            "module_wall_ms": module_wall_ms,
            "thread_ms": round(module_thread_ms, 3),
            "parallelism": round(module_thread_ms / module_wall_ms, 3)
            if module_wall_ms
            else None,
            "phase_wall_sum_ms": round(phase_wall_sum, 3),
            "phase_thread_sum_ms": round(phase_thread_sum, 3),
            "phases": phases,
            "rollup": rollup,
        },
        "stage_census": census,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--repo-root", type=Path, default=Path(__file__).resolve().parents[5]
    )
    ap.add_argument("--runs", type=Path, default=None)
    ap.add_argument("--out", type=Path, default=None)
    args = ap.parse_args()
    root = args.repo_root.resolve()
    runs = args.runs or root / "target" / "t134" / "runs"
    out = args.out or Path(__file__).resolve().parent / "reduced" / "summary.json"
    out.parent.mkdir(parents=True, exist_ok=True)

    rows = {}
    rows_path = runs / "rows.csv"
    if rows_path.exists():
        with rows_path.open(encoding="utf-8") as f:
            for row in csv.DictReader(f):
                rows[row["cell"]] = row

    cells = {}
    for cell in CELLS:
        stream = runs / f"{cell}.stderr.jsonl"
        if not stream.exists():
            raise SystemExit(f"missing stream: {stream}")
        parsed = parse_cell(stream)
        row = rows.get(cell, {})
        parsed["run"] = {
            "wall_seconds": float(row["wall_seconds"])
            if row.get("wall_seconds")
            else None,
            "gcode_bytes": int(row["gcode_bytes"]) if row.get("gcode_bytes") else None,
            "sha256": row.get("sha256"),
            "exit_code": int(row["exit_code"]) if row.get("exit_code") else None,
        }
        cells[cell] = parsed

    exe = root / "target" / "release" / "pnp_cli.exe"
    planner_wasm = (
        root
        / "modules"
        / "core-modules"
        / "tree-support-planner"
        / "tree-support-planner.wasm"
    )

    repeat_dir = root / "target" / "t134-runs"
    repeats = {}
    for cell in ("base-classic-on", "benchy-classic-on"):
        paths = [runs / f"{cell}.stderr.jsonl"]
        if repeat_dir.exists():
            paths.extend(sorted(repeat_dir.glob(f"{cell}*.stderr.jsonl")))
        seen = set()
        entries = []
        for path in paths:
            if not path.exists():
                continue
            key = str(path.resolve())
            if key in seen:
                continue
            seen.add(key)
            parsed = parse_cell(path)
            entries.append(
                {
                    "source": str(path.relative_to(root)).replace("\\", "/"),
                    "module_wall_ms": parsed["planner"]["module_wall_ms"],
                    "thread_ms": parsed["planner"]["thread_ms"],
                    "p6d_wall_ms": parsed["planner"]["phases"]["p6d_occupancy"][
                        "wall_ms"
                    ],
                    "p6a_wall_ms": parsed["planner"]["phases"]["p6a_clip_core"][
                        "wall_ms"
                    ],
                }
            )
        walls = [e["module_wall_ms"] for e in entries if e["module_wall_ms"]]
        repeats[cell] = {
            "passes": entries,
            "module_wall_spread_pct": round(
                (max(walls) - min(walls)) / min(walls) * 100.0, 2
            )
            if len(walls) > 1
            else 0.0,
        }

    summary = {
        "take": "t134",
        "probe_env": {"PNP_T134_PROBE": "1", "RAYON_NUM_THREADS": 12},
        "raw_source": "target/t134/runs/*.stderr.jsonl (gitignored)",
        "reference_set": "ticket 123 candidate-ordinary",
        "binary": {
            "pnp_cli_sha256": sha256(exe) if exe.exists() else None,
            "planner_wasm_sha256": sha256(planner_wasm)
            if planner_wasm.exists()
            else None,
        },
        "cells": cells,
        "repeatability": repeats,
    }
    out.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {out}")
    for cell, data in cells.items():
        p = data["planner"]
        print(
            f"{cell:22s} slice={data['slice_wall_ms']} planner={p['module_wall_ms']}ms "
            f"thread={p['thread_ms']:.1f}ms par={p['parallelism']}x"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

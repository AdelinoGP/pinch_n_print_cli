#!/usr/bin/env python3
"""t136 clip-batch fan-out reducer: probe stderr -> reduced/summary.json.

Attribution only. The instrumented walls are not wall claims; every number is
either a probe-counter sum or a ratio of one.

Two line families are read from the probe build's stderr:

    t136-batch   one per `clip_polygons_batch` host call
    t136-segment one per guest phase segment, carrying the guest's own wall
    t136-chunk   one per SDK call, carrying marshal+call+decode wall

The segment line is the unit of decomposition: it carries the guest span around
the segment and the summed host-call counters for every call issued inside it,
so `guest - host`, `host - ideal` and `sdk - host` are exact complements.

The additive split of the clip-core wall is:

    guest_wall = host_wall + marshal_decode + guest_cpu
    host_wall  = work_floor + tail_chunking + dispatch_excess

where `work_floor` is `thread / threads` (what 12 perfect workers need for the
same request set), `tail_chunking` is `sum(ideal) - work_floor` (the part the
current call boundaries make unavoidable because each call pays its own longest
request), and `dispatch_excess` is `host_wall - sum(ideal)` (what even the
current chunking should not have paid: rayon wake-up, imbalance, per-call
floor). `ideal` per call is `max(thread/12, max request)`.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

SEGMENT_FIELDS = (
    "name",
    "calls",
    "requests",
    "vertices",
    "wall_sum_ns",
    "thread_ns",
    "max_ns",
    "ideal_ns",
    "guest_wall_ns",
)
BATCH_FIELDS = (
    "phase",
    "seq",
    "requests",
    "mode",
    "vertices",
    "wall_ns",
    "thread_ns",
    "max_ns",
    "ideal_ns",
)
CHUNK_FIELDS = ("phase", "idx", "requests", "sdk_ns")


def parse_kv(line: str, fields: tuple[str, ...]) -> dict | None:
    out: dict[str, object] = {}
    for field in fields:
        match = re.search(rf"(?:^| ){field}=(\S+)", line)
        if match is None:
            return None
        out[field] = match.group(1)
    for field in fields:
        if field not in ("name", "phase", "mode"):
            out[field] = int(out[field])  # type: ignore[arg-type]
    return out


def parse_stderr(path: Path) -> dict:
    segments: list[dict] = []
    batches: list[dict] = []
    chunks: list[dict] = []
    with path.open("r", encoding="utf-8", errors="replace") as f:
        for raw in f:
            if raw.startswith("t136-segment "):
                row = parse_kv(raw[len("t136-segment ") :].rstrip("\n"), SEGMENT_FIELDS)
                if row is not None:
                    segments.append(row)
            elif raw.startswith("t136-batch "):
                row = parse_kv(raw[len("t136-batch ") :].rstrip("\n"), BATCH_FIELDS)
                if row is not None:
                    batches.append(row)
            elif raw.startswith("t136-chunk "):
                row = parse_kv(raw[len("t136-chunk ") :].rstrip("\n"), CHUNK_FIELDS)
                if row is not None:
                    chunks.append(row)
    return {"segments": segments, "batches": batches, "chunks": chunks}


def quantiles(values: list[int]) -> dict:
    if not values:
        return {"n": 0}
    ordered = sorted(values)
    n = len(ordered)

    def at(q: float) -> int:
        return ordered[min(n - 1, int(round(q * (n - 1))))]

    return {
        "n": n,
        "min": ordered[0],
        "p50": at(0.50),
        "p90": at(0.90),
        "p99": at(0.99),
        "max": ordered[-1],
        "sum": sum(ordered),
    }


def merge_floor(series: list[tuple[int, int]], group: int, threads: int) -> int:
    """Sum of `max(thread/threads, longest request)` over merged groups.

    `series` is `(thread_ns, max_request_ns)` per segment in stream order.
    `group` is how many consecutive segments are merged into one batch. This is
    a lower bound for the merged wall (LPT achieves within 4/3 of it).
    """
    total = 0
    for start in range(0, len(series), group):
        chunk = series[start : start + group]
        thread = sum(t for t, _ in chunk)
        longest = max(m for _, m in chunk)
        total += max(thread // threads, longest)
    return total


def summarise(parsed: dict, threads: int) -> dict:
    # Per-chunk segment series, in stream order, keyed by phase name.
    series: dict[str, list[tuple[int, int]]] = defaultdict(list)
    per_name: dict[str, dict] = defaultdict(
        lambda: {
            "segments": 0,
            "calls": 0,
            "requests": 0,
            "vertices": 0,
            "host_wall_ns": 0,
            "thread_ns": 0,
            "ideal_ns": 0,
            "max_request_ns": 0,
            "guest_wall_ns": 0,
            "sdk_ns": 0,
            "serial_calls": 0,
            "parallel_calls": 0,
            "serial_wall_ns": 0,
            "parallel_wall_ns": 0,
            "serial_requests": 0,
            "tail_dominated_calls": 0,
        }
    )
    call_walls: dict[str, list[int]] = defaultdict(list)
    call_requests: dict[str, list[int]] = defaultdict(list)

    for seg in parsed["segments"]:
        name = seg["name"]
        acc = per_name[name]
        acc["segments"] += 1
        for field in (
            "calls",
            "requests",
            "vertices",
            "thread_ns",
            "ideal_ns",
            "guest_wall_ns",
        ):
            acc[field] += seg[field]
        acc["host_wall_ns"] += seg["wall_sum_ns"]
        acc["max_request_ns"] = max(acc["max_request_ns"], seg["max_ns"])
        series[name].append((seg["thread_ns"], seg["max_ns"]))

    for chunk in parsed["chunks"]:
        if chunk["phase"] in per_name:
            per_name[chunk["phase"]]["sdk_ns"] += chunk["sdk_ns"]

    for call in parsed["batches"]:
        phase = call["phase"]
        if phase not in per_name:
            continue
        acc = per_name[phase]
        if call["mode"] == "Serial":
            acc["serial_calls"] += 1
            acc["serial_wall_ns"] += call["wall_ns"]
            acc["serial_requests"] += call["requests"]
        else:
            acc["parallel_calls"] += 1
            acc["parallel_wall_ns"] += call["wall_ns"]
        if call["thread_ns"] and call["max_ns"] * 2 >= call["thread_ns"]:
            acc["tail_dominated_calls"] += 1
        call_walls[phase].append(call["wall_ns"])
        call_requests[phase].append(call["requests"])

    for name, acc in per_name.items():
        acc["guest_serial_ns"] = acc["guest_wall_ns"] - acc["host_wall_ns"]
        acc["marshal_decode_ns"] = acc["sdk_ns"] - acc["host_wall_ns"]
        acc["guest_cpu_ns"] = acc["guest_wall_ns"] - acc["sdk_ns"]
        acc["dispatch_excess_ns"] = acc["host_wall_ns"] - acc["ideal_ns"]
        acc["work_floor_ns"] = acc["thread_ns"] // threads
        acc["tail_chunking_ns"] = acc["ideal_ns"] - acc["work_floor_ns"]
        acc["host_fanout"] = (
            acc["thread_ns"] / acc["host_wall_ns"] if acc["host_wall_ns"] else None
        )
        acc["end_to_end_fanout"] = (
            acc["thread_ns"] / acc["guest_wall_ns"] if acc["guest_wall_ns"] else None
        )
        acc["chunks"] = len(series[name])
        acc["merged_floor_ns"] = merge_floor(series[name], 1, threads)
        acc["merge2_floor_ns"] = merge_floor(series[name], 2, threads)
        acc["merge4_floor_ns"] = merge_floor(series[name], 4, threads)
        acc["phase_merged_floor_ns"] = (
            max(acc["thread_ns"] // threads, acc["max_request_ns"])
            if acc["calls"]
            else 0
        )
        acc["call_wall_quantiles"] = quantiles(call_walls[name])
        acc["call_requests_quantiles"] = quantiles(call_requests[name])

    planner = {k: v for k, v in per_name.items() if k not in ("none", "end")}
    totals: dict[str, object] = {
        name: sum(acc[name] for acc in planner.values())
        for name in (
            "guest_wall_ns",
            "host_wall_ns",
            "thread_ns",
            "ideal_ns",
            "sdk_ns",
            "requests",
            "vertices",
            "segments",
        )
    }
    totals["calls"] = sum(acc["calls"] for acc in planner.values())
    totals["guest_serial_ns"] = totals["guest_wall_ns"] - totals["host_wall_ns"]
    totals["marshal_decode_ns"] = totals["sdk_ns"] - totals["host_wall_ns"]
    totals["guest_cpu_ns"] = totals["guest_wall_ns"] - totals["sdk_ns"]
    totals["dispatch_excess_ns"] = totals["host_wall_ns"] - totals["ideal_ns"]
    totals["work_floor_ns"] = totals["thread_ns"] // threads
    totals["tail_chunking_ns"] = totals["ideal_ns"] - totals["work_floor_ns"]
    totals["host_fanout"] = totals["thread_ns"] / totals["host_wall_ns"]
    totals["end_to_end_fanout"] = totals["thread_ns"] / totals["guest_wall_ns"]
    totals["serial_calls"] = sum(acc["serial_calls"] for acc in planner.values())
    totals["parallel_calls"] = sum(acc["parallel_calls"] for acc in planner.values())
    totals["serial_wall_ns"] = sum(acc["serial_wall_ns"] for acc in planner.values())
    totals["serial_requests"] = sum(acc["serial_requests"] for acc in planner.values())
    totals["parallel_wall_ns"] = sum(acc["parallel_wall_ns"] for acc in planner.values())
    totals["serial_wall_share"] = totals["serial_wall_ns"] / totals["host_wall_ns"]
    totals["tail_dominated_calls"] = sum(
        acc["tail_dominated_calls"] for acc in planner.values()
    )
    totals["threads"] = threads
    totals["host_wall_share_of_guest"] = (
        totals["host_wall_ns"] / totals["guest_wall_ns"]
    )

    other = [b for b in parsed["batches"] if b["phase"] not in per_name]
    other_summary = {
        "calls": len(other),
        "requests": sum(b["requests"] for b in other),
        "thread_ns": sum(b["thread_ns"] for b in other),
        "wall_ns": sum(b["wall_ns"] for b in other),
        "max_ns": max((b["max_ns"] for b in other), default=0),
    }

    return {
        "phases": dict(sorted(per_name.items())),
        "totals": totals,
        "other_modules": other_summary,
        # Compact so the in-repo set stays small: "thread:max;thread:max;...".
        # Only phases that issued calls carry a series; the guest-only phases
        # have no requests to merge.
        "chunk_series": {
            name: ";".join(f"{thread}:{longest}" for thread, longest in rows)
            for name, rows in sorted(series.items())
            if any(thread or longest for thread, longest in rows)
        },
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--repo-root", type=Path, default=Path(__file__).resolve().parents[5]
    )
    ap.add_argument("--run-dir", type=Path, default=None)
    ap.add_argument("--out", type=Path, default=None)
    args = ap.parse_args()

    root = args.repo_root.resolve()
    run_dir = args.run_dir or (root / "target" / "t136" / "campaign")
    if not run_dir.is_absolute():
        run_dir = root / run_dir
    out = args.out or (Path(__file__).resolve().parent / "reduced" / "summary.json")
    rows_path = run_dir / "rows.csv"
    if not rows_path.exists():
        print(f"missing {rows_path}", file=sys.stderr)
        return 2

    passes: dict[str, list[dict]] = defaultdict(list)
    with rows_path.open(newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            stderr = run_dir / row["stderr"]
            if not stderr.exists():
                print(f"missing {stderr}", file=sys.stderr)
                return 2
            parsed = parse_stderr(stderr)
            passes[row["cell"]].append(
                {
                    "pass": int(row["pass"]),
                    "wall_seconds": float(row["wall_seconds"]),
                    "sha256": row["sha256"],
                    "gcode_bytes": int(row["gcode_bytes"]),
                    "threads": int(row["threads"]),
                    "probe": summarise(parsed, int(row["threads"])),
                }
            )

    summary = {
        "take": "t136 planner clip-batch fan-out",
        "probe_env": "PNP_T136_PROBE=1",
        "note": (
            "Instrumented attribution run: the walls here are probe passes, not "
            "benchmark cells. Every field is a probe-counter sum or a ratio of "
            "one. `chunk_series` holds each phase's per-chunk "
            "`thread_ns:max_request_ns` list so the merge floors are "
            "re-derivable."
        ),
        "cells": {
            cell: {"passes": sorted(rows, key=lambda e: e["pass"])}
            for cell, rows in sorted(passes.items())
        },
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

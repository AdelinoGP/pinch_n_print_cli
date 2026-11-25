#!/usr/bin/env python3
"""t155 -- reduce the temporary probe streams into the committed table set.

Reads the raw `--instrument-stderr` streams captured under
`target/t155-probe/` (gitignored) plus the matching G-code, and writes
`probe.json`.

Two families of facts are reduced:

1. **Ordinary instrumented events** (`module_start` / `module_complete` /
   `phase_*` / `slice_complete`): the pass's instrumented wall and phase walls,
   validation flags, and the per-module bracket sums used to price the block.
2. **Probe rows** (`event == "probe"`, only emitted when `PNP_T155_PROBE=1`):
   host-side site deltas inside each module's bracket, the sub-segments of
   `prepare_regions`, the `derive_needs_support` call census, and the
   fill-sync read-field audit.

Probe rows are attribution only. No number here is a wall claim.

The last run for each cell is the one reduced into `cells[<cell>]`; earlier
runs of the same cell are kept in that cell's `runs` list so the run-to-run
spread stays visible.

Usage:
  python reduce.py [--raw <dir>] [--out probe.json]
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
DEFAULT_RAW = REPO / "target" / "t155-probe"
PREPROBE = REPO / "target" / "t155-instrumented"

# (cell, tag, raw stem)
RUNS = [
    ("base-classic-off", "preprobe", PREPROBE / "base-classic-off"),
    ("base-classic-off", "probe-v1", DEFAULT_RAW / "base-classic-off"),
    ("base-classic-off", "probe-v2", DEFAULT_RAW / "base-classic-off-sub"),
    ("base-classic-off", "probe-v3", DEFAULT_RAW / "base-classic-off-ns"),
    ("base-classic-off-decline", "decline", DEFAULT_RAW / "base-classic-off-decline"),
    ("base-classic-on", "probe-v1", DEFAULT_RAW / "base-classic-on"),
    ("base-classic-on", "probe-v2", DEFAULT_RAW / "base-classic-on-sub"),
    ("base-classic-on", "probe-v3", DEFAULT_RAW / "base-classic-on-ns"),
]

FOCUS_MODULES = {
    ("Layer::Perimeters", "com.core.classic-perimeters"),
    ("Layer::Infill", "com.core.raft-default"),
    ("Layer::Infill", "com.core.gyroid-infill"),
    ("Layer::Support", "com.core.tree-support"),
}

REFERENCE = (
    REPO / "docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour/reference.json"
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load(path: Path) -> tuple[list[dict], list[dict]]:
    """Return (instrumented events, probe rows)."""
    events: list[dict] = []
    probes: list[dict] = []
    text = path.read_text(encoding="utf-8", errors="replace")
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if row.get("event") == "probe":
            probes.append(row)
        else:
            events.append(row)
    return events, probes


def p50(values: list[int]) -> int:
    if not values:
        return 0
    ordered = sorted(values)
    return ordered[len(ordered) // 2]


def reduce_run(cell: str, tag: str, stem: Path) -> dict:
    raw = stem.with_suffix(".jsonl")
    gcode = stem.with_suffix(".gcode")
    events, probes = load(raw)
    complete = next((e for e in events if e.get("event") == "slice_complete"), {})
    phases = {
        e["phase"]: e.get("elapsed_ms")
        for e in events
        if e.get("event") == "phase_complete"
    }
    layer_events = [e for e in events if e.get("event") == "layer_complete"]
    hashes = json.loads(REFERENCE.read_text(encoding="utf-8"))["active_set"]
    ref_key = cell.removesuffix("-decline")
    gcode_hash = sha256(gcode) if gcode.exists() else ""

    sites: dict[tuple[str, str, str], list[int]] = collections.defaultdict(list)
    for row in probes:
        sites[(row["stage"], row["module"], row["site"])].append(int(row["us"]))

    module_sites = {}
    for (stage, module, site), values in sites.items():
        key = f"{stage}|{module}"
        module_sites.setdefault(key, {})[site] = {
            "n": len(values),
            "p50_us": p50(values),
            "p90_us": sorted(values)[min(len(values) - 1, int(len(values) * 0.9))],
            "sum_us": sum(values),
        }

    prep_views = {
        key: {
            "calls": rows["prep_views"]["n"],
            "p50_us": rows["prep_views"]["p50_us"],
            "sum_us": rows["prep_views"]["sum_us"],
        }
        for key, rows in module_sites.items()
        if "prep_views" in rows and rows["prep_views"]["sum_us"] > 0
    }

    ns = [p for p in probes if p.get("site") == "ns_call"]
    audit = [p for p in probes if p.get("site") == "sync_audit"]
    segments = collections.defaultdict(list)
    for row in probes:
        if row.get("module") == "host:prepare_regions":
            segments[row["site"]].append(int(row["us"]))

    return {
        "cell": cell,
        "tag": tag,
        "raw": str(raw.relative_to(REPO)),
        "exit": 0 if complete else None,
        "slice_complete": {
            "elapsed_ms": complete.get("elapsed_ms"),
            "degraded": complete.get("degraded"),
            "fatal_error_count": complete.get("fatal_error_count"),
            "non_fatal_error_count": complete.get("non_fatal_error_count"),
        },
        "phase_elapsed_ms": phases,
        "layer_count": len(layer_events),
        "gcode_sha256": gcode_hash,
        "reference_sha256": hashes.get(ref_key),
        "reference_match": gcode_hash == hashes.get(ref_key),
        "probe_rows": len(probes),
        "prep_views_by_stage_module": prep_views,
        "module_sites": module_sites,
        "prepare_regions_segments": {
            site: {"n": len(values), "p50_us": p50(values), "sum_us": sum(values)}
            for site, values in segments.items()
        },
        "needs_support": {
            "calls": len(ns),
            "p50_us": p50([int(p["us"]) for p in ns]),
            "sum_us": sum(int(p["us"]) for p in ns),
            "footprints_p50": p50([int(p["footprints"]) for p in ns]) if ns else 0,
            "footprints_max": max((int(p["footprints"]) for p in ns), default=0),
            "bbox_hits_max": max((int(p["bbox_hits"]) for p in ns), default=0),
            "value_true": sum(1 for p in ns if int(p["value"]) == 1),
            "value_false": sum(1 for p in ns if int(p["value"]) == 0),
        },
        "sync_audit": {
            "calls": len(audit),
            "changed": sum(1 for p in audit if int(p["us"]) != 0),
            "points_p50": p50([int(p["points_before"]) for p in audit]) if audit else 0,
        },
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", default=str(DEFAULT_RAW))
    ap.add_argument("--out", default=str(HERE / "probe.json"))
    args = ap.parse_args()
    raw = Path(args.raw)

    runs = []
    for cell, tag, stem in RUNS:
        path = Path(stem)
        if not path.with_suffix(".jsonl").exists():
            print(f"missing {path.with_suffix('.jsonl')}")
            continue
        runs.append(reduce_run(cell, tag, path))
        print(f"{cell}/{tag}: {runs[-1]['probe_rows']} probe rows")

    cells = {}
    for run in runs:
        cells.setdefault(run["cell"], []).append(run)
    reduced = {cell: entries[-1] for cell, entries in cells.items()}
    for cell, entries in cells.items():
        reduced[cell]["runs"] = [
            {
                "tag": e["tag"],
                "raw": e["raw"],
                "slice_elapsed_ms": e["slice_complete"]["elapsed_ms"],
                "per_layer_ms": e["phase_elapsed_ms"].get("per_layer"),
                "prepass_ms": e["phase_elapsed_ms"].get("prepass"),
                "reference_match": e["reference_match"],
                "gcode_sha256": e["gcode_sha256"],
            }
            for e in entries
        ]

    (Path(args.out)).write_text(
        json.dumps({"raw_dir": str(raw.relative_to(REPO)), "cells": reduced}, indent=1)
        + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Reduce the t94 Phase-1 raw probe captures into `summary.json`.

The raw captures live in the gitignored artefact tree
(`.local-artifacts/perimeter-reference-preparation/t94-remaining-drop/probe/captures`);
this script is the in-repo reducer the map's evidence policy requires, so the
committed `summary.json` is re-derivable from a plain checkout plus that tree.

Usage:
    python extract.py                 # default capture dir, writes summary.json
    python extract.py --captures DIR  # explicit capture dir
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
DEFAULT_CAPTURES = (
    REPO
    / ".local-artifacts"
    / "perimeter-reference-preparation"
    / "t94-remaining-drop"
    / "probe"
    / "captures"
)

PROBE_RE = re.compile(
    r"^T94 EMIT rotate_us=(?P<rotate>\d+) walk_us=(?P<walk>\d+) "
    r"drop_us=(?P<drop>\d+) layers=(?P<layers>\d+) parallel=(?P<parallel>true|false)$"
)

# t91's kept candidate-arm uninstrumented walls (`evidence/t91-exact-z-memo/FINDINGS.md`).
# These are the only post-t90, post-t91 ordinary base supports-on walls on disk;
# t92/t93 changed no production source, so they stand at this take's HEAD.
CELL_WALLS = {
    "ordinary-base-classic-on": 164.803,
    "ordinary-base-arachne-on": 166.931,
}

FLOOR_FRACTION = 0.008


def load_jsonl_events(path: Path) -> list[dict]:
    events = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return events


def parse_run(captures: Path, run_json: Path) -> dict:
    run = json.loads(run_json.read_text(encoding="utf-8"))
    stem = run_json.name[: -len(".run.json")]
    probe_line = None
    for line in (
        (captures / f"{stem}.probe.log").read_text(encoding="utf-8").splitlines()
    ):
        match = PROBE_RE.match(line.strip())
        if match:
            probe_line = match.groupdict()
    if probe_line is None:
        raise SystemExit(f"{stem}: no T94 EMIT probe line")

    events = [
        event
        for event in load_jsonl_events(captures / f"{stem}.jsonl")
        if event.get("phase") == "postpass"
    ]
    emit = next(
        event
        for event in events
        if event.get("event") == "module_complete"
        and event.get("stage") == "PostPass::GCodeEmit"
        and event.get("module_id") == "host:gcode_emit"
    )
    phase = next(event for event in events if event.get("event") == "phase_complete")
    gpp = [
        event
        for event in events
        if event.get("event") == "module_complete"
        and event.get("stage") == "PostPass::GCodePostProcess"
    ]
    serialize = [
        event
        for event in events
        if event.get("event") == "module_complete"
        and event.get("stage") == "PostPass::GCodeSerialize"
    ]
    return {
        "id": run["id"],
        "cell": f"{run['mode']}-{run['fixture']}-{run['generator']}-{run['supports']}",
        "arm": "parallel" if run["parallel"] == 1 else "serial",
        "parallel": run["parallel"] == 1,
        "threads": run["threads"],
        "exit_code": run["exit_code"],
        "wall_s": run["wall_seconds"],
        "cpu_s": run["cpu_seconds"],
        "cpu_wall": run["cpu_wall_ratio"],
        "sha256": run["sha256"],
        "bytes": run["bytes"],
        "reference_sha256": run["reference_sha256"],
        "layers": int(probe_line["layers"]),
        "rotate_us": int(probe_line["rotate"]),
        "walk_us": int(probe_line["walk"]),
        "drop_us": int(probe_line["drop"]),
        "probe_parallel": probe_line["parallel"] == "true",
        "emit_bracket_ms": emit["elapsed_ms"],
        "tail_ms": phase["timestamp_ms"] - emit["timestamp_ms"],
        "postpass_ms": phase.get("elapsed_ms"),
        "gcode_postprocess_ms": gpp[0]["elapsed_ms"] if gpp else 0,
        "serialize_ms": serialize[0]["elapsed_ms"] if serialize else 0,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--captures", type=Path, default=DEFAULT_CAPTURES)
    parser.add_argument("--out", type=Path, default=HERE / "summary.json")
    args = parser.parse_args()

    run_jsons = sorted(args.captures.glob("*.run.json"))
    if len(run_jsons) != 4:
        raise SystemExit(f"expected 4 run.json files, found {len(run_jsons)}")
    runs = [parse_run(args.captures, path) for path in run_jsons]
    runs.sort(key=lambda row: (row["cell"], row["arm"]))

    derived = {}
    for cell in sorted({row["cell"] for row in runs}):
        serial = next(
            row for row in runs if row["cell"] == cell and row["arm"] == "serial"
        )
        parallel = next(
            row for row in runs if row["cell"] == cell and row["arm"] == "parallel"
        )
        wall = CELL_WALLS[cell]
        floor_s = wall * FLOOR_FRACTION
        saving_s = (serial["drop_us"] - parallel["drop_us"]) / 1e6
        tail_s = serial["tail_ms"] / 1000.0
        derived[cell] = {
            "cell_wall_s": wall,
            "cell_wall_source": "t91-exact-z-memo/FINDINGS.md candidate arm (ordinary)",
            "floor_s": round(floor_s, 6),
            "bracket_serial_s": serial["emit_bracket_ms"] / 1000.0,
            "bracket_parallel_s": parallel["emit_bracket_ms"] / 1000.0,
            "rotate_s": serial["rotate_us"] / 1e6,
            "walk_s": serial["walk_us"] / 1e6,
            "serial_drop_s": serial["drop_us"] / 1e6,
            "parallel_drop_s": parallel["drop_us"] / 1e6,
            "parallel_saving_s": round(saving_s, 6),
            "parallel_saving_fraction": round(saving_s / wall, 9),
            "tail_serial_s": round(tail_s, 6),
            "tail_parallel_s": round(parallel["tail_ms"] / 1000.0, 6),
            "deferred_ceiling_s": round(tail_s, 6),
            "deferred_ceiling_fraction": round(tail_s / wall, 9),
            "deferred_ceiling_vs_floor": round(tail_s / floor_s, 6),
        }

    payload = {
        "ticket": "94-remaining-whole-print-drop",
        "take": "2026-10-29",
        "head": "57234b81899d0b0b551187c50ada8e7c380d74e6",
        "probe_env": {"PNP_T94_PROBE": "1", "RAYON_NUM_THREADS": "12"},
        "floor_fraction": FLOOR_FRACTION,
        "runs": runs,
        "derived": derived,
    }
    args.out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {args.out} ({len(runs)} runs, {len(derived)} cells)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

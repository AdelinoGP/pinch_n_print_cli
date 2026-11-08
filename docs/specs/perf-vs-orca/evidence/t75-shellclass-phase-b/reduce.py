#!/usr/bin/env python3
"""Reduce the t75 attribution captures into split.json.

Raw captures are the gitignored tree under `target/t75/` (baseline probe runs
in `t75-probe/`, candidate runs in `t75-candidate/`, run logs in `*.log`).
The in-repo reduced set is this script's output only; `verify.py` checks it
without the raw tree.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
CELLS = {
    "base-classic-off": ("base", "classic", "off"),
    "base-classic-on": ("base", "classic", "on"),
    "base-arachne-on": ("base", "arachne", "on"),
    "benchy-classic-on": ("benchy", "classic", "on"),
}
PROBE_PREFIX = "T75PROBE "
PAIR_KEYS = {
    "deep_before": ("deep_before_polys", "deep_before_verts"),
    "deep_after": ("deep_after_remove_polys", "deep_after_remove_verts"),
    "clip": ("clip_polys", "clip_verts"),
    "erosion": ("erosion_polys", "erosion_verts"),
    "qualified": ("qualified_polys", "qualified_verts"),
}
LOG_RE = re.compile(
    r"\[t75\] DONE (?P<cell>\S+) wall=(?P<wall>[0-9,.]+)s cpu=(?P<cpu>[0-9,.]+)s"
)


def parse_probe(path: Path) -> dict:
    line = path.read_text(encoding="utf-8").strip()
    if not line.startswith(PROBE_PREFIX):
        raise SystemExit(f"not a probe line: {path}")
    out = {}
    for token in line[len(PROBE_PREFIX) :].split():
        if "=" not in token:
            continue
        key, value = token.split("=", 1)
        if key in PAIR_KEYS:
            polys, verts = value.split("/")
            out[PAIR_KEYS[key][0]] = int(polys)
            out[PAIR_KEYS[key][1]] = int(verts)
        elif key in {
            "timelines",
            "preps",
            "clip_calls",
            "erosion_calls",
            "angles_calls",
            "surface_calls",
            "retain_calls",
            "lower_layers",
            "lower_polys",
            "region_surfaces",
            "region_infill",
        }:
            out[key] = int(value)
        else:
            out[key] = float(value)
    return out


def parse_stage_ms(path: Path) -> int:
    stage = []
    complete = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if not line.startswith("{"):
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if (
            event.get("phase") == "prepass"
            and event.get("stage") == "PrePass::ShellClassification"
            and event.get("status") == "ok"
            and "elapsed_ms" in event
            and "module_id" not in event
        ):
            stage.append(int(event["elapsed_ms"]))
        if event.get("event") == "slice_complete":
            complete = int(event["elapsed_ms"])
    if not stage or complete is None:
        raise SystemExit(f"missing stage/slice_complete in {path}")
    return max(stage), complete


def digest(path: Path) -> tuple[str, int]:
    data = path.read_bytes()
    return hashlib.sha256(data).hexdigest(), len(data)


def parse_logs(raw: Path) -> dict:
    out = {}
    for path in sorted(raw.glob("*.log")):
        for match in LOG_RE.finditer(
            path.read_text(encoding="utf-8", errors="replace")
        ):
            out[match.group("cell")] = {
                "wall_seconds": float(match.group("wall").replace(",", "")),
                "cpu_seconds": float(match.group("cpu").replace(",", "")),
            }
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw-root", default="target/t75")
    args = parser.parse_args()
    raw = Path(args.raw_root)
    logs = parse_logs(raw)
    references = {
        cell["cell"]: cell
        for cell in json.loads(
            (HERE.parent / "t58-finalization-copies" / "qualification.json").read_text(
                encoding="utf-8"
            )
        )["cells"]
    }
    cells = {}
    for cell in CELLS:
        baseline_probe = raw / "t75-probe" / f"{cell}.probe.txt"
        baseline_jsonl = raw / "t75-probe" / f"{cell}.stderr.jsonl"
        baseline_gcode = raw / "t75-probe" / f"{cell}.gcode"
        candidate_jsonl = raw / "t75-candidate" / f"{cell}.stderr.jsonl"
        candidate_gcode = raw / "t75-candidate" / f"{cell}.gcode"
        for path in (
            baseline_probe,
            baseline_jsonl,
            baseline_gcode,
            candidate_jsonl,
            candidate_gcode,
        ):
            if not path.exists():
                raise SystemExit(f"missing raw capture: {path}")
        base_stage, base_complete = parse_stage_ms(baseline_jsonl)
        cand_stage, cand_complete = parse_stage_ms(candidate_jsonl)
        base_sha, base_bytes = digest(baseline_gcode)
        cand_sha, cand_bytes = digest(candidate_gcode)
        reference = references[
            f"ordinary-{CELLS[cell][0]}-{CELLS[cell][1]}-{CELLS[cell][2]}"
        ]
        cells[cell] = {
            "fixture": CELLS[cell][0],
            "generator": CELLS[cell][1],
            "supports": CELLS[cell][2],
            "baseline": {
                "probe": parse_probe(baseline_probe),
                "stage_ms": base_stage,
                "slice_complete_ms": base_complete,
                **logs[cell],
                "sha256": base_sha,
                "bytes": base_bytes,
            },
            "candidate": {
                "stage_ms": cand_stage,
                "slice_complete_ms": cand_complete,
                **logs[cell],
                "sha256": cand_sha,
                "bytes": cand_bytes,
            },
            "reference": {
                "sha256": reference["sha256"],
                "bytes": reference["bytes"],
            },
        }
    payload = {
        "ticket": "t75-shellclassification-phase-b",
        "units": "ms for probe/stage fields, seconds for process wall/cpu",
        "note": "Instrumented observer runs are attribution only; wall_seconds and cpu_seconds are recorded for disclosure, never as acceptance timings.",
        "cells": cells,
    }
    (HERE / "split.json").write_text(json.dumps(payload, indent=1), encoding="utf-8")
    print(f"reduced {len(cells)} cells into {HERE / 'split.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

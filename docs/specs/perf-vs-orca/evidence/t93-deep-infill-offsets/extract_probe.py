#!/usr/bin/env python3
"""t93 Phase-1 extractor.

Reads the two probe captures from this machine's `.local-artifacts` root and
writes `probe-rows.json` next to this script: per-run wall/CPU/ratio, the
output identity against the post-t81 reference, and one row per
`compute_internal_bridge_layer` call.

Attribution only: instrumented timings are never a wall claim.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
CAPTURE = (
    HERE.parents[4]
    / ".local-artifacts"
    / "perimeter-reference-preparation"
    / "t93-deep-infill-offsets"
    / "probe"
    / "captures"
)

RUNS = (
    ("run1", "run1-probe.log", "run1-run.json"),
    (
        "run2",
        "ordinary-base-classic-off-probe.probe.log",
        "ordinary-base-classic-off-probe.run.json",
    ),
)

INT_FIELDS = {
    "surfaces",
    "surfaces_passed_clip",
    "deep_polys",
    "deep_holes",
    "deep_vertices",
    "deep_fp",
    "bbox_skip",
    "deep_clip_polys",
    "pre_qualified",
    "qualified",
    "window_operand_polys",
    "window_operand_vertices",
    "layer",
    "region",
}
FLOAT_FIELDS = {
    "print_z",
    "deep_clip_offset_ms",
    "surface_offset_ms",
    "surface_clip_ms",
    "qualify_ms",
    "erosion_ms",
    "window_clip_ms",
    "window_offset_ms",
    "retain_ms",
    "total_ms",
}
STR_FIELDS = {"phase", "object"}


def parse_row(line: str) -> dict:
    row: dict = {}
    for key, value in re.findall(r"(\w+)=(\S+)", line):
        if key in STR_FIELDS:
            row[key] = value
        elif key in INT_FIELDS:
            row[key] = int(value)
        elif key in FLOAT_FIELDS:
            row[key] = float(value)
    return row


def parse_run(tag: str, log_name: str, run_name: str) -> dict:
    log_path = CAPTURE / log_name
    run_path = CAPTURE / run_name
    rows = [
        parse_row(line)
        for line in log_path.read_text(encoding="utf-8", errors="replace").splitlines()
        if line.startswith("T93LAYER ")
    ]
    run = json.loads(run_path.read_text(encoding="utf-8"))
    return {
        "run": tag,
        "wall_seconds": run["wall_seconds"],
        "cpu_seconds": run["cpu_seconds"],
        "cpu_wall_ratio": run["cpu_wall_ratio"],
        "sha256": run["sha256"],
        "reference_sha256": run["reference_sha256"],
        "bytes": run["bytes"],
        "reference_bytes": 21023739,
        "rows": rows,
    }


def main() -> int:
    runs = [parse_run(*spec) for spec in RUNS]
    payload = {
        "cell": "ordinary-base-classic-off",
        "note": (
            "t93 Phase-1 screen. Per-call rows from PNP_T93_PROBE=1 on the "
            "matched job; accumulated over the layer axis and converted to a "
            "wall equivalent with each run's own external cpu/wall ratio. "
            "Instrumented: never a wall claim."
        ),
        "runs": runs,
    }
    (HERE / "probe-rows.json").write_text(
        json.dumps(payload, indent=1) + "\n", encoding="utf-8"
    )
    for run in runs:
        rows = run["rows"]
        pre = [row for row in rows if row["pre_qualified"] > 0]
        saving = sum(
            row["erosion_ms"] - row["window_clip_ms"] - row["window_offset_ms"]
            for row in pre
        )
        print(
            f"{run['run']}: rows={len(rows)} pre_qualified={len(pre)} "
            f"erosion={sum(row['erosion_ms'] for row in pre):.1f} ms "
            f"windowed={sum(row['window_clip_ms'] + row['window_offset_ms'] for row in pre):.1f} ms "
            f"saving={saving:.1f} ms ({saving / run['cpu_wall_ratio']:.1f} ms wall-eq)"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

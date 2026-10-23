#!/usr/bin/env python3
"""t79 screening reducer.

Reads `sweep/runs.jsonl` plus each run's instrumented stderr and reports, per
variant, the `com.core.tree-support-planner` module bracket (the screening
statistic), the `PrePass::SupportGeometry` and `Layer::Support` stage walls,
the instrumented slice elapsed, the process peak working set and the output
hash. Instrumented numbers are screening only; they never become wall claims.

The committed `sweep-summary.json` carries the reduced rows, so `verify.py` can
check the take without the raw sweep tree.
"""

from __future__ import annotations

import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SWEEP = HERE / "sweep"
RUNS = SWEEP / "runs.jsonl"
PLANNER = "com.core.tree-support-planner"


def repo_root() -> Path:
    for ancestor in [HERE, *HERE.parents]:
        if (ancestor / ".local-artifacts").is_dir():
            return ancestor
    raise SystemExit("could not locate repo root (.local-artifacts)")


def reduce_run(row: dict) -> dict:
    path = Path(row["stderr"])
    if not path.is_absolute():
        path = repo_root() / path
    planner_ms = 0
    invocations = 0
    stages: dict[str, float] = {}
    slice_elapsed = 0.0
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        event = json.loads(line)
        kind = event.get("event")
        if kind == "module_complete" and event.get("module_id") == PLANNER:
            planner_ms += int(event.get("elapsed_ms", 0))
            invocations += 1
        elif kind == "stage_complete":
            stage = event.get("stage")
            if stage:
                stages[stage] = stages.get(stage, 0.0) + float(
                    event.get("elapsed_ms", 0)
                )
        elif kind == "slice_complete":
            slice_elapsed = float(event.get("elapsed_ms", 0))
    return {
        "variant": row["variant"],
        "e": int(row["e"]),
        "c": int(row["c"]),
        "repeat": int(row["repeat"]),
        "planner_ms": planner_ms,
        "planner_invocations": invocations,
        "support_geometry_ms": stages.get("PrePass::SupportGeometry", 0.0),
        "layer_support_ms": stages.get("Layer::Support", 0.0),
        "slice_elapsed_ms": slice_elapsed,
        "wall_seconds": float(row["wall_seconds"]),
        "peak_working_set_mb": float(row["peak_working_set_mb"]),
        "bytes": int(row["bytes"]),
        "sha256": row["sha256"],
        "status": row["status"],
        "degraded": row["degraded"],
    }


def main() -> int:
    if not RUNS.exists():
        raise SystemExit(f"no sweep rows at {RUNS}")
    rows = [
        reduce_run(json.loads(line))
        for line in RUNS.read_text(encoding="utf-8-sig").splitlines()
        if line.strip()
    ]
    groups: dict[tuple[int, int], list[dict]] = {}
    for row in rows:
        groups.setdefault((row["e"], row["c"]), []).append(row)
    cells = []
    for (e, c), group in sorted(
        groups.items(), key=lambda item: (item[0][0], item[0][1])
    ):
        by_planner = sorted(r["planner_ms"] for r in group)
        by_wall = sorted(r["wall_seconds"] for r in group)
        cells.append(
            {
                "e": e,
                "c": c,
                "runs": len(group),
                "planner_ms": [r["planner_ms"] for r in group],
                "planner_median_ms": statistics.median(by_planner),
                "planner_invocations": [r["planner_invocations"] for r in group],
                "support_geometry_ms": [r["support_geometry_ms"] for r in group],
                "slice_elapsed_ms": [r["slice_elapsed_ms"] for r in group],
                "wall_median_s": statistics.median(by_wall),
                "peak_working_set_mb_max": max(r["peak_working_set_mb"] for r in group),
                "sha256": sorted({r["sha256"] for r in group}),
                "clean": all(
                    r["status"] == "ok" and r["degraded"] in ("False", False)
                    for r in group
                ),
            }
        )
    out = {
        "cell": "ordinary-base-classic-on",
        "instrumented_only": True,
        "note": (
            "Screening only. The planner module bracket chooses the constants; "
            "the standing paired gate decides the keep/drop. Every run must hash "
            "to the preserved t58 reference."
        ),
        "variants": cells,
    }
    (HERE / "sweep-summary.json").write_text(
        json.dumps(out, indent=1), encoding="utf-8"
    )
    print(f"{'e':>4} {'c':>5} {'n':>2} {'planner_ms':>11} {'wall_s':>7} {'peakMB':>7}")
    for cell in cells:
        print(
            f"{cell['e']:>4} {cell['c']:>5} {cell['runs']:>2} "
            f"{cell['planner_median_ms']:>11.0f} {cell['wall_median_s']:>7.1f} "
            f"{cell['peak_working_set_mb_max']:>7.0f}"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())

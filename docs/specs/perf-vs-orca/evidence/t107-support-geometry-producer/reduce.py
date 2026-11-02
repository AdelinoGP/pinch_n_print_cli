#!/usr/bin/env python3
"""t107 `PrePass::SupportGeometry` screen -- reduce the committed raw streams.

The stage id `PrePass::SupportGeometry` is carried by **two** stages that run
back to back:

1. the host builtin `host:support_geometry`
   (`SUPPORT_GEOMETRY_PRODUCER`), which builds `SupportGeometryIR.entries`; and
2. the support-planner module stage, which runs every module claiming
   `support-planner` (`com.core.traditional-support-planner`,
   `com.core.tree-support-planner`).

`stage_start`/`stage_complete` are keyed on `(stage, layer)` only, so ticket 104
summed both under one name. This reducer pairs each `stage_complete` with its
own `stage_start`, reads the nested `module_complete` rows to attribute the
planner stage to its modules, and reconciles the two-bracket sum against ticket
104's committed `phase-split.json`.

Usage:
  python reduce.py [--raw target/t104-instrumented] [--out summary.json]
"""

from __future__ import annotations

import argparse
import collections
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
T104 = (
    REPO
    / "docs"
    / "specs"
    / "perf-vs-orca"
    / "evidence"
    / "t104-phase-split"
    / "phase-split.json"
)
T102 = (
    REPO
    / "docs"
    / "specs"
    / "perf-vs-orca"
    / "evidence"
    / "t102-allocator"
    / "summary.json"
)
T103 = (
    REPO
    / "docs"
    / "specs"
    / "perf-vs-orca"
    / "evidence"
    / "t103-orca-rebaseline"
    / "summary.json"
)

STAGE = "PrePass::SupportGeometry"
PRODUCER_MODULE = "host:support_geometry"
CELLS = (
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
)


def load_events(path: Path) -> list[dict]:
    rows = []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    return rows


def reduce_brackets(rows: list[dict]) -> list[dict]:
    """Pair every `stage_start` with its own `stage_complete`, keeping the
    nested module ids and their own `module_complete` durations."""
    out: list[dict] = []
    open_bracket: dict | None = None
    module_ms: dict[str, int] = {}
    for r in rows:
        ev = r.get("event")
        if ev == "stage_start" and r.get("stage"):
            open_bracket = {"stage": r["stage"], "modules": []}
            module_ms = {}
        elif (
            ev == "module_start"
            and open_bracket
            and r.get("stage") == open_bracket["stage"]
        ):
            mod = r.get("module_id")
            if mod and mod not in open_bracket["modules"]:
                open_bracket["modules"].append(mod)
        elif (
            ev == "module_complete"
            and open_bracket
            and r.get("stage") == open_bracket["stage"]
        ):
            mod = r.get("module_id")
            if mod:
                module_ms[mod] = module_ms.get(mod, 0) + (r.get("elapsed_ms") or 0)
        elif ev == "stage_complete" and r.get("stage"):
            if open_bracket and r["stage"] == open_bracket["stage"]:
                open_bracket["elapsed_ms"] = r.get("elapsed_ms")
                open_bracket["module_ms"] = module_ms
                out.append(open_bracket)
                open_bracket = None
            else:
                # A stage that completed without a start is not expected here;
                # record it so the reducer fails loudly instead of dropping it.
                out.append(
                    {
                        "stage": r["stage"],
                        "elapsed_ms": r.get("elapsed_ms"),
                        "modules": [],
                        "module_ms": {},
                        "unpaired": True,
                    }
                )
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", default=str(REPO / "target" / "t104-instrumented"))
    ap.add_argument("--preview-counts", default=str(HERE / "preview-counts.json"))
    ap.add_argument("--out", default=str(HERE / "summary.json"))
    a = ap.parse_args()
    raw = Path(a.raw)

    t104 = json.loads(T104.read_text(encoding="utf-8"))["cells"]
    t102 = json.loads(T102.read_text(encoding="utf-8"))
    t103 = json.loads(T103.read_text(encoding="utf-8"))["cells"]
    preview = json.loads(Path(a.preview_counts).read_text(encoding="utf-8"))

    cells = {}
    for cell in CELLS:
        rows = load_events(raw / f"{cell}.jsonl")
        complete = next((r for r in rows if r.get("event") == "slice_complete"), None)
        if complete is None:
            raise SystemExit(
                f"{cell}: no slice_complete event in {raw / (cell + '.jsonl')}"
            )
        cell_ms = complete["elapsed_ms"]

        counts = collections.Counter(
            r["stage"]
            for r in rows
            if r.get("event") == "stage_complete" and r.get("stage")
        )
        duplicated_prepass = sorted(
            k for k, v in counts.items() if v > 1 and k.startswith("PrePass::")
        )

        brackets = [b for b in reduce_brackets(rows) if b["stage"] == STAGE]
        producer = [b for b in brackets if PRODUCER_MODULE in b["modules"]]
        planner = [b for b in brackets if PRODUCER_MODULE not in b["modules"]]
        if len(brackets) != 2 or len(producer) != 1 or len(planner) != 1:
            raise SystemExit(
                f"{cell}: expected two {STAGE} brackets (producer + planner), got "
                f"{[(b['modules'], b['elapsed_ms']) for b in brackets]}. "
                "The shared-stage-id premise changed -- re-derive before reducing."
            )
        producer_ms = producer[0]["elapsed_ms"]
        planner_ms = planner[0]["elapsed_ms"]
        stage_sum = sum(b["elapsed_ms"] for b in brackets)
        t104_ms = t104[cell]["prepass_stages_ms"].get(STAGE)
        if t104_ms != stage_sum:
            raise SystemExit(
                f"{cell}: bracket sum {stage_sum} ms does not reconcile with t104's "
                f"committed {t104_ms} ms -- the reduction changed, re-derive."
            )

        fixture, generator, supports = cell.split("-", 2)
        wall_s = t103[f"{fixture}/{generator}-{supports}"]["tools"]["pnp-ordinary"][
            "wall_s"
        ]
        floor_s = (
            t102["cells"]["ordinary-base-classic-on"]["floor_seconds"]
            if cell == "base-classic-on"
            else 0.008 * wall_s
        )

        def share_pct(ms: float) -> float:
            return round(100.0 * ms / cell_ms, 4)

        cells[cell] = {
            "instrumented_cell_ms": cell_ms,
            "layer_count": complete.get("layer_count"),
            "degraded": complete.get("degraded"),
            "non_fatal_error_count": complete.get("non_fatal_error_count"),
            "stage_brackets": [
                {
                    "role": "host_producer"
                    if b is producer[0]
                    else "planner_module_stage",
                    "elapsed_ms": b["elapsed_ms"],
                    "modules": b["modules"],
                    "module_ms": b["module_ms"],
                }
                for b in brackets
            ],
            "duplicated_prepass_stage_ids": duplicated_prepass,
            "stage_sum_ms": stage_sum,
            "t104_committed_stage_ms": t104_ms,
            "reconciles": True,
            "producer_ms": producer_ms,
            "planner_ms": planner_ms,
            "planner_module_ms": planner[0]["module_ms"],
            "share_of_cell_pct": {
                "producer": share_pct(producer_ms),
                "planner": share_pct(planner_ms),
            },
            "implied_wall_s": {
                "producer": round(producer_ms / cell_ms * wall_s, 4),
                "planner": round(planner_ms / cell_ms * wall_s, 4),
                "cell": wall_s,
            },
            "floor_s": round(floor_s, 6),
            "floor_multiple": {
                "producer": round((producer_ms / 1000.0) / floor_s, 4),
                "planner": round((planner_ms / 1000.0) / floor_s, 2),
            },
        }

    summary = {
        "take": "t107-support-geometry-producer",
        "head": "9a1921ac",
        "binary_sha256": "4cb969421ca1193be9501b8147324e2fcf86fbf6dcd42bd5903315cf5844a1e9",
        "source_streams": "target/t104-instrumented/<cell>.jsonl (gitignored, produced by evidence/t104-phase-split/run-instrumented.py)",
        "floor_definition": "0.80 % of the cell: base classic-on uses t102's committed floor (0.80 % of the t91 wall); the other cells use 0.80 % of their t103 PnP ordinary wall (identical to t106's recorded value).",
        "preview_counts_benchy_on": preview,
        "cells": cells,
    }
    Path(a.out).write_text(json.dumps(summary, indent=1), encoding="utf-8")
    for cell, v in cells.items():
        print(
            f"{cell}: cell {v['instrumented_cell_ms']} ms | producer {v['producer_ms']} ms "
            f"({v['share_of_cell_pct']['producer']} %, {v['floor_multiple']['producer']}x floor) | "
            f"planner {v['planner_ms']} ms ({v['share_of_cell_pct']['planner']} %, "
            f"{v['floor_multiple']['planner']}x floor) modules={v['planner_module_ms']}"
        )
    print(f"wrote {a.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

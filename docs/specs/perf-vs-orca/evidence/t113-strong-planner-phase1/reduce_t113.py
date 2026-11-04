#!/usr/bin/env python3
"""t113 Strong-planner Phase-1 reducer.

Reads the four `--instrument-stderr` JSONL captures from a raw root (default
`target/t113-probe/`) and writes `reduced/summary.json`.

The probe line is the `t113-planner-split` module_log emitted by the temporary
guest probe in `probe.patch`. On supports-off cells the planner early-returns
before the probe guard, so only the host `module_complete` bracket exists.

Attribution only: the instrumented cell walls are not wall claims. The floor
multiple uses the same run's `slice_complete` wall, so it is a conservative
same-run reading; the map's quiet cell wall would make the same share larger
relative to a smaller floor.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-off",
    "base-classic-on",
    "base-arachne-on",
]
FLOOR = 0.0080
PROBE_PREFIX = "t113-planner-split"


def parse_terms(line: str) -> dict[str, dict[str, int]]:
    terms: dict[str, dict[str, int]] = {}
    for token in line.split()[1:]:
        match = re.fullmatch(r"([a-z0-9_]+)=(\d+)us/(\d+)", token)
        if not match:
            continue
        name, us, calls = match.groups()
        terms[name] = {"us": int(us), "calls": int(calls)}
    return terms


def parse_capture(path: Path) -> dict:
    wall_ms = None
    module_ms = None
    probe_line = None
    clean = None
    with path.open(encoding="utf-8", errors="replace") as handle:
        for line in handle:
            if not line.startswith("{"):
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            kind = event.get("event")
            if (
                kind == "module_complete"
                and event.get("module_id") == "com.core.tree-support-planner"
            ):
                module_ms = event.get("elapsed_ms")
            elif kind == "slice_complete":
                wall_ms = event.get("elapsed_ms")
                clean = {
                    "status": event.get("status"),
                    "degraded": event.get("degraded"),
                    "fatal_error_count": event.get("fatal_error_count"),
                    "non_fatal_error_count": event.get("non_fatal_error_count"),
                }
            elif kind == "module_log":
                message = event.get("message", "")
                if message.startswith(PROBE_PREFIX):
                    probe_line = message
    if wall_ms is None or module_ms is None:
        raise SystemExit(f"{path}: missing slice_complete or planner module_complete")
    terms = parse_terms(probe_line) if probe_line else {}
    return {
        "capture": path.name,
        "wall_ms": wall_ms,
        "planner_module_ms": module_ms,
        "probe_line": probe_line,
        "terms": terms,
        "clean": clean,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw-root", default=str(REPO / "target" / "t113-probe"))
    parser.add_argument("--out", default=str(HERE / "reduced" / "summary.json"))
    args = parser.parse_args()

    raw_root = Path(args.raw_root)
    cells = {}
    for cell in CELLS:
        parsed = parse_capture(raw_root / f"{cell}.jsonl")
        wall_ms = parsed["wall_ms"]
        floor_ms = wall_ms * FLOOR
        row = {
            "capture": parsed["capture"],
            "wall_ms": wall_ms,
            "planner_module_ms": parsed["planner_module_ms"],
            "planner_share_of_cell": parsed["planner_module_ms"] / wall_ms,
            "floor_ms": floor_ms,
            "planner_multiple_of_floor": parsed["planner_module_ms"] / floor_ms,
            "clean": parsed["clean"],
            "probe_line": parsed["probe_line"],
            "terms": parsed["terms"],
        }
        probe_total = parsed["terms"].get("p1_total", {}).get("us")
        if probe_total:
            row["probe_total_ms"] = probe_total / 1000.0
            row["probe_total_share_of_cell"] = probe_total / 1000.0 / wall_ms
            inflate = parsed["terms"]["p6d_inflate"]["us"] / 1000.0
            clip = parsed["terms"]["p6a_build_roles"]["us"] / 1000.0
            drop = parsed["terms"]["p3_drop"]["us"] / 1000.0
            row["subterms_ms"] = {
                "occupancy_inflate": inflate,
                "clip_core": clip,
                "drop_loop": drop,
                "contacts": parsed["terms"]["p2_contacts"]["us"] / 1000.0,
                "node_roles": parsed["terms"]["p6e_node_roles"]["us"] / 1000.0,
                "node_ellipses": parsed["terms"]["p6f_node_ellipses"]["us"] / 1000.0,
                "volumes": parsed["terms"]["p0_volumes_new"]["us"] / 1000.0,
                "smooth": parsed["terms"]["p5_smooth"]["us"] / 1000.0,
                "stamp": parsed["terms"]["p7_stamp"]["us"] / 1000.0,
            }
            row["subterm_shares_of_probe_total"] = {
                key: value / (probe_total / 1000.0)
                for key, value in row["subterms_ms"].items()
            }
            row["occupancy_inflate_share_of_cell"] = inflate / wall_ms
            row["occupancy_inflate_multiple_of_floor"] = inflate / floor_ms
            # Ideal 12-thread fan-out and the t78 measured effective 2.6x.
            row["occupancy_inflate_ceiling_12x_ms"] = inflate * (1.0 - 1.0 / 12.0)
            row["occupancy_inflate_ceiling_12x_multiple_of_floor"] = (
                row["occupancy_inflate_ceiling_12x_ms"] / floor_ms
            )
            row["occupancy_inflate_ceiling_2_6x_ms"] = inflate * (1.0 - 1.0 / 2.6)
            row["occupancy_inflate_ceiling_2_6x_multiple_of_floor"] = (
                row["occupancy_inflate_ceiling_2_6x_ms"] / floor_ms
            )
            row["occupancy_inflate_layer_ms"] = (
                parsed["terms"]["p6d_layer"]["us"] / 1000.0
            )
            row["occupancy_inflate_entry_ms"] = (
                parsed["terms"]["p6d_entry"]["us"] / 1000.0
            )
        cells[cell] = row

    summary = {
        "route": "wayfinder ticket 113 (Strong support planner Phase 1)",
        "floor_fraction": FLOOR,
        "attribution_only": True,
        "cells": cells,
    }
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

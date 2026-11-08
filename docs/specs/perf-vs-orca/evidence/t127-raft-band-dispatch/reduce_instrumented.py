#!/usr/bin/env python3
"""Reduce the ticket 127 instrumented probe into the committed evidence set.

The baseline per-call cost and call counts are ticket 126's committed
instrumented pass; this reducer only adds the candidate's observed counts and
checks the output against the post-t123 reference set.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
T126 = HERE.parent / "t126-post-t123-phase-split" / "summary.json"
T123 = HERE.parent / "t123-facet-retirement" / "reduced" / "summary.json"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--rows", default="target/t127/instrumented/rows.json")
    ap.add_argument("--out", default=str(HERE / "reduced" / "instrumented.json"))
    args = ap.parse_args()

    rows = {
        row["cell"]: row
        for row in json.loads(Path(args.rows).read_text(encoding="utf-8"))
    }
    baseline = json.loads(T126.read_text(encoding="utf-8"))["cells"]
    reference = json.loads(T123.read_text(encoding="utf-8"))["output"]

    cells = {}
    for cell, row in rows.items():
        stats = baseline[cell]["module_call_stats"]["per_layer"][
            "com.core.raft-default"
        ]
        want = reference[cell]["candidate-ordinary"]["sha256"]
        cells[cell] = {
            "layers": row["layers"],
            "raft_default_calls": row["raft_default_calls"],
            "raft_default_thread_ms": row["raft_default_thread_ms"],
            "baseline_raft_default_calls": stats["calls"],
            "baseline_raft_default_thread_ms": stats["thread_sum_ms"],
            "baseline_raft_default_p50_ms": stats["p50_ms"],
            "infill_max_calls": row["infill_max_calls"],
            "infill_modules_observed": row["infill_modules_observed"],
            "gcode_sha256": row["gcode_sha256"],
            "reference_sha256": want,
            "gcode_matches_reference": row["gcode_sha256"] == want,
        }
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "ticket": "127",
        "note": (
            "Attribution only: one candidate instrumented pass per cell. Baseline "
            "call counts and thread sums are ticket 126's committed instrumented "
            "pass, not a same-run control."
        ),
        "cells": cells,
    }
    out_path.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {out_path}")
    for cell, row in cells.items():
        print(
            f"  {cell}: raft-default {row['baseline_raft_default_calls']} -> "
            f"{row['raft_default_calls']} calls, {row['baseline_raft_default_thread_ms']} -> "
            f"{row['raft_default_thread_ms']:.0f} thread-ms, ref={row['gcode_matches_reference']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

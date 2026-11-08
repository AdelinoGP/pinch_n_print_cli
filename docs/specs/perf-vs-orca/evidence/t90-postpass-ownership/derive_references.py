#!/usr/bin/env python3
"""Derive the t90 output references from the t81 campaign rows.

The reference set for this take is the t81 resolution-port campaign's
**candidate** arm: after that port, those bytes are the tree's standing output
(t84 reproduced the two base classic cells exactly, t83 the base arachne-off
cell). This script re-derives the table from the in-repo t81 CSVs instead of
pinning it by hand, and refuses to emit a reference for a cell whose campaign
runs disagree on the G-code SHA.

The t81 campaign is ordinary-mode only. Accelerated-mode references do not
exist for the post-t81 tree, so the accelerated proof cells are compared
arm-to-arm plus (reported, not gated) ordinary-to-accelerated.
"""

from __future__ import annotations

import collections
import csv
import glob
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
T81 = HERE.parent / "t81-resolution-port" / "campaign"


def main() -> int:
    shas: dict[tuple[str, str, str], dict[str, set[str]]] = collections.defaultdict(
        lambda: collections.defaultdict(set)
    )
    sizes: dict[tuple[str, str, str], dict[str, set[int]]] = collections.defaultdict(
        lambda: collections.defaultdict(set)
    )
    for path in sorted(T81.glob("*-rows.csv")):
        if "noisy" in path.name:
            continue
        with path.open(encoding="utf-8-sig", newline="") as handle:
            for row in csv.DictReader(handle):
                key = (row["fixture"], row["generator"], row["supports"])
                shas[key][row["arm"]].add(row["sha256_of_gcode"])
                sizes[key][row["arm"]].add(int(row["gcode_bytes"]))
    cells = {}
    problems = []
    for key in sorted(shas):
        candidate = shas[key]["candidate"]
        baseline = shas[key]["baseline"]
        if len(candidate) != 1:
            problems.append(
                f"{key}: candidate SHAs not deterministic ({len(candidate)})"
            )
            continue
        if len(baseline) != 1:
            problems.append(f"{key}: baseline SHAs not deterministic ({len(baseline)})")
            continue
        sha = next(iter(candidate))
        size = sizes[key]["candidate"]
        if len(size) != 1:
            problems.append(f"{key}: candidate byte sizes disagree")
            continue
        cells[f"ordinary-{key[0]}-{key[1]}-{key[2]}"] = {
            "sha256": sha,
            "bytes": next(iter(size)),
            "source": "t81-resolution-port/campaign (candidate arm)",
            "t81_baseline_sha256": next(iter(baseline)),
        }
    out = {
        "note": (
            "Post-t81 output references for the ordinary-mode proof cells. "
            "Derived, never hand-pinned: re-run derive_references.py."
        ),
        "cells": cells,
        "problems": problems,
    }
    (HERE / "references.json").write_text(json.dumps(out, indent=1), encoding="utf-8")
    print(f"references: {len(cells)} ordinary cells, {len(problems)} problem(s)")
    for problem in problems:
        print("PROBLEM:", problem)
    return 0 if not problems and len(cells) == 8 else 1


if __name__ == "__main__":
    raise SystemExit(main())

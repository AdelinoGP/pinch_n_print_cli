#!/usr/bin/env python3
"""Reduce this machine's t85 captures into the committed evidence rows.

Reads `.local-artifacts/perimeter-reference-preparation/t85-seam-inset-batching/`
and writes, next to this script:

  proof-rows.json     16 cells x 2 arms (the qualification rows)
  measure-rows.json    4 cells x (2 warmups + 12 measured) rows
  attribution.json     the instrumented `PrePass::SeamPlanning` walls per arm

`reduce.py` derives `qualification.json` / `ab-reduction.json` / `summary.json`
from those committed rows, so `verify.py` can reproduce every headline claim in
a plain checkout.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import reduce

HERE = reduce.HERE
ROOT = reduce.ROOT
SEAM_STAGE = "PrePass::SeamPlanning"
SEAM_MODULE = "com.core.seam-planner-default"


def capture_terms(path: Path) -> dict:
    slice_ms = None
    stage_ms = None
    module_ms = None
    fuel = None
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line.startswith("{"):
                continue
            event = json.loads(line)
            kind = event.get("event")
            if kind == "slice_complete":
                slice_ms = event.get("elapsed_ms")
            elif kind == "stage_complete" and event.get("stage") == SEAM_STAGE:
                stage_ms = event.get("elapsed_ms")
            elif kind == "module_complete" and event.get("module_id") == SEAM_MODULE:
                module_ms = event.get("elapsed_ms")
                fuel = (event.get("profile_scopes") or {}).get("call_fuel")
    return {
        "slice_ms": slice_ms,
        "seam_stage_ms": stage_ms,
        "seam_module_ms": module_ms,
        "seam_module_fuel": fuel,
    }


ATTRIBUTION_TAG = "attribution"


def build_attribution() -> dict:
    """Committed attribution comes from the `attribution` capture tag only.

    Other tags (e.g. the pre-campaign `sanity` pair, taken under desktop load)
    are deliberately excluded: they are disclosed in the findings, not used.
    """
    cells = {}
    tag_dir = ROOT / "attribution" / ATTRIBUTION_TAG
    paths = sorted(tag_dir.glob("*.jsonl")) if tag_dir.is_dir() else []
    for path in paths:
        stem = path.stem  # <mode>-<fixture>-<generator>-<supports>-<arm>
        for arm in ("baseline", "candidate"):
            if not stem.endswith(f"-{arm}"):
                continue
            cell = stem[: -(len(arm) + 1)]
            cells.setdefault(cell, {})[arm] = capture_terms(path)
    for cell, arms in cells.items():
        if "baseline" in arms and "candidate" in arms:
            drop = (
                arms["baseline"]["seam_stage_ms"] - arms["candidate"]["seam_stage_ms"]
            )
            arms["stage_drop_ms"] = drop
            arms["stage_drop_pct"] = 100.0 * drop / arms["baseline"]["seam_stage_ms"]
    return {
        "note": (
            "Instrumented captures: attribution only, never a wall claim "
            "(docs/17_agent_debugging.md). `seam_stage_ms` is the "
            "`stage_complete` wall for `PrePass::SeamPlanning`; the host view "
            "projection and the harvest are unchanged between arms, so the "
            "stage delta equals the guest offset delta."
        ),
        "cells": cells,
    }


def main() -> int:
    proof_rows = reduce.load_raw_proof()
    measure_rows = reduce.load_raw_measure()
    missing_proof = [cell for cell, rows in proof_rows.items() if len(rows) != 2]
    incomplete_measure = [
        cell
        for cell, rows in measure_rows.items()
        if len(rows) != 2 + 2 * reduce.PAIRS_REQUIRED
    ]
    if missing_proof:
        print("proof cells incomplete:", missing_proof)
    if incomplete_measure:
        print("measure cells incomplete:", incomplete_measure)

    (HERE / "proof-rows.json").write_text(
        json.dumps(proof_rows, indent=1), encoding="utf-8"
    )
    (HERE / "measure-rows.json").write_text(
        json.dumps(measure_rows, indent=1), encoding="utf-8"
    )
    (HERE / "attribution.json").write_text(
        json.dumps(build_attribution(), indent=1), encoding="utf-8"
    )
    print(
        f"proof cells {sum(1 for rows in proof_rows.values() if len(rows) == 2)}/16, "
        f"measure cells "
        f"{sum(1 for rows in measure_rows.values() if len(rows) == 2 + 2 * reduce.PAIRS_REQUIRED)}/4"
    )
    return 0 if not missing_proof and not incomplete_measure else 1


if __name__ == "__main__":
    raise SystemExit(main())

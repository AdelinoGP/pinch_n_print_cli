#!/usr/bin/env python3
"""Reduce this machine's t89 captures into the committed evidence rows.

Reads `.local-artifacts/perimeter-reference-preparation/t89-exact-z-parallel/`
and writes, next to this script:

  proof-rows.json     16 cells x 2 arms (the qualification rows)
  measure-rows.json    4 cells x (2 warmups + 12 measured) rows
  attribution.json     the instrumented `PrePass::SupportGeometry` post-planner
                       tail per arm

`reduce.py` derives `qualification.json` / `ab-reduction.json` / `summary.json`
from those committed rows, so `verify.py` can reproduce every headline claim in
a plain checkout.
"""

from __future__ import annotations

import csv
import json

import reduce

HERE = reduce.HERE
ROOT = reduce.ROOT
SUPPORT_GEOMETRY_STAGE = "PrePass::SupportGeometry"
PLANNER_MODULE = "com.core.tree-support-planner"


def capture_terms(path) -> dict:
    slice_ms = None
    stage_ms = None
    planner_ms = None
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line.startswith("{"):
                continue
            event = json.loads(line)
            kind = event.get("event")
            if kind == "slice_complete":
                slice_ms = event.get("elapsed_ms")
            elif (
                kind == "stage_complete"
                and event.get("stage") == SUPPORT_GEOMETRY_STAGE
            ):
                stage_ms = (stage_ms or 0.0) + float(event.get("elapsed_ms") or 0.0)
            elif kind == "module_complete" and event.get("module_id") == PLANNER_MODULE:
                planner_ms = (planner_ms or 0.0) + float(event.get("elapsed_ms") or 0.0)
    post_planner_ms = None
    if stage_ms is not None and planner_ms is not None:
        post_planner_ms = stage_ms - planner_ms
    return {
        "slice_ms": slice_ms,
        "support_geometry_stage_ms": stage_ms,
        "planner_module_ms": planner_ms,
        "post_planner_ms": post_planner_ms,
    }


ATTRIBUTION_TAG = "attribution"


def load_pregate_rows() -> list[dict]:
    """Read the pre-gate rows (3 measured pairs + 1 warmup pair)."""
    path = ROOT / "pregate" / "rows.csv"
    if not path.exists():
        return []
    with path.open(encoding="utf-8-sig", newline="") as handle:
        return [dict(row) for row in csv.DictReader(handle)]


def build_probe() -> dict:
    """The exact-Z probe's counters, reduced from the instrumented stderr.

    `entries == distinct_object_z` says an object-level `(object, z)` cache has
    no headroom on this job. The three split terms are accumulated thread-time
    under 12-way concurrency, so they are shares, not wall claims.
    """
    probe = {"source": "probe.patch (env-gated PNP_T89_PROBE)"}
    for name in ("t89-probe", "t89-split"):
        path = HERE.parents[4] / "target" / f"{name}.stderr.log"
        if not path.exists():
            continue
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.startswith("T89PROBE "):
                fields = dict(
                    token.split("=", 1) for token in line.split()[1:] if "=" in token
                )
                probe["counters"] = {
                    "entries": int(fields["entries"]),
                    "admitted": int(fields["admitted"]),
                    "distinct_object_z": int(fields["distinct_object_z"]),
                    "queries": int(fields["queries"]),
                    "misses": int(fields["misses"]),
                    "validate_ms": float(fields["validate_ms"]),
                }
            elif line.startswith("T89SPLIT "):
                fields = dict(
                    token.split("=", 1) for token in line.split()[1:] if "=" in token
                )
                terms = {
                    "mesh_us": int(fields["mesh_us"]),
                    "z_section_us": int(fields["z_section_us"]),
                    "top_section_us": int(fields["top_section_us"]),
                }
                total = sum(terms.values())
                probe["split"] = {
                    "terms_us": terms,
                    "total_us": total,
                    "shares": {key: value / total for key, value in terms.items()},
                }
    return probe


def build_attribution() -> dict:
    """Committed attribution comes from the `attribution` capture tag only.

    The candidate is a host-side change: the planner module wall must be
    unchanged between arms, and the term that moves is the stage's post-module
    tail (the exact-Z validation loop). Instrumented walls are attribution
    only, never wall claims.
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
            base_tail = arms["baseline"]["post_planner_ms"]
            cand_tail = arms["candidate"]["post_planner_ms"]
            if base_tail is None or cand_tail is None:
                continue
            drop = base_tail - cand_tail
            arms["tail_drop_ms"] = drop
            arms["tail_drop_pct"] = 100.0 * drop / base_tail if base_tail else 0.0
    return {
        "note": (
            "Instrumented captures: attribution only, never a wall claim "
            "(docs/17_agent_debugging.md). `support_geometry_stage_ms` is the "
            "`stage_complete` wall for `PrePass::SupportGeometry`; "
            "`post_planner_ms` is that wall minus the tree-support planner's "
            "module bracket, which is the host-side tail the exact-Z validation "
            "loop lives in."
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
    pregate_rows = load_pregate_rows()
    (HERE / "pregate-rows.json").write_text(
        json.dumps(pregate_rows, indent=1), encoding="utf-8"
    )
    (HERE / "probe.json").write_text(
        json.dumps(build_probe(), indent=1), encoding="utf-8"
    )
    print(
        f"proof cells {sum(1 for rows in proof_rows.values() if len(rows) == 2)}/16, "
        f"measure cells "
        f"{sum(1 for rows in measure_rows.values() if len(rows) == 2 + 2 * reduce.PAIRS_REQUIRED)}/4, "
        f"pregate rows {len(pregate_rows)}"
    )
    return 0 if not missing_proof else 1


if __name__ == "__main__":
    raise SystemExit(main())

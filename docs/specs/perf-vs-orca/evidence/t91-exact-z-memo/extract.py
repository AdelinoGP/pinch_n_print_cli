#!/usr/bin/env python3
"""Reduce this machine's t91 captures into the committed evidence rows.

Reads `.local-artifacts/perimeter-reference-preparation/t91-exact-z-memo/`
and writes, next to this script:

  proof-rows.json     16 cells x 2 arms (the qualification rows)
  measure-rows.json    4 cells x (2 warmups + 12 measured) rows
  attribution.json     the instrumented support-geometry stage / planner /
                       post-planner-gap terms per arm
  provenance.json      per-arm host binary SHA and module-tree equality

`reduce.py` derives `qualification.json` / `ab-reduction.json` / `summary.json`
from those committed rows, so `verify.py` can reproduce every headline claim in
a plain checkout.

Attribution terms (instrumented captures only, never a wall claim —
docs/17_agent_debugging.md):

  slice_ms                      `slice_complete` wall
  prepass_phase_ms              `phase_complete` for `prepass`
  support_geometry_stage_ms     `stage_complete` for `PrePass::SupportGeometry`
  planner_module_ms             sum of the support planner's module walls
  aggregation_module_ms         `host:support_plan_aggregation`, when emitted
  post_planner_gap_ms           stage wall minus the planner module walls: the
                                bracket that holds the exact-Z validation tail
"""

from __future__ import annotations

import json
from pathlib import Path

import reduce

HERE = reduce.HERE
ROOT = reduce.ROOT

ATTRIBUTION_TAG = "attribution"
SUPPORT_STAGE = "PrePass::SupportGeometry"
PLANNER_MODULES = (
    "com.core.tree-support-planner",
    "com.core.traditional-support-planner",
)
AGGREGATION_MODULE = "host:support_plan_aggregation"


def parse_capture(path: Path) -> dict:
    events = []
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                events.append(json.loads(line))
            except json.JSONDecodeError:
                continue

    stages: dict[str, float] = {}
    modules: list[dict] = []
    phases: dict[str, float] = {}
    slice_ms = None
    for event in events:
        kind = event.get("event")
        if kind == "stage_complete":
            stages[event["stage"]] = event["elapsed_ms"]
        elif kind == "module_complete":
            modules.append(event)
        elif kind == "phase_complete":
            phases[event["phase"]] = event["elapsed_ms"]
        elif kind == "slice_complete":
            slice_ms = event["elapsed_ms"]

    planner = sum(
        event["elapsed_ms"]
        for event in modules
        if event.get("stage") == SUPPORT_STAGE
        and event.get("module_id") in PLANNER_MODULES
    )
    aggregation = next(
        (
            event["elapsed_ms"]
            for event in modules
            if event.get("module_id") == AGGREGATION_MODULE
        ),
        None,
    )
    stage = stages.get(SUPPORT_STAGE)
    return {
        "slice_ms": slice_ms,
        "prepass_phase_ms": phases.get("prepass"),
        "support_geometry_stage_ms": stage,
        "planner_module_ms": planner if planner else None,
        "aggregation_module_ms": aggregation,
        "post_planner_gap_ms": (stage - planner)
        if stage is not None and planner
        else None,
    }


def build_attribution() -> dict:
    """Committed attribution comes from the `attribution` capture tag only."""
    cells: dict[str, dict] = {}
    tag_dir = ROOT / "attribution" / ATTRIBUTION_TAG
    paths = sorted(tag_dir.glob("*.jsonl")) if tag_dir.is_dir() else []
    for path in paths:
        stem = path.stem  # <mode>-<fixture>-<generator>-<supports>-<arm>
        for arm in ("baseline", "candidate"):
            if not stem.endswith(f"-{arm}"):
                continue
            cell = stem[: -(len(arm) + 1)]
            cells.setdefault(cell, {})[arm] = parse_capture(path)
    for arms in cells.values():
        base = arms.get("baseline")
        cand = arms.get("candidate")
        if base is None or cand is None:
            continue
        base_gap = base.get("post_planner_gap_ms")
        cand_gap = cand.get("post_planner_gap_ms")
        if base_gap and cand_gap is not None:
            drop = base_gap - cand_gap
            arms["post_planner_gap_drop_ms"] = drop
            arms["post_planner_gap_drop_pct"] = 100.0 * drop / base_gap
        base_stage = base.get("support_geometry_stage_ms")
        cand_stage = cand.get("support_geometry_stage_ms")
        if base_stage and cand_stage:
            arms["support_geometry_stage_drop_ms"] = base_stage - cand_stage
    return {
        "note": (
            "Instrumented captures: attribution only, never a wall claim "
            "(docs/17_agent_debugging.md). Each arm is one `--instrument-stderr "
            "--profile --profile-verbose` run. `post_planner_gap_ms` is the "
            "support-geometry stage wall minus the planner module walls — the "
            "bracket that carries the exact-Z validation loop — so its drop is "
            "where the memo removes work, not the cell's wall delta."
        ),
        "cells": cells,
    }


PROBE_PREFIX = "T91PROBE "


def build_memo_probe() -> dict:
    """Parse the probe captures into the direct memo attribution.

    Each capture is one instrumented run of the probe snapshot; the probe
    prints a single `T91PROBE` line with the query / miss / geometry-build
    counts and the exact-Z loop timings (instrumented, never a wall claim).
    """
    capture_dir = ROOT / "probe" / "captures"
    arms: dict[str, dict] = {}
    paths = sorted(capture_dir.glob("*-.jsonl")) if capture_dir.is_dir() else []
    if capture_dir.is_dir():
        paths = sorted(capture_dir.glob("*.jsonl"))
    for path in paths:
        for arm in ("baseline", "candidate"):
            if not path.stem.endswith(f"-{arm}"):
                continue
            for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
                if not line.startswith(PROBE_PREFIX):
                    continue
                fields: dict[str, float | int | str] = {}
                for token in line[len(PROBE_PREFIX) :].split():
                    key, _, value = token.partition("=")
                    try:
                        fields[key] = float(value) if "." in value else int(value)
                    except ValueError:
                        fields[key] = value
                arms[arm] = fields
    if len(arms) != 2:
        return {"available": False, "arms": arms}
    base = arms["baseline"]
    cand = arms["candidate"]
    return {
        "available": True,
        "note": (
            "Direct memo attribution from the probe snapshots "
            "(probe/build-probe.ps1 + run-probe.ps1), ordinary base classic-on. "
            "Instrumented: never a wall claim."
        ),
        "cell": "ordinary-base-classic-on",
        "arms": arms,
        "query_ms_drop": base["query_ms"] - cand["query_ms"],
        "query_ms_drop_pct": 100.0
        * (base["query_ms"] - cand["query_ms"])
        / base["query_ms"],
        "validate_ms_drop": base["validate_ms"] - cand["validate_ms"],
        "geometry_builds_before": base["builds"],
        "geometry_builds_after": cand["builds"],
    }


def build_provenance() -> dict:
    """Snapshot provenance: per-arm binary SHA and module-tree equality.

    The candidate is host-only, so both arms must ship byte-identical guest
    modules; only `pnp_cli.exe` differs. This is what pins the isolation claim
    in the findings without shipping 24 wasm files.
    """
    prov_dir = ROOT / "provenance"
    entries = {}
    for path in sorted(prov_dir.glob("*.json")) if prov_dir.is_dir() else []:
        data = json.loads(path.read_text(encoding="utf-8"))
        # PowerShell's ConvertTo-Json collapses an empty `@()` to null and a
        # one-element array to a scalar; normalise to a list of strings.
        dirty = data.get("candidate_sources_dirty")
        if dirty is None:
            dirty = []
        elif isinstance(dirty, str):
            dirty = [dirty]
        data["candidate_sources_dirty"] = dirty
        entries[f"{data['mode']}-{data['arm']}"] = data
    module_trees_equal = {}
    for mode in ("ordinary", "accelerated"):
        base = entries.get(f"{mode}-baseline")
        cand = entries.get(f"{mode}-candidate")
        if base is None or cand is None:
            continue
        module_trees_equal[mode] = base["modules"] == cand["modules"]
    return {
        "note": (
            "Built by build-snapshots.ps1. `modules` is the sorted per-file "
            "SHA256 list of the snapshot's external module tree; "
            "`candidate_sources_dirty` records which candidate sources were "
            "stashed for the baseline arm, so the arm diff is auditable."
        ),
        "snapshots": entries,
        "module_trees_equal": module_trees_equal,
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
    attribution = build_attribution()
    attribution["memo_probe"] = build_memo_probe()
    (HERE / "attribution.json").write_text(
        json.dumps(attribution, indent=1), encoding="utf-8"
    )
    (HERE / "provenance.json").write_text(
        json.dumps(build_provenance(), indent=1), encoding="utf-8"
    )
    print(
        f"proof cells {sum(1 for rows in proof_rows.values() if len(rows) == 2)}/16, "
        f"measure cells "
        f"{sum(1 for rows in measure_rows.values() if len(rows) == 2 + 2 * reduce.PAIRS_REQUIRED)}/4"
    )
    return 0 if not missing_proof and not incomplete_measure else 1


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Reduce this machine's t90 captures into the committed evidence rows.

Reads `.local-artifacts/perimeter-reference-preparation/t90-postpass-ownership/`
and writes, next to this script:

  proof-rows.json     16 cells x 2 arms (the qualification rows)
  measure-rows.json    4 cells x (2 warmups + 12 measured) rows
  attribution.json     the instrumented PostPass postpass/emit/finalization walls
                       per arm

`reduce.py` derives `qualification.json` / `ab-reduction.json` / `summary.json`
from those committed rows, so `verify.py` can reproduce every headline claim in
a plain checkout.

Attribution terms (instrumented captures only, never a wall claim —
docs/17_agent_debugging.md):

  slice_ms                        `slice_complete` wall
  postpass_phase_ms               `phase_complete` for `PostPass`
  finalization_stage_ms           `stage_complete` for `PostPass::LayerFinalization`
  finalization_before_first_ms    stage start -> first module start (the clone)
  finalization_after_last_ms      last module end -> stage end (the commit drop)
  finalization_modules_ms         sum of the finalization modules' own walls
  emit_stage_ms                   `stage_complete` for `PostPass::GCodeEmit`
  emit_module_ms                  `host:gcode_emit`'s own wall (walk + drop)
  serialize_stage_ms              `stage_complete` for `PostPass::GCodeSerialize`
  postpass_stage_sum_ms           sum of the four postpass stage brackets
  postpass_residual_ms            postpass phase wall minus that sum (teardown)
"""

from __future__ import annotations

import json
from pathlib import Path

import reduce

HERE = reduce.HERE
ROOT = reduce.ROOT

ATTRIBUTION_TAG = "attribution"
POSTPASS_STAGES = (
    "PostPass::LayerFinalization",
    "PostPass::GCodeEmit",
    "PostPass::GCodePostProcess",
    "PostPass::GCodeSerialize",
)


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

    # Stage-gap decomposition for the finalization stage: time before the first
    # module is the whole-print clone, time after the last module is the commit
    # that publishes the working copy and drops the replaced vector.
    before_first = None
    after_last = None
    module_wall = 0.0
    for index, event in enumerate(events):
        if event.get("event") != "stage_start":
            continue
        if event.get("stage") != "PostPass::LayerFinalization":
            continue
        start = event["timestamp_ms"]
        spans = []
        cursor = index + 1
        while cursor < len(events):
            candidate = events[cursor]
            if candidate.get("event") == "stage_start":
                break
            if candidate.get("event") == "stage_complete":
                break
            if candidate.get("event") in ("module_start", "module_complete"):
                spans.append(candidate)
            cursor += 1
        if cursor >= len(events) or events[cursor].get("event") != "stage_complete":
            break
        end = events[cursor]["timestamp_ms"]
        if spans:
            before_first = spans[0]["timestamp_ms"] - start
            after_last = end - spans[-1]["timestamp_ms"]
        break
    for event in modules:
        if event.get("stage") == "PostPass::LayerFinalization":
            module_wall += event["elapsed_ms"]

    emit_module = next(
        (
            event["elapsed_ms"]
            for event in modules
            if event.get("stage") == "PostPass::GCodeEmit"
            and event.get("module_id") == "host:gcode_emit"
        ),
        None,
    )
    stage_sum = sum(stages.get(stage, 0.0) for stage in POSTPASS_STAGES)
    postpass_phase = phases.get("postpass")
    return {
        "slice_ms": slice_ms,
        "postpass_phase_ms": postpass_phase,
        "finalization_stage_ms": stages.get("PostPass::LayerFinalization"),
        "finalization_before_first_ms": before_first,
        "finalization_after_last_ms": after_last,
        "finalization_modules_ms": module_wall if module_wall else None,
        "emit_stage_ms": stages.get("PostPass::GCodeEmit"),
        "emit_module_ms": emit_module,
        "serialize_stage_ms": stages.get("PostPass::GCodeSerialize"),
        "postpass_stage_sum_ms": stage_sum,
        "postpass_residual_ms": (
            postpass_phase - stage_sum if postpass_phase is not None else None
        ),
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
        if base.get("finalization_stage_ms") and cand.get("finalization_stage_ms"):
            drop = base["finalization_stage_ms"] - cand["finalization_stage_ms"]
            arms["finalization_stage_drop_ms"] = drop
            arms["finalization_stage_drop_pct"] = (
                100.0 * drop / base["finalization_stage_ms"]
            )
        if (
            base.get("postpass_residual_ms") is not None
            and cand.get("postpass_residual_ms") is not None
        ):
            arms["postpass_residual_drop_ms"] = (
                base["postpass_residual_ms"] - cand["postpass_residual_ms"]
            )
    return {
        "note": (
            "Instrumented captures: attribution only, never a wall claim "
            "(docs/17_agent_debugging.md). Each arm is one `--instrument-stderr "
            "--profile --profile-verbose` run; the stage brackets and their "
            "before/after-module gaps locate the whole-print clone, the working "
            "copy's drop and the postpass teardown residual, not the cell's wall "
            "delta."
        ),
        "cells": cells,
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
        dirty = data.get("ownership_sources_dirty")
        if dirty is None:
            dirty = []
        elif isinstance(dirty, str):
            dirty = [dirty]
        data["ownership_sources_dirty"] = dirty
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
            "`ownership_sources_dirty` records which candidate sources were "
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
    (HERE / "attribution.json").write_text(
        json.dumps(build_attribution(), indent=1), encoding="utf-8"
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

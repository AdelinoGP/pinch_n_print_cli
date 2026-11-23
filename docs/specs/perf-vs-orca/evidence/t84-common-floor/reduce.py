#!/usr/bin/env python3
"""Derive the t84 common-floor attribution and shape ceilings.

Inputs are the committed per-run captures (``captures/*.json``, produced by
``extract.py`` from the gitignored raw tree).  Output is ``summary.json``, the
single reduced artifact ``verify.py`` checks.

    python reduce.py            # write summary.json
"""

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent

# Pre-declared in the ticket: a shape graduates only if its measured
# occupancy-normalized ceiling is at least this share of cell wall.
FLOOR_PCT = 0.80

# t78 measured this effective speedup for the same `map_batch` fan-out
# mechanism on the tree-support planner's clip batch (12 rayon threads).
REFERENCE_BATCH_SPEEDUP = 2.6


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def pct(part, whole):
    if not whole:
        return None
    return round(100.0 * part / whole, 3)


def capture(cell, label, store=None):
    if store is not None:
        return store[(cell, label)]
    return load(HERE / "captures" / f"{cell}-{label}.json")


def cell_summary(cell, window, store=None):
    head = capture(cell, "head", store)
    probe = capture(cell, "probe", store)

    cell_wall_ms = head["slice_complete_ms"]
    overhang_stage = probe["stages_ms"]["PrePass::OverhangAnnotation"]
    seam_stage = probe["stages_ms"]["PrePass::SeamPlanning"]

    overhang = probe["terms"]["overhang"]
    host = {s["scope"]: s for s in probe["profile"]["overhang_host"]["scopes"]}
    offsets = host.get("polygon_ops::offset", {})
    clips = host.get("polygon_ops::clip_polygons", {})
    kernel_native_ms = offsets.get("wall_ms", 0.0) + clips.get("wall_ms", 0.0)
    occupancy = (
        round(kernel_native_ms / overhang["kernel_ms"], 3)
        if overhang["kernel_ms"]
        else None
    )

    host_terms = (
        overhang.get("gather_ms", 0.0)
        + overhang.get("merge_ms", 0.0)
        + overhang.get("clone_ms", 0.0)
        + overhang.get("commit_ms", 0.0)
    )

    seam_project = probe["terms"]["seam_project"]
    seam_call = probe["terms"]["seam_call"]
    seam_harvest = probe["terms"]["seam_harvest"]
    scopes = {s["scope"]: s for s in probe["profile"]["seam"]["scopes"]}
    seam_offsets = scopes.get("t84.offset_project", {}).get("wall_ms", 0.0)
    seam_candidates = scopes.get("t84.candidates", {}).get("wall_ms", 0.0)
    seam_choose = scopes.get("t84.choose", {}).get("wall_ms", 0.0)
    seam_unscoped = round(
        probe["profile"]["seam"]["module_wall_ms"]
        - seam_offsets
        - seam_candidates
        - seam_choose,
        3,
    )

    # Upper bound if every offset vanished; the realistic bound applies the
    # t78 reference effective speedup to the same fan-out mechanism.
    overhang_offset_free_ms = (
        offsets.get("wall_ms", 0.0) / occupancy if occupancy else 0.0
    )
    seam_batched_ms = seam_offsets * (1.0 - 1.0 / REFERENCE_BATCH_SPEEDUP)
    window_removable = 1.0 - window["retained_frac"]
    overhang_window_ms = window_removable * overhang_offset_free_ms

    shapes = [
        {
            "name": "seam-batch-inset-offsets",
            "basis": "495 independent host::offset_polygons calls (one per region) "
            "inside the guest loop; t78's existing offset_polygons_batch uses the "
            "same input-ordered map_batch fan-out",
            "ceiling_upper_pct": pct(seam_offsets, cell_wall_ms),
            "ceiling_at_reference_speedup_pct": pct(seam_batched_ms, cell_wall_ms),
            "verdict": "GRADUATE"
            if pct(seam_batched_ms, cell_wall_ms) >= FLOOR_PCT
            else "DROP",
        },
        {
            "name": "overhang-bbox-window-operand",
            "basis": f"bbox window retains {window['retained_frac']:.6f} of the "
            "previous-layer points, so at most "
            f"{100.0 * window_removable:.3f} % of the operand is removable",
            "ceiling_upper_pct": pct(overhang_window_ms, cell_wall_ms),
            "ceiling_at_reference_speedup_pct": None,
            "verdict": "GRADUATE"
            if pct(overhang_window_ms, cell_wall_ms) >= FLOOR_PCT
            else "DROP",
        },
        {
            "name": "seam-candidate-list",
            "basis": "2,553,550 candidates are generated, scored, harvested and "
            "retained in SeamPlanIR; no production consumer reads "
            "SeamPlanEntry.scored_candidates",
            "ceiling_upper_pct": pct(
                seam_candidates + seam_choose + seam_harvest["harvest_ms"] + seam_unscoped,
                cell_wall_ms,
            ),
            "ceiling_at_reference_speedup_pct": None,
            "verdict": "DROP",
        },
    ]

    return {
        "cell_wall_ms": cell_wall_ms,
        "process_wall_s": head["wall_seconds"],
        "process_cpu_s": head["cpu_seconds"],
        "output": {
            "sha256": head["sha256"],
            "bytes": head["bytes"],
            "status": head["status"],
            "degraded": head["degraded"],
        },
        "overhang": {
            "stage_ms_head": head["stages_ms"]["PrePass::OverhangAnnotation"],
            "stage_ms_probe": overhang_stage,
            "stage_share_of_cell_pct": pct(
                head["stages_ms"]["PrePass::OverhangAnnotation"], cell_wall_ms
            ),
            "kernel_ms": overhang["kernel_ms"],
            "kernel_share_of_stage_pct": pct(overhang["kernel_ms"], overhang_stage),
            "host_terms_ms": round(host_terms, 3),
            "host_terms_share_of_stage_pct": pct(host_terms, overhang_stage),
            "native_offset_ms": offsets.get("wall_ms"),
            "native_offset_calls": offsets.get("calls"),
            "native_clip_ms": clips.get("wall_ms"),
            "native_clip_calls": clips.get("calls"),
            "native_occupancy": occupancy,
            "offset_share_of_kernel_work_pct": pct(
                offsets.get("wall_ms", 0.0), kernel_native_ms
            ),
            "offset_free_saving_ms": round(overhang_offset_free_ms, 3),
            "offset_free_saving_pct": pct(overhang_offset_free_ms, cell_wall_ms),
            "counts": {
                key: overhang[key]
                for key in (
                    "objects",
                    "layer_entries",
                    "polys",
                    "points",
                    "band_layers",
                    "bands",
                    "prev_layers",
                    "prev_polys",
                    "prev_points",
                )
            },
        },
        "seam": {
            "stage_ms_head": head["stages_ms"]["PrePass::SeamPlanning"],
            "stage_ms_probe": seam_stage,
            "stage_share_of_cell_pct": pct(
                head["stages_ms"]["PrePass::SeamPlanning"], cell_wall_ms
            ),
            "project_ms": seam_project["project_ms"],
            "project_share_of_stage_pct": pct(seam_project["project_ms"], seam_stage),
            "call_ms": seam_call["call_ms"],
            "push_ms": seam_call["push_ms"],
            "pushes": seam_call["pushes"],
            "harvest_ms": seam_harvest["harvest_ms"],
            "harvest_share_of_stage_pct": pct(seam_harvest["harvest_ms"], seam_stage),
            "offset_project_ms": seam_offsets,
            "offset_project_share_of_stage_pct": pct(seam_offsets, seam_stage),
            "offset_project_share_of_cell_pct": pct(seam_offsets, cell_wall_ms),
            "candidates_ms": seam_candidates,
            "candidates_share_of_stage_pct": pct(seam_candidates, seam_stage),
            "choose_ms": seam_choose,
            "unscoped_module_ms": seam_unscoped,
            "unscoped_share_of_stage_pct": pct(seam_unscoped, seam_stage),
            "counts": {
                "regions": seam_project["regions"],
                "polys": seam_project["polys"],
                "points": seam_project["points"],
                "annotations": seam_project["annotations"],
                "entries": seam_harvest["entries"],
                "candidates": seam_harvest["candidates"],
            },
        },
        "shapes": shapes,
    }


def build(store=None):
    probe3 = capture("base-classic-off", "probe3", store)
    window = probe3["terms"]["overhang_window"]
    return {
        "floor_pct_of_cell_wall": FLOOR_PCT,
        "operand_window": {
            "prev_points": window["prev_points"],
            "prev_in_window": window["prev_in_window"],
            "retained_frac": round(window["retained_frac"], 6),
            "removable_frac": round(1.0 - window["retained_frac"], 6),
            "measured_on": "base-classic-off probe3",
        },
        "reference_batch_speedup": REFERENCE_BATCH_SPEEDUP,
        "cells": {
            cell: cell_summary(cell, window, store)
            for cell in ("base-classic-off", "base-classic-on")
        },
    }


def main():
    summary = build()
    (HERE / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    print("wrote summary.json")


if __name__ == "__main__":
    main()

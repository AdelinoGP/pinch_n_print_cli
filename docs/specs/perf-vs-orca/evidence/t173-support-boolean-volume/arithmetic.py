#!/usr/bin/env python3
"""t173: price the support-path boolean volume and the shapes that survive.

Every input is a committed artifact from this take or another one. The script
writes `reduced/arithmetic.json`; `verify.py` re-derives every quoted number
from that file plus the same inputs.

Regimes, never mixed:
  - The probe census is accumulated thread-time and operand counts. It is
    attribution only; probe bookkeeping and `--profile` both inflate it.
  - The instrumented stage share is an instrumented wall fraction. Multiplying
    it by t172's uninstrumented paired wall is a composition across windows,
    labelled an estimate.
  - The route ratios are t172's own two-step composition: t163's ordinary board
    x (1 - t169 saving) x (1 - t172 saving). They are estimates, not ratios of
    record; t161 forbids carrying a required cut across windows as a
    measurement, and t174 is the ticket that replaces them with a fresh board.
  - A shape price is a ceiling: it assumes the reduced operand costs its
    point/edge share and nothing else. It is not a gate result and no shape here
    was run on the standing gate.
"""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
EVID = HERE.parent
CELLS = [
    "base-classic-on",
    "base-arachne-on",
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-off",
]
CLIP_OPS = {"union", "intersection", "difference", "xor"}


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def route(t163: dict, t169: dict, t172: dict) -> dict:
    out = {}
    for cell in CELLS:
        orca = t163["cells"][cell]["orca"]["wall_median_s"]
        pnp_t163 = t163["cells"][cell]["pnp-ordinary"]["wall_median_s"]
        ratio_163 = pnp_t163 / orca
        s169 = t169[cell]["wall"]["median_saving_pct"] / 100.0
        s172 = t172[cell]["wall"]["median_saving_pct"] / 100.0
        pnp_172 = t172[cell]["wall"]["candidate_median_s"]
        ratio = ratio_163 * (1.0 - s169) * (1.0 - s172)
        out[cell] = {
            "t163_ratio": round(ratio_163, 5),
            "t169_saving": round(s169, 6),
            "t172_saving": round(s172, 6),
            "estimated_ratio": round(ratio, 5),
            "required_cut": round(1.0 - 1.0 / ratio, 6),
            "pnp_median_s": pnp_172,
            "orca_s": round(orca, 5),
        }
    return out


def stage_split(instrumented: dict, route_: dict) -> dict:
    out = {}
    for cell in CELLS:
        if cell not in instrumented["cells"]:
            continue
        rec = instrumented["cells"][cell]
        median = rec["median"]
        total_ms = median["slice_complete_ms"]
        stage_ms = 0
        for p in rec["passes"]:
            stage_ms = max(
                stage_ms, p["prepass_stages_ms"].get("PrePass::SupportGeometry", 0)
            )
        # median stage wall across passes (the median pass's table is not
        # guaranteed to be the median stage, so take the median directly).
        stage_values = sorted(
            p["prepass_stages_ms"].get("PrePass::SupportGeometry", 0)
            for p in rec["passes"]
        )
        stage_median = stage_values[len(stage_values) // 2]
        share = stage_median / total_ms
        out[cell] = {
            "slice_complete_ms": total_ms,
            "phase_shares": median["shares"],
            "stage_ms_instrumented": stage_median,
            "stage_share_of_cell": round(share, 6),
            "stage_wall_estimate_s": round(route_[cell]["pnp_median_s"] * share, 4),
            "stage_share_min": round(stage_values[0] / total_ms, 6),
            "stage_share_max": round(stage_values[-1] / total_ms, 6),
        }
    return out


def cell_census(probe: dict) -> dict:
    out = {}
    for cell in CELLS:
        if cell not in probe["cells"]:
            continue
        rec = probe["cells"][cell]
        rows = rec["rows"]
        batches = rec["batches"]
        # Site -> op kind, for matching batch walls (batches carry no op).
        site_op = {}
        for row in rows:
            site_op[(row["module"], row["site"])] = row["op"]
        planner_clip_thread = 0
        planner_offset_thread = 0
        all_clip_thread = 0
        all_offset_thread = 0
        for row in rows:
            planner = row["module"] == "com.core.tree-support-planner"
            if row["op"] in CLIP_OPS:
                all_clip_thread += row["thread_ns"]
                if planner:
                    planner_clip_thread += row["thread_ns"]
            else:
                all_offset_thread += row["thread_ns"]
                if planner:
                    planner_offset_thread += row["thread_ns"]
        planner_clip_wall = 0
        planner_offset_wall = 0
        all_clip_wall = 0
        for b in batches:
            planner = b["module"] == "com.core.tree-support-planner"
            op = site_op.get((b["module"], b["site"]), "offset")
            if op in CLIP_OPS:
                all_clip_wall += b["wall_ns"]
                if planner:
                    planner_clip_wall += b["wall_ns"]
            elif planner:
                planner_offset_wall += b["wall_ns"]
        # Serial (non-batched) clip calls are single-threaded: their thread time
        # is their wall. They are included in the thread sums above.
        serial_clip_wall = sum(
            r["thread_ns"]
            for r in rows
            if r["op"] in CLIP_OPS and r["entry"] == "wit-serial"
        )
        phase_rows = {
            r["site"]: r for r in rows if r["module"] == "com.core.tree-support-planner"
        }
        phase_batches = {
            b["site"]: b
            for b in batches
            if b["module"] == "com.core.tree-support-planner"
        }
        phases = {}
        for site, row in phase_rows.items():
            wall = phase_batches.get(site, {}).get("wall_ns", 0)
            phases[site] = {
                "op": row["op"],
                "calls": row["calls"],
                "thread_ms": round(row["thread_ns"] / 1e6, 1),
                "wall_ms": round(wall / 1e6, 1),
                "fanout": round(row["thread_ns"] / wall, 3) if wall else None,
                "in_points": row["in_points"],
                "clip_points": row["clip_points"],
                "out_points": row["out_points"],
                "in_bbox_points": row["in_bbox_points"],
                "near_points": row["near_points"],
                "empty_calls": row["empty_calls"],
                "empty_thread_ms": round(row["empty_thread_ns"] / 1e6, 1),
            }
        out[cell] = {
            "total_probe_thread_ms": round(rec["total_clip_thread_ns"] / 1e6, 1),
            "planner_clip_thread_ms": round(planner_clip_thread / 1e6, 1),
            "planner_offset_thread_ms": round(planner_offset_thread / 1e6, 1),
            "planner_clip_wall_ms": round(planner_clip_wall / 1e6, 1),
            "planner_offset_wall_ms": round(planner_offset_wall / 1e6, 1),
            "planner_clip_plus_offset_wall_ms": round(
                (planner_clip_wall + planner_offset_wall) / 1e6, 1
            ),
            "all_clip_thread_ms": round(all_clip_thread / 1e6, 1),
            "all_offset_thread_ms": round(all_offset_thread / 1e6, 1),
            "all_clip_wall_ms": round(all_clip_wall / 1e6, 1),
            "serial_clip_wall_ms": round(serial_clip_wall / 1e6, 1),
            "planner_share_of_clip_thread": round(
                planner_clip_thread / all_clip_thread, 6
            )
            if all_clip_thread
            else None,
            "phases": dict(sorted(phases.items(), key=lambda kv: -kv[1]["thread_ms"])),
            "batch_rows": batches,
            "profile_wall_scopes": rec["profile_wall_scopes"]["wall"],
        }
    return out


def shape_prices(census: dict, split: dict, route_: dict) -> dict:
    out = {}
    for cell in CELLS:
        c = census[cell]
        s = split[cell]
        r = route_[cell]
        cut_s = r["required_cut"] * r["pnp_median_s"]
        p1 = c["phases"].get("planner::batched_phase1_carve")
        p3 = c["phases"].get("planner::batched_phase3_union")
        prices = {}
        if r["estimated_ratio"] <= 1.0:
            out[cell] = {
                "required_cut_s": round(cut_s, 4),
                "pnp_median_s": r["pnp_median_s"],
                "orca_s": r["orca_s"],
                "stage_wall_estimate_s": s["stage_wall_estimate_s"],
                "prices": {},
                "not_applicable": "cell is estimated below 1.0x; supports-off control",
            }
            continue
        if p1:
            subj = 1.0
            # Edge-share ceiling: a bbox-windowed operand would carry the
            # in-bbox clip points plus the subject's own edges. The subject
            # edges are the floor, so the ceiling cannot exceed this share.
            total_edges = p1["in_points"]
            windowed_edges = p1["in_points"] - p1["clip_points"] + p1["in_bbox_points"]
            edge_share = windowed_edges / total_edges
            saving_thread = p1["thread_ms"] * (1.0 - edge_share)
            saving_wall = saving_thread / p1["fanout"]
            prices["phase1_bbox_window_ceiling"] = {
                "thread_ms": p1["thread_ms"],
                "wall_ms": p1["wall_ms"],
                "fanout": p1["fanout"],
                "clip_points": p1["clip_points"],
                "in_bbox_points": p1["in_bbox_points"],
                "in_bbox_share": round(p1["in_bbox_points"] / p1["clip_points"], 9),
                "windowed_edge_share": round(edge_share, 9),
                "saving_thread_ms": round(saving_thread, 1),
                "saving_wall_ms": round(saving_wall, 1),
                "pct_of_cell_wall": round(
                    100.0 * saving_wall / (r["pnp_median_s"] * 1000.0), 2
                ),
                "pct_of_required_cut": round(100.0 * saving_wall / (cut_s * 1000.0), 2),
                "ratio_after": round(
                    (r["pnp_median_s"] * 1000.0 - saving_wall) / (r["orca_s"] * 1000.0),
                    4,
                ),
            }
            # The cheap exact shape: drop clip polygons whose bbox is disjoint
            # from the subject bbox before flattening/sweeping.
            near_share = p1["near_points"] / p1["clip_points"]
            saving_thread_near = p1["thread_ms"] * (1.0 - near_share)
            saving_wall_near = saving_thread_near / p1["fanout"]
            prices["phase1_bbox_disjoint_polygon_filter"] = {
                "near_share": round(near_share, 6),
                "saving_wall_ms": round(saving_wall_near, 1),
                "pct_of_required_cut": round(
                    100.0 * saving_wall_near / (cut_s * 1000.0), 2
                ),
                "ratio_after": round(
                    (r["pnp_median_s"] * 1000.0 - saving_wall_near)
                    / (r["orca_s"] * 1000.0),
                    4,
                ),
            }
        if p3:
            prices["phase3_union_free_ceiling"] = {
                "thread_ms": p3["thread_ms"],
                "wall_ms": p3["wall_ms"],
                "fanout": p3["fanout"],
                "pct_of_required_cut": round(
                    100.0 * p3["wall_ms"] / (cut_s * 1000.0), 2
                ),
                "ratio_after": round(
                    (r["pnp_median_s"] * 1000.0 - p3["wall_ms"])
                    / (r["orca_s"] * 1000.0),
                    4,
                ),
            }
        prices["all_planner_clip_free_ceiling"] = {
            "wall_ms": c["planner_clip_wall_ms"],
            "pct_of_required_cut": round(
                100.0 * c["planner_clip_wall_ms"] / (cut_s * 1000.0), 2
            ),
            "ratio_after": round(
                (r["pnp_median_s"] * 1000.0 - c["planner_clip_wall_ms"])
                / (r["orca_s"] * 1000.0),
                4,
            ),
        }
        prices["all_planner_clip_plus_offset_free_ceiling"] = {
            "wall_ms": c["planner_clip_plus_offset_wall_ms"],
            "pct_of_required_cut": round(
                100.0 * c["planner_clip_plus_offset_wall_ms"] / (cut_s * 1000.0), 2
            ),
            "ratio_after": round(
                (r["pnp_median_s"] * 1000.0 - c["planner_clip_plus_offset_wall_ms"])
                / (r["orca_s"] * 1000.0),
                4,
            ),
        }
        out[cell] = {
            "required_cut_s": round(cut_s, 4),
            "pnp_median_s": r["pnp_median_s"],
            "orca_s": r["orca_s"],
            "stage_wall_estimate_s": s["stage_wall_estimate_s"],
            "prices": prices,
        }
    return out


def decomposition(census: dict, split: dict, profile: dict) -> dict:
    out = {}
    for cell in CELLS:
        if cell not in census or cell not in split:
            continue
        c = census[cell]
        s = split[cell]
        fuel = profile["cells"][cell]["fuel"]
        planner_fuel = fuel["modules"].get("com.core.tree-support-planner", {})
        stage_wall_ms = s["stage_wall_estimate_s"] * 1000.0
        host_clipper_ms = c["planner_clip_plus_offset_wall_ms"]
        out[cell] = {
            "stage_wall_estimate_ms": round(stage_wall_ms, 1),
            "planner_host_clip_ms": c["planner_clip_wall_ms"],
            "planner_host_offset_ms": c["planner_offset_wall_ms"],
            "planner_host_clipper_share_of_stage": round(
                host_clipper_ms / stage_wall_ms, 4
            )
            if stage_wall_ms
            else None,
            "stage_remainder_ms": round(stage_wall_ms - host_clipper_ms, 1),
            "planner_fuel": planner_fuel.get("amount"),
            "cell_fuel": fuel["total"],
            "planner_fuel_share": round(
                planner_fuel.get("amount", 0) / fuel["total"], 6
            ),
            "planner_clip_thread_ms": c["planner_clip_thread_ms"],
            "planner_offset_thread_ms": c["planner_offset_thread_ms"],
            "planner_clip_share_of_clipper_thread": (
                round(
                    c["planner_clip_thread_ms"]
                    / (c["planner_clip_thread_ms"] + c["planner_offset_thread_ms"]),
                    6,
                )
                if (c["planner_clip_thread_ms"] + c["planner_offset_thread_ms"])
                else None
            ),
        }
    return out


def main() -> int:
    instrumented = load(HERE / "reduced" / "instrumented.json")
    instrumented_combined = load(HERE / "reduced" / "instrumented-combined.json")
    probe = load(HERE / "reduced" / "census-probe.json")
    probe_combined = load(HERE / "reduced" / "census-combined.json")
    profile = load(HERE / "reduced" / "profile-detail-probe.json")
    t163 = load(EVID / "t163-accelerated-board" / "reduced" / "summary.json")
    t169 = load(EVID / "t169-offset-decimation-flat-opening" / "reduced" / "ab.json")
    t172 = load(EVID / "t172-offset-concave-crossing" / "reduced" / "ab.json")

    route_ = route(t163, t169, t172)
    # The two base supports-on cells were re-taken with the probe and the
    # instrumented stream in one window (quieter box); benchy and the control
    # keep the probe-only take.
    split = stage_split(instrumented, route_)
    census = cell_census(probe)
    # Corroboration only: the two base cells were re-taken with the probe and
    # the instrumented stream in one window. This is not the price basis.
    combined_census = cell_census(probe_combined)
    combined_split = stage_split(instrumented_combined, route_)
    combined_decomp = decomposition(combined_census, combined_split, profile)
    prices = shape_prices(census, split, route_)
    decomp = decomposition(census, split, profile)

    payload = {
        "note": (
            "ESTIMATE chain, never a ratio of record. Shares are this take's "
            "instrumented median over three passes; walls of record are t172's "
            "paired candidate medians; the route table is t163's board composed "
            "with t169's and t172's measured savings. Shape prices are ceilings "
            "from measured operand counts, not gate results."
        ),
        "cells": CELLS,
        "route": route_,
        "split": split,
        "census": census,
        "decomposition": decomp,
        "combined_window": combined_decomp,
        "prices": prices,
    }
    out = HERE / "reduced" / "arithmetic.json"
    out.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")

    print("route (t163 x t169 x t172):")
    for cell in CELLS:
        r = route_[cell]
        print(
            f"  {cell:20s} {r['estimated_ratio']:5.3f}x cut {100 * r['required_cut']:5.1f}% "
            f"pnp {r['pnp_median_s']:7.3f}s orca {r['orca_s']:7.3f}s"
        )
    print("support-path census (probe):")
    for cell in CELLS:
        c = census[cell]
        print(
            f"  {cell:20s} planner clip {c['planner_clip_thread_ms'] / 1000:7.1f}s thread "
            f"{c['planner_clip_wall_ms'] / 1000:6.2f}s wall  "
            f"offset {c['planner_offset_wall_ms'] / 1000:5.2f}s wall  "
            f"stage est {split[cell]['stage_wall_estimate_s']:6.2f}s"
        )
    print("shape ceilings (fraction of the cell's required cut):")
    for cell in CELLS:
        p = prices[cell]["prices"]
        p1 = p.get("phase1_bbox_window_ceiling")
        p3 = p.get("phase3_union_free_ceiling")
        if not p:
            print(f"  {cell:20s} not applicable (estimated below 1.0x)")
            continue
        print(
            f"  {cell:20s} "
            + (
                f"phase1-window {p1['pct_of_required_cut']:6.1f}% -> {p1['ratio_after']:.3f}x  "
                if p1
                else ""
            )
            + (f"phase3-free {p3['pct_of_required_cut']:5.1f}%  " if p3 else "")
            + f"all-clip-free {p['all_planner_clip_free_ceiling']['pct_of_required_cut']:6.1f}%"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

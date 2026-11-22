#!/usr/bin/env python3
"""Checkout-only arithmetic calculator for Wayfinder Ticket 178.

Re-derives all production Phase-3 shares, mechanism route projections (ESTIMATE ONLY),
t137 4-cell supports-on floor multiples, and geometric audit summaries from committed
reduced JSONs only (no raw captures, no fixtures, Python 3 stdlib only).
"""

from __future__ import annotations

import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REDUCED = HERE / "reduced"
T177_ARITH = HERE.parent / "t177-post-t175-attribution" / "reduced" / "arithmetic.json"
T175_REF = HERE.parent / "t175-tiled-collision-carve" / "reference.json"
FLOOR_PCT = 0.80

SUPPORTS_ON_CELLS = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]

KEY_MECHANISMS = [
    "M0_baseline_head_tail",
    "M1_subj_all_clip_empty",
    "M2_cc_bypass_head_tail",
    "M2b_cc_bypass_subj_all",
    "M3_convex_containment_prune",
    "M4_hier_morton_k4_head_tail",
    "M4_hier_morton_k8_head_tail",
    "M4_hier_morton_k16_head_tail",
    "M4c_hier_morton_2lvl_k16",
    "M7_prune_hier_morton_k4",
    "M7_prune_hier_morton_k8",
    "M7b_prune_hier_2lvl_k16",
    "M8_host_worker_hier_k4",
    "M8_host_worker_hier_k8",
    "M9_host_worker_prune_hier_k4",
    "M9_host_worker_prune_hier_k8",
    "M9_host_worker_prune_hier_k16",
    "M9n_host_nested_prune_hier_k4",
    "M9n_host_nested_prune_hier_k8",
    "M5a_lpt_within_batch12",
    "M5b_global_single_batch",
    "M6_presimplify_50u",
    "M6_presimplify_125u",
]


def compute_arithmetic() -> dict:
    prod = json.loads((REDUCED / "prod_timing.json").read_text(encoding="utf-8"))
    rep_benchy = json.loads((REDUCED / "replay_benchy.json").read_text(encoding="utf-8"))
    rep_base = json.loads((REDUCED / "replay_base.json").read_text(encoding="utf-8"))
    t177 = json.loads(T177_ARITH.read_text(encoding="utf-8"))
    t175_ref = json.loads(T175_REF.read_text(encoding="utf-8"))

    # Median uninstrumented total cell wall from t178 clean timing runs
    cell_walls_ms: dict[str, float] = {}
    for cell in SUPPORTS_ON_CELLS:
        ws = [
            r["wall_s"] * 1000.0
            for r in prod["runs"]
            if r["cell"] == cell and r["kind"] == "timing"
        ]
        cell_walls_ms[cell] = round(statistics.median(ws), 2)

    # Index replay mechanisms by model
    replay_by_model = {
        "benchy": {m["name"]: m for m in rep_benchy["mechanisms"]},
        "base": {m["name"]: m for m in rep_base["mechanisms"]},
    }

    # Production Phase-3 baseline summary per supports-on cell
    prod_cells = {}
    for cell in SUPPORTS_ON_CELLS:
        cinfo = prod["cells"][cell]
        mp = cinfo["median_pass"]
        r177 = t177["route"][cell]
        s177 = t177["split"][cell]
        p3_wall_ms = cinfo["batch_wall_ms_median"]
        p3_thread_ms = cinfo["clip_thread_ms_median"]
        tot_wall_ms = cell_walls_ms[cell]
        p3_share_pct = 100.0 * p3_wall_ms / tot_wall_ms
        # Also compute zero-Phase-3 ceiling
        est_ord = r177["est_post_t175_ratio_ord"]
        est_acc = r177["est_post_t175_ratio_acc"]
        req_cut_ord_pct = round(100.0 * r177["required_cut_1x_ord"], 3)
        req_cut_acc_pct = round(100.0 * r177["required_cut_1x_acc"], 3)
        t181_tail_pct = round(100.0 * s177["support_geometry_post_planner_tail_share_of_cell"], 3)
        prod_cells[cell] = {
            "capture_cell_wall_ms_median": tot_wall_ms,
            "prod_phase3_wall_ms_median": p3_wall_ms,
            "prod_phase3_thread_ms_median": p3_thread_ms,
            "prod_phase3_eff_par": mp["eff_par_clip"],
            "prod_phase3_share_of_cell_pct": round(p3_share_pct, 3),
            "prod_phase3_floor_multiple_if_zero": round(p3_share_pct / FLOOR_PCT, 2),
            "prod_sum_batch_max_clip_ms": mp["sum_batch_max_clip_ms"],
            "prod_max_req_share_of_wall_pct": mp["max_req_share_of_wall_pct"],
            "prod_ideal_12t_floor_ms": mp["ideal_12t_floor_ms"],
            "prod_conversion_overhead_pct": mp["conversion_overhead_pct"],
            "est_ratio_ord_pre": est_ord,
            "est_ratio_acc_pre": est_acc,
            "req_cut_ord_pct": req_cut_ord_pct,
            "req_cut_acc_pct": req_cut_acc_pct,
            "est_ratio_ord_if_phase3_zero": round(est_ord * (1.0 - p3_share_pct / 100.0), 4),
            "est_ratio_acc_if_phase3_zero": round(est_acc * (1.0 - p3_share_pct / 100.0), 4),
            "t181_host_tail_cell_pct": t181_tail_pct,
        }

    # Route projection per mechanism across all 4 supports-on cells
    mechanism_route = {}
    for mname in KEY_MECHANISMS:
        per_cell = {}
        all_4_clear_floor = True
        for cell in SUPPORTS_ON_CELLS:
            model = "benchy" if cell.startswith("benchy-") else "base"
            m = replay_by_model[model][mname]
            pc = prod_cells[cell]
            frac_wall_cut = m["wall_vs_m0_pct"] / 100.0
            frac_thread_cut = m["clip_thread_vs_m0_pct"] / 100.0
            proj_save_ms = pc["prod_phase3_wall_ms_median"] * frac_wall_cut
            proj_thread_save_ms = pc["prod_phase3_thread_ms_median"] * frac_thread_cut
            proj_cell_cut_pct = 100.0 * proj_save_ms / pc["capture_cell_wall_ms_median"]
            floor_x = proj_cell_cut_pct / FLOOR_PCT
            if floor_x < 1.0:
                all_4_clear_floor = False
            est_ord_after = pc["est_ratio_ord_pre"] * (1.0 - proj_cell_cut_pct / 100.0)
            est_acc_after = pc["est_ratio_acc_pre"] * (1.0 - proj_cell_cut_pct / 100.0)
            t181_pct = pc["t181_host_tail_cell_pct"]
            # Additive conservative stack with t181 host tail (50% and 100% of tail)
            est_ord_plus_t181_half = pc["est_ratio_ord_pre"] * (
                1.0 - (proj_cell_cut_pct + 0.5 * t181_pct) / 100.0
            )
            est_ord_plus_t181_full = pc["est_ratio_ord_pre"] * (
                1.0 - (proj_cell_cut_pct + t181_pct) / 100.0
            )
            share_of_req_cut_ord_pct = (
                100.0 * proj_cell_cut_pct / pc["req_cut_ord_pct"]
                if pc["req_cut_ord_pct"] > 0
                else 0.0
            )
            per_cell[cell] = {
                "offline_wall_vs_m0_pct": round(m["wall_vs_m0_pct"], 2),
                "offline_thread_vs_m0_pct": round(m["clip_thread_vs_m0_pct"], 2),
                "projected_wall_save_ms": round(proj_save_ms, 1),
                "projected_thread_save_ms": round(proj_thread_save_ms, 1),
                "projected_cell_cut_pct": round(proj_cell_cut_pct, 3),
                "t137_floor_multiple": round(floor_x, 2),
                "share_of_req_cut_ord_pct": round(share_of_req_cut_ord_pct, 2),
                "est_ratio_ord_after": round(est_ord_after, 4),
                "est_ratio_acc_after": round(est_acc_after, 4),
                "crosses_1x_ord": est_ord_after < 1.0,
                "crosses_1x_acc": est_acc_after < 1.0,
                "est_ratio_ord_plus_t181_half": round(est_ord_plus_t181_half, 4),
                "est_ratio_ord_plus_t181_full": round(est_ord_plus_t181_full, 4),
                "diff_calls": m["diff_calls"],
                "total_calls": m["total_calls"],
                "total_xor_area_mm2": m["total_xor_area_mm2"],
                "max_call_xor_pct_of_union": m["max_call_xor_pct_of_union"],
                "max_boundary_hausdorff_um": round(m["max_boundary_hausdorff_um"], 4),
                "max_vertex_shift_um": round(m["max_vertex_shift_um"], 3),
            }
        mechanism_route[mname] = {
            "clears_t137_4cell_floor": all_4_clear_floor,
            "cells": per_cell,
        }

    # Qualification summary against t175 active reference set
    qual_runs = [r for r in prod["runs"] if r["kind"].startswith("qual")]
    timing_runs = [r for r in prod["runs"] if r["kind"] == "timing"]
    all_runs_ok = all(r["ok"] for r in prod["runs"]) and prod["capture_meta"]["failures"] == 0
    all_match_ref = all(
        r["gcode_sha256"] == t175_ref["active_set"][r["cell"]]
        and r["gcode_bytes"] == t175_ref["bytes"][r["cell"]]
        for r in prod["runs"]
    )
    all_qual_clean = all(
        r["degraded"] is False
        and r["fatal_error_count"] == 0
        and r["non_fatal_error_count"] == 0
        and r["code_1200_count"] == 0
        for r in qual_runs
    )

    return {
        "note": "ESTIMATE ONLY for cross-window post-t175 route ratios (t174 quiet board x t175 paired saving x t178 Phase-3 projection). Never a three-tool scoreboard ratio of record.",
        "qualification": {
            "total_runs": len(prod["runs"]),
            "qual_runs": len(qual_runs),
            "timing_runs": len(timing_runs),
            "all_runs_ok": all_runs_ok,
            "all_match_t175_reference": all_match_ref,
            "all_qual_clean": all_qual_clean,
            "classic_arachne_operand_identity": prod["classic_arachne_operand_identity"],
            "benchy_baseline_exact_replay_ok": rep_benchy["baseline_exact_replay_ok"],
            "base_baseline_exact_replay_ok": rep_base["baseline_exact_replay_ok"],
        },
        "census": {
            "benchy": {
                "batches": rep_benchy["batches"],
                "calls": rep_benchy["total_calls"],
                "in_polys": rep_benchy["total_in_polys"],
                "in_holes": rep_benchy["total_in_holes"],
                "in_verts": rep_benchy["total_in_verts"],
                "out_polys": rep_benchy["total_out_polys"],
                "out_holes": rep_benchy["total_out_holes"],
                "out_verts": rep_benchy["total_out_verts"],
                "vertex_collapse_ratio": round(rep_benchy["vertex_collapse_ratio"], 5),
                "area_overlap_factor": round(rep_benchy["area_overlap_factor"], 3),
                "avg_nodes_values": rep_benchy["avg_nodes_values"],
                "simplify_active_calls": rep_benchy["simplify_active_calls"],
                "exact_duplicate_polys": rep_benchy["exact_duplicate_polys_in_calls"],
                "identical_layer_role_repeats": rep_benchy["identical_layer_role_input_repeats"],
                "convex_contained_polys": rep_benchy["convex_contained_polys_prunable"],
                "convex_contained_pct": round(
                    100.0 * rep_benchy["convex_contained_polys_prunable"] / rep_benchy["total_in_polys"],
                    2,
                ),
                "max_comp_polys": rep_benchy["max_comp_polys"],
                "singleton_audit": rep_benchy["singleton_audit"],
                "cc_scanbeam_proof": rep_benchy["cc_scanbeam_proof"],
            },
            "base": {
                "batches": rep_base["batches"],
                "calls": rep_base["total_calls"],
                "in_polys": rep_base["total_in_polys"],
                "in_holes": rep_base["total_in_holes"],
                "in_verts": rep_base["total_in_verts"],
                "out_polys": rep_base["total_out_polys"],
                "out_holes": rep_base["total_out_holes"],
                "out_verts": rep_base["total_out_verts"],
                "vertex_collapse_ratio": round(rep_base["vertex_collapse_ratio"], 5),
                "area_overlap_factor": round(rep_base["area_overlap_factor"], 3),
                "avg_nodes_values": rep_base["avg_nodes_values"],
                "simplify_active_calls": rep_base["simplify_active_calls"],
                "exact_duplicate_polys": rep_base["exact_duplicate_polys_in_calls"],
                "identical_layer_role_repeats": rep_base["identical_layer_role_input_repeats"],
                "convex_contained_polys": rep_base["convex_contained_polys_prunable"],
                "convex_contained_pct": round(
                    100.0 * rep_base["convex_contained_polys_prunable"] / rep_base["total_in_polys"],
                    2,
                ),
                "max_comp_polys": rep_base["max_comp_polys"],
                "singleton_audit": rep_base["singleton_audit"],
                "cc_scanbeam_proof": rep_base["cc_scanbeam_proof"],
            },
        },
        "prod_cells": prod_cells,
        "mechanisms": mechanism_route,
    }


def main() -> int:
    doc = compute_arithmetic()
    dest = REDUCED / "arithmetic.json"
    dest.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {dest}")
    for mname in [
        "M2_cc_bypass_head_tail",
        "M3_convex_containment_prune",
        "M4c_hier_morton_2lvl_k16",
        "M8_host_worker_hier_k4",
        "M9_host_worker_prune_hier_k4",
        "M9_host_worker_prune_hier_k8",
        "M9n_host_nested_prune_hier_k4",
    ]:
        info = doc["mechanisms"][mname]
        bco = info["cells"]["base-classic-on"]
        bao = info["cells"]["base-arachne-on"]
        bnc = info["cells"]["benchy-classic-on"]
        print(
            f"  {mname:30} clear4={str(info['clears_t137_4cell_floor']):5} "
            f"base-c-on: -{bco['projected_wall_save_ms']:6.1f}ms (-{bco['projected_cell_cut_pct']:5.2f}%, {bco['t137_floor_multiple']:4.1f}x) -> {bco['est_ratio_ord_after']:.3f}x (acc {bco['est_ratio_acc_after']:.3f}x, +t181 {bco['est_ratio_ord_plus_t181_full']:.3f}x) "
            f"base-a-on: -> {bao['est_ratio_ord_after']:.3f}x (acc {bao['est_ratio_acc_after']:.3f}x) "
            f"benchy-c-on: -{bnc['projected_wall_save_ms']:5.1f}ms (-{bnc['projected_cell_cut_pct']:4.2f}%, {bnc['t137_floor_multiple']:4.1f}x) "
            f"haus={bco['max_boundary_hausdorff_um']:.3f}um"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

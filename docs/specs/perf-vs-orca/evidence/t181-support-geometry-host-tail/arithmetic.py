#!/usr/bin/env python3
"""Checkout-only arithmetic calculator for Wayfinder Ticket 181.

Re-derives all qualification checks, t177 reconciliations, observation-only
probe breakdowns, 12-thread exact IR replay speedups, zero-tolerance adversarial
falsification counts, standing t137 4-cell planner gate multiples, and
ESTIMATE ONLY single-track & stacked disjoint-stage route projections strictly
from committed in-repo JSON files (Python 3 stdlib only, zero target/, zero STL).
"""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REDUCED = HERE / "reduced"
T175_REF = HERE.parent / "t175-tiled-collision-carve" / "reference.json"
T177_ARITH = HERE.parent / "t177-post-t175-attribution" / "reduced" / "arithmetic.json"
T178_ARITH = HERE.parent / "t178-planner-phase3-union" / "reduced" / "arithmetic.json"
T179_ARITH = HERE.parent / "t179-per-layer-guest-screen" / "reduced" / "arithmetic.json"
T180_ARITH = HERE.parent / "t180-shell-classification-residual" / "reduced" / "arithmetic.json"

FLOOR_PCT = 0.80

ALL_CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
    "base-classic-off",
    "base-classic-on",
    "base-arachne-off",
    "base-arachne-on",
]

SUPPORTS_ON_CELLS = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]

SUPPORTS_OFF_CELLS = [
    "benchy-classic-off",
    "benchy-arachne-off",
    "base-classic-off",
    "base-arachne-off",
]

PROJECTED_MODES = [
    "M1_ppo_reuse",
    "M2_bbox_cull",
    "M123_ppo_bbox_zeroclone",
    "M1235_ppo_bbox_zeroclone_par",
    "M1234_ppo_bbox_zeroclone_cert",
    "M12345_full_exact_bundle",
]


def r2(x: float) -> float:
    return round(x, 2)


def r3(x: float) -> float:
    return round(x, 3)


def r4(x: float) -> float:
    return round(x, 4)


def model_of(cell: str) -> str:
    return "benchy" if cell.startswith("benchy-") else "base"


def compute_arithmetic(
    *,
    t175_ref: dict | None = None,
    t177_arith: dict | None = None,
    t178_arith: dict | None = None,
    t179_arith: dict | None = None,
    t180_arith: dict | None = None,
    probe: dict | None = None,
    rep_benchy: dict | None = None,
    rep_base: dict | None = None,
) -> dict:
    if t175_ref is None:
        t175_ref = json.loads(T175_REF.read_text(encoding="utf-8"))
    if t177_arith is None:
        t177_arith = json.loads(T177_ARITH.read_text(encoding="utf-8"))
    if t178_arith is None:
        t178_arith = json.loads(T178_ARITH.read_text(encoding="utf-8"))
    if t179_arith is None:
        t179_arith = json.loads(T179_ARITH.read_text(encoding="utf-8"))
    if t180_arith is None:
        t180_arith = json.loads(T180_ARITH.read_text(encoding="utf-8"))
    if probe is None:
        probe = json.loads((REDUCED / "probe.json").read_text(encoding="utf-8"))
    if rep_benchy is None:
        rep_benchy = json.loads((REDUCED / "replay_benchy.json").read_text(encoding="utf-8"))
    if rep_base is None:
        rep_base = json.loads((REDUCED / "replay_base.json").read_text(encoding="utf-8"))

    # 1. Qualification, invariance, and PPO equivalence
    runs = probe["runs"]
    exit_zero_ok = sum(1 for r in runs if r["exit_code"] == 0)
    ref_ok = sum(
        1
        for r in runs
        if r["sha256"] == t175_ref["active_set"].get(r["cell"])
        and r["bytes"] == t175_ref["bytes"].get(r["cell"])
        and r["matches_reference"] is True
    )
    qual_runs = [r for r in runs if r["instrumented"]]
    qual_clean_ok = sum(
        1
        for r in qual_runs
        if r["exit_code"] == 0
        and r.get("qual") is not None
        and r["qual"]["degraded"] is False
        and r["qual"]["fatal_error_count"] == 0
        and r["qual"]["non_fatal_error_count"] == 0
        and r["qual"]["code_1200_count"] == 0
    )
    invariance_all_identical = all(
        v["identical_counters"] is True and len(v["diff_keys"]) == 0
        for v in probe["invariance"].values()
    )
    ppo_all_exact = all(
        v["total_planes"] > 0
        and v["found_in_ppo"] == v["total_planes"]
        and v["exact_poly_equal"] == v["total_planes"]
        and v["missing_from_ppo"] == 0
        for v in probe["ppo_verify"].values()
    )
    controls_inert = all(
        v["active_entries"] == 0
        and v["overlap_pairs_total"] == 0
        and v["qual_instrument_tail_ms"] == 0
        and v["ctrl_tail_ms"] < 1.0
        for v in probe["supports_off_controls"].values()
    )

    qualification = {
        "total_runs": len(runs),
        "exit_zero_runs": exit_zero_ok,
        "reference_identical_runs": ref_ok,
        "qual_runs_total": len(qual_runs),
        "qual_runs_clean": qual_clean_ok,
        "invariance_pairs_total": len(probe["invariance"]),
        "invariance_all_identical": invariance_all_identical,
        "ppo_verify": probe["ppo_verify"],
        "ppo_all_exact": ppo_all_exact,
        "supports_off_controls_inert": controls_inert,
    }

    # 2. t177 reconciliation + probe sub-bracket attribution on all 4 supports-on cells
    reconciliation = {}
    attribution = {}
    for cell in SUPPORTS_ON_CELLS:
        s177 = t177_arith["split"][cell]
        r177 = t177_arith["route"][cell]
        p = probe["supports_on_cells"][cell]
        b = p["brackets_median_ms"]
        cnt = p["counters"]
        med_tail = p["median_tail_ms"]

        t177_stage_ms = s177["prepass_stage_ms"]["PrePass::SupportGeometry"]
        t177_tail_ms = s177["support_geometry_post_planner_tail_ms"]
        t177_tail_share_cell_pct = r3(100.0 * s177["support_geometry_post_planner_tail_share_of_cell"])
        t177_tail_share_stage_pct = r2(100.0 * t177_tail_ms / t177_stage_ms)

        clones_and_drops_ms = r3(
            b["t1b_arc_clone_ms"]
            + b["t3a_plans_clone_ms"]
            + b["t3b_entries_flatten_clone_ms"]
            + b["t3j_try_agg_drop_ms"]
            + b["t3_drop_plans_ms"]
        )
        prefetch_ms = b["t3e_prefetch_total_ms"]
        overlaps_ms = b["t3f4_overlaps_any_ms"]
        overlaps_disj_ms = b["t3f4a_overlaps_strict_disjoint_ms"]
        overlaps_touch_ms = b["t3f4b_overlaps_touch_only_ms"]
        overlaps_inter_ms = b["t3f4c_overlaps_bbox_intersect_ms"]
        other_ms = r3(med_tail - prefetch_ms - overlaps_ms - clones_and_drops_ms)

        reconciliation[cell] = {
            "t177_slice_complete_ms": s177["slice_complete_ms"],
            "t177_sg_stage_ms": t177_stage_ms,
            "t177_post_planner_tail_ms": t177_tail_ms,
            "t177_tail_share_of_stage_pct": t177_tail_share_stage_pct,
            "t177_tail_share_of_cell_pct": t177_tail_share_cell_pct,
            "probe_median_wall_s": p["median_wall_s"],
            "probe_median_stage_ms": p["median_stage_ms"],
            "probe_median_tail_ms": med_tail,
            "probe_tail_share_of_stage_pct": p["tail_share_of_stage_pct"],
            "probe_tail_share_of_process_wall_pct": p["tail_share_of_process_wall_pct"],
            "qual_probe_tail_ms": p["reconciliation"]["qual_probe_tail_ms"],
            "qual_instrument_tail_ms": p["reconciliation"]["qual_instrument_tail_ms"],
            "qual_delta_ms": p["reconciliation"]["qual_delta_ms"],
            "max_top_level_residual_ms": p["reconciliation"]["max_top_level_residual_ms"],
            "max_t3_internal_residual_ms": p["reconciliation"]["max_t3_internal_residual_ms"],
            "est_post_t175_ratio_ord": r4(r177["est_post_t175_ratio_ord"]),
            "est_post_t175_ratio_acc": r4(r177["est_post_t175_ratio_acc"]),
            "req_cut_1x_ord_pct": r2(100.0 * r177["required_cut_1x_ord"]),
            "req_cut_05x_ord_pct": r2(100.0 * r177["required_cut_05x_ord"]),
        }

        attribution[cell] = {
            "median_tail_ms": med_tail,
            "prefetch_total_ms": prefetch_ms,
            "prefetch_share_pct": r2(100.0 * prefetch_ms / med_tail),
            "slice_mesh_ex_ms": b["t3e3_slice_mesh_ex_ns_ms"] if "t3e3_slice_mesh_ex_ns_ms" in b else b["t3e3_slice_mesh_ex_ms"],
            "object_geometry_ms": b["t3e1_object_geometry_ms"],
            "world_mesh_ms": b["t3e1a_world_mesh_ms"],
            "top_cross_section_ms": b["t3e1c_top_cross_section_ms"],
            "overlaps_any_ms": overlaps_ms,
            "overlaps_any_share_pct": r2(100.0 * overlaps_ms / med_tail),
            "overlaps_strict_disjoint_ms": overlaps_disj_ms,
            "overlaps_strict_disjoint_share_pct": r2(100.0 * overlaps_disj_ms / med_tail),
            "overlaps_touch_only_ms": overlaps_touch_ms,
            "overlaps_bbox_intersect_ms": overlaps_inter_ms,
            "overlaps_bbox_intersect_share_pct": r2(100.0 * overlaps_inter_ms / med_tail),
            "clones_and_drops_ms": clones_and_drops_ms,
            "clones_and_drops_share_pct": r2(100.0 * clones_and_drops_ms / med_tail),
            "other_ms": other_ms,
            "other_share_pct": r2(100.0 * other_ms / med_tail),
            "two_dominant_share_pct": r2(100.0 * (prefetch_ms + overlaps_ms) / med_tail),
            "three_dominant_share_pct": r2(100.0 * (prefetch_ms + overlaps_ms + clones_and_drops_ms) / med_tail),
            "counters": cnt,
            "disjoint_pair_fraction_pct": r2(100.0 * cnt["overlap_pairs_strict_disjoint"] / cnt["overlap_pairs_total"]),
            "bbox_intersect_pair_fraction_pct": r2(100.0 * cnt["overlap_pairs_bbox_intersect"] / cnt["overlap_pairs_total"]),
        }

    # 3. 12-thread replay summary & adversarial falsification summary
    replay_summary = {}
    for model_name, rep in (("benchy", rep_benchy), ("base", rep_base)):
        modes_by_name = {m["name"]: m for m in rep["modes"]}
        m0 = modes_by_name["M0_baseline"]
        probe_med = probe["supports_on_cells"][f"{model_name}-classic-on"]["median_tail_ms"]
        mode_rows = {}
        for name, m in modes_by_name.items():
            save_ms = r3(m0["median_wall_ms"] - m["median_wall_ms"])
            save_pct = r2(100.0 * (m0["median_wall_ms"] - m["median_wall_ms"]) / m0["median_wall_ms"])
            cpu_save_pct = r2(100.0 * (m0["median_cpu_ms"] - m["median_cpu_ms"]) / m0["median_cpu_ms"])
            mode_rows[name] = {
                "exact_bit_identical": m["exact_bit_identical"],
                "median_wall_ms": m["median_wall_ms"],
                "median_cpu_ms": m["median_cpu_ms"],
                "save_wall_ms": save_ms,
                "save_wall_pct": save_pct,
                "save_cpu_pct": cpu_save_pct,
                "speedup_wall_x": r2(m0["median_wall_ms"] / m["median_wall_ms"]),
                "counters": m["counters"],
            }
        adv = rep["adversarial"]
        replay_summary[model_name] = {
            "rayon_threads": rep["rayon_threads"],
            "rounds": rep["rounds"],
            "baseline_entries": rep["baseline_entries"],
            "baseline_diag_count": rep["baseline_diag_count"],
            "m0_replay_wall_ms": m0["median_wall_ms"],
            "live_probe_classic_on_tail_ms": probe_med,
            "m0_vs_live_probe_ratio": r4(m0["median_wall_ms"] / probe_med),
            "modes": mode_rows,
            "adversarial": {
                "real_pairs_checked": adv["real_pairs_checked"],
                "real_pairs_exact_match": adv["real_pairs_exact_match"],
                "real_pairs_bbox_disjoint": adv["real_pairs_bbox_disjoint"],
                "real_pairs_cert_disjoint": adv["real_pairs_cert_disjoint"],
                "real_pairs_fallback": adv["real_pairs_fallback"],
                "synthetic_cases_checked": adv["synthetic_cases_checked"],
                "synthetic_true_overlaps": adv["synthetic_true_overlaps"],
                "synthetic_true_disjoints": adv["synthetic_true_disjoints"],
                "false_negatives": adv["false_negatives"],
                "false_positives": adv["false_positives"],
            },
        }

    adversarial_totals = {
        "real_pairs_checked": sum(replay_summary[m]["adversarial"]["real_pairs_checked"] for m in ("benchy", "base")),
        "real_pairs_exact_match": sum(replay_summary[m]["adversarial"]["real_pairs_exact_match"] for m in ("benchy", "base")),
        "synthetic_cases_checked": sum(replay_summary[m]["adversarial"]["synthetic_cases_checked"] for m in ("benchy", "base")),
        "total_pairs_verified": sum(
            replay_summary[m]["adversarial"]["real_pairs_checked"]
            + replay_summary[m]["adversarial"]["synthetic_cases_checked"]
            for m in ("benchy", "base")
        ),
        "total_false_negatives": sum(replay_summary[m]["adversarial"]["false_negatives"] for m in ("benchy", "base")),
        "total_false_positives": sum(replay_summary[m]["adversarial"]["false_positives"] for m in ("benchy", "base")),
    }

    # 4. Standing t137 4-cell planner gate projections (ESTIMATE ONLY)
    gate_projections = {}
    for mode_name in PROJECTED_MODES:
        per_cell = {}
        fmults_t177 = []
        fmults_probe = []
        clears_t177 = 0
        clears_probe = 0
        for cell in SUPPORTS_ON_CELLS:
            m = model_of(cell)
            rec = reconciliation[cell]
            save_pct = replay_summary[m]["modes"][mode_name]["save_wall_pct"]
            cut_t177 = r3(rec["t177_tail_share_of_cell_pct"] * save_pct / 100.0)
            cut_probe = r3(rec["probe_tail_share_of_process_wall_pct"] * save_pct / 100.0)
            fm_t177 = r2(cut_t177 / FLOOR_PCT)
            fm_probe = r2(cut_probe / FLOOR_PCT)
            if cut_t177 >= FLOOR_PCT:
                clears_t177 += 1
            if cut_probe >= FLOOR_PCT:
                clears_probe += 1
            fmults_t177.append(fm_t177)
            fmults_probe.append(fm_probe)

            ord_pre = rec["est_post_t175_ratio_ord"]
            acc_pre = rec["est_post_t175_ratio_acc"]
            ord_after = r4(ord_pre * (1.0 - cut_t177 / 100.0))
            acc_after = r4(acc_pre * (1.0 - cut_t177 / 100.0))
            per_cell[cell] = {
                "tail_save_pct": save_pct,
                "t177_tail_share_pct": rec["t177_tail_share_of_cell_pct"],
                "projected_cell_cut_t177_scale_pct": cut_t177,
                "projected_cell_cut_probe_scale_pct": cut_probe,
                "floor_multiple_t177_scale": fm_t177,
                "floor_multiple_probe_scale": fm_probe,
                "clears_080_t177_scale": cut_t177 >= FLOOR_PCT,
                "clears_080_probe_scale": cut_probe >= FLOOR_PCT,
                "est_ratio_ord_after": ord_after,
                "est_ratio_acc_after": acc_after,
                "share_of_req_cut_1x_ord_pct": r2(100.0 * cut_t177 / rec["req_cut_1x_ord_pct"]),
            }
        gate_projections[mode_name] = {
            "exact_bit_identical_both": (
                replay_summary["benchy"]["modes"][mode_name]["exact_bit_identical"]
                and replay_summary["base"]["modes"][mode_name]["exact_bit_identical"]
            ),
            "clears_all_4_t177_scale": clears_t177 == 4,
            "clears_all_4_probe_scale": clears_probe == 4,
            "min_floor_multiple_t177_scale": min(fmults_t177),
            "max_floor_multiple_t177_scale": max(fmults_t177),
            "min_floor_multiple_probe_scale": min(fmults_probe),
            "max_floor_multiple_probe_scale": max(fmults_probe),
            "cells": per_cell,
        }

    # 5. Stacked disjoint-stage route context (ESTIMATE ONLY)
    # Composes t185 (M12345 host tail), t182 (M9n_k8 Phase-3 union),
    # t183 (TS-2 + IL-ALL per-layer), and t184 (EXACT_BUNDLE ShellClassification).
    t178_m9n = t178_arith["mechanisms"]["M9n_host_nested_prune_hier_k8"]["cells"]
    t179_rt = t179_arith["route_projections_estimate_only"]
    t180_ex = t180_arith["mechanism_8cell_projections_estimate_only"]["EXACT_BUNDLE"]["cells"]
    t185_m12345 = gate_projections["M12345_full_exact_bundle"]["cells"]

    stacked_route = {}
    for cell in ALL_CELLS:
        r177 = t177_arith["route"][cell]
        ord_pre = r4(r177["est_post_t175_ratio_ord"])
        acc_pre = r4(r177["est_post_t175_ratio_acc"])

        cut_t185 = t185_m12345[cell]["projected_cell_cut_t177_scale_pct"] if cell in t185_m12345 else 0.0
        cut_t182 = t178_m9n[cell]["projected_cell_cut_pct"] if cell in t178_m9n else 0.0
        cut_t183 = t179_rt[cell]["screen_measured_combined_cut_pct"]
        cut_t184 = t180_ex[cell]["projected_cell_cut_pct"]

        def apply_cuts(base_r: float, *cuts: float) -> float:
            return r4(base_r * (1.0 - sum(cuts) / 100.0))

        stacked_route[cell] = {
            "post_t175_ord": ord_pre,
            "post_t175_acc": acc_pre,
            "cuts_pct": {
                "t185_host_tail_m12345": cut_t185,
                "t182_phase3_m9n": cut_t182,
                "t183_per_layer_screen": cut_t183,
                "t184_shell_exact": cut_t184,
            },
            "after_t185_ord": apply_cuts(ord_pre, cut_t185),
            "after_t185_acc": apply_cuts(acc_pre, cut_t185),
            "after_t185_t182_ord": apply_cuts(ord_pre, cut_t185, cut_t182),
            "after_t185_t182_acc": apply_cuts(acc_pre, cut_t185, cut_t182),
            "after_t185_t182_t183_ord": apply_cuts(ord_pre, cut_t185, cut_t182, cut_t183),
            "after_t185_t182_t183_acc": apply_cuts(acc_pre, cut_t185, cut_t182, cut_t183),
            "after_all_four_ord": apply_cuts(ord_pre, cut_t185, cut_t182, cut_t183, cut_t184),
            "after_all_four_acc": apply_cuts(acc_pre, cut_t185, cut_t182, cut_t183, cut_t184),
        }

    return {
        "schema": 1,
        "note": (
            "ESTIMATE ONLY route chain, never a ratio of record. Stage & tail cell shares come from "
            "t177's post-t175 attribution and t181's 22-run observation-only probe (probe.patch cleanly "
            "reverted, 22/22 byte-identical to t175); candidate speedups and adversarial zero-tolerance "
            "proofs come from 12-thread standalone exact IR replay on real benchy (200 entries) and base "
            "(432 entries) snapshots. Disjoint-stage stacks compose additively as ESTIMATE ONLY."
        ),
        "qualification": qualification,
        "t177_reconciliation": reconciliation,
        "probe_attribution": attribution,
        "replay_summary": replay_summary,
        "adversarial_totals": adversarial_totals,
        "t137_4cell_gate_projections_estimate_only": gate_projections,
        "stacked_route_context_estimate_only": stacked_route,
    }


def main() -> int:
    doc = compute_arithmetic()
    out_path = REDUCED / "arithmetic.json"
    out_path.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {out_path} ({out_path.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

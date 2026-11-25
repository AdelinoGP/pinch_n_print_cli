#!/usr/bin/env python3
"""Checkout-only arithmetic calculator for Wayfinder Ticket 180.

Re-derives all t177 8-cell reconciliations, observation-only probe breakdowns,
12-thread replay speedups, standing all-8-cell 0.80 % floor multiples, and
ESTIMATE ONLY route projections strictly from committed in-repo JSON files
(Python 3 stdlib only, zero raw captures, zero heavy STL fixtures).
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

MECHANISM_CLASS = {
    "M0_baseline": "baseline",
    "E1_p2_par_merge_diff": "exact_bit_identical",
    "E2_ib_lazy_cluster_cache": "exact_bit_identical",
    "E3_ib_phaseA_windowed": "exact_bit_identical",
    "E4a_ext_fast_score": "exact_bit_identical",
    "E4ab_ext_score_rect_clip": "falsified_exactness",
    "E5_p2_bbox_filter_nbr": "exact_bit_identical",
    "EXACT_BUNDLE": "exact_bit_identical",
    "EXACT_PLUS_E4b": "falsified_exactness",
    "L1_sliver_prefilter": "falsified_exactness",
    "L2_p2_per_poly_neg": "sub_um_scanbeam_parity",
    "L3_p1_canon_open": "falsified_exactness",
    "EXACT_PLUS_P2_PERPOLY": "sub_um_scanbeam_parity",
    "EXACT_PLUS_P1_P2_CANON": "falsified_exactness",
    "FULL_STACK_WITH_E4b": "falsified_exactness",
}

PROJECTED_MECHANISMS = [
    "E1_p2_par_merge_diff",
    "E2_ib_lazy_cluster_cache",
    "E3_ib_phaseA_windowed",
    "E4a_ext_fast_score",
    "E5_p2_bbox_filter_nbr",
    "EXACT_BUNDLE",
    "L2_p2_per_poly_neg",
    "EXACT_PLUS_P2_PERPOLY",
    "EXACT_PLUS_P1_P2_CANON",
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
    if probe is None:
        probe = json.loads((REDUCED / "probe.json").read_text(encoding="utf-8"))
    if rep_benchy is None:
        rep_benchy = json.loads((REDUCED / "replay_benchy.json").read_text(encoding="utf-8"))
    if rep_base is None:
        rep_base = json.loads((REDUCED / "replay_base.json").read_text(encoding="utf-8"))

    # 1. Qualification vs t175 reference currency
    runs = probe["runs"]
    exit_zero_ok = sum(1 for r in runs if r["exit_code"] == 0)
    ref_sha_and_bytes_ok = sum(
        1
        for r in runs
        if r["gcode_sha256"] == t175_ref["active_set"].get(r["cell"])
        and r["gcode_bytes"] == t175_ref["bytes"].get(r["cell"])
        and r["reference_match"] is True
    )
    qual_runs = [r for r in runs if r["instrumented"]]
    qual_clean_ok = sum(
        1
        for r in qual_runs
        if r["exit_code"] == 0
        and r["instrument_events"] is not None
        and r["instrument_events"]["degraded"] is False
        and r["instrument_events"]["fatal_error_count"] == 0
        and r["instrument_events"]["non_fatal_error_count"] == 0
        and r["instrument_events"]["code_1200_count"] == 0
    )
    invariance_all_identical = all(
        v["identical_calls_and_verts"] is True and len(v["diff_sites"]) == 0
        for v in probe["invariance"].values()
    )
    qualification = {
        "total_runs": len(runs),
        "exit_zero_ok": exit_zero_ok,
        "ref_sha_and_bytes_ok": ref_sha_and_bytes_ok,
        "qual_runs": len(qual_runs),
        "qual_clean_ok": qual_clean_ok,
        "all_byte_identical_t175": (ref_sha_and_bytes_ok == len(runs) and exit_zero_ok == len(runs)),
        "invariance_pairs": len(probe["invariance"]),
        "invariance_all_identical": invariance_all_identical,
    }

    # 2. t177 8-cell ShellClassification reconciliation + op-kind call reconciliation
    t177_cells = {}
    for cell in ALL_CELLS:
        sh = t177_arith["ticket_inputs"]["for_t180_prepass_stages"][cell]["PrePass::ShellClassification"]
        rt = t177_arith["route"][cell]
        scopes = sh["profile_scopes"]
        t177_cells[cell] = {
            "instrumented_ms": sh["instrumented_ms"],
            "share_of_cell_pct": r3(100.0 * sh["share_of_cell"]),
            "share_of_prepass_pct": r3(100.0 * sh["share_of_prepass"]),
            "profile_thread_s": sh["profile_thread_s"],
            "profile_calls": sh["profile_calls"],
            "profile_thread_over_inst_wall": sh["profile_thread_over_inst_wall"],
            "profile_scopes": {
                "clip_polygons": {"calls": scopes["polygon_ops::clip_polygons"]["calls"], "s": scopes["polygon_ops::clip_polygons"]["s"]},
                "offset": {"calls": scopes["polygon_ops::offset"]["calls"], "s": scopes["polygon_ops::offset"]["s"]},
                "offset2_ex": {"calls": scopes["polygon_ops::offset2_ex"]["calls"], "s": scopes["polygon_ops::offset2_ex"]["s"]},
                "closing_ex": {"calls": scopes["polygon_ops::closing_ex"]["calls"], "s": scopes["polygon_ops::closing_ex"]["s"]},
            },
            "est_post_t175_ratio_ord": rt["est_post_t175_ratio_ord"],
            "est_post_t175_ratio_acc": rt["est_post_t175_ratio_acc"],
            "ratio_if_zero_ord": sh["ratio_if_zero_ord"],
            "ratio_if_zero_acc": sh["ratio_if_zero_acc"],
            "req_cut_1x_ord_pct": r3(100.0 * rt["required_cut_1x_ord"]),
            "req_cut_05x_ord_pct": r3(100.0 * rt["required_cut_05x_ord"]),
        }

    call_reconciliation = {}
    for cell in ["benchy-classic-off", "benchy-classic-on", "base-classic-off"]:
        mp = probe["cells"][cell]["median_pass"]
        ok = mp["op_kinds"]
        t7 = t177_cells[cell]
        sc = t7["profile_scopes"]
        diffs = {
            "clip": ok["clip"]["calls"] - sc["clip_polygons"]["calls"],
            "offset": ok["offset"]["calls"] - sc["offset"]["calls"],
            "offset2_ex": ok["offset2_ex"]["calls"] - sc["offset2_ex"]["calls"],
            "closing_ex": ok["closing_ex"]["calls"] - sc["closing_ex"]["calls"],
            "total": mp["clipper_total_calls"] - t7["profile_calls"],
        }
        call_reconciliation[cell] = {
            "probe_clipper_calls": mp["clipper_total_calls"],
            "t177_profile_calls": t7["profile_calls"],
            "diffs_by_op_kind": diffs,
            "exact_match": all(v == 0 for v in diffs.values()),
        }

    # 3. Probe attribution summary (brackets + waste/locality counters)
    probe_attribution = {}
    for cell in ["benchy-classic-off", "benchy-classic-on", "base-classic-off"]:
        cd = probe["cells"][cell]
        mp = cd["median_pass"]
        loc = mp["locality"]
        b_map = {b["name"]: b for b in mp["brackets"]}
        p1 = loc["pass1"]
        p2t = loc["pass2_top"]
        p2b = loc["pass2_bot"]
        p2e = loc["pass2e_int_solid"]
        iba = loc["ib_phase_a"]
        ibb = loc["ib_phase_b_precomp"]
        ibc = loc["ib_phase_b_commit"]

        p1_sliv = p1["top_diff_slivers"] + p1["bot_diff_slivers"]
        p1_tot = p1["top_diff_polys"] + p1["bot_diff_polys"]
        p2_sliv = p2t["inter_slivers"] + p2b["inter_slivers"]
        p2_tot = p2t["inter_polys_tot"] + p2b["inter_polys_tot"]

        probe_attribution[cell] = {
            "pass_stage_walls_ms": cd["pass_stage_walls_ms"],
            "stage_wall_ms": mp["stage_wall_ms"],
            "sum_brackets_ms": mp["sum_brackets_ms"],
            "bracket_gap_pct": r3(100.0 * abs(mp["bracket_gap_ms"]) / mp["stage_wall_ms"]),
            "serial_ms": mp["serial_ms"],
            "serial_pct": mp["serial_pct"],
            "parallel_ms": mp["parallel_ms"],
            "parallel_pct": mp["parallel_pct"],
            "total_thread_ms": mp["total_thread_ms"],
            "eff_par": mp["eff_par"],
            "clipper_total_calls": mp["clipper_total_calls"],
            "clipper_total_thread_ms": mp["clipper_total_thread_ms"],
            "cpu_unmarked_thread_ms": mp["op_kinds"]["cpu"]["thread_ms"],
            "key_brackets_ms": {
                "B1_pass1": b_map["B1_pass1"]["wall_ms"],
                "B2a_p2t_proj": b_map["B2a_p2t_proj"]["wall_ms"],
                "B2c_p2b_proj": b_map["B2c_p2b_proj"]["wall_ms"],
                "B2_serial_merge_diff": r3(
                    b_map["B2b_p2t_merge"]["wall_ms"]
                    + b_map["B2d_p2b_merge"]["wall_ms"]
                    + b_map["B2e_int_solid_diff"]["wall_ms"]
                ),
                "B4_ext_bridge": b_map["B4_ext_bridge"]["wall_ms"],
                "B5b_ib_phaseA": b_map["B5b_ib_phaseA"]["wall_ms"],
                "B5c_ib_phaseB_pre": b_map["B5c_ib_phaseB_pre"]["wall_ms"],
                "B5d_ib_phaseB_com": b_map["B5d_ib_phaseB_com"]["wall_ms"],
            },
            "stragglers": {
                "B2c_max_ms": b_map["B2c_p2b_proj"]["max_ms"],
                "B2c_max_idx": b_map["B2c_p2b_proj"]["max_idx"],
                "B5b_max_ms": b_map["B5b_ib_phaseA"]["max_ms"],
                "B5b_max_idx": b_map["B5b_ib_phaseA"]["max_idx"],
                "B5c_max_ms": b_map["B5c_ib_phaseB_pre"]["max_ms"],
                "B5c_max_idx": b_map["B5c_ib_phaseB_pre"]["max_idx"],
            },
            "waste_and_locality_pct": {
                "p1_sliver_pct": r2(100.0 * p1_sliv / max(1, p1_tot)),
                "p2_sliver_pct": r2(100.0 * p2_sliv / max(1, p2_tot)),
                "p2e_no_p2_contrib_pct": r2(100.0 * p2e["no_p2_contrib"] / max(1, p2e["total"])),
                "ib_a_lf_verts_in_box_pct": r2(100.0 * iba["lf_verts_in_box"] / max(1, iba["lf_verts_tot"])),
                "ib_a_ls_verts_in_box_pct": r2(100.0 * iba["ls_verts_in_box"] / max(1, iba["ls_verts_tot"])),
                "ib_b_none_layer_pct": r2(100.0 * ibb["none"] / max(1, ibb["total"])),
                "ib_b_wasted_none_thread_pct": r2(
                    100.0 * ibb["wasted_none_thread_ms"] / max(0.001, b_map["B5c_ib_phaseB_pre"]["thread_ms"])
                ),
                "ib_b_gather_layers_gt1": ibb["gather_layers"]["layers_gt1"],
                "ib_b_recomputes": ibc["recomputes"],
                "ib_b_recomp_share_of_stage_pct": r2(100.0 * ibc["recomp_wall_ms"] / mp["stage_wall_ms"]),
            },
        }

    # 4. Replay summary (benchy & base)
    replay_raw = {"benchy": rep_benchy, "base": rep_base}
    replay_by_model: dict[str, dict[str, dict]] = {}
    replay_summary = {}
    for model, rep in replay_raw.items():
        m_map = {m["name"]: m for m in rep["mechanisms"]}
        replay_by_model[model] = m_map
        m0 = m_map["M0_baseline"]["median_ms"]
        ex = m_map["EXACT_BUNDLE"]["median_ms"]
        ex_p2 = m_map["EXACT_PLUS_P2_PERPOLY"]["median_ms"]
        ex_p1p2 = m_map["EXACT_PLUS_P1_P2_CANON"]["median_ms"]
        probe_cell = "benchy-classic-off" if model == "benchy" else "base-classic-off"
        probe_ms = probe["cells"][probe_cell]["median_pass"]["stage_wall_ms"]

        rows = {}
        for m in rep["mechanisms"]:
            name = m["name"]
            save_ms = m0 - m["median_ms"]
            save_pct = 100.0 * save_ms / m0
            rows[name] = {
                "median_ms": m["median_ms"],
                "save_ms_vs_m0": r3(save_ms),
                "save_pct_vs_m0": r2(save_pct),
                "bit_identical": m["bit_identical"],
                "exactness": m["exactness"],
                "classification": MECHANISM_CLASS[name],
                "brackets_ms": m["brackets_ms"],
            }

        replay_summary[model] = {
            "layers": rep["layers"],
            "rounds": rep["rounds"],
            "threads": rep["threads"],
            "probe_median_stage_ms": probe_ms,
            "m0_replay_median_ms": m0,
            "m0_vs_probe_pct": r2(100.0 * (m0 - probe_ms) / probe_ms),
            "m0_bit_identical": m_map["M0_baseline"]["bit_identical"],
            "exact_bundle_bit_identical": m_map["EXACT_BUNDLE"]["bit_identical"],
            "exact_bundle_capture_of_exact_plus_p2_pct": r2(100.0 * (m0 - ex) / (m0 - ex_p2)),
            "marginal_l2_over_exact_ms": r3(ex - ex_p2),
            "marginal_l2_over_exact_pct_of_m0": r2(100.0 * (ex - ex_p2) / m0),
            "marginal_l3_over_exact_p2_ms": r3(ex_p2 - ex_p1p2),
            "marginal_l3_over_exact_p2_pct_of_m0": r2(100.0 * (ex_p2 - ex_p1p2) / m0),
            "mechanisms": rows,
        }

    # 5. 8-cell gate & ESTIMATE ONLY route projections per candidate mechanism
    def project_saving_across_8_cells(
        benchy_save_pct: float,
        base_save_pct: float,
        classification: str,
        bit_identical_both: bool,
    ) -> dict:
        per_cell = {}
        floor_multiples = []
        clears_count = 0
        for cell in ALL_CELLS:
            m = model_of(cell)
            save_pct = benchy_save_pct if m == "benchy" else base_save_pct
            t7 = t177_cells[cell]
            stage_share = t7["share_of_cell_pct"]
            cell_cut_pct = stage_share * save_pct / 100.0
            fmult = cell_cut_pct / FLOOR_PCT
            clears = cell_cut_pct >= FLOOR_PCT
            if clears:
                clears_count += 1
            floor_multiples.append(fmult)
            ord_pre = t7["est_post_t175_ratio_ord"]
            acc_pre = t7["est_post_t175_ratio_acc"]
            ord_after = ord_pre * (1.0 - cell_cut_pct / 100.0)
            acc_after = acc_pre * (1.0 - cell_cut_pct / 100.0)
            req1 = t7["req_cut_1x_ord_pct"]
            req05 = t7["req_cut_05x_ord_pct"]
            per_cell[cell] = {
                "stage_share_of_cell_pct": stage_share,
                "stage_saving_pct": r2(save_pct),
                "projected_stage_save_ms_on_t177": r2(t7["instrumented_ms"] * save_pct / 100.0),
                "projected_cell_cut_pct": r3(cell_cut_pct),
                "floor_multiple_080": r2(fmult),
                "clears_080_floor": clears,
                "est_ratio_ord_after": r4(ord_after),
                "est_ratio_acc_after": r4(acc_after),
                "crosses_1x_ord": ord_after < 1.0,
                "crosses_1x_acc": acc_after < 1.0,
                "share_of_req_cut_1x_ord_pct": r2(100.0 * cell_cut_pct / req1) if req1 > 0.0 else None,
                "share_of_req_cut_05x_ord_pct": r2(100.0 * cell_cut_pct / req05) if req05 > 0.0 else None,
            }
        return {
            "classification": classification,
            "bit_identical_both_models": bit_identical_both,
            "cells_clearing_080_floor": clears_count,
            "clears_all_8_cells": clears_count == len(ALL_CELLS),
            "min_floor_multiple_080": r2(min(floor_multiples)),
            "max_floor_multiple_080": r2(max(floor_multiples)),
            "cells": per_cell,
        }

    mechanism_projections = {}
    for name in PROJECTED_MECHANISMS:
        rb = replay_summary["benchy"]["mechanisms"][name]
        rba = replay_summary["base"]["mechanisms"][name]
        mechanism_projections[name] = project_saving_across_8_cells(
            benchy_save_pct=rb["save_pct_vs_m0"],
            base_save_pct=rba["save_pct_vs_m0"],
            classification=MECHANISM_CLASS[name],
            bit_identical_both=(rb["bit_identical"] and rba["bit_identical"]),
        )

    # Also project marginal L2 over EXACT_BUNDLE and marginal L3 over EXACT_PLUS_P2_PERPOLY
    mechanism_projections["MARGINAL_L2_OVER_EXACT"] = project_saving_across_8_cells(
        benchy_save_pct=replay_summary["benchy"]["marginal_l2_over_exact_pct_of_m0"],
        base_save_pct=replay_summary["base"]["marginal_l2_over_exact_pct_of_m0"],
        classification="sub_um_scanbeam_parity",
        bit_identical_both=False,
    )
    mechanism_projections["MARGINAL_L3_OVER_EXACT_P2"] = project_saving_across_8_cells(
        benchy_save_pct=replay_summary["benchy"]["marginal_l3_over_exact_p2_pct_of_m0"],
        base_save_pct=replay_summary["base"]["marginal_l3_over_exact_p2_pct_of_m0"],
        classification="falsified_exactness",
        bit_identical_both=False,
    )

    # 6. Stacked disjoint-stage route context (ESTIMATE ONLY: t184 Shell + t183 PerLayer + t182 Phase3 + t181 HostTail)
    stacked_route = {}
    ex_proj = mechanism_projections["EXACT_BUNDLE"]["cells"]
    t178_m9n = t178_arith["mechanisms"]["M9n_host_nested_prune_hier_k8"]["cells"]
    t179_rt = t179_arith["route_projections_estimate_only"]
    for cell in ALL_CELLS:
        t7 = t177_cells[cell]
        s177 = t177_arith["split"][cell]
        ord_pre = t7["est_post_t175_ratio_ord"]
        acc_pre = t7["est_post_t175_ratio_acc"]

        cut_t184 = ex_proj[cell]["projected_cell_cut_pct"]
        # Use t179's screen_measured_combined_cut_pct (independent per_layer phase!)
        cut_t183 = t179_rt[cell]["screen_measured_combined_cut_pct"]
        cut_t182 = t178_m9n[cell]["projected_cell_cut_pct"] if cell in t178_m9n else 0.0
        cut_t181_full = r3(100.0 * s177["support_geometry_post_planner_tail_share_of_cell"])

        def apply_cuts(base_r: float, *cuts: float) -> float:
            return r4(base_r * (1.0 - sum(cuts) / 100.0))

        stacked_route[cell] = {
            "post_t175_ord": ord_pre,
            "post_t175_acc": acc_pre,
            "cuts_pct": {
                "t184_shell_exact": cut_t184,
                "t183_per_layer_screen": cut_t183,
                "t182_phase3_m9n": cut_t182,
                "t181_host_tail_ceiling": cut_t181_full,
            },
            "after_t184_ord": apply_cuts(ord_pre, cut_t184),
            "after_t184_acc": apply_cuts(acc_pre, cut_t184),
            "after_t184_t183_ord": apply_cuts(ord_pre, cut_t184, cut_t183),
            "after_t184_t183_acc": apply_cuts(acc_pre, cut_t184, cut_t183),
            "after_t184_t183_t182_ord": apply_cuts(ord_pre, cut_t184, cut_t183, cut_t182),
            "after_t184_t183_t182_acc": apply_cuts(acc_pre, cut_t184, cut_t183, cut_t182),
            "after_t184_t183_t182_t181full_ord": apply_cuts(ord_pre, cut_t184, cut_t183, cut_t182, cut_t181_full),
            "after_t184_t183_t182_t181full_acc": apply_cuts(acc_pre, cut_t184, cut_t183, cut_t182, cut_t181_full),
        }

    return {
        "schema": 1,
        "note": (
            "ESTIMATE ONLY route chain, never a ratio of record. Stage wall shares come from t177's "
            "post-t175 attribution; sub-phase splits come from t180's observation-only probe (reverted, "
            "15/15 byte-identical to t175); candidate speedups come from 12-thread standalone IR replay "
            "on real captured benchy (240 layers) and base (496 layers) snapshots. Cross-ticket stacks "
            "compose disjoint pipeline stages additively as ESTIMATE ONLY."
        ),
        "qualification": qualification,
        "t177_reconciliation": {
            "cells": t177_cells,
            "call_reconciliation": call_reconciliation,
        },
        "probe_attribution": probe_attribution,
        "replay_summary": replay_summary,
        "mechanism_8cell_projections_estimate_only": mechanism_projections,
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

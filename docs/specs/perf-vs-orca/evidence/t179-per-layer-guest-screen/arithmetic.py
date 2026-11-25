#!/usr/bin/env python3
"""Checkout-only arithmetic calculator for Wayfinder Ticket 179.

Re-derives all t177 reconciliation figures, tree-support and infill-linker inner-kernel
attributions (fuel fold + scope wall + structural counters + benchy L2 breakdown),
candidate deterministic fuel ablations, unsandboxed attribution/wall screen summaries,
and ESTIMATE ONLY route projections from committed in-repo JSONs only.

Python 3 stdlib only; zero raw captures; zero heavy STL fixtures.
"""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REDUCED = HERE / "reduced"
T175_REF = HERE.parent / "t175-tiled-collision-carve" / "reference.json"
T177_ARITH = HERE.parent / "t177-post-t175-attribution" / "reduced" / "arithmetic.json"
T177_PROF = HERE.parent / "t177-post-t175-attribution" / "reduced" / "profile-detail-clean.json"
T178_ARITH = HERE.parent / "t178-planner-phase3-union" / "reduced" / "arithmetic.json"

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

BENCHY_CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
]


def _pct(num: float, den: float, ndigits: int = 3) -> float:
    return round(100.0 * num / den, ndigits) if den else 0.0


def _module_fold_ok(mod_entry: dict) -> bool:
    scope_self_sum = sum(sc["self_fuel"] for sc in mod_entry["scopes"])
    return (mod_entry["unmarked_self_fuel"] + scope_self_sum) == mod_entry["total_fuel"]


def _scope_map(scopes: list[dict]) -> dict[str, dict]:
    return {sc["scope"]: sc for sc in scopes}


def _round_wins_and_sep(rows: list[dict], cand_arm: str, metric: str) -> tuple[int, int, bool]:
    base_by_r = {r["round"]: r[metric] for r in rows if r["arm"] == "baseline"}
    cand_by_r = {r["round"]: r[metric] for r in rows if r["arm"] == cand_arm}
    rounds = sorted(set(base_by_r) & set(cand_by_r))
    wins = sum(1 for r in rounds if cand_by_r[r] < base_by_r[r])
    strict_sep = bool(rounds) and (max(cand_by_r.values()) < min(base_by_r.values()))
    return wins, len(rounds), strict_sep


def compute_arithmetic(
    t175_ref: dict | None = None,
    t177_arith: dict | None = None,
    t177_prof: dict | None = None,
    t178_arith: dict | None = None,
    probe: dict | None = None,
    screen: dict | None = None,
) -> dict:
    if t175_ref is None:
        t175_ref = json.loads(T175_REF.read_text(encoding="utf-8"))
    if t177_arith is None:
        t177_arith = json.loads(T177_ARITH.read_text(encoding="utf-8"))
    if t177_prof is None:
        t177_prof = json.loads(T177_PROF.read_text(encoding="utf-8"))
    if t178_arith is None:
        t178_arith = json.loads(T178_ARITH.read_text(encoding="utf-8"))
    if probe is None:
        probe = json.loads((REDUCED / "probe_attribution.json").read_text(encoding="utf-8"))
    if screen is None:
        screen = json.loads((REDUCED / "screen_runs.json").read_text(encoding="utf-8"))

    # 1. Qualification & invariant checks
    probe_cells = probe["probe_cells"]
    cand_ab = probe["candidate_fuel_ablations"]
    instr = screen["instrumented"]
    wall = screen["uninstrumented_wall"]

    probe_ref_ok = 0
    probe_clean_ok = 0
    fuel_fold_ok_count = 0
    fuel_fold_total = 0
    for cell in ALL_CELLS:
        pc = probe_cells[cell]
        if pc["reference_sha256"] == t175_ref["active_set"][cell] and pc["reference_match"]:
            probe_ref_ok += 1
        sm = pc["slice_meta"]
        if (
            sm["degraded"] is False
            and sm["fatal_error_count"] == 0
            and sm["non_fatal_error_count"] == 0
            and pc["code_1200_count"] == 0
        ):
            probe_clean_ok += 1
        for mid in ("com.core.tree-support", "com.core.infill-linker"):
            fuel_fold_total += 1
            if _module_fold_ok(pc["modules"][mid]):
                fuel_fold_ok_count += 1

    cand_fuel_runs = 0
    cand_fuel_ref_ok = 0
    cand_fuel_clean_ok = 0
    for cname, cmap in cand_ab.items():
        for cell, rec in cmap.items():
            cand_fuel_runs += 1
            if rec["reference_sha256"] == t175_ref["active_set"][cell] and rec["reference_match"]:
                cand_fuel_ref_ok += 1
            if (
                rec["degraded"] is False
                and rec["fatal_error_count"] == 0
                and rec["non_fatal_error_count"] == 0
                and rec["code_1200_count"] == 0
            ):
                cand_fuel_clean_ok += 1

    instr_runs = 0
    instr_ref_ok = 0
    instr_clean_ok = 0
    for cell, arms in instr.items():
        for arm, rec in arms.items():
            instr_runs += 1
            if (
                rec["exit_code"] == 0
                and rec["gcode_sha256"] == t175_ref["active_set"][cell]
                and rec["gcode_bytes"] == t175_ref["bytes"][cell]
                and rec["reference_match"]
            ):
                instr_ref_ok += 1
            if (
                rec["degraded"] is False
                and rec["fatal_error_count"] == 0
                and rec["non_fatal_error_count"] == 0
                and rec["code_1200_count"] == 0
            ):
                instr_clean_ok += 1

    wall_runs = 0
    wall_ref_ok = 0
    for cell, cdoc in wall.items():
        for r in cdoc["rows"]:
            wall_runs += 1
            if (
                r["exit_code"] == 0
                and r["gcode_sha256"] == t175_ref["active_set"][cell]
                and r["gcode_bytes"] == t175_ref["bytes"][cell]
                and r["reference_match"]
            ):
                wall_ref_ok += 1

    qualification = {
        "probe_cells_count": len(ALL_CELLS),
        "probe_ref_ok": probe_ref_ok,
        "probe_clean_ok": probe_clean_ok,
        "fuel_fold_exact_ok": fuel_fold_ok_count,
        "fuel_fold_total": fuel_fold_total,
        "cand_fuel_runs": cand_fuel_runs,
        "cand_fuel_ref_ok": cand_fuel_ref_ok,
        "cand_fuel_clean_ok": cand_fuel_clean_ok,
        "screen_instr_runs": instr_runs,
        "screen_instr_ref_ok": instr_ref_ok,
        "screen_instr_clean_ok": instr_clean_ok,
        "screen_wall_runs": wall_runs,
        "screen_wall_ref_ok": wall_ref_ok,
        "total_verified_runs": len(ALL_CELLS) + cand_fuel_runs + instr_runs + wall_runs,
        "all_byte_identical_t175": (
            probe_ref_ok == len(ALL_CELLS)
            and cand_fuel_ref_ok == cand_fuel_runs
            and instr_ref_ok == instr_runs
            and wall_ref_ok == wall_runs
        ),
    }

    # 2. Reconciliation against t177 post-t175 shares & clean fuel
    t177_reconcile = {}
    for cell in ALL_CELLS:
        sp = t177_arith["split"][cell]
        pl = sp["per_layer_detail"]
        rt = t177_arith["route"][cell]
        pf = t177_prof["cells"][cell]["fuel"]
        tot_fuel = pf["total"]
        ts_fuel = pf["modules"]["com.core.tree-support"]["amount"]
        il_fuel = pf["modules"]["com.core.infill-linker"]["amount"]
        ts_mod = pl["modules"]["com.core.tree-support"]
        il_mod = pl["modules"]["com.core.infill-linker"]
        st = pl["straggler_tail"]

        t177_reconcile[cell] = {
            "t177_slice_complete_ms": sp["slice_complete_ms"],
            "t177_per_layer_ms": sp["per_layer_ms"],
            "t177_per_layer_share_pct": round(100.0 * sp["phase_shares"]["per_layer"], 3),
            "t177_straggler_tail_ms": st["single_layer_straggler_tail_ms"],
            "t177_straggler_last_layer": st["last_layer_index"],
            "t177_straggler_share_of_cell_pct": round(100.0 * st["share_of_cell"], 3),
            "t177_total_guest_fuel": tot_fuel,
            "tree_support": {
                "t177_thread_ms": ts_mod["thread_sum_ms"],
                "t177_share_of_per_layer_thread_pct": round(100.0 * ts_mod["share_of_per_layer_thread"], 3),
                "t177_implied_wall_share_pct": round(100.0 * ts_mod["implied_wall_share_of_cell"], 3),
                "t177_clean_fuel": ts_fuel,
                "t177_fuel_share_of_all_guest_pct": _pct(ts_fuel, tot_fuel, 3),
            },
            "infill_linker": {
                "t177_thread_ms": il_mod["thread_sum_ms"],
                "t177_share_of_per_layer_thread_pct": round(100.0 * il_mod["share_of_per_layer_thread"], 3),
                "t177_implied_wall_share_pct": round(100.0 * il_mod["implied_wall_share_of_cell"], 3),
                "t177_clean_fuel": il_fuel,
                "t177_fuel_share_of_all_guest_pct": _pct(il_fuel, tot_fuel, 3),
            },
            "combined_ts_plus_il_implied_wall_share_pct": round(
                100.0 * (ts_mod["implied_wall_share_of_cell"] + il_mod["implied_wall_share_of_cell"]),
                3,
            ),
            "combined_ts_plus_il_fuel_share_pct": _pct(ts_fuel + il_fuel, tot_fuel, 3),
            "est_post_t175_ratio_ord": rt["est_post_t175_ratio_ord"],
            "est_post_t175_ratio_acc": rt["est_post_t175_ratio_acc"],
            "req_cut_1x_ord_pct": round(100.0 * rt["required_cut_1x_ord"], 3),
            "req_cut_1x_acc_pct": round(100.0 * rt["required_cut_1x_acc"], 3),
            "req_cut_05x_ord_pct": round(100.0 * rt["required_cut_05x_ord"], 3),
        }

    # 3. tree-support inner-kernel attribution (4 supports-on cells)
    ts_attr = {}
    for cell in SUPPORTS_ON_CELLS:
        pc = probe_cells[cell]
        mod = pc["modules"]["com.core.tree-support"]
        clean_f = pc["clean_baseline_fuel"]["com.core.tree-support"]
        probed_f = mod["total_fuel"]
        smap = _scope_map(mod["scopes"])
        total_scope_wall_ns = sum(sc["self_wall_ns"] for sc in mod["scopes"])

        # Nested scopes: regularize & role_union wrap polygon_ops::clip_polygons/offset.
        # Compute clean exclusive breakdown for both fuel and scope-wall.
        paint_sc = smap["t179::ts::paint_policy"]
        skel_sc = smap["t179::ts::skeleton"]
        role_u_sc = smap["t179::ts::role_union"]
        reg_sc = smap["t179::ts::regularize"]
        scan_sc = smap["t179::ts::scan_fill"]
        walls_sc = smap["t179::ts::render_walls"]
        fill_off_sc = smap["t179::ts::render_fill_offset"]
        emit_sc = smap["t179::ts::output_emit"]

        ts1_fuel = cand_ab["cand1_ts1_il1"][cell]["modules"]["com.core.tree-support"]["total_fuel"]
        ts2_rec = cand_ab["cand_ts2_ilall"].get(cell)
        ts2_fuel = ts2_rec["modules"]["com.core.tree-support"]["total_fuel"] if ts2_rec else None

        cnt = pc["tree_support_counters"]
        skel_pts = cnt["skel_pts"]
        skel_needed = cnt["skel_needed_tests"]
        skel_le_best = skel_pts - skel_needed

        ts_attr[cell] = {
            "clean_fuel": clean_f,
            "probed_fuel": probed_f,
            "probe_fuel_overhead_pct": _pct(probed_f - clean_f, clean_f, 4),
            "total_scope_wall_ms_profile_attr_only": round(total_scope_wall_ns / 1e6, 2),
            "kernels": {
                "paint_policy_for": {
                    "calls": paint_sc["calls"],
                    "fuel": paint_sc["self_fuel"],
                    "fuel_pct": _pct(paint_sc["self_fuel"], probed_f, 3),
                    "scope_wall_ms": round(paint_sc["self_wall_ns"] / 1e6, 2),
                    "scope_wall_pct": _pct(paint_sc["self_wall_ns"], total_scope_wall_ns, 2),
                },
                "skeleton_wall_count": {
                    "calls": skel_sc["calls"],
                    "fuel": skel_sc["self_fuel"],
                    "fuel_pct": _pct(skel_sc["self_fuel"], probed_f, 3),
                    "scope_wall_ms": round(skel_sc["self_wall_ns"] / 1e6, 2),
                    "scope_wall_pct": _pct(skel_sc["self_wall_ns"], total_scope_wall_ns, 2),
                },
                "role_union_inclusive": {
                    "calls": role_u_sc["calls"],
                    "fuel": role_u_sc["total_fuel"],
                    "fuel_pct": _pct(role_u_sc["total_fuel"], probed_f, 3),
                    "scope_wall_ms": round(role_u_sc["total_wall_ns"] / 1e6, 2),
                    "scope_wall_pct": _pct(role_u_sc["total_wall_ns"], total_scope_wall_ns, 2),
                },
                "regularize_inclusive": {
                    "calls": reg_sc["calls"],
                    "fuel": reg_sc["total_fuel"],
                    "fuel_pct": _pct(reg_sc["total_fuel"], probed_f, 3),
                    "scope_wall_ms": round(reg_sc["total_wall_ns"] / 1e6, 2),
                    "scope_wall_pct": _pct(reg_sc["total_wall_ns"], total_scope_wall_ns, 2),
                },
                "render_walls_host_offset": {
                    "calls": walls_sc["calls"],
                    "fuel": walls_sc["self_fuel"],
                    "fuel_pct": _pct(walls_sc["self_fuel"], probed_f, 3),
                    "scope_wall_ms": round(walls_sc["self_wall_ns"] / 1e6, 2),
                    "scope_wall_pct": _pct(walls_sc["self_wall_ns"], total_scope_wall_ns, 2),
                },
                "render_fill_offset_host": {
                    "calls": fill_off_sc["calls"],
                    "fuel": fill_off_sc["self_fuel"],
                    "fuel_pct": _pct(fill_off_sc["self_fuel"], probed_f, 3),
                    "scope_wall_ms": round(fill_off_sc["self_wall_ns"] / 1e6, 2),
                    "scope_wall_pct": _pct(fill_off_sc["self_wall_ns"], total_scope_wall_ns, 2),
                },
                "scan_fill_region": {
                    "calls": scan_sc["calls"],
                    "fuel": scan_sc["self_fuel"],
                    "fuel_pct": _pct(scan_sc["self_fuel"], probed_f, 3),
                    "scope_wall_ms": round(scan_sc["self_wall_ns"] / 1e6, 2),
                    "scope_wall_pct": _pct(scan_sc["self_wall_ns"], total_scope_wall_ns, 2),
                },
                "output_emit": {
                    "calls": emit_sc["calls"],
                    "fuel": emit_sc["self_fuel"],
                    "fuel_pct": _pct(emit_sc["self_fuel"], probed_f, 3),
                    "scope_wall_ms": round(emit_sc["self_wall_ns"] / 1e6, 2),
                    "scope_wall_pct": _pct(emit_sc["self_wall_ns"], total_scope_wall_ns, 2),
                },
                "unmarked_self": {
                    "fuel": mod["unmarked_self_fuel"],
                    "fuel_pct": _pct(mod["unmarked_self_fuel"], probed_f, 3),
                },
            },
            "counters": {
                **cnt,
                "skel_le_best": skel_le_best,
                "skel_le_best_pct": _pct(skel_le_best, skel_pts, 2),
                "regize_none_pct": _pct(cnt["regize_none"], cnt["regize_calls"], 2),
            },
            "candidates_fuel": {
                "TS1_paint_and_skeleton": {
                    "cand_fuel": ts1_fuel,
                    "fuel_cut_pct": _pct(clean_f - ts1_fuel, clean_f, 3),
                    "fuel_speedup_x": round(clean_f / ts1_fuel, 2),
                },
                "TS2_ts1_plus_role_union_skip": (
                    {
                        "cand_fuel": ts2_fuel,
                        "fuel_cut_pct": _pct(clean_f - ts2_fuel, clean_f, 3),
                        "fuel_speedup_x": round(clean_f / ts2_fuel, 2),
                    }
                    if ts2_fuel is not None
                    else None
                ),
            },
        }

    # Verify tree-support classic == arachne counter invariance
    ts_invariance = {
        "benchy_counters_equal": (
            probe_cells["benchy-classic-on"]["tree_support_counters"]
            == probe_cells["benchy-arachne-on"]["tree_support_counters"]
        ),
        "base_counters_equal": (
            probe_cells["base-classic-on"]["tree_support_counters"]
            == probe_cells["base-arachne-on"]["tree_support_counters"]
        ),
    }

    # 4. infill-linker inner-kernel attribution (all 8 cells + benchy L2)
    il_attr = {}
    il_l2_benchy = {}
    IL_SCOPE_KEYS = [
        ("clip_batch_repair", "t179::il::clip_batch_repair"),
        ("clip_batch_attr", "t179::il::clip_batch_attr"),
        ("clip_batch_exec", "t179::il::clip_batch_exec"),
        ("post_clip_meta", "t179::il::post_clip_meta"),
        ("connect_bpos_proj", "t179::il::connect_bpos_proj"),
        ("connect_pair_scan", "t179::il::connect_pair_scan"),
        ("prep_clip_new", "t179::il::prep_clip_new"),
        ("connect_merge", "t179::il::connect_merge"),
        ("connect_bpos_hole", "t179::il::connect_bpos_hole"),
        ("setup", "t179::il::setup"),
        ("offset_boundary", "t179::il::offset_boundary"),
        ("graph_new", "t179::il::graph_new"),
        ("chain_order", "t179::il::chain_order"),
        ("clip_batch_fallback", "t179::il::clip_batch_fallback"),
        ("clip_polygons_sub", "polygon_ops::clip_polygons"),
        ("clip_single_subj", "t179::il::clip_single_subj"),
        ("output_emit", "t179::il::output_emit"),
    ]
    for cell in ALL_CELLS:
        pc = probe_cells[cell]
        mod = pc["modules"]["com.core.infill-linker"]
        clean_f = pc["clean_baseline_fuel"]["com.core.infill-linker"]
        probed_f = mod["total_fuel"]
        smap = _scope_map(mod["scopes"])
        total_scope_wall_ns = sum(sc["self_wall_ns"] for sc in mod["scopes"])

        kernels = {}
        for short, sname in IL_SCOPE_KEYS:
            sc = smap.get(sname)
            if sc is None:
                kernels[short] = {
                    "calls": 0,
                    "fuel": 0,
                    "fuel_pct": 0.0,
                    "scope_wall_ms": 0.0,
                    "scope_wall_pct": 0.0,
                }
            else:
                kernels[short] = {
                    "calls": sc["calls"],
                    "fuel": sc["self_fuel"],
                    "fuel_pct": _pct(sc["self_fuel"], probed_f, 3),
                    "scope_wall_ms": round(sc["self_wall_ns"] / 1e6, 2),
                    "scope_wall_pct": _pct(sc["self_wall_ns"], total_scope_wall_ns, 2),
                }
        kernels["unmarked_self"] = {
            "fuel": mod["unmarked_self_fuel"],
            "fuel_pct": _pct(mod["unmarked_self_fuel"], probed_f, 3),
        }

        il1_fuel = cand_ab["cand1_ts1_il1"][cell]["modules"]["com.core.infill-linker"]["total_fuel"]
        ilall_fuel = cand_ab["cand2_ts1_ilall"][cell]["modules"]["com.core.infill-linker"]["total_fuel"]

        il_attr[cell] = {
            "clean_fuel": clean_f,
            "probed_fuel": probed_f,
            "probe_fuel_overhead_pct": _pct(probed_f - clean_f, clean_f, 4),
            "total_scope_wall_ms_profile_attr_only": round(total_scope_wall_ns / 1e6, 2),
            "attr_plus_repair_fuel_pct": round(
                kernels["clip_batch_repair"]["fuel_pct"]
                + kernels["clip_batch_attr"]["fuel_pct"]
                + kernels["clip_batch_fallback"]["fuel_pct"],
                3,
            ),
            "connect_total_fuel_pct": round(
                kernels["connect_bpos_proj"]["fuel_pct"]
                + kernels["connect_pair_scan"]["fuel_pct"]
                + kernels["connect_merge"]["fuel_pct"]
                + kernels["connect_bpos_hole"]["fuel_pct"]
                + kernels["graph_new"]["fuel_pct"],
                3,
            ),
            "kernels": kernels,
            "counters": pc["infill_linker_counters"],
            "candidates_fuel": {
                "IL1_clipper64_open_tag": {
                    "cand_fuel": il1_fuel,
                    "fuel_cut_pct": _pct(clean_f - il1_fuel, clean_f, 3),
                    "fuel_speedup_x": round(clean_f / il1_fuel, 2),
                },
                "IL_ALL_il1_il2_il3": {
                    "cand_fuel": ilall_fuel,
                    "fuel_cut_pct": _pct(clean_f - ilall_fuel, clean_f, 3),
                    "fuel_speedup_x": round(clean_f / ilall_fuel, 2),
                },
            },
        }

        if cell in BENCHY_CELLS:
            l2s = pc["infill_linker_l2_scopes"]
            l2_smap = _scope_map(l2s["scopes"])
            l2_f = l2s["call_fuel"]
            l2_wall_ns = sum(sc["self_wall_ns"] for sc in l2s["scopes"])
            il_l2_benchy[cell] = {
                "l2_call_fuel": l2_f,
                "l2_share_of_slice_il_fuel_pct": _pct(l2_f, probed_f, 3),
                "l2_elapsed_ms_probed": l2s["elapsed_ms_probed"],
                "l2_counters": pc["infill_linker_l2_counters"],
                "l2_top_kernels": {
                    short: {
                        "calls": l2_smap[sname]["calls"],
                        "fuel": l2_smap[sname]["self_fuel"],
                        "fuel_pct": _pct(l2_smap[sname]["self_fuel"], l2_f, 3),
                        "scope_wall_ms": round(l2_smap[sname]["self_wall_ns"] / 1e6, 2),
                        "scope_wall_pct": _pct(l2_smap[sname]["self_wall_ns"], l2_wall_ns, 2),
                    }
                    for short, sname in [
                        ("clip_batch_repair", "t179::il::clip_batch_repair"),
                        ("connect_bpos_proj", "t179::il::connect_bpos_proj"),
                        ("clip_batch_exec", "t179::il::clip_batch_exec"),
                        ("prep_clip_new", "t179::il::prep_clip_new"),
                        ("connect_merge", "t179::il::connect_merge"),
                        ("connect_pair_scan", "t179::il::connect_pair_scan"),
                        ("clip_batch_attr", "t179::il::clip_batch_attr"),
                        ("offset_boundary", "t179::il::offset_boundary"),
                    ]
                    if sname in l2_smap
                },
            }

    # 5. Unsandboxed screen summaries (Part 1 instrumented + Part 2 wall/CPU)
    screen_summary = {}
    combined_8cell = {}
    for cell in ALL_CELLS:
        ib = instr[cell]["baseline"]
        wdoc = wall[cell]
        wb = wdoc["summary"]["baseline"]
        arms_out = {}
        for arm in wdoc["summary"]:
            if arm == "baseline":
                continue
            ia = instr[cell][arm]
            wa = wdoc["summary"][arm]
            w_wins, n_rounds, w_sep = _round_wins_and_sep(wdoc["rows"], arm, "wall_seconds")
            c_wins, _, c_sep = _round_wins_and_sep(wdoc["rows"], arm, "cpu_seconds")
            pl_delta_ms = ib["per_layer_ms"] - ia["per_layer_ms"]
            arms_out[arm] = {
                "instr_per_layer_base_ms": ib["per_layer_ms"],
                "instr_per_layer_cand_ms": ia["per_layer_ms"],
                "instr_per_layer_saved_ms": pl_delta_ms,
                "instr_per_layer_saved_pct": _pct(pl_delta_ms, ib["per_layer_ms"], 2),
                "instr_ts_thread_base_ms": ib["tree_support_thread_ms"],
                "instr_ts_thread_cand_ms": ia["tree_support_thread_ms"],
                "instr_ts_thread_saved_pct": _pct(
                    ib["tree_support_thread_ms"] - ia["tree_support_thread_ms"],
                    ib["tree_support_thread_ms"],
                    2,
                ),
                "instr_il_thread_base_ms": ib["infill_linker_thread_ms"],
                "instr_il_thread_cand_ms": ia["infill_linker_thread_ms"],
                "instr_il_thread_saved_pct": _pct(
                    ib["infill_linker_thread_ms"] - ia["infill_linker_thread_ms"],
                    ib["infill_linker_thread_ms"],
                    2,
                ),
                "instr_l2_il_base_ms": ib["l2_infill_linker_ms"],
                "instr_l2_il_cand_ms": ia["l2_infill_linker_ms"],
                "instr_straggler_tail_base_ms": ib["straggler_tail_ms"],
                "instr_straggler_tail_cand_ms": ia["straggler_tail_ms"],
                "instr_straggler_last_layer_cand": ia["last_layer_index"],
                "wall_base_median_s": wb["median_wall_seconds"],
                "wall_cand_median_s": wa["median_wall_seconds"],
                "wall_saved_ms": wa["wall_saved_ms_vs_baseline"],
                "wall_saved_pct": wa["wall_saved_pct_vs_baseline"],
                "wall_floor_multiple_080": round(wa["wall_saved_pct_vs_baseline"] / FLOOR_PCT, 2),
                "wall_round_wins": f"{w_wins}/{n_rounds}",
                "wall_strict_separation": w_sep,
                "cpu_base_median_s": wb["median_cpu_seconds"],
                "cpu_cand_median_s": wa["median_cpu_seconds"],
                "cpu_saved_pct": wa["cpu_saved_pct_vs_baseline"],
                "cpu_round_wins": f"{c_wins}/{n_rounds}",
                "cpu_strict_separation": c_sep,
                "cpu_over_wall_base": wb["median_cpu_over_wall"],
                "cpu_over_wall_cand": wa["median_cpu_over_wall"],
            }
        screen_summary[cell] = arms_out
        eff_arm = "arm_both" if cell in SUPPORTS_ON_CELLS else "arm_il"
        combined_8cell[cell] = {
            "effective_screen_arm": eff_arm,
            **arms_out[eff_arm],
        }

    # Check whether combined TS-2 + IL-ALL clears 0.80% floor & strict separation in all 8 cells
    combined_gate_summary = {
        "cells_clearing_080_wall_floor": sum(
            1 for c in ALL_CELLS if combined_8cell[c]["wall_saved_pct"] >= FLOOR_PCT
        ),
        "cells_strict_wall_separation": sum(
            1 for c in ALL_CELLS if combined_8cell[c]["wall_strict_separation"]
        ),
        "cells_strict_cpu_separation": sum(
            1 for c in ALL_CELLS if combined_8cell[c]["cpu_strict_separation"]
        ),
        "min_wall_saved_pct": min(combined_8cell[c]["wall_saved_pct"] for c in ALL_CELLS),
        "max_wall_saved_pct": max(combined_8cell[c]["wall_saved_pct"] for c in ALL_CELLS),
        "min_floor_multiple_080": min(combined_8cell[c]["wall_floor_multiple_080"] for c in ALL_CELLS),
        "max_floor_multiple_080": max(combined_8cell[c]["wall_floor_multiple_080"] for c in ALL_CELLS),
        "min_cpu_saved_pct": min(combined_8cell[c]["cpu_saved_pct"] for c in ALL_CELLS),
        "max_cpu_saved_pct": max(combined_8cell[c]["cpu_saved_pct"] for c in ALL_CELLS),
    }

    # 6. ESTIMATE ONLY route projections (t174 x t175 x candidate, plus stack with t182/t181)
    m9n_cells = t178_arith["mechanisms"]["M9n_host_nested_prune_hier_k8"]["cells"]
    route_est = {}
    for cell in ALL_CELLS:
        rec = t177_reconcile[cell]
        comb = combined_8cell[cell]
        ord_pre = rec["est_post_t175_ratio_ord"]
        acc_pre = rec["est_post_t175_ratio_acc"]

        # Model A (t177-implied proportional thread ceiling):
        # ts_implied * ts_thread_cut_frac + il_implied * il_thread_cut_frac
        ts_cut_frac = (comb["instr_ts_thread_saved_pct"] / 100.0) if cell in SUPPORTS_ON_CELLS else 0.0
        il_cut_frac = comb["instr_il_thread_saved_pct"] / 100.0
        t177_implied_cut_pct = round(
            rec["tree_support"]["t177_implied_wall_share_pct"] * ts_cut_frac
            + rec["infill_linker"]["t177_implied_wall_share_pct"] * il_cut_frac,
            3,
        )

        # Model B (screen-measured 3-round uninstrumented wall cut %)
        screen_cut_pct = comb["wall_saved_pct"]

        ord_after_t177_model = round(ord_pre * (1.0 - t177_implied_cut_pct / 100.0), 4)
        acc_after_t177_model = round(acc_pre * (1.0 - t177_implied_cut_pct / 100.0), 4)
        ord_after_screen = round(ord_pre * (1.0 - screen_cut_pct / 100.0), 4)
        acc_after_screen = round(acc_pre * (1.0 - screen_cut_pct / 100.0), 4)

        entry = {
            "est_post_t175_ratio_ord": ord_pre,
            "est_post_t175_ratio_acc": acc_pre,
            "req_cut_1x_ord_pct": rec["req_cut_1x_ord_pct"],
            "req_cut_05x_ord_pct": rec["req_cut_05x_ord_pct"],
            "t177_implied_combined_cut_pct": t177_implied_cut_pct,
            "screen_measured_combined_cut_pct": screen_cut_pct,
            "est_ratio_ord_after_t177_model": ord_after_t177_model,
            "est_ratio_acc_after_t177_model": acc_after_t177_model,
            "est_ratio_ord_after_screen": ord_after_screen,
            "est_ratio_acc_after_screen": acc_after_screen,
            "share_of_req_cut_1x_ord_screen_pct": (
                round(100.0 * screen_cut_pct / rec["req_cut_1x_ord_pct"], 2)
                if rec["req_cut_1x_ord_pct"] > 0
                else None
            ),
            "share_of_req_cut_05x_ord_screen_pct": round(
                100.0 * screen_cut_pct / rec["req_cut_05x_ord_pct"], 2
            ),
        }

        if cell in SUPPORTS_ON_CELLS:
            m9n_cut_pct = m9n_cells[cell]["projected_cell_cut_pct"]
            t181_tail_pct = round(
                100.0 * t177_arith["split"][cell]["support_geometry_post_planner_tail_share_of_cell"],
                3,
            )
            # Additive on t175 cell-fraction basis (independent phases: prepass vs per_layer)
            stack_m9n_pct = round(m9n_cut_pct + screen_cut_pct, 3)
            stack_m9n_t181_half_pct = round(m9n_cut_pct + 0.5 * t181_tail_pct + screen_cut_pct, 3)
            stack_m9n_t181_full_pct = round(m9n_cut_pct + t181_tail_pct + screen_cut_pct, 3)
            entry["stacked_with_t182_and_t181_estimate_only"] = {
                "t182_m9n_k8_cut_pct": m9n_cut_pct,
                "t181_host_tail_ceiling_pct": t181_tail_pct,
                "est_ord_plus_t182": round(ord_pre * (1.0 - stack_m9n_pct / 100.0), 4),
                "est_acc_plus_t182": round(acc_pre * (1.0 - stack_m9n_pct / 100.0), 4),
                "est_ord_plus_t182_half_t181": round(ord_pre * (1.0 - stack_m9n_t181_half_pct / 100.0), 4),
                "est_ord_plus_t182_full_t181": round(ord_pre * (1.0 - stack_m9n_t181_full_pct / 100.0), 4),
            }

        route_est[cell] = entry

    return {
        "schema": 1,
        "note": (
            "ESTIMATE ONLY for cross-window route projections (t174 quiet board x t175 paired saving x t179 candidate). "
            "Screen wall/CPU figures are 3-round unsandboxed attribution screens, NEVER a formal landing gate. "
            "Profile scope_wall_ms values are parallel thread-wall sums under --profile (attribution only, never wall claims)."
        ),
        "qualification": qualification,
        "t177_reconciliation": t177_reconcile,
        "tree_support_invariance": ts_invariance,
        "tree_support_attribution": ts_attr,
        "infill_linker_attribution": il_attr,
        "infill_linker_benchy_l2": il_l2_benchy,
        "screen_summary": screen_summary,
        "combined_ts2_ilall_8cell_screen": combined_8cell,
        "combined_ts2_ilall_gate_summary": combined_gate_summary,
        "route_projections_estimate_only": route_est,
    }


def main() -> int:
    doc = compute_arithmetic()
    out_path = REDUCED / "arithmetic.json"
    out_path.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {out_path}")
    q = doc["qualification"]
    print(
        f"Qualification: total_runs={q['total_verified_runs']} all_byte_identical_t175={q['all_byte_identical_t175']} "
        f"fuel_fold={q['fuel_fold_exact_ok']}/{q['fuel_fold_total']}"
    )
    gs = doc["combined_ts2_ilall_gate_summary"]
    print(
        f"Combined TS-2+IL-ALL 8-cell screen: floor={gs['cells_clearing_080_wall_floor']}/8 "
        f"({gs['min_wall_saved_pct']:.2f}%..{gs['max_wall_saved_pct']:.2f}%, "
        f"{gs['min_floor_multiple_080']:.2f}x..{gs['max_floor_multiple_080']:.2f}x), "
        f"strict_wall={gs['cells_strict_wall_separation']}/8, strict_cpu={gs['cells_strict_cpu_separation']}/8 "
        f"(CPU {gs['min_cpu_saved_pct']:.2f}%..{gs['max_cpu_saved_pct']:.2f}%)"
    )
    print("Route projections (ESTIMATE ONLY):")
    for cell in ALL_CELLS:
        r = doc["route_projections_estimate_only"][cell]
        st = r.get("stacked_with_t182_and_t181_estimate_only")
        extra = (
            f" | +t182={st['est_ord_plus_t182']:.3f}x +t182+t181={st['est_ord_plus_t182_full_t181']:.3f}x"
            if st
            else ""
        )
        print(
            f"  {cell:18s}: pre={r['est_post_t175_ratio_ord']:.3f}x "
            f"t177_cut={r['t177_implied_combined_cut_pct']:5.2f}%-> {r['est_ratio_ord_after_t177_model']:.3f}x | "
            f"scr_cut={r['screen_measured_combined_cut_pct']:5.2f}%-> ord={r['est_ratio_ord_after_screen']:.3f}x "
            f"acc={r['est_ratio_acc_after_screen']:.3f}x{extra}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Checkout-only arithmetic reducer for t189.

Reads ONLY committed JSONs under docs/specs/perf-vs-orca/evidence/ and computes
reduced/arithmetic.json:
  1. 8-cell PostPass + run.rs tail substage attribution
  2. 8-cell Zero-WIT screen summary (A/B medians, 0.80% floor multiples, CPU
     corroboration disclosure, instrumented stage reductions)
  3. 4-cell x 6-config monotonic ablation summary
  4. Deterministic --profile guest-fuel delta summary
  5. ESTIMATE ONLY cross-window route projections (standalone post-t192 and
     stacked t190 + t191 + t192 across all 8 cells)
  6. Priced residual WIT-extension leads (PP-4 GCodePostProcess & PP-1b
     part-cooling entity-roles)
"""
from __future__ import annotations

import json
import statistics
from pathlib import Path

EVIDENCE_DIR = Path(__file__).resolve().parent
REDUCED_DIR = EVIDENCE_DIR / "reduced"
T182_REF = EVIDENCE_DIR.parent / "t182-spatial-hierarchical-phase3-union" / "reference.json"
T185_ROUTE = EVIDENCE_DIR.parent / "t185-support-geometry-host-tail-accel" / "reduced" / "route.json"
T188_ARITH = (
    EVIDENCE_DIR.parent / "t188-support-analysis-screen" / "reduced" / "arithmetic.json"
)

GATE_FLOOR_PCT = 0.80
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
SUPPORTS_ON_CELLS = {
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
}
ABLATION_CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]
FIN_MODULES = [
    "com.core.overhang-classifier-default",
    "com.core.part-cooling",
    "com.core.skirt-brim",
    "com.core.wipe-tower",
]
POSTPASS_WASM_MODULES = FIN_MODULES + ["com.core.machine-gcode-emit"]


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def compute_arithmetic(
    probe_wall: dict,
    screen: dict,
    t182_ref: dict,
    t185_route: dict,
    t188_arith: dict,
) -> dict:
    # 1. 8-cell substage attribution from 3-pass probe medians
    attribution = {}
    for cell in ALL_CELLS:
        med = probe_wall["cells"][cell]["median"]
        pr = med["probes"]
        stages = med["postpass_stage_ms"]
        slice_ms = med["slice_complete_ms"]
        postpass_ms = med["phase_ms"]["postpass"]

        fd = pr["fin_dispatch"]
        fa = pr["fin_apply"]
        ge = pr["gcode_emit"]
        gp = pr["gcode_postprocess"]
        gs_in = pr["gcode_serialize_inner"]
        gs_wr = pr["gcode_serialize_wrapper"]
        rt = pr["run_tail"]

        lf_total_ns = sum(fd[m]["total_ns"] + fa[m]["apply_ns"] for m in FIN_MODULES)
        lf_host_clone_drop_ns = sum(
            fd[m]["push_views_ns"] + fd[m]["store_drop_ns"] for m in FIN_MODULES
        )
        lf_empty_return_ns = (
            fd["com.core.wipe-tower"]["total_ns"]
            + fa["com.core.wipe-tower"]["apply_ns"]
            + fd["com.core.overhang-classifier-default"]["total_ns"]
            + fa["com.core.overhang-classifier-default"]["apply_ns"]
        )
        lf_modules = {}
        for m in FIN_MODULES:
            d = fd[m]
            a = fa[m]
            lf_modules[m] = {
                "dispatch_ms": round(d["total_ns"] / 1e6, 2),
                "push_views_ms": round(d["push_views_ns"] / 1e6, 2),
                "call_run_ms": round(d["call_run_ns"] / 1e6, 2),
                "store_drop_ms": round(d["store_drop_ns"] / 1e6, 2),
                "apply_ms": round(a["apply_ns"] / 1e6, 3),
                "pushes_count": d["pushes_count"],
                "host_oe_calls": d["host_oe_calls"],
                "layers": d["layers"],
                "entities": d["entities"],
                "points": d["points"],
                "took_rollback_clone": a["took_rollback_clone"],
            }

        rt_total_ns = (
            rt["second_estimate_ns"] + rt["layer_count_scan_ns"] + rt["gcode_ir_drop_ns"]
        )
        postpass_plus_tail_ms = postpass_ms + rt_total_ns / 1e6
        gs_bytes = gs_wr["output_bytes"]

        attribution[cell] = {
            "slice_complete_ms": slice_ms,
            "postpass_phase_ms": postpass_ms,
            "run_tail_ms": round(rt_total_ns / 1e6, 2),
            "postpass_plus_tail_ms": round(postpass_plus_tail_ms, 2),
            "postpass_plus_tail_pct_of_slice": round(
                100.0 * postpass_plus_tail_ms / slice_ms, 2
            ),
            "stage_ms": stages,
            "layer_finalization": {
                "sum_dispatch_apply_ms": round(lf_total_ns / 1e6, 2),
                "empty_return_modules_ms": round(lf_empty_return_ns / 1e6, 2),
                "empty_return_pct_of_lf": round(100.0 * lf_empty_return_ns / lf_total_ns, 2),
                "host_clone_plus_drop_ms": round(lf_host_clone_drop_ns / 1e6, 2),
                "host_clone_plus_drop_pct_of_lf": round(
                    100.0 * lf_host_clone_drop_ns / lf_total_ns, 2
                ),
                "skirt_brim_unused_layers_pct": round(
                    100.0
                    * (fd["com.core.skirt-brim"]["layers"] - 1)
                    / fd["com.core.skirt-brim"]["layers"],
                    2,
                ),
                "reconcile_ms": round(pr["postpass_reconcile"]["reconcile_ns"] / 1e6, 3),
                "modules": lf_modules,
            },
            "gcode_emit": {
                "total_ms": round(ge["total_ns"] / 1e6, 2),
                "drop_layers_ms": round(ge["drop_layers_ns"] / 1e6, 2),
                "drop_layers_pct": round(100.0 * ge["drop_layers_ns"] / ge["total_ns"], 2),
                "simplify_ms": round(ge["simplify_ns"] / 1e6, 2),
                "emit_moves_ms": round(ge["emit_moves_ns"] / 1e6, 2),
                "estimator_ms": round(ge["estimator_ns"] / 1e6, 2),
                "inject_m73_ms": round(ge["inject_m73_ns"] / 1e6, 2),
                "cmds_after_m73": ge["cmds_after_m73"],
                "points_in": ge["points_in"],
                "points_out": ge["points_out"],
            },
            "gcode_postprocess": {
                "total_ms": round(gp["total_ns"] / 1e6, 2),
                "call_run_ms": round(gp["call_run_ns"] / 1e6, 2),
                "call_run_pct": round(100.0 * gp["call_run_ns"] / gp["total_ns"], 2),
                "convert_in_ms": round(gp["convert_in_ns"] / 1e6, 2),
                "collect_out_ms": round(gp["collect_out_ns"] / 1e6, 2),
                "drop_ms": round(gp["drop_ns"] / 1e6, 2),
                "in_cmds": gp["in_cmds"],
                "out_cmds": gp["out_cmds"],
                "net_added_cmds": gp["out_cmds"] - gp["in_cmds"],
                "host_push_calls": gp["host_push_calls"],
            },
            "gcode_serialize": {
                "total_wrapper_ms": round(gs_wr["total_ns"] / 1e6, 2),
                "commands_loop_ms": round(gs_in["commands_loop_ns"] / 1e6, 2),
                "commands_loop_pct": round(
                    100.0 * gs_in["commands_loop_ns"] / gs_wr["total_ns"], 2
                ),
                "cmds": gs_in["cmds"],
                "output_bytes": gs_bytes,
                "ns_per_cmd": round(gs_in["commands_loop_ns"] / gs_in["cmds"], 1),
                "mb_per_s": round(
                    (gs_bytes / 1e6) / (gs_wr["total_ns"] / 1e9), 2
                ),
            },
            "run_tail": {
                "total_ms": round(rt_total_ns / 1e6, 2),
                "second_estimate_ms": round(rt["second_estimate_ns"] / 1e6, 2),
                "layer_count_scan_ms": round(rt["layer_count_scan_ns"] / 1e6, 2),
                "gcode_ir_drop_ms": round(rt["gcode_ir_drop_ns"] / 1e6, 2),
            },
        }

    # 2. 8-cell Zero-WIT screen summary
    screen_cells = {}
    total_wall_wins = 0
    total_pairs = 0
    cells_clearing_wall_floor = 0
    cells_cpu_corroborating = 0

    for cell in ALL_CELLS:
        pairs = screen["ab"][cell]["pairs"]
        bw = statistics.median(p["baseline_wall_s"] for p in pairs)
        cw = statistics.median(p["candidate_wall_s"] for p in pairs)
        bc = statistics.median(p["baseline_cpu_s"] for p in pairs)
        cc = statistics.median(p["candidate_cpu_s"] for p in pairs)

        wall_saved_s = bw - cw
        wall_saved_pct = 100.0 * wall_saved_s / bw
        cpu_saved_s = bc - cc
        cpu_saved_pct = 100.0 * cpu_saved_s / bc
        wall_wins = sum(1 for p in pairs if p["candidate_wall_s"] < p["baseline_wall_s"])
        cpu_wins = sum(1 for p in pairs if p["candidate_cpu_s"] < p["baseline_cpu_s"])
        total_wall_wins += wall_wins
        total_pairs += len(pairs)
        clears_wall = (wall_saved_pct >= GATE_FLOOR_PCT) and (wall_wins == len(pairs))
        if clears_wall:
            cells_clearing_wall_floor += 1
        cpu_corr = cpu_saved_pct > 0.0
        if cpu_corr:
            cells_cpu_corroborating += 1

        bi = screen["instrumented"][cell]["baseline"]["inst"]
        ci = screen["instrumented"][cell]["candidate"]["inst"]
        b_pp = bi["phase_ms"]["postpass"]
        c_pp = ci["phase_ms"]["postpass"]
        pp_saved_ms = b_pp - c_pp
        pp_saved_pct = 100.0 * pp_saved_ms / b_pp
        postpass_stage_implied_pct = 100.0 * (pp_saved_ms / 1000.0) / bw

        stage_deltas = {}
        for st in (
            "PostPass::LayerFinalization",
            "PostPass::GCodeEmit",
            "PostPass::GCodePostProcess",
            "PostPass::GCodeSerialize",
        ):
            bs = bi["postpass_stage_ms"][st]
            cs = ci["postpass_stage_ms"][st]
            saved = bs - cs
            stage_deltas[st] = {
                "base_ms": bs,
                "cand_ms": cs,
                "saved_ms": saved,
                "saved_pct": round(100.0 * saved / bs, 2) if bs > 0 else 0.0,
                "speedup_x": round(bs / cs, 2) if cs > 0 else None,
            }

        screen_cells[cell] = {
            "base_wall_med_s": round(bw, 5),
            "cand_wall_med_s": round(cw, 5),
            "wall_saved_s": round(wall_saved_s, 5),
            "wall_saved_pct": round(wall_saved_pct, 2),
            "gate_floor_multiple": round(wall_saved_pct / GATE_FLOOR_PCT, 2),
            "pair_wall_wins": wall_wins,
            "pair_count": len(pairs),
            "clears_wall_floor": clears_wall,
            "base_cpu_med_s": round(bc, 5),
            "cand_cpu_med_s": round(cc, 5),
            "cpu_saved_s": round(cpu_saved_s, 5),
            "cpu_saved_pct": round(cpu_saved_pct, 2),
            "pair_cpu_wins": cpu_wins,
            "cpu_corroborating": cpu_corr,
            "base_postpass_ms": b_pp,
            "cand_postpass_ms": c_pp,
            "postpass_saved_ms": pp_saved_ms,
            "postpass_saved_pct": round(pp_saved_pct, 2),
            "postpass_stage_implied_wall_pct": round(postpass_stage_implied_pct, 2),
            "stages": stage_deltas,
        }

    screen_aggregate = {
        "total_runs_verified_reference_match": screen["total_runs_verified_reference_match"],
        "cells_clearing_wall_floor": cells_clearing_wall_floor,
        "total_cells": len(ALL_CELLS),
        "total_wall_wins": total_wall_wins,
        "total_pairs": total_pairs,
        "min_wall_saved_pct": min(v["wall_saved_pct"] for v in screen_cells.values()),
        "max_wall_saved_pct": max(v["wall_saved_pct"] for v in screen_cells.values()),
        "min_gate_floor_multiple": min(v["gate_floor_multiple"] for v in screen_cells.values()),
        "max_gate_floor_multiple": max(v["gate_floor_multiple"] for v in screen_cells.values()),
        "cells_cpu_corroborating": cells_cpu_corroborating,
        "min_cpu_saved_pct": min(v["cpu_saved_pct"] for v in screen_cells.values()),
        "max_cpu_saved_pct": max(v["cpu_saved_pct"] for v in screen_cells.values()),
    }

    # 3. 4-cell ablation summary
    ablation_summary = {}
    for cell in ABLATION_CELLS:
        ab = screen["ablation"][cell]
        rows = {}
        for cfg, rec in ab.items():
            inst = rec["inst"]
            rows[cfg] = {
                "wall_s": rec["wall_s"],
                "postpass_ms": inst["phase_ms"]["postpass"],
                "lf_ms": inst["postpass_stage_ms"]["PostPass::LayerFinalization"],
                "emit_ms": inst["postpass_stage_ms"]["PostPass::GCodeEmit"],
                "postproc_ms": inst["postpass_stage_ms"]["PostPass::GCodePostProcess"],
                "ser_ms": inst["postpass_stage_ms"]["PostPass::GCodeSerialize"],
            }
        base = rows["baseline"]
        ablation_summary[cell] = {
            "configs": rows,
            "pp2_ser_saved_ms": base["ser_ms"] - rows["pp2_only"]["ser_ms"],
            "pp3_emit_saved_ms": base["emit_ms"] - rows["pp3_only"]["emit_ms"],
            "pp1_lf_saved_ms": base["lf_ms"] - rows["pp1_only"]["lf_ms"],
            "host_only_postpass_saved_ms": base["postpass_ms"] - rows["pp2_pp3_host_only"]["postpass_ms"],
            "full_postpass_saved_ms": base["postpass_ms"] - rows["full_pp1_pp2_pp3"]["postpass_ms"],
        }

    # 4. Deterministic --profile guest-fuel summary
    profile_fuel_summary = {}
    for cell in ("benchy-classic-on", "base-classic-on"):
        b = screen["profile_fuel"][cell]["baseline"]
        c = screen["profile_fuel"][cell]["candidate"]
        mods = {}
        b_lf_sum = 0
        c_lf_sum = 0
        for m in POSTPASS_WASM_MODULES:
            bf = b["postpass_modules"][m]["total_fuel"]
            cf = c["postpass_modules"][m]["total_fuel"]
            saved = bf - cf
            if m in FIN_MODULES:
                b_lf_sum += bf
                c_lf_sum += cf
            mods[m] = {
                "base_fuel": bf,
                "cand_fuel": cf,
                "saved_fuel": saved,
                "saved_pct": round(100.0 * saved / bf, 2) if bf > 0 else 0.0,
                "speedup_x": round(bf / cf, 2) if cf > 0 else None,
            }
        lf_saved = b_lf_sum - c_lf_sum
        tot_b = b["fuel_total"]
        tot_c = c["fuel_total"]
        profile_fuel_summary[cell] = {
            "total_fuel_base": tot_b,
            "total_fuel_cand": tot_c,
            "total_fuel_saved": tot_b - tot_c,
            "total_fuel_saved_pct": round(100.0 * (tot_b - tot_c) / tot_b, 2),
            "layer_finalization_fuel_base": b_lf_sum,
            "layer_finalization_fuel_cand": c_lf_sum,
            "layer_finalization_fuel_saved": lf_saved,
            "layer_finalization_fuel_saved_pct": round(100.0 * lf_saved / b_lf_sum, 2),
            "modules": mods,
        }

    # 5. ESTIMATE ONLY cross-window route projections
    route_standalone = {}
    route_stacked = {}
    est_below_1x_ab = 0
    est_below_1x_stage = 0

    for cell in ALL_CELLS:
        t185_ord = t185_route["cells"][cell]["est_post_t185_ratio_ordinary"]
        sc = screen_cells[cell]
        w_pct = sc["wall_saved_pct"]
        st_pct = sc["postpass_stage_implied_wall_pct"]

        sa_ab = t185_ord * (1.0 - w_pct / 100.0)
        sa_st = t185_ord * (1.0 - st_pct / 100.0)
        route_standalone[cell] = {
            "t185_est_ord": round(t185_ord, 4),
            "t189_wall_saved_pct": w_pct,
            "t189_postpass_stage_implied_pct": st_pct,
            "post_t192_standalone_est_ord_ab": round(sa_ab, 4),
            "post_t192_standalone_est_ord_stage": round(sa_st, 4),
            "crosses_1x_ab": sa_ab < 1.0,
        }

        if cell in SUPPORTS_ON_CELLS:
            pre_ab = t188_arith["route_estimate_only_stacked_t187_t188"][cell]["stacked_est_ord_ab"]
            pre_st = t188_arith["route_estimate_only_stacked_t187_t188"][cell]["stacked_est_ord_stage"]
        else:
            pre_ab = t185_ord
            pre_st = t185_ord

        fin_ab = pre_ab * (1.0 - w_pct / 100.0)
        fin_st = pre_st * (1.0 - st_pct / 100.0)
        if fin_ab < 1.0:
            est_below_1x_ab += 1
        if fin_st < 1.0:
            est_below_1x_stage += 1
        route_stacked[cell] = {
            "pre_t192_est_ord_ab": round(pre_ab, 4),
            "pre_t192_est_ord_stage": round(pre_st, 4),
            "t189_wall_saved_pct": w_pct,
            "t189_postpass_stage_implied_pct": st_pct,
            "stacked_t190_t191_t192_est_ord_ab": round(fin_ab, 4),
            "stacked_t190_t191_t192_est_ord_stage": round(fin_st, 4),
            "crosses_1x_ab": fin_ab < 1.0,
            "crosses_1x_stage": fin_st < 1.0,
        }

    # 6. Priced residual WIT-extension leads (after t192 lands)
    residual_wit = {}
    for cell in ALL_CELLS:
        cand_wall_ms = screen_cells[cell]["cand_wall_med_s"] * 1000.0
        ci_mods = screen["instrumented"][cell]["candidate"]["inst"]["postpass_module_ms"]
        ci_stages = screen["instrumented"][cell]["candidate"]["inst"]["postpass_stage_ms"]
        gp_ms = ci_stages["PostPass::GCodePostProcess"]
        pc_ms = ci_mods["PostPass::LayerFinalization/com.core.part-cooling"]
        residual_wit[cell] = {
            "post_t192_cand_wall_ms": round(cand_wall_ms, 1),
            "pp4_gcode_postprocess_ms": gp_ms,
            "pp4_pct_of_post_t192_wall": round(100.0 * gp_ms / cand_wall_ms, 2),
            "pp1b_part_cooling_ms": pc_ms,
            "pp1b_pct_of_post_t192_wall": round(100.0 * pc_ms / cand_wall_ms, 2),
            "combined_wit_leads_ms": gp_ms + pc_ms,
            "combined_pct_of_post_t192_wall": round(100.0 * (gp_ms + pc_ms) / cand_wall_ms, 2),
        }

    return {
        "schema": "t189_arithmetic_v1",
        "gate_floor_pct": GATE_FLOOR_PCT,
        "reference_currency": "t182-spatial-hierarchical-phase3-union/reference.json",
        "attribution": attribution,
        "screen_aggregate": screen_aggregate,
        "screen_cells": screen_cells,
        "ablation_summary": ablation_summary,
        "profile_fuel_summary": profile_fuel_summary,
        "route_estimate_only": {
            "note": (
                "ESTIMATE ONLY — cross-window composition of t185/reduced/route.json and "
                "t188/reduced/arithmetic.json (t190+t191) with t189 3-round screen A/B median "
                "savings (and conservative instrumented PostPass-only stage-delta rate). "
                "Not a scoreboard ratio of record."
            ),
            "est_cells_below_1_00x_stacked_ab": est_below_1x_ab,
            "est_cells_below_1_00x_stacked_stage": est_below_1x_stage,
            "standalone_post_t192": route_standalone,
            "stacked_t190_t191_t192": route_stacked,
        },
        "residual_wit_leads_priced": residual_wit,
    }


def main() -> int:
    probe_wall = load_json(REDUCED_DIR / "probe_wall.json")
    screen = load_json(REDUCED_DIR / "screen.json")
    t182_ref = load_json(T182_REF)
    t185_route = load_json(T185_ROUTE)
    t188_arith = load_json(T188_ARITH)
    out = compute_arithmetic(probe_wall, screen, t182_ref, t185_route, t188_arith)
    (REDUCED_DIR / "arithmetic.json").write_text(
        json.dumps(out, indent=2) + "\n", encoding="utf-8"
    )
    print(f"Wrote {REDUCED_DIR / 'arithmetic.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

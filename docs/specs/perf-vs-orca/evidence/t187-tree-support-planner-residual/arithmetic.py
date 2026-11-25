#!/usr/bin/env python3
"""Checkout-only arithmetic reducer for t187.

Reads only committed JSONs in docs/specs/perf-vs-orca/evidence/ and computes
reduced/arithmetic.json (invariance proof, attribution tables, screen gate-floor
multiples, fuel/host-op deltas, and ESTIMATE ONLY cross-window route projections).
"""

from __future__ import annotations

import json
from pathlib import Path

EVIDENCE_DIR = Path(__file__).resolve().parent
REDUCED_DIR = EVIDENCE_DIR / "reduced"
T182_REF = (
    EVIDENCE_DIR.parent
    / "t182-spatial-hierarchical-phase3-union"
    / "reference.json"
)
T185_ROUTE = (
    EVIDENCE_DIR.parent
    / "t185-support-geometry-host-tail-accel"
    / "reduced"
    / "route.json"
)

GATE_FLOOR_PCT = 0.80
SUPPORTS_ON_CELLS = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def compute_arithmetic(
    probe_wall: dict,
    probe_fuel: dict,
    screen: dict,
    t182_ref: dict,
    t185_route: dict,
) -> dict:
    ref_cells = {
        cell: {"sha256": sha}
        for cell, sha in t182_ref["active_set"].items()
    }

    # 1. Classic == Arachne invariance check (probe + clean baseline + clean candidate)
    invariance = {}
    for model in ("benchy", "base"):
        c_cell = f"{model}-classic-on"
        a_cell = f"{model}-arachne-on"
        pw_c = probe_wall["cells"][c_cell]["median"]
        pw_a = probe_wall["cells"][a_cell]["median"]
        counter_diffs = {
            k: [pw_c["counters"][k], pw_a["counters"][k]]
            for k in pw_c["counters"]
            if pw_c["counters"][k] != pw_a["counters"][k]
        }
        pf_c = probe_fuel["cells"][c_cell]
        pf_a = probe_fuel["cells"][a_cell]
        sc_c = screen["supports_on"][c_cell]["profile"]
        sc_a = screen["supports_on"][a_cell]["profile"]

        invariance[model] = {
            "probe_counter_diff_count": len(counter_diffs),
            "probe_phase5_audit_equal": pw_c["phase5_audit"] == pw_a["phase5_audit"],
            "probe_fuel_classic": pf_c["run_support_geometry_total_fuel"],
            "probe_fuel_arachne": pf_a["run_support_geometry_total_fuel"],
            "probe_fuel_exact_match": (
                pf_c["run_support_geometry_total_fuel"]
                == pf_a["run_support_geometry_total_fuel"]
            ),
            "clean_baseline_planner_fuel_classic": sc_c["baseline"]["tree_support_planner"]["total_fuel"],
            "clean_baseline_planner_fuel_arachne": sc_a["baseline"]["tree_support_planner"]["total_fuel"],
            "clean_baseline_planner_fuel_exact_match": (
                sc_c["baseline"]["tree_support_planner"]["total_fuel"]
                == sc_a["baseline"]["tree_support_planner"]["total_fuel"]
            ),
            "clean_candidate_planner_fuel_classic": sc_c["candidate"]["tree_support_planner"]["total_fuel"],
            "clean_candidate_planner_fuel_arachne": sc_a["candidate"]["tree_support_planner"]["total_fuel"],
            "clean_candidate_planner_fuel_exact_match": (
                sc_c["candidate"]["tree_support_planner"]["total_fuel"]
                == sc_a["candidate"]["tree_support_planner"]["total_fuel"]
            ),
        }

    # 2. Attribution summary per model (using classic-on 3-pass median)
    attribution = {}
    for model in ("benchy", "base"):
        cell = f"{model}-classic-on"
        med = probe_wall["cells"][cell]["median"]
        disp_trad = med["dispatch"]["com.core.traditional-support-planner"]
        disp_tree = med["dispatch"]["com.core.tree-support-planner"]
        scopes = med["scopes"]
        host_ops = med["host_ops"]
        ctr = med["counters"]

        run_total_ns = scopes["run_support_geometry_total"]["inc_wall_ns"]

        # Host-op serial vs batch totals
        host_total_wall_ns = sum(v["batch_wall_ns"] for v in host_ops.values())
        host_total_worker_ns = sum(v["worker_total_ns"] for v in host_ops.values())
        host_serial_wall_ns = sum(
            v["batch_wall_ns"]
            for k, v in host_ops.items()
            if "_single" in k
        )
        host_batch_wall_ns = sum(
            v["batch_wall_ns"]
            for k, v in host_ops.items()
            if "_batch" in k
        )

        # Avoidance & collision over-materialization
        avoid_mat = ctr["tv_avoidance_layers_materialized"]
        avoid_maxp1 = ctr["tv_avoidance_max_layer_plus1_sum"]
        avoid_unused_above = avoid_mat - avoid_maxp1
        coll_mat = ctr["tv_collision_layers_materialized"]
        coll_distinct = ctr["tv_collision_distinct_layers_queried"]

        mo_off = host_ops["move_out_expolys_offset::offset_single"]
        mo_uni = host_ops["move_out_expolys_union::clip_single(Union)"]

        # Top fuel::* scopes from probe_fuel
        pf_scopes = probe_fuel["cells"][cell]["scopes"]
        pf_total = probe_fuel["cells"][cell]["run_support_geometry_total_fuel"]
        fine_fuel = sorted(
            [
                {
                    "scope": k,
                    "calls": v["calls"],
                    "inc_fuel": v["inc_fuel"],
                    "exc_fuel": v["exc_fuel"],
                    "inc_pct_of_probed_planner": round(v["inc_fuel"] / pf_total * 100.0, 2),
                }
                for k, v in pf_scopes.items()
                if k.startswith("fuel::")
            ],
            key=lambda x: x["inc_fuel"],
            reverse=True,
        )

        attribution[model] = {
            "traditional_support_planner_wasted_dispatch_ms": round(
                disp_trad["total_ns"] / 1e6, 2
            ),
            "traditional_wit_call_ms": round(disp_trad["call_run_ns"] / 1e6, 2),
            "tree_support_planner_dispatch_total_ms": round(
                disp_tree["total_ns"] / 1e6, 2
            ),
            "tree_support_planner_wit_call_ms": round(
                disp_tree["call_run_ns"] / 1e6, 2
            ),
            "tree_support_planner_run_body_ms": round(run_total_ns / 1e6, 2),
            "tree_support_planner_wit_input_lowering_est_ms": round(
                (disp_tree["call_run_ns"] - run_total_ns) / 1e6, 2
            ),
            "tree_support_planner_harvest_ms": round(disp_tree["harvest_ns"] / 1e6, 2),
            "host_ops_total_wall_ms": round(host_total_wall_ns / 1e6, 2),
            "host_ops_serial_wall_ms": round(host_serial_wall_ns / 1e6, 2),
            "host_ops_batch_wall_ms": round(host_batch_wall_ns / 1e6, 2),
            "host_ops_total_worker_thread_ms": round(host_total_worker_ns / 1e6, 2),
            "guest_plus_marshal_residual_wall_ms": round(
                (run_total_ns - host_total_wall_ns) / 1e6, 2
            ),
            "avoidance_buckets": ctr["tv_avoidance_buckets_materialized"],
            "avoidance_layers_materialized": avoid_mat,
            "avoidance_max_layer_plus1_sum": avoid_maxp1,
            "avoidance_layers_strictly_above_max_queried": avoid_unused_above,
            "avoidance_unused_above_pct": round(avoid_unused_above / avoid_mat * 100.0, 2),
            "avoidance_wall_ms": round(scopes["tv_ensure_avoidance"]["inc_wall_ns"] / 1e6, 2),
            "collision_buckets": ctr["tv_collision_buckets_materialized"],
            "collision_layers_materialized": coll_mat,
            "collision_distinct_layers_queried": coll_distinct,
            "collision_wall_ms": round(scopes["tv_ensure_collision"]["inc_wall_ns"] / 1e6, 2),
            "move_out_offset_dup_pct": round(
                mo_off["dup_requests"] / mo_off["requests"] * 100.0, 2
            ),
            "move_out_dup_wall_ms": round(
                (mo_off["dup_wall_ns"] + mo_uni["dup_wall_ns"]) / 1e6, 2
            ),
            "contact_erode_serial_wall_ms": round(
                host_ops["contact_sample_erode::offset_single"]["batch_wall_ns"] / 1e6, 2
            ),
            "emit_prefetch_occupancy_wall_ms": round(
                scopes["emit_prefetch_occupancy"]["inc_wall_ns"] / 1e6, 2
            ),
            "br_phase3_union_wall_ms": round(
                scopes["br_phase3_union"]["inc_wall_ns"] / 1e6, 2
            ),
            "br_phase5_set_diff_wall_ms": round(
                scopes["br_phase5_set_diff"]["inc_wall_ns"] / 1e6, 2
            ),
            "phase5_audit": med["phase5_audit"],
            "top_fine_fuel_scopes": fine_fuel,
        }

    # 3. Screen metrics, gate multiples, and ESTIMATE ONLY route projection
    screen_cells = {}
    route_cells = {}
    for cell in SUPPORTS_ON_CELLS:
        sc = screen["supports_on"][cell]
        ab = sc["ab_summary"]
        inst_b = sc["instrumented"]["baseline"]
        inst_c = sc["instrumented"]["candidate"]
        prof_b = sc["profile"]["baseline"]
        prof_c = sc["profile"]["candidate"]

        tsp_b_ms = inst_b["tree_support_planner_ms"]
        tsp_c_ms = inst_c["tree_support_planner_ms"]
        sg_b_ms = inst_b["support_geometry_stage_ms"]
        sg_c_ms = inst_c["support_geometry_stage_ms"]

        fuel_b = prof_b["tree_support_planner"]["total_fuel"]
        fuel_c = prof_c["tree_support_planner"]["total_fuel"]
        fuel_cut_pct = round((fuel_b - fuel_c) / fuel_b * 100.0, 2)

        hn_b = prof_b["host_native"]
        hn_c = prof_c["host_native"]
        hn_calls_cut_pct = round((hn_b["calls"] - hn_c["calls"]) / hn_b["calls"] * 100.0, 2)
        hn_thread_cut_pct = round(
            (hn_b["accumulated_thread_wall_ns"] - hn_c["accumulated_thread_wall_ns"])
            / hn_b["accumulated_thread_wall_ns"]
            * 100.0,
            2,
        )

        # Conservative stage-implied saving = (sg_b_ms - sg_c_ms) / (ab_base_med_s * 1000)
        stage_saved_ms = sg_b_ms - sg_c_ms
        stage_implied_wall_pct = round(
            stage_saved_ms / (ab["baseline_wall_med_s"] * 1000.0) * 100.0, 2
        )

        screen_cells[cell] = {
            "reference_sha256": ref_cells[cell]["sha256"],
            "all_10_runs_reference_match": sc["all_10_runs_reference_match"],
            "ab_wall_saved_s": ab["wall_saved_s"],
            "ab_wall_saved_pct": ab["wall_saved_pct"],
            "gate_floor_multiple": round(ab["wall_saved_pct"] / GATE_FLOOR_PCT, 2),
            "wall_pair_wins": ab["wall_pair_wins"],
            "ab_cpu_saved_pct": ab["cpu_saved_pct"],
            "cpu_pair_wins": ab["cpu_pair_wins"],
            "tree_support_planner_ms_baseline": tsp_b_ms,
            "tree_support_planner_ms_candidate": tsp_c_ms,
            "tree_support_planner_saved_ms": tsp_b_ms - tsp_c_ms,
            "tree_support_planner_saved_pct": round((tsp_b_ms - tsp_c_ms) / tsp_b_ms * 100.0, 2),
            "support_geometry_stage_ms_baseline": sg_b_ms,
            "support_geometry_stage_ms_candidate": sg_c_ms,
            "support_geometry_stage_saved_ms": stage_saved_ms,
            "support_geometry_stage_saved_pct": round(stage_saved_ms / sg_b_ms * 100.0, 2),
            "stage_implied_wall_saved_pct": stage_implied_wall_pct,
            "planner_fuel_baseline": fuel_b,
            "planner_fuel_candidate": fuel_c,
            "planner_fuel_saved_pct": fuel_cut_pct,
            "host_native_calls_baseline": hn_b["calls"],
            "host_native_calls_candidate": hn_c["calls"],
            "host_native_calls_saved_pct": hn_calls_cut_pct,
            "host_native_accumulated_thread_s_baseline": round(
                hn_b["accumulated_thread_wall_ns"] / 1e9, 3
            ),
            "host_native_accumulated_thread_s_candidate": round(
                hn_c["accumulated_thread_wall_ns"] / 1e9, 3
            ),
            "host_native_accumulated_thread_saved_pct": hn_thread_cut_pct,
            "host_support_analysis_calls_unchanged": (
                prof_b["host_support_analysis"]["calls"]
                == prof_c["host_support_analysis"]["calls"]
            ),
        }

        # ESTIMATE ONLY cross-window route composition
        base_ratio = t185_route["cells"][cell]["est_post_t185_ratio_ordinary"]
        est_ratio_ab = base_ratio * (1.0 - ab["wall_saved_pct"] / 100.0)
        est_ratio_stage = base_ratio * (1.0 - stage_implied_wall_pct / 100.0)
        route_cells[cell] = {
            "est_post_t185_ratio_ordinary": round(base_ratio, 4),
            "est_post_t190_ratio_ordinary_from_ab_median": round(est_ratio_ab, 4),
            "est_post_t190_ratio_ordinary_from_stage_delta": round(est_ratio_stage, 4),
            "crosses_below_1_00x": est_ratio_ab < 1.0,
        }

    # Include supports-off unchanged ratios from t185 route for 8-cell count
    for cell, r in t185_route["cells"].items():
        if cell not in route_cells:
            route_cells[cell] = {
                "est_post_t185_ratio_ordinary": round(r["est_post_t185_ratio_ordinary"], 4),
                "est_post_t190_ratio_ordinary_from_ab_median": round(
                    r["est_post_t185_ratio_ordinary"], 4
                ),
                "est_post_t190_ratio_ordinary_from_stage_delta": round(
                    r["est_post_t185_ratio_ordinary"], 4
                ),
                "crosses_below_1_00x": r["est_post_t185_ratio_ordinary"] < 1.0,
            }

    est_cells_under_1x = sum(
        1 for v in route_cells.values() if v["est_post_t190_ratio_ordinary_from_ab_median"] < 1.0
    )

    return {
        "schema": 1,
        "gate_floor_pct": GATE_FLOOR_PCT,
        "classic_arachne_invariance": invariance,
        "attribution": attribution,
        "screen_cells": screen_cells,
        "screen_summary": {
            "total_measured_runs": screen["measured_runs_count"],
            "all_measured_runs_reference_match": screen["all_measured_runs_reference_match"],
            "total_ab_wall_wins": sum(v["wall_pair_wins"] for v in screen_cells.values()),
            "total_ab_pairs": 12,
            "all_4_clear_t137_floor_with_cpu_corroborating": all(
                v["ab_wall_saved_pct"] >= GATE_FLOOR_PCT
                and v["ab_cpu_saved_pct"] > 0.0
                and v["wall_pair_wins"] == 3
                for v in screen_cells.values()
            ),
            "supports_off_all_reference_match": all(
                v["reference_match"]
                for v in screen["supports_off_inertness"].values()
            ),
            "p5_tile_falsified_on_base": (
                screen["falsified_candidates"]["P5_tile_phase5_collision_tile_windowing"][
                    "cells"
                ]["base-classic-on"]["reference_match"]
                is False
            ),
        },
        "route_projection_estimate_only": {
            "note": "ESTIMATE ONLY — cross-window composition of t185/reduced/route.json est_post_t185_ratio_ordinary with t187 3-round screen A/B median savings (and conservative instrumented stage-delta rate). Not a scoreboard ratio of record.",
            "est_cells_below_1_00x_ordinary": est_cells_under_1x,
            "cells": route_cells,
        },
    }


def main() -> None:
    probe_wall = load_json(REDUCED_DIR / "probe_wall.json")
    probe_fuel = load_json(REDUCED_DIR / "probe_fuel.json")
    screen = load_json(REDUCED_DIR / "screen.json")
    t182_ref = load_json(T182_REF)
    t185_route = load_json(T185_ROUTE)

    out = compute_arithmetic(probe_wall, probe_fuel, screen, t182_ref, t185_route)
    (REDUCED_DIR / "arithmetic.json").write_text(
        json.dumps(out, indent=2) + "\n", encoding="utf-8"
    )
    print(f"Wrote {REDUCED_DIR / 'arithmetic.json'}")


if __name__ == "__main__":
    main()

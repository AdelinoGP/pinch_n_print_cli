#!/usr/bin/env python3
"""Checkout-only arithmetic reducer for t188.

Reads only committed JSONs in docs/specs/perf-vs-orca/evidence/ and computes
reduced/arithmetic.json (classic==arachne invariance proof, substage attribution,
screen gate-floor multiples, host:support_analysis call/time deltas, ablation
ladder, and ESTIMATE ONLY cross-window route projections standalone & stacked
with t187/t190).
"""
from __future__ import annotations

import json
import statistics
from pathlib import Path

EVIDENCE_DIR = Path(__file__).resolve().parent
REDUCED_DIR = EVIDENCE_DIR / "reduced"
T182_REF = EVIDENCE_DIR.parent / "t182-spatial-hierarchical-phase3-union" / "reference.json"
T185_ROUTE = EVIDENCE_DIR.parent / "t185-support-geometry-host-tail-accel" / "reduced" / "route.json"
T187_ARITH = (
    EVIDENCE_DIR.parent / "t187-tree-support-planner-residual" / "reduced" / "arithmetic.json"
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
    screen: dict,
    t182_ref: dict,
    t185_route: dict,
    t187_arith: dict,
) -> dict:
    # 1. Classic == Arachne invariance inside PrePass::SupportAnalysis
    invariance = {}
    for model in ("benchy", "base"):
        c_cell = f"{model}-classic-on"
        a_cell = f"{model}-arachne-on"
        pc = probe_wall["cells"][c_cell]["median"]
        pa = probe_wall["cells"][a_cell]["median"]
        sc_c = screen["supports_on"][c_cell]["profile"]
        sc_a = screen["supports_on"][a_cell]["profile"]
        invariance[model] = {
            "probe_counts_equal": pc["counts"] == pa["counts"],
            "probe_op_counts_equal": {
                k: pc["ops"][k]["count"] for k in pc["ops"]
            } == {k: pa["ops"][k]["count"] for k in pa["ops"]},
            "probe_op_vin_equal": {
                k: pc["ops"][k]["vin"] for k in pc["ops"]
            } == {k: pa["ops"][k]["vin"] for k in pa["ops"]},
            "probe_op_vout_equal": {
                k: pc["ops"][k]["vout"] for k in pc["ops"]
            } == {k: pa["ops"][k]["vout"] for k in pa["ops"]},
            "baseline_host_sa_calls_classic": sc_c["baseline"]["sa_total_calls"],
            "baseline_host_sa_calls_arachne": sc_a["baseline"]["sa_total_calls"],
            "baseline_host_sa_calls_equal": (
                sc_c["baseline"]["sa_total_calls"] == sc_a["baseline"]["sa_total_calls"]
            ),
            "candidate_host_sa_calls_classic": sc_c["candidate"]["sa_total_calls"],
            "candidate_host_sa_calls_arachne": sc_a["candidate"]["sa_total_calls"],
            "candidate_host_sa_calls_equal": (
                sc_c["candidate"]["sa_total_calls"] == sc_a["candidate"]["sa_total_calls"]
            ),
        }

    # 2. Substage attribution summary per model (classic-on 3-pass median)
    attribution = {}
    for model in ("benchy", "base"):
        cell = f"{model}-classic-on"
        med = probe_wall["cells"][cell]["median"]
        tot_ns = med["total_wall_ns"]
        ph = med["phases"]
        ops = med["ops"]
        serial_ns = ph["ppo_ns"] + ph["layer_union_ns"] + ph["gather_ns"] + ph["post_contacts_ns"]
        contacts_sum_ns = ops["contact_item_total"]["ns"]
        attribution[model] = {
            "total_wall_ms": round(tot_ns / 1e6, 2),
            "ppo_ms": round(ph["ppo_ns"] / 1e6, 2),
            "ppo_pct": round(100.0 * ph["ppo_ns"] / tot_ns, 2),
            "ppo_make_lines_ms": round(ops["ppo_make_lines"]["ns"] / 1e6, 2),
            "ppo_chain_ms": round(ops["ppo_chain"]["ns"] / 1e6, 2),
            "ppo_world_mesh_ms": round(ops["ppo_world_mesh"]["ns"] / 1e6, 2),
            "layer_union_ms": round(ph["layer_union_ns"] / 1e6, 2),
            "layer_union_pct": round(100.0 * ph["layer_union_ns"] / tot_ns, 2),
            "serial_total_ms": round(serial_ns / 1e6, 2),
            "serial_total_pct": round(100.0 * serial_ns / tot_ns, 2),
            "contacts_par_wall_ms": round(ph["contacts_par_ns"] / 1e6, 2),
            "contacts_par_wall_pct": round(100.0 * ph["contacts_par_ns"] / tot_ns, 2),
            "contacts_thread_sum_ms": round(contacts_sum_ns / 1e6, 2),
            "contacts_effective_parallelism": round(contacts_sum_ns / ph["contacts_par_ns"], 2),
            "contacts_max_layer_ms": round(med["layer_summary"]["max_total_ns"] / 1e6, 2),
            "contacts_p50_layer_ms": round(med["layer_summary"]["p50_ns"] / 1e6, 2),
            "grown_offset_ms": round(ops["grown_offset"]["ns"] / 1e6, 2),
            "grown_offset_pct_of_contacts": round(
                100.0 * ops["grown_offset"]["ns"] / contacts_sum_ns, 2
            ),
            "cantilever_offset_ms": round(ops["cantilever_offset"]["ns"] / 1e6, 2),
            "cantilever_offset_pct_of_contacts": round(
                100.0 * ops["cantilever_offset"]["ns"] / contacts_sum_ns, 2
            ),
            "enforced_blocked_empty_ms": round(
                ops["enforced_blocked_intersections"]["ns"] / 1e6, 2
            ),
            "enforced_blocked_empty_calls": ops["enforced_blocked_intersections"]["count"] * 2,
            "trimmed_diff_ms": round(ops["trimmed_diff"]["ns"] / 1e6, 2),
            "expand_back_total_ms": round(
                (ops["expand_back_offset"]["ns"] + ops["expand_back_intersect_diff"]["ns"]) / 1e6, 2
            ),
            "cantilever_hits": med["counts"]["cantilever_count"],
            "candidates_count": med["counts"]["candidates_count"],
            "single_region_union_changed": med["counts"]["single_region_union_changed"],
        }

    # 3. Screen A/B, stage delta, host:support_analysis deltas, and route projections
    screen_summary = {}
    route_standalone = {}
    route_stacked_with_t187 = {}

    for cell in SUPPORTS_ON_CELLS:
        entry = screen["supports_on"][cell]
        pairs = entry["ab_pairs"]
        bw = statistics.median(p["baseline_wall_s"] for p in pairs)
        cw = statistics.median(p["candidate_wall_s"] for p in pairs)
        bc = statistics.median(p["baseline_cpu_s"] for p in pairs)
        cc = statistics.median(p["candidate_cpu_s"] for p in pairs)
        wall_saved_s = bw - cw
        wall_saved_pct = 100.0 * wall_saved_s / bw
        cpu_saved_s = bc - cc
        cpu_saved_pct = 100.0 * cpu_saved_s / bc
        pair_wall_wins = sum(1 for p in pairs if p["candidate_wall_s"] < p["baseline_wall_s"])
        pair_cpu_wins = sum(1 for p in pairs if p["candidate_cpu_s"] < p["baseline_cpu_s"])

        bi = entry["inst"]["baseline"]
        ci = entry["inst"]["candidate"]
        sa_saved_ms = bi["sa_stage_ms"] - ci["sa_stage_ms"]
        sa_saved_pct = 100.0 * sa_saved_ms / bi["sa_stage_ms"]
        stage_implied_pct = 100.0 * (sa_saved_ms / 1000.0) / bw

        bp = entry["profile"]["baseline"]
        cp = entry["profile"]["candidate"]
        calls_saved = bp["sa_total_calls"] - cp["sa_total_calls"]
        calls_saved_pct = 100.0 * calls_saved / bp["sa_total_calls"]
        host_sa_saved_pct = (
            100.0 * (bp["sa_total_wall_ns"] - cp["sa_total_wall_ns"]) / bp["sa_total_wall_ns"]
        )

        screen_summary[cell] = {
            "base_wall_med_s": round(bw, 4),
            "cand_wall_med_s": round(cw, 4),
            "wall_saved_s": round(wall_saved_s, 4),
            "wall_saved_pct": round(wall_saved_pct, 2),
            "gate_floor_multiple": round(wall_saved_pct / GATE_FLOOR_PCT, 2),
            "pair_wall_wins": pair_wall_wins,
            "pair_count": len(pairs),
            "base_cpu_med_s": round(bc, 4),
            "cand_cpu_med_s": round(cc, 4),
            "cpu_saved_pct": round(cpu_saved_pct, 2),
            "pair_cpu_wins": pair_cpu_wins,
            "cpu_corroborating": cpu_saved_pct > 0.0,
            "clears_t137_floor": wall_saved_pct >= GATE_FLOOR_PCT and cpu_saved_pct > 0.0,
            "base_sa_ms": bi["sa_stage_ms"],
            "cand_sa_ms": ci["sa_stage_ms"],
            "sa_saved_ms": sa_saved_ms,
            "sa_saved_pct": round(sa_saved_pct, 2),
            "sa_speedup_x": round(bi["sa_stage_ms"] / ci["sa_stage_ms"], 2),
            "stage_implied_wall_pct": round(stage_implied_pct, 2),
            "base_host_sa_calls": bp["sa_total_calls"],
            "cand_host_sa_calls": cp["sa_total_calls"],
            "host_sa_calls_saved": calls_saved,
            "host_sa_calls_saved_pct": round(calls_saved_pct, 2),
            "base_host_sa_s": round(bp["sa_total_wall_ns"] / 1e9, 4),
            "cand_host_sa_s": round(cp["sa_total_wall_ns"] / 1e9, 4),
            "host_sa_wall_saved_pct": round(host_sa_saved_pct, 2),
            "base_offset_calls": bp["scopes"]["polygon_ops::offset"]["calls"],
            "cand_offset_calls": cp["scopes"]["polygon_ops::offset"]["calls"],
            "base_clip_calls": bp["scopes"]["polygon_ops::clip_polygons"]["calls"],
            "cand_clip_calls": cp["scopes"]["polygon_ops::clip_polygons"]["calls"],
        }

        # ESTIMATE ONLY route projections from t185/reduced/route.json (and stacked with t187/t190)
        t185_ord = t185_route["cells"][cell]["est_post_t185_ratio_ordinary"]
        t187_cell = t187_arith["route_projection_estimate_only"]["cells"][cell]
        t190_ord_ab = t187_cell["est_post_t190_ratio_ordinary_from_ab_median"]
        t190_ord_stage = t187_cell["est_post_t190_ratio_ordinary_from_stage_delta"]
        est_ab = t185_ord * (1.0 - wall_saved_pct / 100.0)
        est_stage = t185_ord * (1.0 - stage_implied_pct / 100.0)
        route_standalone[cell] = {
            "t185_est_ord": round(t185_ord, 4),
            "post_t191_standalone_est_ord_ab": round(est_ab, 4),
            "post_t191_standalone_est_ord_stage": round(est_stage, 4),
            "crosses_1x_ab": est_ab < 1.0,
        }
        stacked_ab = t190_ord_ab * (1.0 - wall_saved_pct / 100.0)
        stacked_stage = t190_ord_stage * (1.0 - stage_implied_pct / 100.0)
        route_stacked_with_t187[cell] = {
            "t185_est_ord": round(t185_ord, 4),
            "post_t190_est_ord_ab": t190_ord_ab,
            "t188_wall_pct": round(wall_saved_pct, 2),
            "t188_stage_implied_pct": round(stage_implied_pct, 2),
            "stacked_est_ord_ab": round(stacked_ab, 4),
            "stacked_est_ord_stage": round(stacked_stage, 4),
            "remaining_cut_to_1x_pct": round(max(0.0, (1.0 - 1.0 / stacked_ab) * 100.0), 2),
        }

    # 4. Ablation ladder summary
    ablation_summary = {}
    for cell in ("benchy-classic-on", "base-classic-on"):
        base_ms = screen["supports_on"][cell]["inst"]["baseline"]["sa_stage_ms"]
        full_ms = screen["supports_on"][cell]["inst"]["candidate"]["sa_stage_ms"]
        ab = screen["ablation"][cell]
        ablation_summary[cell] = {
            "baseline_sa_ms": base_ms,
            "sa1_sa_ms": ab["sa1"]["sa_stage_ms"],
            "sa1_delta_ms": base_ms - ab["sa1"]["sa_stage_ms"],
            "sa123_sa_ms": ab["sa123"]["sa_stage_ms"],
            "sa23_incremental_ms": ab["sa1"]["sa_stage_ms"] - ab["sa123"]["sa_stage_ms"],
            "sa4_alone_sa_ms": ab["sa4"]["sa_stage_ms"],
            "sa4_alone_delta_ms": base_ms - ab["sa4"]["sa_stage_ms"],
            "sa1234_sa_ms": ab["sa1234"]["sa_stage_ms"],
            "sa4_incremental_ms": ab["sa123"]["sa_stage_ms"] - ab["sa1234"]["sa_stage_ms"],
            "full_sa_ms": full_ms,
            "sa56_incremental_ms": ab["sa1234"]["sa_stage_ms"] - full_ms,
            "total_saved_ms": base_ms - full_ms,
        }

    return {
        "schema": "t188_arithmetic_v1",
        "gate_floor_pct": GATE_FLOOR_PCT,
        "reference_currency": "t182-spatial-hierarchical-phase3-union/reference.json",
        "classic_arachne_invariance": invariance,
        "attribution": attribution,
        "screen_summary": screen_summary,
        "ablation_summary": ablation_summary,
        "route_estimate_only_standalone": route_standalone,
        "route_estimate_only_stacked_t187_t188": route_stacked_with_t187,
    }


def main() -> int:
    probe_wall = load_json(REDUCED_DIR / "probe_wall.json")
    screen = load_json(REDUCED_DIR / "screen.json")
    t182_ref = load_json(T182_REF)
    t185_route = load_json(T185_ROUTE)
    t187_arith = load_json(T187_ARITH)
    out = compute_arithmetic(probe_wall, screen, t182_ref, t185_route, t187_arith)
    (REDUCED_DIR / "arithmetic.json").write_text(
        json.dumps(out, indent=2) + "\n", encoding="utf-8"
    )
    print(f"Wrote {REDUCED_DIR / 'arithmetic.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

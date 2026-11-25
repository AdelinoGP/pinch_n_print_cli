#!/usr/bin/env python3
"""Reduce raw t180 probe JSONs in target/t180-probe/ into committed reduced/probe.json.

Only needed when regenerating reduced/probe.json from fresh local probe captures;
verify.py and arithmetic.py run strictly against committed reduced JSONs.
"""

from __future__ import annotations

import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / "target" / "t180-probe"
REDUCED = HERE / "reduced"

BRACKET_META = [
    ("B0_setup", "SER", "b0_setup_ns", None),
    ("B1_pass1", "PAR", "b1_pass1_wall_ns", "pass1"),
    ("B2a_p2t_proj", "PAR", "b2a_p2t_proj_wall_ns", "p2t_proj"),
    ("B2b_p2t_merge", "SER", "b2b_p2t_merge_wall_ns", None),
    ("B2c_p2b_proj", "PAR", "b2c_p2b_proj_wall_ns", "p2b_proj"),
    ("B2d_p2b_merge", "SER", "b2d_p2b_merge_wall_ns", None),
    ("B2e_int_solid_diff", "SER", "b2e_int_solid_wall_ns", None),
    ("B3_apply_maps", "SER", "b3_apply_and_maps_wall_ns", None),
    ("B4_ext_bridge", "PAR", "b4_ext_bridge_wall_ns", "b4_ext_bridge"),
    ("B5a_ib_solids_cfg", "SER", "b5a_lower_solids_cfg_wall_ns", None),
    ("B5b_ib_phaseA", "PAR", "b5b_ib_phase_a_wall_ns", "ib_phase_a"),
    ("B5c_ib_phaseB_pre", "PAR", "b5c_ib_phase_b_precomp_wall_ns", "ib_phase_b_precomp"),
    ("B5d_ib_phaseB_com", "SER", "b5d_ib_phase_b_commit_wall_ns", None),
    ("B5e_ib_extra", "SER", "b5e_ib_extra_wall_ns", None),
    ("B6_replace", "SER", "b6_replace_wall_ns", None),
]


def r3(x: float) -> float:
    return round(x, 3)


def r2(x: float) -> float:
    return round(x, 2)


def load_raw_probe(cell: str, tag: str) -> dict:
    return json.loads((RAW / f"{cell}.{tag}.probe.json").read_text(encoding="utf-8"))


def reduce_cell(cell: str, passes: int = 3) -> dict:
    docs = [load_raw_probe(cell, f"pass{p}") for p in range(1, passes + 1)]
    pass_stage_walls_ms = [r3(d["counters"]["stage_wall_ns"] / 1e6) for d in docs]
    ordered = sorted(enumerate(docs, start=1), key=lambda kv: kv[1]["counters"]["stage_wall_ns"])
    med_pass_num, med = ordered[len(ordered) // 2]

    c = med["counters"]
    dists = med["dists"]
    stage_ms = c["stage_wall_ns"] / 1e6

    med_brackets_ms = {
        "stage_wall_ms": r3(statistics.median(d["counters"]["stage_wall_ns"] for d in docs) / 1e6)
    }
    for name, _, ckey, _ in BRACKET_META:
        med_brackets_ms[name] = r3(statistics.median(d["counters"][ckey] for d in docs) / 1e6)

    brackets = []
    sum_b_ms = 0.0
    serial_ms = 0.0
    parallel_ms = 0.0
    total_thread_ms = 0.0

    for name, mode, ckey, dkey in BRACKET_META:
        w_ms = c[ckey] / 1e6
        sum_b_ms += w_ms
        if mode == "SER":
            serial_ms += w_ms
            total_thread_ms += w_ms
            brackets.append(
                {
                    "name": name,
                    "mode": "SER",
                    "wall_ms": r3(w_ms),
                    "share_of_stage_pct": r2(100.0 * w_ms / stage_ms),
                    "thread_ms": r3(w_ms),
                    "eff_par": 1.0,
                }
            )
        else:
            dist = dists[dkey]
            th_ms = dist["sum_ns"] / 1e6
            parallel_ms += w_ms
            total_thread_ms += th_ms
            eff_par = th_ms / w_ms if w_ms > 0.0 else 0.0
            brackets.append(
                {
                    "name": name,
                    "mode": "PAR",
                    "wall_ms": r3(w_ms),
                    "share_of_stage_pct": r2(100.0 * w_ms / stage_ms),
                    "thread_ms": r3(th_ms),
                    "eff_par": r2(eff_par),
                    "count": dist["count"],
                    "p50_ms": r3(dist["p50_ns"] / 1e6),
                    "p95_ms": r3(dist["p95_ns"] / 1e6),
                    "max_ms": r3(dist["max_ns"] / 1e6),
                    "max_idx": dist["max_idx"],
                }
            )

    sites_active = [s for s in med["sites"] if s["calls"] > 0]
    sites_active.sort(key=lambda s: s["total_ns"], reverse=True)

    op_kinds_acc: dict[str, list[int]] = {}
    for s in sites_active:
        k = s["op_kind"]
        op_kinds_acc.setdefault(k, [0, 0])
        op_kinds_acc[k][0] += s["calls"]
        op_kinds_acc[k][1] += s["total_ns"]

    op_kinds = {
        k: {"calls": v[0], "thread_ms": r3(v[1] / 1e6)}
        for k, v in sorted(op_kinds_acc.items(), key=lambda kv: -kv[1][1])
    }
    clipper_total_calls = sum(v[0] for k, v in op_kinds_acc.items() if k != "cpu")
    clipper_total_thread_ms = r3(sum(v[1] for k, v in op_kinds_acc.items() if k != "cpu") / 1e6)

    sites = [
        {
            "name": s["name"],
            "op_kind": s["op_kind"],
            "calls": s["calls"],
            "total_ms": r3(s["total_ns"] / 1e6),
            "max_ms": r3(s["max_ns"] / 1e6),
            "in_polys": s["in_polys"],
            "in_verts": s["in_verts"],
            "out_verts": s["out_verts"],
            "empty_out": s["empty_out"],
        }
        for s in sites_active
    ]

    locality = {
        "pass1": {
            "top_identical": c["p1_top_identical"],
            "bot_identical": c["p1_bot_identical"],
            "top_empty_clip": c["p1_top_empty_clip"],
            "bot_empty_clip": c["p1_bot_empty_clip"],
            "top_diff_slivers": c["p1_top_diff_slivers"],
            "top_diff_polys": c["p1_top_diff_polys"],
            "bot_diff_slivers": c["p1_bot_diff_slivers"],
            "bot_diff_polys": c["p1_bot_diff_polys"],
            "sum_layer_polys": c["p1_sum_layer_polys"],
            "max_layer_polys": c["p1_max_layer_polys"],
        },
        "pass2_top": {
            "seeds": c["p2t_seeds"],
            "steps": c["p2t_steps"],
            "break_inter_empty": c["p2t_break_inter_empty"],
            "break_off_empty": c["p2t_break_off_empty"],
            "nbr_polys_ovl": c["p2t_nbr_polys_ovl"],
            "nbr_polys_tot": c["p2t_nbr_polys_tot"],
            "nbr_verts_in_bbox": c["p2t_nbr_verts_in_bbox"],
            "nbr_verts_tot": c["p2t_nbr_verts_tot"],
            "inter_slivers": c["p2t_inter_slivers"],
            "inter_polys_tot": c["p2t_inter_polys_tot"],
            "merge_contribs": c["p2t_merge_contribs"],
            "merge_unique_targets": c["p2t_merge_unique_targets"],
            "merge_empty_existing": c["p2t_merge_empty_existing"],
            "merge_max_per_target": c["p2t_merge_max_per_target"],
        },
        "pass2_bot": {
            "seeds": c["p2b_seeds"],
            "steps": c["p2b_steps"],
            "break_inter_empty": c["p2b_break_inter_empty"],
            "break_off_empty": c["p2b_break_off_empty"],
            "nbr_polys_ovl": c["p2b_nbr_polys_ovl"],
            "nbr_polys_tot": c["p2b_nbr_polys_tot"],
            "nbr_verts_in_bbox": c["p2b_nbr_verts_in_bbox"],
            "nbr_verts_tot": c["p2b_nbr_verts_tot"],
            "inter_slivers": c["p2b_inter_slivers"],
            "inter_polys_tot": c["p2b_inter_polys_tot"],
            "merge_contribs": c["p2b_merge_contribs"],
            "merge_unique_targets": c["p2b_merge_unique_targets"],
            "merge_empty_existing": c["p2b_merge_empty_existing"],
            "merge_max_per_target": c["p2b_merge_max_per_target"],
        },
        "pass2e_int_solid": {
            "total": c["p2e_total"],
            "seed_empty": c["p2e_seed_empty"],
            "no_p2_contrib": c["p2e_no_p2_contrib"],
            "both_nonempty": c["p2e_both_nonempty"],
            "nonempty_out": c["p2e_nonempty_out"],
        },
        "b4_ext_bridge": {
            "nonempty_layers": c["b4_nonempty_layers"],
        },
        "ib_phase_a": {
            "total": c["ib_a_total"],
            "no_lower": c["ib_a_no_lower"],
            "empty_is": c["ib_a_empty_is"],
            "ran_unsup": c["ib_a_ran_unsup"],
            "unsup_empty": c["ib_a_unsup_empty"],
            "survived": c["ib_a_survived"],
            "lf_verts_in_box": c["ib_a_lf_verts_in_box"],
            "lf_verts_tot": c["ib_a_lf_verts_tot"],
            "ls_verts_in_box": c["ib_a_ls_verts_in_box"],
            "ls_verts_tot": c["ib_a_ls_verts_tot"],
        },
        "ib_phase_b_precomp": {
            "total": c["ib_b_precomp_total"],
            "some": c["ib_b_precomp_some"],
            "none": c["ib_b_precomp_none"],
            "wasted_none_thread_ms": r3(c["ib_b_wasted_none_thread_ns"] / 1e6),
            "wasted_none_gather_ms": r3(c["ib_b_wasted_none_gather_ns"] / 1e6),
            "wasted_none_pos_off_ms": r3(c["ib_b_wasted_none_pos_off_ns"] / 1e6),
            "kept_some_gather_ms": r3(c["ib_b_kept_some_gather_ns"] / 1e6),
            "kept_some_pos_off_ms": r3(c["ib_b_kept_some_pos_off_ns"] / 1e6),
            "gather_layers": med["gather_layers"],
            "surf_total": c["ib_b_surf_total"],
            "surf_deep_empty": c["ib_b_surf_deep_empty"],
            "surf_qual_some": c["ib_b_surf_qual_some"],
            "surf_qual_none": c["ib_b_surf_qual_none"],
            "qual_layers": c["ib_b_qual_layers"],
            "qual_cands_total": c["ib_b_qual_cands_total"],
            "qual_cands_max": c["ib_b_qual_cands_max"],
        },
        "ib_phase_b_commit": {
            "published": c["ib_b_commit_published"],
            "recomputes": c["ib_b_commit_recomputes"],
            "recomp_ovl_is": c["ib_b_commit_recomp_ovl_is"],
            "recomp_wall_ms": r3(c["ib_b_commit_recomp_wall_ns"] / 1e6),
            "recomp_gather_ms": r3(c["ib_b_commit_recomp_gather_ns"] / 1e6),
            "recomp_remove_ms": r3(c["ib_b_commit_recomp_remove_ns"] / 1e6),
            "recomp_compute_ms": r3(c["ib_b_commit_recomp_compute_ns"] / 1e6),
            "recomp_layers": [
                {
                    "layer": x["layer"],
                    "ms": r3(x["ns"] / 1e6),
                    "ovl_is": x["ovl_is"],
                }
                for x in med["ib_recomp_layers"]
            ],
        },
    }

    return {
        "cell": cell,
        "pass_stage_walls_ms": pass_stage_walls_ms,
        "median_pass_number": med_pass_num,
        "median_brackets_ms": med_brackets_ms,
        "median_pass": {
            "stage_wall_ms": r3(stage_ms),
            "sum_brackets_ms": r3(sum_b_ms),
            "bracket_gap_ms": r3(stage_ms - sum_b_ms),
            "serial_ms": r3(serial_ms),
            "parallel_ms": r3(parallel_ms),
            "serial_pct": r2(100.0 * serial_ms / stage_ms),
            "parallel_pct": r2(100.0 * parallel_ms / stage_ms),
            "total_thread_ms": r3(total_thread_ms),
            "eff_par": r2(total_thread_ms / stage_ms),
            "params": {
                "timelines_count": c["timelines_count"],
                "timeline_len": c["timeline_len"],
                "k_top": c["k_top"],
                "k_bot": c["k_bot"],
                "opening_radius_um": c["opening_radius_um"],
                "shell_width_um": c["shell_width_um"],
                "extra_bridge_enabled": c["extra_bridge_enabled"],
                "nofilter_enabled": c["nofilter_enabled"],
            },
            "brackets": brackets,
            "op_kinds": op_kinds,
            "clipper_total_calls": clipper_total_calls,
            "clipper_total_thread_ms": clipper_total_thread_ms,
            "sites": sites,
            "locality": locality,
        },
    }


def main() -> int:
    probe_runs = json.loads((RAW / "probe_runs.json").read_text(encoding="utf-8"))
    runs = probe_runs["runs"]

    # Exact call + vertex invariance checks across classic/arachne and off/on
    inv_pairs = [
        ("benchy_classic_off_vs_on", "benchy-classic-off", "pass1", "benchy-classic-on", "pass1"),
        ("benchy_classic_vs_arachne", "benchy-classic-off", "pass1", "benchy-arachne-off", "inv1"),
        ("base_classic_off_vs_on", "base-classic-off", "pass1", "base-classic-on", "inv1"),
        ("base_classic_vs_arachne", "base-classic-off", "pass1", "base-arachne-off", "inv1"),
    ]
    invariance = {}
    for key, c1, t1, c2, t2 in inv_pairs:
        d1 = load_raw_probe(c1, t1)
        d2 = load_raw_probe(c2, t2)
        s1 = {s["name"]: (s["calls"], s["in_verts"], s["out_verts"]) for s in d1["sites"]}
        s2 = {s["name"]: (s["calls"], s["in_verts"], s["out_verts"]) for s in d2["sites"]}
        diffs = [k for k in s1 if s1[k] != s2.get(k)]
        invariance[key] = {
            "pair": [f"{c1}:{t1}", f"{c2}:{t2}"],
            "identical_calls_and_verts": len(diffs) == 0,
            "diff_sites": diffs,
        }

    qual_runs = [r for r in runs if r["instrumented"]]
    qual_clean_ok = sum(
        1
        for r in qual_runs
        if r["exit_code"] == 0
        and r["reference_match"]
        and r["instrument_events"] is not None
        and r["instrument_events"]["degraded"] is False
        and r["instrument_events"]["fatal_error_count"] == 0
        and r["instrument_events"]["non_fatal_error_count"] == 0
        and r["instrument_events"]["code_1200_count"] == 0
    )

    out = {
        "schema": 1,
        "note": (
            "Reduced attribution from observation-only probe (probe.patch, cleanly reverted). "
            "All 15 runs match t175/reference.json byte-for-byte. Stage and bracket times are "
            "attribution-only; no wall claim is made for probed runs."
        ),
        "capture_meta": {
            "captured_utc": probe_runs["captured_utc"],
            "exe_sha256": probe_runs["exe_sha256"],
            "rayon_num_threads": probe_runs["rayon_num_threads"],
            "total_runs": len(runs),
            "exit_zero_ok": sum(1 for r in runs if r["exit_code"] == 0),
            "ref_match_ok": sum(1 for r in runs if r["reference_match"]),
            "qual_runs": len(qual_runs),
            "qual_clean_ok": qual_clean_ok,
        },
        "runs": runs,
        "invariance": invariance,
        "cells": {
            cell: reduce_cell(cell, passes=3)
            for cell in ["benchy-classic-off", "benchy-classic-on", "base-classic-off"]
        },
    }

    REDUCED.mkdir(parents=True, exist_ok=True)
    out_path = REDUCED / "probe.json"
    out_path.write_text(json.dumps(out, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {out_path} ({out_path.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

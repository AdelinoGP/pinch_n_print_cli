#!/usr/bin/env python3
"""Analyze t180 probe JSON outputs into a readable summary + reduced/probe.json."""

from __future__ import annotations

import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / "target" / "t180-probe"


def load_probe(cell: str, tag: str) -> dict:
    return json.loads((RAW / f"{cell}.{tag}.probe.json").read_text(encoding="utf-8"))


def median_probes(cell: str, passes: int = 3) -> dict:
    docs = [load_probe(cell, f"pass{p}") for p in range(1, passes + 1)]
    # Pick the pass with median stage_wall_ns for exact internal consistency,
    # and also report all-pass median for every bracket!
    ordered = sorted(docs, key=lambda d: d["counters"]["stage_wall_ns"])
    med = ordered[len(ordered) // 2]
    brackets = [
        "stage_wall_ns",
        "b0_setup_ns",
        "b1_pass1_wall_ns",
        "b2a_p2t_proj_wall_ns",
        "b2b_p2t_merge_wall_ns",
        "b2c_p2b_proj_wall_ns",
        "b2d_p2b_merge_wall_ns",
        "b2e_int_solid_wall_ns",
        "b3_apply_and_maps_wall_ns",
        "b3a_apply_edits_ns",
        "b3b_lower_maps_ns",
        "b4_ext_bridge_wall_ns",
        "b5a_lower_solids_cfg_wall_ns",
        "b5b_ib_phase_a_wall_ns",
        "b5c_ib_phase_b_precomp_wall_ns",
        "b5d_ib_phase_b_commit_wall_ns",
        "b5e_ib_extra_wall_ns",
        "b6_replace_wall_ns",
    ]
    med_brackets = {
        k: round(statistics.median(d["counters"][k] for d in docs) / 1e6, 3)
        for k in brackets
    }
    pass_walls = [round(d["counters"]["stage_wall_ns"] / 1e6, 3) for d in docs]
    return {
        "cell": cell,
        "pass_stage_walls_ms": pass_walls,
        "median_brackets_ms": med_brackets,
        "median_pass": med,
    }


def print_report(summary: dict) -> None:
    cell = summary["cell"]
    med = summary["median_pass"]
    c = med["counters"]
    d = med["dists"]
    stage_ms = c["stage_wall_ns"] / 1e6
    print(f"\n==================== {cell} (stage={stage_ms:.2f} ms, passes={summary['pass_stage_walls_ms']}) ====================")
    print(
        f"Params: timelines={c['timelines_count']} len={c['timeline_len']} "
        f"k_top={c['k_top']} k_bot={c['k_bot']} open_r={c['opening_radius_um']}um "
        f"shell_w={c['shell_width_um']}um extra_br={c['extra_bridge_enabled']} nofilter={c['nofilter_enabled']}"
    )

    rows = [
        ("B0_setup (SER)", c["b0_setup_ns"], None),
        ("B1_pass1 (PAR)", c["b1_pass1_wall_ns"], d["pass1"]),
        ("B2a_p2t_proj (PAR)", c["b2a_p2t_proj_wall_ns"], d["p2t_proj"]),
        ("B2b_p2t_merge (SER)", c["b2b_p2t_merge_wall_ns"], None),
        ("B2c_p2b_proj (PAR)", c["b2c_p2b_proj_wall_ns"], d["p2b_proj"]),
        ("B2d_p2b_merge (SER)", c["b2d_p2b_merge_wall_ns"], None),
        ("B2e_int_solid_diff (SER)", c["b2e_int_solid_wall_ns"], None),
        ("B3_apply+maps (SER)", c["b3_apply_and_maps_wall_ns"], None),
        ("B4_ext_bridge (PAR)", c["b4_ext_bridge_wall_ns"], d["b4_ext_bridge"]),
        ("B5a_ib_solids+cfg (SER)", c["b5a_lower_solids_cfg_wall_ns"], None),
        ("B5b_ib_phaseA (PAR)", c["b5b_ib_phase_a_wall_ns"], d["ib_phase_a"]),
        ("B5c_ib_phaseB_pre (PAR)", c["b5c_ib_phase_b_precomp_wall_ns"], d["ib_phase_b_precomp"]),
        ("B5d_ib_phaseB_com (SER)", c["b5d_ib_phase_b_commit_wall_ns"], None),
        ("B5e_ib_extra (SER)", c["b5e_ib_extra_wall_ns"], None),
        ("B6_replace (SER)", c["b6_replace_wall_ns"], None),
    ]
    sum_b = 0.0
    ser_ms = 0.0
    par_ms = 0.0
    tot_thread_ms = 0.0
    print(f"{'Bracket':28s} {'Wall ms':>9s} {'%stage':>7s} {'Thread ms':>10s} {'Par':>6s} {'p50':>7s} {'p95':>7s} {'max (idx)':>14s}")
    for name, w_ns, dist in rows:
        w = w_ns / 1e6
        sum_b += w
        if "(SER)" in name:
            ser_ms += w
            tot_thread_ms += w
            print(f"{name:28s} {w:9.2f} {100*w/stage_ms:6.2f}% {w:10.2f} {'1.00x':>6s}")
        else:
            par_ms += w
            th = dist["sum_ns"] / 1e6
            tot_thread_ms += th
            ep = th / w if w > 0 else 0.0
            print(
                f"{name:28s} {w:9.2f} {100*w/stage_ms:6.2f}% {th:10.2f} {ep:5.2f}x "
                f"{dist['p50_ns']/1e6:7.2f} {dist['p95_ns']/1e6:7.2f} "
                f"{dist['max_ns']/1e6:7.2f} (L{dist['max_idx']})"
            )
    print(
        f"SUM brackets = {sum_b:.2f} ms (vs stage {stage_ms:.2f} ms, diff={stage_ms-sum_b:+.2f} ms) | "
        f"SERIAL={ser_ms:.2f} ms ({100*ser_ms/stage_ms:.1f}%) PAR={par_ms:.2f} ms ({100*par_ms/stage_ms:.1f}%) | "
        f"Total thread={tot_thread_ms:.1f} ms ({tot_thread_ms/stage_ms:.2f}x)"
    )

    # Site breakdown sorted by total_ns desc
    sites = [s for s in med["sites"] if s["calls"] > 0]
    sites.sort(key=lambda s: s["total_ns"], reverse=True)
    by_kind = {}
    for s in sites:
        k = s["op_kind"]
        by_kind.setdefault(k, [0, 0])
        by_kind[k][0] += s["calls"]
        by_kind[k][1] += s["total_ns"]
    print("\nOp-kind reconciliation:")
    for k, (cnt, ns) in sorted(by_kind.items(), key=lambda kv: -kv[1][1]):
        print(f"  {k:12s}: {cnt:5d} calls, {ns/1e6:9.2f} ms")
    clipper_calls = sum(cnt for k, (cnt, _) in by_kind.items() if k != "cpu")
    clipper_ms = sum(ns for k, (_, ns) in by_kind.items() if k != "cpu") / 1e6
    print(f"  TOTAL Clipper calls = {clipper_calls}, thread = {clipper_ms:.2f} ms")

    print(f"\nTop call sites (of {len(sites)} active):")
    print(f"{'Site':26s} {'Kind':10s} {'Calls':>6s} {'Tot ms':>9s} {'%clip':>6s} {'Max ms':>8s} {'InP':>7s} {'InV':>10s} {'OutV':>10s} {'EmpOut':>6s}")
    for s in sites:
        ms = s["total_ns"] / 1e6
        pct = 100.0 * ms / clipper_ms if s["op_kind"] != "cpu" and clipper_ms > 0 else 0.0
        print(
            f"{s['name']:26s} {s['op_kind']:10s} {s['calls']:6d} {ms:9.2f} {pct:5.1f}% "
            f"{s['max_ns']/1e6:8.2f} {s['in_polys']:7d} {s['in_verts']:10d} {s['out_verts']:10d} {s['empty_out']:6d}"
        )

    # Structural & locality highlights
    print("\nStructural & locality counters:")
    print(
        f"  Pass1: top_ident={c['p1_top_identical']} bot_ident={c['p1_bot_identical']} "
        f"top_emp_clip={c['p1_top_empty_clip']} bot_emp_clip={c['p1_bot_empty_clip']} "
        f"top_slivers={c['p1_top_diff_slivers']}/{c['p1_top_diff_polys']} "
        f"bot_slivers={c['p1_bot_diff_slivers']}/{c['p1_bot_diff_polys']} "
        f"avg_polys/layer={c['p1_sum_layer_polys']/max(1,c['timeline_len']):.2f} max={c['p1_max_layer_polys']}"
    )
    for label, p in [("Pass2-Top", "p2t"), ("Pass2-Bot", "p2b")]:
        v_tot = max(1, c[f"{p}_nbr_verts_tot"])
        p_tot = max(1, c[f"{p}_nbr_polys_tot"])
        print(
            f"  {label}: seeds={c[f'{p}_seeds']} steps={c[f'{p}_steps']} "
            f"br_inter={c[f'{p}_break_inter_empty']} br_off={c[f'{p}_break_off_empty']} | "
            f"nbr_polys_ovl={c[f'{p}_nbr_polys_ovl']}/{c[f'{p}_nbr_polys_tot']} ({100*c[f'{p}_nbr_polys_ovl']/p_tot:.1f}%) "
            f"nbr_verts_in_bbox={c[f'{p}_nbr_verts_in_bbox']}/{c[f'{p}_nbr_verts_tot']} ({100*c[f'{p}_nbr_verts_in_bbox']/v_tot:.1f}%) | "
            f"inter_slivers={c[f'{p}_inter_slivers']}/{c[f'{p}_inter_polys_tot']} | "
            f"merge contribs={c[f'{p}_merge_contribs']} targets={c[f'{p}_merge_unique_targets']} "
            f"empty_exist={c[f'{p}_merge_empty_existing']} max/target={c[f'{p}_merge_max_per_target']}"
        )
    print(
        f"  Pass2e int_solid: total={c['p2e_total']} seed_empty={c['p2e_seed_empty']} "
        f"no_p2_contrib={c['p2e_no_p2_contrib']} both_nonempty={c['p2e_both_nonempty']} nonempty_out={c['p2e_nonempty_out']}"
    )
    print(f"  B4 ext_bridge: nonempty_layers={c['b4_nonempty_layers']}")
    lf_tot = max(1, c["ib_a_lf_verts_tot"])
    ls_tot = max(1, c["ib_a_ls_verts_tot"])
    print(
        f"  IB PhaseA: total={c['ib_a_total']} no_lower={c['ib_a_no_lower']} empty_is={c['ib_a_empty_is']} "
        f"ran_unsup={c['ib_a_ran_unsup']} unsup_empty={c['ib_a_unsup_empty']} survived={c['ib_a_survived']} | "
        f"lf_verts_in_box={c['ib_a_lf_verts_in_box']}/{c['ib_a_lf_verts_tot']} ({100*c['ib_a_lf_verts_in_box']/lf_tot:.1f}%) "
        f"ls_verts_in_box={c['ib_a_ls_verts_in_box']}/{c['ib_a_ls_verts_tot']} ({100*c['ib_a_ls_verts_in_box']/ls_tot:.1f}%)"
    )
    print(
        f"  IB PhaseB precomp: total={c['ib_b_precomp_total']} some={c['ib_b_precomp_some']} none={c['ib_b_precomp_none']} | "
        f"wasted_none_thread={c['ib_b_wasted_none_thread_ns']/1e6:.1f}ms "
        f"(gather={c['ib_b_wasted_none_gather_ns']/1e6:.1f}ms pos_off={c['ib_b_wasted_none_pos_off_ns']/1e6:.1f}ms) | "
        f"kept_some gather={c['ib_b_kept_some_gather_ns']/1e6:.1f}ms pos_off={c['ib_b_kept_some_pos_off_ns']/1e6:.1f}ms | "
        f"gather_layers={med['gather_layers']}"
    )
    print(
        f"  IB PhaseB surfs: total={c['ib_b_surf_total']} deep_empty={c['ib_b_surf_deep_empty']} "
        f"qual_some={c['ib_b_surf_qual_some']} qual_none={c['ib_b_surf_qual_none']} | "
        f"qual_layers={c['ib_b_qual_layers']} cands_tot={c['ib_b_qual_cands_total']} max_cands={c['ib_b_qual_cands_max']}"
    )
    print(
        f"  IB PhaseB commit: published={c['ib_b_commit_published']} recomputes={c['ib_b_commit_recomputes']} "
        f"(ovl_is={c['ib_b_commit_recomp_ovl_is']}) recomp_wall={c['ib_b_commit_recomp_wall_ns']/1e6:.1f}ms "
        f"(gather={c['ib_b_commit_recomp_gather_ns']/1e6:.1f}ms remove={c['ib_b_commit_recomp_remove_ns']/1e6:.1f}ms "
        f"compute={c['ib_b_commit_recomp_compute_ns']/1e6:.1f}ms)"
    )
    if med["ib_recomp_layers"]:
        print(f"  IB Recomp layers ({len(med['ib_recomp_layers'])}): " + ", ".join(
            f"L{x['layer']}={x['ns']/1e6:.1f}ms(ovl={x['ovl_is']})" for x in med["ib_recomp_layers"]
        ))


def main() -> int:
    for cell in ["benchy-classic-off", "benchy-classic-on", "base-classic-off"]:
        if (RAW / f"{cell}.pass1.probe.json").exists():
            print_report(median_probes(cell, passes=3))
    # Check invariance
    print("\n=== Invariance check (exact call counts & vertex totals) ===")
    pairs = [
        ("benchy-classic-off", "pass1", "benchy-classic-on", "pass1"),
        ("benchy-classic-off", "pass1", "benchy-arachne-off", "inv1"),
        ("base-classic-off", "pass1", "base-classic-on", "inv1"),
        ("base-classic-off", "pass1", "base-arachne-off", "inv1"),
    ]
    for c1, t1, c2, t2 in pairs:
        p1 = RAW / f"{c1}.{t1}.probe.json"
        p2 = RAW / f"{c2}.{t2}.probe.json"
        if p1.exists() and p2.exists():
            d1, d2 = json.loads(p1.read_text()), json.loads(p2.read_text())
            s1 = {s["name"]: (s["calls"], s["in_verts"], s["out_verts"]) for s in d1["sites"]}
            s2 = {s["name"]: (s["calls"], s["in_verts"], s["out_verts"]) for s in d2["sites"]}
            diffs = [k for k in s1 if s1[k] != s2.get(k)]
            print(f"  {c1} vs {c2}: {'IDENTICAL' if not diffs else f'DIFFS: {diffs}'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

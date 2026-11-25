#!/usr/bin/env python3
"""t186 -- post-t185 full-matrix attribution and 8/8 + 0.50x headroom arithmetic.

Pure checkout-only derivation from committed artifacts:
  - `reduced/instrumented.json` (this take, 3 passes/cell)
  - `reduced/profile-clean.json` (this take, 1 pass/cell)
  - `reduced/profile-detail-clean.json` (this take, 1 pass/cell)
  - `../t174-post-t172-scoreboard/reduced/summary.json` (official quiet board)
  - `../t185-support-geometry-host-tail-accel/reduced/route.json` (post-t185 ESTIMATE ONLY route)
  - `../t177-post-t175-attribution/reduced/arithmetic.json` (post-t175 comparison)

Regimes are never mixed:
  - Phase and sequential prepass/postpass stage shares are instrumented
    attribution only (`--instrument-stderr` inflates wall; never a wall claim).
  - Per-layer module shares are thread-accumulated (`module_thread_sum_ms`)
    converted to proportional implied wall ceilings via the per-layer phase's
    `module_busy_union_ms` under a uniform-concurrency assumption (disclosed).
  - `--profile` host built-in times are accumulated thread-time and guest fuel
    is deterministic executed WASM instructions; neither is denominated as wall.
  - Cross-window post-t185 ratios (`t174 x t175 x t182 x t183 x t184 x t185`)
    and all `ratio_if_zero` ceilings are explicitly `ESTIMATE ONLY`, never
    three-tool scoreboard ratios of record.
"""

from __future__ import annotations

import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
EVID = HERE.parent

CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
    "base-classic-off",
    "base-classic-on",
    "base-arachne-off",
    "base-arachne-on",
]
SUPPORTS_ON = {c for c in CELLS if c.endswith("-on")}
PROVISIONALLY_WON_OFFICIAL = {"base-classic-off", "base-arachne-off"}
GATE_FLOOR = 0.008


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def median_int(values: list[int]) -> int:
    if not values:
        return 0
    return int(round(statistics.median(values)))


def median_stage_map(passes: list[dict], key: str) -> dict[str, int]:
    names = sorted({k for p in passes for k in p.get(key, {})})
    out = {
        name: median_int([p.get(key, {}).get(name, 0) for p in passes])
        for name in names
    }
    return dict(sorted(out.items(), key=lambda kv: (-kv[1], kv[0])))


def median_module_phase(passes: list[dict], phase: str) -> dict[str, dict]:
    names = sorted(
        {
            mid
            for p in passes
            for mid in p.get("module_call_stats", {}).get(phase, {})
        }
    )
    out = {}
    for mid in names:
        recs = [
            p["module_call_stats"][phase][mid]
            for p in passes
            if mid in p.get("module_call_stats", {}).get(phase, {})
        ]
        out[mid] = {
            "calls": median_int([r["calls"] for r in recs]),
            "thread_sum_ms": median_int([r["thread_sum_ms"] for r in recs]),
            "spans_ms": median_int([r["spans_ms"] for r in recs]),
            "busy_union_ms": median_int([r["busy_union_ms"] for r in recs]),
            "max_ms": median_int([r["max_ms"] for r in recs]),
            "p50_ms": median_int([r["p50_ms"] for r in recs]),
            "p90_ms": median_int([r["p90_ms"] for r in recs]),
            "p99_ms": median_int([r["p99_ms"] for r in recs]),
        }
    return dict(sorted(out.items(), key=lambda kv: (-kv[1]["thread_sum_ms"], kv[0])))


def split_section(instrumented: dict, t177: dict) -> dict:
    """Per-cell phase, stage, and per-layer module split (median of 3 passes)."""
    out = {}
    for cell in CELLS:
        rec = instrumented["cells"][cell]
        passes = rec["passes"]
        median = rec["median"]
        total = median["slice_complete_ms"]
        shares = {k: round(v, 6) for k, v in median["shares"].items()}

        prepass_stages = median_stage_map(passes, "prepass_stages_ms")
        postpass_stages = median_stage_map(passes, "postpass_stages_ms")
        per_layer_stage_thread = median_stage_map(
            passes, "per_layer_stages_thread_sum_ms"
        )

        prepass_ms = median_int([p["phases_ms"].get("prepass", 0) for p in passes])
        per_layer_ms = median_int([p["phases_ms"].get("per_layer", 0) for p in passes])
        postpass_ms = median_int([p["phases_ms"].get("postpass", 0) for p in passes])
        outside_ms = median_int([p.get("outside_phases_ms", 0) for p in passes])

        pl_busy = median_int(
            [p["phase_detail"]["per_layer"]["module_busy_union_ms"] for p in passes]
        )
        pl_gap = median_int(
            [p["phase_detail"]["per_layer"]["host_gap_ms"] for p in passes]
        )
        sg_post_tail = median_int(
            [p.get("support_geometry_post_planner_tail_ms", 0) for p in passes]
        )
        tail_sorted = sorted(
            [p.get("per_layer_tail", {}) for p in passes],
            key=lambda t: t.get("single_layer_straggler_tail_ms", 0),
        )
        tail_med = tail_sorted[len(tail_sorted) // 2] if tail_sorted else {}
        straggler_ms = tail_med.get("single_layer_straggler_tail_ms", 0)

        pl_mods = median_module_phase(passes, "per_layer")
        pl_thread_total = sum(m["thread_sum_ms"] for m in pl_mods.values())
        pl_eff_par = round(pl_thread_total / pl_busy, 3) if pl_busy else 0.0
        pl_busy_share_of_cell = pl_busy / total if total else 0.0

        pl_modules_enriched = {}
        for mid, m in pl_mods.items():
            th_share = m["thread_sum_ms"] / pl_thread_total if pl_thread_total else 0.0
            implied_cell_share = pl_busy_share_of_cell * th_share
            pl_modules_enriched[mid] = {
                **m,
                "share_of_per_layer_thread": round(th_share, 6),
                "implied_wall_share_of_cell": round(implied_cell_share, 6),
                "implied_wall_ms_instrumented": round(implied_cell_share * total, 1),
            }

        t177_sp = t177["split"][cell]
        t177_shares = t177_sp["phase_shares"]
        delta_vs_t177_pp = {
            k: round(100.0 * (shares.get(k, 0.0) - t177_shares.get(k, 0.0)), 2)
            for k in sorted(set(shares) | set(t177_shares))
        }
        t177_total = t177_sp["slice_complete_ms"]

        out[cell] = {
            "slice_complete_ms": total,
            "t177_slice_complete_ms": t177_total,
            "slice_complete_delta_vs_t177_pct": (
                round(100.0 * (total - t177_total) / t177_total, 2)
                if t177_total
                else None
            ),
            "layer_count": median["layer_count"],
            "phase_shares": shares,
            "delta_phase_shares_vs_t177_pp": delta_vs_t177_pp,
            "prepass_ms": prepass_ms,
            "per_layer_ms": per_layer_ms,
            "postpass_ms": postpass_ms,
            "outside_ms": outside_ms,
            "prepass_stage_ms": prepass_stages,
            "prepass_stage_sum_ms": sum(prepass_stages.values()),
            "prepass_stage_shares_of_cell": {
                k: round(v / total, 6) for k, v in prepass_stages.items()
            },
            "prepass_stage_shares_of_prepass": {
                k: round(v / prepass_ms, 6) if prepass_ms else 0.0
                for k, v in prepass_stages.items()
            },
            "support_geometry_post_planner_tail_ms": sg_post_tail,
            "support_geometry_post_planner_tail_share_of_cell": (
                round(sg_post_tail / total, 6) if total else 0.0
            ),
            "postpass_stage_ms": postpass_stages,
            "postpass_stage_sum_ms": sum(postpass_stages.values()),
            "postpass_stage_shares_of_cell": {
                k: round(v / total, 6) for k, v in postpass_stages.items()
            },
            "per_layer_detail": {
                "wall_ms": per_layer_ms,
                "module_busy_union_ms": pl_busy,
                "host_gap_ms": pl_gap,
                "host_gap_share_of_cell": round(pl_gap / total, 6) if total else 0.0,
                "module_busy_share_of_cell": round(pl_busy_share_of_cell, 6),
                "module_thread_sum_total_ms": pl_thread_total,
                "effective_parallelism": pl_eff_par,
                "straggler_tail": {
                    **tail_med,
                    "share_of_per_layer": (
                        round(straggler_ms / per_layer_ms, 6) if per_layer_ms else 0.0
                    ),
                    "share_of_cell": round(straggler_ms / total, 6) if total else 0.0,
                },
                "stages_thread_sum_ms": per_layer_stage_thread,
                "modules": pl_modules_enriched,
            },
            "prepass_modules": median_module_phase(passes, "prepass"),
            "postpass_modules": median_module_phase(passes, "postpass"),
            "repeat_max_share_delta_pp": rec["repeat_max_share_delta_pp"],
            "gcode_sha256": median["gcode_sha256"],
        }
    return out


def route_section(t174: dict, t185_route: dict) -> dict:
    """ESTIMATE ONLY post-t185 route table from t174 quiet board + t185 route chain."""
    out = {}
    for cell in CELLS:
        c174 = t174["cells"][cell]
        orca_s = c174["orca"]["wall_median_s"]
        r185 = t185_route["cells"][cell]
        est_ratio_ord = r185["est_post_t185_ratio_ordinary"]
        est_ratio_acc = r185["est_post_t185_ratio_accelerated"]
        is_on = cell in SUPPORTS_ON

        out[cell] = {
            "supports": "on" if is_on else "off",
            "provisionally_won_official": cell in PROVISIONALLY_WON_OFFICIAL,
            "est_under_1x_ord": est_ratio_ord < 1.0,
            "est_under_1x_acc": est_ratio_acc < 1.0,
            "t174_orca_s": round(orca_s, 4),
            "t174_ratio_ord": round(c174["ratio_ordinary_over_orca"], 4),
            "t174_cpu_ratio_ord": round(c174["cpu_ratio_ordinary_over_orca"], 4),
            "est_post_t185_ratio_ord": round(est_ratio_ord, 4),
            "est_post_t185_ratio_acc": round(est_ratio_acc, 4),
            "est_post_t185_pnp_ord_s_on_t174_scale": round(orca_s * est_ratio_ord, 4),
            "est_post_t185_pnp_acc_s_on_t174_scale": round(orca_s * est_ratio_acc, 4),
            "required_cut_1x_ord": round(1.0 - 1.0 / est_ratio_ord, 6),
            "required_cut_1x_acc": round(1.0 - 1.0 / est_ratio_acc, 6),
            "required_cut_05x_ord": round(1.0 - 0.5 / est_ratio_ord, 6),
            "required_cut_05x_acc": round(1.0 - 0.5 / est_ratio_acc, 6),
        }
    return out


def zero_entry(
    share: float,
    ratio_ord: float,
    ratio_acc: float,
    cut_1x_ord: float,
    cut_05x_ord: float,
    est_pnp_ord_s: float,
    *,
    basis: str,
) -> dict:
    r1_ord = ratio_ord * (1.0 - share)
    r1_acc = ratio_acc * (1.0 - share)
    return {
        "basis": basis,
        "share_of_cell": round(share, 6),
        "gate_floor_multiple": round(share / GATE_FLOOR, 2),
        "est_wall_s_on_t174_scale": round(share * est_pnp_ord_s, 4),
        "ratio_if_zero_ord": round(r1_ord, 4),
        "ratio_if_zero_acc": round(r1_acc, 4),
        "wins_1x_ord": r1_ord < 1.0,
        "wins_1x_acc": r1_acc < 1.0,
        "hits_05x_ord": r1_ord <= 0.50,
        "hits_05x_acc": r1_acc <= 0.50,
        "frac_of_cut_1x_ord": (
            round(share / cut_1x_ord, 4) if cut_1x_ord > 0.0 else None
        ),
        "frac_of_cut_05x_ord": (
            round(share / cut_05x_ord, 4) if cut_05x_ord > 0.0 else None
        ),
    }


def stage_budget_section(split: dict, route: dict) -> dict:
    """Single-phase, single-stage, and proportional single-module zeroing ceilings."""
    out = {}
    for cell in CELLS:
        sp = split[cell]
        rt = route[cell]
        r_ord = rt["est_post_t185_ratio_ord"]
        r_acc = rt["est_post_t185_ratio_acc"]
        c1 = rt["required_cut_1x_ord"]
        c05 = rt["required_cut_05x_ord"]
        est_s = rt["est_post_t185_pnp_ord_s_on_t174_scale"]

        phases = {
            k: zero_entry(v, r_ord, r_acc, c1, c05, est_s, basis="instrumented_phase_bracket")
            for k, v in sp["phase_shares"].items()
        }
        prepass_stages = {
            k: {
                "ms": sp["prepass_stage_ms"][k],
                "share_of_prepass": sp["prepass_stage_shares_of_prepass"][k],
                **zero_entry(
                    v, r_ord, r_acc, c1, c05, est_s, basis="instrumented_sequential_stage"
                ),
            }
            for k, v in sp["prepass_stage_shares_of_cell"].items()
        }
        postpass_stages = {
            k: {
                "ms": sp["postpass_stage_ms"][k],
                **zero_entry(
                    v, r_ord, r_acc, c1, c05, est_s, basis="instrumented_sequential_stage"
                ),
            }
            for k, v in sp["postpass_stage_shares_of_cell"].items()
        }
        per_layer_modules = {
            mid: {
                "thread_sum_ms": m["thread_sum_ms"],
                "share_of_per_layer_thread": m["share_of_per_layer_thread"],
                **zero_entry(
                    m["implied_wall_share_of_cell"],
                    r_ord,
                    r_acc,
                    c1,
                    c05,
                    est_s,
                    basis="proportional_per_layer_thread_ceiling",
                ),
            }
            for mid, m in sp["per_layer_detail"]["modules"].items()
        }

        out[cell] = {
            "est_post_t185_ratio_ord": r_ord,
            "est_post_t185_ratio_acc": r_acc,
            "required_cut_1x_ord": c1,
            "required_cut_05x_ord": c05,
            "phases": phases,
            "prepass_stages": prepass_stages,
            "postpass_stages": postpass_stages,
            "per_layer_modules": per_layer_modules,
        }
    return out


def combination_budget_section(split: dict, route: dict) -> dict:
    """Multi-term non-overlapping zeroing ceilings toward < 1.0x and <= 0.50x."""
    out = {}
    for cell in CELLS:
        sp = split[cell]
        rt = route[cell]
        r_ord = rt["est_post_t185_ratio_ord"]
        r_acc = rt["est_post_t185_ratio_acc"]
        c1 = rt["required_cut_1x_ord"]
        c05 = rt["required_cut_05x_ord"]
        est_s = rt["est_post_t185_pnp_ord_s_on_t174_scale"]

        ps = sp["phase_shares"]
        pre_st = sp["prepass_stage_shares_of_cell"]
        post_st = sp["postpass_stage_shares_of_cell"]
        pl_mods = sp["per_layer_detail"]["modules"]

        def s_share(st: str) -> float:
            return pre_st.get(st, 0.0)

        def p_share(st: str) -> float:
            return post_st.get(st, 0.0)

        def m_share(mid: str) -> float:
            return pl_mods.get(mid, {}).get("implied_wall_share_of_cell", 0.0)

        all_terms = [
            *[(st, sh) for st, sh in pre_st.items()],
            ("phase:per_layer", ps["per_layer"]),
            ("phase:postpass", ps["postpass"]),
        ]
        all_terms.sort(key=lambda kv: (-kv[1], kv[0]))

        combos = {
            "prepass_all": (
                ["phase:prepass"],
                ps["prepass"],
                "instrumented_phase_bracket",
            ),
            "prepass_plus_per_layer": (
                ["phase:prepass", "phase:per_layer"],
                ps["prepass"] + ps["per_layer"],
                "instrumented_phase_brackets",
            ),
            "top2_all_terms": (
                [k for k, _ in all_terms[:2]],
                sum(v for _, v in all_terms[:2]),
                "instrumented_non_overlapping_terms",
            ),
            "top3_all_terms": (
                [k for k, _ in all_terms[:3]],
                sum(v for _, v in all_terms[:3]),
                "instrumented_non_overlapping_terms",
            ),
            "support_geometry_plus_support_analysis": (
                ["PrePass::SupportGeometry", "PrePass::SupportAnalysis"],
                s_share("PrePass::SupportGeometry") + s_share("PrePass::SupportAnalysis"),
                "instrumented_sequential_stages",
            ),
            "support_analysis_plus_postpass": (
                ["PrePass::SupportAnalysis", "phase:postpass"],
                s_share("PrePass::SupportAnalysis") + ps["postpass"],
                "instrumented_stage_plus_phase",
            ),
            "support_geometry_plus_postpass": (
                ["PrePass::SupportGeometry", "phase:postpass"],
                s_share("PrePass::SupportGeometry") + ps["postpass"],
                "instrumented_stage_plus_phase",
            ),
            "support_analysis_plus_layer_finalization": (
                ["PrePass::SupportAnalysis", "PostPass::LayerFinalization"],
                s_share("PrePass::SupportAnalysis") + p_share("PostPass::LayerFinalization"),
                "instrumented_sequential_stages",
            ),
            "support_analysis_plus_slice": (
                ["PrePass::SupportAnalysis", "PrePass::Slice"],
                s_share("PrePass::SupportAnalysis") + s_share("PrePass::Slice"),
                "instrumented_sequential_stages",
            ),
            "support_geometry_plus_support_analysis_plus_postpass": (
                ["PrePass::SupportGeometry", "PrePass::SupportAnalysis", "phase:postpass"],
                s_share("PrePass::SupportGeometry")
                + s_share("PrePass::SupportAnalysis")
                + ps["postpass"],
                "instrumented_stages_plus_phase",
            ),
            "support_geometry_plus_per_layer": (
                ["PrePass::SupportGeometry", "phase:per_layer"],
                s_share("PrePass::SupportGeometry") + ps["per_layer"],
                "instrumented_stage_plus_phase",
            ),
            "shell_plus_slice_plus_per_layer": (
                ["PrePass::ShellClassification", "PrePass::Slice", "phase:per_layer"],
                s_share("PrePass::ShellClassification")
                + s_share("PrePass::Slice")
                + ps["per_layer"],
                "instrumented_stages_plus_phase",
            ),
            "support_analysis_plus_tree_support_est": (
                ["PrePass::SupportAnalysis", "com.core.tree-support"],
                s_share("PrePass::SupportAnalysis") + m_share("com.core.tree-support"),
                "stage_plus_proportional_module",
            ),
        }

        out[cell] = {
            name: {
                "terms": terms,
                **zero_entry(share, r_ord, r_acc, c1, c05, est_s, basis=basis),
            }
            for name, (terms, share, basis) in combos.items()
        }
    return out


def census_section(detail: dict, t177: dict) -> dict:
    """Per-cell `--profile` host clipper thread-time (by op & built-in) and guest fuel."""
    out = {}
    for cell in CELLS:
        rec = detail["cells"][cell]
        host_ns: dict[str, int] = {}
        host_calls: dict[str, int] = {}
        builtins_summary = {}
        for bname, brow in sorted(
            rec["wall"]["builtins"].items(), key=lambda kv: (-kv[1]["amount"], kv[0])
        ):
            scope_map = {}
            for sname, srow in sorted(
                brow["scopes"].items(), key=lambda kv: (-kv[1]["amount"], kv[0])
            ):
                host_ns[sname] = host_ns.get(sname, 0) + srow["amount"]
                host_calls[sname] = host_calls.get(sname, 0) + srow["calls"]
                scope_map[sname] = {
                    "ns": srow["amount"],
                    "s": round(srow["amount"] / 1e9, 4),
                    "calls": srow["calls"],
                }
            builtins_summary[bname] = {
                "total_ns": brow["amount"],
                "total_s": round(brow["amount"] / 1e9, 4),
                "calls": brow["calls"],
                "self_ns": brow["self"],
                "scopes": scope_map,
            }

        host_total = sum(host_ns.values())
        host_wall_total = rec["wall"]["total"]

        fuel_units: dict[str, int] = {}
        modules_ranked = []
        fuel_total = rec["fuel"]["total"]
        for mid, mrow in sorted(
            rec["fuel"]["modules"].items(), key=lambda kv: (-kv[1]["amount"], kv[0])
        ):
            m_scopes = {}
            for sname, srow in sorted(
                mrow["scopes"].items(), key=lambda kv: (-kv[1]["amount"], kv[0])
            ):
                fuel_units[sname] = fuel_units.get(sname, 0) + srow["amount"]
                m_scopes[sname] = {
                    "fuel": srow["amount"],
                    "share_of_module": (
                        round(srow["amount"] / mrow["amount"], 6)
                        if mrow["amount"]
                        else 0.0
                    ),
                    "calls": srow["calls"],
                }
            modules_ranked.append(
                {
                    "module": mid,
                    "fuel": mrow["amount"],
                    "fuel_b": round(mrow["amount"] / 1e9, 4),
                    "share": round(mrow["amount"] / fuel_total, 6) if fuel_total else 0.0,
                    "calls": mrow["calls"],
                    "self_fuel": mrow["self"],
                    "self_share_of_module": (
                        round(mrow["self"] / mrow["amount"], 6) if mrow["amount"] else 0.0
                    ),
                    "scopes": m_scopes,
                }
            )

        t177_c = t177["census"][cell]
        t177_host_ns = t177_c["host_thread_total_ns"]
        t177_fuel = t177_c["guest_fuel_total"]

        out[cell] = {
            "host_wall_total_ns": host_wall_total,
            "host_thread_total_ns": host_total,
            "host_thread_total_s": round(host_total / 1e9, 4),
            "t177_host_thread_total_s": round(t177_host_ns / 1e9, 4),
            "host_thread_ratio_vs_t177": (
                round(host_total / t177_host_ns, 4) if t177_host_ns else None
            ),
            "host_thread_ns": dict(
                sorted(host_ns.items(), key=lambda kv: (-kv[1], kv[0]))
            ),
            "host_scope_calls": dict(
                sorted(host_calls.items(), key=lambda kv: (-host_ns.get(kv[0], 0), kv[0]))
            ),
            "host_scope_shares": {
                k: round(v / host_total, 6) if host_total else 0.0
                for k, v in sorted(host_ns.items(), key=lambda kv: (-kv[1], kv[0]))
            },
            "host_builtins": builtins_summary,
            "guest_fuel_total": fuel_total,
            "guest_fuel_total_b": round(fuel_total / 1e9, 4),
            "t177_guest_fuel_total_b": round(t177_fuel / 1e9, 4),
            "guest_fuel_delta_vs_t177_pct": (
                round(100.0 * (fuel_total - t177_fuel) / t177_fuel, 2)
                if t177_fuel
                else None
            ),
            "guest_fuel_scopes": dict(
                sorted(fuel_units.items(), key=lambda kv: (-kv[1], kv[0]))
            ),
            "guest_fuel_scope_shares": {
                k: round(v / fuel_total, 6) if fuel_total else 0.0
                for k, v in sorted(fuel_units.items(), key=lambda kv: (-kv[1], kv[0]))
            },
            "modules": modules_ranked,
        }
    return out


def candidate_surfaces_section(
    split: dict, route: dict, stage_budget: dict, census: dict
) -> dict:
    """Per-cell breakdown of the top post-t185 candidate surfaces."""
    stage_to_builtin = {
        "PrePass::ShellClassification": "host:shell_classification",
        "PrePass::Slice": "host:slice",
        "PrePass::SupportAnalysis": "host:support_analysis",
        "PrePass::MeshAnalysis": "host:mesh_analysis",
        "PrePass::OverhangAnnotation": "host:overhang_annotation",
    }
    out = {}
    for cell in CELLS:
        sp = split[cell]
        sb = stage_budget[cell]
        cn = census[cell]
        mods_by_id = {m["module"]: m for m in cn["modules"]}

        prepass_rows = {}
        for st in [
            "PrePass::SupportGeometry",
            "PrePass::SupportAnalysis",
            "PrePass::ShellClassification",
            "PrePass::Slice",
            "PrePass::SeamPlanning",
            "PrePass::MeshAnalysis",
            "PrePass::OverhangAnnotation",
        ]:
            srow = sb["prepass_stages"].get(st)
            if not srow:
                continue
            bkey = stage_to_builtin.get(st)
            bdata = cn["host_builtins"].get(bkey) if bkey else None
            row = {
                "instrumented_ms": srow["ms"],
                "share_of_cell": srow["share_of_cell"],
                "gate_floor_multiple": srow["gate_floor_multiple"],
                "est_wall_s_on_t174_scale": srow["est_wall_s_on_t174_scale"],
                "ratio_if_zero_ord": srow["ratio_if_zero_ord"],
                "wins_1x_ord": srow["wins_1x_ord"],
                "frac_of_cut_1x_ord": srow["frac_of_cut_1x_ord"],
            }
            if bdata:
                row["profile_builtin"] = bkey
                row["profile_thread_s"] = bdata["total_s"]
                row["profile_calls"] = bdata["calls"]
                row["profile_self_ns"] = bdata["self_ns"]
                row["profile_scopes"] = bdata["scopes"]
                row["profile_thread_over_inst_wall"] = (
                    round(bdata["total_s"] / (srow["ms"] / 1000.0), 2)
                    if srow["ms"] > 0
                    else None
                )
            if st == "PrePass::SupportGeometry":
                planner_pre = sp["prepass_modules"].get("com.core.tree-support-planner", {})
                trad_pre = sp["prepass_modules"].get("com.core.traditional-support-planner", {})
                planner_fuel = mods_by_id.get("com.core.tree-support-planner", {})
                hn = cn["host_builtins"].get("host:native", {})
                row["tree_planner_ms"] = planner_pre.get("thread_sum_ms", 0)
                row["trad_planner_ms"] = trad_pre.get("thread_sum_ms", 0)
                row["post_planner_tail_ms"] = sp["support_geometry_post_planner_tail_ms"]
                row["planner_guest_fuel_b"] = planner_fuel.get("fuel_b", 0.0)
                row["planner_guest_fuel_share"] = planner_fuel.get("share", 0.0)
                row["host_native_total_s"] = hn.get("total_s", 0.0)
                row["host_native_scopes"] = hn.get("scopes", {})
            prepass_rows[st] = row

        postpass_rows = {
            st: {
                "instrumented_ms": srow["ms"],
                "share_of_cell": srow["share_of_cell"],
                "gate_floor_multiple": srow["gate_floor_multiple"],
                "est_wall_s_on_t174_scale": srow["est_wall_s_on_t174_scale"],
                "ratio_if_zero_ord": srow["ratio_if_zero_ord"],
            }
            for st, srow in sb["postpass_stages"].items()
        }

        out[cell] = {
            "prepass": prepass_rows,
            "postpass_phase_ms": sp["postpass_ms"],
            "postpass_phase_share": sp["phase_shares"]["postpass"],
            "postpass_stages": postpass_rows,
            "per_layer_phase_ms": sp["per_layer_ms"],
            "per_layer_phase_share": sp["phase_shares"]["per_layer"],
            "per_layer_eff_par": sp["per_layer_detail"]["effective_parallelism"],
            "per_layer_host_gap_ms": sp["per_layer_detail"]["host_gap_ms"],
            "per_layer_straggler_tail": sp["per_layer_detail"]["straggler_tail"],
            "per_layer_modules": sb["per_layer_modules"],
            "outside_ms": sp["outside_ms"],
            "outside_share": sp["phase_shares"]["outside"],
        }
    return out


def compute_all(
    instrumented: dict,
    profile_clean: dict,
    profile_detail: dict,
    t174: dict,
    t185_route: dict,
    t177: dict,
) -> dict:
    split = split_section(instrumented, t177)
    route = route_section(t174, t185_route)
    stage_budget = stage_budget_section(split, route)
    combo_budget = combination_budget_section(split, route)
    census = census_section(profile_detail, t177)
    candidate_surfaces = candidate_surfaces_section(split, route, stage_budget, census)
    return {
        "schema": 1,
        "note": (
            "ESTIMATE ONLY chain, never a ratio of record. Phase and sequential "
            "stage shares are this take's instrumented median over 3 passes/cell; "
            "per-layer module wall shares are proportional thread-time ceilings; "
            "baseline ratios are t185's reduced/route.json (t174 x t175 x t182 x "
            "t183 x t184 x t185). No instrumented or --profile time is a wall claim."
        ),
        "split": split,
        "route": route,
        "stage_budget": stage_budget,
        "combination_budget": combo_budget,
        "census": census,
        "candidate_surfaces": candidate_surfaces,
        "profile_scopes": {
            cell: profile_clean["cells"][cell]["scopes"] for cell in CELLS
        },
    }


def main() -> int:
    instrumented = load(HERE / "reduced" / "instrumented.json")
    profile_clean = load(HERE / "reduced" / "profile-clean.json")
    profile_detail = load(HERE / "reduced" / "profile-detail-clean.json")
    t174 = load(EVID / "t174-post-t172-scoreboard" / "reduced" / "summary.json")
    t185_route = load(
        EVID / "t185-support-geometry-host-tail-accel" / "reduced" / "route.json"
    )
    t177 = load(EVID / "t177-post-t175-attribution" / "reduced" / "arithmetic.json")

    payload = compute_all(
        instrumented, profile_clean, profile_detail, t174, t185_route, t177
    )

    out = HERE / "reduced" / "arithmetic.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=1) + "\n", encoding="utf-8")

    print("Phase shares (median of 3 instrumented passes, post-t185):")
    for cell in CELLS:
        sp = payload["split"][cell]
        s = sp["phase_shares"]
        d = sp["delta_phase_shares_vs_t177_pp"]
        print(
            f"  {cell:20s} {sp['slice_complete_ms']:6d}ms ({sp['slice_complete_delta_vs_t177_pct']:+5.1f}% vs t177)  "
            f"prepass {100*s['prepass']:5.1f}% ({d['prepass']:+5.1f}pp)  "
            f"per-layer {100*s['per_layer']:5.1f}% ({d['per_layer']:+5.1f}pp)  "
            f"postpass {100*s['postpass']:4.1f}% ({d['postpass']:+5.1f}pp)  "
            f"outside {100*s['outside']:4.1f}%"
        )
    print("\nPost-t185 ESTIMATE ONLY route (<1.0x and <=0.50x cuts, ordinary):")
    for cell in CELLS:
        r = payload["route"][cell]
        print(
            f"  {cell:20s} est={r['est_post_t185_ratio_ord']:5.3f}x  "
            f"cut_1x={100*r['required_cut_1x_ord']:+5.1f}%  "
            f"cut_05x={100*r['required_cut_05x_ord']:+5.1f}%  "
            f"orca={r['t174_orca_s']:6.2f}s  est_pnp={r['est_post_t185_pnp_ord_s_on_t174_scale']:6.2f}s"
        )
    print("\nHost clipper thread-time split (--profile attribution only):")
    for cell in CELLS:
        c = payload["census"][cell]
        sh = c["host_scope_shares"]
        print(
            f"  {cell:20s} clip {100*sh.get('polygon_ops::clip_polygons',0):5.1f}%  "
            f"offset {100*sh.get('polygon_ops::offset',0):5.1f}%  "
            f"offset2_ex {100*sh.get('polygon_ops::offset2_ex',0):5.1f}%  "
            f"closing {100*sh.get('polygon_ops::closing_ex',0):4.1f}%  "
            f"total {c['host_thread_total_s']:6.2f}s (t177: {c['t177_host_thread_total_s']:6.2f}s)"
        )
    print("\nGuest fuel (deterministic):")
    for cell in CELLS:
        c = payload["census"][cell]
        top = ", ".join(
            f"{m['module'].replace('com.core.','')} {100*m['share']:.1f}%"
            for m in c["modules"][:3]
        )
        print(
            f"  {cell:20s} {c['guest_fuel_total_b']:6.2f}B "
            f"({c['guest_fuel_delta_vs_t177_pct']:+5.1f}% vs t177)  {top}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

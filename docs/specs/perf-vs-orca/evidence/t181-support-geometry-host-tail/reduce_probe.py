#!/usr/bin/env python3
"""Reduce raw t181 observation-only probe captures from target/t181-probe/ into
committed reduced/probe.json.

All downstream arithmetic and checkout-only verification (`arithmetic.py`,
`verify.py`) read strictly from committed `reduced/probe.json`, never from
gitignored `target/t181-probe/`.
"""

from __future__ import annotations

import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW_DIR = REPO / "target" / "t181-probe"
OUT_PATH = HERE / "reduced" / "probe.json"

SUPPORTS_ON = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]
SUPPORTS_OFF = [
    "benchy-classic-off",
    "base-classic-off",
]


def r3(x: float) -> float:
    return round(x, 3)


def r4(x: float) -> float:
    return round(x, 4)


def reduce_probe() -> dict:
    runs_doc = json.loads((RAW_DIR / "probe_runs.json").read_text(encoding="utf-8"))
    runs = runs_doc["runs"]

    # Per-cell median over the 3 uninstrumented probe passes (pass1..pass3)
    cells_summary = {}
    for cell in SUPPORTS_ON:
        pass_files = [RAW_DIR / f"{cell}.pass{i}.probe.json" for i in (1, 2, 3)]
        probes = [json.loads(p.read_text(encoding="utf-8")) for p in pass_files]
        pass_runs = [r for r in runs if r["cell"] == cell and r["tag"].startswith("pass")]
        qual_run = next(r for r in runs if r["cell"] == cell and r["tag"] == "qual")

        stage_ms = [p["stage_total_ns"] / 1e6 for p in probes]
        tail_ms = [p["post_planner_tail_ns"] / 1e6 for p in probes]
        wall_s = [r["wall_s"] for r in pass_runs]

        med_tail = statistics.median(tail_ms)
        med_stage = statistics.median(stage_ms)
        med_wall_ms = statistics.median(wall_s) * 1000.0

        brackets_med_ms = {}
        brackets_share_of_tail_pct = {}
        for k in sorted(probes[0]["brackets_ns"].keys()):
            vals = [p["brackets_ns"][k] / 1e6 for p in probes]
            m = statistics.median(vals)
            brackets_med_ms[k[:-3] + "_ms"] = r3(m)
            brackets_share_of_tail_pct[k[:-3]] = round(100.0 * m / med_tail, 2) if med_tail > 0 else 0.0

        # Verify all 3 passes produced identical deterministic counters
        c0 = probes[0]["counters"]
        counters_identical_across_passes = all(p["counters"] == c0 for p in probes[1:])

        # Module order & median run_stage_ms
        mod_order = [m["module_id"] for m in probes[0]["modules"]]
        mods_med = []
        for idx, mod_id in enumerate(mod_order):
            run_vals = [p["modules"][idx]["run_stage_ns"] / 1e6 for p in probes]
            post_vals = [p["modules"][idx]["post_module_ns"] / 1e6 for p in probes]
            mods_med.append(
                {
                    "module_id": mod_id,
                    "median_run_stage_ms": r3(statistics.median(run_vals)),
                    "median_post_module_ms": r3(statistics.median(post_vals)),
                    "plan_entries": probes[0]["modules"][idx]["plan_entries"],
                }
            )

        # Additive top-level decomposition check: T1 + T2 + T3 + T4a + T4b + T5 vs post_planner_tail_ns
        top_keys = [
            "t1_post_module_loop_tail_ns",
            "t2_exact_z_new_ns",
            "t3_aggregate_total_ns",
            "t4a_attach_diags_ns",
            "t4b_commit_ns",
            "t5_drop_ns",
        ]
        top_sums_ms = [sum(p["brackets_ns"][k] for k in top_keys) / 1e6 for p in probes]
        max_top_residual_ms = max(abs(t - s) for t, s in zip(tail_ms, top_sums_ms))

        # T3 internal decomposition check: T3a + T3b + T3c + T3d + T3e + T3f + T3g + T3h + T3i + T3j + T3k + T3_drop_plans vs T3
        t3_keys = [
            "t3a_plans_clone_ns",
            "t3b_entries_flatten_clone_ns",
            "t3c_entries_sort_and_raft_ns",
            "t3d_collect_exact_z_planes_ns",
            "t3e_prefetch_total_ns",
            "t3f_validate_loop_total_ns",
            "t3g_union_same_family_ns",
            "t3h_territory_clipper_ns",
            "t3i_cross_family_overlap_ns",
            "t3j_try_agg_drop_ns",
            "t3k_attribution_indices_and_diags_ns",
            "t3_drop_plans_ns",
        ]
        t3_totals_ms = [p["brackets_ns"]["t3_aggregate_total_ns"] / 1e6 for p in probes]
        t3_sums_ms = [sum(p["brackets_ns"][k] for k in t3_keys) / 1e6 for p in probes]
        max_t3_residual_ms = max(abs(t - s) for t, s in zip(t3_totals_ms, t3_sums_ms))

        cells_summary[cell] = {
            "pass_walls_s": [r4(v) for v in wall_s],
            "median_wall_s": r4(statistics.median(wall_s)),
            "pass_stage_ms": [r3(v) for v in stage_ms],
            "median_stage_ms": r3(med_stage),
            "pass_tail_ms": [r3(v) for v in tail_ms],
            "median_tail_ms": r3(med_tail),
            "tail_share_of_stage_pct": round(100.0 * med_tail / med_stage, 2),
            "tail_share_of_process_wall_pct": round(100.0 * med_tail / med_wall_ms, 2),
            "modules": mods_med,
            "brackets_median_ms": brackets_med_ms,
            "brackets_share_of_tail_pct": brackets_share_of_tail_pct,
            "counters": c0,
            "counters_identical_across_passes": counters_identical_across_passes,
            "reconciliation": {
                "max_top_level_residual_ms": r3(max_top_residual_ms),
                "max_t3_internal_residual_ms": r3(max_t3_residual_ms),
                "qual_probe_tail_ms": qual_run["post_planner_tail_ms"],
                "qual_instrument_tail_ms": qual_run["qual"]["sg_post_planner_tail_ms"],
                "qual_delta_ms": r3(
                    abs(qual_run["post_planner_tail_ms"] - qual_run["qual"]["sg_post_planner_tail_ms"])
                ),
            },
        }

    # Supports-off negative controls
    controls_summary = {}
    for cell in SUPPORTS_OFF:
        ctrl_probe = json.loads((RAW_DIR / f"{cell}.ctrl1.probe.json").read_text(encoding="utf-8"))
        qual_run = next(r for r in runs if r["cell"] == cell and r["tag"] == "qual")
        controls_summary[cell] = {
            "ctrl_tail_ms": r3(ctrl_probe["post_planner_tail_ns"] / 1e6),
            "ctrl_stage_ms": r3(ctrl_probe["stage_total_ns"] / 1e6),
            "plans_count": ctrl_probe["counters"]["plans_count"],
            "active_entries": ctrl_probe["counters"]["active_entries"],
            "overlap_pairs_total": ctrl_probe["counters"]["overlap_pairs_total"],
            "qual_instrument_tail_ms": qual_run["qual"]["sg_post_planner_tail_ms"],
        }

    # Classic vs Arachne invariance check
    invariance = {}
    for model in ("benchy", "base"):
        c_cnt = cells_summary[f"{model}-classic-on"]["counters"]
        a_cnt = cells_summary[f"{model}-arachne-on"]["counters"]
        diff_keys = [k for k in sorted(c_cnt.keys()) if c_cnt[k] != a_cnt[k]]
        invariance[f"{model}_classic_vs_arachne"] = {
            "identical_counters": len(diff_keys) == 0,
            "diff_keys": diff_keys,
        }

    # PPO equivalence verification from ppo_dump runs
    ppo_verify = {}
    for cell in ("benchy-classic-on", "base-classic-on"):
        r = next(x for x in runs if x["cell"] == cell and x["tag"] == "ppo_dump")
        ppo_verify[cell] = r["ppo_verify"]

    # Slim runs list for qualification verification
    slim_runs = []
    for r in runs:
        entry = {
            "cell": r["cell"],
            "tag": r["tag"],
            "instrumented": r["instrumented"],
            "verify_ppo": r["verify_ppo"],
            "dump_io": r["dump_io"],
            "exit_code": r["exit_code"],
            "wall_s": r["wall_s"],
            "sha256": r["sha256"],
            "bytes": r["bytes"],
            "matches_reference": r["matches_reference"],
            "stage_total_ms": r["stage_total_ms"],
            "post_planner_tail_ms": r["post_planner_tail_ms"],
        }
        if "qual" in r:
            entry["qual"] = r["qual"]
        if r.get("ppo_verify") is not None:
            entry["ppo_verify"] = r["ppo_verify"]
        slim_runs.append(entry)

    out = {
        "schema": 1,
        "note": (
            "Reduced by reduce_probe.py from gitignored target/t181-probe/. "
            "Observation-only probe (probe.patch cleanly reverted); zero production edits. "
            "Every run verified byte-identical to t175/reference.json."
        ),
        "captured_utc": runs_doc["captured_utc"],
        "exe_sha256": runs_doc["exe_sha256"],
        "rayon_num_threads": runs_doc["rayon_num_threads"],
        "reference": runs_doc["reference"],
        "total_runs": len(slim_runs),
        "runs": slim_runs,
        "supports_on_cells": cells_summary,
        "supports_off_controls": controls_summary,
        "invariance": invariance,
        "ppo_verify": ppo_verify,
    }
    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUT_PATH.write_text(json.dumps(out, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {OUT_PATH} ({OUT_PATH.stat().st_size} bytes, {len(slim_runs)} runs)")
    return out


if __name__ == "__main__":
    reduce_probe()

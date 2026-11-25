#!/usr/bin/env python3
"""Reduce t157 probe JSONL logs into probe.json."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
DEFAULT_RAW_DIR = REPO / "target" / "t157-probe"
CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]
MODES = ["dual", "fast"]


def reduce_log(path: Path) -> dict:
    ns_dual_by_caller: dict[str, dict] = defaultdict(
        lambda: {
            "calls": 0,
            "fast_us_total": 0,
            "full_us_total": 0,
            "mismatches": 0,
            "true_count": 0,
            "false_count": 0,
            "pip_tests_total": 0,
            "branches": defaultdict(int),
            "a_polys_total": 0,
            "a_verts_total": 0,
            "b_polys_total": 0,
            "b_verts_total": 0,
        }
    )
    ns_fast_by_caller: dict[str, dict] = defaultdict(
        lambda: {
            "calls": 0,
            "fast_us_total": 0,
            "true_count": 0,
            "false_count": 0,
            "pip_tests_total": 0,
            "branches": defaultdict(int),
            "a_polys_total": 0,
            "a_verts_total": 0,
            "b_polys_total": 0,
            "b_verts_total": 0,
        }
    )
    prep_views_by_stage: dict[str, dict] = defaultdict(
        lambda: {"calls": 0, "prep_us_total": 0}
    )
    run_stage_by_stage: dict[str, dict] = defaultdict(
        lambda: {"calls": 0, "run_us_total": 0}
    )
    module_complete_by_stage: dict[str, dict] = defaultdict(
        lambda: {"calls": 0, "elapsed_ms_total": 0}
    )
    stage_complete_ms: dict[str, int] = defaultdict(int)
    phase_complete_ms: dict[str, int] = {}
    sa_eligibility: dict | None = None
    sa_contacts: dict | None = None
    slice_complete: dict | None = None
    slice_stats: dict | None = None

    with path.open("r", encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if not line or not line.startswith("{"):
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue

            probe_kind = rec.get("t157_probe")
            if probe_kind == "ns_dual":
                caller = rec["caller"]
                d = ns_dual_by_caller[caller]
                d["calls"] += 1
                d["fast_us_total"] += int(rec["fast_us"])
                d["full_us_total"] += int(rec["full_us"])
                if not rec["match"]:
                    d["mismatches"] += 1
                if rec["full_val"]:
                    d["true_count"] += 1
                else:
                    d["false_count"] += 1
                d["pip_tests_total"] += int(rec["pip_tests"])
                d["branches"][rec["branch"]] += 1
                d["a_polys_total"] += int(rec["a_polys"])
                d["a_verts_total"] += int(rec["a_verts"])
                d["b_polys_total"] += int(rec["b_polys"])
                d["b_verts_total"] += int(rec["b_verts"])
            elif probe_kind == "ns_fast":
                caller = rec["caller"]
                d = ns_fast_by_caller[caller]
                d["calls"] += 1
                d["fast_us_total"] += int(rec["fast_us"])
                if rec["val"]:
                    d["true_count"] += 1
                else:
                    d["false_count"] += 1
                d["pip_tests_total"] += int(rec["pip_tests"])
                d["branches"][rec["branch"]] += 1
                d["a_polys_total"] += int(rec["a_polys"])
                d["a_verts_total"] += int(rec["a_verts"])
                d["b_polys_total"] += int(rec["b_polys"])
                d["b_verts_total"] += int(rec["b_verts"])
            elif probe_kind == "prep_views":
                st = rec["stage_id"]
                prep_views_by_stage[st]["calls"] += 1
                prep_views_by_stage[st]["prep_us_total"] += int(rec["prep_us"])
            elif probe_kind == "run_stage":
                st = rec["stage_id"]
                run_stage_by_stage[st]["calls"] += 1
                run_stage_by_stage[st]["run_us_total"] += int(rec["run_us"])
            elif probe_kind == "sa_eligibility":
                sa_eligibility = {
                    "regions": int(rec["regions"]),
                    "true_count": int(rec["true_count"]),
                    "wall_us": int(rec["wall_us"]),
                    "wall_ms": round(int(rec["wall_us"]) / 1000.0, 3),
                }
            elif probe_kind == "sa_contacts":
                sa_contacts = {
                    "contacts": int(rec["contacts"]),
                    "wall_us": int(rec["wall_us"]),
                    "wall_ms": round(int(rec["wall_us"]) / 1000.0, 3),
                }
            else:
                ev = rec.get("event")
                if ev == "module_complete":
                    st = rec.get("stage") or "unknown"
                    module_complete_by_stage[st]["calls"] += 1
                    module_complete_by_stage[st]["elapsed_ms_total"] += int(
                        rec.get("elapsed_ms") or 0
                    )
                elif ev == "stage_complete":
                    st = rec.get("stage")
                    if st:
                        stage_complete_ms[st] += int(rec.get("elapsed_ms") or 0)
                elif ev == "phase_complete":
                    ph = rec.get("phase")
                    if ph:
                        phase_complete_ms[ph] = int(rec.get("elapsed_ms") or 0)
                elif ev == "slice_complete":
                    slice_complete = {
                        "elapsed_ms": int(rec.get("elapsed_ms") or 0),
                        "degraded": bool(rec.get("degraded")),
                        "fatal_error_count": int(rec.get("fatal_error_count") or 0),
                        "non_fatal_error_count": int(
                            rec.get("non_fatal_error_count") or 0
                        ),
                    }
                elif ev == "slice_stats":
                    slice_stats = {
                        "layer_count": int(rec.get("layer_count") or 0),
                    }

    def finalize_ns_dual(raw_map: dict) -> dict:
        out = {}
        for caller, d in sorted(raw_map.items()):
            calls = d["calls"]
            out[caller] = {
                "calls": calls,
                "fast_us_total": d["fast_us_total"],
                "fast_ms_total": round(d["fast_us_total"] / 1000.0, 3),
                "fast_us_mean": round(d["fast_us_total"] / calls, 3) if calls else 0.0,
                "full_us_total": d["full_us_total"],
                "full_ms_total": round(d["full_us_total"] / 1000.0, 3),
                "full_us_mean": round(d["full_us_total"] / calls, 3) if calls else 0.0,
                "saved_us_total": d["full_us_total"] - d["fast_us_total"],
                "saved_ms_total": round(
                    (d["full_us_total"] - d["fast_us_total"]) / 1000.0, 3
                ),
                "speedup_ratio": (
                    round(d["full_us_total"] / max(d["fast_us_total"], 1), 2)
                    if calls
                    else 0.0
                ),
                "mismatches": d["mismatches"],
                "true_count": d["true_count"],
                "false_count": d["false_count"],
                "pip_tests_total": d["pip_tests_total"],
                "pip_tests_mean": (
                    round(d["pip_tests_total"] / calls, 2) if calls else 0.0
                ),
                "a_polys_mean": round(d["a_polys_total"] / calls, 2) if calls else 0.0,
                "a_verts_mean": round(d["a_verts_total"] / calls, 1) if calls else 0.0,
                "b_polys_mean": round(d["b_polys_total"] / calls, 2) if calls else 0.0,
                "b_verts_mean": round(d["b_verts_total"] / calls, 1) if calls else 0.0,
                "branches": dict(sorted(d["branches"].items())),
            }
        return out

    def finalize_ns_fast(raw_map: dict) -> dict:
        out = {}
        for caller, d in sorted(raw_map.items()):
            calls = d["calls"]
            out[caller] = {
                "calls": calls,
                "fast_us_total": d["fast_us_total"],
                "fast_ms_total": round(d["fast_us_total"] / 1000.0, 3),
                "fast_us_mean": round(d["fast_us_total"] / calls, 3) if calls else 0.0,
                "true_count": d["true_count"],
                "false_count": d["false_count"],
                "pip_tests_total": d["pip_tests_total"],
                "pip_tests_mean": (
                    round(d["pip_tests_total"] / calls, 2) if calls else 0.0
                ),
                "a_polys_mean": round(d["a_polys_total"] / calls, 2) if calls else 0.0,
                "a_verts_mean": round(d["a_verts_total"] / calls, 1) if calls else 0.0,
                "b_polys_mean": round(d["b_polys_total"] / calls, 2) if calls else 0.0,
                "b_verts_mean": round(d["b_verts_total"] / calls, 1) if calls else 0.0,
                "branches": dict(sorted(d["branches"].items())),
            }
        return out

    per_layer_stages = {}
    all_stages = sorted(
        set(prep_views_by_stage.keys())
        | set(run_stage_by_stage.keys())
        | set(module_complete_by_stage.keys())
    )
    for st in all_stages:
        if not st.startswith("Layer::"):
            continue
        p = prep_views_by_stage.get(st, {"calls": 0, "prep_us_total": 0})
        r = run_stage_by_stage.get(st, {"calls": 0, "run_us_total": 0})
        m = module_complete_by_stage.get(st, {"calls": 0, "elapsed_ms_total": 0})
        st_ms = stage_complete_ms.get(st, 0)
        per_layer_stages[st] = {
            "calls": m["calls"] or p["calls"],
            "stage_complete_thread_sum_ms": st_ms,
            "module_complete_thread_sum_ms": m["elapsed_ms_total"],
            "prep_views_ms": round(p["prep_us_total"] / 1000.0, 3),
            "run_stage_ms": round(r["run_us_total"] / 1000.0, 3),
            "prep_plus_run_ms": round(
                (p["prep_us_total"] + r["run_us_total"]) / 1000.0, 3
            ),
        }

    prepass_stages = {}
    for st, dur_ms in sorted(stage_complete_ms.items()):
        if st.startswith("PrePass::") or st.startswith("PostPass::"):
            prepass_stages[st] = dur_ms

    return {
        "slice_complete": slice_complete,
        "slice_stats": slice_stats,
        "phases_ms": phase_complete_ms,
        "ns_dual_by_caller": finalize_ns_dual(ns_dual_by_caller),
        "ns_fast_by_caller": finalize_ns_fast(ns_fast_by_caller),
        "sa_eligibility": sa_eligibility,
        "sa_contacts": sa_contacts,
        "per_layer_stages": per_layer_stages,
        "prepass_postpass_stages_ms": prepass_stages,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw-dir", default=str(DEFAULT_RAW_DIR))
    a = ap.parse_args()
    raw_dir = Path(a.raw_dir)

    runs = json.loads((HERE / "runs.json").read_text(encoding="utf-8"))
    runs_index = {(r["cell"], r["mode"]): r for r in runs}

    probe_out = {"cells": {}}
    for cell in CELLS:
        probe_out["cells"][cell] = {}
        for mode in MODES:
            log_path = raw_dir / f"{cell}.{mode}.jsonl"
            reduced = reduce_log(log_path)
            reduced["run_meta"] = runs_index[(cell, mode)]
            probe_out["cells"][cell][mode] = reduced

    (HERE / "probe.json").write_text(
        json.dumps(probe_out, indent=2) + "\n", encoding="utf-8"
    )
    print("Wrote probe.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

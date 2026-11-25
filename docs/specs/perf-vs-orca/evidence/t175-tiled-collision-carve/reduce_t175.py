#!/usr/bin/env python3
"""t175 reduce: tile-size sweep, attribution probe, standing t137 paired A/B +
null + accelerated corroboration, 16-cell output currency, and cross-window
route composition.

Reads the gitignored raw tree under `target/t175/` plus sibling committed
reduced artifacts (`t172/reference.json`, `t173/reduced/arithmetic.json`,
`t174/reduced/summary.json`), and writes the in-repo reduced set under
`reduced/`:
  - `sweep.json`
  - `probe.json`
  - `ab.json`
  - `null.json`
  - `accel.json`
  - `output.json`
  - `route.json`

Pre-declared gate (standing t137 generalized 4-cell planner gate):
  - All 4 supports-on cells clear the 0.80 % paired wall floor AND sit strictly
    outside their same-window null band's any-pair bound, with process CPU
    corroborating (positive median CPU saving).
  - All 4 supports-off cells show no regression (within null/floor band;
    structurally inert when supports are off).
  - Baseline reproduces t172's active reference set in 8/8 ordinary + 8/8
    accelerated; candidate accelerated is byte-identical to candidate ordinary
    in 8/8 (16/16 total).
  - Zero degraded runs, zero fatal errors, zero code-1200 diagnostics.

Usage:  python reduce_t175.py
"""

import csv
import json
import os
import re
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
RAW = os.path.join(REPO, "target", "t175")
OUT = os.path.join(HERE, "reduced")

T172_REF_PATH = os.path.join(
    HERE, "..", "t172-offset-concave-crossing", "reference.json"
)
T173_ARITH_PATH = os.path.join(
    HERE, "..", "t173-support-boolean-volume", "reduced", "arithmetic.json"
)
T174_SUMMARY_PATH = os.path.join(
    HERE, "..", "t174-post-t172-scoreboard", "reduced", "summary.json"
)

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
SUPPORTS_ON = [c for c in CELLS if c.endswith("-on")]
SUPPORTS_OFF = [c for c in CELLS if c.endswith("-off")]
SWEEP_ARMS = ["baseline", "tile-1mm", "tile-2mm", "tile-5mm", "tile-10mm"]
FLOOR_PCT = 0.80

PROBE_BATCH_RE = re.compile(r"T175_PROBE_BATCH (\{.*\})")


def read_json(path):
    with open(path, encoding="utf-8-sig") as fh:
        return json.load(fh)


def read_rows(path):
    with open(path, encoding="utf-8-sig", newline="") as fh:
        return list(csv.DictReader(fh))


def write_json(name, doc):
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, name)
    with open(path, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(doc, fh, indent=2)
        fh.write("\n")


def pair_savings(rows, metric):
    by = {}
    for row in rows:
        if row.get("null_control") in ("True", "true", "1"):
            continue
        by.setdefault((row["cell"], int(row["pair"])), {})[row["side"]] = float(
            row[metric]
        )
    out = {}
    for (cell, pair), sides in sorted(by.items()):
        if "baseline" not in sides or "candidate" not in sides:
            continue
        b, c = sides["baseline"], sides["candidate"]
        pct = (b - c) / b * 100.0
        out.setdefault(cell, []).append(
            {"pair": pair, "baseline_s": b, "candidate_s": c, "saving_pct": pct}
        )
    return out


def cell_stats(savings):
    vals = [p["saving_pct"] for p in savings]
    return {
        "pairs": len(vals),
        "median_saving_pct": statistics.median(vals),
        "mean_saving_pct": statistics.fmean(vals),
        "min_saving_pct": min(vals),
        "max_saving_pct": max(vals),
        "wins": sum(1 for v in vals if v > 0.0),
        "baseline_median_s": statistics.median([p["baseline_s"] for p in savings]),
        "candidate_median_s": statistics.median([p["candidate_s"] for p in savings]),
        "floor_multiple": statistics.median(vals) / FLOOR_PCT,
    }


def null_band(rows):
    by = {}
    for row in rows:
        by.setdefault((row["cell"], int(row["pair"])), {})[row["side"]] = row
    out = {}
    for (cell, pair), sides in sorted(by.items()):
        a, b = sides.get("baseline"), sides.get("baseline2")
        if not a or not b:
            continue
        walls = abs(
            (float(a["wall_s"]) - float(b["wall_s"])) / float(b["wall_s"]) * 100.0
        )
        cpus = abs((float(a["cpu_s"]) - float(b["cpu_s"])) / float(b["cpu_s"]) * 100.0)
        out.setdefault(cell, {"wall_abs": [], "cpu_abs": []})
        out[cell]["wall_abs"].append(walls)
        out[cell]["cpu_abs"].append(cpus)
    res = {}
    for cell in CELLS:
        data = out[cell]
        res[cell] = {
            "pairs": len(data["wall_abs"]),
            "wall_any_pair_pct": max(data["wall_abs"]),
            "wall_median_pct": statistics.median(data["wall_abs"]),
            "cpu_any_pair_pct": max(data["cpu_abs"]),
            "cpu_median_pct": statistics.median(data["cpu_abs"]),
        }
    res["_worst"] = {
        "wall_any_pair_pct": max(res[c]["wall_any_pair_pct"] for c in CELLS),
        "wall_median_pct": max(res[c]["wall_median_pct"] for c in CELLS),
        "cpu_any_pair_pct": max(res[c]["cpu_any_pair_pct"] for c in CELLS),
        "cpu_median_pct": max(res[c]["cpu_median_pct"] for c in CELLS),
    }
    return res


def gate_t137(summary, null):
    """Standing t137 generalized 4-cell planner gate."""
    verdicts = {}
    for cell in CELLS:
        w = summary[cell]["wall"]
        c = summary[cell]["cpu"]
        n = null[cell]
        if cell in SUPPORTS_ON:
            clears_floor = w["median_saving_pct"] >= FLOOR_PCT
            outside_null = w["median_saving_pct"] > n["wall_any_pair_pct"]
            cpu_ok = c["median_saving_pct"] > 0.0
            if clears_floor and outside_null and cpu_ok:
                verdicts[cell] = "PASS"
            elif not clears_floor:
                verdicts[cell] = "SUB-FLOOR"
            elif not outside_null:
                verdicts[cell] = "INSIDE-NULL"
            else:
                verdicts[cell] = "CPU-CONTRADICTED"
        else:
            # Supports-off: must not regress beyond max(FLOOR_PCT, null any-pair bound)
            bound = max(FLOOR_PCT, n["wall_any_pair_pct"])
            if w["median_saving_pct"] >= -bound:
                verdicts[cell] = "INERT-PASS"
            else:
                verdicts[cell] = "REGRESSION"
    return verdicts


def inspect_jsonl_stderr(path):
    """Inspect a --instrument-stderr JSONL file for slice_complete & code 1200."""
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    slice_elapsed_ms = None
    planner_elapsed_ms = None
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if not line.startswith("{"):
                continue
            try:
                ev = json.loads(line)
            except json.JSONDecodeError:
                continue
            kind = ev.get("event")
            if kind == "slice_complete":
                degraded = ev.get("degraded")
                fatal = ev.get("fatal_error_count")
                non_fatal = ev.get("non_fatal_error_count")
                slice_elapsed_ms = ev.get("elapsed_ms")
            elif (
                kind == "module_complete"
                and ev.get("module_id") == "com.core.tree-support-planner"
            ):
                planner_elapsed_ms = ev.get("elapsed_ms")
            elif kind == "diagnostic" and ev.get("code") == 1200:
                code_1200 += 1
            elif "code 1200" in str(ev.get("message", "")):
                code_1200 += 1
    return {
        "degraded": degraded,
        "fatal_error_count": fatal,
        "non_fatal_error_count": non_fatal,
        "code_1200_count": code_1200,
        "slice_elapsed_ms": slice_elapsed_ms,
        "planner_elapsed_ms": planner_elapsed_ms,
    }


def reduce_sweep(t172_active):
    rows = read_rows(os.path.join(RAW, "sweep", "sweep1", "rows.csv"))
    meta = read_json(os.path.join(RAW, "sweep", "sweep1", "batch_meta.json"))
    by_arm = {}
    for row in rows:
        by_arm.setdefault(row["arm"], []).append(row)

    arms = {}
    for arm in SWEEP_ARMS:
        arows = by_arm[arm]
        walls = [float(r["wall_s"]) for r in arows]
        cpus = [float(r["cpu_s"]) for r in arows]
        shas = sorted({r["gcode_sha256"] for r in arows})
        bytes_set = sorted({int(r["gcode_bytes"]) for r in arows})
        wasms = sorted({r["planner_wasm_sha256"] for r in arows})
        exits = sorted({int(r["exit_code"]) for r in arows})
        arms[arm] = {
            "runs": len(arows),
            "wall_s": walls,
            "cpu_s": cpus,
            "wall_median_s": statistics.median(walls),
            "cpu_median_s": statistics.median(cpus),
            "wall_min_s": min(walls),
            "wall_max_s": max(walls),
            "cpu_min_s": min(cpus),
            "cpu_max_s": max(cpus),
            "gcode_sha256": shas[0] if len(shas) == 1 else shas,
            "gcode_bytes": bytes_set[0] if len(bytes_set) == 1 else bytes_set,
            "planner_wasm_sha256": wasms[0] if len(wasms) == 1 else wasms,
            "exit_codes": exits,
            "matches_t172": (len(shas) == 1 and shas[0] == t172_active["base-classic-on"]),
        }

    base_wall = arms["baseline"]["wall_median_s"]
    base_cpu = arms["baseline"]["cpu_median_s"]
    for arm in SWEEP_ARMS:
        a = arms[arm]
        a["wall_saving_pct"] = (base_wall - a["wall_median_s"]) / base_wall * 100.0
        a["cpu_saving_pct"] = (base_cpu - a["cpu_median_s"]) / base_cpu * 100.0

    tiled_arms = [a for a in SWEEP_ARMS if a != "baseline"]
    chosen = min(tiled_arms, key=lambda a: arms[a]["wall_median_s"])
    return {
        "schema": 1,
        "cell": "base-classic-on",
        "selection_rule": "lowest measured median wall across {1, 2, 5, 10} mm",
        "chosen_arm": chosen,
        "meta": meta,
        "arms": arms,
    }


def parse_probe_file(path):
    """Parse one `--profile --instrument-stderr` capture."""
    counter_keys = [
        "layers_indexed",
        "regions_total",
        "outer_gated",
        "empty_gated",
        "covered_dropped",
        "clip_requests",
        "cache_hits",
        "orig_clip_pts",
        "win_clip_pts",
        "sent_subj_pts",
        "grid_cells",
        "cell_entries",
        "row_entries",
    ]
    sums = {k: 0 for k in counter_keys}
    batches = 0
    tile_units = None
    tiling = None
    planner_elapsed_ms = None
    slice_elapsed_ms = None
    profile_obj = None
    degraded = None
    fatal = None

    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if line.startswith("{"):
                try:
                    ev = json.loads(line)
                except json.JSONDecodeError:
                    ev = None
                if ev is not None:
                    kind = ev.get("event")
                    if kind == "module_log":
                        msg = ev.get("message", "")
                        if msg.startswith("T175_PROBE_BATCH "):
                            payload = json.loads(msg.split(" ", 1)[1])
                            batches += 1
                            tile_units = payload.get("tile_units")
                            tiling = payload.get("tiling")
                            for k in counter_keys:
                                sums[k] += int(payload.get(k, 0))
                    elif (
                        kind == "module_complete"
                        and ev.get("module_id") == "com.core.tree-support-planner"
                    ):
                        planner_elapsed_ms = ev.get("elapsed_ms")
                    elif kind == "profile_summary":
                        profile_obj = ev.get("profile")
                    elif kind == "slice_complete":
                        slice_elapsed_ms = ev.get("elapsed_ms")
                        degraded = ev.get("degraded")
                        fatal = ev.get("fatal_error_count")
                continue

            # Non-JSON plain WARN lines (if any)
            m = PROBE_BATCH_RE.search(line)
            if m and batches == 0:
                # Only fallback if not already parsed from module_log JSON
                pass

    # Extract planner & host:native scopes from profile_summary JSON
    planner_prof = {}
    host_native_prof = {}
    fuel_total = None
    wall_total_ns = None
    if profile_obj:
        fuel_total = profile_obj.get("fuel_total")
        wall_total_ns = profile_obj.get("wall_total_ns")
        for mod in profile_obj.get("modules", []):
            mid = mod.get("module_id")
            if mid == "com.core.tree-support-planner":
                scopes = {}
                for sc in mod.get("scopes", []):
                    scopes[sc["scope"]] = {
                        "calls": sc["calls"],
                        "fuel": sc["total_fuel"],
                        "wall_ns": sc["total_wall_ns"],
                    }
                planner_prof = {
                    "total_fuel": mod["total_fuel"],
                    "self_fuel": mod["self_fuel"],
                    "total_wall_ns": mod["total_wall_ns"],
                    "self_wall_ns": mod["self_wall_ns"],
                    "scopes": scopes,
                }
            elif mid == "host:native":
                scopes = {}
                for sc in mod.get("scopes", []):
                    scopes[sc["scope"]] = {
                        "calls": sc["calls"],
                        "wall_ns": sc["total_wall_ns"],
                    }
                host_native_prof = {
                    "calls": mod["calls"],
                    "total_wall_ns": mod["total_wall_ns"],
                    "scopes": scopes,
                }

    orig_pts = sums["orig_clip_pts"]
    win_pts = sums["win_clip_pts"]
    queries = sums["regions_total"] - sums["outer_gated"]
    clip_pt_red_pct = (
        (orig_pts - win_pts) / orig_pts * 100.0 if orig_pts > 0 else 0.0
    )
    req_red_pct = (
        (queries - sums["clip_requests"]) / queries * 100.0 if queries > 0 else 0.0
    )

    return {
        "tile_units": tile_units,
        "tiling": tiling,
        "batches": batches,
        "degraded": degraded,
        "fatal_error_count": fatal,
        "slice_elapsed_ms": slice_elapsed_ms,
        "planner_elapsed_ms": planner_elapsed_ms,
        "counters": sums,
        "derived": {
            "phase1_queries_after_outer_gate": queries,
            "clip_point_reduction_pct": clip_pt_red_pct,
            "clip_point_remaining_pct": (win_pts / orig_pts * 100.0)
            if orig_pts > 0
            else 0.0,
            "request_reduction_pct": req_red_pct,
            "mean_orig_clip_pts_per_query": (orig_pts / queries) if queries > 0 else 0.0,
            "mean_win_clip_pts_per_query": (win_pts / queries) if queries > 0 else 0.0,
            "mean_win_clip_pts_per_sent_req": (win_pts / sums["clip_requests"])
            if sums["clip_requests"] > 0
            else 0.0,
        },
        "profile": {
            "fuel_total": fuel_total,
            "wall_total_ns": wall_total_ns,
            "planner": planner_prof,
            "host_native": host_native_prof,
        },
    }


def reduce_probe():
    probe_dir = os.path.join(RAW, "probe")
    captures = [
        ("base-classic-on", "baseline"),
        ("base-classic-on", "tile-1mm"),
        ("base-classic-on", "tile-2mm"),
        ("base-classic-on", "tile-5mm"),
        ("base-classic-on", "tile-10mm"),
        ("base-arachne-on", "baseline"),
        ("base-arachne-on", "tile-2mm"),
        ("benchy-classic-on", "baseline"),
        ("benchy-classic-on", "tile-2mm"),
        ("benchy-arachne-on", "baseline"),
        ("benchy-arachne-on", "tile-2mm"),
    ]
    out = {
        "schema": 1,
        "note": (
            "Attribution only (`--profile --instrument-stderr` with `probe.patch`). "
            "Fuel is deterministic executed guest instructions; wall under --profile "
            "is inflated by mark overhead and `host:native` sums concurrent worker "
            "threads. Never treat --profile wall as a gate measurement."
        ),
        "cells": {},
    }
    for cell, arm in captures:
        path = os.path.join(probe_dir, f"{cell}__{arm}.stderr.txt")
        if not os.path.exists(path):
            raise SystemExit(f"missing probe file: {path}")
        out["cells"].setdefault(cell, {})[arm] = parse_probe_file(path)
    return out


def reduce_route(ab, accel, probe, t174, t173):
    """Cross-window indicative route composition against t174 quiet board
    and comparison vs t173's host-only phase-1 ceiling."""
    rows = {}
    for cell in CELLS:
        t174_cell = t174["cells"][cell]
        r_ord = t174_cell["ratio_ordinary_over_orca"]
        r_acc = t174_cell["ratio_accelerated_over_orca"]
        req_ord = t174_cell["required_cut_ordinary_pct"]
        req_acc = t174_cell["required_cut_accelerated_pct"]
        s_ord = ab[cell]["wall"]["median_saving_pct"]
        s_acc = accel[cell]["wall"]["median_saving_pct"]

        if cell in SUPPORTS_ON:
            est_ord = r_ord * (1.0 - s_ord / 100.0)
            est_acc = r_acc * (1.0 - s_acc / 100.0)
            cut_closed_ord = (s_ord / req_ord * 100.0) if req_ord > 0 else None
            cut_closed_acc = (s_acc / req_acc * 100.0) if req_acc > 0 else None
            t173_ceil = t173["prices"][cell]["prices"]["phase1_bbox_window_ceiling"]
            t173_p3 = t173["prices"][cell]["prices"]["phase3_union_free_ceiling"]
            p2 = probe["cells"][cell]["tile-2mm"]
            rem_clip_thread_ms = (
                p2["profile"]["host_native"]["scopes"]["polygon_ops::clip_polygons"][
                    "wall_ns"
                ]
                / 1e6
            )
            p3_share_of_rem = t173_p3["thread_ms"] / rem_clip_thread_ms * 100.0
            rows[cell] = {
                "supports": "on",
                "t174_ratio_ordinary": r_ord,
                "t174_required_cut_ordinary_pct": req_ord,
                "t175_wall_saving_ordinary_pct": s_ord,
                "est_post_t175_ratio_ordinary": est_ord,
                "est_post_t175_required_cut_ordinary_pct": (1.0 - 1.0 / est_ord) * 100.0,
                "share_of_t174_cut_closed_ordinary_pct": cut_closed_ord,
                "t174_ratio_accelerated": r_acc,
                "t174_required_cut_accelerated_pct": req_acc,
                "t175_wall_saving_accelerated_pct": s_acc,
                "est_post_t175_ratio_accelerated": est_acc,
                "share_of_t174_cut_closed_accelerated_pct": cut_closed_acc,
                "t173_phase1_ceiling_wall_pct": t173_ceil["pct_of_cell_wall"],
                "t173_phase1_ceiling_cut_pct": t173_ceil["pct_of_required_cut"],
                "t173_phase1_ceiling_remaining_pts_pct": t173_ceil[
                    "windowed_edge_share"
                ]
                * 100.0,
                "t175_realized_remaining_pts_pct": p2["derived"][
                    "clip_point_remaining_pct"
                ],
                "t175_remaining_host_native_clip_thread_ms": rem_clip_thread_ms,
                "t173_phase3_union_thread_ms": t173_p3["thread_ms"],
                "est_phase3_share_of_remaining_host_clip_pct": p3_share_of_rem,
            }
        else:
            rows[cell] = {
                "supports": "off",
                "t174_ratio_ordinary": r_ord,
                "t174_ratio_accelerated": r_acc,
                "t175_wall_saving_ordinary_pct": s_ord,
                "t175_wall_saving_accelerated_pct": s_acc,
                "est_post_t175_ratio_ordinary": r_ord,
                "est_post_t175_ratio_accelerated": r_acc,
                "note": "supports-off control; phase 1 does not run (structurally inert)",
            }
    return {
        "schema": 1,
        "note": (
            "ESTIMATE ONLY (cross-window composition of t174 quiet-board official "
            "ratios with t175 paired savings). Not a three-tool scoreboard ratio of record."
        ),
        "cells": rows,
    }


def main():
    t172_ref = read_json(T172_REF_PATH)
    t172_active = t172_ref["active_set"]
    t173_arith = read_json(T173_ARITH_PATH)
    t174_summary = read_json(T174_SUMMARY_PATH)

    # 1. Sweep
    sweep = reduce_sweep(t172_active)

    # 2. Attribution probe
    probe = reduce_probe()

    # 3. Paired A/B, null control, accelerated corroboration
    ab_rows = read_rows(os.path.join(RAW, "ab", "ab", "rows.csv"))
    null_rows = read_rows(os.path.join(RAW, "ab", "null", "rows.csv"))
    accel_rows = read_rows(os.path.join(RAW, "ab", "accel", "rows.csv"))

    wall_pairs = pair_savings(ab_rows, "wall_s")
    cpu_pairs = pair_savings(ab_rows, "cpu_s")
    summary = {}
    for cell in CELLS:
        summary[cell] = {
            "wall": cell_stats(wall_pairs[cell]),
            "cpu": cell_stats(cpu_pairs[cell]),
        }
    null = null_band(null_rows)
    verdicts = gate_t137(summary, null)
    summary["_gate"] = verdicts
    summary["_meta"] = {
        b: read_json(os.path.join(RAW, "ab", b, "batch_meta.json"))
        for b in ("ab", "null", "accel")
    }

    accel = {}
    aw = pair_savings(accel_rows, "wall_s")
    ac = pair_savings(accel_rows, "cpu_s")
    for cell in CELLS:
        accel[cell] = {"wall": cell_stats(aw[cell]), "cpu": cell_stats(ac[cell])}

    # 4. 16-cell output currency & diagnostics
    manifest = read_json(os.path.join(RAW, "manifest.json"))
    out = {
        "schema": 1,
        "t172_active": t172_active,
        "manifest": manifest,
        "cells": {},
        "accelerated": {},
        "diagnostics": {
            "total_runs_checked": 0,
            "nonzero_exit_runs": 0,
            "degraded_runs": 0,
            "fatal_error_runs": 0,
            "code_1200_total": 0,
        },
    }
    for row in read_rows(os.path.join(RAW, "output", "ref", "rows.csv")):
        diag = inspect_jsonl_stderr(row["stderr_path"])
        out["diagnostics"]["total_runs_checked"] += 1
        if int(row["exit_code"]) != 0:
            out["diagnostics"]["nonzero_exit_runs"] += 1
        if diag["degraded"] is not False:
            out["diagnostics"]["degraded_runs"] += 1
        if diag["fatal_error_count"] not in (0, None):
            out["diagnostics"]["fatal_error_runs"] += 1
        out["diagnostics"]["code_1200_total"] += diag["code_1200_count"]
        out["cells"].setdefault(row["cell"], {})[row["arm"]] = {
            "sha256": row["gcode_sha256"],
            "bytes": int(row["gcode_bytes"]),
            "exit_code": int(row["exit_code"]),
            "degraded": diag["degraded"],
            "code_1200_count": diag["code_1200_count"],
        }
    for row in read_rows(os.path.join(RAW, "output", "accel", "rows.csv")):
        diag = inspect_jsonl_stderr(row["stderr_path"])
        out["diagnostics"]["total_runs_checked"] += 1
        if int(row["exit_code"]) != 0:
            out["diagnostics"]["nonzero_exit_runs"] += 1
        if diag["degraded"] is not False:
            out["diagnostics"]["degraded_runs"] += 1
        if diag["fatal_error_count"] not in (0, None):
            out["diagnostics"]["fatal_error_runs"] += 1
        out["diagnostics"]["code_1200_total"] += diag["code_1200_count"]
        out["accelerated"].setdefault(row["cell"], {})[row["arm"]] = row["gcode_sha256"]

    # Also verify zero degradation/code-1200 across all 80+48+48 timed A/B/null/accel logs
    for bname, brows in (("ab", ab_rows), ("null", null_rows), ("accel", accel_rows)):
        bdir = os.path.join(RAW, "ab", bname)
        for row in brows:
            out["diagnostics"]["total_runs_checked"] += 1
            if int(row["exit_code"]) != 0:
                out["diagnostics"]["nonzero_exit_runs"] += 1
            sp = os.path.join(
                bdir, f"p{row['pair']}__{row['cell']}__{row['side']}.stderr.jsonl"
            )
            if os.path.exists(sp):
                diag = inspect_jsonl_stderr(sp)
                if diag["degraded"] is not False:
                    out["diagnostics"]["degraded_runs"] += 1
                if diag["fatal_error_count"] not in (0, None):
                    out["diagnostics"]["fatal_error_runs"] += 1
                out["diagnostics"]["code_1200_total"] += diag["code_1200_count"]

    # 5. Route composition
    route = reduce_route(summary, accel, probe, t174_summary, t173_arith)

    for fname, doc in (
        ("sweep.json", sweep),
        ("probe.json", probe),
        ("ab.json", summary),
        ("null.json", null),
        ("accel.json", accel),
        ("output.json", out),
        ("route.json", route),
    ):
        write_json(fname, doc)

    print("=== SWEEP (base-classic-on, chosen: %s) ===" % sweep["chosen_arm"])
    for arm in SWEEP_ARMS:
        a = sweep["arms"][arm]
        print(
            "%-12s wall=%7.3fs (%+6.2f%%) cpu=%7.2fs (%+6.2f%%) sha=%s"
            % (
                arm,
                a["wall_median_s"],
                a["wall_saving_pct"],
                a["cpu_median_s"],
                a["cpu_saving_pct"],
                str(a["gcode_sha256"])[:8],
            )
        )

    print("\n=== PAIRED A/B (5 pairs/cell, ordinary) ===")
    print(
        "%-22s %8s %8s %8s %8s %7s %5s %7s  %s"
        % ("cell", "base_s", "cand_s", "wall%", "cpu%", "floorx", "wins", "null%", "verdict")
    )
    for cell in CELLS:
        w = summary[cell]["wall"]
        c = summary[cell]["cpu"]
        n = null[cell]
        print(
            "%-22s %8.3f %8.3f %+8.2f %+8.2f %7.2f %5d %7.2f  %s"
            % (
                cell,
                w["baseline_median_s"],
                w["candidate_median_s"],
                w["median_saving_pct"],
                c["median_saving_pct"],
                w["floor_multiple"],
                w["wins"],
                n["wall_any_pair_pct"],
                verdicts[cell],
            )
        )

    print("\n=== ACCELERATED CORROBORATION (3 pairs/cell) ===")
    for cell in CELLS:
        w = accel[cell]["wall"]
        c = accel[cell]["cpu"]
        print(
            "%-22s %8.3f %8.3f %+8.2f %+8.2f wins=%d/3"
            % (
                cell,
                w["baseline_median_s"],
                w["candidate_median_s"],
                w["median_saving_pct"],
                c["median_saving_pct"],
                w["wins"],
            )
        )

    print("\n=== PROBE SUMMARY (tile-2mm vs baseline) ===")
    for cell in SUPPORTS_ON:
        pb = probe["cells"][cell]["baseline"]
        p2 = probe["cells"][cell]["tile-2mm"]
        b_sc = pb["profile"]["planner"]["scopes"]
        c_sc = p2["profile"]["planner"]["scopes"]
        print(
            "%-20s req=%d->%d (empty=%d cov=%d cache=%d) pts=%d->%d (-%.2f%%) "
            "build=%.1fms query=%.1fms p1clip=%.1f->%.1fms host_clip=%.2f->%.2fs"
            % (
                cell,
                pb["counters"]["clip_requests"],
                p2["counters"]["clip_requests"],
                p2["counters"]["empty_gated"],
                p2["counters"]["covered_dropped"],
                p2["counters"]["cache_hits"],
                p2["counters"]["orig_clip_pts"],
                p2["counters"]["win_clip_pts"],
                p2["derived"]["clip_point_reduction_pct"],
                c_sc["t175::tile_index_build"]["wall_ns"] / 1e6,
                c_sc["t175::tile_index_query"]["wall_ns"] / 1e6,
                b_sc["t175::phase1_clip_batches"]["wall_ns"] / 1e6,
                c_sc["t175::phase1_clip_batches"]["wall_ns"] / 1e6,
                pb["profile"]["host_native"]["scopes"]["polygon_ops::clip_polygons"][
                    "wall_ns"
                ]
                / 1e9,
                p2["profile"]["host_native"]["scopes"]["polygon_ops::clip_polygons"][
                    "wall_ns"
                ]
                / 1e9,
            )
        )

    print("\n=== OUTPUT & DIAGNOSTICS ===")
    d = out["diagnostics"]
    print(
        "runs_checked=%d nonzero_exit=%d degraded=%d fatal=%d code_1200=%d"
        % (
            d["total_runs_checked"],
            d["nonzero_exit_runs"],
            d["degraded_runs"],
            d["fatal_error_runs"],
            d["code_1200_total"],
        )
    )
    bad_base = [
        c
        for c in CELLS
        if out["cells"][c]["baseline-ordinary"]["sha256"] != t172_active[c]
    ]
    moved = [
        c
        for c in CELLS
        if out["cells"][c]["candidate-ordinary"]["sha256"] != t172_active[c]
    ]
    print("baseline matches t172: %d/8" % (8 - len(bad_base)))
    print("candidate byte-identical to t172: %d/8 (moved: %s)" % (8 - len(moved), ", ".join(moved) or "none"))
    return 1 if bad_base or d["nonzero_exit_runs"] or d["degraded_runs"] or d["code_1200_total"] else 0


if __name__ == "__main__":
    sys.exit(main())

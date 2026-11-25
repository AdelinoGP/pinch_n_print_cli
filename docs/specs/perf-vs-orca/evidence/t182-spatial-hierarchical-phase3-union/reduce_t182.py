#!/usr/bin/env python3
"""t182 reduce: base-classic-on parameter sweep ({worker,nested} x k in {4,8,16}),
standing t137 paired A/B + null control + accelerated corroboration, 16-cell
output currency & diagnostics, and cross-window route composition.

Reads the gitignored raw tree under `target/t182/` plus sibling committed
reduced artifacts (`t175/reference.json`, `t175/reduced/route.json`,
`t178/reduced/arithmetic.json`), and writes the in-repo reduced set under
`reduced/`:
  - `sweep.json`
  - `ab.json`
  - `null.json`
  - `accel.json`
  - `output.json`
  - `route.json`

Usage:  python reduce_t182.py
"""

import csv
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
RAW = os.path.join(REPO, "target", "t182")
OUT = os.path.join(HERE, "reduced")

T175_REF_PATH = os.path.join(HERE, "..", "t175-tiled-collision-carve", "reference.json")
T175_ROUTE_PATH = os.path.join(HERE, "..", "t175-tiled-collision-carve", "reduced", "route.json")
T178_ARITH_PATH = os.path.join(HERE, "..", "t178-planner-phase3-union", "reduced", "arithmetic.json")

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
SWEEP_ARMS = [
    "baseline",
    "worker-k4",
    "worker-k8",
    "worker-k16",
    "nested-k4",
    "nested-k8",
    "nested-k16",
]
FLOOR_PCT = 0.80


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


def gate_t137_literal(summary, null):
    """Literal standing t137 4-cell planner gate:
    - supports-on: >= 0.80% paired median wall saving, > null median wall, CPU decreasing.
    - supports-off: structurally inert, no regression beyond max(0.80%, null any-pair)."""
    verdicts = {}
    for cell in CELLS:
        w = summary[cell]["wall"]
        c = summary[cell]["cpu"]
        n = null[cell]
        if cell in SUPPORTS_ON:
            clears_floor = w["median_saving_pct"] >= FLOOR_PCT
            above_null_med = w["median_saving_pct"] > n["wall_median_pct"]
            cpu_ok = c["median_saving_pct"] > 0.0
            if clears_floor and above_null_med and cpu_ok:
                verdicts[cell] = "PASS"
            elif not clears_floor:
                verdicts[cell] = "SUB-FLOOR"
            elif not above_null_med:
                verdicts[cell] = "INSIDE-NULL-MEDIAN"
            else:
                verdicts[cell] = "CPU-CONTRADICTED"
        else:
            bound = max(FLOOR_PCT, n["wall_any_pair_pct"])
            if w["median_saving_pct"] >= -bound:
                verdicts[cell] = "INERT-PASS"
            else:
                verdicts[cell] = "REGRESSION"
    return verdicts


def gate_t175_strict_any_pair(summary, null):
    """Stricter t175 any-pair null lens: additionally requires median wall saving
    strictly above the single worst null pair (`wall_any_pair_pct`)."""
    verdicts = {}
    for cell in CELLS:
        w = summary[cell]["wall"]
        c = summary[cell]["cpu"]
        n = null[cell]
        if cell in SUPPORTS_ON:
            clears_floor = w["median_saving_pct"] >= FLOOR_PCT
            outside_any = w["median_saving_pct"] > n["wall_any_pair_pct"]
            cpu_ok = c["median_saving_pct"] > 0.0
            if clears_floor and outside_any and cpu_ok:
                verdicts[cell] = "PASS"
            elif not clears_floor:
                verdicts[cell] = "SUB-FLOOR"
            elif not outside_any:
                verdicts[cell] = "INSIDE-NULL-ANY-PAIR"
            else:
                verdicts[cell] = "CPU-CONTRADICTED"
        else:
            bound = max(FLOOR_PCT, n["wall_any_pair_pct"])
            if w["median_saving_pct"] >= -bound:
                verdicts[cell] = "INERT-PASS"
            else:
                verdicts[cell] = "REGRESSION"
    return verdicts


def inspect_jsonl_stderr(path):
    degraded = None
    fatal = None
    non_fatal = None
    code_1200 = 0
    slice_elapsed_ms = None
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
    }


def reduce_sweep(t175_active):
    rows = read_rows(os.path.join(RAW, "sweep", "sweep1", "rows.csv"))
    meta = read_json(os.path.join(RAW, "sweep", "sweep1", "batch_meta.json"))
    sweep_audit_path = os.path.join(OUT, "sweep_audit.json")
    sweep_audit = read_json(sweep_audit_path) if os.path.exists(sweep_audit_path) else None

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
        exes = sorted({r["exe_sha256"] for r in arows})
        exits = sorted({int(r["exit_code"]) for r in arows})
        diag_bad = 0
        for r in arows:
            d = inspect_jsonl_stderr(r["stderr_path"])
            if d["degraded"] is not False or d["fatal_error_count"] not in (0, None) or d["code_1200_count"] != 0:
                diag_bad += 1
        entry = {
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
            "exe_sha256": exes[0] if len(exes) == 1 else exes,
            "exit_codes": exits,
            "diag_issues": diag_bad,
            "matches_t175": (len(shas) == 1 and shas[0] == t175_active["base-classic-on"]),
        }
        if sweep_audit and arm in sweep_audit["arms"]:
            sa = sweep_audit["arms"][arm]
            entry["audit_summary"] = {
                "delta_bytes": sa["delta_bytes"],
                "delta_lines": sa["delta_lines"],
                "total_extruding_delta_mm": sa["total_extruding_mm"]["delta_mm"],
                "total_extruding_delta_pct": sa["total_extruding_mm"]["delta_pct"],
                "moved_sections": sa["moved_sections"],
                "part_sections_moved": sa["part_sections_moved"],
                "part_motion_sha256_equal": sa["part_motion_sha256_equal"],
                "non_support_xy_mismatches": sa["part_geometry_audit"]["non_support_xy_mismatches"],
            }
        arms[arm] = entry

    base_wall = arms["baseline"]["wall_median_s"]
    base_cpu = arms["baseline"]["cpu_median_s"]
    for arm in SWEEP_ARMS:
        a = arms[arm]
        a["wall_saving_pct"] = (base_wall - a["wall_median_s"]) / base_wall * 100.0
        a["cpu_saving_pct"] = (base_cpu - a["cpu_median_s"]) / base_cpu * 100.0

    return {
        "schema": 1,
        "cell": "base-classic-on",
        "selection_rule": (
            "Lowest measured median wall across {worker, nested} x k in {4, 8, 16}, "
            "preferring k = 8 over k = 4 when within repeat noise for sub-micron Hausdorff "
            "(worker-k8 is 0.057 s / 0.23% from nested-k4 within a 0.337-1.307 s repeat spread, "
            "beats nested-k8 on wall and CPU, beats nested-k4 on CPU, and cuts Support path delta "
            "5x from -1.195 mm to -0.242 mm)."
        ),
        "chosen_arm": "worker-k8",
        "meta": meta,
        "arms": arms,
    }


def reduce_route(ab, accel, t175_route, t178_arith):
    """Cross-window indicative route composition: t174 x t175 x t182 (ESTIMATE ONLY)."""
    rows = {}
    for cell in CELLS:
        r175 = t175_route["cells"][cell]
        r_ord = r175["est_post_t175_ratio_ordinary"]
        r_acc = r175["est_post_t175_ratio_accelerated"]
        s_ord = ab[cell]["wall"]["median_saving_pct"]
        s_acc = accel[cell]["wall"]["median_saving_pct"]
        if cell in SUPPORTS_ON:
            req_ord = r175["est_post_t175_required_cut_ordinary_pct"]
            req_acc = (1.0 - 1.0 / r_acc) * 100.0
            est_ord = r_ord * (1.0 - s_ord / 100.0)
            est_acc = r_acc * (1.0 - s_acc / 100.0)
            cut_closed_ord = (s_ord / req_ord * 100.0) if req_ord > 0 else None
            cut_closed_acc = (s_acc / req_acc * 100.0) if req_acc > 0 else None
            t178_cell = t178_arith["prod_cells"][cell]
            rows[cell] = {
                "supports": "on",
                "est_post_t175_ratio_ordinary": r_ord,
                "est_post_t175_required_cut_ordinary_pct": req_ord,
                "t182_wall_saving_ordinary_pct": s_ord,
                "est_post_t182_ratio_ordinary": est_ord,
                "est_post_t182_required_cut_ordinary_pct": max(0.0, (1.0 - 1.0 / est_ord) * 100.0),
                "share_of_post_t175_cut_closed_ordinary_pct": cut_closed_ord,
                "est_post_t175_ratio_accelerated": r_acc,
                "est_post_t175_required_cut_accelerated_pct": req_acc,
                "t182_wall_saving_accelerated_pct": s_acc,
                "est_post_t182_ratio_accelerated": est_acc,
                "est_post_t182_required_cut_accelerated_pct": max(0.0, (1.0 - 1.0 / est_acc) * 100.0),
                "share_of_post_t175_cut_closed_accelerated_pct": cut_closed_acc,
                "t178_clean_phase3_wall_ms": t178_cell["prod_phase3_wall_ms_median"],
                "t178_clean_phase3_pct_of_cell": t178_cell["prod_phase3_share_of_cell_pct"],
            }
        else:
            rows[cell] = {
                "supports": "off",
                "est_post_t175_ratio_ordinary": r_ord,
                "est_post_t175_ratio_accelerated": r_acc,
                "t182_wall_saving_ordinary_pct": s_ord,
                "t182_wall_saving_accelerated_pct": s_acc,
                "est_post_t182_ratio_ordinary": r_ord,
                "est_post_t182_ratio_accelerated": r_acc,
                "note": "supports-off control; clip_polygons_batch never runs (structurally inert)",
            }
    return {
        "schema": 1,
        "note": (
            "ESTIMATE ONLY (cross-window composition of t174 quiet-board official "
            "ratios x t175 paired savings x t182 paired savings). Not a three-tool "
            "scoreboard ratio of record."
        ),
        "cells": rows,
    }


def main():
    t175_ref = read_json(T175_REF_PATH)
    t175_active = t175_ref["active_set"]
    t175_route = read_json(T175_ROUTE_PATH)
    t178_arith = read_json(T178_ARITH_PATH)

    sweep = reduce_sweep(t175_active)

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
    summary["_gate_t137"] = gate_t137_literal(summary, null)
    summary["_gate_strict_any_pair_null"] = gate_t175_strict_any_pair(summary, null)
    summary["_meta"] = {
        b: read_json(os.path.join(RAW, "ab", b, "batch_meta.json"))
        for b in ("ab", "null", "accel")
    }

    accel = {}
    aw = pair_savings(accel_rows, "wall_s")
    ac = pair_savings(accel_rows, "cpu_s")
    for cell in CELLS:
        accel[cell] = {"wall": cell_stats(aw[cell]), "cpu": cell_stats(ac[cell])}

    manifest = read_json(os.path.join(RAW, "manifest.json"))
    out = {
        "schema": 1,
        "t175_active": t175_active,
        "manifest": manifest,
        "cells": {},
        "accelerated": {},
        "diagnostics": {
            "total_runs_checked": 0,
            "nonzero_exit_runs": 0,
            "degraded_runs": 0,
            "fatal_error_runs": 0,
            "non_fatal_error_runs": 0,
            "code_1200_total": 0,
        },
    }

    def check_diag(row):
        diag = inspect_jsonl_stderr(row["stderr_path"])
        out["diagnostics"]["total_runs_checked"] += 1
        if int(row["exit_code"]) != 0:
            out["diagnostics"]["nonzero_exit_runs"] += 1
        if diag["degraded"] is not False:
            out["diagnostics"]["degraded_runs"] += 1
        if diag["fatal_error_count"] not in (0, None):
            out["diagnostics"]["fatal_error_runs"] += 1
        if diag["non_fatal_error_count"] not in (0, None):
            out["diagnostics"]["non_fatal_error_runs"] += 1
        out["diagnostics"]["code_1200_total"] += diag["code_1200_count"]
        return diag

    for row in read_rows(os.path.join(RAW, "output", "ref", "rows.csv")):
        diag = check_diag(row)
        out["cells"].setdefault(row["cell"], {})[row["arm"]] = {
            "sha256": row["gcode_sha256"],
            "bytes": int(row["gcode_bytes"]),
            "exit_code": int(row["exit_code"]),
            "degraded": diag["degraded"],
            "code_1200_count": diag["code_1200_count"],
        }
    for row in read_rows(os.path.join(RAW, "output", "accel", "rows.csv")):
        check_diag(row)
        out["accelerated"].setdefault(row["cell"], {})[row["arm"]] = row["gcode_sha256"]

    for bname, brows in (("ab", ab_rows), ("null", null_rows), ("accel", accel_rows)):
        for row in brows:
            check_diag(row)
    for row in read_rows(os.path.join(RAW, "sweep", "sweep1", "rows.csv")):
        check_diag(row)

    route = reduce_route(summary, accel, t175_route, t178_arith)

    for fname, doc in (
        ("sweep.json", sweep),
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
        "%-22s %7s %7s %7s %7s %6s %5s %6s %6s  %-10s  %s"
        % ("cell", "base_s", "cand_s", "wall%", "cpu%", "flrx", "wins", "n_med", "n_any", "t137", "strict_any")
    )
    for cell in CELLS:
        w = summary[cell]["wall"]
        c = summary[cell]["cpu"]
        n = null[cell]
        print(
            "%-22s %7.3f %7.3f %+7.2f %+7.2f %6.2f %5d %6.2f %6.2f  %-10s  %s"
            % (
                cell,
                w["baseline_median_s"],
                w["candidate_median_s"],
                w["median_saving_pct"],
                c["median_saving_pct"],
                w["floor_multiple"],
                w["wins"],
                n["wall_median_pct"],
                n["wall_any_pair_pct"],
                summary["_gate_t137"][cell],
                summary["_gate_strict_any_pair_null"][cell],
            )
        )

    print("\n=== ACCELERATED CORROBORATION (3 pairs/cell) ===")
    for cell in CELLS:
        w = accel[cell]["wall"]
        c = accel[cell]["cpu"]
        print(
            "%-22s %7.3f %7.3f %+7.2f %+7.2f wall_wins=%d/3 cpu_wins=%d/3"
            % (
                cell,
                w["baseline_median_s"],
                w["candidate_median_s"],
                w["median_saving_pct"],
                c["median_saving_pct"],
                w["wins"],
                c["wins"],
            )
        )

    print("\n=== ESTIMATE ONLY ROUTE (t174 x t175 x t182) ===")
    for cell in SUPPORTS_ON:
        rc = route["cells"][cell]
        print(
            "%-22s ord: %.4fx -> %.4fx (%.1f%% cut closed) | acc: %.4fx -> %.4fx"
            % (
                cell,
                rc["est_post_t175_ratio_ordinary"],
                rc["est_post_t182_ratio_ordinary"],
                rc["share_of_post_t175_cut_closed_ordinary_pct"],
                rc["est_post_t175_ratio_accelerated"],
                rc["est_post_t182_ratio_accelerated"],
            )
        )

    print("\n=== OUTPUT & DIAGNOSTICS ===")
    d = out["diagnostics"]
    print(
        "runs_checked=%d nonzero_exit=%d degraded=%d fatal=%d non_fatal=%d code_1200=%d"
        % (
            d["total_runs_checked"],
            d["nonzero_exit_runs"],
            d["degraded_runs"],
            d["fatal_error_runs"],
            d["non_fatal_error_runs"],
            d["code_1200_total"],
        )
    )
    bad_base = [
        c
        for c in CELLS
        if out["cells"][c]["baseline-ordinary"]["sha256"] != t175_active[c]
    ]
    bad_acc = [
        c
        for c in CELLS
        if out["accelerated"][c]["candidate-accelerated"]
        != out["cells"][c]["candidate-ordinary"]["sha256"]
    ]
    moved = [
        c
        for c in CELLS
        if out["cells"][c]["candidate-ordinary"]["sha256"] != t175_active[c]
    ]
    print("baseline matches t175: %d/8" % (8 - len(bad_base)))
    print("candidate ord == acc:  %d/8" % (8 - len(bad_acc)))
    print("candidate byte-identical to t175: %d/8 (moved: %s)" % (8 - len(moved), ", ".join(moved) or "none"))
    return 1 if bad_base or bad_acc or d["nonzero_exit_runs"] or d["degraded_runs"] or d["fatal_error_runs"] or d["code_1200_total"] else 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""t183 reduce: literal standing all-8-cell paired ordinary + null + accelerated
gate, 16/16 byte-identical output & diagnostics against t182's active currency,
deterministic before/after guest fuel (--profile), and cross-window ESTIMATE ONLY
route composition.

Reads the gitignored raw tree under `target/t183/` plus committed sibling
artifacts (`t182/reference.json`, `t182/reduced/route.json`), and writes the
in-repo reduced set under `reduced/`:
  - `ab.json`
  - `null.json`
  - `accel.json`
  - `output.json`
  - `fuel.json`
  - `route.json`

Usage:  python reduce_t183.py
"""

import csv
import json
import os
import re
import statistics

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
RAW = os.path.join(REPO, "target", "t183")
OUT = os.path.join(HERE, "reduced")

T182_REF_PATH = os.path.join(
    HERE, "..", "t182-spatial-hierarchical-phase3-union", "reference.json"
)
T182_ROUTE_PATH = os.path.join(
    HERE, "..", "t182-spatial-hierarchical-phase3-union", "reduced", "route.json"
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
FLOOR_PCT = 0.80

RE_TOTAL_FUEL = re.compile(r"^total:\s+([\d,]+)\s+fuel\s+across\s+(\d+)\s+module\(s\)")
RE_MOD_FUEL = re.compile(
    r"^(com\.core\.[a-z0-9\-]+)\s+fuel\s+([\d.]+)%\s+([\d,]+)\s+calls\s+(\d+)"
)


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
    b_vals = [p["baseline_s"] for p in savings]
    c_vals = [p["candidate_s"] for p in savings]
    med = statistics.median(vals)
    return {
        "pairs": len(vals),
        "median_saving_pct": med,
        "mean_saving_pct": statistics.fmean(vals),
        "min_saving_pct": min(vals),
        "max_saving_pct": max(vals),
        "wins": sum(1 for v in vals if v > 0.0),
        "strict_separation": max(c_vals) < min(b_vals),
        "baseline_median_s": statistics.median(b_vals),
        "candidate_median_s": statistics.median(c_vals),
        "baseline_min_s": min(b_vals),
        "baseline_max_s": max(b_vals),
        "candidate_min_s": min(c_vals),
        "candidate_max_s": max(c_vals),
        "floor_multiple": med / FLOOR_PCT,
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


def gate_literal_8cell(summary, null):
    """Literal standing all-8-cell 0.80 % gate:
    every cell must clear >= 0.80 % paired median wall saving, exceed null median
    wall band, and show lower process CPU."""
    verdicts = {}
    for cell in CELLS:
        w = summary[cell]["wall"]
        c = summary[cell]["cpu"]
        n = null[cell]
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
    return verdicts


def gate_strict_any_pair_null(summary, null):
    """Stricter any-pair null lens: additionally requires median wall saving
    strictly above the single worst null pair (`wall_any_pair_pct`)."""
    verdicts = {}
    for cell in CELLS:
        w = summary[cell]["wall"]
        c = summary[cell]["cpu"]
        n = null[cell]
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


def parse_profile_fuel(path):
    total_fuel = None
    module_count = None
    modules = {}
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            s = line.strip()
            m_tot = RE_TOTAL_FUEL.match(s)
            if m_tot:
                total_fuel = int(m_tot.group(1).replace(",", ""))
                module_count = int(m_tot.group(2))
                continue
            m_mod = RE_MOD_FUEL.match(s)
            if m_mod:
                name = m_mod.group(1)
                fuel = int(m_mod.group(3).replace(",", ""))
                calls = int(m_mod.group(4))
                modules[name] = {"fuel": fuel, "calls": calls}
    return {
        "total_fuel": total_fuel,
        "module_count": module_count,
        "modules": modules,
    }


def reduce_fuel(fuel_rows, t182_active, check_diag):
    by_cell = {}
    for row in fuel_rows:
        diag = check_diag(row)
        parsed = parse_profile_fuel(row["stderr_path"])
        by_cell.setdefault(row["cell"], {})[row["arm"]] = {
            "sha256": row["gcode_sha256"],
            "bytes": int(row["gcode_bytes"]),
            "matches_t182": row["gcode_sha256"] == t182_active[row["cell"]],
            "degraded": diag["degraded"],
            "code_1200_count": diag["code_1200_count"],
            "total_fuel": parsed["total_fuel"],
            "module_count": parsed["module_count"],
            "tree_support_fuel": parsed["modules"]["com.core.tree-support"]["fuel"],
            "tree_support_calls": parsed["modules"]["com.core.tree-support"]["calls"],
            "infill_linker_fuel": parsed["modules"]["com.core.infill-linker"]["fuel"],
            "infill_linker_calls": parsed["modules"]["com.core.infill-linker"]["calls"],
        }

    cells = {}
    for cell in CELLS:
        b = by_cell[cell]["baseline-ordinary"]
        c = by_cell[cell]["candidate-ordinary"]

        def metric(bval, cval):
            saved = bval - cval
            pct = (saved / bval * 100.0) if bval > 0 else 0.0
            mult = (bval / cval) if cval > 0 else 0.0
            return {
                "baseline": bval,
                "candidate": cval,
                "saved": saved,
                "cut_pct": pct,
                "speedup_x": mult,
            }

        cells[cell] = {
            "baseline_sha256": b["sha256"],
            "candidate_sha256": c["sha256"],
            "matches_t182": b["matches_t182"] and c["matches_t182"],
            "tree_support": metric(b["tree_support_fuel"], c["tree_support_fuel"]),
            "infill_linker": metric(b["infill_linker_fuel"], c["infill_linker_fuel"]),
            "total_guest_fuel": metric(b["total_fuel"], c["total_fuel"]),
            "tree_support_calls": b["tree_support_calls"],
            "infill_linker_calls": b["infill_linker_calls"],
        }

    return {
        "schema": 1,
        "note": (
            "Deterministic WASM guest fuel captured via --profile on ordinary baseline "
            "vs candidate (attribution only; wall time under --profile is never used)."
        ),
        "cells": cells,
    }


def reduce_route(ab, accel, t182_route):
    """Cross-window indicative route composition: t174 x t175 x t182 x t183 (ESTIMATE ONLY)."""
    rows = {}
    for cell in CELLS:
        r182 = t182_route["cells"][cell]
        r_ord = r182["est_post_t182_ratio_ordinary"]
        r_acc = r182["est_post_t182_ratio_accelerated"]
        s_ord = ab[cell]["wall"]["median_saving_pct"]
        s_acc = accel[cell]["wall"]["median_saving_pct"]
        est_ord = r_ord * (1.0 - s_ord / 100.0)
        est_acc = r_acc * (1.0 - s_acc / 100.0)
        req_before_ord = max(0.0, (1.0 - 1.0 / r_ord) * 100.0)
        req_after_ord = max(0.0, (1.0 - 1.0 / est_ord) * 100.0)
        req_before_acc = max(0.0, (1.0 - 1.0 / r_acc) * 100.0)
        req_after_acc = max(0.0, (1.0 - 1.0 / est_acc) * 100.0)
        cut_closed_ord = (s_ord / req_before_ord * 100.0) if req_before_ord > 0 else None
        cut_closed_acc = (s_acc / req_before_acc * 100.0) if req_before_acc > 0 else None
        rows[cell] = {
            "supports": r182["supports"],
            "est_post_t182_ratio_ordinary": r_ord,
            "est_post_t182_required_cut_ordinary_pct": req_before_ord,
            "t183_wall_saving_ordinary_pct": s_ord,
            "est_post_t183_ratio_ordinary": est_ord,
            "est_post_t183_required_cut_ordinary_pct": req_after_ord,
            "share_of_post_t182_cut_closed_ordinary_pct": cut_closed_ord,
            "est_post_t182_ratio_accelerated": r_acc,
            "est_post_t182_required_cut_accelerated_pct": req_before_acc,
            "t183_wall_saving_accelerated_pct": s_acc,
            "est_post_t183_ratio_accelerated": est_acc,
            "est_post_t183_required_cut_accelerated_pct": req_after_acc,
            "share_of_post_t182_cut_closed_accelerated_pct": cut_closed_acc,
        }
    return {
        "schema": 1,
        "note": (
            "ESTIMATE ONLY (cross-window composition of t174 quiet-board official "
            "ratios x t175 paired savings x t182 paired savings x t183 paired savings). "
            "Not a three-tool scoreboard ratio of record."
        ),
        "cells": rows,
    }


def main():
    t182_ref = read_json(T182_REF_PATH)
    t182_active = t182_ref["active_set"]
    t182_route = read_json(T182_ROUTE_PATH)

    ab_rows = read_rows(os.path.join(RAW, "ab", "ab", "rows.csv"))
    null_rows = read_rows(os.path.join(RAW, "ab", "null", "rows.csv"))
    accel_rows = read_rows(os.path.join(RAW, "ab", "accel", "rows.csv"))
    ref_rows = read_rows(os.path.join(RAW, "output", "ref", "rows.csv"))
    acc_out_rows = read_rows(os.path.join(RAW, "output", "accel", "rows.csv"))
    fuel_rows = read_rows(os.path.join(RAW, "fuel", "fuel", "rows.csv"))

    wall_pairs = pair_savings(ab_rows, "wall_s")
    cpu_pairs = pair_savings(ab_rows, "cpu_s")
    summary = {}
    for cell in CELLS:
        summary[cell] = {
            "wall": cell_stats(wall_pairs[cell]),
            "cpu": cell_stats(cpu_pairs[cell]),
        }
    null = null_band(null_rows)
    summary["_gate_literal_8cell"] = gate_literal_8cell(summary, null)
    summary["_gate_strict_any_pair_null"] = gate_strict_any_pair_null(summary, null)
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
        "t182_active": t182_active,
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
            "hash_mismatches": 0,
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
        if row["gcode_sha256"] != t182_active[row["cell"]]:
            out["diagnostics"]["hash_mismatches"] += 1
        return diag

    for row in ref_rows:
        diag = check_diag(row)
        out["cells"].setdefault(row["cell"], {})[row["arm"]] = {
            "sha256": row["gcode_sha256"],
            "bytes": int(row["gcode_bytes"]),
            "exit_code": int(row["exit_code"]),
            "degraded": diag["degraded"],
            "code_1200_count": diag["code_1200_count"],
        }
    for row in acc_out_rows:
        check_diag(row)
        out["accelerated"].setdefault(row["cell"], {})[row["arm"]] = row["gcode_sha256"]

    for row in ab_rows:
        check_diag(row)
    for row in null_rows:
        check_diag(row)
    for row in accel_rows:
        check_diag(row)

    fuel = reduce_fuel(fuel_rows, t182_active, check_diag)
    route = reduce_route(summary, accel, t182_route)

    write_json("ab.json", summary)
    write_json("null.json", null)
    write_json("accel.json", accel)
    write_json("output.json", out)
    write_json("fuel.json", fuel)
    write_json("route.json", route)

    print("Wrote reduced/*.json")
    print("Diagnostics:", out["diagnostics"])
    print("\n--- Ordinary 5-pair A/B + Null + Gate ---")
    for cell in CELLS:
        w = summary[cell]["wall"]
        c = summary[cell]["cpu"]
        n = null[cell]
        aw_s = accel[cell]["wall"]
        ac_s = accel[cell]["cpu"]
        print(
            f"{cell:20s} | ord wall {w['median_saving_pct']:+6.2f}% ({w['wins']}/{w['pairs']}, {w['floor_multiple']:5.2f}x, sep={w['strict_separation']}) "
            f"| cpu {c['median_saving_pct']:+6.2f}% ({c['wins']}/{c['pairs']}) "
            f"| null med/any {n['wall_median_pct']:4.2f}/{n['wall_any_pair_pct']:4.2f}% "
            f"| acc {aw_s['median_saving_pct']:+6.2f}%/{ac_s['median_saving_pct']:+6.2f}% "
            f"| lit={summary['_gate_literal_8cell'][cell]} strict={summary['_gate_strict_any_pair_null'][cell]}"
        )
    print("\n--- Deterministic Guest Fuel ---")
    for cell in CELLS:
        f = fuel["cells"][cell]
        print(
            f"{cell:20s} | TS: {f['tree_support']['baseline']:15,d} -> {f['tree_support']['candidate']:13,d} ({f['tree_support']['cut_pct']:+6.2f}%, {f['tree_support']['speedup_x']:5.2f}x) "
            f"| IL: {f['infill_linker']['baseline']:14,d} -> {f['infill_linker']['candidate']:12,d} ({f['infill_linker']['cut_pct']:+6.2f}%, {f['infill_linker']['speedup_x']:4.2f}x)"
        )
    print("\n--- Route (ESTIMATE ONLY: t174 x t175 x t182 x t183) ---")
    for cell in CELLS:
        r = route["cells"][cell]
        print(
            f"{cell:20s} | ord {r['est_post_t182_ratio_ordinary']:.4f}x -> {r['est_post_t183_ratio_ordinary']:.4f}x "
            f"| acc {r['est_post_t182_ratio_accelerated']:.4f}x -> {r['est_post_t183_ratio_accelerated']:.4f}x"
        )


if __name__ == "__main__":
    main()

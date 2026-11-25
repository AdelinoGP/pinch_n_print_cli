#!/usr/bin/env python3
"""t192 reduce: exact Zero-WIT `PostPass` acceleration (`PP-1 + PP-2 + PP-3`).

Literal standing all-8-cell 0.80 % gate across all 8 matrix cells, 16/16 ordinary
+ 16/16 accelerated byte-identical output and clean diagnostics against t182's
active currency, both host executables and guest module trees updated with zero
WIT edits, instrumented `PostPass` (`LayerFinalization`, `GCodeEmit`,
`GCodeSerialize`, total `PostPass`) stage reduction across all 8 cells,
deterministic `--profile` `LayerFinalization` guest fuel reduction on all 4
supports-on cells, and ESTIMATE ONLY route composed on `t191/reduced/route.json`.

Reads the gitignored raw tree `target/t192/` plus committed siblings
(`t182/reference.json`, `t191/reduced/route.json`) and writes `reduced/`.

Usage:  python reduce_t192.py
"""

import csv
import json
import os
import statistics

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
RAW = os.path.join(REPO, "target", "t192")
OUT = os.path.join(HERE, "reduced")
T182_REF_PATH = os.path.join(HERE, "..", "t182-spatial-hierarchical-phase3-union", "reference.json")
T191_ROUTE_PATH = os.path.join(HERE, "..", "t191-support-analysis-exact-accel", "reduced", "route.json")

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
PROFILE_CELLS = [cell for cell in CELLS if cell.endswith("-on")]
FLOOR_PCT = 0.80
POSTPASS_STAGES = (
    "PostPass::LayerFinalization",
    "PostPass::GCodeEmit",
    "PostPass::GCodePostProcess",
    "PostPass::GCodeSerialize",
)
LF_MODULES = (
    "com.core.wipe-tower",
    "com.core.overhang-classifier-default",
    "com.core.skirt-brim",
    "com.core.part-cooling",
)


def read_json(path):
    with open(path, encoding="utf-8-sig") as fh:
        return json.load(fh)


def read_rows(*parts):
    with open(os.path.join(RAW, *parts, "rows.csv"), encoding="utf-8-sig", newline="") as fh:
        return list(csv.DictReader(fh))


def has_batch(*parts):
    return os.path.exists(os.path.join(RAW, *parts, "rows.csv"))


def write_json(name, doc):
    os.makedirs(OUT, exist_ok=True)
    with open(os.path.join(OUT, name), "w", encoding="utf-8", newline="\n") as fh:
        json.dump(doc, fh, indent=2)
        fh.write("\n")


def paired(rows, value):
    by = {}
    for row in rows:
        if row.get("null_control") in ("True", "true", "1"):
            continue
        by.setdefault((row["cell"], row["batch"], int(row["pair"])), {})[row["side"]] = value(row)
    out = {}
    for (cell, batch, pair), sides in sorted(by.items()):
        if "baseline" in sides and "candidate" in sides:
            b, c = sides["baseline"], sides["candidate"]
            out.setdefault(cell, []).append(
                {
                    "batch": batch,
                    "pair": pair,
                    "baseline_s": b,
                    "candidate_s": c,
                    "saving_pct": (b - c) / b * 100.0 if b > 0.0 else 0.0,
                }
            )
    return out


def cell_stats(pairs):
    vals = [p["saving_pct"] for p in pairs]
    b_vals = [p["baseline_s"] for p in pairs]
    c_vals = [p["candidate_s"] for p in pairs]
    med = statistics.median(vals)
    return {
        "pairs": len(vals),
        "median_saving_pct": med,
        "wins": sum(1 for v in vals if v > 0.0),
        "strict_separation": max(c_vals) < min(b_vals),
        "baseline_median_s": statistics.median(b_vals),
        "candidate_median_s": statistics.median(c_vals),
        "floor_multiple": med / FLOOR_PCT,
        "pairs_detail": pairs,
    }


def null_band(rows):
    by = {}
    for row in rows:
        by.setdefault((row["cell"], row["batch"], int(row["pair"])), {})[row["side"]] = row
    out = {}
    for (cell, _b, _p), sides in sorted(by.items()):
        a, b = sides.get("baseline"), sides.get("baseline2")
        if a and b:
            d = out.setdefault(cell, {"wall": [], "cpu": []})
            d["wall"].append(abs(float(a["wall_s"]) - float(b["wall_s"])) / float(b["wall_s"]) * 100.0)
            d["cpu"].append(abs(float(a["cpu_s"]) - float(b["cpu_s"])) / float(b["cpu_s"]) * 100.0)
    return {
        cell: {
            "pairs": len(out[cell]["wall"]),
            "wall_median_pct": statistics.median(out[cell]["wall"]),
            "wall_any_pair_pct": max(out[cell]["wall"]),
            "cpu_median_pct": statistics.median(out[cell]["cpu"]),
            "cpu_any_pair_pct": max(out[cell]["cpu"]),
        }
        for cell in CELLS
        if cell in out
    }


def verdicts(summary, null):
    """Literal standing all-8-cell 0.80 % gate."""
    out = {}
    for cell in CELLS:
        w, c, n = summary[cell]["wall"], summary[cell]["cpu"], null[cell]
        if w["median_saving_pct"] < FLOOR_PCT:
            out[cell] = "SUB-FLOOR"
        elif not w["median_saving_pct"] > n["wall_median_pct"]:
            out[cell] = "INSIDE-NULL-MEDIAN"
        elif not c["median_saving_pct"] > 0.0:
            out[cell] = "CPU-CONTRADICTED"
        else:
            out[cell] = "PASS"
    return out


def strict_verdicts(summary, null):
    return {
        cell: (
            "PASS"
            if summary[cell]["wall"]["median_saving_pct"] > null[cell]["wall_any_pair_pct"]
            else "INSIDE-NULL-ANY-PAIR"
        )
        for cell in CELLS
    }


def events(path):
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if line.startswith("{"):
                try:
                    yield json.loads(line)
                except json.JSONDecodeError:
                    pass


def diagnostics_of(path):
    diag = {"degraded": None, "fatal": None, "non_fatal": None, "code_1200": 0}
    for ev in events(path):
        if ev.get("event") == "slice_complete":
            diag["degraded"] = ev.get("degraded")
            diag["fatal"] = ev.get("fatal_error_count")
            diag["non_fatal"] = ev.get("non_fatal_error_count")
        elif ev.get("event") == "diagnostic" and ev.get("code") == 1200:
            diag["code_1200"] += 1
        elif "code 1200" in str(ev.get("message", "")):
            diag["code_1200"] += 1
    return diag


def postpass_stage_metrics_ms(path):
    """Extract PostPass substage elapsed_ms and slice_complete elapsed_ms."""
    by_stage = {s: 0.0 for s in POSTPASS_STAGES}
    slice_complete_ms = 0.0
    for ev in events(path):
        kind = ev.get("event")
        if kind == "slice_complete":
            val = ev.get("elapsed_ms")
            if val is not None:
                slice_complete_ms = float(val)
        elif kind == "stage_complete" and ev.get("stage") in by_stage:
            val = ev.get("elapsed_ms")
            if val is not None:
                by_stage[ev["stage"]] += float(val)
    total_postpass = sum(by_stage.values())
    return {
        "layer_finalization_ms": by_stage["PostPass::LayerFinalization"],
        "gcode_emit_ms": by_stage["PostPass::GCodeEmit"],
        "gcode_postprocess_ms": by_stage["PostPass::GCodePostProcess"],
        "gcode_serialize_ms": by_stage["PostPass::GCodeSerialize"],
        "postpass_total_ms": total_postpass,
        "slice_complete_ms": slice_complete_ms,
    }


def parse_profile_json(path):
    """Extract deterministic LayerFinalization guest fuel and machine-gcode-emit fuel."""
    report = None
    for ev in events(path):
        if "modules" in ev:
            report = ev
        elif isinstance(ev.get("profile"), dict) and "modules" in ev["profile"]:
            report = ev["profile"]
    if report is None:
        raise SystemExit(f"No --profile JSON found in {path}")
    mods = {m["module_id"]: m for m in report["modules"]}
    lf_fuel = {m: int(mods.get(m, {}).get("total_fuel", 0)) for m in LF_MODULES}
    return {
        "fuel_total": int(report.get("fuel_total", 0)),
        "lf_modules_fuel": lf_fuel,
        "lf_total_fuel": sum(lf_fuel.values()),
        "machine_gcode_emit_fuel": int(mods.get("com.core.machine-gcode-emit", {}).get("total_fuel", 0)),
    }


def main():
    active = read_json(T182_REF_PATH)["active_set"]
    t191_route = read_json(T191_ROUTE_PATH)
    manifest = read_json(os.path.join(RAW, "manifest.json"))

    ord_windows = ["ab"] + (["ab2"] if has_batch("ab", "ab2") else [])
    null_windows = ["null"] + (["null2"] if has_batch("ab", "null2") else [])
    accel_windows = ["accel"] + (["accel2"] if has_batch("ab", "accel2") else [])
    ab_rows = [r for b in ord_windows for r in read_rows("ab", b)]
    null_rows = [r for b in null_windows for r in read_rows("ab", b)]
    accel_rows = [r for b in accel_windows for r in read_rows("ab", b)]
    stage_rows = read_rows("ab", "stage")
    prof_rows = read_rows("profile", "profile")
    ref_rows = read_rows("output", "ref")
    acc_out_rows = read_rows("output", "accel")

    diag = {
        k: 0
        for k in (
            "total_runs_checked",
            "nonzero_exit_runs",
            "degraded_runs",
            "fatal_error_runs",
            "non_fatal_error_runs",
            "code_1200_total",
            "hash_mismatches",
        )
    }
    all_checked = ref_rows + acc_out_rows + ab_rows + null_rows + accel_rows + stage_rows + prof_rows
    for row in all_checked:
        d = diagnostics_of(row["stderr_path"])
        diag["total_runs_checked"] += 1
        diag["nonzero_exit_runs"] += int(int(row["exit_code"]) != 0)
        diag["degraded_runs"] += int(d["degraded"] is True)
        diag["fatal_error_runs"] += int(d["fatal"] not in (0, None))
        diag["non_fatal_error_runs"] += int(d["non_fatal"] not in (0, None))
        diag["code_1200_total"] += d["code_1200"]
        diag["hash_mismatches"] += int(row["gcode_sha256"] != active[row["cell"]])

    exe_changed = {
        "ordinary": manifest["baseline-ordinary"]["exe_sha256"]
        != manifest["candidate-ordinary"]["exe_sha256"],
        "accelerated": manifest["baseline-accelerated"]["exe_sha256"]
        != manifest["candidate-accelerated"]["exe_sha256"],
    }
    modules_tree_changed = {
        "ordinary": manifest["baseline-ordinary"]["modules_tree_sha256"]
        != manifest["candidate-ordinary"]["modules_tree_sha256"],
        "accelerated": manifest["baseline-accelerated"]["modules_tree_sha256"]
        != manifest["candidate-accelerated"]["modules_tree_sha256"],
    }

    output = {
        "schema": 1,
        "t182_active": active,
        "manifest": manifest,
        "exe_changed": exe_changed,
        "modules_tree_changed": modules_tree_changed,
        "cells": {},
        "accelerated": {},
        "diagnostics": diag,
    }
    for row in ref_rows:
        output["cells"].setdefault(row["cell"], {})[row["arm"]] = {
            "sha256": row["gcode_sha256"],
            "bytes": int(row["gcode_bytes"]),
        }
    for row in acc_out_rows:
        output["accelerated"].setdefault(row["cell"], {})[row["arm"]] = row["gcode_sha256"]

    wall = paired(ab_rows, lambda r: float(r["wall_s"]))
    cpu = paired(ab_rows, lambda r: float(r["cpu_s"]))
    summary = {cell: {"wall": cell_stats(wall[cell]), "cpu": cell_stats(cpu[cell])} for cell in CELLS}
    null = null_band(null_rows)
    summary["_verdict_of_record"] = (
        f"pooled windows ({len(ord_windows)} ord, {len(null_windows)} null, {len(accel_windows)} accel)"
        if len(ord_windows) > 1 or len(accel_windows) > 1
        else "single quiet window (5 pairs ord, 3 pairs null, 3 pairs accel)"
    )
    summary["_gate"] = verdicts(summary, null)
    summary["_gate_strict_any_pair_null"] = strict_verdicts(summary, null)
    meta_batches = ord_windows + null_windows + accel_windows + ["stage"]
    summary["_meta"] = {
        b: read_json(os.path.join(RAW, "ab", b, "batch_meta.json")) for b in meta_batches
    }

    aw = paired(accel_rows, lambda r: float(r["wall_s"]))
    ac = paired(accel_rows, lambda r: float(r["cpu_s"]))
    accel = {
        cell: {"wall": cell_stats(aw[cell]), "cpu": cell_stats(ac[cell])}
        for cell in CELLS
        if cell in aw
    }

    # Instrumented PostPass substages across all 8 cells
    stage_by_cell = {}
    for row in stage_rows:
        m = postpass_stage_metrics_ms(row["stderr_path"])
        stage_by_cell.setdefault((row["cell"], int(row["pair"])), {})[row["side"]] = m
    stage_cells = {}
    for cell in CELLS:
        pairs_ms = []
        for (_c, p), sides in sorted(stage_by_cell.items()):
            if _c == cell and "baseline" in sides and "candidate" in sides:
                pairs_ms.append(
                    {
                        "pair": p,
                        "baseline_postpass_ms": sides["baseline"]["postpass_total_ms"],
                        "candidate_postpass_ms": sides["candidate"]["postpass_total_ms"],
                        "baseline_lf_ms": sides["baseline"]["layer_finalization_ms"],
                        "candidate_lf_ms": sides["candidate"]["layer_finalization_ms"],
                        "baseline_emit_ms": sides["baseline"]["gcode_emit_ms"],
                        "candidate_emit_ms": sides["candidate"]["gcode_emit_ms"],
                        "baseline_postproc_ms": sides["baseline"]["gcode_postprocess_ms"],
                        "candidate_postproc_ms": sides["candidate"]["gcode_postprocess_ms"],
                        "baseline_ser_ms": sides["baseline"]["gcode_serialize_ms"],
                        "candidate_ser_ms": sides["candidate"]["gcode_serialize_ms"],
                        "baseline_slice_complete_ms": sides["baseline"]["slice_complete_ms"],
                        "candidate_slice_complete_ms": sides["candidate"]["slice_complete_ms"],
                    }
                )
        def med(key):
            return statistics.median(x[key] for x in pairs_ms)
        b_pp, c_pp = med("baseline_postpass_ms"), med("candidate_postpass_ms")
        b_lf, c_lf = med("baseline_lf_ms"), med("candidate_lf_ms")
        b_em, c_em = med("baseline_emit_ms"), med("candidate_emit_ms")
        b_sr, c_sr = med("baseline_ser_ms"), med("candidate_ser_ms")
        stage_cells[cell] = {
            "baseline_postpass_median_ms": b_pp,
            "candidate_postpass_median_ms": c_pp,
            "postpass_saving_pct": (b_pp - c_pp) / b_pp * 100.0 if b_pp > 0 else 0.0,
            "postpass_speedup_x": b_pp / c_pp if c_pp > 0 else 0.0,
            "baseline_lf_median_ms": b_lf,
            "candidate_lf_median_ms": c_lf,
            "lf_saving_pct": (b_lf - c_lf) / b_lf * 100.0 if b_lf > 0 else 0.0,
            "baseline_emit_median_ms": b_em,
            "candidate_emit_median_ms": c_em,
            "emit_saving_pct": (b_em - c_em) / b_em * 100.0 if b_em > 0 else 0.0,
            "baseline_ser_median_ms": b_sr,
            "candidate_ser_median_ms": c_sr,
            "ser_saving_pct": (b_sr - c_sr) / b_sr * 100.0 if b_sr > 0 else 0.0,
            "ser_speedup_x": b_sr / c_sr if c_sr > 0 else 0.0,
            "baseline_slice_complete_median_ms": med("baseline_slice_complete_ms"),
            "candidate_slice_complete_median_ms": med("candidate_slice_complete_ms"),
            "pairs_ms": pairs_ms,
        }
    stage = {
        "schema": 1,
        "note": "Instrumented (--instrument-stderr) pairs across all 8 cells; attribution only.",
        "cells": stage_cells,
    }

    # Deterministic --profile LayerFinalization guest fuel
    prof_by_cell = {}
    for row in prof_rows:
        arm_short = "baseline" if row["arm"].startswith("baseline") else "candidate"
        prof_by_cell.setdefault(row["cell"], {})[arm_short] = parse_profile_json(row["stderr_path"])
    prof_cells = {}
    for cell in sorted(prof_by_cell.keys()):
        b = prof_by_cell[cell]["baseline"]
        c = prof_by_cell[cell]["candidate"]
        lf_b, lf_c = b["lf_total_fuel"], c["lf_total_fuel"]
        prof_cells[cell] = {
            "baseline": b,
            "candidate": c,
            "lf_fuel_delta": lf_c - lf_b,
            "lf_fuel_saving_pct": (lf_b - lf_c) / lf_b * 100.0 if lf_b > 0 else 0.0,
            "machine_gcode_emit_fuel_delta": c["machine_gcode_emit_fuel"] - b["machine_gcode_emit_fuel"],
        }
    profile = {
        "schema": 1,
        "cells": prof_cells,
    }

    # Cross-window ESTIMATE ONLY route composed on t191/reduced/route.json
    route = {
        "schema": 1,
        "note": (
            "ESTIMATE ONLY (cross-window composition of t174 quiet-board ratios x t175 x "
            "t182 x t183 x t184 x t185 x t190 x t191 x t192 paired savings). Not a scoreboard ratio of record."
        ),
        "cells": {},
    }
    for cell in CELLS:
        prior = t191_route["cells"][cell]
        row = {"supports": prior["supports"]}
        for arm in ("ordinary", "accelerated"):
            before = prior[f"est_post_t191_ratio_{arm}"]
            stats_src = summary if arm == "ordinary" else accel
            saving = stats_src[cell]["wall"]["median_saving_pct"]
            row[f"est_post_t191_ratio_{arm}"] = before
            row[f"t192_wall_saving_{arm}_pct"] = saving
            row[f"est_post_t192_ratio_{arm}"] = before * (1.0 - saving / 100.0)
        route["cells"][cell] = row

    design = {
        "ordinary_windows": ord_windows,
        "null_windows": null_windows,
        "accel_windows": accel_windows,
        "ordinary_pairs_per_window": summary[CELLS[0]]["wall"]["pairs"] // len(ord_windows),
        "null_pairs_per_window": null[CELLS[0]]["pairs"] // len(null_windows),
        "accelerated_pairs": accel[CELLS[0]]["wall"]["pairs"],
        "accel_cells": sorted(accel.keys()),
        "stage_pairs": len(stage_cells[CELLS[0]]["pairs_ms"]),
        "stage_cells": sorted({r["cell"] for r in stage_rows}),
        "profile_cells": sorted(prof_by_cell.keys()),
    }
    expected = (
        len(ref_rows)
        + len(acc_out_rows)
        + len(ab_rows)
        + len(null_rows)
        + len(accel_rows)
        + len(stage_rows)
        + len(prof_rows)
    )
    write_json("campaign.json", {"schema": 1, "design": design, "expected_runs": expected})
    write_json("output.json", output)
    write_json("ab.json", summary)
    write_json("null.json", null)
    write_json("accel.json", accel)
    write_json("stage.json", stage)
    write_json("profile.json", profile)
    write_json("route.json", route)

    print("Diagnostics:", diag)
    print("exe_changed:", exe_changed, "| modules_tree_changed:", modules_tree_changed)
    print("\n--- LITERAL STANDING ALL-8-CELL GATE ---")
    for cell in CELLS:
        w, c, nb = summary[cell]["wall"], summary[cell]["cpu"], null[cell]
        a = accel[cell]
        a_str = f"acc {a['wall']['median_saving_pct']:+6.2f}%/{a['cpu']['median_saving_pct']:+6.2f}%"
        print(
            f"{cell:20s} wall {w['median_saving_pct']:+6.2f}% ({w['wins']}/{w['pairs']}, {w['floor_multiple']:5.2f}x) "
            f"cpu {c['median_saving_pct']:+6.2f}% ({c['wins']}/{c['pairs']}) "
            f"null {nb['wall_median_pct']:4.2f}/{nb['wall_any_pair_pct']:5.2f}% {a_str:18s} -> {summary['_gate'][cell]}"
        )
    print("\n--- POSTPASS STAGE ATTRIBUTION ---")
    for cell in CELLS:
        s = stage_cells[cell]
        print(
            f"{cell:20s} PP {s['baseline_postpass_median_ms']:6.1f}->{s['candidate_postpass_median_ms']:5.1f}ms ({s['postpass_saving_pct']:+5.1f}%) "
            f"| LF {s['baseline_lf_median_ms']:5.1f}->{s['candidate_lf_median_ms']:5.1f} "
            f"| Emit {s['baseline_emit_median_ms']:5.1f}->{s['candidate_emit_median_ms']:5.1f} "
            f"| Ser {s['baseline_ser_median_ms']:5.1f}->{s['candidate_ser_median_ms']:4.1f} ({s['ser_speedup_x']:.1f}x)"
        )
    print("\n--- LAYER FINALIZATION FUEL (--profile) ---")
    for cell, p in prof_cells.items():
        print(
            f"{cell:20s} LF fuel {p['baseline']['lf_total_fuel']/1e6:7.2f}M -> {p['candidate']['lf_total_fuel']/1e6:6.2f}M ({p['lf_fuel_saving_pct']:+5.2f}%) "
            f"| machine-gcode-emit delta={p['machine_gcode_emit_fuel_delta']}"
        )
    print("\n--- ROUTE (ESTIMATE ONLY) ---")
    for cell in CELLS:
        r = route["cells"][cell]
        print(
            f"{cell:20s} ord {r['est_post_t191_ratio_ordinary']:.4f}x -> {r['est_post_t192_ratio_ordinary']:.4f}x "
            f"| acc {r['est_post_t191_ratio_accelerated']:.4f}x -> {r['est_post_t192_ratio_accelerated']:.4f}x"
        )


if __name__ == "__main__":
    main()

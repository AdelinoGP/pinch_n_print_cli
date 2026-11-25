#!/usr/bin/env python3
"""t190 reduce: exact `com.core.tree-support-planner` pure-guest acceleration.

Standing t137 4-cell planner gate on the four supports-on cells, non-regression
on the four structurally inert supports-off cells, 16/16 ordinary + 16/16
accelerated byte-identical output and clean diagnostics against t182's active
currency, identical host executables and identical non-planner guest modules
between arms (pure-guest edit strictly inside `tree-support-planner`),
instrumented `PrePass::SupportGeometry` & `com.core.tree-support-planner` stage
reduction, deterministic `--profile` planner fuel & `host:native` call reduction
(including classic == arachne invariance), and ESTIMATE ONLY route composed on
`t185/reduced/route.json`.

Reads the gitignored raw tree `target/t190/` plus committed siblings
(`t182/reference.json`, `t185/reduced/route.json`) and writes `reduced/`.

Usage:  python reduce_t190.py
"""

import csv
import json
import os
import statistics

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
RAW = os.path.join(REPO, "target", "t190")
OUT = os.path.join(HERE, "reduced")
T182_REF_PATH = os.path.join(HERE, "..", "t182-spatial-hierarchical-phase3-union", "reference.json")
T185_ROUTE_PATH = os.path.join(HERE, "..", "t185-support-geometry-host-tail-accel", "reduced", "route.json")

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
GATED = [cell for cell in CELLS if cell.endswith("-on")]
INERT = [cell for cell in CELLS if cell.endswith("-off")]
FLOOR_PCT = 0.80
STAGE = "PrePass::SupportGeometry"
PLANNER_ID = "com.core.tree-support-planner"


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
    """Gated cells: standing t137 4-cell gate. Inert cells: no regression beyond floor."""
    out = {}
    for cell in CELLS:
        w, c, n = summary[cell]["wall"], summary[cell]["cpu"], null[cell]
        if cell in INERT:
            out[cell] = "INERT-PASS" if w["median_saving_pct"] >= -FLOOR_PCT else "REGRESSION"
        elif w["median_saving_pct"] < FLOOR_PCT:
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
        for cell in GATED
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


def planner_stage_metrics_ms(path):
    """Extract `PrePass::SupportGeometry` stage elapsed_ms and
    `com.core.tree-support-planner` module elapsed_ms from `--instrument-stderr`."""
    stage_ms = 0.0
    planner_ms = 0.0
    for ev in events(path):
        if ev.get("stage") != STAGE:
            continue
        kind = ev.get("event")
        if kind == "module_complete" and ev.get("module_id") == PLANNER_ID:
            planner_ms += float(ev.get("elapsed_ms", 0.0))
        elif kind == "stage_complete":
            val = ev.get("elapsed_ms")
            if val is not None:
                stage_ms += float(val)
    return {"stage_ms": stage_ms, "planner_ms": planner_ms}


def parse_profile_json(path):
    """Extract deterministic planner fuel and host:native calls from `--profile`."""
    report = None
    for ev in events(path):
        if "modules" in ev:
            report = ev
        elif isinstance(ev.get("profile"), dict) and "modules" in ev["profile"]:
            report = ev["profile"]
    if report is None:
        raise SystemExit(f"No --profile JSON found in {path}")
    mods = {m["module_id"]: m for m in report["modules"]}
    tsp = mods[PLANNER_ID]
    hn = mods["host:native"]
    scopes = {s["scope"]: s for s in hn.get("scopes", [])}
    return {
        "planner_fuel": int(tsp["total_fuel"]),
        "tree_support_fuel": int(mods["com.core.tree-support"]["total_fuel"]),
        "host_native_calls": int(hn["calls"]),
        "host_native_wall_ns": int(hn["total_wall_ns"]),
        "host_native_clip_calls": int(scopes.get("polygon_ops::clip_polygons", {}).get("calls", 0)),
        "host_native_offset_calls": int(scopes.get("polygon_ops::offset", {}).get("calls", 0)),
    }


def main():
    active = read_json(T182_REF_PATH)["active_set"]
    t185_route = read_json(T185_ROUTE_PATH)
    manifest = read_json(os.path.join(RAW, "manifest.json"))

    ord_windows = ["ab"] + (["ab2"] if has_batch("ab", "ab2") else [])
    null_windows = ["null"] + (["null2"] if has_batch("ab", "null2") else [])
    ab_rows = [r for b in ord_windows for r in read_rows("ab", b)]
    null_rows = [r for b in null_windows for r in read_rows("ab", b)]
    accel_rows = read_rows("ab", "accel")
    stage_rows = read_rows("ab", "stage")
    fuel_rows = read_rows("fuel", "profile")
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
    all_checked = ref_rows + acc_out_rows + ab_rows + null_rows + accel_rows + stage_rows + fuel_rows
    for row in all_checked:
        d = diagnostics_of(row["stderr_path"])
        diag["total_runs_checked"] += 1
        diag["nonzero_exit_runs"] += int(int(row["exit_code"]) != 0)
        # Only `--instrument-stderr` runs emit `slice_complete` JSON with `degraded`;
        # non-instrumented runs have `d["degraded"] is None` (not degraded).
        diag["degraded_runs"] += int(d["degraded"] is True)
        diag["fatal_error_runs"] += int(d["fatal"] not in (0, None))
        diag["non_fatal_error_runs"] += int(d["non_fatal"] not in (0, None))
        diag["code_1200_total"] += d["code_1200"]
        diag["hash_mismatches"] += int(row["gcode_sha256"] != active[row["cell"]])

    exe_identical = {
        "ordinary": manifest["baseline-ordinary"]["exe_sha256"]
        == manifest["candidate-ordinary"]["exe_sha256"],
        "accelerated": manifest["baseline-accelerated"]["exe_sha256"]
        == manifest["candidate-accelerated"]["exe_sha256"],
    }
    other_modules_identical = {
        "ordinary": manifest["baseline-ordinary"]["other_modules_tree_sha256"]
        == manifest["candidate-ordinary"]["other_modules_tree_sha256"],
        "accelerated": manifest["baseline-accelerated"]["other_modules_tree_sha256"]
        == manifest["candidate-accelerated"]["other_modules_tree_sha256"],
    }
    planner_wasm_changed = {
        "ordinary": manifest["baseline-ordinary"]["tree_support_planner_wasm_sha256"]
        != manifest["candidate-ordinary"]["tree_support_planner_wasm_sha256"],
        "accelerated": manifest["baseline-accelerated"]["tree_support_planner_wasm_sha256"]
        != manifest["candidate-accelerated"]["tree_support_planner_wasm_sha256"],
    }

    output = {
        "schema": 1,
        "t182_active": active,
        "manifest": manifest,
        "exe_identical": exe_identical,
        "other_modules_identical": other_modules_identical,
        "planner_wasm_changed": planner_wasm_changed,
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
        "pooled windows" if len(ord_windows) > 1 else "single quiet window (5 pairs ord, 3 pairs null)"
    )
    summary["_gate"] = verdicts(summary, null)
    summary["_gate_strict_any_pair_null"] = strict_verdicts(summary, null)
    meta_batches = ord_windows + null_windows + ["accel", "stage"]
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

    # Instrumented stage + planner module timing
    stage_by_cell = {}
    for row in stage_rows:
        m = planner_stage_metrics_ms(row["stderr_path"])
        stage_by_cell.setdefault((row["cell"], int(row["pair"])), {})[row["side"]] = m
    stage_cells = {}
    inert_cells = {}
    for cell in CELLS:
        pairs_ms = []
        for (_c, p), sides in sorted(stage_by_cell.items()):
            if _c == cell and "baseline" in sides and "candidate" in sides:
                pairs_ms.append(
                    {
                        "pair": p,
                        "baseline_planner_ms": sides["baseline"]["planner_ms"],
                        "candidate_planner_ms": sides["candidate"]["planner_ms"],
                        "baseline_stage_ms": sides["baseline"]["stage_ms"],
                        "candidate_stage_ms": sides["candidate"]["stage_ms"],
                    }
                )
        b_pm = statistics.median(x["baseline_planner_ms"] for x in pairs_ms)
        c_pm = statistics.median(x["candidate_planner_ms"] for x in pairs_ms)
        b_sm = statistics.median(x["baseline_stage_ms"] for x in pairs_ms)
        c_sm = statistics.median(x["candidate_stage_ms"] for x in pairs_ms)
        entry = {
            "baseline_planner_median_ms": b_pm,
            "candidate_planner_median_ms": c_pm,
            "planner_saving_pct": (b_pm - c_pm) / b_pm * 100.0 if b_pm > 0 else 0.0,
            "baseline_stage_median_ms": b_sm,
            "candidate_stage_median_ms": c_sm,
            "stage_saving_pct": (b_sm - c_sm) / b_sm * 100.0 if b_sm > 0 else 0.0,
            "pairs_ms": pairs_ms,
        }
        if cell in GATED:
            stage_cells[cell] = entry
        else:
            inert_cells[cell] = entry
    stage = {
        "schema": 1,
        "note": (
            "Instrumented (--instrument-stderr) pairs; attribution only. On supports-off "
            "cells, com.core.tree-support-planner early-returns on !enable_support; residual "
            "elapsed_ms (~17 ms benchy, ~104 ms base) is host WIT IR marshalling shared identically "
            "by baseline and candidate."
        ),
        "cells": stage_cells,
        "inert_cells": inert_cells,
    }

    # Deterministic --profile fuel + host:native call counts
    fuel_by_cell = {}
    for row in fuel_rows:
        arm_short = "baseline" if row["arm"].startswith("baseline") else "candidate"
        fuel_by_cell.setdefault(row["cell"], {})[arm_short] = parse_profile_json(row["stderr_path"])
    fuel_cells = {}
    for cell in GATED:
        b = fuel_by_cell[cell]["baseline"]
        c = fuel_by_cell[cell]["candidate"]
        fuel_cells[cell] = {
            "baseline": b,
            "candidate": c,
            "planner_fuel_saving_pct": (b["planner_fuel"] - c["planner_fuel"]) / b["planner_fuel"] * 100.0,
            "host_native_call_delta": c["host_native_calls"] - b["host_native_calls"],
            "host_native_call_saving_pct": (b["host_native_calls"] - c["host_native_calls"]) / b["host_native_calls"] * 100.0,
        }
    fuel = {
        "schema": 1,
        "cells": fuel_cells,
        "classic_arachne_invariance": {
            "benchy_host_native_delta_equal": (
                fuel_cells["benchy-classic-on"]["host_native_call_delta"]
                == fuel_cells["benchy-arachne-on"]["host_native_call_delta"]
            ),
            "base_host_native_delta_equal": (
                fuel_cells["base-classic-on"]["host_native_call_delta"]
                == fuel_cells["base-arachne-on"]["host_native_call_delta"]
            ),
        },
    }

    # Cross-window ESTIMATE ONLY route composed on t185/reduced/route.json
    route = {
        "schema": 1,
        "note": (
            "ESTIMATE ONLY (cross-window composition of t174 quiet-board ratios x t175 x "
            "t182 x t183 x t184 x t185 x t190 paired savings). Not a scoreboard ratio of record."
        ),
        "cells": {},
    }
    for cell in CELLS:
        prior = t185_route["cells"][cell]
        row = {"supports": prior["supports"]}
        for arm in ("ordinary", "accelerated"):
            before = prior[f"est_post_t185_ratio_{arm}"]
            stats_src = summary if arm == "ordinary" else accel
            if cell in stats_src:
                saving = stats_src[cell]["wall"]["median_saving_pct"]
                row[f"est_post_t185_ratio_{arm}"] = before
                row[f"t190_wall_saving_{arm}_pct"] = saving
                # For structurally inert supports-off cells, carry forward exact t185 ratio
                # while disclosing the measured noise delta separately.
                if cell in INERT:
                    row[f"est_post_t190_ratio_{arm}"] = before
                else:
                    row[f"est_post_t190_ratio_{arm}"] = before * (1.0 - saving / 100.0)
            else:
                row[f"est_post_t185_ratio_{arm}"] = before
                row[f"t190_wall_saving_{arm}_pct"] = 0.0
                row[f"est_post_t190_ratio_{arm}"] = before
        route["cells"][cell] = row

    design = {
        "ordinary_windows": ord_windows,
        "null_windows": null_windows,
        "ordinary_pairs_per_window": summary[GATED[0]]["wall"]["pairs"] // len(ord_windows),
        "null_pairs_per_window": null[GATED[0]]["pairs"] // len(null_windows),
        "accelerated_pairs": accel[GATED[0]]["wall"]["pairs"],
        "accel_cells": sorted(accel.keys()),
        "stage_pairs": len(stage_cells[GATED[0]]["pairs_ms"]),
        "stage_cells": sorted({r["cell"] for r in stage_rows}),
        "fuel_cells": sorted(fuel_by_cell.keys()),
    }
    expected = (
        len(ref_rows)
        + len(acc_out_rows)
        + len(ab_rows)
        + len(null_rows)
        + len(accel_rows)
        + len(stage_rows)
        + len(fuel_rows)
    )
    write_json("campaign.json", {"schema": 1, "design": design, "expected_runs": expected})
    write_json("output.json", output)
    write_json("ab.json", summary)
    write_json("null.json", null)
    write_json("accel.json", accel)
    write_json("stage.json", stage)
    write_json("fuel.json", fuel)
    write_json("route.json", route)

    print("Diagnostics:", diag)
    print("exe_identical:", exe_identical, "| other_modules_identical:", other_modules_identical, "| planner_wasm_changed:", planner_wasm_changed)
    print("\n--- STANDING t137 GATE ---")
    for cell in CELLS:
        w, c, nb = summary[cell]["wall"], summary[cell]["cpu"], null[cell]
        a_str = ""
        if cell in accel:
            a = accel[cell]
            a_str = f"acc {a['wall']['median_saving_pct']:+6.2f}%/{a['cpu']['median_saving_pct']:+6.2f}%"
        print(
            f"{cell:20s} wall {w['median_saving_pct']:+6.2f}% ({w['wins']}/{w['pairs']}, {w['floor_multiple']:5.2f}x) "
            f"cpu {c['median_saving_pct']:+6.2f}% ({c['wins']}/{c['pairs']}) "
            f"null {nb['wall_median_pct']:4.2f}/{nb['wall_any_pair_pct']:5.2f}% {a_str:18s} -> {summary['_gate'][cell]}"
        )
    print("\n--- PLANNER STAGE & FUEL ---")
    for cell in GATED:
        s = stage_cells[cell]
        f = fuel_cells[cell]
        print(
            f"{cell:20s} planner {s['baseline_planner_median_ms']:6.1f} -> {s['candidate_planner_median_ms']:6.1f} ms ({s['planner_saving_pct']:+5.2f}%) "
            f"| fuel {f['baseline']['planner_fuel']/1e9:6.3f} -> {f['candidate']['planner_fuel']/1e9:6.3f} B ({f['planner_fuel_saving_pct']:+5.2f}%) "
            f"| host:native {f['baseline']['host_native_calls']} -> {f['candidate']['host_native_calls']} ({f['host_native_call_saving_pct']:+5.2f}%)"
        )
    print("\n--- ROUTE (ESTIMATE ONLY) ---")
    for cell in CELLS:
        r = route["cells"][cell]
        print(
            f"{cell:20s} ord {r['est_post_t185_ratio_ordinary']:.4f}x -> {r['est_post_t190_ratio_ordinary']:.4f}x "
            f"| acc {r['est_post_t185_ratio_accelerated']:.4f}x -> {r['est_post_t190_ratio_accelerated']:.4f}x"
        )


if __name__ == "__main__":
    main()

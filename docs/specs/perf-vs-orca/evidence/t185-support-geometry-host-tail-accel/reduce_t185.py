#!/usr/bin/env python3
"""t185 reduce: exact `PrePass::SupportGeometry` post-planner host-tail acceleration.

Standing t137 planner gate on the four supports-on cells (pooled two ordinary
windows, PREREGISTRATION.md), non-regression on the four structurally inert
supports-off cells, 16/16 byte-identical output and clean diagnostics against
t182's active currency, identical guest module trees between arms (t185 edits
host crates only), the instrumented post-planner tail, and the ESTIMATE ONLY
route composed on t184's.

Reads the gitignored raw tree `target/t185/` plus committed siblings
(`t182/reference.json`, `t184/reduced/route.json`) and writes `reduced/`.

Usage:  python reduce_t185.py
"""

import csv
import json
import os
import statistics

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
RAW = os.path.join(REPO, "target", "t185")
OUT = os.path.join(HERE, "reduced")
T182_REF_PATH = os.path.join(HERE, "..", "t182-spatial-hierarchical-phase3-union", "reference.json")
T184_ROUTE_PATH = os.path.join(HERE, "..", "t184-shell-classification-exact-accel", "reduced", "route.json")

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
DESIGN = {
    "ordinary_windows": ["ab", "ab2"],
    "null_windows": ["null", "null2"],
    "ordinary_pairs": 5,
    "null_pairs": 3,
    "accelerated_pairs": 3,
    "stage_pairs": 3,
}


def read_json(path):
    with open(path, encoding="utf-8-sig") as fh:
        return json.load(fh)


def read_rows(*parts):
    with open(os.path.join(RAW, *parts, "rows.csv"), encoding="utf-8-sig", newline="") as fh:
        return list(csv.DictReader(fh))


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
                {"batch": batch, "pair": pair, "baseline_s": b, "candidate_s": c,
                 "saving_pct": (b - c) / b * 100.0}
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
    }


def verdicts(summary, null):
    """Gated cells: t137 literal gate. Inert cells: no regression beyond the floor."""
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
        cell: ("PASS" if summary[cell]["wall"]["median_saving_pct"] > null[cell]["wall_any_pair_pct"]
               else "INSIDE-NULL-ANY-PAIR")
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


def support_tail_ms(path):
    """Post-planner host tail of the planner `SupportGeometry` stage instance:
    its `stage_complete` minus its last planner `module_complete`. 0 when the
    stage ran no planner module (supports off)."""
    tails = []
    last_module = None
    for ev in events(path):
        if ev.get("stage") != STAGE:
            continue
        kind = ev.get("event")
        if kind == "stage_start":
            last_module = None
        elif kind == "module_complete" and not str(ev.get("module_id", "")).startswith("host:"):
            last_module = ev["timestamp_ms"]
        elif kind == "stage_complete" and last_module is not None:
            tails.append(ev["timestamp_ms"] - last_module)
            last_module = None
    if len(tails) > 1:
        raise SystemExit(f"{path}: {len(tails)} planner stage instances")
    return float(tails[0]) if tails else 0.0


def main():
    active = read_json(T182_REF_PATH)["active_set"]
    t184_route = read_json(T184_ROUTE_PATH)
    windows = {
        name: (read_rows("ab", a), read_rows("ab", n))
        for name, a, n in zip(("window1", "window2"), DESIGN["ordinary_windows"], DESIGN["null_windows"])
    }
    ab_rows = [r for a, _ in windows.values() for r in a]
    null_rows = [r for _, n in windows.values() for r in n]
    accel_rows = read_rows("ab", "accel")
    stage_rows = read_rows("ab", "stage")
    ref_rows = read_rows("output", "ref")
    acc_out_rows = read_rows("output", "accel")
    manifest = read_json(os.path.join(RAW, "manifest.json"))

    diag = {k: 0 for k in ("total_runs_checked", "nonzero_exit_runs", "degraded_runs",
                           "fatal_error_runs", "non_fatal_error_runs", "code_1200_total",
                           "hash_mismatches")}
    for row in ref_rows + acc_out_rows + ab_rows + null_rows + accel_rows + stage_rows:
        d = diagnostics_of(row["stderr_path"])
        diag["total_runs_checked"] += 1
        diag["nonzero_exit_runs"] += int(int(row["exit_code"]) != 0)
        diag["degraded_runs"] += int(d["degraded"] is not False)
        diag["fatal_error_runs"] += int(d["fatal"] not in (0, None))
        diag["non_fatal_error_runs"] += int(d["non_fatal"] not in (0, None))
        diag["code_1200_total"] += d["code_1200"]
        diag["hash_mismatches"] += int(row["gcode_sha256"] != active[row["cell"]])

    output = {
        "schema": 1,
        "t182_active": active,
        "manifest": manifest,
        "modules_identical": {
            "ordinary": manifest["baseline-ordinary"]["modules_tree_sha256"]
            == manifest["candidate-ordinary"]["modules_tree_sha256"],
            "accelerated": manifest["baseline-accelerated"]["modules_tree_sha256"]
            == manifest["candidate-accelerated"]["modules_tree_sha256"],
        },
        "cells": {},
        "accelerated": {},
        "diagnostics": diag,
    }
    for row in ref_rows:
        output["cells"].setdefault(row["cell"], {})[row["arm"]] = {
            "sha256": row["gcode_sha256"], "bytes": int(row["gcode_bytes"])}
    for row in acc_out_rows:
        output["accelerated"].setdefault(row["cell"], {})[row["arm"]] = row["gcode_sha256"]

    wall = paired(ab_rows, lambda r: float(r["wall_s"]))
    cpu = paired(ab_rows, lambda r: float(r["cpu_s"]))
    summary = {cell: {"wall": cell_stats(wall[cell]), "cpu": cell_stats(cpu[cell])} for cell in CELLS}
    null = null_band(null_rows)
    summary["_verdict_of_record"] = "pooled windows (PREREGISTRATION.md)"
    summary["_gate"] = verdicts(summary, null)
    summary["_gate_strict_any_pair_null"] = strict_verdicts(summary, null)
    per_window = {}
    for name, (a_rows, n_rows) in windows.items():
        w = paired(a_rows, lambda r: float(r["wall_s"]))
        c = paired(a_rows, lambda r: float(r["cpu_s"]))
        ws = {cell: {"wall": cell_stats(w[cell]), "cpu": cell_stats(c[cell])} for cell in CELLS}
        wn = null_band(n_rows)
        per_window[name] = {
            "gate": verdicts(ws, wn),
            "cells": {
                cell: {
                    "wall_median_saving_pct": ws[cell]["wall"]["median_saving_pct"],
                    "cpu_median_saving_pct": ws[cell]["cpu"]["median_saving_pct"],
                    "null_wall_median_pct": wn[cell]["wall_median_pct"],
                }
                for cell in CELLS
            },
        }
    summary["_per_window"] = per_window
    summary["_meta"] = {
        b: read_json(os.path.join(RAW, "ab", b, "batch_meta.json"))
        for b in ("ab", "ab2", "null", "null2", "accel", "stage")
    }
    aw = paired(accel_rows, lambda r: float(r["wall_s"]))
    ac = paired(accel_rows, lambda r: float(r["cpu_s"]))
    accel = {cell: {"wall": cell_stats(aw[cell]), "cpu": cell_stats(ac[cell])} for cell in CELLS}

    inert_tails = {r["cell"]: support_tail_ms(r["stderr_path"]) for r in stage_rows if r["cell"] in INERT}
    if any(inert_tails.values()):
        raise SystemExit(f"supports-off cell ran a planner tail: {inert_tails}")
    tails = paired([r for r in stage_rows if r["cell"] in GATED],
                   lambda r: support_tail_ms(r["stderr_path"]) / 1000.0)
    stage = {
        "schema": 1,
        "note": (
            "Instrumented (--instrument-stderr) pairs; post-planner host tail = planner "
            "SupportGeometry stage_complete minus its last planner module_complete. "
            "Attribution only."
        ),
        "cells": {
            cell: {
                "baseline_median_ms": statistics.median(p["baseline_s"] for p in tails[cell]) * 1000.0,
                "candidate_median_ms": statistics.median(p["candidate_s"] for p in tails[cell]) * 1000.0,
                "pairs_ms": [
                    {"pair": p["pair"], "baseline_ms": p["baseline_s"] * 1000.0,
                     "candidate_ms": p["candidate_s"] * 1000.0}
                    for p in tails[cell]
                ],
            }
            for cell in GATED
        },
        "inert_cells_tail_ms": {cell: 0.0 for cell in sorted(inert_tails)},
    }

    route = {"schema": 1, "note": (
        "ESTIMATE ONLY (cross-window composition of t174 quiet-board ratios x t175 x "
        "t182 x t183 x t184 x t185 paired savings). Not a scoreboard ratio of record."), "cells": {}}
    for cell in CELLS:
        prior = t184_route["cells"][cell]
        row = {"supports": prior["supports"]}
        for arm, stats in (("ordinary", summary[cell]), ("accelerated", accel[cell])):
            before = prior[f"est_post_t184_ratio_{arm}"]
            saving = stats["wall"]["median_saving_pct"]
            row[f"est_post_t184_ratio_{arm}"] = before
            row[f"t185_wall_saving_{arm}_pct"] = saving
            row[f"est_post_t185_ratio_{arm}"] = before * (1.0 - saving / 100.0)
        route["cells"][cell] = row

    n = len(CELLS)
    expected = (4 * n + 2 * n * (DESIGN["ordinary_pairs"] * 2 + DESIGN["null_pairs"] * 2
                                + DESIGN["accelerated_pairs"] + DESIGN["stage_pairs"]))
    write_json("campaign.json", {"schema": 1, "design": DESIGN, "expected_runs": expected})
    write_json("output.json", output)
    write_json("ab.json", summary)
    write_json("null.json", null)
    write_json("accel.json", accel)
    write_json("stage.json", stage)
    write_json("route.json", route)

    print("Diagnostics:", diag, "| modules identical:", output["modules_identical"])
    for name, w in per_window.items():
        print(f"\n--- {name} ---")
        for cell in CELLS:
            c = w["cells"][cell]
            print(f"{cell:20s} wall {c['wall_median_saving_pct']:+6.2f}% cpu {c['cpu_median_saving_pct']:+6.2f}% "
                  f"null {c['null_wall_median_pct']:4.2f}% -> {w['gate'][cell]}")
    print("\n--- POOLED (verdict of record) ---")
    for cell in CELLS:
        w, c, nb = summary[cell]["wall"], summary[cell]["cpu"], null[cell]
        a = accel[cell]
        print(f"{cell:20s} wall {w['median_saving_pct']:+6.2f}% ({w['wins']}/{w['pairs']}, {w['floor_multiple']:5.2f}x) "
              f"cpu {c['median_saving_pct']:+6.2f}% ({c['wins']}/{c['pairs']}) null {nb['wall_median_pct']:4.2f}/{nb['wall_any_pair_pct']:5.2f}% "
              f"acc {a['wall']['median_saving_pct']:+6.2f}%/{a['cpu']['median_saving_pct']:+6.2f}% -> {summary['_gate'][cell]}")
    print("\n--- post-planner tail (instrumented) ---")
    for cell in GATED:
        s = stage["cells"][cell]
        print(f"{cell:20s} {s['baseline_median_ms']:8.1f} -> {s['candidate_median_ms']:8.1f} ms")
    print("\n--- route (ESTIMATE ONLY) ---")
    for cell in CELLS:
        r = route["cells"][cell]
        print(f"{cell:20s} ord {r['est_post_t184_ratio_ordinary']:.4f}x -> {r['est_post_t185_ratio_ordinary']:.4f}x "
              f"| acc {r['est_post_t184_ratio_accelerated']:.4f}x -> {r['est_post_t185_ratio_accelerated']:.4f}x")


if __name__ == "__main__":
    main()

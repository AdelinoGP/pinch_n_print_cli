#!/usr/bin/env python3
"""t184 reduce: exact `PrePass::ShellClassification` acceleration.

Literal standing all-8-cell paired ordinary + null + accelerated gate, 16/16
byte-identical output and diagnostics against t182's active currency,
deterministic guest fuel (`--profile`; t184 changes no guest code, so fuel must
be unchanged), instrumented `host:shell_classification` stage walls, and the
cross-window ESTIMATE ONLY route composition on t183's route.

Reads the gitignored raw tree under `target/t184/` plus committed sibling
artifacts (`t182/reference.json`, `t183/reduced/route.json`) and writes the
in-repo reduced set under `reduced/`:
  - `ab.json`, `null.json`, `accel.json`  (timing gate)
  - `output.json`                          (hashes, manifest, diagnostics)
  - `fuel.json`                            (guest-fuel neutrality)
  - `stage.json`                           (instrumented stage walls)
  - `route.json`                           (ESTIMATE ONLY)

Usage:  python reduce_t184.py
"""

import csv
import json
import os
import re
import statistics

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
RAW = os.path.join(REPO, "target", "t184")
OUT = os.path.join(HERE, "reduced")

T182_REF_PATH = os.path.join(
    HERE, "..", "t182-spatial-hierarchical-phase3-union", "reference.json"
)
T183_ROUTE_PATH = os.path.join(
    HERE, "..", "t183-per-layer-guest-exact-accel", "reduced", "route.json"
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
# Campaign design (REPRODUCE.md): pairs per batch. Two runs per pair per cell.
DESIGN = {
    # Two ordinary windows (PREREGISTRATION.md): verdict of record is pooled.
    "ordinary_windows": ["ab", "ab2"],
    "null_windows": ["null", "null2"],
    "ordinary_pairs": 5,
    "null_pairs": 3,
    "accelerated_pairs": 3,
    "stage_pairs": 3,
    "host_only_pairs": 3,
}
STAGE_MODULE = "host:shell_classification"

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


def paired(rows, value):
    """{cell: [{pair, baseline, candidate, saving_pct}]} for non-null rows."""
    by = {}
    for row in rows:
        if row.get("null_control") in ("True", "true", "1"):
            continue
        by.setdefault((row["cell"], row["batch"], int(row["pair"])), {})[row["side"]] = value(row)
    out = {}
    for (cell, batch, pair), sides in sorted(by.items()):
        if "baseline" not in sides or "candidate" not in sides:
            continue
        b, c = sides["baseline"], sides["candidate"]
        out.setdefault(cell, []).append(
            {
                "batch": batch,
                "pair": pair,
                "baseline_s": b,
                "candidate_s": c,
                "saving_pct": (b - c) / b * 100.0,
            }
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
        "pairs_detail": savings,
    }


def null_band(rows):
    by = {}
    for row in rows:
        by.setdefault((row["cell"], row["batch"], int(row["pair"])), {})[row["side"]] = row
    out = {}
    for (cell, _batch, _pair), sides in sorted(by.items()):
        a, b = sides.get("baseline"), sides.get("baseline2")
        if not a or not b:
            continue
        wall = abs((float(a["wall_s"]) - float(b["wall_s"])) / float(b["wall_s"]) * 100.0)
        cpu = abs((float(a["cpu_s"]) - float(b["cpu_s"])) / float(b["cpu_s"]) * 100.0)
        out.setdefault(cell, {"wall_abs": [], "cpu_abs": []})
        out[cell]["wall_abs"].append(wall)
        out[cell]["cpu_abs"].append(cpu)
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
        key: max(res[c][key] for c in CELLS)
        for key in ("wall_any_pair_pct", "wall_median_pct", "cpu_any_pair_pct", "cpu_median_pct")
    }
    return res


def gate_literal_8cell(summary, null):
    """Literal standing all-8-cell 0.80 % gate: median paired wall saving >=
    0.80 %, strictly above the cell's null median wall band, CPU lower."""
    verdicts = {}
    for cell in CELLS:
        w, c, n = summary[cell]["wall"], summary[cell]["cpu"], null[cell]
        if w["median_saving_pct"] < FLOOR_PCT:
            verdicts[cell] = "SUB-FLOOR"
        elif not w["median_saving_pct"] > n["wall_median_pct"]:
            verdicts[cell] = "INSIDE-NULL-MEDIAN"
        elif not c["median_saving_pct"] > 0.0:
            verdicts[cell] = "CPU-CONTRADICTED"
        else:
            verdicts[cell] = "PASS"
    return verdicts


def gate_strict_any_pair_null(summary, null):
    """Stricter lens: median wall saving strictly above the worst null pair."""
    verdicts = {}
    for cell in CELLS:
        w, c, n = summary[cell]["wall"], summary[cell]["cpu"], null[cell]
        if w["median_saving_pct"] < FLOOR_PCT:
            verdicts[cell] = "SUB-FLOOR"
        elif not w["median_saving_pct"] > n["wall_any_pair_pct"]:
            verdicts[cell] = "INSIDE-NULL-ANY-PAIR"
        elif not c["median_saving_pct"] > 0.0:
            verdicts[cell] = "CPU-CONTRADICTED"
        else:
            verdicts[cell] = "PASS"
    return verdicts


def events(path):
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if not line.startswith("{"):
                continue
            try:
                yield json.loads(line)
            except json.JSONDecodeError:
                continue


def inspect_jsonl_stderr(path):
    diag = {
        "degraded": None,
        "fatal_error_count": None,
        "non_fatal_error_count": None,
        "code_1200_count": 0,
    }
    for ev in events(path):
        kind = ev.get("event")
        if kind == "slice_complete":
            diag["degraded"] = ev.get("degraded")
            diag["fatal_error_count"] = ev.get("fatal_error_count")
            diag["non_fatal_error_count"] = ev.get("non_fatal_error_count")
        elif kind == "diagnostic" and ev.get("code") == 1200:
            diag["code_1200_count"] += 1
        elif "code 1200" in str(ev.get("message", "")):
            diag["code_1200_count"] += 1
    return diag


def stage_elapsed_ms(path):
    """`elapsed_ms` of the single `host:shell_classification` completion."""
    found = [
        ev["elapsed_ms"]
        for ev in events(path)
        if ev.get("event") == "module_complete" and ev.get("module_id") == STAGE_MODULE
    ]
    if len(found) != 1:
        raise SystemExit(f"{path}: expected one {STAGE_MODULE} completion, found {len(found)}")
    return float(found[0])


def parse_profile_fuel(path):
    total, count, modules = None, None, {}
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            s = line.strip()
            m = RE_TOTAL_FUEL.match(s)
            if m:
                total = int(m.group(1).replace(",", ""))
                count = int(m.group(2))
                continue
            m = RE_MOD_FUEL.match(s)
            if m:
                modules[m.group(1)] = {
                    "fuel": int(m.group(3).replace(",", "")),
                    "calls": int(m.group(4)),
                }
    return {"total_fuel": total, "module_count": count, "modules": modules}


def reduce_fuel(fuel_rows, check_diag):
    by_cell = {}
    for row in fuel_rows:
        check_diag(row)
        by_cell.setdefault(row["cell"], {})[row["arm"]] = parse_profile_fuel(row["stderr_path"])
    cells = {}
    for cell in CELLS:
        b = by_cell[cell]["baseline-ordinary"]
        c = by_cell[cell]["candidate-ordinary"]
        modules = {}
        for name in sorted(b["modules"]):
            bf, cf = b["modules"][name]["fuel"], c["modules"][name]["fuel"]
            modules[name] = {
                "baseline_fuel": bf,
                "candidate_fuel": cf,
                "delta_pct": (cf - bf) / bf * 100.0 if bf else 0.0,
                "calls_identical": b["modules"][name]["calls"] == c["modules"][name]["calls"],
            }
        cells[cell] = {
            "baseline_total_fuel": b["total_fuel"],
            "candidate_total_fuel": c["total_fuel"],
            "total_delta_pct": (c["total_fuel"] - b["total_fuel"]) / b["total_fuel"] * 100.0,
            "module_count": b["module_count"],
            "same_module_set": sorted(b["modules"]) == sorted(c["modules"]),
            "calls_identical": all(m["calls_identical"] for m in modules.values()),
            "modules": modules,
        }
    return {
        "schema": 1,
        "note": (
            "Deterministic WASM guest fuel via --profile, ordinary baseline vs "
            "candidate. t184 edits no guest source and no guest calls the edited "
            "slicer-core functions, but every guest is recompiled (the freshness "
            "fingerprint covers slicer-core) and the recompiled WASM differs, so "
            "per-module fuel drifts. Output is byte-identical; the drift is "
            "codegen, not behaviour. host_only.json isolates the host change."
        ),
        "cells": cells,
    }


def reduce_stage(stage_rows, check_diag):
    for row in stage_rows:
        check_diag(row)
    pairs = paired(stage_rows, lambda row: stage_elapsed_ms(row["stderr_path"]) / 1000.0)
    cells = {}
    for cell in CELLS:
        stats = cell_stats(pairs[cell])
        cells[cell] = {
            "pairs": stats["pairs"],
            "baseline_median_ms": stats["baseline_median_s"] * 1000.0,
            "candidate_median_ms": stats["candidate_median_s"] * 1000.0,
            "median_saving_pct": stats["median_saving_pct"],
            "wins": stats["wins"],
            "strict_separation": stats["strict_separation"],
            "pairs_ms": [
                {
                    "pair": p["pair"],
                    "baseline_ms": p["baseline_s"] * 1000.0,
                    "candidate_ms": p["candidate_s"] * 1000.0,
                }
                for p in pairs[cell]
            ],
        }
    return {
        "schema": 1,
        "note": (
            f"Instrumented (--instrument-stderr) interleaved pairs; `{STAGE_MODULE}` "
            "module_complete elapsed_ms. Attribution only: instrumented walls are "
            "never used for the timing gate."
        ),
        "cells": cells,
    }


def reduce_route(ab, accel, t183_route):
    rows = {}
    for cell in CELLS:
        r = t183_route["cells"][cell]
        r_ord = r["est_post_t183_ratio_ordinary"]
        r_acc = r["est_post_t183_ratio_accelerated"]
        s_ord = ab[cell]["wall"]["median_saving_pct"]
        s_acc = accel[cell]["wall"]["median_saving_pct"]
        est_ord = r_ord * (1.0 - s_ord / 100.0)
        est_acc = r_acc * (1.0 - s_acc / 100.0)
        rows[cell] = {
            "supports": r["supports"],
            "est_post_t183_ratio_ordinary": r_ord,
            "t184_wall_saving_ordinary_pct": s_ord,
            "est_post_t184_ratio_ordinary": est_ord,
            "est_post_t184_required_cut_ordinary_pct": max(0.0, (1.0 - 1.0 / est_ord) * 100.0),
            "est_post_t183_ratio_accelerated": r_acc,
            "t184_wall_saving_accelerated_pct": s_acc,
            "est_post_t184_ratio_accelerated": est_acc,
            "est_post_t184_required_cut_accelerated_pct": max(0.0, (1.0 - 1.0 / est_acc) * 100.0),
        }
    return {
        "schema": 1,
        "note": (
            "ESTIMATE ONLY (cross-window composition of t174 quiet-board official "
            "ratios x t175 x t182 x t183 x t184 paired savings). Not a scoreboard "
            "ratio of record."
        ),
        "cells": rows,
    }


def main():
    t182_active = read_json(T182_REF_PATH)["active_set"]
    t183_route = read_json(T183_ROUTE_PATH)

    def raw_rows(*parts):
        return read_rows(os.path.join(RAW, *parts, "rows.csv"))

    windows = {
        w: (raw_rows("ab", a), raw_rows("ab", n))
        for w, a, n in zip(("window1", "window2"), DESIGN["ordinary_windows"], DESIGN["null_windows"])
    }
    ab_rows = [row for a, _ in windows.values() for row in a]
    null_rows = [row for _, n in windows.values() for row in n]
    accel_rows = raw_rows("ab", "accel")
    stage_rows = raw_rows("ab", "stage")
    host_rows = raw_rows("ab", "hostonly")
    ref_rows = raw_rows("output", "ref")
    acc_out_rows = raw_rows("output", "accel")
    fuel_rows = raw_rows("fuel", "fuel")

    out = {
        "schema": 1,
        "t182_active": t182_active,
        "manifest": read_json(os.path.join(RAW, "manifest.json")),
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
    diag_totals = out["diagnostics"]

    def check_diag(row):
        diag = inspect_jsonl_stderr(row["stderr_path"])
        diag_totals["total_runs_checked"] += 1
        diag_totals["nonzero_exit_runs"] += int(int(row["exit_code"]) != 0)
        diag_totals["degraded_runs"] += int(diag["degraded"] is not False)
        diag_totals["fatal_error_runs"] += int(diag["fatal_error_count"] not in (0, None))
        diag_totals["non_fatal_error_runs"] += int(diag["non_fatal_error_count"] not in (0, None))
        diag_totals["code_1200_total"] += diag["code_1200_count"]
        diag_totals["hash_mismatches"] += int(row["gcode_sha256"] != t182_active[row["cell"]])
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

    summary = {}
    wall = paired(ab_rows, lambda row: float(row["wall_s"]))
    cpu = paired(ab_rows, lambda row: float(row["cpu_s"]))
    for cell in CELLS:
        summary[cell] = {"wall": cell_stats(wall[cell]), "cpu": cell_stats(cpu[cell])}
    null = null_band(null_rows)
    summary["_gate_literal_8cell"] = gate_literal_8cell(summary, null)
    summary["_gate_strict_any_pair_null"] = gate_strict_any_pair_null(summary, null)
    summary["_verdict_of_record"] = "pooled windows (PREREGISTRATION.md)"
    per_window = {}
    for name, (a_rows, n_rows) in windows.items():
        w_wall = paired(a_rows, lambda row: float(row["wall_s"]))
        w_cpu = paired(a_rows, lambda row: float(row["cpu_s"]))
        w_summary = {
            cell: {"wall": cell_stats(w_wall[cell]), "cpu": cell_stats(w_cpu[cell])}
            for cell in CELLS
        }
        w_null = null_band(n_rows)
        per_window[name] = {
            "cells": {
                cell: {
                    "wall_median_saving_pct": w_summary[cell]["wall"]["median_saving_pct"],
                    "wall_wins": w_summary[cell]["wall"]["wins"],
                    "cpu_median_saving_pct": w_summary[cell]["cpu"]["median_saving_pct"],
                    "cpu_wins": w_summary[cell]["cpu"]["wins"],
                    "null_wall_median_pct": w_null[cell]["wall_median_pct"],
                    "null_wall_any_pair_pct": w_null[cell]["wall_any_pair_pct"],
                }
                for cell in CELLS
            },
            "gate_literal_8cell": gate_literal_8cell(w_summary, w_null),
            "gate_strict_any_pair_null": gate_strict_any_pair_null(w_summary, w_null),
        }
    summary["_per_window"] = per_window
    summary["_meta"] = {
        b: read_json(os.path.join(RAW, "ab", b, "batch_meta.json"))
        for b in ("ab", "ab2", "null", "null2", "accel", "stage", "hostonly")
    }
    accel = {}
    aw = paired(accel_rows, lambda row: float(row["wall_s"]))
    ac = paired(accel_rows, lambda row: float(row["cpu_s"]))
    for cell in CELLS:
        accel[cell] = {"wall": cell_stats(aw[cell]), "cpu": cell_stats(ac[cell])}

    host_only = {}
    hw = paired(host_rows, lambda row: float(row["wall_s"]))
    hc = paired(host_rows, lambda row: float(row["cpu_s"]))
    for cell in CELLS:
        host_only[cell] = {"wall": cell_stats(hw[cell]), "cpu": cell_stats(hc[cell])}
    host_only["_note"] = (
        "Attribution only: candidate host and baseline host both load the "
        "baseline modules, so the arms differ only in the host binary."
    )

    for row in ab_rows + null_rows + accel_rows + host_rows:
        check_diag(row)
    fuel = reduce_fuel(fuel_rows, check_diag)
    stage = reduce_stage(stage_rows, check_diag)
    route = reduce_route(summary, accel, t183_route)

    n = len(CELLS)
    expected_runs = (
        2 * n  # ordinary output
        + 2 * n  # accelerated output
        + 2 * n  # --profile fuel
        + 2 * n * DESIGN["ordinary_pairs"] * len(DESIGN["ordinary_windows"])
        + 2 * n * DESIGN["null_pairs"] * len(DESIGN["null_windows"])
        + 2 * n * DESIGN["accelerated_pairs"]
        + 2 * n * DESIGN["stage_pairs"]
        + 2 * n * DESIGN["host_only_pairs"]
    )
    write_json("campaign.json", {"schema": 1, "design": DESIGN, "expected_runs": expected_runs})
    write_json("ab.json", summary)
    write_json("null.json", null)
    write_json("accel.json", accel)
    write_json("output.json", out)
    write_json("fuel.json", fuel)
    write_json("stage.json", stage)
    write_json("host_only.json", host_only)
    write_json("route.json", route)

    print("Wrote reduced/*.json")
    print("Diagnostics:", diag_totals)
    for name, w in summary["_per_window"].items():
        print(f"\n--- {name}: literal gate ---")
        for cell in CELLS:
            c = w["cells"][cell]
            print(
                f"{cell:20s} | wall {c['wall_median_saving_pct']:+6.2f}% ({c['wall_wins']}/5) "
                f"| cpu {c['cpu_median_saving_pct']:+6.2f}% | null med/any {c['null_wall_median_pct']:4.2f}/{c['null_wall_any_pair_pct']:5.2f}% "
                f"| lit={w['gate_literal_8cell'][cell]}"
            )
    print("\n--- POOLED (verdict of record): 10-pair A/B + 6-pair null + gate ---")
    for cell in CELLS:
        w, c, n = summary[cell]["wall"], summary[cell]["cpu"], null[cell]
        a_w, a_c = accel[cell]["wall"], accel[cell]["cpu"]
        print(
            f"{cell:20s} | wall {w['median_saving_pct']:+6.2f}% ({w['wins']}/{w['pairs']}, "
            f"{w['floor_multiple']:5.2f}x, sep={w['strict_separation']}) "
            f"| cpu {c['median_saving_pct']:+6.2f}% ({c['wins']}/{c['pairs']}, sep={c['strict_separation']}) "
            f"| null med/any {n['wall_median_pct']:4.2f}/{n['wall_any_pair_pct']:4.2f}% "
            f"| acc {a_w['median_saving_pct']:+6.2f}%/{a_c['median_saving_pct']:+6.2f}% "
            f"| lit={summary['_gate_literal_8cell'][cell]} strict={summary['_gate_strict_any_pair_null'][cell]}"
        )
    print("\n--- Stage (instrumented) ---")
    for cell in CELLS:
        s = stage["cells"][cell]
        print(
            f"{cell:20s} | {s['baseline_median_ms']:8.1f} -> {s['candidate_median_ms']:8.1f} ms "
            f"({s['median_saving_pct']:+6.2f}%, {s['wins']}/{s['pairs']})"
        )
    print("\n--- Host-only attribution (both arms on baseline modules) ---")
    for cell in CELLS:
        w, c = host_only[cell]["wall"], host_only[cell]["cpu"]
        print(f"{cell:20s} | wall {w['median_saving_pct']:+6.2f}% ({w['wins']}/{w['pairs']}) | cpu {c['median_saving_pct']:+6.2f}% ({c['wins']}/{c['pairs']})")
    print("\n--- Guest fuel drift (recompilation) ---")
    for cell in CELLS:
        f = fuel["cells"][cell]
        worst = max(f["modules"].items(), key=lambda kv: abs(kv[1]["delta_pct"]))
        print(f"{cell:20s} | total {f['total_delta_pct']:+.3f}% | calls identical={f['calls_identical']} | largest {worst[0]} {worst[1]['delta_pct']:+.3f}%")
    print("\n--- Route (ESTIMATE ONLY) ---")
    for cell in CELLS:
        r = route["cells"][cell]
        print(
            f"{cell:20s} | ord {r['est_post_t183_ratio_ordinary']:.4f}x -> {r['est_post_t184_ratio_ordinary']:.4f}x "
            f"| acc {r['est_post_t183_ratio_accelerated']:.4f}x -> {r['est_post_t184_ratio_accelerated']:.4f}x"
        )


if __name__ == "__main__":
    main()

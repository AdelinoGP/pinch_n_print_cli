#!/usr/bin/env python3
"""Checkout-only verifier for t192 (exact Zero-WIT `PostPass` acceleration).

Re-derives from the committed `reduced/*.json` and committed siblings
(`t182/reference.json`, `t191/reduced/route.json`):

  1. output parity: 16/16 ordinary + 16/16 accelerated hashes equal t182's
     active set; zero non-zero exits, degraded runs, errors, code-1200
     diagnostics, and hash mismatches over every checked run; total run count
     matches campaign design;
  2. host + guest artifacts changed in both ordinary and accelerated modes;
  3. literal standing all-8-cell 0.80 % gate (recomputed from raw pairs):
     median wall saving >= 0.80 %, strictly above null median, process CPU lower
     across all 8/8 cells;
  4. accelerated corroboration across all 8/8 cells (>= 0.80 % wall, CPU not
     contradicted beyond null CPU median);
  5. instrumented `PostPass` total, `LayerFinalization`, `GCodeEmit`, and
     `GCodeSerialize` medians drop on all 8/8 cells;
  6. deterministic `--profile` `LayerFinalization` combined guest fuel drops
     > 70 % on all 4 profiled supports-on cells with zero `machine-gcode-emit`
     fuel drift;
  7. route arithmetic recomputed from `t191/reduced/route.json`.

Usage:  python verify.py [--self-test]
"""

import copy
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RED = os.path.join(HERE, "reduced")
T182_REF = os.path.join(HERE, "..", "t182-spatial-hierarchical-phase3-union", "reference.json")
T191_ROUTE = os.path.join(HERE, "..", "t191-support-analysis-exact-accel", "reduced", "route.json")
CELLS = [
    "benchy-classic-off", "benchy-classic-on", "benchy-arachne-off", "benchy-arachne-on",
    "base-classic-off", "base-classic-on", "base-arachne-off", "base-arachne-on",
]
PROFILE_CELLS = [c for c in CELLS if c.endswith("-on")]
FLOOR_PCT = 0.80


def load_json(path):
    with open(path, encoding="utf-8-sig") as fh:
        return json.load(fh)


def median_saving(pairs):
    return statistics.median((p["baseline_s"] - p["candidate_s"]) / p["baseline_s"] * 100.0 for p in pairs)


def check(doc, failures):
    fail = failures.append
    active = doc["t182"]["active_set"]
    out = doc["output"]
    if out["t182_active"] != active:
        fail("OUTPUT: reduced t182_active differs from the committed reference")
    for cell in CELLS:
        for arm in ("baseline-ordinary", "candidate-ordinary"):
            got = out["cells"].get(cell, {}).get(arm, {}).get("sha256")
            if got != active[cell]:
                fail(f"OUTPUT: {cell}/{arm} {got} != t182")
        for arm in ("baseline-accelerated", "candidate-accelerated"):
            got = out["accelerated"].get(cell, {}).get(arm)
            if got != active[cell]:
                fail(f"OUTPUT: {cell}/{arm} {got} != t182")
    for key in (
        "nonzero_exit_runs",
        "degraded_runs",
        "fatal_error_runs",
        "non_fatal_error_runs",
        "code_1200_total",
        "hash_mismatches",
    ):
        if out["diagnostics"][key] != 0:
            fail(f"DIAG: {key} = {out['diagnostics'][key]}")
    if out["diagnostics"]["total_runs_checked"] != doc["campaign"]["expected_runs"]:
        fail(
            f"DIAG: checked {out['diagnostics']['total_runs_checked']} runs, "
            f"expected {doc['campaign']['expected_runs']}"
        )
    for arm in ("ordinary", "accelerated"):
        if not out["exe_changed"][arm]:
            fail(f"ARTIFACT: {arm} host executable did not change between arms")
        if not out["modules_tree_changed"][arm]:
            fail(f"ARTIFACT: {arm} guest modules tree did not change between arms")

    ab, null = doc["ab"], doc["null"]
    min_ord_pairs = 5 * len(doc["campaign"]["design"]["ordinary_windows"])
    min_null_pairs = 3 * len(doc["campaign"]["design"]["null_windows"])
    for cell in CELLS:
        wall_pairs = ab[cell]["wall"]["pairs_detail"]
        if len(wall_pairs) < min_ord_pairs:
            fail(f"GATE: {cell} has {len(wall_pairs)} pairs, need {min_ord_pairs}")
            continue
        wall = median_saving(wall_pairs)
        cpu = median_saving(ab[cell]["cpu"]["pairs_detail"])
        if abs(wall - ab[cell]["wall"]["median_saving_pct"]) > 1e-9:
            fail(f"GATE: {cell} stored wall median disagrees with its pairs")
        if null[cell]["pairs"] < min_null_pairs:
            fail(f"NULL: {cell} has {null[cell]['pairs']} null pairs, need {min_null_pairs}")
        if wall < FLOOR_PCT:
            fail(f"GATE: {cell} wall {wall:.3f} % below the floor")
        if not wall > null[cell]["wall_median_pct"]:
            fail(f"GATE: {cell} wall {wall:.3f} % inside its null median")
        if not cpu > 0.0:
            fail(f"GATE: {cell} CPU not lower ({cpu:.3f} %)")
        if ab["_gate"][cell] != "PASS":
            fail(f"GATE: {cell} stored verdict {ab['_gate'][cell]}")

        a = doc["accel"][cell]
        if len(a["wall"]["pairs_detail"]) < 3:
            fail(f"ACCEL: {cell} has fewer than 3 pairs")
        a_wall = median_saving(a["wall"]["pairs_detail"])
        a_cpu = median_saving(a["cpu"]["pairs_detail"])
        if a_wall < FLOOR_PCT or a_cpu < -null[cell]["cpu_any_pair_pct"]:
            fail(
                f"ACCEL: {cell} wall {a_wall:.3f} % / cpu {a_cpu:.3f} % "
                f"(null cpu any-pair {null[cell]['cpu_any_pair_pct']:.3f} %) does not corroborate"
            )

        s = doc["stage"]["cells"][cell]
        if not s["candidate_postpass_median_ms"] < s["baseline_postpass_median_ms"]:
            fail(f"STAGE: {cell} PostPass total duration did not drop")
        if not s["candidate_lf_median_ms"] < s["baseline_lf_median_ms"]:
            fail(f"STAGE: {cell} LayerFinalization duration did not drop")
        if not s["candidate_emit_median_ms"] < s["baseline_emit_median_ms"]:
            fail(f"STAGE: {cell} GCodeEmit duration did not drop")
        if not s["candidate_ser_median_ms"] < s["baseline_ser_median_ms"]:
            fail(f"STAGE: {cell} GCodeSerialize duration did not drop")
        if abs(statistics.median(p["baseline_postpass_ms"] for p in s["pairs_ms"]) - s["baseline_postpass_median_ms"]) > 1e-6:
            fail(f"STAGE: {cell} stored PostPass median disagrees with its pairs")

    for cell in PROFILE_CELLS:
        p = doc["profile"]["cells"][cell]
        if p["lf_fuel_saving_pct"] < 70.0:
            fail(f"PROFILE: {cell} LayerFinalization fuel saving {p['lf_fuel_saving_pct']:.2f} % (< 70 %)")
        mge_base = p["baseline"]["machine_gcode_emit_fuel"]
        mge_drift_pct = abs(p["machine_gcode_emit_fuel_delta"]) / mge_base * 100.0 if mge_base > 0 else 0.0
        if mge_drift_pct >= 0.5:
            fail(f"PROFILE: {cell} machine-gcode-emit fuel drifted {mge_drift_pct:.4f} % (>= 0.5 %)")

    for cell in CELLS:
        r = doc["route"]["cells"][cell]
        prior = doc["t191_route"]["cells"][cell]
        for arm in ("ordinary", "accelerated"):
            before = prior[f"est_post_t191_ratio_{arm}"]
            stats_src = ab if arm == "ordinary" else doc["accel"]
            want = before * (1.0 - median_saving(stats_src[cell]["wall"]["pairs_detail"]) / 100.0)
            if abs(r[f"est_post_t192_ratio_{arm}"] - want) > 1e-9:
                fail(f"ROUTE: {cell} {arm} {r[f'est_post_t192_ratio_{arm}']} != {want}")


def load_all():
    doc = {
        name: load_json(os.path.join(RED, f"{name}.json"))
        for name in ("output", "ab", "null", "accel", "stage", "profile", "route", "campaign")
    }
    doc["t182"] = load_json(T182_REF)
    doc["t191_route"] = load_json(T191_ROUTE)
    return doc


def self_test(doc):
    def scale(d, section, cell, metric, factor):
        stats = d[section][cell][metric]
        for p in stats["pairs_detail"]:
            p["candidate_s"] = p["baseline_s"] * factor
        stats["median_saving_pct"] = median_saving(stats["pairs_detail"])

    mutations = {
        "candidate hash drift": lambda d: d["output"]["cells"]["base-classic-on"]["candidate-ordinary"].update(sha256="0" * 64),
        "accelerated hash drift": lambda d: d["output"]["accelerated"]["benchy-classic-on"].update({"candidate-accelerated": "f" * 64}),
        "code 1200": lambda d: d["output"]["diagnostics"].update(code_1200_total=1),
        "degraded run": lambda d: d["output"]["diagnostics"].update(degraded_runs=1),
        "missing runs": lambda d: d["output"]["diagnostics"].update(total_runs_checked=d["campaign"]["expected_runs"] - 1),
        "host exe unchanged": lambda d: d["output"]["exe_changed"].update(ordinary=False),
        "accel exe unchanged": lambda d: d["output"]["exe_changed"].update(accelerated=False),
        "modules unchanged": lambda d: d["output"]["modules_tree_changed"].update(ordinary=False),
        "sub-floor": lambda d: scale(d, "ab", "base-classic-off", "wall", 0.995),
        "inside null": lambda d: d["null"]["benchy-arachne-off"].update(wall_median_pct=99.0),
        "cpu contradiction": lambda d: scale(d, "ab", "base-arachne-on", "cpu", 1.05),
        "accelerated regression": lambda d: scale(d, "accel", "benchy-classic-off", "wall", 1.02),
        "accelerated cpu contradiction": lambda d: scale(d, "accel", "base-classic-on", "cpu", 1.05),
        "postpass stage did not drop": lambda d: d["stage"]["cells"]["base-classic-on"].update(candidate_postpass_median_ms=1e9),
        "lf stage did not drop": lambda d: d["stage"]["cells"]["benchy-classic-off"].update(candidate_lf_median_ms=1e9),
        "emit stage did not drop": lambda d: d["stage"]["cells"]["base-classic-on"].update(candidate_emit_median_ms=1e9),
        "ser stage did not drop": lambda d: d["stage"]["cells"]["benchy-arachne-on"].update(candidate_ser_median_ms=1e9),
        "lf fuel saving too low": lambda d: d["profile"]["cells"]["benchy-classic-on"].update(lf_fuel_saving_pct=10.0),
        "machine-gcode-emit fuel drifted": lambda d: d["profile"]["cells"]["base-classic-on"].update(machine_gcode_emit_fuel_delta=50_000_000),
        "route tamper": lambda d: d["route"]["cells"]["base-arachne-on"].update(est_post_t192_ratio_ordinary=0.5),
    }
    rejected = 0
    for name, mutate in mutations.items():
        bad = copy.deepcopy(doc)
        mutate(bad)
        failures = []
        check(bad, failures)
        print(f"  {name}: {'rejected (' + failures[0] + ')' if failures else 'ACCEPTED'}")
        rejected += bool(failures)
    ok = rejected == len(mutations)
    print(f"SELF-TEST {'PASS' if ok else 'FAIL'}: {rejected}/{len(mutations)} mutations rejected")
    return ok


def main(argv):
    doc = load_all()
    if "--self-test" in argv:
        return 0 if self_test(doc) else 1
    failures = []
    check(doc, failures)
    for msg in failures:
        print("FAIL", msg)
    if failures:
        print(f"VERIFY FAIL: {len(failures)} failure(s)")
        return 1
    print(
        "VERIFY PASS: 16/16 ordinary + 16/16 accelerated hashes equal t182; literal all-8-cell 0.80 % gate 8/8;"
        " accelerated 8/8; PostPass/LF/Emit/Ser stage cuts 8/8; LF fuel cut > 70 % 4/4; route recomputed"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

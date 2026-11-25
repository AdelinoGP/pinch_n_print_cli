#!/usr/bin/env python3
"""t183 verify: re-derive all headline claims from the committed reduced set only.

Reads `reduced/*.json` and committed sibling reduced artifacts
(`t182/reference.json`, `t182/reduced/route.json`), and NEVER touches `target/`.

Usage:  python verify.py [--self-test]
"""

import copy
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REDUCED = os.path.join(HERE, "reduced")
T182_REF = os.path.join(
    HERE, "..", "t182-spatial-hierarchical-phase3-union", "reference.json"
)
T182_ROUTE = os.path.join(
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
SUPPORTS_ON = [c for c in CELLS if c.endswith("-on")]
FLOOR_PCT = 0.80


def load_json(path):
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def gate_literal_8cell(ab, null):
    verdicts = {}
    for cell in CELLS:
        w = ab[cell]["wall"]
        c = ab[cell]["cpu"]
        n = null[cell]
        clears = w["median_saving_pct"] >= FLOOR_PCT
        above_med = w["median_saving_pct"] > n["wall_median_pct"]
        cpu_ok = c["median_saving_pct"] > 0.0
        if clears and above_med and cpu_ok:
            verdicts[cell] = "PASS"
        elif not clears:
            verdicts[cell] = "SUB-FLOOR"
        elif not above_med:
            verdicts[cell] = "INSIDE-NULL-MEDIAN"
        else:
            verdicts[cell] = "CPU-CONTRADICTED"
    return verdicts


def gate_strict_any_pair(ab, null):
    verdicts = {}
    for cell in CELLS:
        w = ab[cell]["wall"]
        c = ab[cell]["cpu"]
        n = null[cell]
        clears = w["median_saving_pct"] >= FLOOR_PCT
        outside = w["median_saving_pct"] > n["wall_any_pair_pct"]
        cpu_ok = c["median_saving_pct"] > 0.0
        if clears and outside and cpu_ok:
            verdicts[cell] = "PASS"
        elif not clears:
            verdicts[cell] = "SUB-FLOOR"
        elif not outside:
            verdicts[cell] = "INSIDE-NULL-ANY-PAIR"
        else:
            verdicts[cell] = "CPU-CONTRADICTED"
    return verdicts


def check(doc, failures):
    ab = doc["ab"]
    null = doc["null"]
    accel = doc["accel"]
    output = doc["output"]
    fuel = doc["fuel"]
    route = doc["route"]
    t182_ref = doc["t182_ref"]
    t182_route = doc["t182_route"]

    # 1. Paired ordinary A/B (>= 5 pairs/cell) across all 8 cells.
    for cell in CELLS:
        for metric in ("wall", "cpu"):
            s = ab[cell][metric]
            if s["pairs"] < 5:
                failures.append("%s %s has %d pairs, expected >= 5" % (cell, metric, s["pairs"]))
            if not (s["min_saving_pct"] <= s["median_saving_pct"] <= s["max_saving_pct"]):
                failures.append("%s %s median outside spread" % (cell, metric))
            if not 0 <= s["wins"] <= s["pairs"]:
                failures.append("%s %s win count out of range" % (cell, metric))
            if abs(s["floor_multiple"] - s["median_saving_pct"] / FLOOR_PCT) > 1e-9:
                failures.append("%s %s floor_multiple does not recompute" % (cell, metric))
        if ab[cell]["wall"]["baseline_median_s"] <= 0.0 or ab[cell]["wall"]["candidate_median_s"] <= 0.0:
            failures.append("%s non-positive median wall" % cell)

    # 2. Null band ordering and _worst recomputation.
    for cell in CELLS:
        b = null[cell]
        if b["pairs"] < 3:
            failures.append("%s null has %d pairs, expected >= 3" % (cell, b["pairs"]))
        if b["wall_any_pair_pct"] < b["wall_median_pct"]:
            failures.append("%s null wall band unordered" % cell)
        if b["cpu_any_pair_pct"] < b["cpu_median_pct"]:
            failures.append("%s null cpu band unordered" % cell)
    worst = null["_worst"]
    for key in ("wall_any_pair_pct", "wall_median_pct", "cpu_any_pair_pct", "cpu_median_pct"):
        want = max(null[c][key] for c in CELLS)
        if abs(worst[key] - want) > 1e-12:
            failures.append("null _worst.%s is not max over cells" % key)

    # 3. Literal all-8-cell standing gate MUST PASS 8/8; strict any-pair lens must recompute.
    implied_lit = gate_literal_8cell(ab, null)
    if ab["_gate_literal_8cell"] != implied_lit:
        failures.append("recorded _gate_literal_8cell does not match recomputed verdicts")
    for cell in CELLS:
        if implied_lit[cell] != "PASS":
            failures.append("cell %s failed literal 8-cell gate: %s" % (cell, implied_lit[cell]))
    implied_strict = gate_strict_any_pair(ab, null)
    if ab["_gate_strict_any_pair_null"] != implied_strict:
        failures.append("recorded _gate_strict_any_pair_null does not match recomputed verdicts")

    # 4. Accelerated corroboration (3 pairs/cell, all 8 cells positive wall & CPU >= floor).
    for cell in CELLS:
        for metric in ("wall", "cpu"):
            s = accel[cell][metric]
            if s["pairs"] != 3:
                failures.append("%s accel %s has %d pairs, expected 3" % (cell, metric, s["pairs"]))
            if not (s["min_saving_pct"] <= s["median_saving_pct"] <= s["max_saving_pct"]):
                failures.append("%s accel %s median outside spread" % (cell, metric))
        if accel[cell]["wall"]["median_saving_pct"] < FLOOR_PCT:
            failures.append("%s accel wall saving below floor" % cell)
        if accel[cell]["cpu"]["median_saving_pct"] <= 0.0:
            failures.append("%s accel cpu not corroborating" % cell)

    # 5. Output currency: 100 % byte-identical to t182/reference.json across 16/16 + all 224 runs.
    if len(output["cells"]) != 8:
        failures.append("output covers %d cells, expected 8" % len(output["cells"]))
    diag = output["diagnostics"]
    if (
        diag["total_runs_checked"] < 200
        or diag["nonzero_exit_runs"] != 0
        or diag["degraded_runs"] != 0
        or diag["fatal_error_runs"] != 0
        or diag["non_fatal_error_runs"] != 0
        or diag["code_1200_total"] != 0
        or diag["hash_mismatches"] != 0
    ):
        failures.append("output diagnostics are not clean: %s" % diag)

    for cell in CELLS:
        want_t182 = t182_ref["active_set"][cell]
        if output["t182_active"][cell] != want_t182:
            failures.append("%s output.t182_active disagrees with t182/reference.json" % cell)
        b_ord = output["cells"][cell]["baseline-ordinary"]["sha256"]
        c_ord = output["cells"][cell]["candidate-ordinary"]["sha256"]
        b_acc = output["accelerated"][cell]["baseline-accelerated"]
        c_acc = output["accelerated"][cell]["candidate-accelerated"]
        if b_ord != want_t182:
            failures.append("%s baseline-ordinary does not reproduce t182" % cell)
        if c_ord != want_t182:
            failures.append("%s candidate-ordinary diverges from t182" % cell)
        if b_acc != want_t182:
            failures.append("%s baseline-accelerated does not reproduce t182" % cell)
        if c_acc != want_t182:
            failures.append("%s candidate-accelerated diverges from t182" % cell)

    # 6. Deterministic guest fuel reduction & arithmetic reconciliation.
    for cell in CELLS:
        fc = fuel["cells"][cell]
        if not fc["matches_t182"]:
            failures.append("%s fuel run did not match t182" % cell)
        if fc["baseline_sha256"] != t182_ref["active_set"][cell] or fc["candidate_sha256"] != t182_ref["active_set"][cell]:
            failures.append("%s fuel sha256 != t182 active currency" % cell)
        for key in ("tree_support", "infill_linker", "total_guest_fuel"):
            m = fc[key]
            if m["saved"] != m["baseline"] - m["candidate"]:
                failures.append("%s fuel %s saved mismatch" % (cell, key))
            want_pct = (m["saved"] / m["baseline"] * 100.0) if m["baseline"] > 0 else 0.0
            want_x = (m["baseline"] / m["candidate"]) if m["candidate"] > 0 else 0.0
            if abs(m["cut_pct"] - want_pct) > 1e-9:
                failures.append("%s fuel %s cut_pct mismatch" % (cell, key))
            if abs(m["speedup_x"] - want_x) > 1e-9:
                failures.append("%s fuel %s speedup_x mismatch" % (cell, key))
        if fc["infill_linker"]["cut_pct"] < 70.0:
            failures.append("%s infill-linker fuel cut unexpectedly low: %.2f%%" % (cell, fc["infill_linker"]["cut_pct"]))
        if cell in SUPPORTS_ON and fc["tree_support"]["cut_pct"] < 95.0:
            failures.append("%s tree-support fuel cut unexpectedly low: %.2f%%" % (cell, fc["tree_support"]["cut_pct"]))

    # 7. Cross-window route composition reconciliation (ESTIMATE ONLY).
    if "ESTIMATE ONLY" not in route["note"]:
        failures.append("route.json note missing ESTIMATE ONLY label")
    for cell in CELLS:
        rc = route["cells"][cell]
        r182 = t182_route["cells"][cell]
        want_ord = r182["est_post_t182_ratio_ordinary"] * (
            1.0 - ab[cell]["wall"]["median_saving_pct"] / 100.0
        )
        want_acc = r182["est_post_t182_ratio_accelerated"] * (
            1.0 - accel[cell]["wall"]["median_saving_pct"] / 100.0
        )
        if abs(rc["est_post_t183_ratio_ordinary"] - want_ord) > 1e-12:
            failures.append("%s route est_post_t183_ratio_ordinary does not recompute" % cell)
        if abs(rc["est_post_t183_ratio_accelerated"] - want_acc) > 1e-12:
            failures.append("%s route est_post_t183_ratio_accelerated does not recompute" % cell)


def load_all():
    return {
        "ab": load_json(os.path.join(REDUCED, "ab.json")),
        "null": load_json(os.path.join(REDUCED, "null.json")),
        "accel": load_json(os.path.join(REDUCED, "accel.json")),
        "output": load_json(os.path.join(REDUCED, "output.json")),
        "fuel": load_json(os.path.join(REDUCED, "fuel.json")),
        "route": load_json(os.path.join(REDUCED, "route.json")),
        "t182_ref": load_json(T182_REF),
        "t182_route": load_json(T182_ROUTE),
    }


def self_test(doc):
    controls = []

    def run(name, mutate):
        bad = copy.deepcopy(doc)
        mutate(bad)
        fails = []
        check(bad, fails)
        controls.append((name, bool(fails), fails[:1]))

    run("ab below 5 pairs", lambda d: d["ab"]["base-classic-on"]["wall"].update(pairs=4))
    run("ab median outside spread", lambda d: d["ab"]["base-classic-on"]["wall"].update(median_saving_pct=99.0))
    run("ab floor_multiple tampered", lambda d: d["ab"]["benchy-classic-off"]["wall"].update(floor_multiple=99.0))
    run("null band unordered", lambda d: d["null"]["base-classic-on"].update(wall_any_pair_pct=0.01))
    run("null _worst tampered", lambda d: d["null"]["_worst"].update(wall_any_pair_pct=0.0))
    run("literal gate tampered", lambda d: d["ab"]["_gate_literal_8cell"].update({"base-arachne-off": "SUB-FLOOR"}))
    run("ab cell below floor", lambda d: d["ab"]["base-arachne-off"]["wall"].update(min_saving_pct=0.2, median_saving_pct=0.5, floor_multiple=0.5 / FLOOR_PCT))
    run("accel cpu contradicted", lambda d: d["accel"]["base-classic-on"]["cpu"].update(min_saving_pct=-2.0, median_saving_pct=-1.0))
    run("accel wall below floor", lambda d: d["accel"]["base-arachne-off"]["wall"].update(median_saving_pct=0.5))
    run("degraded run nonzero", lambda d: d["output"]["diagnostics"].update(degraded_runs=1))
    run("code_1200 nonzero", lambda d: d["output"]["diagnostics"].update(code_1200_total=1))
    run("hash mismatch nonzero", lambda d: d["output"]["diagnostics"].update(hash_mismatches=1))
    run("candidate hash moved", lambda d: d["output"]["cells"]["benchy-classic-on"]["candidate-ordinary"].update(sha256="0" * 64))
    run("accel hash diverged", lambda d: d["output"]["accelerated"]["base-classic-on"].update({"candidate-accelerated": "0" * 64}))
    run("fuel cut_pct tampered", lambda d: d["fuel"]["cells"]["base-classic-on"]["tree_support"].update(cut_pct=50.0))
    run("route ratio tampered", lambda d: d["route"]["cells"]["base-classic-on"].update(est_post_t183_ratio_ordinary=0.5))
    return controls


def main(argv):
    doc = load_all()
    if "--self-test" in argv:
        controls = self_test(doc)
        ok = all(rejected for _, rejected, _ in controls)
        for name, rejected, detail in controls:
            print("%-32s %s %s" % (name, "REJECTED" if rejected else "ACCEPTED", detail if rejected else ""))
        print("self-test: %s (%d/%d)" % ("PASS" if ok else "FAIL", sum(r for _, r, _ in controls), len(controls)))
        return 0 if ok else 1

    failures = []
    check(doc, failures)
    if failures:
        for f in failures:
            print("FAIL:", f)
        return 1
    print("PASS: all t183 reduced artifacts, literal 8/8 paired gate, accelerated corroboration, 16/16 byte-identity, deterministic guest fuel, and route composition verified.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

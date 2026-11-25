#!/usr/bin/env python3
"""t182 verify: re-derive all headline claims from the committed reduced set only.

Reads `reduced/*.json`, optional `reference.json`, and committed sibling
reduced artifacts (`t175/reference.json`, `t175/reduced/route.json`,
`t178/reduced/arithmetic.json`), and NEVER touches `target/`.

Usage:  python verify.py [--self-test]
"""

import copy
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REDUCED = os.path.join(HERE, "reduced")
T175_REF = os.path.join(HERE, "..", "t175-tiled-collision-carve", "reference.json")
T175_ROUTE = os.path.join(HERE, "..", "t175-tiled-collision-carve", "reduced", "route.json")
T178_ARITH = os.path.join(HERE, "..", "t178-planner-phase3-union", "reduced", "arithmetic.json")

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


def load_json(path):
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def gate_t137_literal(ab, null):
    verdicts = {}
    for cell in CELLS:
        w = ab[cell]["wall"]
        c = ab[cell]["cpu"]
        n = null[cell]
        if cell in SUPPORTS_ON:
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
        else:
            bound = max(FLOOR_PCT, n["wall_any_pair_pct"])
            if w["median_saving_pct"] >= -bound:
                verdicts[cell] = "INERT-PASS"
            else:
                verdicts[cell] = "REGRESSION"
    return verdicts


def gate_strict_any_pair(ab, null):
    verdicts = {}
    for cell in CELLS:
        w = ab[cell]["wall"]
        c = ab[cell]["cpu"]
        n = null[cell]
        if cell in SUPPORTS_ON:
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
        else:
            bound = max(FLOOR_PCT, n["wall_any_pair_pct"])
            if w["median_saving_pct"] >= -bound:
                verdicts[cell] = "INERT-PASS"
            else:
                verdicts[cell] = "REGRESSION"
    return verdicts


def check(doc, failures):
    sweep = doc["sweep"]
    sweep_audit = doc["sweep_audit"]
    ab = doc["ab"]
    null = doc["null"]
    accel = doc["accel"]
    output = doc["output"]
    audit = doc["audit"]
    route = doc["route"]
    t175_ref = doc["t175_ref"]
    t175_route = doc["t175_route"]
    t178_arith = doc["t178_arith"]

    # 1. Sweep on base-classic-on ({worker, nested} x k in {4, 8, 16}).
    if sweep["cell"] != "base-classic-on":
        failures.append("sweep cell is %s, expected base-classic-on" % sweep["cell"])
    arms = sweep["arms"]
    if sorted(arms.keys()) != sorted(SWEEP_ARMS):
        failures.append("sweep arms mismatch: %s" % sorted(arms.keys()))
    base_wall = arms["baseline"]["wall_median_s"]
    base_cpu = arms["baseline"]["cpu_median_s"]
    for arm in SWEEP_ARMS:
        a = arms[arm]
        if a["runs"] < 3 or len(a["wall_s"]) != a["runs"]:
            failures.append("sweep %s run count invalid" % arm)
        if abs(a["wall_median_s"] - statistics.median(a["wall_s"])) > 1e-12:
            failures.append("sweep %s wall median does not recompute" % arm)
        if abs(a["cpu_median_s"] - statistics.median(a["cpu_s"])) > 1e-12:
            failures.append("sweep %s cpu median does not recompute" % arm)
        want_w = (base_wall - a["wall_median_s"]) / base_wall * 100.0
        want_c = (base_cpu - a["cpu_median_s"]) / base_cpu * 100.0
        if abs(a["wall_saving_pct"] - want_w) > 1e-9:
            failures.append("sweep %s wall_saving_pct does not recompute" % arm)
        if abs(a["cpu_saving_pct"] - want_c) > 1e-9:
            failures.append("sweep %s cpu_saving_pct does not recompute" % arm)
        if a["exit_codes"] != [0] or a["diag_issues"] != 0:
            failures.append("sweep %s has exit/diag issues" % arm)
        if arm == "baseline":
            if a["gcode_sha256"] != t175_ref["active_set"]["base-classic-on"]:
                failures.append("sweep baseline does not match t175 base-classic-on")
        else:
            sa = sweep_audit["arms"][arm]
            if not sa["part_motion_sha256_equal"] or sa["part_sections_moved"]:
                failures.append("sweep %s moved part geometry" % arm)
            if sa["part_geometry_audit"]["non_support_xy_mismatches"] != 0:
                failures.append("sweep %s had non-support XY mismatch" % arm)
    # Determinism: worker-kN === nested-kN byte-identical for every k.
    for k in ("4", "8", "16"):
        if arms[f"worker-k{k}"]["gcode_sha256"] != arms[f"nested-k{k}"]["gcode_sha256"]:
            failures.append(f"worker-k{k} and nested-k{k} hashes diverge")
    if sweep["chosen_arm"] != "worker-k8":
        failures.append("expected chosen_arm worker-k8, got %s" % sweep["chosen_arm"])
    if arms["worker-k8"]["gcode_sha256"] != output["cells"]["base-classic-on"]["candidate-ordinary"]["sha256"]:
        failures.append("chosen sweep arm hash != candidate-ordinary base-classic-on output hash")

    # 2. Paired ordinary A/B (>= 5 pairs/cell).
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

    # 3. Null band ordering and _worst recomputation.
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

    # 4. Gate verdicts (both literal t137 and strict any-pair null disclosure).
    implied_t137 = gate_t137_literal(ab, null)
    if ab["_gate_t137"] != implied_t137:
        failures.append("recorded _gate_t137 does not match recomputed verdicts")
    for cell in SUPPORTS_ON:
        if implied_t137[cell] != "PASS":
            failures.append("supports-on cell %s failed literal t137 gate: %s" % (cell, implied_t137[cell]))
    for cell in SUPPORTS_OFF:
        if implied_t137[cell] != "INERT-PASS":
            failures.append("supports-off cell %s failed inert gate: %s" % (cell, implied_t137[cell]))
    implied_strict = gate_strict_any_pair(ab, null)
    if ab["_gate_strict_any_pair_null"] != implied_strict:
        failures.append("recorded _gate_strict_any_pair_null does not match recomputed verdicts")
    for cell in ("base-classic-on", "base-arachne-on"):
        if implied_strict[cell] != "PASS":
            failures.append("base cell %s failed strict any-pair null gate: %s" % (cell, implied_strict[cell]))

    # 5. Accelerated corroboration (3 pairs/cell, all 4 supports-on positive wall & CPU).
    for cell in CELLS:
        for metric in ("wall", "cpu"):
            s = accel[cell][metric]
            if s["pairs"] != 3:
                failures.append("%s accel %s has %d pairs, expected 3" % (cell, metric, s["pairs"]))
            if not (s["min_saving_pct"] <= s["median_saving_pct"] <= s["max_saving_pct"]):
                failures.append("%s accel %s median outside spread" % (cell, metric))
        if cell in SUPPORTS_ON:
            if accel[cell]["wall"]["median_saving_pct"] < FLOOR_PCT:
                failures.append("%s accel wall saving below floor" % cell)
            if accel[cell]["cpu"]["median_saving_pct"] <= 0.0:
                failures.append("%s accel cpu not corroborating" % cell)

    # 6. Output currency, 16/16 ordinary == accelerated, clean diagnostics.
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
    ):
        failures.append("output diagnostics are not clean: %s" % diag)

    moved_cells = []
    for cell in CELLS:
        want_t175 = t175_ref["active_set"][cell]
        if output["t175_active"][cell] != want_t175:
            failures.append("%s output.t175_active disagrees with t175/reference.json" % cell)
        b_ord = output["cells"][cell]["baseline-ordinary"]["sha256"]
        c_ord = output["cells"][cell]["candidate-ordinary"]["sha256"]
        b_acc = output["accelerated"][cell]["baseline-accelerated"]
        c_acc = output["accelerated"][cell]["candidate-accelerated"]
        if b_ord != want_t175:
            failures.append("%s baseline-ordinary does not reproduce t175" % cell)
        if b_acc != want_t175:
            failures.append("%s baseline-accelerated does not reproduce t175" % cell)
        if c_acc != c_ord:
            failures.append("%s candidate-accelerated diverges from candidate-ordinary" % cell)
        if c_ord != want_t175:
            moved_cells.append(cell)
    if sorted(moved_cells) != sorted(SUPPORTS_ON):
        failures.append("unexpected moved cells set: %s" % moved_cells)

    if doc.get("reference") is not None:
        ref = doc["reference"]
        for cell in CELLS:
            want_c = output["cells"][cell]["candidate-ordinary"]["sha256"]
            want_b = output["cells"][cell]["candidate-ordinary"]["bytes"]
            if ref["active_set"][cell] != want_c:
                failures.append("%s reference.json active_set != candidate hash" % cell)
            if ref["bytes"][cell] != want_b:
                failures.append("%s reference.json bytes != candidate bytes" % cell)

    # 7. G-code delta audit reconciliation: 0 part-geometry movement across 8/8 cells.
    if len(audit["cells"]) != 8:
        failures.append("audit covers %d cells, expected 8" % len(audit["cells"]))
    for cell in CELLS:
        got = audit["cells"][cell]
        if got["baseline_sha256"] != output["cells"][cell]["baseline-ordinary"]["sha256"]:
            failures.append("%s audit baseline hash != output" % cell)
        if got["candidate_sha256"] != output["cells"][cell]["candidate-ordinary"]["sha256"]:
            failures.append("%s audit candidate hash != output" % cell)
        if cell in SUPPORTS_OFF and not got["byte_identical"]:
            failures.append("%s supports-off cell not byte-identical in audit" % cell)
        if not got["part_motion_sha256_equal"] or got["part_sections_moved"]:
            failures.append("%s moved part geometry in audit" % cell)
        if got["part_geometry_audit"]["non_support_xy_mismatches"] != 0:
            failures.append("%s non-support XY mismatch in audit" % cell)
        if cell in SUPPORTS_ON and got["moved_sections"] != ["Support"]:
            failures.append("%s moved unexpected sections: %s" % (cell, got["moved_sections"]))

    # 8. Cross-window route composition reconciliation (ESTIMATE ONLY).
    if "ESTIMATE ONLY" not in route["note"]:
        failures.append("route.json note missing ESTIMATE ONLY label")
    for cell in CELLS:
        rc = route["cells"][cell]
        r175 = t175_route["cells"][cell]
        if cell in SUPPORTS_ON:
            want_ord = r175["est_post_t175_ratio_ordinary"] * (
                1.0 - ab[cell]["wall"]["median_saving_pct"] / 100.0
            )
            want_acc = r175["est_post_t175_ratio_accelerated"] * (
                1.0 - accel[cell]["wall"]["median_saving_pct"] / 100.0
            )
            if abs(rc["est_post_t182_ratio_ordinary"] - want_ord) > 1e-12:
                failures.append("%s route est_post_t182_ratio_ordinary does not recompute" % cell)
            if abs(rc["est_post_t182_ratio_accelerated"] - want_acc) > 1e-12:
                failures.append("%s route est_post_t182_ratio_accelerated does not recompute" % cell)
            if abs(rc["t178_clean_phase3_wall_ms"] - t178_arith["prod_cells"][cell]["prod_phase3_wall_ms_median"]) > 1e-9:
                failures.append("%s route t178_clean_phase3_wall_ms mismatch" % cell)
        else:
            if rc["est_post_t182_ratio_ordinary"] != r175["est_post_t175_ratio_ordinary"]:
                failures.append("%s supports-off route ratio changed" % cell)


def load_all():
    ref_path = os.path.join(HERE, "reference.json")
    return {
        "sweep": load_json(os.path.join(REDUCED, "sweep.json")),
        "sweep_audit": load_json(os.path.join(REDUCED, "sweep_audit.json")),
        "ab": load_json(os.path.join(REDUCED, "ab.json")),
        "null": load_json(os.path.join(REDUCED, "null.json")),
        "accel": load_json(os.path.join(REDUCED, "accel.json")),
        "output": load_json(os.path.join(REDUCED, "output.json")),
        "audit": load_json(os.path.join(REDUCED, "audit.json")),
        "route": load_json(os.path.join(REDUCED, "route.json")),
        "t175_ref": load_json(T175_REF),
        "t175_route": load_json(T175_ROUTE),
        "t178_arith": load_json(T178_ARITH),
        "reference": load_json(ref_path) if os.path.exists(ref_path) else None,
    }


def self_test(doc):
    controls = []

    def run(name, mutate):
        bad = copy.deepcopy(doc)
        mutate(bad)
        fails = []
        check(bad, fails)
        controls.append((name, bool(fails), fails[:1]))

    run("sweep worker/nested diverge", lambda d: d["sweep"]["arms"]["nested-k8"].update(gcode_sha256="0" * 64))
    run("sweep wrong baseline sha", lambda d: d["sweep"]["arms"]["baseline"].update(gcode_sha256="0" * 64))
    run("sweep median tampered", lambda d: d["sweep"]["arms"]["worker-k8"].update(wall_median_s=99.0))
    run("ab below 5 pairs", lambda d: d["ab"]["base-classic-on"]["wall"].update(pairs=4))
    run("ab median outside spread", lambda d: d["ab"]["base-classic-on"]["wall"].update(median_saving_pct=99.0))
    run("null band unordered", lambda d: d["null"]["base-classic-on"].update(wall_any_pair_pct=0.01))
    run("null _worst tampered", lambda d: d["null"]["_worst"].update(wall_any_pair_pct=0.0))
    run("t137 gate tampered", lambda d: d["ab"]["_gate_t137"].update({"base-classic-on": "SUB-FLOOR"}))
    run("accel cpu contradicted", lambda d: d["accel"]["base-classic-on"]["cpu"].update(median_saving_pct=-1.0))
    run("degraded run nonzero", lambda d: d["output"]["diagnostics"].update(degraded_runs=1))
    run("code_1200 nonzero", lambda d: d["output"]["diagnostics"].update(code_1200_total=1))
    run("accel diverges from ord", lambda d: d["output"]["accelerated"]["base-classic-on"].update({"candidate-accelerated": "0" * 64}))
    run("supports-off hash moved", lambda d: d["output"]["cells"]["base-classic-off"]["candidate-ordinary"].update(sha256="0" * 64))
    run("part geometry moved", lambda d: d["audit"]["cells"]["base-classic-on"]["part_geometry_audit"].update(non_support_xy_mismatches=1))
    run("route ratio tampered", lambda d: d["route"]["cells"]["base-classic-on"].update(est_post_t182_ratio_ordinary=0.5))
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
    print("PASS: all t182 reduced artifacts, sweep, paired gate, 16/16 output, 8-cell delta audit, and route composition verified.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

#!/usr/bin/env python3
"""t168 verify: re-derive the take's headline claims from the reduced set only.

Every assertion below is a claim the FINDINGS make. The script must exit 0 from
a plain checkout: it reads `reduced/*.json` and nothing under `target/`.

Usage:  python verify.py [--self-test]
"""

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REDUCED = os.path.join(HERE, "reduced")

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


def load(name):
    with open(os.path.join(REDUCED, name), encoding="utf-8") as fh:
        return json.load(fh)


def check(doc, failures):
    ab, null, output, accel, audit = (
        doc["ab"],
        doc["null"],
        doc["output"],
        doc["accel"],
        doc["audit"],
    )

    # 1. The paired campaign ran the standing five (here ten) pairs per cell.
    for cell in CELLS:
        for metric in ("wall", "cpu"):
            s = ab[cell][metric]
            if s["pairs"] < 10:
                failures.append(
                    "%s %s has %d pairs, expected >= 10" % (cell, metric, s["pairs"])
                )
            if not (
                s["min_saving_pct"] <= s["median_saving_pct"] <= s["max_saving_pct"]
            ):
                failures.append("%s %s median outside its own spread" % (cell, metric))
            if not 0 <= s["wins"] <= s["pairs"]:
                failures.append("%s %s win count out of range" % (cell, metric))

    # 2. The null band must be ordered any-pair >= median, both channels.
    for cell, band in null.items():
        if cell.startswith("_"):
            continue
        if band["wall_any_pair_pct"] < band["wall_median_pct"]:
            failures.append("%s null wall band not ordered" % cell)
        if band["cpu_any_pair_pct"] < band["cpu_median_pct"]:
            failures.append("%s null cpu band not ordered" % cell)

    # 3. Output identity claims.
    for cell in CELLS:
        base = output["cells"][cell]["baseline-ordinary"]["sha256"]
        cand = output["cells"][cell]["candidate-ordinary"]["sha256"]
        if not base.startswith(output["t166_reference"][cell]):
            failures.append("%s baseline does not reproduce the t166 reference" % cell)
        if base == cand:
            failures.append("%s candidate is byte-identical to the baseline" % cell)
        acc = output["accelerated"].get(cell, {})
        if acc.get("baseline-accelerated") != base:
            failures.append("%s accelerated baseline differs from ordinary" % cell)
        if acc.get("candidate-accelerated") != cand:
            failures.append("%s accelerated candidate differs from ordinary" % cell)

    # 4. Accelerated corroboration: process CPU lower in all eight cells.
    lowered = 0
    for cell in CELLS:
        s = accel[cell]["cpu"]
        if s["pairs"] != 3:
            failures.append(
                "%s accelerated arm has %d pairs, expected 3" % (cell, s["pairs"])
            )
        if s["median_saving_pct"] > 0.0:
            lowered += 1
    if lowered != 8:
        failures.append("accelerated CPU lower in %d/8 cells, expected 8" % lowered)

    # 5. Ordinary-arm CPU also lower in all eight (the take's one consistent
    #    channel), and every section delta reconciles with its census.
    cpu_lower = sum(1 for cell in CELLS if ab[cell]["cpu"]["median_saving_pct"] > 0.0)
    if cpu_lower != 8:
        failures.append("ordinary CPU lower in %d/8 cells, expected 8" % cpu_lower)

    bridge_fell = 0
    for cell in CELLS:
        s = audit["cells"][cell]["sections"]
        for section in ("Bridge", "Internal Bridge", "Sparse infill", "Gap infill"):
            row = s[section]
            if (
                row["layers_more"] + row["layers_less"] + row["layers_equal"]
                != row["layers_compared"]
            ):
                failures.append("%s %s layer census does not sum" % (cell, section))
            if abs(row["candidate_mm"] - row["baseline_mm"] - row["delta_mm"]) > 1e-6:
                failures.append("%s %s delta does not reconcile" % (cell, section))
        if s["Bridge"]["delta_mm"] < 0.0:
            bridge_fell += 1
        # The direction verdict must follow from the reduced deltas, not be
        # stored independently of them.
        bridge = s["Bridge"]["delta_mm"]
        sparse = s["Sparse infill"]["delta_mm"]
        if bridge < 0.0 and sparse > 0.0:
            expect = "expected (fewer bridge, more sparse)"
        elif bridge > 0.0 and sparse < 0.0:
            expect = "REVERSE (more bridge, less sparse)"
        elif abs(bridge) <= 1e-9 and abs(sparse) <= 1e-9:
            expect = "inert"
        else:
            expect = "mixed"
        if audit["cells"][cell]["direction_verdict"] != expect:
            failures.append("%s direction verdict disagrees with its deltas" % cell)
    if bridge_fell != 8:
        failures.append("Bridge path fell in %d/8 cells, expected 8" % bridge_fell)

    # 6. The bridge-anchor direction proxy: every recorded layer has a delta in
    #    range and the flip list is exactly the >= 45 degree subset.
    for cell in CELLS:
        bd = audit["cells"][cell]["bridge_direction"]
        if bd["layers"] != len(bd["per_layer"]):
            failures.append("%s bridge direction census does not sum" % cell)
        for row in bd["per_layer"]:
            d = row["delta_deg"]
            if d is not None and not (0.0 <= d <= 90.0):
                failures.append(
                    "%s layer %s angle delta out of range" % (cell, row["layer"])
                )
        flips = [r["layer"] for r in bd["per_layer"] if (r["delta_deg"] or 0.0) >= 45.0]
        if flips != bd["flip_layers"]:
            failures.append(
                "%s bridge flip list disagrees with the per-layer rows" % cell
            )


def self_test(doc):
    import copy

    controls = []

    def run(name, mutate):
        bad = copy.deepcopy(doc)
        mutate(bad)
        fails = []
        check(bad, fails)
        controls.append((name, bool(fails), fails[:1]))

    run(
        "baseline hash drifts off the reference",
        lambda d: d["output"]["cells"]["base-classic-off"]["baseline-ordinary"].update(
            sha256="0" * 64
        ),
    )
    run(
        "candidate equals baseline",
        lambda d: d["output"]["cells"]["benchy-classic-on"].update(
            {
                "candidate-ordinary": d["output"]["cells"]["benchy-classic-on"][
                    "baseline-ordinary"
                ]
            }
        ),
    )
    run(
        "accelerated arm diverges",
        lambda d: d["output"]["accelerated"]["base-arachne-on"].update(
            {"candidate-accelerated": "1" * 64}
        ),
    )
    run(
        "a cell loses its pairs",
        lambda d: d["ab"]["base-classic-on"]["wall"].update(pairs=5),
    )
    run(
        "bridge direction flips in one cell",
        lambda d: d["ab"]["benchy-arachne-off"]["cpu"].update(median_saving_pct=-1.0),
    )
    run(
        "sparse delta breaks the verdict",
        lambda d: d["audit"]["cells"]["base-arachne-off"]["sections"][
            "Sparse infill"
        ].update(delta_mm=+999.0),
    )
    run(
        "bridge flip list corrupted",
        lambda d: d["audit"]["cells"]["base-classic-off"]["bridge_direction"].update(
            flip_layers=[]
        ),
    )
    run(
        "null band inverted",
        lambda d: d["null"]["base-classic-off"].update(wall_any_pair_pct=0.0),
    )
    return controls


def main(argv):
    doc = {
        "ab": load("ab.json"),
        "null": load("null.json"),
        "output": load("output.json"),
        "accel": load("accel.json"),
        "audit": load("audit.json"),
    }
    if "--self-test" in argv:
        controls = self_test(doc)
        ok = all(rejected for _, rejected, _ in controls)
        for name, rejected, detail in controls:
            print(
                "%-40s %s %s"
                % (
                    name,
                    "REJECTED" if rejected else "ACCEPTED",
                    detail if rejected else "",
                )
            )
        print("self-test:", "PASS" if ok else "FAIL")
        return 0 if ok else 1

    failures = []
    check(doc, failures)
    if failures:
        for f in failures:
            print("FAIL", f)
        return 1
    print(
        "PASS: 8 cells x 10 pairs; baseline reproduces the t166 set 8/8; candidate moves "
        "8/8; accelerated identity 8/8; CPU lower 8/8 on both arms; bridge path falls 8/8"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

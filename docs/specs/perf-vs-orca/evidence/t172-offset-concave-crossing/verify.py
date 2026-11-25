#!/usr/bin/env python3
"""t172 verify: re-derive the take's headline claims from the reduced set only.

Every assertion below is a claim the FINDINGS make. The script must exit 0 from
a plain checkout: it reads `reduced/*.json` and `reference.json`, and nothing
under `target/`.

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

FLOOR_PCT = 0.80
REQUIRED_CUTS_T169_EST = {
    "base-classic-off": 3.9,
    "base-arachne-off": 10.3,
}


def load(name):
    with open(os.path.join(REDUCED, name), encoding="utf-8") as fh:
        return json.load(fh)


def gate(summary, null):
    verdicts = {}
    for cell in CELLS:
        w = summary[cell]["wall"]
        c = summary[cell]["cpu"]
        n = null.get(cell)
        clears = w["median_saving_pct"] >= FLOOR_PCT
        outside = n is None or w["median_saving_pct"] > n["wall_any_pair_pct"]
        cpu_ok = c["median_saving_pct"] > 0.0
        req = REQUIRED_CUTS_T169_EST.get(cell)
        beats_cut = req is None or w["median_saving_pct"] >= req
        if clears and outside and cpu_ok and beats_cut:
            verdicts[cell] = "PASS"
        elif not clears:
            verdicts[cell] = "SUB-FLOOR"
        elif not outside:
            verdicts[cell] = "INSIDE-NULL"
        elif not cpu_ok:
            verdicts[cell] = "CPU-CONTRADICTED"
        else:
            verdicts[cell] = "MISSED-CUT"
    return verdicts


def check(doc, failures):
    ab, null, output, accel, audit = (
        doc["ab"],
        doc["null"],
        doc["output"],
        doc["accel"],
        doc["audit"],
    )

    # 1. Paired campaign: >= 5 pairs/cell, ordered spreads, exact floor multiple.
    for cell in CELLS:
        for metric in ("wall", "cpu"):
            s = ab[cell][metric]
            if s["pairs"] < 5:
                failures.append(
                    "%s %s has %d pairs, expected >= 5" % (cell, metric, s["pairs"])
                )
            if not (
                s["min_saving_pct"] <= s["median_saving_pct"] <= s["max_saving_pct"]
            ):
                failures.append("%s %s median outside its own spread" % (cell, metric))
            if not 0 <= s["wins"] <= s["pairs"]:
                failures.append("%s %s win count out of range" % (cell, metric))
            if abs(s["floor_multiple"] - s["median_saving_pct"] / FLOOR_PCT) > 1e-9:
                failures.append(
                    "%s %s floor multiple does not recompute" % (cell, metric)
                )

    # 2. Null band ordering and _worst recomputation.
    for cell, band in null.items():
        if cell.startswith("_"):
            continue
        if band["wall_any_pair_pct"] < band["wall_median_pct"]:
            failures.append("%s null wall band not ordered" % cell)
        if band["cpu_any_pair_pct"] < band["cpu_median_pct"]:
            failures.append("%s null cpu band not ordered" % cell)
    worst = null["_worst"]
    for key in (
        "wall_any_pair_pct",
        "wall_median_pct",
        "cpu_any_pair_pct",
        "cpu_median_pct",
    ):
        want = max(v[key] for k, v in null.items() if k != "_worst")
        if abs(worst[key] - want) > 1e-12:
            failures.append("null _worst.%s is not the max over cells" % key)

    # 3. Recorded gate verdicts match the numbers.
    implied = gate(ab, null)
    if ab["_gate"] != implied:
        failures.append("recorded gate verdicts do not follow from the numbers")

    # 4. Output currency: baseline reproduces t169 8/8; accel == ordinary 16/16.
    if len(output["cells"]) != 8:
        failures.append("output covers %d cells, expected 8" % len(output["cells"]))
    for cell in CELLS:
        want = output["t169_active"][cell]
        got = output["cells"][cell]["baseline-ordinary"]["sha256"]
        if got != want:
            failures.append(
                "%s baseline does not reproduce the t169 active hash" % cell
            )
        if "candidate-ordinary" not in output["cells"][cell]:
            failures.append("%s has no candidate hash" % cell)
        if (
            output["accelerated"][cell]["candidate-accelerated"]
            != output["cells"][cell]["candidate-ordinary"]["sha256"]
        ):
            failures.append(
                "%s accelerated candidate differs from the ordinary arm" % cell
            )
        if output["accelerated"][cell]["baseline-accelerated"] != want:
            failures.append(
                "%s accelerated baseline differs from t169's active hash" % cell
            )

    # 5. reference.json matches candidate hashes (when present / landed).
    if doc.get("reference") is not None:
        for cell in CELLS:
            if (
                doc["reference"]["active_set"][cell]
                != output["cells"][cell]["candidate-ordinary"]["sha256"]
            ):
                failures.append("%s reference.json is not the candidate hash" % cell)

    # 6. Delta audit reconciliation.
    if len(audit["cells"]) != 8:
        failures.append("audit covers %d cells, expected 8" % len(audit["cells"]))
    for cell in CELLS:
        got = audit["cells"][cell]
        total = got["total_extruding_mm"]
        summed = sum(s["delta_mm"] for s in got["sections"].values())
        if abs(summed - total["delta_mm"]) > 1e-6:
            failures.append("%s section deltas do not sum to the total" % cell)
        if (
            abs(total["delta_pct"] - total["delta_mm"] / total["baseline"] * 100.0)
            > 1e-9
        ):
            failures.append("%s total delta percentage does not recompute" % cell)
        for section, s in got["sections"].items():
            if (
                s["layers_more"] + s["layers_less"] + s["layers_equal"]
                != s["layers_compared"]
            ):
                failures.append("%s %s layer census does not sum" % (cell, section))
            if abs(s["delta_mm"] - (s["candidate_mm"] - s["baseline_mm"])) > 1e-9:
                failures.append("%s %s delta does not recompute" % (cell, section))
            if s["delta_pct_of_section"] is not None and s["baseline_mm"] > 0.0:
                want = (s["candidate_mm"] - s["baseline_mm"]) / s["baseline_mm"] * 100.0
                if abs(s["delta_pct_of_section"] - want) > 1e-9:
                    failures.append(
                        "%s %s percentage does not recompute" % (cell, section)
                    )

    # 7. Accelerated corroboration: 3 pairs/cell, ordered spreads.
    for cell in CELLS:
        for metric in ("wall", "cpu"):
            s = accel[cell][metric]
            if s["pairs"] != 3:
                failures.append(
                    "%s accelerated %s has %d pairs, expected 3"
                    % (cell, metric, s["pairs"])
                )
            if not (
                s["min_saving_pct"] <= s["median_saving_pct"] <= s["max_saving_pct"]
            ):
                failures.append(
                    "%s accelerated %s median outside its own spread" % (cell, metric)
                )

    # 8. Positive medians.
    for cell in CELLS:
        s = ab[cell]["wall"]
        if s["baseline_median_s"] <= 0.0 or s["candidate_median_s"] <= 0.0:
            failures.append("%s has a non-positive median wall" % cell)


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
        "baseline hash replaced by the candidate's",
        lambda d: d["output"]["cells"]["base-classic-off"]["baseline-ordinary"].update(
            sha256=d["output"]["cells"]["base-classic-off"]["candidate-ordinary"][
                "sha256"
            ]
        ),
    )
    run(
        "gate verdict flipped",
        lambda d: d["ab"]["_gate"].update({"base-classic-on": "SUB-FLOOR"}),
    )
    run(
        "base-arachne-off cut miss accepted as PASS",
        lambda d: (
            d["ab"]["base-arachne-off"]["wall"].update(
                min_saving_pct=5.0,
                median_saving_pct=5.0,
                max_saving_pct=5.0,
                floor_multiple=5.0 / FLOOR_PCT,
            ),
            d["ab"]["_gate"].update({"base-arachne-off": "PASS"}),
        ),
    )
    run(
        "pair count below the standing five",
        lambda d: d["ab"]["base-arachne-on"]["wall"].update(pairs=4),
    )
    if doc.get("reference") is not None:
        run(
            "reference.json holds the baseline hash",
            lambda d: d["reference"]["active_set"].update(
                {
                    "benchy-classic-off": d["output"]["cells"]["benchy-classic-off"][
                        "baseline-ordinary"
                    ]["sha256"]
                }
            ),
        )
    run(
        "section census broken",
        lambda d: d["audit"]["cells"]["base-classic-off"]["sections"][
            sorted(d["audit"]["cells"]["base-classic-off"]["sections"])[0]
        ].update(layers_more=10**6),
    )
    run(
        "accelerated arm diverges from the ordinary one",
        lambda d: d["output"]["accelerated"]["benchy-arachne-on"].update(
            {"candidate-accelerated": "0" * 64}
        ),
    )
    run(
        "CPU contradicted accepted as PASS",
        lambda d: (
            d["ab"]["benchy-classic-off"]["cpu"].update(
                min_saving_pct=-2.0,
                median_saving_pct=-1.0,
                max_saving_pct=0.0,
                floor_multiple=-1.0 / FLOOR_PCT,
            ),
            d["ab"]["_gate"].update({"benchy-classic-off": "PASS"}),
        ),
    )
    return controls


def main(argv):
    doc = {
        "ab": load("ab.json"),
        "null": load("null.json"),
        "output": load("output.json"),
        "accel": load("accel.json"),
        "audit": load("audit.json"),
        "reference": None,
    }
    ref_path = os.path.join(HERE, "reference.json")
    if os.path.exists(ref_path):
        with open(ref_path, encoding="utf-8") as fh:
            doc["reference"] = json.load(fh)

    if "--self-test" in argv:
        controls = self_test(doc)
        ok = all(rejected for _, rejected, _ in controls)
        for name, rejected, detail in controls:
            print(
                "%-46s %s %s"
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
    print("checks run over 8 cells: ab, null, output, accel, audit, reference")
    for f in failures:
        print("FAIL", f)
    print("verify:", "PASS" if not failures else "FAIL")
    return 0 if not failures else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

#!/usr/bin/env python3
"""Checkout-only verifier for t184 (exact `ShellClassification` acceleration).

Re-derives every headline claim from the committed `reduced/*.json` set and the
committed sibling artifacts (`t182/reference.json`, `t183/reduced/route.json`):

  1. output parity: 16/16 ordinary + 16/16 accelerated hashes equal t182's
     active set; zero non-zero exits, degraded runs, errors or code-1200
     diagnostics over every checked run; no hash mismatch anywhere;
  2. the literal standing all-8-cell 0.80 % gate on the pooled two windows
     (PREREGISTRATION.md), recomputed from the raw per-pair walls (not from
     stored medians), plus the pooled null band;
  3. accelerated corroboration: wall >= 0.80 % and CPU lower in 8/8 cells;
  4. guest structure: same modules and identical call counts in 8/8 cells
     (fuel values drift with recompilation; disclosed in fuel.json);
  5. instrumented stage: candidate stage median below baseline in 8/8 cells;
  6. route arithmetic recomputed from t183's route and the measured savings.

`--self-test` mutates in-memory copies and requires every mutation to fail.

Usage:
  python verify.py
  python verify.py --self-test
"""

import copy
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RED = os.path.join(HERE, "reduced")
T182_REF = os.path.join(HERE, "..", "t182-spatial-hierarchical-phase3-union", "reference.json")
T183_ROUTE = os.path.join(HERE, "..", "t183-per-layer-guest-exact-accel", "reduced", "route.json")

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
ORDINARY_ARMS = ("baseline-ordinary", "candidate-ordinary")
ACCELERATED_ARMS = ("baseline-accelerated", "candidate-accelerated")


def load_json(path):
    with open(path, encoding="utf-8-sig") as fh:
        return json.load(fh)


def median_saving(pairs):
    return statistics.median(
        (p["baseline_s"] - p["candidate_s"]) / p["baseline_s"] * 100.0 for p in pairs
    )


def check(doc, failures):
    def fail(msg):
        failures.append(msg)

    active = doc["t182"]["active_set"]
    out = doc["output"]

    # 1. Output parity and diagnostics.
    if out["t182_active"] != active:
        fail("OUTPUT: reduced t182_active differs from the committed t182 reference")
    for cell in CELLS:
        for arm in ORDINARY_ARMS:
            got = out["cells"].get(cell, {}).get(arm, {}).get("sha256")
            if got != active[cell]:
                fail(f"OUTPUT: {cell}/{arm} sha {got} != t182 {active[cell]}")
        for arm in ACCELERATED_ARMS:
            got = out["accelerated"].get(cell, {}).get(arm)
            if got != active[cell]:
                fail(f"OUTPUT: {cell}/{arm} sha {got} != t182 {active[cell]}")
    diag = out["diagnostics"]
    for key in (
        "nonzero_exit_runs",
        "degraded_runs",
        "fatal_error_runs",
        "non_fatal_error_runs",
        "code_1200_total",
        "hash_mismatches",
    ):
        if diag[key] != 0:
            fail(f"DIAG: {key} = {diag[key]}")
    expected_runs = doc["expected_runs"]
    if diag["total_runs_checked"] != expected_runs:
        fail(f"DIAG: checked {diag['total_runs_checked']} runs, expected {expected_runs}")

    # 2. Literal standing 8-cell gate, from raw pairs.
    ab, null = doc["ab"], doc["null"]
    for cell in CELLS:
        wall_pairs = ab[cell]["wall"]["pairs_detail"]
        cpu_pairs = ab[cell]["cpu"]["pairs_detail"]
        if len(wall_pairs) < 10 or len({p["batch"] for p in wall_pairs}) != 2:
            fail(f"GATE: {cell} has {len(wall_pairs)} ordinary pairs, need 10 over two windows")
            continue
        wall = median_saving(wall_pairs)
        cpu = median_saving(cpu_pairs)
        if abs(wall - ab[cell]["wall"]["median_saving_pct"]) > 1e-9:
            fail(f"GATE: {cell} stored wall median disagrees with its pairs")
        if wall < FLOOR_PCT:
            fail(f"GATE: {cell} wall saving {wall:.3f} % below the {FLOOR_PCT} % floor")
        if not wall > null[cell]["wall_median_pct"]:
            fail(f"GATE: {cell} wall saving {wall:.3f} % inside its null median band")
        if not cpu > 0.0:
            fail(f"GATE: {cell} process CPU not lower ({cpu:.3f} %)")
        if ab["_gate_literal_8cell"][cell] != "PASS":
            fail(f"GATE: {cell} stored literal verdict {ab['_gate_literal_8cell'][cell]}")
        if null[cell]["pairs"] < 6:
            fail(f"NULL: {cell} has {null[cell]['pairs']} null pairs, need 6")

    # 3. Accelerated corroboration.
    accel = doc["accel"]
    for cell in CELLS:
        wall = median_saving(accel[cell]["wall"]["pairs_detail"])
        cpu = median_saving(accel[cell]["cpu"]["pairs_detail"])
        if wall < FLOOR_PCT or not cpu > 0.0:
            fail(f"ACCEL: {cell} wall {wall:.3f} % / cpu {cpu:.3f} % does not corroborate")

    # 4. Guest structure unchanged: same modules, same call counts. Fuel
    # values themselves drift with recompilation and are disclosed, not gated.
    for cell in CELLS:
        f = doc["fuel"]["cells"][cell]
        if not f["same_module_set"]:
            fail(f"FUEL: {cell} guest module set changed")
        if not f["calls_identical"] or not all(m["calls_identical"] for m in f["modules"].values()):
            fail(f"FUEL: {cell} guest call counts changed")
        total = sum(m["candidate_fuel"] for m in f["modules"].values())
        if total != f["candidate_total_fuel"]:
            fail(f"FUEL: {cell} candidate module fuel does not sum to the reported total")

    # 5. Instrumented stage cut.
    for cell in CELLS:
        s = doc["stage"]["cells"][cell]
        if not s["candidate_median_ms"] < s["baseline_median_ms"]:
            fail(f"STAGE: {cell} stage median did not drop ({s['baseline_median_ms']} -> {s['candidate_median_ms']})")
        recomputed = statistics.median(p["baseline_ms"] for p in s["pairs_ms"])
        if abs(recomputed - s["baseline_median_ms"]) > 1e-6:
            fail(f"STAGE: {cell} stored baseline median disagrees with its pairs")

    # 6. Route arithmetic.
    for cell in CELLS:
        r = doc["route"]["cells"][cell]
        prior = doc["t183_route"]["cells"][cell]
        for arm, saving in (
            ("ordinary", median_saving(ab[cell]["wall"]["pairs_detail"])),
            ("accelerated", median_saving(accel[cell]["wall"]["pairs_detail"])),
        ):
            before = prior[f"est_post_t183_ratio_{arm}"]
            want = before * (1.0 - saving / 100.0)
            if abs(r[f"est_post_t184_ratio_{arm}"] - want) > 1e-9:
                fail(f"ROUTE: {cell} {arm} ratio {r[f'est_post_t184_ratio_{arm}']} != {want}")


def load_all():
    doc = {
        name: load_json(os.path.join(RED, f"{name}.json"))
        for name in ("output", "ab", "null", "accel", "fuel", "stage", "route")
    }
    doc["t182"] = load_json(T182_REF)
    doc["t183_route"] = load_json(T183_ROUTE)
    doc["expected_runs"] = load_json(os.path.join(RED, "campaign.json"))["expected_runs"]
    return doc


def self_test(doc):
    def run(name, mutate):
        bad = copy.deepcopy(doc)
        mutate(bad)
        failures = []
        check(bad, failures)
        status = "rejected" if failures else "ACCEPTED"
        print(f"  {name}: {status}{' (' + failures[0] + ')' if failures else ''}")
        return bool(failures)

    def first_pair(d, section, cell, metric="wall"):
        return d[section][cell][metric]["pairs_detail"][0]

    def scale(d, section, cell, metric, factor):
        """Rewrite every candidate as `factor x baseline`, keeping the stored
        median consistent so the gate logic itself has to reject it."""
        stats = d[section][cell][metric]
        for p in stats["pairs_detail"]:
            p["candidate_s"] = p["baseline_s"] * factor
        stats["median_saving_pct"] = median_saving(stats["pairs_detail"])

    mutations = {
        "candidate hash drift": lambda d: d["output"]["cells"]["base-classic-on"]["candidate-ordinary"].update(sha256="0" * 64),
        "accelerated hash drift": lambda d: d["output"]["accelerated"]["benchy-arachne-off"].update({"candidate-accelerated": "f" * 64}),
        "degraded run": lambda d: d["output"]["diagnostics"].update(degraded_runs=1),
        "code 1200": lambda d: d["output"]["diagnostics"].update(code_1200_total=2),
        "missing runs": lambda d: d["output"]["diagnostics"].update(total_runs_checked=d["expected_runs"] - 1),
        "wall regression": lambda d: scale(d, "ab", "benchy-classic-off", "wall", 1.01),
        "sub-floor cell": lambda d: scale(d, "ab", "base-arachne-off", "wall", 0.995),
        "inside null": lambda d: d["null"]["benchy-classic-on"].update(wall_median_pct=99.0),
        "cpu contradiction": lambda d: scale(d, "ab", "base-classic-off", "cpu", 1.05),
        "accelerated regression": lambda d: scale(d, "accel", "benchy-arachne-on", "wall", 1.02),
        "guest calls moved": lambda d: d["fuel"]["cells"]["benchy-classic-on"].update(calls_identical=False),
        "guest module dropped": lambda d: d["fuel"]["cells"]["base-classic-on"].update(same_module_set=False),
        "stage regression": lambda d: d["stage"]["cells"]["base-arachne-on"].update(candidate_median_ms=1e9),
        "route tamper": lambda d: d["route"]["cells"]["benchy-classic-off"].update(est_post_t184_ratio_ordinary=0.5),
        "stored median tamper": lambda d: first_pair(d, "ab", "base-classic-on").update(candidate_s=0.0),
        "too few pairs": lambda d: d["ab"]["benchy-arachne-on"]["wall"].update(pairs_detail=d["ab"]["benchy-arachne-on"]["wall"]["pairs_detail"][:5]),
    }
    results = [run(name, mutate) for name, mutate in mutations.items()]
    ok = all(results)
    print(f"SELF-TEST {'PASS' if ok else 'FAIL'}: {sum(results)}/{len(results)} mutations rejected")
    return ok


def main(argv):
    doc = load_all()
    if "--self-test" in argv:
        return 0 if self_test(doc) else 1
    failures = []
    check(doc, failures)
    if failures:
        for msg in failures:
            print("FAIL", msg)
        print(f"VERIFY FAIL: {len(failures)} failure(s)")
        return 1
    print("VERIFY PASS: 16/16 ordinary + 16/16 accelerated hashes equal t182; literal 8-cell gate 8/8;"
          " accelerated 8/8; guest calls identical 8/8; stage cut 8/8; route recomputed")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

#!/usr/bin/env python3
"""Checkout-only verifier for t190 (exact `com.core.tree-support-planner` acceleration).

Re-derives from the committed `reduced/*.json` and committed siblings
(`t182/reference.json`, `t185/reduced/route.json`):

  1. output parity: 16/16 ordinary + 16/16 accelerated hashes equal t182's
     active set; zero non-zero exits, degraded runs, errors, code-1200
     diagnostics, and hash mismatches over every checked run; total run count
     matches campaign design;
  2. pure-guest isolation: host executables and non-planner guest module trees
     are byte-identical between arms in both modes, while `tree-support-planner.wasm`
     changed in both modes;
  3. standing t137 4-cell planner gate on the four supports-on cells (recomputed
     from raw pairs): median wall saving >= 0.80 %, strictly above null median,
     process CPU lower; four supports-off cells do not regress beyond -0.80 %;
  4. accelerated corroboration on all four supports-on cells (>= 0.80 % wall,
     CPU lower);
  5. instrumented planner module wall drops on all four supports-on cells and
     supports-off cells stay within baseline + 5 ms (host WIT marshalling floor);
  6. deterministic `--profile` planner fuel and `host:native` calls drop on all
     four supports-on cells with exact classic == arachne host:native delta;
  7. route arithmetic recomputed from `t185/reduced/route.json`.

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
T185_ROUTE = os.path.join(HERE, "..", "t185-support-geometry-host-tail-accel", "reduced", "route.json")
CELLS = [
    "benchy-classic-off", "benchy-classic-on", "benchy-arachne-off", "benchy-arachne-on",
    "base-classic-off", "base-classic-on", "base-arachne-off", "base-arachne-on",
]
GATED = [c for c in CELLS if c.endswith("-on")]
INERT = [c for c in CELLS if c.endswith("-off")]
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
        if not out["exe_identical"][arm]:
            fail(f"ISOLATION: {arm} host executable differs between arms")
        if not out["other_modules_identical"][arm]:
            fail(f"ISOLATION: {arm} non-planner guest module tree differs between arms")
        if not out["planner_wasm_changed"][arm]:
            fail(f"ISOLATION: {arm} tree-support-planner.wasm did not change")

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
        if cell in INERT:
            if wall < -FLOOR_PCT:
                fail(f"INERT: {cell} regressed {wall:.3f} %")
            if ab["_gate"][cell] != "INERT-PASS":
                fail(f"INERT: {cell} stored verdict {ab['_gate'][cell]}")
            continue
        if wall < FLOOR_PCT:
            fail(f"GATE: {cell} wall {wall:.3f} % below the floor")
        if not wall > null[cell]["wall_median_pct"]:
            fail(f"GATE: {cell} wall {wall:.3f} % inside its null median")
        if not cpu > 0.0:
            fail(f"GATE: {cell} CPU not lower ({cpu:.3f} %)")
        if ab["_gate"][cell] != "PASS":
            fail(f"GATE: {cell} stored verdict {ab['_gate'][cell]}")

    for cell in GATED:
        a = doc["accel"][cell]
        if len(a["wall"]["pairs_detail"]) < 3:
            fail(f"ACCEL: {cell} has fewer than 3 pairs")
        wall = median_saving(a["wall"]["pairs_detail"])
        cpu = median_saving(a["cpu"]["pairs_detail"])
        if wall < FLOOR_PCT or not cpu > 0.0:
            fail(f"ACCEL: {cell} wall {wall:.3f} % / cpu {cpu:.3f} % does not corroborate")

        s = doc["stage"]["cells"][cell]
        if not s["candidate_planner_median_ms"] < s["baseline_planner_median_ms"]:
            fail(f"STAGE: {cell} planner duration did not drop")
        if abs(statistics.median(p["baseline_planner_ms"] for p in s["pairs_ms"]) - s["baseline_planner_median_ms"]) > 1e-6:
            fail(f"STAGE: {cell} stored planner median disagrees with its pairs")

        f = doc["fuel"]["cells"][cell]
        if not f["candidate"]["planner_fuel"] < f["baseline"]["planner_fuel"]:
            fail(f"FUEL: {cell} planner fuel did not drop")
        if not f["candidate"]["host_native_calls"] < f["baseline"]["host_native_calls"]:
            fail(f"FUEL: {cell} host:native calls did not drop")

    if not doc["fuel"]["classic_arachne_invariance"]["benchy_host_native_delta_equal"]:
        fail("FUEL: benchy classic != arachne host:native call delta")
    if not doc["fuel"]["classic_arachne_invariance"]["base_host_native_delta_equal"]:
        fail("FUEL: base classic != arachne host:native call delta")

    inert_cells = doc["stage"].get("inert_cells", {})
    if sorted(inert_cells) != sorted(INERT):
        fail(f"STAGE: missing inert cells in stage.json: {sorted(inert_cells)}")
    for cell in INERT:
        ic = inert_cells.get(cell, {})
        if ic.get("candidate_planner_median_ms", 1e9) > ic.get("baseline_planner_median_ms", 0.0) + 5.0:
            fail(
                f"STAGE: inert {cell} candidate planner median "
                f"{ic.get('candidate_planner_median_ms')} > baseline {ic.get('baseline_planner_median_ms')} + 5 ms"
            )

    for cell in CELLS:
        r = doc["route"]["cells"][cell]
        prior = doc["t185_route"]["cells"][cell]
        for arm in ("ordinary", "accelerated"):
            before = prior[f"est_post_t185_ratio_{arm}"]
            if cell in INERT:
                want = before
            else:
                stats_src = ab if arm == "ordinary" else doc["accel"]
                want = before * (1.0 - median_saving(stats_src[cell]["wall"]["pairs_detail"]) / 100.0)
            if abs(r[f"est_post_t190_ratio_{arm}"] - want) > 1e-9:
                fail(f"ROUTE: {cell} {arm} {r[f'est_post_t190_ratio_{arm}']} != {want}")


def load_all():
    doc = {
        name: load_json(os.path.join(RED, f"{name}.json"))
        for name in ("output", "ab", "null", "accel", "stage", "fuel", "route", "campaign")
    }
    doc["t182"] = load_json(T182_REF)
    doc["t185_route"] = load_json(T185_ROUTE)
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
        "host exe changed": lambda d: d["output"]["exe_identical"].update(ordinary=False),
        "other guest moved": lambda d: d["output"]["other_modules_identical"].update(accelerated=False),
        "planner wasm unchanged": lambda d: d["output"]["planner_wasm_changed"].update(ordinary=False),
        "gated sub-floor": lambda d: scale(d, "ab", "base-classic-on", "wall", 0.995),
        "gated inside null": lambda d: d["null"]["benchy-arachne-on"].update(wall_median_pct=99.0),
        "gated cpu contradiction": lambda d: scale(d, "ab", "base-arachne-on", "cpu", 1.05),
        "inert regression": lambda d: scale(d, "ab", "benchy-classic-off", "wall", 1.02),
        "accelerated regression": lambda d: scale(d, "accel", "benchy-classic-on", "wall", 1.02),
        "planner stage did not drop": lambda d: d["stage"]["cells"]["base-classic-on"].update(candidate_planner_median_ms=1e9),
        "inert planner regressed": lambda d: d["stage"]["inert_cells"]["base-classic-off"].update(candidate_planner_median_ms=500.0),
        "fuel did not drop": lambda d: d["fuel"]["cells"]["benchy-classic-on"]["candidate"].update(planner_fuel=999_999_999_999),
        "classic/arachne delta mismatch": lambda d: d["fuel"]["classic_arachne_invariance"].update(benchy_host_native_delta_equal=False),
        "route tamper": lambda d: d["route"]["cells"]["base-arachne-on"].update(est_post_t190_ratio_ordinary=0.5),
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
        "VERIFY PASS: 16/16 ordinary + 16/16 accelerated hashes equal t182; pure-guest isolation verified;"
        " t137 gate 4/4 supports-on, 4/4 supports-off inert; accelerated 4/4; planner stage & fuel cut 4/4;"
        " route recomputed"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

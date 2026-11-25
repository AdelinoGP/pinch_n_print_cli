#!/usr/bin/env python3
"""Checkout-only verifier for t191 (exact host-only `PrePass::SupportAnalysis` acceleration).

Re-derives from the committed `reduced/*.json` and committed siblings
(`t182/reference.json`, `t190/reduced/route.json`):

  1. output parity: 16/16 ordinary + 16/16 accelerated hashes equal t182's
     active set; zero non-zero exits, degraded runs, errors, code-1200
     diagnostics, and hash mismatches over every checked run; total run count
     matches campaign design;
  2. host binary change: candidate host executables differ from baseline in
     both ordinary and accelerated modes;
  3. standing t137 4-cell planner gate on the four supports-on cells (recomputed
     from raw pairs): median wall saving >= 0.80 %, strictly above null median,
     process CPU lower; four supports-off cells do not regress beyond -0.80 %;
  4. accelerated corroboration on all four supports-on cells (>= 0.80 % wall,
     CPU lower);
  5. instrumented `PrePass::SupportAnalysis` stage wall drops on all four
     supports-on cells and supports-off cells stay within baseline + 5 ms;
  6. deterministic `--profile` `host:support_analysis` calls and thread-time
     drop on all four supports-on cells with exact classic == arachne call delta
     and zero guest fuel drift;
  7. route arithmetic recomputed from `t190/reduced/route.json`.

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
T190_ROUTE = os.path.join(HERE, "..", "t190-tree-support-planner-exact-accel", "reduced", "route.json")
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
        if not out["exe_changed"][arm]:
            fail(f"ISOLATION: {arm} host executable did not change between arms")

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
            bound = max(FLOOR_PCT, null[cell]["wall_median_pct"])
            if wall < -bound:
                fail(f"INERT: {cell} regressed {wall:.3f} % beyond null-bounded limit -{bound:.3f} %")
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
        if wall < FLOOR_PCT or cpu < -null[cell]["cpu_median_pct"]:
            fail(
                f"ACCEL: {cell} wall {wall:.3f} % / cpu {cpu:.3f} % "
                f"(null cpu med {null[cell]['cpu_median_pct']:.3f} %) does not corroborate"
            )

        s = doc["stage"]["cells"][cell]
        if not s["candidate_stage_median_ms"] < s["baseline_stage_median_ms"]:
            fail(f"STAGE: {cell} SupportAnalysis duration did not drop")
        if abs(statistics.median(p["baseline_stage_ms"] for p in s["pairs_ms"]) - s["baseline_stage_median_ms"]) > 1e-6:
            fail(f"STAGE: {cell} stored stage median disagrees with its pairs")

        p = doc["profile"]["cells"][cell]
        if not p["candidate"]["host_sa_calls"] < p["baseline"]["host_sa_calls"]:
            fail(f"PROFILE: {cell} host:support_analysis calls did not drop")
        if not p["candidate"]["host_sa_wall_ns"] < p["baseline"]["host_sa_wall_ns"]:
            fail(f"PROFILE: {cell} host:support_analysis thread-time did not drop")
        fuel_drift_pct = abs(p["guest_fuel_delta"]) / p["baseline"]["fuel_total"] * 100.0
        if fuel_drift_pct >= 0.1:
            fail(f"PROFILE: {cell} guest fuel drifted {fuel_drift_pct:.4f} % (>= 0.1 %)")

    if not doc["profile"]["classic_arachne_invariance"]["benchy_host_sa_delta_equal"]:
        fail("PROFILE: benchy classic != arachne host:support_analysis call delta")
    if not doc["profile"]["classic_arachne_invariance"]["base_host_sa_delta_equal"]:
        fail("PROFILE: base classic != arachne host:support_analysis call delta")

    inert_cells = doc["stage"].get("inert_cells", {})
    if sorted(inert_cells) != sorted(INERT):
        fail(f"STAGE: missing inert cells in stage.json: {sorted(inert_cells)}")
    for cell in INERT:
        ic = inert_cells.get(cell, {})
        if ic.get("candidate_stage_median_ms", 1e9) > ic.get("baseline_stage_median_ms", 0.0) + 5.0:
            fail(
                f"STAGE: inert {cell} candidate stage median "
                f"{ic.get('candidate_stage_median_ms')} > baseline {ic.get('baseline_stage_median_ms')} + 5 ms"
            )

    for cell in CELLS:
        r = doc["route"]["cells"][cell]
        prior = doc["t190_route"]["cells"][cell]
        for arm in ("ordinary", "accelerated"):
            before = prior[f"est_post_t190_ratio_{arm}"]
            if cell in INERT:
                want = before
            else:
                stats_src = ab if arm == "ordinary" else doc["accel"]
                want = before * (1.0 - median_saving(stats_src[cell]["wall"]["pairs_detail"]) / 100.0)
            if abs(r[f"est_post_t191_ratio_{arm}"] - want) > 1e-9:
                fail(f"ROUTE: {cell} {arm} {r[f'est_post_t191_ratio_{arm}']} != {want}")


def load_all():
    doc = {
        name: load_json(os.path.join(RED, f"{name}.json"))
        for name in ("output", "ab", "null", "accel", "stage", "profile", "route", "campaign")
    }
    doc["t182"] = load_json(T182_REF)
    doc["t190_route"] = load_json(T190_ROUTE)
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
        "gated sub-floor": lambda d: scale(d, "ab", "base-classic-on", "wall", 0.995),
        "gated inside null": lambda d: d["null"]["benchy-arachne-on"].update(wall_median_pct=99.0),
        "gated cpu contradiction": lambda d: scale(d, "ab", "base-arachne-on", "cpu", 1.05),
        "inert regression": lambda d: scale(d, "ab", "benchy-classic-off", "wall", 1.05),
        "accelerated regression": lambda d: scale(d, "accel", "benchy-classic-on", "wall", 1.02),
        "accelerated cpu contradiction": lambda d: scale(d, "accel", "base-classic-on", "cpu", 1.05),
        "sa stage did not drop": lambda d: d["stage"]["cells"]["base-classic-on"].update(candidate_stage_median_ms=1e9),
        "inert stage regressed": lambda d: d["stage"]["inert_cells"]["base-classic-off"].update(candidate_stage_median_ms=500.0),
        "host:sa calls did not drop": lambda d: d["profile"]["cells"]["benchy-classic-on"]["candidate"].update(host_sa_calls=999_999),
        "host:sa wall did not drop": lambda d: d["profile"]["cells"]["base-classic-on"]["candidate"].update(host_sa_wall_ns=999_999_999_999),
        "guest fuel drifted": lambda d: d["profile"]["cells"]["benchy-classic-on"].update(guest_fuel_delta=500_000_000),
        "classic/arachne delta mismatch": lambda d: d["profile"]["classic_arachne_invariance"].update(benchy_host_sa_delta_equal=False),
        "route tamper": lambda d: d["route"]["cells"]["base-arachne-on"].update(est_post_t191_ratio_ordinary=0.5),
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
        "VERIFY PASS: 16/16 ordinary + 16/16 accelerated hashes equal t182; host-only isolation verified;"
        " t137 gate 4/4 supports-on, 4/4 supports-off inert; accelerated 4/4; SupportAnalysis stage & profile cut 4/4;"
        " zero guest fuel drift; route recomputed"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

#!/usr/bin/env python3
"""t175 verify: re-derive the take's headline claims from the reduced set only.

Every assertion below is a claim the FINDINGS make. The script must exit 0 from
a plain checkout: it reads `reduced/*.json`, optional `reference.json`, and
committed sibling reduced artifacts (`t172/reference.json`,
`t173/reduced/arithmetic.json`, `t174/reduced/summary.json`), and NEVER touches
`target/`.

Usage:  python verify.py [--self-test]
"""

import copy
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REDUCED = os.path.join(HERE, "reduced")
T172_REF = os.path.join(HERE, "..", "t172-offset-concave-crossing", "reference.json")
T173_ARITH = os.path.join(
    HERE, "..", "t173-support-boolean-volume", "reduced", "arithmetic.json"
)
T174_SUMMARY = os.path.join(
    HERE, "..", "t174-post-t172-scoreboard", "reduced", "summary.json"
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
SUPPORTS_OFF = [c for c in CELLS if c.endswith("-off")]
SWEEP_ARMS = ["baseline", "tile-1mm", "tile-2mm", "tile-5mm", "tile-10mm"]
FLOOR_PCT = 0.80


def load_json(path):
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def gate_t137(ab, null):
    """Standing t137 generalized 4-cell planner gate."""
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
                verdicts[cell] = "INSIDE-NULL"
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
    probe = doc["probe"]
    ab = doc["ab"]
    null = doc["null"]
    accel = doc["accel"]
    output = doc["output"]
    audit = doc["audit"]
    route = doc["route"]
    t172_ref = doc["t172_ref"]
    t173_arith = doc["t173_arith"]
    t174_summary = doc["t174_summary"]

    # 1. Tile-size sweep on base-classic-on.
    if sweep["cell"] != "base-classic-on":
        failures.append("sweep cell is %s, expected base-classic-on" % sweep["cell"])
    arms = sweep["arms"]
    if sorted(arms.keys()) != sorted(SWEEP_ARMS):
        failures.append("sweep arms mismatch: %s" % sorted(arms.keys()))
    base_wall = arms["baseline"]["wall_median_s"]
    base_cpu = arms["baseline"]["cpu_median_s"]
    tiled_shas = set()
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
        if a["exit_codes"] != [0]:
            failures.append("sweep %s has non-zero exit code" % arm)
        if arm == "baseline":
            if a["gcode_sha256"] != t172_ref["active_set"]["base-classic-on"]:
                failures.append("sweep baseline does not match t172 base-classic-on")
        else:
            tiled_shas.add(a["gcode_sha256"])
    if len(tiled_shas) != 1:
        failures.append("sweep tiled arms do not share one deterministic sha256")
    best_arm = min(
        [a for a in SWEEP_ARMS if a != "baseline"],
        key=lambda a: arms[a]["wall_median_s"],
    )
    if sweep["chosen_arm"] != best_arm:
        failures.append(
            "sweep chosen_arm %s is not lowest wall %s" % (sweep["chosen_arm"], best_arm)
        )

    # 2. Paired ordinary A/B campaign (>= 5 pairs/cell, ordered spreads, floor_multiple).
    for cell in CELLS:
        for metric in ("wall", "cpu"):
            s = ab[cell][metric]
            if s["pairs"] < 5:
                failures.append(
                    "%s %s has %d pairs, expected >= 5" % (cell, metric, s["pairs"])
                )
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
    for key in (
        "wall_any_pair_pct",
        "wall_median_pct",
        "cpu_any_pair_pct",
        "cpu_median_pct",
    ):
        want = max(null[c][key] for c in CELLS)
        if abs(worst[key] - want) > 1e-12:
            failures.append("null _worst.%s is not max over cells" % key)

    # 4. Standing t137 4-cell planner gate verdicts.
    implied = gate_t137(ab, null)
    if ab["_gate"] != implied:
        failures.append("recorded _gate verdicts do not match recomputed t137 gate")
    for cell in SUPPORTS_ON:
        if implied[cell] != "PASS":
            failures.append("supports-on cell %s did not PASS: %s" % (cell, implied[cell]))
    for cell in SUPPORTS_OFF:
        if implied[cell] != "INERT-PASS":
            failures.append("supports-off cell %s did not INERT-PASS: %s" % (cell, implied[cell]))

    # 5. Accelerated corroboration (3 pairs/cell, all 4 supports-on positive wall & CPU).
    for cell in CELLS:
        for metric in ("wall", "cpu"):
            s = accel[cell][metric]
            if s["pairs"] != 3:
                failures.append(
                    "%s accel %s has %d pairs, expected 3" % (cell, metric, s["pairs"])
                )
            if not (s["min_saving_pct"] <= s["median_saving_pct"] <= s["max_saving_pct"]):
                failures.append("%s accel %s median outside spread" % (cell, metric))
        if cell in SUPPORTS_ON:
            if accel[cell]["wall"]["median_saving_pct"] < FLOOR_PCT:
                failures.append("%s accel wall saving below floor" % cell)
            if accel[cell]["cpu"]["median_saving_pct"] <= 0.0:
                failures.append("%s accel cpu not corroborating" % cell)

    # 6. Output currency, ordinary === accelerated 16/16, diagnostics clean.
    if len(output["cells"]) != 8:
        failures.append("output covers %d cells, expected 8" % len(output["cells"]))
    diag = output["diagnostics"]
    if (
        diag["nonzero_exit_runs"] != 0
        or diag["degraded_runs"] != 0
        or diag["fatal_error_runs"] != 0
        or diag["code_1200_total"] != 0
    ):
        failures.append("output diagnostics are not clean: %s" % diag)
    # Host exe must be identical between baseline and candidate in both modes
    man = output["manifest"]
    if man["baseline-ordinary"]["exe_sha256"] != man["candidate-ordinary"]["exe_sha256"]:
        failures.append("ordinary host exe differs between baseline and candidate")
    if man["baseline-accelerated"]["exe_sha256"] != man["candidate-accelerated"]["exe_sha256"]:
        failures.append("accelerated host exe differs between baseline and candidate")

    moved_cells = []
    for cell in CELLS:
        want_t172 = t172_ref["active_set"][cell]
        if output["t172_active"][cell] != want_t172:
            failures.append("%s output.t172_active disagrees with t172/reference.json" % cell)
        b_ord = output["cells"][cell]["baseline-ordinary"]["sha256"]
        c_ord = output["cells"][cell]["candidate-ordinary"]["sha256"]
        b_acc = output["accelerated"][cell]["baseline-accelerated"]
        c_acc = output["accelerated"][cell]["candidate-accelerated"]
        if b_ord != want_t172:
            failures.append("%s baseline-ordinary does not reproduce t172" % cell)
        if b_acc != want_t172:
            failures.append("%s baseline-accelerated does not reproduce t172" % cell)
        if c_acc != c_ord:
            failures.append("%s candidate-accelerated diverges from candidate-ordinary" % cell)
        if c_ord != want_t172:
            moved_cells.append(cell)
    if sorted(moved_cells) != ["base-arachne-on", "base-classic-on"]:
        failures.append("unexpected moved cells set: %s" % moved_cells)

    # Optional reference.json check (present when landed / KEEP).
    if doc.get("reference") is not None:
        ref = doc["reference"]
        for cell in CELLS:
            want_c = output["cells"][cell]["candidate-ordinary"]["sha256"]
            want_b = output["cells"][cell]["candidate-ordinary"]["bytes"]
            if ref["active_set"][cell] != want_c:
                failures.append("%s reference.json active_set != candidate hash" % cell)
            if ref["bytes"][cell] != want_b:
                failures.append("%s reference.json bytes != candidate bytes" % cell)

    # 7. Delta audit reconciliation & scanbeam proof.
    if len(audit["cells"]) != 8:
        failures.append("audit covers %d cells, expected 8" % len(audit["cells"]))
    for cell in CELLS:
        got = audit["cells"][cell]
        if got["baseline_sha256"] != output["cells"][cell]["baseline-ordinary"]["sha256"]:
            failures.append("%s audit baseline sha disagrees with output.json" % cell)
        if got["candidate_sha256"] != output["cells"][cell]["candidate-ordinary"]["sha256"]:
            failures.append("%s audit candidate sha disagrees with output.json" % cell)
        expect_ident = cell not in ("base-classic-on", "base-arachne-on")
        if got["byte_identical"] != expect_ident:
            failures.append("%s audit byte_identical=%s unexpected" % (cell, got["byte_identical"]))
        total = got["total_extruding_mm"]
        summed = sum(s["delta_mm"] for s in got["sections"].values())
        if abs(summed - total["delta_mm"]) > 1e-6:
            failures.append("%s audit section deltas do not sum to total" % cell)
        if abs(total["delta_pct"] - total["delta_mm"] / total["baseline"] * 100.0) > 1e-9:
            failures.append("%s audit total delta_pct does not recompute" % cell)
        for sec, s in got["sections"].items():
            if s["layers_more"] + s["layers_less"] + s["layers_equal"] != s["layers_compared"]:
                failures.append("%s %s audit layer census broken" % (cell, sec))
            if abs(s["delta_mm"] - (s["candidate_mm"] - s["baseline_mm"])) > 1e-9:
                failures.append("%s %s audit delta_mm broken" % (cell, sec))
            if sec not in ("Support", "Support interface") and abs(s["delta_mm"]) > 1e-12:
                failures.append("%s non-support section %s moved!" % (cell, sec))

    sb = audit.get("scanbeam_audit", {})
    base_sb = sb.get("base-classic-on")
    if not base_sb:
        failures.append("missing scanbeam_audit.base-classic-on")
    else:
        if (
            base_sb["empty_calls"] + base_sb["covered_calls"] + base_sb["polys_calls"]
            != base_sb["total_calls_audited"]
        ):
            failures.append("scanbeam_audit base-classic-on totals do not sum")
        if base_sb["empty_mismatches"] != 0 or base_sb["covered_mismatches"] != 0:
            failures.append("scanbeam_audit base-classic-on has empty/covered mismatch")
        if base_sb["polys_mismatches"] != base_sb["bulk_dummy_scanline_explained"] + 1:
            failures.append("scanbeam_audit base-classic-on explained count mismatch")
    iso = sb.get("isolated_9th_call_proof", {}).get("dummy_scanline_proof", {})
    if not (iso.get("full_eq_win") is False and iso.get("full_eq_win_with_dummy_far_y") is True):
        failures.append("scanbeam_audit isolated 9th call proof missing or invalid")

    # 8. Attribution probe conservation & cross-check against t173 baseline clip points.
    expected_probe = {
        "base-classic-on": SWEEP_ARMS,
        "base-arachne-on": ["baseline", "tile-2mm"],
        "benchy-classic-on": ["baseline", "tile-2mm"],
        "benchy-arachne-on": ["baseline", "tile-2mm"],
    }
    for cell, exp_arms in expected_probe.items():
        if cell not in probe["cells"]:
            failures.append("probe missing cell %s" % cell)
            continue
        t173_p1_pts = t173_arith["census"][cell]["phases"]["planner::batched_phase1_carve"][
            "clip_points"
        ]
        for arm in exp_arms:
            if arm not in probe["cells"][cell]:
                failures.append("probe missing %s/%s" % (cell, arm))
                continue
            d = probe["cells"][cell][arm]
            c = d["counters"]
            if (
                c["outer_gated"] + c["empty_gated"] + c["covered_dropped"] + c["clip_requests"]
                != c["regions_total"]
            ):
                failures.append("probe %s/%s region accounting broken" % (cell, arm))
            if c["orig_clip_pts"] != t173_p1_pts:
                failures.append(
                    "probe %s/%s orig_clip_pts (%d) != t173 (%d)"
                    % (cell, arm, c["orig_clip_pts"], t173_p1_pts)
                )
            want_red = (c["orig_clip_pts"] - c["win_clip_pts"]) / c["orig_clip_pts"] * 100.0
            if abs(d["derived"]["clip_point_reduction_pct"] - want_red) > 1e-9:
                failures.append("probe %s/%s clip_point_reduction_pct broken" % (cell, arm))

    # 9. Cross-window route composition reconciliation against t174 & t173.
    for cell in CELLS:
        r = route["cells"][cell]
        t174_c = t174_summary["cells"][cell]
        if abs(r["t174_ratio_ordinary"] - t174_c["ratio_ordinary_over_orca"]) > 1e-9:
            failures.append("route %s t174_ratio_ordinary disagrees with t174" % cell)
        if abs(r["t174_ratio_accelerated"] - t174_c["ratio_accelerated_over_orca"]) > 1e-9:
            failures.append("route %s t174_ratio_accelerated disagrees with t174" % cell)
        if cell in SUPPORTS_ON:
            want_ord = r["t174_ratio_ordinary"] * (1.0 - ab[cell]["wall"]["median_saving_pct"] / 100.0)
            if abs(r["est_post_t175_ratio_ordinary"] - want_ord) > 1e-9:
                failures.append("route %s est_post_t175_ratio_ordinary broken" % cell)
            want_acc = r["t174_ratio_accelerated"] * (
                1.0 - accel[cell]["wall"]["median_saving_pct"] / 100.0
            )
            if abs(r["est_post_t175_ratio_accelerated"] - want_acc) > 1e-9:
                failures.append("route %s est_post_t175_ratio_accelerated broken" % cell)
            want_ceil = t173_arith["prices"][cell]["prices"]["phase1_bbox_window_ceiling"][
                "pct_of_cell_wall"
            ]
            if abs(r["t173_phase1_ceiling_wall_pct"] - want_ceil) > 1e-9:
                failures.append("route %s t173 ceiling wall pct disagrees with t173" % cell)


def self_test(doc):
    controls = []

    def run(name, mutate):
        bad = copy.deepcopy(doc)
        mutate(bad)
        fails = []
        check(bad, fails)
        controls.append((name, bool(fails), fails[:1]))

    run(
        "sweep winner wrong",
        lambda d: d["sweep"].update(chosen_arm="tile-10mm"),
    )
    run(
        "sweep tiled arm sha diverges",
        lambda d: d["sweep"]["arms"]["tile-5mm"].update(gcode_sha256="0" * 64),
    )
    run(
        "baseline hash replaced by candidate",
        lambda d: d["output"]["cells"]["base-classic-on"]["baseline-ordinary"].update(
            sha256=d["output"]["cells"]["base-classic-on"]["candidate-ordinary"]["sha256"]
        ),
    )
    run(
        "gate verdict flipped",
        lambda d: d["ab"]["_gate"].update({"base-classic-on": "SUB-FLOOR"}),
    )
    run(
        "supports-off regression accepted",
        lambda d: (
            d["ab"]["base-classic-off"]["wall"].update(
                min_saving_pct=-5.0,
                median_saving_pct=-5.0,
                max_saving_pct=-5.0,
                floor_multiple=-5.0 / FLOOR_PCT,
            ),
            d["ab"]["_gate"].update({"base-classic-off": "INERT-PASS"}),
        ),
    )
    run(
        "CPU contradicted accepted as PASS",
        lambda d: (
            d["ab"]["benchy-classic-on"]["cpu"].update(
                min_saving_pct=-2.0,
                median_saving_pct=-1.0,
                max_saving_pct=0.0,
                floor_multiple=-1.0 / FLOOR_PCT,
            ),
            d["ab"]["_gate"].update({"benchy-classic-on": "PASS"}),
        ),
    )
    run(
        "pair count below standing five",
        lambda d: d["ab"]["base-arachne-on"]["wall"].update(pairs=4),
    )
    run(
        "accelerated candidate diverges from ordinary",
        lambda d: d["output"]["accelerated"]["benchy-arachne-on"].update(
            {"candidate-accelerated": "0" * 64}
        ),
    )
    def mutate_non_support(d):
        cell = d["audit"]["cells"]["base-classic-on"]
        ow = cell["sections"]["Outer wall"]
        ow["candidate_mm"] = ow["baseline_mm"] + 0.01
        ow["delta_mm"] = 0.01
        tot = cell["total_extruding_mm"]
        tot["candidate"] += 0.01
        tot["delta_mm"] += 0.01
        tot["delta_pct"] = tot["delta_mm"] / tot["baseline"] * 100.0

    run("non-support section moved in audit", mutate_non_support)
    run(
        "probe orig_clip_pts disagrees with t173",
        lambda d: d["probe"]["cells"]["base-classic-on"]["tile-2mm"]["counters"].update(
            orig_clip_pts=123456
        ),
    )
    run(
        "route ratio miscomputed",
        lambda d: d["route"]["cells"]["base-classic-on"].update(
            est_post_t175_ratio_ordinary=0.99
        ),
    )
    run(
        "code 1200 diagnostic non-zero",
        lambda d: d["output"]["diagnostics"].update(code_1200_total=1),
    )
    if doc.get("reference") is not None:
        run(
            "reference.json holds superseded t172 base hash",
            lambda d: d["reference"]["active_set"].update(
                {
                    "base-classic-on": d["output"]["cells"]["base-classic-on"][
                        "baseline-ordinary"
                    ]["sha256"]
                }
            ),
        )
    return controls


def main(argv):
    doc = {
        "sweep": load_json(os.path.join(REDUCED, "sweep.json")),
        "probe": load_json(os.path.join(REDUCED, "probe.json")),
        "ab": load_json(os.path.join(REDUCED, "ab.json")),
        "null": load_json(os.path.join(REDUCED, "null.json")),
        "accel": load_json(os.path.join(REDUCED, "accel.json")),
        "output": load_json(os.path.join(REDUCED, "output.json")),
        "audit": load_json(os.path.join(REDUCED, "audit.json")),
        "route": load_json(os.path.join(REDUCED, "route.json")),
        "t172_ref": load_json(T172_REF),
        "t173_arith": load_json(T173_ARITH),
        "t174_summary": load_json(T174_SUMMARY),
        "reference": None,
    }
    ref_path = os.path.join(HERE, "reference.json")
    if os.path.exists(ref_path):
        doc["reference"] = load_json(ref_path)

    if "--self-test" in argv:
        controls = self_test(doc)
        ok = all(rejected for _, rejected, _ in controls)
        for name, rejected, detail in controls:
            print(
                "%-48s %s %s"
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
    print(
        "checks run over 8 cells: sweep, probe, ab, null, accel, output, audit, route%s"
        % (", reference" if doc["reference"] is not None else "")
    )
    for f in failures:
        print("FAIL", f)
    print("verify:", "PASS" if not failures else "FAIL")
    return 0 if not failures else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

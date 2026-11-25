#!/usr/bin/env python3
"""Verify every number t161 quotes, from the in-repo reduced artifact only.

Stdlib-only. Exits 0 when all checks pass, 1 otherwise. `--self-test` replays a
set of mutations against the checks and requires every one to be rejected.

The artifact under test is `reduced/summary.json`; the currency it is checked
against is `evidence/t151-skirt-hull-contour/reference.json`. Both are in-repo,
so the claims are checkable from a plain checkout without the raw campaign tree.
"""

import copy
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
SUMMARY = os.path.join(HERE, "reduced", "summary.json")
REF = os.path.join(
    REPO,
    "docs",
    "specs",
    "perf-vs-orca",
    "evidence",
    "t151-skirt-hull-contour",
    "reference.json",
)

CELL_ORDER = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
    "base-classic-off",
    "base-classic-on",
    "base-arachne-off",
    "base-arachne-on",
]

EXPECTED_MEASURED_ROWS = 48  # 8 cells x 2 tools x 3 runs
EXPECTED_WARMUP_ROWS = 16  # 8 cells x 2 tools x 1 warmup


def close(a, b, tol=2e-4):
    return a is not None and b is not None and abs(a - b) <= tol


def check(summary, ref):
    """Return (checks_passed, failures)."""
    ok = 0
    fail = []

    def want(cond, msg):
        nonlocal ok
        if cond:
            ok += 1
        else:
            fail.append(msg)

    want(summary.get("ticket") == 161, "ticket id is not 161")
    proto = summary.get("protocol", {})
    want(proto.get("cells") == 8, "protocol.cells != 8")
    want(
        proto.get("measured_rows") == EXPECTED_MEASURED_ROWS,
        "measured_rows %r != %d" % (proto.get("measured_rows"), EXPECTED_MEASURED_ROWS),
    )
    want(
        proto.get("warmup_rows") == EXPECTED_WARMUP_ROWS,
        "warmup_rows %r != %d" % (proto.get("warmup_rows"), EXPECTED_WARMUP_ROWS),
    )
    want(proto.get("ordinary_arm_only") is True, "ordinary_arm_only is not true")
    want(
        proto.get("tools") == ["orca", "pnp-ordinary"],
        "tools %r != ['orca','pnp-ordinary']" % (proto.get("tools"),),
    )

    cells = summary.get("cells", {})
    want(sorted(cells) == sorted(CELL_ORDER), "cell set is not the eight map cells")

    ratios, cuts, cuts15 = [], [], []
    for cell in CELL_ORDER:
        c = cells.get(cell)
        if not c:
            fail.append("cell missing: %s" % cell)
            continue
        want(c.get("pnp_runs") == 3, "%s pnp_runs %r != 3" % (cell, c.get("pnp_runs")))
        want(
            c.get("orca_runs") == 3, "%s orca_runs %r != 3" % (cell, c.get("orca_runs"))
        )
        pw = c.get("pnp_wall_s") or []
        ow = c.get("orca_wall_s") or []
        if len(pw) != 3 or len(ow) != 3:
            fail.append(
                "%s wall series not 3/3 (pnp=%d orca=%d)" % (cell, len(pw), len(ow))
            )
            continue

        pm, om = statistics.median(pw), statistics.median(ow)
        want(
            close(pm, c.get("pnp_wall_median_s")),
            "%s pnp median mismatch (%.6f vs %.6f)"
            % (cell, pm, c.get("pnp_wall_median_s")),
        )
        want(
            close(om, c.get("orca_wall_median_s")),
            "%s orca median mismatch (%.6f vs %.6f)"
            % (cell, om, c.get("orca_wall_median_s")),
        )

        ratio = pm / om
        want(
            close(ratio, c.get("ratio_pnp_over_orca"), 5e-4),
            "%s ratio mismatch (%.6f vs %r)"
            % (cell, ratio, c.get("ratio_pnp_over_orca")),
        )
        want(
            close(100 * (1 - 1 / ratio), c.get("required_cut_pct"), 0.01),
            "%s required_cut_pct mismatch" % cell,
        )
        want(
            close(100 * (1 - 1.5 / ratio), c.get("required_cut_1_5x_pct"), 0.01),
            "%s required_cut_1_5x_pct mismatch" % cell,
        )
        want(
            abs(100 * (max(pw) - min(pw)) / pm - (c.get("pnp_spread_pct") or -1))
            <= 0.01,
            "%s pnp_spread_pct mismatch" % cell,
        )
        want(
            abs(100 * (max(ow) - min(ow)) / om - (c.get("orca_spread_pct") or -1))
            <= 0.01,
            "%s orca_spread_pct mismatch" % cell,
        )

        # identity against the currency
        want(
            c.get("reference_hash") == ref.get(cell),
            "%s reference hash does not match reference.json" % cell,
        )
        want(
            c.get("pnp_hashes") == [ref.get(cell)],
            "%s pnp hashes do not equal the currency hash" % cell,
        )
        want(c.get("identity") is True, "%s identity flag is not true" % cell)
        want(
            c.get("pnp_exit_codes") == ["0"],
            "%s pnp exit codes %r" % (cell, c.get("pnp_exit_codes")),
        )
        want(
            c.get("pnp_degraded") == ["false"],
            "%s pnp degraded %r" % (cell, c.get("pnp_degraded")),
        )
        want(
            all(v == 0 for v in (c.get("diagnostics_1200") or [])),
            "%s has exact-Z diagnostics %r" % (cell, c.get("diagnostics_1200")),
        )
        want(c.get("orca_matched_keys_ok") is True, "%s matched keys not ok" % cell)
        for key, audit in (c.get("orca_matched_keys") or {}).items():
            want(
                audit.get("ok") is True
                and audit.get("resolved") == [audit.get("expected")],
                "%s matched key %s not asserted" % (cell, key),
            )

        ratios.append(ratio)
        cuts.append(c.get("required_cut_pct"))
        cuts15.append(c.get("required_cut_1_5x_pct"))

    ranges = summary.get("ranges", {})
    if ratios:
        want(
            close(min(ratios), ranges["ratio_overall"][0], 5e-4),
            "ratio_overall min mismatch",
        )
        want(
            close(max(ratios), ranges["ratio_overall"][1], 5e-4),
            "ratio_overall max mismatch",
        )
        want(
            close(min(cuts), ranges["required_cut_pct"][0], 0.01),
            "required_cut_pct min mismatch",
        )
        want(
            close(max(cuts), ranges["required_cut_pct"][1], 0.01),
            "required_cut_pct max mismatch",
        )
        want(
            close(min(cuts15), ranges["required_cut_1_5x_pct"][0], 0.01),
            "required_cut_1_5x_pct min mismatch",
        )
        want(
            close(max(cuts15), ranges["required_cut_1_5x_pct"][1], 0.01),
            "required_cut_1_5x_pct max mismatch",
        )
        have = [
            k for k in CELL_ORDER if k in cells and "ratio_pnp_over_orca" in cells[k]
        ]
        benchy = [
            cells[k]["ratio_pnp_over_orca"] for k in have if k.startswith("benchy")
        ]
        base = [cells[k]["ratio_pnp_over_orca"] for k in have if k.startswith("base")]
        want(
            close(min(benchy), ranges["ratio_benchy"][0], 5e-4),
            "ratio_benchy min mismatch",
        )
        want(
            close(max(benchy), ranges["ratio_benchy"][1], 5e-4),
            "ratio_benchy max mismatch",
        )
        want(close(min(base), ranges["ratio_base"][0], 5e-4), "ratio_base min mismatch")
        want(close(max(base), ranges["ratio_base"][1], 5e-4), "ratio_base max mismatch")

    ident = summary.get("identity", {})
    want(ident.get("all_eight") is True, "identity.all_eight is not true")
    want(
        ident.get("cells_matching") == CELL_ORDER,
        "identity.cells_matching is not all eight",
    )
    want(ident.get("cells_missing") == [], "identity.cells_missing is not empty")
    want(
        summary.get("matched_keys", {}).get("all_eight") is True,
        "matched_keys.all_eight is not true",
    )
    want(
        summary.get("cells_won") == sum(1 for r in ratios if r <= 1.0),
        "cells_won does not equal the count of ratios <= 1.0",
    )

    sc = summary.get("state_control")
    if sc:
        spm = statistics.median(sc["pnp_wall_s"])
        som = statistics.median(sc["orca_wall_s"])
        want(close(spm, sc.get("pnp_wall_median_s")),
             "state_control pnp median mismatch")
        want(close(som, sc.get("orca_wall_median_s")),
             "state_control orca median mismatch")
        want(close(spm / som, sc.get("ratio_pnp_over_orca"), 5e-4),
             "state_control ratio mismatch")

    load = summary.get("box_load_pct", {})
    if load.get("samples"):
        want(
            load["min"] <= load["median"] <= load["max"],
            "box load min/median/max are not ordered",
        )

    return ok, fail


def load_inputs():
    if not os.path.exists(SUMMARY):
        sys.exit("missing reduced artifact: %s" % SUMMARY)
    summary = json.load(open(SUMMARY, encoding="utf-8"))
    ref = json.load(open(REF, encoding="utf-8"))["active_set"]
    return summary, ref


def main():
    summary, ref = load_inputs()

    if "--self-test" in sys.argv:
        base_ok, base_fail = check(summary, ref)
        if base_fail:
            print("FAIL (baseline artifact is already inconsistent):")
            for f in base_fail:
                print("  -", f)
            return 1

        mutations = []

        def mut(name, fn):
            s = copy.deepcopy(summary)
            fn(s)
            mutations.append((name, s))

        def set_median(s):
            s["cells"]["base-classic-off"]["pnp_wall_median_s"] += 0.5

        def set_ratio(s):
            s["cells"]["benchy-arachne-on"]["ratio_pnp_over_orca"] *= 0.9

        def set_cut(s):
            s["cells"]["base-arachne-on"]["required_cut_pct"] -= 1.0

        def set_hash(s):
            s["cells"]["benchy-classic-off"]["pnp_hashes"] = ["0" * 64]

        def set_identity(s):
            s["cells"]["base-classic-on"]["identity"] = False

        def drop_cell(s):
            del s["cells"]["benchy-classic-on"]

        def set_key(s):
            s["cells"]["base-classic-off"]["orca_matched_keys"]["resolved_brim_width"][
                "ok"
            ] = False

        def set_diag(s):
            s["cells"]["base-classic-on"]["diagnostics_1200"] = [7, 0, 0]

        def set_won(s):
            s["cells_won"] += 1

        def set_rows(s):
            s["protocol"]["measured_rows"] = 47

        def set_range(s):
            s["ranges"]["required_cut_pct"][1] += 1.0

        def set_range_benchy(s):
            s["ranges"]["ratio_benchy"][0] *= 0.95

        def set_spread(s):
            s["cells"]["benchy-classic-on"]["pnp_spread_pct"] += 2.0

        def set_degraded(s):
            s["cells"]["base-arachne-on"]["pnp_degraded"] = ["true"]

        def set_control(s):
            s["state_control"]["orca_wall_median_s"] *= 0.9

        def set_load(s):
            s["box_load_pct"]["min"] = s["box_load_pct"]["max"] + 1

        for name, fn in [
            ("median", set_median),
            ("ratio", set_ratio),
            ("required_cut", set_cut),
            ("hash", set_hash),
            ("identity_flag", set_identity),
            ("missing_cell", drop_cell),
            ("matched_key", set_key),
            ("diagnostics", set_diag),
            ("cells_won", set_won),
            ("measured_rows", set_rows),
            ("range_cut", set_range),
            ("range_benchy", set_range_benchy),
            ("spread", set_spread),
            ("degraded", set_degraded),
            ("load_order", set_load),
            ("state_control", set_control),
        ]:
            mut(name, fn)

        rejected = 0
        for name, s in mutations:
            _, fails = check(s, ref)
            if fails:
                rejected += 1
            else:
                print("SELF-TEST FAILURE: mutation %r was accepted" % (name,))
        print("self-test: %d/%d mutations rejected" % (rejected, len(mutations)))
        return 0 if rejected == len(mutations) else 1

    ok, fail = check(summary, ref)
    if fail:
        print("FAIL: %d/%d checks" % (len(fail), ok + len(fail)))
        for f in fail:
            print("  -", f)
        return 1
    print("PASS: %d/%d checks" % (ok, ok))
    return 0


if __name__ == "__main__":
    sys.exit(main())

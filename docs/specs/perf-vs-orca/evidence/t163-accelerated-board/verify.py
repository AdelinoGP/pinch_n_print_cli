#!/usr/bin/env python3
"""Verify every number t163 quotes, from the in-repo reduced artifact only.

Stdlib-only. Exits 0 when all checks pass, 1 otherwise. `--self-test` replays
mutations and requires each one to be rejected.
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


def close(a, b, tol=2e-4):
    return a is not None and b is not None and abs(a - b) <= tol


def spread(values):
    if len(values) < 2:
        return None
    m = statistics.median(values)
    return round(100.0 * (max(values) - min(values)) / m, 2) if m else None


def check(summary, ref):
    ok = 0
    fail = []

    def want(cond, msg):
        nonlocal ok
        if cond:
            ok += 1
        else:
            fail.append(msg)

    want(summary.get("ticket") == 163, "ticket id is not 163")
    proto = summary.get("protocol", {})
    want(proto.get("cells") == 8, "protocol.cells != 8")
    want(
        proto.get("tools") == ["orca", "pnp-ordinary", "pnp-accelerated"],
        "protocol.tools is not the three-tool set",
    )
    want(proto.get("measured_rows") == 72, "measured_rows != 72")
    want(proto.get("warmup_rows") == 24, "warmup_rows != 24")

    cells = summary.get("cells", {})
    want(sorted(cells) == sorted(CELL_ORDER), "cell set is not the eight map cells")

    ord_ratios, acc_ratios, wall_deltas, cpu_deltas = [], [], [], []
    for cell in CELL_ORDER:
        c = cells.get(cell)
        if not c:
            fail.append("cell missing: %s" % cell)
            continue
        for arm in ("orca", "pnp-ordinary", "pnp-accelerated"):
            block = c.get(arm)
            if not block:
                fail.append("%s missing arm %s" % (cell, arm))
                continue
            walls, cpus = block.get("wall_s") or [], block.get("cpu_s") or []
            want(len(walls) == 3, "%s/%s wall series is not 3 runs" % (cell, arm))
            want(len(cpus) == 3, "%s/%s cpu series is not 3 runs" % (cell, arm))
            if len(walls) != 3:
                continue
            wm, cm = statistics.median(walls), statistics.median(cpus)
            want(
                close(wm, block.get("wall_median_s")),
                "%s/%s wall median mismatch" % (cell, arm),
            )
            want(
                close(cm, block.get("cpu_median_s")),
                "%s/%s cpu median mismatch" % (cell, arm),
            )
            want(
                block.get("wall_spread_pct") == spread(walls),
                "%s/%s wall spread mismatch" % (cell, arm),
            )
            want(
                block.get("cpu_spread_pct") == spread(cpus),
                "%s/%s cpu spread mismatch" % (cell, arm),
            )

        o = c.get("pnp-ordinary")
        a = c.get("pnp-accelerated")
        orca = c.get("orca")
        if not (o and a and orca):
            continue
        om, am, rm = o["wall_median_s"], a["wall_median_s"], orca["wall_median_s"]
        want(
            close(om / rm, c.get("ratio_ordinary_over_orca"), 5e-4),
            "%s ordinary ratio mismatch" % cell,
        )
        want(
            close(am / rm, c.get("ratio_accelerated_over_orca"), 5e-4),
            "%s accelerated ratio mismatch" % cell,
        )
        want(
            close(100 * (1 - 1 / (om / rm)), c.get("required_cut_ordinary_pct"), 0.01),
            "%s ordinary required cut mismatch" % cell,
        )
        want(
            close(
                100 * (1 - 1 / (am / rm)), c.get("required_cut_accelerated_pct"), 0.01
            ),
            "%s accelerated required cut mismatch" % cell,
        )
        want(
            close(100 * (am / om - 1), c.get("accel_vs_ordinary_wall_pct"), 0.01),
            "%s accel-vs-ordinary wall delta mismatch" % cell,
        )
        want(
            close(
                100 * (a["cpu_median_s"] / o["cpu_median_s"] - 1),
                c.get("accel_vs_ordinary_cpu_pct"),
                0.01,
            ),
            "%s accel-vs-ordinary cpu delta mismatch" % cell,
        )
        ord_ratios.append(om / rm)
        acc_ratios.append(am / rm)
        wall_deltas.append(c["accel_vs_ordinary_wall_pct"])
        cpu_deltas.append(c["accel_vs_ordinary_cpu_pct"])

        want(
            c.get("reference_hash") == ref.get(cell),
            "%s reference hash does not match reference.json" % cell,
        )
        want(c.get("identity_ordinary") is True, "%s ordinary identity false" % cell)
        want(
            c.get("identity_accelerated") is True,
            "%s accelerated identity false" % cell,
        )
        want(
            c.get("hash_ordinary") == ref.get(cell),
            "%s ordinary hash != currency" % cell,
        )
        want(
            c.get("hash_accelerated") == ref.get(cell),
            "%s accelerated hash != currency" % cell,
        )
        want(c.get("exit_codes_ordinary") == ["0"], "%s ordinary exit codes" % cell)
        want(
            c.get("exit_codes_accelerated") == ["0"], "%s accelerated exit codes" % cell
        )
        want(c.get("degraded_ordinary") == ["false"], "%s ordinary degraded" % cell)
        want(
            c.get("degraded_accelerated") == ["false"], "%s accelerated degraded" % cell
        )
        want(c.get("orca_matched_keys_ok") is True, "%s matched keys not ok" % cell)
        for key, audit in (c.get("orca_matched_keys") or {}).items():
            want(
                audit.get("ok") is True
                and audit.get("resolved") == [audit.get("expected")],
                "%s matched key %s not asserted" % (cell, key),
            )

    ranges = summary.get("ranges", {})
    if len(ord_ratios) == 8:
        want(
            close(min(ord_ratios), ranges["ratio_ordinary_over_orca"][0], 5e-4),
            "ordinary ratio range min mismatch",
        )
        want(
            close(max(ord_ratios), ranges["ratio_ordinary_over_orca"][1], 5e-4),
            "ordinary ratio range max mismatch",
        )
        want(
            close(min(acc_ratios), ranges["ratio_accelerated_over_orca"][0], 5e-4),
            "accelerated ratio range min mismatch",
        )
        want(
            close(max(acc_ratios), ranges["ratio_accelerated_over_orca"][1], 5e-4),
            "accelerated ratio range max mismatch",
        )
        want(
            close(min(wall_deltas), ranges["accel_vs_ordinary_wall_pct"][0], 0.01),
            "wall delta range min mismatch",
        )
        want(
            close(max(wall_deltas), ranges["accel_vs_ordinary_wall_pct"][1], 0.01),
            "wall delta range max mismatch",
        )
        want(
            close(min(cpu_deltas), ranges["accel_vs_ordinary_cpu_pct"][0], 0.01),
            "cpu delta range min mismatch",
        )
        want(
            close(max(cpu_deltas), ranges["accel_vs_ordinary_cpu_pct"][1], 0.01),
            "cpu delta range max mismatch",
        )

    ident = summary.get("identity", {})
    want(ident.get("ordinary_all_eight") is True, "identity.ordinary_all_eight false")
    want(
        ident.get("accelerated_all_eight") is True,
        "identity.accelerated_all_eight false",
    )
    want(ident.get("ordinary_missing") == [], "ordinary identity misses recorded")
    want(ident.get("accelerated_missing") == [], "accelerated identity misses recorded")
    want(
        summary.get("matched_keys", {}).get("all_eight") is True,
        "matched_keys.all_eight false",
    )
    want(
        summary.get("cells_accelerated_faster") == sum(1 for d in wall_deltas if d < 0),
        "cells_accelerated_faster does not match the deltas",
    )
    want(
        summary.get("cells_accelerated_slower") == sum(1 for d in wall_deltas if d > 0),
        "cells_accelerated_slower does not match the deltas",
    )

    load = summary.get("box_load_pct", {})
    if load.get("samples"):
        want(
            load["min"] <= load["median"] <= load["max"],
            "box load min/median/max are not ordered",
        )

    return ok, fail


def main():
    if not os.path.exists(SUMMARY):
        sys.exit("missing reduced artifact: %s" % SUMMARY)
    summary = json.load(open(SUMMARY, encoding="utf-8"))
    ref = json.load(open(REF, encoding="utf-8"))["active_set"]

    if "--self-test" in sys.argv:
        ok, fail = check(summary, ref)
        if fail:
            print("FAIL (baseline artifact is already inconsistent):")
            for f in fail:
                print("  -", f)
            return 1

        mutations = []

        def mut(name, fn):
            s = copy.deepcopy(summary)
            fn(s)
            mutations.append((name, s))

        mut(
            "median",
            lambda s: s["cells"]["base-classic-off"]["pnp-accelerated"].__setitem__(
                "wall_median_s", 999.0
            ),
        )
        mut(
            "ratio",
            lambda s: s["cells"]["benchy-classic-on"].__setitem__(
                "ratio_accelerated_over_orca", 1.0
            ),
        )
        mut(
            "delta",
            lambda s: s["cells"]["base-arachne-on"].__setitem__(
                "accel_vs_ordinary_wall_pct", 99.0
            ),
        )
        mut(
            "spread",
            lambda s: s["cells"]["benchy-arachne-off"]["orca"].__setitem__(
                "wall_spread_pct", 0.0
            ),
        )
        mut(
            "hash",
            lambda s: s["cells"]["benchy-classic-off"].__setitem__(
                "hash_accelerated", "0" * 64
            ),
        )
        mut(
            "identity",
            lambda s: s["cells"]["base-classic-on"].__setitem__(
                "identity_ordinary", False
            ),
        )
        mut(
            "matched_key",
            lambda s: s["cells"]["base-arachne-off"]["orca_matched_keys"][
                "resolved_skirt_loops"
            ].__setitem__("ok", False),
        )
        mut("drop_cell", lambda s: s["cells"].pop("benchy-arachne-on"))
        mut("measured_rows", lambda s: s["protocol"].__setitem__("measured_rows", 71))
        mut(
            "range",
            lambda s: s["ranges"]["accel_vs_ordinary_wall_pct"].__setitem__(0, -99.0),
        )
        mut(
            "faster_count",
            lambda s: s.__setitem__(
                "cells_accelerated_faster", s["cells_accelerated_faster"] + 1
            ),
        )
        mut(
            "load",
            lambda s: s["box_load_pct"].__setitem__(
                "min", s["box_load_pct"]["max"] + 1
            ),
        )
        mut(
            "cpu_delta",
            lambda s: s["cells"]["base-classic-off"].__setitem__(
                "accel_vs_ordinary_cpu_pct", 50.0
            ),
        )
        mut(
            "required_cut",
            lambda s: s["cells"]["benchy-classic-off"].__setitem__(
                "required_cut_ordinary_pct", 0.0
            ),
        )

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

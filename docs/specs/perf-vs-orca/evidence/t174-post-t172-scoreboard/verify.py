#!/usr/bin/env python3
"""Verify every number t174 quotes, from the in-repo reduced artifacts only.

Stdlib-only. Exits 0 when all checks pass, 1 otherwise. `--self-test` replays
mutations against summary.json and requires each one to be rejected.
"""

import copy
import csv
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", "..", ".."))
SUMMARY = os.path.join(HERE, "reduced", "summary.json")
LOADED = os.path.join(HERE, "reduced", "loaded-window.json")
LOAD_CSV = os.path.join(HERE, "reduced", "load-samples.csv")
LOADED_LOAD_CSV = os.path.join(HERE, "reduced", "loaded-window-load-samples.csv")
REF = os.path.join(
    REPO,
    "docs",
    "specs",
    "perf-vs-orca",
    "evidence",
    "t172-offset-concave-crossing",
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


def load_stats_from_csv(path):
    if not path or not os.path.exists(path):
        return None, None
    pcts, free_mb = [], []
    with open(path, newline="", encoding="utf-8-sig") as f:
        for r in csv.reader(f):
            if len(r) >= 2 and r[1].strip():
                pcts.append(float(r[1]))
            if len(r) >= 3 and r[2].strip():
                free_mb.append(float(r[2]) / 1024.0)
    return (
        {
            "samples": len(pcts),
            "min": round(min(pcts), 1),
            "median": round(statistics.median(pcts), 1),
            "max": round(max(pcts), 1),
        }
        if pcts
        else None,
        {
            "samples": len(free_mb),
            "min": round(min(free_mb), 1),
            "median": round(statistics.median(free_mb), 1),
            "max": round(max(free_mb), 1),
        }
        if free_mb
        else None,
    )


def check_board(summary, ref_doc, load_csv_path=None, expect_no_starvation=False):
    ok = 0
    fail = []
    ref_hashes = ref_doc["active_set"]
    ref_bytes = ref_doc["bytes"]

    def want(cond, msg):
        nonlocal ok
        if cond:
            ok += 1
        else:
            fail.append(msg)

    want(summary.get("ticket") == 174, "ticket id is not 174")
    proto = summary.get("protocol", {})
    want(proto.get("cells") == 8, "protocol.cells != 8")
    want(
        proto.get("tools") == ["orca", "pnp-ordinary", "pnp-accelerated"],
        "protocol.tools is not the three-tool set",
    )
    want(proto.get("measured_rows") == 72, "measured_rows != 72")
    want(proto.get("warmup_rows") == 24, "warmup_rows != 24")
    want(proto.get("threads") == 12, "threads != 12")

    ids = summary.get("identities", {})
    want(ids.get("ordinary_guests_check_exit") == 0, "ordinary guests check != 0")
    want(ids.get("accelerated_guests_check_exit") == 0, "accelerated guests check != 0")
    want(
        len((ids.get("orca") or {}).get("exe_sha256", "")) == 64,
        "orca exe_sha256 missing",
    )
    want(
        len((ids.get("pnp-ordinary") or {}).get("exe_sha256", "")) == 64,
        "pnp-ordinary exe_sha256 missing",
    )
    want(
        len((ids.get("pnp-accelerated") or {}).get("exe_sha256", "")) == 64,
        "pnp-accelerated exe_sha256 missing",
    )

    cells = summary.get("cells", {})
    want(sorted(cells) == sorted(CELL_ORDER), "cell set is not the eight map cells")

    ord_ratios, acc_ratios = [], []
    ord_cpu_ratios, acc_cpu_ratios = [], []
    wall_deltas, cpu_deltas = [], []
    ord_wall_wins, ord_dest_wins = [], []
    acc_wall_wins, acc_dest_wins = [], []
    code_1200_sum = 0

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
            walls = block.get("wall_s") or []
            cpus = block.get("cpu_s") or []
            ratios = block.get("cpu_wall_ratio") or []
            want(len(walls) == 3, "%s/%s wall series is not 3 runs" % (cell, arm))
            want(len(cpus) == 3, "%s/%s cpu series is not 3 runs" % (cell, arm))
            want(len(ratios) == 3, "%s/%s ratio series is not 3 runs" % (cell, arm))
            if len(walls) != 3:
                continue
            wm, cm, rm = (
                statistics.median(walls),
                statistics.median(cpus),
                statistics.median(ratios),
            )
            want(close(wm, block.get("wall_median_s")), "%s/%s wall median mismatch" % (cell, arm))
            want(close(cm, block.get("cpu_median_s")), "%s/%s cpu median mismatch" % (cell, arm))
            want(
                close(rm, block.get("cpu_wall_ratio_median")),
                "%s/%s cpu/wall median mismatch" % (cell, arm),
            )
            want(block.get("wall_spread_pct") == spread(walls), "%s/%s wall spread mismatch" % (cell, arm))
            want(block.get("cpu_spread_pct") == spread(cpus), "%s/%s cpu spread mismatch" % (cell, arm))
            best_r = max(ratios)
            expected_starved = [i for i, r in enumerate(ratios) if r < 0.75 * best_r]
            want(
                block.get("starved_indices_075") == expected_starved,
                "%s/%s starvation index mismatch" % (cell, arm),
            )
            if expect_no_starvation:
                want(expected_starved == [], "%s/%s unexpected starvation flag in quiet window" % (cell, arm))

        o = c.get("pnp-ordinary")
        a = c.get("pnp-accelerated")
        orca = c.get("orca")
        if not (o and a and orca):
            continue
        om, am, rm = o["wall_median_s"], a["wall_median_s"], orca["wall_median_s"]
        oc, ac, rc = o["cpu_median_s"], a["cpu_median_s"], orca["cpu_median_s"]

        want(close(om / rm, c.get("ratio_ordinary_over_orca"), 5e-4), "%s ordinary ratio mismatch" % cell)
        want(close(am / rm, c.get("ratio_accelerated_over_orca"), 5e-4), "%s accelerated ratio mismatch" % cell)
        want(close(oc / rc, c.get("cpu_ratio_ordinary_over_orca"), 5e-4), "%s ordinary cpu ratio mismatch" % cell)
        want(close(ac / rc, c.get("cpu_ratio_accelerated_over_orca"), 5e-4), "%s accelerated cpu ratio mismatch" % cell)
        want(
            close(100 * (1 - 1 / (om / rm)), c.get("required_cut_ordinary_pct"), 0.01),
            "%s ordinary required cut mismatch" % cell,
        )
        want(
            close(100 * (1 - 1 / (am / rm)), c.get("required_cut_accelerated_pct"), 0.01),
            "%s accelerated required cut mismatch" % cell,
        )
        want(
            close(100 * (am / om - 1), c.get("accel_vs_ordinary_wall_pct"), 0.01),
            "%s accel-vs-ordinary wall delta mismatch" % cell,
        )
        want(
            close(100 * (ac / oc - 1), c.get("accel_vs_ordinary_cpu_pct"), 0.01),
            "%s accel-vs-ordinary cpu delta mismatch" % cell,
        )

        w_ord = om < rm
        c_ord = oc < rc
        w_acc = am < rm
        c_acc = ac < rc
        want(c.get("wall_win_ordinary") is w_ord, "%s wall_win_ordinary mismatch" % cell)
        want(c.get("cpu_corroborates_ordinary") is c_ord, "%s cpu_corroborates_ordinary mismatch" % cell)
        want(c.get("destination_win_ordinary") is (w_ord and c_ord), "%s destination_win_ordinary mismatch" % cell)
        want(
            c.get("strict_separation_ordinary") is (max(o["wall_s"]) < min(orca["wall_s"])),
            "%s strict_separation_ordinary mismatch" % cell,
        )
        want(c.get("wall_win_accelerated") is w_acc, "%s wall_win_accelerated mismatch" % cell)
        want(c.get("cpu_corroborates_accelerated") is c_acc, "%s cpu_corroborates_accelerated mismatch" % cell)
        want(c.get("destination_win_accelerated") is (w_acc and c_acc), "%s destination_win_accelerated mismatch" % cell)
        want(
            c.get("strict_separation_accelerated") is (max(a["wall_s"]) < min(orca["wall_s"])),
            "%s strict_separation_accelerated mismatch" % cell,
        )

        if w_ord:
            ord_wall_wins.append(cell)
        if w_ord and c_ord:
            ord_dest_wins.append(cell)
        if w_acc:
            acc_wall_wins.append(cell)
        if w_acc and c_acc:
            acc_dest_wins.append(cell)

        ord_ratios.append(om / rm)
        acc_ratios.append(am / rm)
        ord_cpu_ratios.append(oc / rc)
        acc_cpu_ratios.append(ac / rc)
        wall_deltas.append(c["accel_vs_ordinary_wall_pct"])
        cpu_deltas.append(c["accel_vs_ordinary_cpu_pct"])

        want(c.get("reference_hash") == ref_hashes.get(cell), "%s reference hash != t172 reference.json" % cell)
        want(c.get("reference_bytes") == ref_bytes.get(cell), "%s reference bytes != t172 reference.json" % cell)
        want(c.get("identity_ordinary") is True, "%s ordinary identity false" % cell)
        want(c.get("identity_accelerated") is True, "%s accelerated identity false" % cell)
        want(c.get("hash_ordinary") == ref_hashes.get(cell), "%s ordinary hash != currency" % cell)
        want(c.get("hash_accelerated") == ref_hashes.get(cell), "%s accelerated hash != currency" % cell)
        want(c.get("gcode_bytes_ordinary") == ref_bytes.get(cell), "%s ordinary bytes != currency" % cell)
        want(c.get("gcode_bytes_accelerated") == ref_bytes.get(cell), "%s accelerated bytes != currency" % cell)
        want(c.get("exit_codes_ordinary") == ["0"], "%s ordinary exit codes" % cell)
        want(c.get("exit_codes_accelerated") == ["0"], "%s accelerated exit codes" % cell)
        want(c.get("degraded_ordinary") == ["false"], "%s ordinary degraded" % cell)
        want(c.get("degraded_accelerated") == ["false"], "%s accelerated degraded" % cell)
        want(c.get("fatal_error_counts_ordinary") == ["0"], "%s ordinary fatal errors" % cell)
        want(c.get("fatal_error_counts_accelerated") == ["0"], "%s accelerated fatal errors" % cell)
        want(c.get("non_fatal_error_counts_ordinary") == ["0"], "%s ordinary non-fatal errors" % cell)
        want(c.get("non_fatal_error_counts_accelerated") == ["0"], "%s accelerated non-fatal errors" % cell)
        want(c.get("code_1200_count_ordinary") == 0, "%s ordinary code-1200 != 0" % cell)
        want(c.get("code_1200_count_accelerated") == 0, "%s accelerated code-1200 != 0" % cell)
        code_1200_sum += (c.get("code_1200_count_ordinary") or 0) + (c.get("code_1200_count_accelerated") or 0)

        want(c.get("orca_matched_keys_ok") is True, "%s matched keys not ok" % cell)
        want(c.get("orca_exit_codes") == ["0"], "%s orca exit codes" % cell)
        for key, audit in (c.get("orca_matched_keys") or {}).items():
            want(
                audit.get("ok") is True and audit.get("resolved") == [audit.get("expected")],
                "%s matched key %s not asserted" % (cell, key),
            )

    ranges = summary.get("ranges", {})
    if len(ord_ratios) == 8:
        want(close(min(ord_ratios), ranges["ratio_ordinary_over_orca"][0], 5e-4), "ordinary ratio min mismatch")
        want(close(max(ord_ratios), ranges["ratio_ordinary_over_orca"][1], 5e-4), "ordinary ratio max mismatch")
        want(close(min(acc_ratios), ranges["ratio_accelerated_over_orca"][0], 5e-4), "accelerated ratio min mismatch")
        want(close(max(acc_ratios), ranges["ratio_accelerated_over_orca"][1], 5e-4), "accelerated ratio max mismatch")
        want(close(min(ord_cpu_ratios), ranges["cpu_ratio_ordinary_over_orca"][0], 5e-4), "ordinary cpu ratio min mismatch")
        want(close(max(ord_cpu_ratios), ranges["cpu_ratio_ordinary_over_orca"][1], 5e-4), "ordinary cpu ratio max mismatch")
        want(close(min(acc_cpu_ratios), ranges["cpu_ratio_accelerated_over_orca"][0], 5e-4), "accelerated cpu ratio min mismatch")
        want(close(max(acc_cpu_ratios), ranges["cpu_ratio_accelerated_over_orca"][1], 5e-4), "accelerated cpu ratio max mismatch")
        want(close(min(wall_deltas), ranges["accel_vs_ordinary_wall_pct"][0], 0.01), "wall delta min mismatch")
        want(close(max(wall_deltas), ranges["accel_vs_ordinary_wall_pct"][1], 0.01), "wall delta max mismatch")
        want(close(min(cpu_deltas), ranges["accel_vs_ordinary_cpu_pct"][0], 0.01), "cpu delta min mismatch")
        want(close(max(cpu_deltas), ranges["accel_vs_ordinary_cpu_pct"][1], 0.01), "cpu delta max mismatch")

    dt = summary.get("destination_tally", {})
    want(dt.get("ordinary_wall_wins") == len(ord_wall_wins), "ordinary_wall_wins mismatch")
    want(dt.get("ordinary_wall_win_cells") == ord_wall_wins, "ordinary_wall_win_cells mismatch")
    want(dt.get("ordinary_cpu_corroborated_wins") == len(ord_dest_wins), "ordinary_cpu_corroborated_wins mismatch")
    want(dt.get("ordinary_cpu_corroborated_win_cells") == ord_dest_wins, "ordinary_cpu_corroborated_win_cells mismatch")
    want(dt.get("accelerated_wall_wins") == len(acc_wall_wins), "accelerated_wall_wins mismatch")
    want(dt.get("accelerated_wall_win_cells") == acc_wall_wins, "accelerated_wall_win_cells mismatch")
    want(dt.get("accelerated_cpu_corroborated_wins") == len(acc_dest_wins), "accelerated_cpu_corroborated_wins mismatch")
    want(dt.get("accelerated_cpu_corroborated_win_cells") == acc_dest_wins, "accelerated_cpu_corroborated_win_cells mismatch")

    ident = summary.get("identity", {})
    want(ident.get("ordinary_all_eight") is True, "identity.ordinary_all_eight false")
    want(ident.get("accelerated_all_eight") is True, "identity.accelerated_all_eight false")
    want(ident.get("ordinary_missing") == [], "ordinary identity misses recorded")
    want(ident.get("accelerated_missing") == [], "accelerated identity misses recorded")
    want(ident.get("code_1200_total") == code_1200_sum == 0, "code_1200_total != 0")
    want(summary.get("matched_keys", {}).get("all_eight") is True, "matched_keys.all_eight false")
    want(
        summary.get("cells_accelerated_faster") == sum(1 for d in wall_deltas if d < 0),
        "cells_accelerated_faster mismatch",
    )
    want(
        summary.get("cells_accelerated_slower") == sum(1 for d in wall_deltas if d > 0),
        "cells_accelerated_slower mismatch",
    )

    load = summary.get("box_load_pct", {})
    if load.get("samples"):
        want(load["min"] <= load["median"] <= load["max"], "box load min/median/max not ordered")
    fmem = summary.get("free_memory_mb", {})
    if fmem.get("samples"):
        want(fmem["min"] <= fmem["median"] <= fmem["max"], "free_memory_mb min/median/max not ordered")
    if load_csv_path and os.path.exists(load_csv_path):
        csv_load, csv_mem = load_stats_from_csv(load_csv_path)
        want(load == csv_load, "box_load_pct does not match load-samples csv")
        want(fmem == csv_mem, "free_memory_mb does not match load-samples csv")

    return ok, fail


def main():
    if not os.path.exists(SUMMARY):
        sys.exit("missing reduced artifact: %s" % SUMMARY)
    with open(SUMMARY, encoding="utf-8") as f:
        summary = json.load(f)
    with open(REF, encoding="utf-8") as f:
        ref_doc = json.load(f)

    if "--self-test" in sys.argv:
        ok, fail = check_board(summary, ref_doc, LOAD_CSV, expect_no_starvation=True)
        if fail:
            print("FAIL (baseline artifact is already inconsistent):")
            for item in fail:
                print("  -", item)
            return 1

        mutations = []

        def mut(name, fn):
            s = copy.deepcopy(summary)
            fn(s)
            mutations.append((name, s))

        mut("median", lambda s: s["cells"]["base-classic-off"]["pnp-ordinary"].__setitem__("wall_median_s", 999.0))
        mut("ratio", lambda s: s["cells"]["base-classic-off"].__setitem__("ratio_ordinary_over_orca", 1.0))
        mut("cpu_ratio", lambda s: s["cells"]["base-arachne-off"].__setitem__("cpu_ratio_ordinary_over_orca", 0.5))
        mut("required_cut", lambda s: s["cells"]["benchy-classic-on"].__setitem__("required_cut_ordinary_pct", 0.0))
        mut("wall_win_flag", lambda s: s["cells"]["base-classic-off"].__setitem__("wall_win_ordinary", False))
        mut("cpu_corroborates_flag", lambda s: s["cells"]["base-classic-off"].__setitem__("cpu_corroborates_ordinary", True))
        mut("dest_tally", lambda s: s["destination_tally"].__setitem__("ordinary_cpu_corroborated_wins", 2))
        mut("strict_sep", lambda s: s["cells"]["base-arachne-off"].__setitem__("strict_separation_accelerated", False))
        mut("delta", lambda s: s["cells"]["base-arachne-on"].__setitem__("accel_vs_ordinary_wall_pct", 99.0))
        mut("spread", lambda s: s["cells"]["benchy-arachne-off"]["orca"].__setitem__("wall_spread_pct", 0.0))
        mut("hash", lambda s: s["cells"]["benchy-classic-off"].__setitem__("hash_accelerated", "0" * 64))
        mut("bytes", lambda s: s["cells"]["base-classic-on"].__setitem__("gcode_bytes_ordinary", 1234))
        mut("code_1200", lambda s: s["cells"]["base-classic-on"].__setitem__("code_1200_count_ordinary", 1))
        mut("matched_key", lambda s: s["cells"]["base-arachne-off"]["orca_matched_keys"]["resolved_support_style"].__setitem__("ok", False))
        mut("drop_cell", lambda s: s["cells"].pop("benchy-arachne-on"))
        mut("measured_rows", lambda s: s["protocol"].__setitem__("measured_rows", 71))
        mut("load_mismatch", lambda s: s["box_load_pct"].__setitem__("median", 1.0))
        mut("guests_exit", lambda s: s["identities"].__setitem__("accelerated_guests_check_exit", 1))

        rejected = 0
        for name, s in mutations:
            _, fails = check_board(s, ref_doc, LOAD_CSV, expect_no_starvation=True)
            if fails:
                rejected += 1
            else:
                print("SELF-TEST FAILURE: mutation %r was accepted" % (name,))
        print("self-test: %d/%d mutations rejected" % (rejected, len(mutations)))
        return 0 if rejected == len(mutations) else 1

    ok1, fail1 = check_board(summary, ref_doc, LOAD_CSV, expect_no_starvation=True)
    total_ok, all_fail = ok1, list(fail1)

    if os.path.exists(LOADED):
        with open(LOADED, encoding="utf-8") as f:
            loaded = json.load(f)
        ok2, fail2 = check_board(loaded, ref_doc, LOADED_LOAD_CSV, expect_no_starvation=False)
        total_ok += ok2
        all_fail.extend("loaded-window: " + x for x in fail2)

    if all_fail:
        print("FAIL: %d/%d checks" % (len(all_fail), total_ok + len(all_fail)))
        for item in all_fail:
            print("  -", item)
        return 1
    print("PASS: %d/%d checks" % (total_ok, total_ok))
    return 0


if __name__ == "__main__":
    sys.exit(main())

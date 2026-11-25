#!/usr/bin/env python3
"""Verify every number t193 quotes, from the in-repo reduced artifacts only.

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
ATTR = os.path.join(HERE, "reduced", "attribution.json")
LOAD_CSV = os.path.join(HERE, "reduced", "load-samples.csv")
REF = os.path.join(
    REPO,
    "docs",
    "specs",
    "perf-vs-orca",
    "evidence",
    "t182-spatial-hierarchical-phase3-union",
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
TOOLS = ["orca", "pnp-ordinary", "pnp-accelerated"]


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


def check_board(summary, ref_doc, load_csv_path=None, expect_no_starvation=True):
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

    want(summary.get("ticket") == 193, "ticket id is not 193")
    proto = summary.get("protocol", {})
    want(proto.get("cells") == 8, "protocol.cells != 8")
    want(proto.get("tools") == TOOLS, "protocol.tools is not the three-tool set")
    want(proto.get("runs_per_cell_tool") == 4, "runs_per_cell_tool != 4")
    want(proto.get("retained_runs_min") == 4, "retained_runs_min != 4")
    want(proto.get("measured_rows") == 96, "measured_rows != 96")
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
        (ids.get("orca") or {}).get("exe_sha256", "").startswith("5013b1bd"),
        "orca exe sha256 is not the 2.5.1 stamp",
    )
    want(
        len((ids.get("pnp-ordinary") or {}).get("exe_sha256", "")) == 64,
        "pnp-ordinary exe_sha256 missing",
    )
    want(
        len((ids.get("pnp-accelerated") or {}).get("exe_sha256", "")) == 64,
        "pnp-accelerated exe_sha256 missing",
    )
    want(
        (ids.get("pnp-ordinary") or {}).get("module_count") == 24,
        "ordinary module_count != 24",
    )
    want(
        (ids.get("pnp-accelerated") or {}).get("module_count") == 24,
        "accelerated module_count != 24",
    )

    cells = summary.get("cells", {})
    want(sorted(cells) == sorted(CELL_ORDER), "cell set is not the eight map cells")

    ord_ratios, acc_ratios = [], []
    ord_cpu_ratios, acc_cpu_ratios = [], []
    wall_deltas, cpu_deltas = [], []
    est_gaps_ord, est_gaps_acc = [], []
    ord_wall_wins, ord_dest_wins = [], []
    acc_wall_wins, acc_dest_wins = [], []
    closing_wins = []
    code_1200_sum = 0

    for cell in CELL_ORDER:
        c = cells.get(cell)
        if not c:
            fail.append("cell missing: %s" % cell)
            continue
        for arm in TOOLS:
            block = c.get(arm)
            if not block:
                fail.append("%s missing arm %s" % (cell, arm))
                continue
            walls = block.get("wall_s") or []
            cpus = block.get("cpu_s") or []
            ratios = block.get("cpu_wall_ratio") or []
            want(len(walls) == 4, "%s/%s wall series is not 4 runs" % (cell, arm))
            want(len(cpus) == 4, "%s/%s cpu series is not 4 runs" % (cell, arm))
            want(len(ratios) == 4, "%s/%s ratio series is not 4 runs" % (cell, arm))
            if len(walls) != 4:
                continue
            best_r = max(ratios)
            expected_starved = [i for i, r in enumerate(ratios) if r < 0.75 * best_r]
            want(
                block.get("starved_indices_075") == expected_starved,
                "%s/%s starvation index mismatch" % (cell, arm),
            )
            retained = [i for i in range(4) if i not in expected_starved] or list(
                range(4)
            )
            want(
                block.get("retained_indices") == retained,
                "%s/%s retained index mismatch" % (cell, arm),
            )
            want(
                block.get("retained_runs") == len(retained),
                "%s/%s retained_runs mismatch" % (cell, arm),
            )
            kept_walls = [walls[i] for i in retained]
            kept_cpus = [cpus[i] for i in retained]
            kept_ratios = [ratios[i] for i in retained]
            want(
                close(statistics.median(kept_walls), block.get("wall_median_s")),
                "%s/%s wall median mismatch" % (cell, arm),
            )
            want(
                close(statistics.median(kept_cpus), block.get("cpu_median_s")),
                "%s/%s cpu median mismatch" % (cell, arm),
            )
            want(
                close(
                    statistics.median(kept_ratios), block.get("cpu_wall_ratio_median")
                ),
                "%s/%s cpu/wall median mismatch" % (cell, arm),
            )
            want(
                close(statistics.median(walls), block.get("wall_median_all_s")),
                "%s/%s all-run wall median mismatch" % (cell, arm),
            )
            want(
                close(statistics.median(cpus), block.get("cpu_median_all_s")),
                "%s/%s all-run cpu median mismatch" % (cell, arm),
            )
            want(
                block.get("wall_spread_pct") == spread(kept_walls),
                "%s/%s wall spread mismatch" % (cell, arm),
            )
            want(
                block.get("cpu_spread_pct") == spread(kept_cpus),
                "%s/%s cpu spread mismatch" % (cell, arm),
            )
            if expect_no_starvation:
                want(
                    expected_starved == [],
                    "%s/%s unexpected starvation flag in quiet window" % (cell, arm),
                )

        o = c.get("pnp-ordinary")
        a = c.get("pnp-accelerated")
        orca = c.get("orca")
        if not (o and a and orca):
            continue
        om, am, rm = o["wall_median_s"], a["wall_median_s"], orca["wall_median_s"]
        oc, ac, rc = o["cpu_median_s"], a["cpu_median_s"], orca["cpu_median_s"]

        want(
            close(om / rm, c.get("ratio_ordinary_over_orca"), 5e-4),
            "%s ordinary ratio mismatch" % cell,
        )
        want(
            close(am / rm, c.get("ratio_accelerated_over_orca"), 5e-4),
            "%s accelerated ratio mismatch" % cell,
        )
        want(
            close(oc / rc, c.get("cpu_ratio_ordinary_over_orca"), 5e-4),
            "%s ordinary cpu ratio mismatch" % cell,
        )
        want(
            close(ac / rc, c.get("cpu_ratio_accelerated_over_orca"), 5e-4),
            "%s accelerated cpu ratio mismatch" % cell,
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
            close(
                100 * ((om / rm) / c.get("t192_route_estimate_ordinary") - 1),
                c.get("margin_vs_t192_estimate_pct"),
                0.02,
            ),
            "%s ordinary t192 estimate gap mismatch" % cell,
        )
        want(
            close(
                100 * ((am / rm) / c.get("t192_route_estimate_accelerated") - 1),
                c.get("margin_vs_t192_estimate_accelerated_pct"),
                0.02,
            ),
            "%s accelerated t192 estimate gap mismatch" % cell,
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
        w_acc = am < rm
        want(
            c.get("wall_win_ordinary") is w_ord, "%s wall_win_ordinary mismatch" % cell
        )
        want(
            c.get("wall_win_accelerated") is w_acc,
            "%s wall_win_accelerated mismatch" % cell,
        )
        want(
            c.get("strict_separation_ordinary")
            is (max(o["retained_wall_s"]) < min(orca["retained_wall_s"])),
            "%s strict_separation_ordinary mismatch" % cell,
        )
        want(
            c.get("strict_separation_accelerated")
            is (max(a["retained_wall_s"]) < min(orca["retained_wall_s"])),
            "%s strict_separation_accelerated mismatch" % cell,
        )
        want(
            c.get("retained_runs_ok_ordinary")
            is (o["retained_runs"] >= 4 and orca["retained_runs"] >= 4),
            "%s retained_runs_ok_ordinary mismatch" % cell,
        )
        want(
            c.get("retained_runs_ok_accelerated")
            is (a["retained_runs"] >= 4 and orca["retained_runs"] >= 4),
            "%s retained_runs_ok_accelerated mismatch" % cell,
        )

        guard_ord = (
            not o["starved_indices_075"]
            and not orca["starved_indices_075"]
            and c.get("exit_codes_ordinary") == ["0"]
            and c.get("degraded_ordinary") == ["false"]
            and c.get("fatal_error_counts_ordinary") == ["0"]
            and c.get("non_fatal_error_counts_ordinary") == ["0"]
            and c.get("code_1200_count_ordinary") == 0
            and c.get("orca_matched_keys_ok") is True
            and c.get("orca_exit_codes") == ["0"]
        )
        guard_acc = (
            not a["starved_indices_075"]
            and not orca["starved_indices_075"]
            and c.get("exit_codes_accelerated") == ["0"]
            and c.get("degraded_accelerated") == ["false"]
            and c.get("fatal_error_counts_accelerated") == ["0"]
            and c.get("non_fatal_error_counts_accelerated") == ["0"]
            and c.get("code_1200_count_accelerated") == 0
            and c.get("orca_matched_keys_ok") is True
            and c.get("orca_exit_codes") == ["0"]
        )
        want(
            c.get("cpu_integrity_guard") is guard_ord,
            "%s cpu_integrity_guard mismatch" % cell,
        )
        want(
            c.get("cpu_integrity_guard_accelerated") is guard_acc,
            "%s cpu_integrity_guard_accelerated mismatch" % cell,
        )
        want(
            c.get("destination_win_ordinary") is (w_ord and guard_ord),
            "%s destination_win_ordinary mismatch" % cell,
        )
        want(
            c.get("destination_win_accelerated") is (w_acc and guard_acc),
            "%s destination_win_accelerated mismatch" % cell,
        )
        want(
            c.get("closing_cell_pass_ordinary")
            is (
                w_ord
                and guard_ord
                and (max(o["retained_wall_s"]) < min(orca["retained_wall_s"]))
                and (o["retained_runs"] >= 4 and orca["retained_runs"] >= 4)
                and c.get("identity_ordinary") is True
            ),
            "%s closing_cell_pass_ordinary mismatch" % cell,
        )
        want(
            c.get("closing_cell_pass_accelerated")
            is (
                w_acc
                and guard_acc
                and (max(a["retained_wall_s"]) < min(orca["retained_wall_s"]))
                and (a["retained_runs"] >= 4 and orca["retained_runs"] >= 4)
                and c.get("identity_accelerated") is True
            ),
            "%s closing_cell_pass_accelerated mismatch" % cell,
        )
        want(
            c.get("margin_tier_ordinary")
            == ("wide" if om / rm <= 0.85 else "close" if om / rm < 1.0 else "loss"),
            "%s margin_tier_ordinary mismatch" % cell,
        )
        want(
            c.get("margin_tier_accelerated")
            == ("wide" if am / rm <= 0.85 else "close" if am / rm < 1.0 else "loss"),
            "%s margin_tier_accelerated mismatch" % cell,
        )

        if w_ord:
            ord_wall_wins.append(cell)
        if w_ord and guard_ord:
            ord_dest_wins.append(cell)
        if w_acc:
            acc_wall_wins.append(cell)
        if w_acc and guard_acc:
            acc_dest_wins.append(cell)
        if c.get("closing_cell_pass_ordinary") is True:
            closing_wins.append(cell)

        ord_ratios.append(om / rm)
        acc_ratios.append(am / rm)
        ord_cpu_ratios.append(oc / rc)
        acc_cpu_ratios.append(ac / rc)
        wall_deltas.append(c["accel_vs_ordinary_wall_pct"])
        cpu_deltas.append(c["accel_vs_ordinary_cpu_pct"])
        est_gaps_ord.append(c["margin_vs_t192_estimate_pct"])
        est_gaps_acc.append(c["margin_vs_t192_estimate_accelerated_pct"])

        want(
            c.get("reference_hash") == ref_hashes.get(cell),
            "%s reference hash != t182 reference.json" % cell,
        )
        want(
            c.get("reference_bytes") == ref_bytes.get(cell),
            "%s reference bytes != t182 reference.json" % cell,
        )
        want(c.get("identity_ordinary") is True, "%s ordinary identity false" % cell)
        want(
            c.get("identity_accelerated") is True,
            "%s accelerated identity false" % cell,
        )
        want(
            c.get("hash_ordinary") == ref_hashes.get(cell),
            "%s ordinary hash != currency" % cell,
        )
        want(
            c.get("hash_accelerated") == ref_hashes.get(cell),
            "%s accelerated hash != currency" % cell,
        )
        want(
            c.get("gcode_bytes_ordinary") == ref_bytes.get(cell),
            "%s ordinary bytes != currency" % cell,
        )
        want(
            c.get("gcode_bytes_accelerated") == ref_bytes.get(cell),
            "%s accelerated bytes != currency" % cell,
        )
        want(c.get("exit_codes_ordinary") == ["0"], "%s ordinary exit codes" % cell)
        want(
            c.get("exit_codes_accelerated") == ["0"], "%s accelerated exit codes" % cell
        )
        want(c.get("degraded_ordinary") == ["false"], "%s ordinary degraded" % cell)
        want(
            c.get("degraded_accelerated") == ["false"], "%s accelerated degraded" % cell
        )
        want(
            c.get("fatal_error_counts_ordinary") == ["0"],
            "%s ordinary fatal errors" % cell,
        )
        want(
            c.get("fatal_error_counts_accelerated") == ["0"],
            "%s accelerated fatal errors" % cell,
        )
        want(
            c.get("non_fatal_error_counts_ordinary") == ["0"],
            "%s ordinary non-fatal errors" % cell,
        )
        want(
            c.get("non_fatal_error_counts_accelerated") == ["0"],
            "%s accelerated non-fatal errors" % cell,
        )
        want(
            c.get("code_1200_count_ordinary") == 0, "%s ordinary code-1200 != 0" % cell
        )
        want(
            c.get("code_1200_count_accelerated") == 0,
            "%s accelerated code-1200 != 0" % cell,
        )
        code_1200_sum += (c.get("code_1200_count_ordinary") or 0) + (
            c.get("code_1200_count_accelerated") or 0
        )

        want(c.get("orca_matched_keys_ok") is True, "%s matched keys not ok" % cell)
        want(c.get("orca_exit_codes") == ["0"], "%s orca exit codes" % cell)
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
            "ordinary ratio min mismatch",
        )
        want(
            close(max(ord_ratios), ranges["ratio_ordinary_over_orca"][1], 5e-4),
            "ordinary ratio max mismatch",
        )
        want(
            close(min(acc_ratios), ranges["ratio_accelerated_over_orca"][0], 5e-4),
            "accelerated ratio min mismatch",
        )
        want(
            close(max(acc_ratios), ranges["ratio_accelerated_over_orca"][1], 5e-4),
            "accelerated ratio max mismatch",
        )
        want(
            close(min(ord_cpu_ratios), ranges["cpu_ratio_ordinary_over_orca"][0], 5e-4),
            "ordinary cpu ratio min mismatch",
        )
        want(
            close(max(ord_cpu_ratios), ranges["cpu_ratio_ordinary_over_orca"][1], 5e-4),
            "ordinary cpu ratio max mismatch",
        )
        want(
            close(
                min(acc_cpu_ratios), ranges["cpu_ratio_accelerated_over_orca"][0], 5e-4
            ),
            "accelerated cpu ratio min mismatch",
        )
        want(
            close(
                max(acc_cpu_ratios), ranges["cpu_ratio_accelerated_over_orca"][1], 5e-4
            ),
            "accelerated cpu ratio max mismatch",
        )
        want(
            close(min(wall_deltas), ranges["accel_vs_ordinary_wall_pct"][0], 0.01),
            "wall delta min mismatch",
        )
        want(
            close(max(wall_deltas), ranges["accel_vs_ordinary_wall_pct"][1], 0.01),
            "wall delta max mismatch",
        )
        want(
            close(min(cpu_deltas), ranges["accel_vs_ordinary_cpu_pct"][0], 0.01),
            "cpu delta min mismatch",
        )
        want(
            close(max(cpu_deltas), ranges["accel_vs_ordinary_cpu_pct"][1], 0.01),
            "cpu delta max mismatch",
        )
        want(
            close(min(est_gaps_ord), ranges["margin_vs_t192_estimate_pct"][0], 0.02),
            "t192 gap min mismatch",
        )
        want(
            close(max(est_gaps_ord), ranges["margin_vs_t192_estimate_pct"][1], 0.02),
            "t192 gap max mismatch",
        )
        want(
            close(
                min(est_gaps_acc),
                ranges["margin_vs_t192_estimate_accelerated_pct"][0],
                0.02,
            ),
            "t192 acc gap min mismatch",
        )
        want(
            close(
                max(est_gaps_acc),
                ranges["margin_vs_t192_estimate_accelerated_pct"][1],
                0.02,
            ),
            "t192 acc gap max mismatch",
        )

    dt = summary.get("destination_tally", {})
    want(
        dt.get("ordinary_wall_wins") == len(ord_wall_wins),
        "ordinary_wall_wins mismatch",
    )
    want(
        dt.get("ordinary_wall_win_cells") == ord_wall_wins,
        "ordinary_wall_win_cells mismatch",
    )
    want(
        dt.get("ordinary_cpu_corroborated_wins") == len(ord_dest_wins),
        "ordinary_cpu_corroborated_wins mismatch",
    )
    want(
        dt.get("ordinary_cpu_corroborated_win_cells") == ord_dest_wins,
        "ordinary_cpu_corroborated_win_cells mismatch",
    )
    want(
        dt.get("accelerated_wall_wins") == len(acc_wall_wins),
        "accelerated_wall_wins mismatch",
    )
    want(
        dt.get("accelerated_wall_win_cells") == acc_wall_wins,
        "accelerated_wall_win_cells mismatch",
    )
    want(
        dt.get("accelerated_cpu_corroborated_wins") == len(acc_dest_wins),
        "accelerated_cpu_corroborated_wins mismatch",
    )
    want(
        dt.get("accelerated_cpu_corroborated_win_cells") == acc_dest_wins,
        "accelerated_cpu_corroborated_win_cells mismatch",
    )

    ct = summary.get("closing_tally", {})
    want(
        ct.get("cells_passed") == len(closing_wins),
        "closing_tally.cells_passed mismatch",
    )
    want(ct.get("passed_cells") == closing_wins, "closing_tally.passed_cells mismatch")
    want(
        ct.get("all_eight_passed") is (len(closing_wins) == 8),
        "closing_tally.all_eight_passed mismatch",
    )
    want(
        ct.get("starved_cells")
        == sorted(
            {
                cell
                for cell in CELL_ORDER
                for arm in TOOLS
                if cells.get(cell, {}).get(arm, {}).get("starved_indices_075")
            }
        ),
        "closing_tally.starved_cells mismatch",
    )
    if expect_no_starvation:
        want(
            ct.get("starved_cells") == [],
            "starvation flags present on the closing board",
        )

    ident = summary.get("identity", {})
    want(ident.get("ordinary_all_eight") is True, "identity.ordinary_all_eight false")
    want(
        ident.get("accelerated_all_eight") is True,
        "identity.accelerated_all_eight false",
    )
    want(ident.get("ordinary_missing") == [], "ordinary identity misses recorded")
    want(ident.get("accelerated_missing") == [], "accelerated identity misses recorded")
    want(ident.get("code_1200_total") == code_1200_sum == 0, "code_1200_total != 0")
    want(
        summary.get("matched_keys", {}).get("all_eight") is True,
        "matched_keys.all_eight false",
    )
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
        want(
            load["min"] <= load["median"] <= load["max"],
            "box load min/median/max not ordered",
        )
    fmem = summary.get("free_memory_mb", {})
    if fmem.get("samples"):
        want(
            fmem["min"] <= fmem["median"] <= fmem["max"],
            "free_memory_mb min/median/max not ordered",
        )
    if load_csv_path and os.path.exists(load_csv_path):
        csv_load, csv_mem = load_stats_from_csv(load_csv_path)
        want(load == csv_load, "box_load_pct does not match load-samples csv")
        want(fmem == csv_mem, "free_memory_mb does not match load-samples csv")
    else:
        want(False, "load-samples.csv missing for the closing board")

    return ok, fail


def check_attribution(attr, ref_doc):
    ok = 0
    fail = []
    cell = "benchy-classic-on"

    def want(cond, msg):
        nonlocal ok
        if cond:
            ok += 1
        else:
            fail.append(msg)

    want(attr.get("ticket") == 193, "attribution ticket is not 193")
    want(attr.get("cell") == cell, "attribution cell is not benchy-classic-on")

    ins = attr.get("instrumented", {})
    phases = ins.get("phases_ms", {})
    want(ins.get("elapsed_ms") == 2547, "attribution elapsed_ms != 2547")
    want(phases.get("validation") == 3, "attribution validation != 3 ms")
    want(phases.get("prepass") == 1629, "attribution prepass != 1629 ms")
    want(phases.get("per_layer") == 563, "attribution per_layer != 563 ms")
    want(phases.get("postpass") == 272, "attribution postpass != 272 ms")
    want(
        ins.get("outside_brackets_ms") == ins.get("elapsed_ms") - sum(phases.values()),
        "attribution outside_brackets != elapsed - phases",
    )
    stages = ins.get("postpass_stages_ms", {})
    want(stages.get("PostPass::GCodePostProcess") == 127, "GCodePostProcess != 127 ms")
    want(stages.get("PostPass::LayerFinalization") == 82, "LayerFinalization != 82 ms")
    want(stages.get("PostPass::GCodeEmit") == 45, "GCodeEmit != 45 ms")
    want(stages.get("PostPass::GCodeSerialize") == 14, "GCodeSerialize != 14 ms")
    want(
        sum(stages.values()) <= phases.get("postpass"),
        "attribution postpass stage sum exceeds postpass",
    )

    prof = attr.get("profile", {})
    fuel = prof.get("module_fuel", {})
    want(prof.get("fuel_total") == 17998173710, "attribution fuel_total mismatch")
    want(
        prof.get("wall_total_ns") == 6387801700,
        "attribution profile wall_total mismatch",
    )
    want(fuel.get("com.core.part-cooling") == 143211608, "part-cooling fuel mismatch")
    want(
        fuel.get("com.core.machine-gcode-emit") == 218641791,
        "machine-gcode-emit fuel mismatch",
    )
    for module_id, value in fuel.items():
        want(value is None or value >= 0, "negative fuel for %s" % module_id)

    out = attr.get("output", {})
    want(
        out.get("matches_reference") is True,
        "attribution output does not match reference",
    )
    want(
        out.get("sha256_of_gcode") == ref_doc["active_set"].get(cell),
        "attribution hash != t182 reference",
    )
    want(
        out.get("bytes") == ref_doc["bytes"].get(cell),
        "attribution bytes != t182 reference",
    )
    want(
        out.get("reference_sha256") == out.get("sha256_of_gcode"),
        "attribution reference hash echo mismatch",
    )
    want(
        out.get("reference_bytes") == out.get("bytes"),
        "attribution reference bytes echo mismatch",
    )
    return ok, fail


def main():
    if not os.path.exists(SUMMARY):
        sys.exit("missing reduced artifact: %s" % SUMMARY)
    if not os.path.exists(ATTR):
        sys.exit("missing reduced attribution artifact: %s" % ATTR)
    with open(SUMMARY, encoding="utf-8") as f:
        summary = json.load(f)
    with open(REF, encoding="utf-8") as f:
        ref_doc = json.load(f)
    with open(ATTR, encoding="utf-8") as f:
        attr = json.load(f)

    if "--self-test" in sys.argv:
        ok, fail = check_board(summary, ref_doc, LOAD_CSV, expect_no_starvation=True)
        aok, afail = check_attribution(attr, ref_doc)
        if fail or afail:
            print("FAIL (baseline artifact is already inconsistent):")
            for item in fail + afail:
                print("  -", item)
            return 1

        mutations = []

        def mut(name, fn):
            s = copy.deepcopy(summary)
            fn(s)
            mutations.append((name, s))

        mut(
            "median",
            lambda s: s["cells"]["base-classic-off"]["pnp-ordinary"].__setitem__(
                "wall_median_s", 999.0
            ),
        )
        mut(
            "ratio",
            lambda s: s["cells"]["base-classic-off"].__setitem__(
                "ratio_ordinary_over_orca", 1.0
            ),
        )
        mut(
            "cpu_ratio",
            lambda s: s["cells"]["base-arachne-off"].__setitem__(
                "cpu_ratio_ordinary_over_orca", 0.5
            ),
        )
        mut(
            "required_cut",
            lambda s: s["cells"]["benchy-classic-on"].__setitem__(
                "required_cut_ordinary_pct", 0.0
            ),
        )
        mut(
            "wall_win_flag",
            lambda s: s["cells"]["base-classic-off"].__setitem__(
                "wall_win_ordinary", False
            ),
        )
        mut(
            "dest_win_flag",
            lambda s: s["cells"]["base-classic-off"].__setitem__(
                "destination_win_ordinary", False
            ),
        )
        mut(
            "guard",
            lambda s: s["cells"]["base-classic-off"].__setitem__(
                "cpu_integrity_guard", False
            ),
        )
        mut(
            "closing_pass",
            lambda s: s["cells"]["benchy-arachne-off"].__setitem__(
                "closing_cell_pass_ordinary", False
            ),
        )
        mut(
            "closing_tally", lambda s: s["closing_tally"].__setitem__("cells_passed", 8)
        )
        mut(
            "strict_sep",
            lambda s: s["cells"]["base-arachne-off"].__setitem__(
                "strict_separation_accelerated", False
            ),
        )
        mut(
            "retained_ok",
            lambda s: s["cells"]["base-classic-on"].__setitem__(
                "retained_runs_ok_ordinary", False
            ),
        )
        mut(
            "margin_tier",
            lambda s: s["cells"]["benchy-classic-off"].__setitem__(
                "margin_tier_ordinary", "loss"
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
            "starved",
            lambda s: s["cells"]["benchy-classic-on"]["orca"].__setitem__(
                "starved_indices_075", [0]
            ),
        )
        mut(
            "retained_runs",
            lambda s: s["cells"]["benchy-classic-on"]["pnp-ordinary"].__setitem__(
                "retained_runs", 3
            ),
        )
        mut(
            "hash",
            lambda s: s["cells"]["benchy-classic-off"].__setitem__(
                "hash_accelerated", "0" * 64
            ),
        )
        mut(
            "bytes",
            lambda s: s["cells"]["base-classic-on"].__setitem__(
                "gcode_bytes_ordinary", 1234
            ),
        )
        mut(
            "code_1200",
            lambda s: s["cells"]["base-classic-on"].__setitem__(
                "code_1200_count_ordinary", 1
            ),
        )
        mut(
            "matched_key",
            lambda s: s["cells"]["base-arachne-off"]["orca_matched_keys"][
                "resolved_support_style"
            ].__setitem__("ok", False),
        )
        mut("drop_cell", lambda s: s["cells"].pop("benchy-arachne-on"))
        mut("measured_rows", lambda s: s["protocol"].__setitem__("measured_rows", 95))
        mut("load_mismatch", lambda s: s["box_load_pct"].__setitem__("median", 1.0))
        mut(
            "guests_exit",
            lambda s: s["identities"].__setitem__("accelerated_guests_check_exit", 1),
        )
        mut(
            "identity_flag",
            lambda s: s["cells"]["benchy-classic-off"].__setitem__(
                "identity_ordinary", False
            ),
        )
        mut(
            "t192_gap",
            lambda s: s["cells"]["base-classic-off"].__setitem__(
                "margin_vs_t192_estimate_pct", 99.0
            ),
        )

        rejected = 0
        for name, s in mutations:
            _, fails = check_board(s, ref_doc, LOAD_CSV, expect_no_starvation=True)
            if fails:
                rejected += 1
            else:
                print("SELF-TEST FAILURE: mutation %r was accepted" % (name,))

        attr_mutations = []

        def amut(name, fn):
            a = copy.deepcopy(attr)
            fn(a)
            attr_mutations.append((name, a))

        amut("elapsed", lambda a: a["instrumented"].__setitem__("elapsed_ms", 9999))
        amut(
            "prepass",
            lambda a: a["instrumented"]["phases_ms"].__setitem__("prepass", 1),
        )
        amut(
            "outside",
            lambda a: a["instrumented"].__setitem__("outside_brackets_ms", 999),
        )
        amut(
            "postpass_stage",
            lambda a: a["instrumented"]["postpass_stages_ms"].__setitem__(
                "PostPass::GCodePostProcess", 0
            ),
        )
        amut(
            "part_cooling_fuel",
            lambda a: a["profile"]["module_fuel"].__setitem__(
                "com.core.part-cooling", 0
            ),
        )
        amut(
            "attr_hash",
            lambda a: a["output"].__setitem__("sha256_of_gcode", "0" * 64),
        )
        amut(
            "attr_match",
            lambda a: a["output"].__setitem__("matches_reference", False),
        )
        amut("attr_cell", lambda a: a.__setitem__("cell", "benchy-classic-off"))

        rejected_attr = 0
        for name, a in attr_mutations:
            _, fails = check_attribution(a, ref_doc)
            if fails:
                rejected_attr += 1
            else:
                print(
                    "SELF-TEST FAILURE: attribution mutation %r was accepted" % (name,)
                )

        total = len(mutations) + len(attr_mutations)
        rejected_total = rejected + rejected_attr
        print("self-test: %d/%d mutations rejected" % (rejected_total, total))
        return 0 if rejected_total == total else 1

    ok, all_fail = check_board(summary, ref_doc, LOAD_CSV, expect_no_starvation=True)
    aok, afail = check_attribution(attr, ref_doc)
    ok += aok
    all_fail.extend(afail)
    if all_fail:
        print("FAIL: %d/%d checks" % (len(all_fail), ok + len(all_fail)))
        for item in all_fail:
            print("  -", item)
        return 1
    print("PASS: %d/%d checks" % (ok, ok))
    return 0


if __name__ == "__main__":
    sys.exit(main())

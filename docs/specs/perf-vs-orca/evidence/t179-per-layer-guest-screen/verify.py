#!/usr/bin/env python3
"""Checkout-only verifier + negative-control self-test for Wayfinder Ticket 179.

Verifies every number in `reduced/arithmetic.json`, `reduced/probe_attribution.json`,
`reduced/screen_runs.json`, and `probe.patch` strictly from committed in-repo files
(Python 3 stdlib only, zero raw captures, zero heavy STL fixtures).
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REDUCED = HERE / "reduced"
T175_REF = HERE.parent / "t175-tiled-collision-carve" / "reference.json"
T177_ARITH = HERE.parent / "t177-post-t175-attribution" / "reduced" / "arithmetic.json"
T177_PROF = HERE.parent / "t177-post-t175-attribution" / "reduced" / "profile-detail-clean.json"
T178_ARITH = HERE.parent / "t178-planner-phase3-union" / "reduced" / "arithmetic.json"

sys.path.insert(0, str(HERE))
import arithmetic  # noqa: E402


def verify_all(
    arith_disk: dict,
    probe: dict,
    screen: dict,
    t175_ref: dict,
    t177_arith: dict,
    t177_prof: dict,
    t178_arith: dict,
    probe_text: str,
) -> list[str]:
    errors: list[str] = []

    # 1. Leaf-for-leaf recomputation of reduced/arithmetic.json
    recomputed = arithmetic.compute_arithmetic(
        t175_ref=t175_ref,
        t177_arith=t177_arith,
        t177_prof=t177_prof,
        t178_arith=t178_arith,
        probe=probe,
        screen=screen,
    )
    if arith_disk != recomputed:
        errors.append("arithmetic_recompute_mismatch: reduced/arithmetic.json differs from compute_arithmetic()")

    # 2. Qualification completeness, reference identity, diagnostics, and fuel fold conservation
    q = recomputed["qualification"]
    if q["probe_ref_ok"] != 8 or q["probe_clean_ok"] != 8:
        errors.append(f"probe_qual_failed:{q['probe_ref_ok']}/8 ref, {q['probe_clean_ok']}/8 clean")
    if q["fuel_fold_exact_ok"] != 16 or q["fuel_fold_total"] != 16:
        errors.append(f"fuel_fold_mismatch:{q['fuel_fold_exact_ok']}/{q['fuel_fold_total']}")
    if q["cand_fuel_runs"] != 18 or q["cand_fuel_ref_ok"] != 18 or q["cand_fuel_clean_ok"] != 18:
        errors.append(
            f"cand_fuel_qual_failed:runs={q['cand_fuel_runs']},ref={q['cand_fuel_ref_ok']},clean={q['cand_fuel_clean_ok']}"
        )
    if q["screen_instr_runs"] != 26 or q["screen_instr_ref_ok"] != 26 or q["screen_instr_clean_ok"] != 26:
        errors.append(
            f"screen_instr_qual_failed:runs={q['screen_instr_runs']},ref={q['screen_instr_ref_ok']},clean={q['screen_instr_clean_ok']}"
        )
    if q["screen_wall_runs"] != 78 or q["screen_wall_ref_ok"] != 78:
        errors.append(f"screen_wall_qual_failed:runs={q['screen_wall_runs']},ref={q['screen_wall_ref_ok']}")
    if not q["all_byte_identical_t175"] or q["total_verified_runs"] != 130:
        errors.append(f"total_ref_check_failed:total={q['total_verified_runs']},all={q['all_byte_identical_t175']}")

    # 3. tree-support attribution & fuel ablation invariants
    ts_inv = recomputed["tree_support_invariance"]
    if not ts_inv["benchy_counters_equal"] or not ts_inv["base_counters_equal"]:
        errors.append(f"ts_classic_arachne_invariance_failed:{ts_inv}")
    for cell in arithmetic.SUPPORTS_ON_CELLS:
        ta = recomputed["tree_support_attribution"][cell]
        if abs(ta["probe_fuel_overhead_pct"]) >= 0.15:
            errors.append(f"ts_probe_overhead_high:{cell}:{ta['probe_fuel_overhead_pct']}")
        cnt = ta["counters"]
        if cnt["paint_has_ann"] != 0 or cnt["paint_cover_hits"] != 0:
            errors.append(f"ts_paint_nonzero:{cell}")
        if ta["candidates_fuel"]["TS1_paint_and_skeleton"]["fuel_cut_pct"] < 95.0:
            errors.append(f"ts1_fuel_cut_low:{cell}")
        ts2 = ta["candidates_fuel"]["TS2_ts1_plus_role_union_skip"]
        if ts2 is not None and ts2["fuel_cut_pct"] < 98.0:
            errors.append(f"ts2_fuel_cut_low:{cell}")

    # 4. infill-linker attribution, L2 straggler, & fuel ablation invariants
    for cell in arithmetic.ALL_CELLS:
        ia = recomputed["infill_linker_attribution"][cell]
        if abs(ia["probe_fuel_overhead_pct"]) >= 0.50:
            errors.append(f"il_probe_overhead_high:{cell}:{ia['probe_fuel_overhead_pct']}")
        if ia["counters"]["union_groups"] != 0:
            errors.append(f"il_union_groups_nonzero:{cell}")
        if ia["candidates_fuel"]["IL1_clipper64_open_tag"]["fuel_cut_pct"] < 49.0:
            errors.append(f"il1_fuel_cut_low:{cell}")
        if ia["candidates_fuel"]["IL_ALL_il1_il2_il3"]["fuel_cut_pct"] < 72.0:
            errors.append(f"ilall_fuel_cut_low:{cell}")

    l2_bco = recomputed["infill_linker_benchy_l2"]["benchy-classic-off"]
    if l2_bco["l2_share_of_slice_il_fuel_pct"] < 30.0:
        errors.append(f"l2_share_low:{l2_bco['l2_share_of_slice_il_fuel_pct']}")
    if l2_bco["l2_top_kernels"]["clip_batch_repair"]["fuel_pct"] < 65.0:
        errors.append(f"l2_repair_share_low:{l2_bco['l2_top_kernels']['clip_batch_repair']['fuel_pct']}")

    # 5. Unsandboxed combined TS-2 + IL-ALL 8-cell screen & L2 unmasking elimination
    gs = recomputed["combined_ts2_ilall_gate_summary"]
    if gs["cells_clearing_080_wall_floor"] != 8:
        errors.append(f"combined_wall_floor_short:{gs['cells_clearing_080_wall_floor']}/8")
    if gs["cells_strict_wall_separation"] < 7:
        errors.append(f"combined_strict_wall_sep_short:{gs['cells_strict_wall_separation']}/8")
    if gs["cells_strict_cpu_separation"] != 8:
        errors.append(f"combined_strict_cpu_sep_short:{gs['cells_strict_cpu_separation']}/8")
    for cell in arithmetic.ALL_CELLS:
        c8 = recomputed["combined_ts2_ilall_8cell_screen"][cell]
        if c8["wall_round_wins"] != "3/3" or c8["cpu_round_wins"] != "3/3":
            errors.append(f"combined_round_wins_short:{cell}:w={c8['wall_round_wins']},c={c8['cpu_round_wins']}")
    for cell in ("benchy-classic-on", "benchy-arachne-on"):
        ts2_tail = recomputed["screen_summary"][cell]["arm_ts2"]["instr_straggler_tail_cand_ms"]
        both_tail = recomputed["screen_summary"][cell]["arm_both"]["instr_straggler_tail_cand_ms"]
        if ts2_tail < 50 or both_tail > 10:
            errors.append(f"l2_unmask_elim_mismatch:{cell}:ts2={ts2_tail},both={both_tail}")

    # 6. Preserved observation-only probe.patch markers
    for marker in (
        "crates/slicer-sdk/src/traits.rs",
        "crates/slicer-core/src/polygon_ops.rs",
        "modules/core-modules/tree-support/src/lib.rs",
        "modules/core-modules/infill-linker/src/orchestrate.rs",
        "t179::ts::paint_policy",
        "t179::il::clip_batch_repair",
    ):
        if marker not in probe_text:
            errors.append(f"probe_patch_missing:{marker}")

    return errors


def run_self_test(
    arith_disk: dict,
    probe: dict,
    screen: dict,
    t175_ref: dict,
    t177_arith: dict,
    t177_prof: dict,
    t178_arith: dict,
    probe_text: str,
) -> bool:
    controls = []

    def add_control(name: str, expected_prefix: str, mutate, recompute_arith: bool = True):
        a = copy.deepcopy(arith_disk)
        p = copy.deepcopy(probe)
        s = copy.deepcopy(screen)
        tr = copy.deepcopy(t175_ref)
        t7a = copy.deepcopy(t177_arith)
        t7p = copy.deepcopy(t177_prof)
        t8a = copy.deepcopy(t178_arith)
        pt = probe_text
        res = mutate(a, p, s, tr, t7a, t7p, t8a, pt)
        if res is not None:
            a, p, s, tr, t7a, t7p, t8a, pt = res
        if recompute_arith:
            a = arithmetic.compute_arithmetic(
                t175_ref=tr,
                t177_arith=t7a,
                t177_prof=t7p,
                t178_arith=t8a,
                probe=p,
                screen=s,
            )
        errs = verify_all(a, p, s, tr, t7a, t7p, t8a, pt)
        matched = [e for e in errs if e.startswith(expected_prefix)]
        controls.append((name, bool(matched), matched[:1] or errs[:1]))

    add_control(
        "1_arithmetic_drift",
        "arithmetic_recompute_mismatch",
        lambda a, *_: a["combined_ts2_ilall_gate_summary"].__setitem__("min_wall_saved_pct", 0.0) and None,
        recompute_arith=False,
    )
    add_control(
        "2_probe_ref_sha_tamper",
        "probe_qual_failed",
        lambda a, p, *_: p["probe_cells"]["benchy-classic-on"].__setitem__("reference_sha256", "deadbeef") and None,
    )
    add_control(
        "3_probe_code1200_nonzero",
        "probe_qual_failed",
        lambda a, p, *_: p["probe_cells"]["base-classic-on"].__setitem__("code_1200_count", 1) and None,
    )
    add_control(
        "4_fuel_fold_mismatch",
        "fuel_fold_mismatch",
        lambda a, p, *_: p["probe_cells"]["benchy-classic-on"]["modules"]["com.core.tree-support"].__setitem__(
            "unmarked_self_fuel", 1
        )
        and None,
    )
    add_control(
        "5_cand_fuel_degraded",
        "cand_fuel_qual_failed",
        lambda a, p, *_: p["candidate_fuel_ablations"]["cand2_ts1_ilall"]["base-classic-off"].__setitem__(
            "degraded", True
        )
        and None,
    )
    add_control(
        "6_screen_instr_sha_tamper",
        "screen_instr_qual_failed",
        lambda a, p, s, *_: s["instrumented"]["benchy-classic-on"]["arm_both"].__setitem__(
            "gcode_sha256", "deadbeef"
        )
        and None,
    )
    add_control(
        "7_screen_wall_bytes_tamper",
        "screen_wall_qual_failed",
        lambda a, p, s, *_: s["uninstrumented_wall"]["base-classic-on"]["rows"][0].__setitem__("gcode_bytes", 1)
        and None,
    )
    add_control(
        "8_ts_classic_arachne_diverge",
        "ts_classic_arachne_invariance_failed",
        lambda a, p, *_: p["probe_cells"]["benchy-arachne-on"]["tree_support_counters"].__setitem__(
            "paint_ring_edges", 1
        )
        and None,
    )
    add_control(
        "9_ts_paint_nonzero",
        "ts_paint_nonzero",
        lambda a, p, *_: p["probe_cells"]["base-classic-on"]["tree_support_counters"].__setitem__("paint_has_ann", 1)
        and None,
    )
    add_control(
        "10_ts1_fuel_cut_regressed",
        "ts1_fuel_cut_low",
        lambda a, p, *_: p["candidate_fuel_ablations"]["cand1_ts1_il1"]["benchy-classic-on"]["modules"][
            "com.core.tree-support"
        ].__setitem__("total_fuel", 10_000_000_000)
        and None,
    )
    add_control(
        "11_il_union_groups_nonzero",
        "il_union_groups_nonzero",
        lambda a, p, *_: p["probe_cells"]["base-classic-off"]["infill_linker_counters"].__setitem__("union_groups", 1)
        and None,
    )
    add_control(
        "12_ilall_fuel_cut_regressed",
        "ilall_fuel_cut_low",
        lambda a, p, *_: p["candidate_fuel_ablations"]["cand2_ts1_ilall"]["benchy-classic-off"]["modules"][
            "com.core.infill-linker"
        ].__setitem__("total_fuel", 8_000_000_000)
        and None,
    )
    add_control(
        "13_combined_wall_subfloor",
        "combined_wall_floor_short",
        lambda a, p, s, *_: s["uninstrumented_wall"]["base-arachne-off"]["summary"]["arm_il"].__setitem__(
            "wall_saved_pct_vs_baseline", 0.5
        )
        and None,
    )
    add_control(
        "14_probe_patch_missing_marker",
        "probe_patch_missing",
        lambda a, p, s, tr, t7a, t7p, t8a, pt: (a, p, s, tr, t7a, t7p, t8a, ""),
        recompute_arith=False,
    )

    passed = 0
    for name, rejected, sample in controls:
        status = "REJECTED (OK)" if rejected else "MISSED (FAIL)"
        if rejected:
            passed += 1
        print(f"  control {name:34}: {status} {sample}")
    print(f"SELF-TEST: {passed}/{len(controls)} negative controls rejected")
    return passed == len(controls)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()

    arith_disk = json.loads((REDUCED / "arithmetic.json").read_text(encoding="utf-8"))
    probe = json.loads((REDUCED / "probe_attribution.json").read_text(encoding="utf-8"))
    screen = json.loads((REDUCED / "screen_runs.json").read_text(encoding="utf-8"))
    t175_ref = json.loads(T175_REF.read_text(encoding="utf-8"))
    t177_arith = json.loads(T177_ARITH.read_text(encoding="utf-8"))
    t177_prof = json.loads(T177_PROF.read_text(encoding="utf-8"))
    t178_arith = json.loads(T178_ARITH.read_text(encoding="utf-8"))
    probe_text = (HERE / "probe.patch").read_text(encoding="utf-8")

    errs = verify_all(arith_disk, probe, screen, t175_ref, t177_arith, t177_prof, t178_arith, probe_text)
    if errs:
        print("VERIFY FAIL:")
        for e in errs:
            print("  -", e)
        return 1
    print(
        "VERIFY PASS: 6/6 checks green (arithmetic recompute, 130/130 ref+diag, 16/16 exact fuel fold, "
        "tree-support invariants, infill-linker + L2 invariants, 8/8 combined screen floor + 24/24 round wins, probe.patch)"
    )

    if args.self_test:
        if not run_self_test(arith_disk, probe, screen, t175_ref, t177_arith, t177_prof, t178_arith, probe_text):
            return 1
    print("PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

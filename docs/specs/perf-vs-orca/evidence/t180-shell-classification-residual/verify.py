#!/usr/bin/env python3
"""Checkout-only verifier + negative-control self-test for Wayfinder Ticket 180.

Verifies every number in `reduced/arithmetic.json`, `reduced/probe.json`,
`reduced/replay_benchy.json`, `reduced/replay_base.json`, and `probe.patch`
strictly from committed in-repo files (Python 3 stdlib only, zero raw captures,
zero heavy STL fixtures).
"""

from __future__ import annotations

import argparse
import copy
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REDUCED = HERE / "reduced"
PROBE_PATCH = HERE / "probe.patch"
T175_REF = HERE.parent / "t175-tiled-collision-carve" / "reference.json"
T177_ARITH = HERE.parent / "t177-post-t175-attribution" / "reduced" / "arithmetic.json"
T178_ARITH = HERE.parent / "t178-planner-phase3-union" / "reduced" / "arithmetic.json"
T179_ARITH = HERE.parent / "t179-per-layer-guest-screen" / "reduced" / "arithmetic.json"

sys.path.insert(0, str(HERE))
import arithmetic  # noqa: E402

EXACT_MECHANISMS = (
    "M0_baseline",
    "E1_p2_par_merge_diff",
    "E2_ib_lazy_cluster_cache",
    "E3_ib_phaseA_windowed",
    "E4a_ext_fast_score",
    "E5_p2_bbox_filter_nbr",
    "EXACT_BUNDLE",
)

NON_EXACT_MECHANISMS = (
    "E4ab_ext_score_rect_clip",
    "EXACT_PLUS_E4b",
    "L1_sliver_prefilter",
    "L2_p2_per_poly_neg",
    "L3_p1_canon_open",
    "EXACT_PLUS_P2_PERPOLY",
    "EXACT_PLUS_P1_P2_CANON",
    "FULL_STACK_WITH_E4b",
)

EXPECTED_PATCH_FILES = {
    "crates/slicer-core/src/algos/bridge_over_infill.rs",
    "crates/slicer-core/src/algos/prepass_slice.rs",
    "crates/slicer-runtime/src/slice_postprocess_prepass.rs",
}


def verify_all(
    *,
    arith_disk: dict,
    probe: dict,
    rep_benchy: dict,
    rep_base: dict,
    t175_ref: dict,
    t177_arith: dict,
    t178_arith: dict,
    t179_arith: dict,
    probe_patch_text: str,
) -> list[str]:
    errors: list[str] = []

    # 1. Leaf-for-leaf recomputation of reduced/arithmetic.json
    recomputed = arithmetic.compute_arithmetic(
        t175_ref=t175_ref,
        t177_arith=t177_arith,
        t178_arith=t178_arith,
        t179_arith=t179_arith,
        probe=probe,
        rep_benchy=rep_benchy,
        rep_base=rep_base,
    )
    if arith_disk != recomputed:
        errors.append("arithmetic_recompute_mismatch: reduced/arithmetic.json differs from compute_arithmetic()")

    # 2. Probe capture qualification & byte-identity vs t175/reference.json
    q = recomputed["qualification"]
    if q["total_runs"] != 15:
        errors.append(f"probe_run_count_wrong:{q['total_runs']}")
    if q["exit_zero_ok"] != 15 or q["ref_sha_and_bytes_ok"] != 15 or not q["all_byte_identical_t175"]:
        errors.append(
            f"probe_ref_check_failed:exit0={q['exit_zero_ok']},ref={q['ref_sha_and_bytes_ok']},all={q['all_byte_identical_t175']}"
        )
    if q["qual_runs"] != 3 or q["qual_clean_ok"] != 3:
        errors.append(f"probe_qual_unclean:runs={q['qual_runs']},clean={q['qual_clean_ok']}")
    if q["invariance_pairs"] != 4 or not q["invariance_all_identical"]:
        errors.append(
            f"invariance_failed:pairs={q['invariance_pairs']},all={q['invariance_all_identical']}"
        )

    # 3. Exact op-kind call-count reconciliation against t177 (--profile scopes)
    crecon = recomputed["t177_reconciliation"]["call_reconciliation"]
    expected_calls = {"benchy-classic-off": 4086, "benchy-classic-on": 4086, "base-classic-off": 8912}
    for cell, want_calls in expected_calls.items():
        cr = crecon[cell]
        if not cr["exact_match"] or cr["probe_clipper_calls"] != want_calls:
            errors.append(
                f"call_reconciliation_mismatch:{cell}:probe={cr['probe_clipper_calls']},want={want_calls},diffs={cr['diffs_by_op_kind']}"
            )

    # 4. Bracket sum reconciliation & key structural attribution checks
    for cell in ("benchy-classic-off", "benchy-classic-on", "base-classic-off"):
        pa = recomputed["probe_attribution"][cell]
        if pa["bracket_gap_pct"] > 0.20:
            errors.append(f"bracket_gap_high:{cell}:{pa['bracket_gap_pct']}")
        wl = pa["waste_and_locality_pct"]
        if wl["ib_b_gather_layers_gt1"] != 0:
            errors.append(f"gather_layers_gt1_nonzero:{cell}:{wl['ib_b_gather_layers_gt1']}")
        if wl["p1_sliver_pct"] < 75.0:
            errors.append(f"p1_sliver_pct_low:{cell}:{wl['p1_sliver_pct']}")
        if wl["ib_b_none_layer_pct"] < 70.0:
            errors.append(f"ib_b_none_layer_pct_low:{cell}:{wl['ib_b_none_layer_pct']}")

    # 5. Replay fidelity, exactness invariants, and falsification boundaries
    for model, rep, want_layers in (("benchy", rep_benchy, 240), ("base", rep_base, 496)):
        if rep.get("layers") != want_layers:
            errors.append(f"replay_layer_count_mismatch:{model}:{rep.get('layers')}")
        if rep.get("threads") != 12 or rep.get("rounds") != 3:
            errors.append(f"replay_config_mismatch:{model}:t={rep.get('threads')},r={rep.get('rounds')}")
        rs = recomputed["replay_summary"][model]
        if abs(rs["m0_vs_probe_pct"]) > 5.0:
            errors.append(f"replay_m0_probe_drift:{model}:{rs['m0_vs_probe_pct']}%")
        mechs = rs["mechanisms"]
        for ename in EXACT_MECHANISMS:
            m = mechs.get(ename)
            if m is None or not m["bit_identical"] or m["exactness"] != "EXACT_BIT_IDENTICAL":
                errors.append(f"exact_mech_not_bit_identical:{model}:{ename}")
        for nname in NON_EXACT_MECHANISMS:
            m = mechs.get(nname)
            if m is None or m["bit_identical"] is not False:
                errors.append(f"non_exact_mech_marked_exact:{model}:{nname}")

    eb_b = recomputed["replay_summary"]["benchy"]["mechanisms"]["EXACT_BUNDLE"]
    eb_ba = recomputed["replay_summary"]["base"]["mechanisms"]["EXACT_BUNDLE"]
    if eb_b["save_pct_vs_m0"] < 50.0:
        errors.append(f"exact_bundle_benchy_saving_low:{eb_b['save_pct_vs_m0']}")
    if eb_ba["save_pct_vs_m0"] < 40.0:
        errors.append(f"exact_bundle_base_saving_low:{eb_ba['save_pct_vs_m0']}")

    # 6. Standing all-8-cell 0.80 % gate & graduation boundary verification
    eb_proj = recomputed["mechanism_8cell_projections_estimate_only"]["EXACT_BUNDLE"]
    if not eb_proj["clears_all_8_cells"] or eb_proj["cells_clearing_080_floor"] != 8:
        errors.append(f"exact_bundle_8cell_floor_short:{eb_proj['cells_clearing_080_floor']}/8")
    if not eb_proj["bit_identical_both_models"]:
        errors.append("exact_bundle_not_bit_identical_both_models")
    if eb_proj["min_floor_multiple_080"] < 6.0:
        errors.append(f"exact_bundle_min_floor_mult_low:{eb_proj['min_floor_multiple_080']}")
    for cell in ("benchy-classic-off", "benchy-arachne-off"):
        c = eb_proj["cells"][cell]
        if not c["crosses_1x_ord"] or not c["crosses_1x_acc"]:
            errors.append(f"exact_bundle_fails_1x_cross:{cell}:ord={c['est_ratio_ord_after']},acc={c['est_ratio_acc_after']}")

    ml2 = recomputed["mechanism_8cell_projections_estimate_only"]["MARGINAL_L2_OVER_EXACT"]
    if ml2["clears_all_8_cells"] or ml2["cells_clearing_080_floor"] >= 8:
        errors.append(f"marginal_l2_unexpectedly_clears_8:{ml2['cells_clearing_080_floor']}")

    # 7. probe.patch integrity check (strictly the 3 expected probe files + env hooks)
    if len(probe_patch_text.strip()) < 1000:
        errors.append("probe_patch_too_small")
    touched = set(re.findall(r"^diff --git a/(.+?) b/(.+?)$", probe_patch_text, flags=re.MULTILINE))
    touched_a = {a for a, _ in touched}
    if touched_a != EXPECTED_PATCH_FILES:
        errors.append(f"probe_patch_unexpected_files:{sorted(touched_a)}")
    if "PNP_T180_PROBE_OUT" not in probe_patch_text or "PNP_T180_DUMP_IO" not in probe_patch_text:
        errors.append("probe_patch_missing_env_hooks")

    return errors


def load_committed() -> dict:
    return {
        "arith_disk": json.loads((REDUCED / "arithmetic.json").read_text(encoding="utf-8")),
        "probe": json.loads((REDUCED / "probe.json").read_text(encoding="utf-8")),
        "rep_benchy": json.loads((REDUCED / "replay_benchy.json").read_text(encoding="utf-8")),
        "rep_base": json.loads((REDUCED / "replay_base.json").read_text(encoding="utf-8")),
        "t175_ref": json.loads(T175_REF.read_text(encoding="utf-8")),
        "t177_arith": json.loads(T177_ARITH.read_text(encoding="utf-8")),
        "t178_arith": json.loads(T178_ARITH.read_text(encoding="utf-8")),
        "t179_arith": json.loads(T179_ARITH.read_text(encoding="utf-8")),
        "probe_patch_text": PROBE_PATCH.read_text(encoding="utf-8"),
    }


def run_self_test() -> int:
    base = load_committed()
    base_errors = verify_all(**base)
    if base_errors:
        print(f"SELF-TEST ABORT: baseline committed data failed: {base_errors}")
        return 1

    def mut(fn):
        d = copy.deepcopy(base)
        fn(d)
        return d

    def recompute_mut(fn):
        d = copy.deepcopy(base)
        fn(d)
        d["arith_disk"] = arithmetic.compute_arithmetic(
            t175_ref=d["t175_ref"],
            t177_arith=d["t177_arith"],
            t178_arith=d["t178_arith"],
            t179_arith=d["t179_arith"],
            probe=d["probe"],
            rep_benchy=d["rep_benchy"],
            rep_base=d["rep_base"],
        )
        return d

    def set_mech(rep: dict, name: str, key: str, val) -> None:
        for m in rep["mechanisms"]:
            if m["name"] == name:
                m[key] = val
                return
        raise KeyError(name)

    cases: list[tuple[str, str, dict]] = [
        (
            "1_arith_tamper",
            "arithmetic_recompute_mismatch",
            mut(lambda d: d["arith_disk"]["qualification"].__setitem__("total_runs", 14)),
        ),
        (
            "2_ref_sha_mismatch",
            "probe_ref_check_failed",
            recompute_mut(lambda d: d["probe"]["runs"][0].__setitem__("gcode_sha256", "0" * 64)),
        ),
        (
            "3_qual_degraded",
            "probe_qual_unclean",
            recompute_mut(lambda d: d["probe"]["runs"][3]["instrument_events"].__setitem__("degraded", True)),
        ),
        (
            "4_qual_code_1200",
            "probe_qual_unclean",
            recompute_mut(lambda d: d["probe"]["runs"][11]["instrument_events"].__setitem__("code_1200_count", 1)),
        ),
        (
            "5_invariance_broken",
            "invariance_failed",
            recompute_mut(lambda d: d["probe"]["invariance"]["benchy_classic_vs_arachne"].__setitem__("identical_calls_and_verts", False)),
        ),
        (
            "6_call_reconcile_mismatch",
            "call_reconciliation_mismatch",
            recompute_mut(lambda d: d["probe"]["cells"]["base-classic-off"]["median_pass"]["op_kinds"]["clip"].__setitem__("calls", 5021)),
        ),
        (
            "7_bracket_gap_high",
            "bracket_gap_high",
            recompute_mut(lambda d: d["probe"]["cells"]["benchy-classic-off"]["median_pass"].__setitem__("bracket_gap_ms", 5.0)),
        ),
        (
            "8_gather_gt1_nonzero",
            "gather_layers_gt1_nonzero",
            recompute_mut(
                lambda d: d["probe"]["cells"]["base-classic-off"]["median_pass"]["locality"]["ib_phase_b_precomp"]["gather_layers"].__setitem__("layers_gt1", 1)
            ),
        ),
        (
            "9_replay_layer_mismatch",
            "replay_layer_count_mismatch",
            recompute_mut(lambda d: d["rep_benchy"].__setitem__("layers", 239)),
        ),
        (
            "10_replay_m0_drift",
            "replay_m0_probe_drift",
            recompute_mut(lambda d: set_mech(d["rep_benchy"], "M0_baseline", "median_ms", 600.0)),
        ),
        (
            "11_exact_bundle_diff",
            "exact_mech_not_bit_identical",
            recompute_mut(lambda d: set_mech(d["rep_base"], "EXACT_BUNDLE", "bit_identical", False)),
        ),
        (
            "12_non_exact_marked_exact",
            "non_exact_mech_marked_exact",
            recompute_mut(lambda d: set_mech(d["rep_benchy"], "L2_p2_per_poly_neg", "bit_identical", True)),
        ),
        (
            "13_exact_bundle_low_save",
            "exact_bundle_benchy_saving_low",
            recompute_mut(lambda d: set_mech(d["rep_benchy"], "EXACT_BUNDLE", "median_ms", 350.0)),
        ),
        (
            "14_probe_patch_unexpected_file",
            "probe_patch_unexpected_files",
            mut(lambda d: d.__setitem__("probe_patch_text", d["probe_patch_text"] + "\ndiff --git a/crates/foo.rs b/crates/foo.rs\n")),
        ),
        (
            "15_probe_patch_missing_hook",
            "probe_patch_missing_env_hooks",
            mut(lambda d: d.__setitem__("probe_patch_text", d["probe_patch_text"].replace("PNP_T180_DUMP_IO", "REDACTED"))),
        ),
    ]

    passed = 0
    for label, want_prefix, mutated in cases:
        errs = verify_all(**mutated)
        if any(e.startswith(want_prefix) for e in errs):
            passed += 1
        else:
            print(f"SELF-TEST FAIL [{label}]: expected prefix '{want_prefix}', got {errs}")
            return 1

    print(f"SELF-TEST PASS: {passed}/{len(cases)} negative controls rejected with exact error prefixes")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="Verify Wayfinder Ticket 180 committed evidence.")
    ap.add_argument("--self-test", action="store_true", help="Run negative-control self-tests.")
    args = ap.parse_args()
    if args.self_test:
        return run_self_test()
    errs = verify_all(**load_committed())
    if errs:
        print("VERIFY FAIL:")
        for e in errs:
            print(f"  - {e}")
        return 1
    print("VERIFY PASS: 7/7 check groups green (15/15 ref-identical runs, 4/4 invariance, 100% t177 call recon, 8/8 EXACT_BUNDLE gate)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

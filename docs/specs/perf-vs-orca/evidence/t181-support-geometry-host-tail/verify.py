#!/usr/bin/env python3
"""Checkout-only verifier and negative-control self-test for Wayfinder Ticket 181.

Reads strictly from committed in-repo files (Python 3 stdlib only, zero target/,
zero STL fixtures).

Usage:
  python verify.py
  python verify.py --self-test
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
T178_ARITH = HERE.parent / "t178-planner-phase3-union" / "reduced" / "arithmetic.json"
T179_ARITH = HERE.parent / "t179-per-layer-guest-screen" / "reduced" / "arithmetic.json"
T180_ARITH = HERE.parent / "t180-shell-classification-residual" / "reduced" / "arithmetic.json"

sys.path.insert(0, str(HERE))
import arithmetic  # noqa: E402


class VerifyError(RuntimeError):
    """Tagged verification error with stable prefix for --self-test."""


def fail(prefix: str, detail: str) -> None:
    raise VerifyError(f"[{prefix}] {detail}")


def verify_all(
    *,
    t175_ref: dict,
    t177_arith: dict,
    t178_arith: dict,
    t179_arith: dict,
    t180_arith: dict,
    probe: dict,
    rep_benchy: dict,
    rep_base: dict,
    committed_arith: dict,
    check_files: bool = True,
) -> dict:
    # 0. Required committed files check
    if check_files:
        for rel in (
            "FINDINGS.md",
            "REPRODUCE.md",
            "probe.patch",
            "run_probe.py",
            "reduce_probe.py",
            "arithmetic.py",
            "verify.py",
            "reduced/probe.json",
            "reduced/replay_benchy.json",
            "reduced/replay_base.json",
            "reduced/arithmetic.json",
        ):
            p = HERE / rel
            if not p.is_file() or p.stat().st_size == 0:
                fail("MISSING_FILE", f"Required artifact missing or empty: {rel}")

    # 1. Exact leaf-for-leaf recomputation of arithmetic.json
    recomputed = arithmetic.compute_arithmetic(
        t175_ref=t175_ref,
        t177_arith=t177_arith,
        t178_arith=t178_arith,
        t179_arith=t179_arith,
        t180_arith=t180_arith,
        probe=probe,
        rep_benchy=rep_benchy,
        rep_base=rep_base,
    )
    if recomputed != committed_arith:
        fail("RECOMPUTE_MISMATCH", "Committed reduced/arithmetic.json differs from recomputed output")

    # 2. Qualification, reference identity, and supports-off negative controls
    q = recomputed["qualification"]
    if q["total_runs"] != 22:
        fail("RUN_COUNT", f"Expected 22 probe runs, got {q['total_runs']}")
    if q["exit_zero_runs"] != q["total_runs"]:
        fail("EXIT_CODE", f"Only {q['exit_zero_runs']}/{q['total_runs']} runs exited 0")
    if q["reference_identical_runs"] != q["total_runs"]:
        fail(
            "REF_MISMATCH",
            f"Only {q['reference_identical_runs']}/{q['total_runs']} runs matched t175 reference SHA+bytes",
        )
    if q["qual_runs_total"] != 6 or q["qual_runs_clean"] != 6:
        fail(
            "QUAL_FAILED",
            f"Qualification failed: {q['qual_runs_clean']}/{q['qual_runs_total']} clean",
        )
    if not q["supports_off_controls_inert"]:
        fail("SUPPORTS_OFF_NOT_INERT", "Supports-off negative control was not inert")

    # 3. Classic vs Arachne counter invariance + PPO 100% polygon-for-polygon equality
    if not q["invariance_all_identical"]:
        fail("INVARIANCE_BROKEN", "Classic vs Arachne counters diverged")
    if not q["ppo_all_exact"]:
        fail("PPO_MISMATCH", f"PPO verification not 100% exact: {q['ppo_verify']}")
    if q["ppo_verify"]["benchy-classic-on"]["total_planes"] != 200:
        fail("PPO_COUNT", "Expected 200 benchy planes in PPO verify")
    if q["ppo_verify"]["base-classic-on"]["total_planes"] != 432:
        fail("PPO_COUNT", "Expected 432 base planes in PPO verify")

    # 4. t177 & bracket additive reconciliation
    for cell, rec in recomputed["t177_reconciliation"].items():
        if rec["qual_delta_ms"] > 1.0:
            fail("QUAL_BRACKET_DRIFT", f"{cell}: qual probe vs instrument delta {rec['qual_delta_ms']} ms > 1.0 ms")
        if rec["max_top_level_residual_ms"] > 0.1:
            fail("TOP_RESIDUAL", f"{cell}: top-level bracket residual {rec['max_top_level_residual_ms']} ms > 0.1 ms")
        if rec["max_t3_internal_residual_ms"] > 0.1:
            fail("T3_RESIDUAL", f"{cell}: T3 internal bracket residual {rec['max_t3_internal_residual_ms']} ms > 0.1 ms")

    for model_name in ("benchy", "base"):
        rs = recomputed["replay_summary"][model_name]
        ratio = rs["m0_vs_live_probe_ratio"]
        if not (0.95 <= ratio <= 1.05):
            fail("REPLAY_M0_DRIFT", f"{model_name}: M0 replay vs live probe ratio {ratio} outside [0.95, 1.05]")

    # 5. Replay bit-for-bit exactness & speedup checks
    for model_name in ("benchy", "base"):
        rs = recomputed["replay_summary"][model_name]
        for mname, m in rs["modes"].items():
            if not m["exact_bit_identical"]:
                fail("REPLAY_NOT_EXACT", f"{model_name} {mname} failed bit-for-bit exactness")
        m_full = rs["modes"]["M12345_full_exact_bundle"]
        if m_full["counters"]["clipper_calls_total"] != 0:
            fail("UNEXPECTED_CLIPPER_FALLBACK", f"{model_name} M12345 had nonzero Clipper calls")
        if m_full["save_wall_pct"] < 99.0:
            fail("SPEEDUP_BELOW_BAR", f"{model_name} M12345 save_wall_pct {m_full['save_wall_pct']} < 99.0%")

    # 6. Exhaustive zero-tolerance adversarial falsification proof
    adv = recomputed["adversarial_totals"]
    if adv["real_pairs_checked"] != 8186 or adv["real_pairs_exact_match"] != 8186:
        fail("ADV_REAL_MISMATCH", f"Real pairs exact match {adv['real_pairs_exact_match']}/{adv['real_pairs_checked']}")
    if adv["synthetic_cases_checked"] < 2700:
        fail("ADV_SYNTH_COUNT", f"Too few synthetic cases: {adv['synthetic_cases_checked']}")
    if adv["total_false_negatives"] != 0:
        fail("ADV_FALSE_NEGATIVE", f"Nonzero false negatives: {adv['total_false_negatives']}")
    if adv["total_false_positives"] != 0:
        fail("ADV_FALSE_POSITIVE", f"Nonzero false positives: {adv['total_false_positives']}")

    # 7. Standing t137 4-cell planner gate clearance
    for mname in ("M1234_ppo_bbox_zeroclone_cert", "M12345_full_exact_bundle"):
        gp = recomputed["t137_4cell_gate_projections_estimate_only"][mname]
        if not (gp["clears_all_4_t177_scale"] and gp["clears_all_4_probe_scale"]):
            fail("GATE_FAIL", f"{mname} failed 4-cell standing gate clearance")
        if gp["min_floor_multiple_t177_scale"] < 7.4:
            fail("GATE_FLOOR", f"{mname} min floor multiple {gp['min_floor_multiple_t177_scale']} < 7.4")

    return {
        "checks_passed": 7,
        "runs": q["total_runs"],
        "qual_clean": q["qual_runs_clean"],
        "ppo_planes": 632,
        "adversarial_total": adv["total_pairs_verified"],
    }


def load_committed() -> dict:
    return {
        "t175_ref": json.loads(T175_REF.read_text(encoding="utf-8")),
        "t177_arith": json.loads(T177_ARITH.read_text(encoding="utf-8")),
        "t178_arith": json.loads(T178_ARITH.read_text(encoding="utf-8")),
        "t179_arith": json.loads(T179_ARITH.read_text(encoding="utf-8")),
        "t180_arith": json.loads(T180_ARITH.read_text(encoding="utf-8")),
        "probe": json.loads((REDUCED / "probe.json").read_text(encoding="utf-8")),
        "rep_benchy": json.loads((REDUCED / "replay_benchy.json").read_text(encoding="utf-8")),
        "rep_base": json.loads((REDUCED / "replay_base.json").read_text(encoding="utf-8")),
        "committed_arith": json.loads((REDUCED / "arithmetic.json").read_text(encoding="utf-8")),
    }


def run_self_test() -> int:
    base = load_committed()

    def make_case(mutator, recompute: bool = True):
        data = copy.deepcopy(base)
        mutator(data)
        if recompute:
            data["committed_arith"] = arithmetic.compute_arithmetic(
                t175_ref=data["t175_ref"],
                t177_arith=data["t177_arith"],
                t178_arith=data["t178_arith"],
                t179_arith=data["t179_arith"],
                t180_arith=data["t180_arith"],
                probe=data["probe"],
                rep_benchy=data["rep_benchy"],
                rep_base=data["rep_base"],
            )
        return data

    controls = [
        (
            "tampered_arithmetic_leaf",
            "RECOMPUTE_MISMATCH",
            lambda d: d["committed_arith"]["adversarial_totals"].__setitem__("total_pairs_verified", 999),
            False,
        ),
        (
            "tampered_gcode_sha256",
            "REF_MISMATCH",
            lambda d: d["probe"]["runs"][0].__setitem__("sha256", "0" * 64),
            True,
        ),
        (
            "tampered_gcode_bytes",
            "REF_MISMATCH",
            lambda d: d["probe"]["runs"][0].__setitem__("bytes", 12345),
            True,
        ),
        (
            "nonzero_exit_code",
            "EXIT_CODE",
            lambda d: d["probe"]["runs"][0].__setitem__("exit_code", 1),
            True,
        ),
        (
            "qual_degraded_true",
            "QUAL_FAILED",
            lambda d: next(r for r in d["probe"]["runs"] if r["instrumented"])["qual"].__setitem__(
                "degraded", True
            ),
            True,
        ),
        (
            "qual_code_1200_nonzero",
            "QUAL_FAILED",
            lambda d: next(r for r in d["probe"]["runs"] if r["instrumented"])["qual"].__setitem__(
                "code_1200_count", 1
            ),
            True,
        ),
        (
            "supports_off_not_inert",
            "SUPPORTS_OFF_NOT_INERT",
            lambda d: d["probe"]["supports_off_controls"]["benchy-classic-off"].__setitem__(
                "active_entries", 1
            ),
            True,
        ),
        (
            "classic_arachne_counter_drift",
            "INVARIANCE_BROKEN",
            lambda d: d["probe"]["invariance"]["base_classic_vs_arachne"].__setitem__(
                "identical_counters", False
            ),
            True,
        ),
        (
            "ppo_missing_plane",
            "PPO_MISMATCH",
            lambda d: d["probe"]["ppo_verify"]["base-classic-on"].__setitem__("missing_from_ppo", 1),
            True,
        ),
        (
            "ppo_polygon_mismatch",
            "PPO_MISMATCH",
            lambda d: d["probe"]["ppo_verify"]["benchy-classic-on"].__setitem__("exact_poly_equal", 199),
            True,
        ),
        (
            "bracket_residual_exceeded",
            "TOP_RESIDUAL",
            lambda d: d["probe"]["supports_on_cells"]["base-classic-on"]["reconciliation"].__setitem__(
                "max_top_level_residual_ms", 2.5
            ),
            True,
        ),
        (
            "replay_mode_not_bit_identical",
            "REPLAY_NOT_EXACT",
            lambda d: d["rep_base"]["modes"][-1].__setitem__("exact_bit_identical", False),
            True,
        ),
        (
            "adversarial_false_negative",
            "ADV_FALSE_NEGATIVE",
            lambda d: d["rep_benchy"]["adversarial"].__setitem__("false_negatives", 1),
            True,
        ),
        (
            "adversarial_false_positive",
            "ADV_FALSE_POSITIVE",
            lambda d: d["rep_base"]["adversarial"].__setitem__("false_positives", 1),
            True,
        ),
    ]

    passed = 0
    for name, expected_prefix, mutator, recomp in controls:
        case_data = make_case(mutator, recompute=recomp)
        try:
            verify_all(**case_data, check_files=False)
        except VerifyError as exc:
            msg = str(exc)
            if not msg.startswith(f"[{expected_prefix}]"):
                print(f"SELF-TEST FAIL ({name}): expected [{expected_prefix}], got {msg}")
                return 1
            passed += 1
        else:
            print(f"SELF-TEST FAIL ({name}): unexpectedly passed without [{expected_prefix}]")
            return 1

    print(
        f"SELF-TEST PASS: {passed}/{len(controls)} negative controls rejected with exact error prefixes"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()

    data = load_committed()
    res = verify_all(**data, check_files=True)
    print(
        f"VERIFY PASS: {res['checks_passed']}/7 check groups green "
        f"({res['runs']}/22 ref-identical runs, {res['qual_clean']}/6 qual clean, "
        f"{res['ppo_planes']}/632 PPO exact, {res['adversarial_total']}/{res['adversarial_total']} "
        f"adversarial 0 FN / 0 FP)"
    )
    if args.self_test:
        return run_self_test()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

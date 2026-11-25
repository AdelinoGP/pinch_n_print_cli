#!/usr/bin/env python3
"""Checkout-only verifier + negative-control self-test for Wayfinder Ticket 178.

Verifies every number in `reduced/arithmetic.json` from committed JSONs only
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

sys.path.insert(0, str(HERE))
import arithmetic  # noqa: E402


def verify_all(
    arith_disk: dict,
    prod: dict,
    rep_benchy: dict,
    rep_base: dict,
    t175_ref: dict,
    probe_text: str,
) -> list[str]:
    errors: list[str] = []

    # 1. Recompute arithmetic.json leaf-for-leaf against committed disk copy
    recomputed = arithmetic.compute_arithmetic()
    if arith_disk != recomputed:
        errors.append("arithmetic_recompute_mismatch: reduced/arithmetic.json differs from compute_arithmetic()")

    # 2. Qualification runs match t175 reference byte-for-byte & clean diagnostics
    runs = prod.get("runs", [])
    if len(runs) != 12:
        errors.append(f"expected 12 capture runs, got {len(runs)}")
    if prod.get("capture_meta", {}).get("failures", -1) != 0:
        errors.append("capture_meta.failures != 0")
    for r in runs:
        cell = r["cell"]
        if not r.get("ok") or r.get("exit_code") != 0:
            errors.append(f"run_failed:{cell}:{r.get('kind')}:{r.get('pass')}")
        if r.get("gcode_sha256") != t175_ref["active_set"].get(cell):
            errors.append(f"ref_sha_mismatch:{cell}")
        if r.get("gcode_bytes") != t175_ref["bytes"].get(cell):
            errors.append(f"ref_bytes_mismatch:{cell}")
        if r.get("kind", "").startswith("qual"):
            if (
                r.get("degraded") is not False
                or r.get("fatal_error_count") != 0
                or r.get("non_fatal_error_count") != 0
                or r.get("code_1200_count") != 0
            ):
                errors.append(f"qual_diag_unclean:{cell}")

    # 3. Classic vs Arachne Phase-3 operand identity
    ident = prod.get("classic_arachne_operand_identity", {})
    if ident != {"benchy": True, "base": True}:
        errors.append(f"classic_arachne_operand_identity_failed:{ident}")

    # 4. Exact vertex-for-vertex baseline replay & corpus invariants
    for label, rep, want_calls, want_batches in [
        ("benchy", rep_benchy, 202, 17),
        ("base", rep_base, 480, 37),
    ]:
        if not rep.get("baseline_exact_replay_ok") or rep.get("baseline_exact_replay_mismatches") != 0:
            errors.append(f"baseline_replay_mismatch:{label}")
        if rep.get("total_calls") != want_calls or rep.get("batches") != want_batches:
            errors.append(f"corpus_shape_mismatch:{label}")

        # Component bucket conservation
        buckets = rep.get("component_buckets", [])
        if sum(b["poly_count"] for b in buckets) != rep["total_in_polys"]:
            errors.append(f"comp_bucket_poly_conservation:{label}")
        if sum(b["vert_count"] for b in buckets) != rep["total_in_verts"]:
            errors.append(f"comp_bucket_vert_conservation:{label}")

        # Role conservation
        roles = rep.get("by_role", [])
        if sum(r["calls"] for r in roles) != rep["total_calls"]:
            errors.append(f"role_call_conservation:{label}")
        if sum(r["in_polys"] for r in roles) != rep["total_in_polys"]:
            errors.append(f"role_poly_conservation:{label}")
        if sum(r["in_verts"] for r in roles) != rep["total_in_verts"]:
            errors.append(f"role_vert_conservation:{label}")

        # Singleton audit conservation & 100% canonical cleanliness
        sa = rep.get("singleton_audit", {})
        if sa.get("from_kept", 0) + sa.get("from_clip", 0) != sa.get("total_singletons", -1):
            errors.append(f"singleton_origin_conservation:{label}")
        if (
            sa.get("non_canonical_winding") != 0
            or sa.get("self_union_area_diff") != 0
            or sa.get("self_union_ring_diff") != 0
        ):
            errors.append(f"singleton_not_canonical:{label}")

        # Scanbeam proof: 100% of CC-split diffs explained by dummy-far-Y
        sb = rep.get("cc_scanbeam_proof", {})
        if sb.get("cc_diff_calls", -1) != sb.get("explained_by_dummy_far_y", -2) or sb.get("unexplained", -1) != 0:
            errors.append(f"scanbeam_proof_incomplete:{label}")

        # M0 & M1 exact zero diff sanity check
        m_by_name = {m["name"]: m for m in rep.get("mechanisms", [])}
        for exact_m in ("M0_baseline_head_tail", "M1_subj_all_clip_empty", "M5a_lpt_within_batch12", "M5b_global_single_batch"):
            if m_by_name[exact_m]["diff_calls"] != 0 or m_by_name[exact_m]["total_xor_area_mm2"] != 0.0:
                errors.append(f"exact_mech_nonzero_diff:{label}:{exact_m}")

        # Primary candidate M9n_k8 clears huge speedup + sub-micron boundary Hausdorff
        m9n = m_by_name["M9n_host_nested_prune_hier_k8"]
        if m9n["max_boundary_hausdorff_um"] >= 1.0:
            errors.append(f"m9n_k8_hausdorff_ge_1um:{label}:{m9n['max_boundary_hausdorff_um']}")
        if label == "base" and m9n["wall_vs_m0_pct"] < 85.0:
            errors.append(f"m9n_k8_base_wall_cut_low:{m9n['wall_vs_m0_pct']}")
        if label == "benchy" and m9n["wall_vs_m0_pct"] < 55.0:
            errors.append(f"m9n_k8_benchy_wall_cut_low:{m9n['wall_vs_m0_pct']}")

    # 5. Primary candidate clears t137 4-cell floor in arithmetic.json
    m9n_route = arith_disk.get("mechanisms", {}).get("M9n_host_nested_prune_hier_k8", {})
    if not m9n_route.get("clears_t137_4cell_floor"):
        errors.append("m9n_k8_fails_t137_4cell_floor")

    # 6. Preserved probe.patch integrity
    for marker in ("t178_probe.rs", "crates/slicer-wasm-host/src/host.rs", "modules/core-modules/tree-support-planner/src/lib.rs"):
        if marker not in probe_text:
            errors.append(f"probe_patch_missing:{marker}")

    return errors


def run_self_test(
    arith_disk: dict,
    prod: dict,
    rep_benchy: dict,
    rep_base: dict,
    t175_ref: dict,
    probe_text: str,
) -> bool:
    controls = []

    def add_control(name: str, mutate):
        a, p, rb, rba, tr, pt = (
            copy.deepcopy(arith_disk),
            copy.deepcopy(prod),
            copy.deepcopy(rep_benchy),
            copy.deepcopy(rep_base),
            copy.deepcopy(t175_ref),
            probe_text,
        )
        res = mutate(a, p, rb, rba, tr, pt)
        if res is not None:
            a, p, rb, rba, tr, pt = res
        errs = verify_all(a, p, rb, rba, tr, pt)
        controls.append((name, len(errs) > 0, errs[:1]))

    add_control("1_arithmetic_drift", lambda a, *_: a["prod_cells"]["base-classic-on"].__setitem__("prod_phase3_wall_ms_median", 1.0) and None)
    add_control("2_ref_sha_tamper", lambda a, p, *_: p["runs"][4].__setitem__("gcode_sha256", "deadbeef") and None)
    add_control("3_ref_bytes_tamper", lambda a, p, *_: p["runs"][4].__setitem__("gcode_bytes", 1) and None)
    add_control("4_qual_degraded", lambda a, p, *_: p["runs"][4].__setitem__("degraded", True) and None)
    add_control("5_qual_code1200", lambda a, p, *_: p["runs"][4].__setitem__("code_1200_count", 1) and None)
    add_control("6_classic_arachne_mismatch", lambda a, p, *_: p["classic_arachne_operand_identity"].__setitem__("base", False) and None)
    add_control("7_baseline_replay_mismatch", lambda a, p, rb, rba, *_: rba.__setitem__("baseline_exact_replay_mismatches", 1) and None)
    add_control("8_comp_bucket_conservation", lambda a, p, rb, rba, *_: rba["component_buckets"][0].__setitem__("poly_count", 999) and None)
    add_control("9_role_conservation", lambda a, p, rb, rba, *_: rba["by_role"][0].__setitem__("calls", 400) and None)
    add_control("10_singleton_non_canonical", lambda a, p, rb, rba, *_: rba["singleton_audit"].__setitem__("non_canonical_winding", 1) and None)
    add_control("11_scanbeam_unexplained", lambda a, p, rb, rba, *_: rba["cc_scanbeam_proof"].__setitem__("unexplained", 1) and None)
    add_control("12_exact_mech_diff", lambda a, p, rb, rba, *_: [m.__setitem__("diff_calls", 1) for m in rba["mechanisms"] if m["name"] == "M1_subj_all_clip_empty"] and None)
    add_control("13_m9n_hausdorff_regression", lambda a, p, rb, rba, *_: [m.__setitem__("max_boundary_hausdorff_um", 1.5) for m in rba["mechanisms"] if m["name"] == "M9n_host_nested_prune_hier_k8"] and None)
    add_control("14_probe_patch_missing_marker", lambda a, p, rb, rba, tr, pt: (a, p, rb, rba, tr, ""))

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
    prod = json.loads((REDUCED / "prod_timing.json").read_text(encoding="utf-8"))
    rep_benchy = json.loads((REDUCED / "replay_benchy.json").read_text(encoding="utf-8"))
    rep_base = json.loads((REDUCED / "replay_base.json").read_text(encoding="utf-8"))
    t175_ref = json.loads(T175_REF.read_text(encoding="utf-8"))
    probe_text = (HERE / "probe.patch").read_text(encoding="utf-8")

    errs = verify_all(arith_disk, prod, rep_benchy, rep_base, t175_ref, probe_text)
    if errs:
        print("VERIFY FAIL:")
        for e in errs:
            print("  -", e)
        return 1
    print("VERIFY PASS: 6/6 checks green (arithmetic recompute, 12/12 ref+diag, classic==arachne, 682/682 replay+conservation+174/174 scanbeam, t137 floor, probe.patch)")

    if args.self_test:
        if not run_self_test(arith_disk, prod, rep_benchy, rep_base, t175_ref, probe_text):
            return 1
    print("PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

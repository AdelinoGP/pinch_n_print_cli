#!/usr/bin/env python3
"""Checkout-only verifier (and negative-control self-test) for t188.

Runs strictly against committed files under docs/specs/perf-vs-orca/evidence/
(no target/, no binary, no STL fixtures required).
"""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

EVIDENCE_DIR = Path(__file__).resolve().parent
REDUCED_DIR = EVIDENCE_DIR / "reduced"
T182_REF = EVIDENCE_DIR.parent / "t182-spatial-hierarchical-phase3-union" / "reference.json"
T185_ROUTE = EVIDENCE_DIR.parent / "t185-support-geometry-host-tail-accel" / "reduced" / "route.json"
T187_ARITH = (
    EVIDENCE_DIR.parent / "t187-tree-support-planner-residual" / "reduced" / "arithmetic.json"
)

sys.path.insert(0, str(EVIDENCE_DIR))
import arithmetic as arith_mod  # noqa: E402

ON_CELLS = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]
OFF_CELLS = ["benchy-classic-off", "base-classic-off"]


def verify_bundle(
    probe_wall: dict,
    screen: dict,
    committed_arith: dict,
    t182_ref: dict,
    t185_route: dict,
    t187_arith: dict,
) -> list[str]:
    errors: list[str] = []
    refs = t182_ref["active_set"]
    ref_bytes = t182_ref["bytes"]

    # 1. Probe run counts, reference hash/byte parity, and qual gates
    if probe_wall.get("total_runs") != 16:
        errors.append(f"probe total_runs != 16: {probe_wall.get('total_runs')}")
    for cell in ON_CELLS:
        c = probe_wall["cells"].get(cell)
        if not c:
            errors.append(f"probe missing cell {cell}")
            continue
        passes = c.get("passes", [])
        if len(passes) != 3:
            errors.append(f"probe {cell} expected 3 passes, got {len(passes)}")
        for p in passes:
            if p["exit_code"] != 0:
                errors.append(f"probe {cell} r{p['round']} nonzero exit_code")
            if p["gcode_sha256"] != refs[cell] or p["gcode_bytes"] != ref_bytes[cell]:
                errors.append(f"probe {cell} r{p['round']} hash/bytes mismatch vs t182/reference.json")
            if not p["reference_match"]:
                errors.append(f"probe {cell} r{p['round']} reference_match is False")
            inst = p["inst"]
            if (
                inst.get("degraded") is not False
                or inst.get("fatal_error_count") != 0
                or inst.get("non_fatal_error_count") != 0
                or inst.get("code_1200_count") != 0
            ):
                errors.append(f"probe {cell} r{p['round']} qual gate failed: {inst}")
        pp = c["profile_pass"]
        if (
            pp["exit_code"] != 0
            or pp["gcode_sha256"] != refs[cell]
            or pp["gcode_bytes"] != ref_bytes[cell]
            or not pp["reference_match"]
        ):
            errors.append(f"probe {cell} profile_pass hash/exit mismatch")

    # 2. Geometry census zero-error checks
    for cell in ("benchy-classic-on", "base-classic-on"):
        gc = probe_wall.get("geometry_census", {}).get(cell)
        if not gc:
            errors.append(f"geometry_census missing {cell}")
            continue
        if gc["cantilever_cert_errors"] != 0:
            errors.append(f"geometry_census {cell} cantilever_cert_errors != 0")
        if gc["grown_cert_errors"] != 0:
            errors.append(f"geometry_census {cell} grown_cert_errors != 0")
        if gc["raw_empty_trimmed_nonempty_errors"] != 0:
            errors.append(f"geometry_census {cell} raw_empty_trimmed_nonempty_errors != 0")
        # Falsified grown mechanisms must stay recorded with non-zero drift
        if gc["gholes_diff_layers"] <= 0 or gc["gwin_diff_layers"] <= 0 or gc["gisland_diff_layers"] <= 0:
            errors.append(f"geometry_census {cell} falsified grown-window drift must be > 0")

    # 3. Screen run counts, 100% reference parity, and gate clearance
    if screen.get("measured_runs") != 50:
        errors.append(f"screen measured_runs != 50: {screen.get('measured_runs')}")
    rejected_ids = {r["id"] for r in screen.get("rejected_mechanisms", [])}
    for req_rej in ("G_holes", "G_win", "G_island_win", "G_cert"):
        if req_rej not in rejected_ids:
            errors.append(f"screen missing rejected mechanism disclosure: {req_rej}")

    for cell in ON_CELLS:
        sc = screen["supports_on"].get(cell)
        if not sc:
            errors.append(f"screen missing cell {cell}")
            continue
        pairs = sc.get("ab_pairs", [])
        if len(pairs) != 3:
            errors.append(f"screen {cell} expected 3 A/B pairs, got {len(pairs)}")
        for p in pairs:
            if (
                p["baseline_sha256"] != refs[cell]
                or p["candidate_sha256"] != refs[cell]
                or p["baseline_bytes"] != ref_bytes[cell]
                or p["candidate_bytes"] != ref_bytes[cell]
                or not p["reference_match"]
            ):
                errors.append(f"screen {cell} ab r{p['round']} reference mismatch")
        for arm in ("baseline", "candidate"):
            ir = sc["inst"][arm]
            if (
                ir["gcode_sha256"] != refs[cell]
                or ir["gcode_bytes"] != ref_bytes[cell]
                or not ir["reference_match"]
                or ir.get("degraded") is not False
                or ir.get("fatal_error_count") != 0
                or ir.get("non_fatal_error_count") != 0
                or ir.get("code_1200_count") != 0
                or not isinstance(ir.get("sa_stage_ms"), int)
            ):
                errors.append(f"screen {cell} inst {arm} failed parity/qual")
            pr = sc["profile"][arm]
            if (
                pr["gcode_sha256"] != refs[cell]
                or pr["gcode_bytes"] != ref_bytes[cell]
                or not pr["reference_match"]
            ):
                errors.append(f"screen {cell} profile {arm} failed parity")

    # Supports-off inertness
    for cell in OFF_CELLS:
        r = screen.get("supports_off_inertness", {}).get(cell)
        if not r or r["gcode_sha256"] != refs[cell] or r["gcode_bytes"] != ref_bytes[cell] or not r["reference_match"]:
            errors.append(f"screen supports-off inertness failed for {cell}")

    # Ablation parity + qual
    for cell in ("benchy-classic-on", "base-classic-on"):
        ab = screen.get("ablation", {}).get(cell, {})
        for mode in ("sa1", "sa123", "sa4", "sa1234"):
            m = ab.get(mode)
            if (
                not m
                or m["gcode_sha256"] != refs[cell]
                or m["gcode_bytes"] != ref_bytes[cell]
                or not m["reference_match"]
                or m.get("degraded") is not False
                or m.get("code_1200_count") != 0
            ):
                errors.append(f"screen ablation {cell}/{mode} failed parity/qual")

    # 4. Recompute arithmetic from scratch and verify exact match with committed arithmetic.json
    recomputed = arith_mod.compute_arithmetic(
        probe_wall, screen, t182_ref, t185_route, t187_arith
    )
    if recomputed != committed_arith:
        errors.append("committed reduced/arithmetic.json differs from recomputed arithmetic")

    # 5. Classic == Arachne invariance invariants
    for model in ("benchy", "base"):
        inv = recomputed["classic_arachne_invariance"][model]
        for k in (
            "probe_counts_equal",
            "probe_op_counts_equal",
            "probe_op_vin_equal",
            "probe_op_vout_equal",
            "baseline_host_sa_calls_equal",
            "candidate_host_sa_calls_equal",
        ):
            if not inv[k]:
                errors.append(f"invariance failed: {model}.{k} is False")

    # 6. Screen t137 floor + CPU corroboration + exact host:support_analysis call reduction
    for cell in ON_CELLS:
        ss = recomputed["screen_summary"][cell]
        if not ss["clears_t137_floor"]:
            errors.append(f"screen {cell} failed t137 0.80% floor or CPU corroboration")
        if ss["pair_wall_wins"] < 3:
            errors.append(f"screen {cell} wall pair wins {ss['pair_wall_wins']}/3 < 3")
        if not ss["cpu_corroborating"]:
            errors.append(f"screen {cell} CPU not corroborating ({ss['cpu_saved_pct']}%)")
        if ss["sa_saved_ms"] <= 0:
            errors.append(f"screen {cell} PrePass::SupportAnalysis stage did not improve")
        if ss["host_sa_calls_saved"] <= 0:
            errors.append(f"screen {cell} host:support_analysis call count did not drop")

    # 7. Monotonic ablation ladder check
    for cell in ("benchy-classic-on", "base-classic-on"):
        al = recomputed["ablation_summary"][cell]
        if not (al["baseline_sa_ms"] > al["sa1_sa_ms"] > al["sa123_sa_ms"] > al["sa1234_sa_ms"] > al["full_sa_ms"]):
            errors.append(f"ablation ladder not strictly monotonic on {cell}: {al}")

    return errors


def run_self_test() -> int:
    probe_wall = arith_mod.load_json(REDUCED_DIR / "probe_wall.json")
    screen = arith_mod.load_json(REDUCED_DIR / "screen.json")
    committed_arith = arith_mod.load_json(REDUCED_DIR / "arithmetic.json")
    t182_ref = arith_mod.load_json(T182_REF)
    t185_route = arith_mod.load_json(T185_ROUTE)
    t187_arith = arith_mod.load_json(T187_ARITH)

    # Positive check on committed bundle
    errs = verify_bundle(probe_wall, screen, committed_arith, t182_ref, t185_route, t187_arith)
    if errs:
        print("FAIL: committed bundle failed verification:")
        for e in errs:
            print(f"  - {e}")
        return 1
    print("[OK] committed t188 bundle verified cleanly")

    # Negative controls (must each be rejected with >= 1 error)
    def mutate_case(name: str, fn):
        pw = copy.deepcopy(probe_wall)
        sc = copy.deepcopy(screen)
        ar = copy.deepcopy(committed_arith)
        fn(pw, sc, ar)
        e = verify_bundle(pw, sc, ar, t182_ref, t185_route, t187_arith)
        if not e:
            print(f"[FAIL] negative control NOT rejected: {name}")
            return False
        print(f"  [OK] rejected negative control: {name} ({len(e)} error(s))")
        return True

    controls = [
        (
            "probe hash mismatch vs t182/reference.json",
            lambda pw, sc, ar: pw["cells"]["benchy-classic-on"]["passes"][0].__setitem__(
                "gcode_sha256", "0" * 64
            ),
        ),
        (
            "probe degraded flag set to True",
            lambda pw, sc, ar: pw["cells"]["base-classic-on"]["passes"][1]["inst"].__setitem__(
                "degraded", True
            ),
        ),
        (
            "probe code-1200 diagnostic non-zero",
            lambda pw, sc, ar: pw["cells"]["benchy-arachne-on"]["passes"][0]["inst"].__setitem__(
                "code_1200_count", 1
            ),
        ),
        (
            "cantilever certificate false negative error",
            lambda pw, sc, ar: pw["geometry_census"]["base-classic-on"].__setitem__(
                "cantilever_cert_errors", 1
            ),
        ),
        (
            "screen candidate A/B hash drift",
            lambda pw, sc, ar: sc["supports_on"]["base-classic-on"]["ab_pairs"][0].__setitem__(
                "candidate_sha256", "f" * 64
            ),
        ),
        (
            "screen cell failing 0.80% wall floor",
            lambda pw, sc, ar: [
                p.__setitem__("candidate_wall_s", p["baseline_wall_s"])
                for p in sc["supports_on"]["benchy-classic-on"]["ab_pairs"]
            ],
        ),
        (
            "screen cell failing CPU corroboration",
            lambda pw, sc, ar: [
                p.__setitem__("candidate_cpu_s", p["baseline_cpu_s"] + 5.0)
                for p in sc["supports_on"]["base-arachne-on"]["ab_pairs"]
            ],
        ),
        (
            "committed arithmetic.json drift",
            lambda pw, sc, ar: ar["screen_summary"]["benchy-classic-on"].__setitem__(
                "wall_saved_pct", 99.99
            ),
        ),
        (
            "falsified G_holes drift cover-up",
            lambda pw, sc, ar: pw["geometry_census"]["base-classic-on"].__setitem__(
                "gholes_diff_layers", 0
            ),
        ),
        (
            "rejected mechanism omitted from screen.json",
            lambda pw, sc, ar: sc["rejected_mechanisms"].pop(0),
        ),
        (
            "supports-off inertness hash mismatch",
            lambda pw, sc, ar: sc["supports_off_inertness"]["benchy-classic-off"].__setitem__(
                "gcode_sha256", "deadbeef"
            ),
        ),
        (
            "broken classic==arachne host:support_analysis call invariance",
            lambda pw, sc, ar: sc["supports_on"]["base-arachne-on"]["profile"]["candidate"].__setitem__(
                "sa_total_calls", 9999
            ),
        ),
        (
            "non-monotonic ablation ladder",
            lambda pw, sc, ar: sc["ablation"]["benchy-classic-on"]["sa123"].__setitem__(
                "sa_stage_ms", 999
            ),
        ),
    ]

    ok_count = sum(1 for name, fn in controls if mutate_case(name, fn))
    if ok_count != len(controls):
        print(f"FAIL: {ok_count}/{len(controls)} negative controls rejected")
        return 1
    print(f"PASS: --self-test passed (committed bundle + {ok_count}/{len(controls)} negative controls rejected)")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return run_self_test()
    probe_wall = arith_mod.load_json(REDUCED_DIR / "probe_wall.json")
    screen = arith_mod.load_json(REDUCED_DIR / "screen.json")
    committed_arith = arith_mod.load_json(REDUCED_DIR / "arithmetic.json")
    t182_ref = arith_mod.load_json(T182_REF)
    t185_route = arith_mod.load_json(T185_ROUTE)
    t187_arith = arith_mod.load_json(T187_ARITH)
    errs = verify_bundle(probe_wall, screen, committed_arith, t182_ref, t185_route, t187_arith)
    if errs:
        for e in errs:
            print(f"FAIL: {e}")
        return 1
    print("PASS: t188 bundle verified cleanly")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

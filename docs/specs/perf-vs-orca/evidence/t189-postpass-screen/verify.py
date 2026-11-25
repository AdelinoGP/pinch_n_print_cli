#!/usr/bin/env python3
"""Checkout-only verifier (and negative-control self-test) for t189.

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
T188_ARITH = (
    EVIDENCE_DIR.parent / "t188-support-analysis-screen" / "reduced" / "arithmetic.json"
)

sys.path.insert(0, str(EVIDENCE_DIR))
import arithmetic as arith_mod  # noqa: E402

ALL_CELLS = arith_mod.ALL_CELLS
ABLATION_CELLS = arith_mod.ABLATION_CELLS
ABLATION_CONFIGS = [
    "baseline",
    "pp2_only",
    "pp3_only",
    "pp2_pp3_host_only",
    "pp1_only",
    "full_pp1_pp2_pp3",
]


def _check_qual(q: dict, label: str, errors: list[str]) -> None:
    if (
        q.get("degraded") is not False
        or q.get("fatal_error_count", q.get("fatal")) != 0
        or q.get("non_fatal_error_count", q.get("non_fatal")) != 0
        or q.get("code_1200_count", q.get("code_1200")) != 0
    ):
        errors.append(f"{label} failed qual gate: {q}")


def verify_bundle(
    probe_wall: dict,
    screen: dict,
    committed_arith: dict,
    t182_ref: dict,
    t185_route: dict,
    t188_arith: dict,
) -> list[str]:
    errors: list[str] = []
    refs = t182_ref["active_set"]
    ref_bytes = t182_ref["bytes"]

    # 1. Probe run counts, 100% reference hash/byte parity, qual gates, and invariant facts
    if probe_wall.get("total_runs") != 32:
        errors.append(f"probe total_runs != 32: {probe_wall.get('total_runs')}")

    for cell in ALL_CELLS:
        c = probe_wall.get("cells", {}).get(cell)
        if not c:
            errors.append(f"probe missing cell {cell}")
            continue
        passes = c.get("passes", [])
        if len(passes) != 3:
            errors.append(f"probe {cell} expected 3 passes, got {len(passes)}")
        for p in passes:
            if (
                p["gcode_sha256"] != refs[cell]
                or p["gcode_bytes"] != ref_bytes[cell]
                or not p["reference_match"]
            ):
                errors.append(f"probe {cell} p{p['pass']} hash/bytes mismatch vs t182/reference.json")
            _check_qual(p["qual"], f"probe {cell} p{p['pass']}", errors)
            # Zero rollback clones and +2 net commands in machine-gcode-emit
            gp = p["probes"]["gcode_postprocess"]
            if gp["out_cmds"] - gp["in_cmds"] != 2:
                errors.append(
                    f"probe {cell} p{p['pass']} machine-gcode-emit net cmds != 2: "
                    f"{gp['out_cmds'] - gp['in_cmds']}"
                )
            for m in ("com.core.wipe-tower", "com.core.overhang-classifier-default"):
                if p["probes"]["fin_dispatch"][m]["pushes_count"] != 0:
                    errors.append(f"probe {cell} p{p['pass']} {m} pushes_count != 0")

        pp = c["profile_pass"]
        if (
            pp["gcode_sha256"] != refs[cell]
            or pp["gcode_bytes"] != ref_bytes[cell]
            or not pp["reference_match"]
        ):
            errors.append(f"probe {cell} profile_pass hash/bytes mismatch")
        _check_qual(pp["qual"], f"probe {cell} profile_pass", errors)
        if not isinstance(pp.get("fuel_total"), int) or pp["fuel_total"] <= 0:
            errors.append(f"probe {cell} profile_pass invalid fuel_total")

    # 2. Screen run counts, 100% reference parity, qual gates, and disclosure checks
    if screen.get("measured_runs") != 92 or screen.get("total_runs_verified_reference_match") != 94:
        errors.append(
            f"screen run count mismatch: measured={screen.get('measured_runs')} "
            f"total={screen.get('total_runs_verified_reference_match')}"
        )
    deferred_ids = {d["id"] for d in screen.get("deferred_wit_leads", [])}
    for req_def in ("PP-4", "PP-1b"):
        if req_def not in deferred_ids:
            errors.append(f"screen missing deferred WIT lead disclosure: {req_def}")

    for cell in ALL_CELLS:
        pairs = screen.get("ab", {}).get(cell, {}).get("pairs", [])
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
                errors.append(f"screen ab {cell} r{p['round']} reference mismatch")

        for arm in ("baseline", "candidate"):
            ir = screen["instrumented"][cell][arm]
            if (
                ir["gcode_sha256"] != refs[cell]
                or ir["gcode_bytes"] != ref_bytes[cell]
                or not ir["reference_match"]
            ):
                errors.append(f"screen inst {cell}/{arm} reference mismatch")
            _check_qual(ir["inst"], f"screen inst {cell}/{arm}", errors)

    for cell in ABLATION_CELLS:
        for cfg in ABLATION_CONFIGS:
            rec = screen.get("ablation", {}).get(cell, {}).get(cfg)
            if (
                not rec
                or rec["gcode_sha256"] != refs[cell]
                or rec["gcode_bytes"] != ref_bytes[cell]
                or not rec["reference_match"]
            ):
                errors.append(f"screen ablation {cell}/{cfg} reference mismatch")
            else:
                _check_qual(rec["inst"], f"screen ablation {cell}/{cfg}", errors)

    for cell in ("benchy-classic-on", "base-classic-on"):
        for arm in ("baseline", "candidate"):
            pr = screen.get("profile_fuel", {}).get(cell, {}).get(arm)
            if (
                not pr
                or pr["gcode_sha256"] != refs[cell]
                or pr["gcode_bytes"] != ref_bytes[cell]
                or not pr["reference_match"]
            ):
                errors.append(f"screen profile_fuel {cell}/{arm} reference mismatch")
            else:
                _check_qual(pr["qual"], f"screen profile_fuel {cell}/{arm}", errors)
                if not isinstance(pr.get("fuel_total"), int) or pr["fuel_total"] <= 0:
                    errors.append(f"screen profile_fuel {cell}/{arm} missing fuel_total")

    # 3. Recompute arithmetic from scratch and verify exact match with committed arithmetic.json
    recomputed = arith_mod.compute_arithmetic(
        probe_wall, screen, t182_ref, t185_route, t188_arith
    )
    if recomputed != committed_arith:
        errors.append("committed reduced/arithmetic.json differs from recomputed arithmetic")

    # 4. Screen wall-floor clearance (all 8 cells >= 0.80%, 24/24 pair wins) and stage reductions
    agg = recomputed["screen_aggregate"]
    if agg["cells_clearing_wall_floor"] != 8 or agg["total_wall_wins"] != 24:
        errors.append(
            f"screen failed 8/8 wall floor or 24/24 wins: "
            f"clear={agg['cells_clearing_wall_floor']}/8 wins={agg['total_wall_wins']}/24"
        )
    for cell in ALL_CELLS:
        sc = recomputed["screen_cells"][cell]
        if not sc["clears_wall_floor"]:
            errors.append(f"screen {cell} failed >= 0.80% wall floor ({sc['wall_saved_pct']}%)")
        if sc["postpass_saved_ms"] <= 0:
            errors.append(f"screen {cell} PostPass phase did not improve")
        for st in (
            "PostPass::LayerFinalization",
            "PostPass::GCodeEmit",
            "PostPass::GCodeSerialize",
        ):
            if sc["stages"][st]["saved_ms"] <= 0:
                errors.append(f"screen {cell} stage {st} did not improve")

    # 5. Ablation stage attribution monotonicity on all 4 cells
    for cell in ABLATION_CELLS:
        al = recomputed["ablation_summary"][cell]
        if al["pp2_ser_saved_ms"] <= 0:
            errors.append(f"ablation {cell} PP-2 did not reduce GCodeSerialize")
        if al["pp3_emit_saved_ms"] <= 0:
            errors.append(f"ablation {cell} PP-3 did not reduce GCodeEmit")
        if al["pp1_lf_saved_ms"] <= 0:
            errors.append(f"ablation {cell} PP-1 did not reduce LayerFinalization")
        if not (
            al["configs"]["baseline"]["postpass_ms"]
            > al["configs"]["pp2_pp3_host_only"]["postpass_ms"]
            > al["configs"]["full_pp1_pp2_pp3"]["postpass_ms"]
        ):
            errors.append(
                f"ablation {cell} not strictly monotonic (baseline > host_only > full): {al['configs']}"
            )

    # 6. Deterministic --profile fuel reduction (>70% LayerFinalization, machine-gcode-emit exact)
    for cell in ("benchy-classic-on", "base-classic-on"):
        pf = recomputed["profile_fuel_summary"][cell]
        if pf["layer_finalization_fuel_saved_pct"] < 70.0:
            errors.append(
                f"profile_fuel {cell} LayerFinalization fuel saved < 70%: "
                f"{pf['layer_finalization_fuel_saved_pct']}%"
            )
        if abs(pf["modules"]["com.core.machine-gcode-emit"]["saved_pct"]) > 0.01:
            errors.append(
                f"profile_fuel {cell} machine-gcode-emit fuel unexpectedly changed "
                f"({pf['modules']['com.core.machine-gcode-emit']['saved_pct']}%; WIT boundary)"
            )

    # 7. Stacked ESTIMATE ONLY route projection crosses < 1.00x on all 8/8 cells
    rte = recomputed["route_estimate_only"]
    if rte["est_cells_below_1_00x_stacked_ab"] != 8 or rte["est_cells_below_1_00x_stacked_stage"] != 8:
        errors.append(
            f"route projection expected 8/8 < 1.00x, got ab={rte['est_cells_below_1_00x_stacked_ab']} "
            f"stage={rte['est_cells_below_1_00x_stacked_stage']}"
        )

    return errors


def run_self_test() -> int:
    probe_wall = arith_mod.load_json(REDUCED_DIR / "probe_wall.json")
    screen = arith_mod.load_json(REDUCED_DIR / "screen.json")
    committed_arith = arith_mod.load_json(REDUCED_DIR / "arithmetic.json")
    t182_ref = arith_mod.load_json(T182_REF)
    t185_route = arith_mod.load_json(T185_ROUTE)
    t188_arith = arith_mod.load_json(T188_ARITH)

    errs = verify_bundle(probe_wall, screen, committed_arith, t182_ref, t185_route, t188_arith)
    if errs:
        print("FAIL: committed bundle failed verification:")
        for e in errs:
            print(f"  - {e}")
        return 1
    print("[OK] committed t189 bundle verified cleanly (checkout-only)")

    def mutate_case(name: str, fn) -> bool:
        pw = copy.deepcopy(probe_wall)
        sc = copy.deepcopy(screen)
        ar = copy.deepcopy(committed_arith)
        fn(pw, sc, ar)
        e = verify_bundle(pw, sc, ar, t182_ref, t185_route, t188_arith)
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
            "probe byte-size mismatch vs t182/reference.json",
            lambda pw, sc, ar: pw["cells"]["base-classic-off"]["passes"][1].__setitem__(
                "gcode_bytes", 12345
            ),
        ),
        (
            "probe degraded flag set to True",
            lambda pw, sc, ar: pw["cells"]["base-classic-on"]["passes"][2]["qual"].__setitem__(
                "degraded", True
            ),
        ),
        (
            "probe code-1200 diagnostic non-zero",
            lambda pw, sc, ar: pw["cells"]["benchy-arachne-on"]["passes"][0]["qual"].__setitem__(
                "code_1200_count", 1
            ),
        ),
        (
            "probe machine-gcode-emit net_added_cmds != 2",
            lambda pw, sc, ar: pw["cells"]["benchy-classic-off"]["passes"][0]["probes"][
                "gcode_postprocess"
            ].__setitem__("out_cmds", 999999),
        ),
        (
            "screen candidate A/B hash drift",
            lambda pw, sc, ar: sc["ab"]["base-arachne-on"]["pairs"][1].__setitem__(
                "candidate_sha256", "f" * 64
            ),
        ),
        (
            "screen cell failing 0.80% wall floor",
            lambda pw, sc, ar: [
                p.__setitem__("candidate_wall_s", p["baseline_wall_s"])
                for p in sc["ab"]["benchy-classic-on"]["pairs"]
            ],
        ),
        (
            "screen instrumented stage GCodeSerialize regression",
            lambda pw, sc, ar: sc["instrumented"]["base-classic-on"]["candidate"]["inst"][
                "postpass_stage_ms"
            ].__setitem__("PostPass::GCodeSerialize", 9999),
        ),
        (
            "screen ablation non-monotonic (host_only <= full)",
            lambda pw, sc, ar: sc["ablation"]["base-classic-on"]["pp2_pp3_host_only"]["inst"][
                "phase_ms"
            ].__setitem__("postpass", 10),
        ),
        (
            "screen ablation hash drift",
            lambda pw, sc, ar: sc["ablation"]["benchy-classic-off"]["pp1_only"].__setitem__(
                "gcode_sha256", "a" * 64
            ),
        ),
        (
            "screen profile_fuel LayerFinalization savings wiped out",
            lambda pw, sc, ar: sc["profile_fuel"]["benchy-classic-on"]["candidate"][
                "postpass_modules"
            ]["com.core.wipe-tower"].__setitem__("total_fuel", 999_999_999),
        ),
        (
            "screen profile_fuel machine-gcode-emit WIT boundary violated",
            lambda pw, sc, ar: sc["profile_fuel"]["base-classic-on"]["candidate"][
                "postpass_modules"
            ]["com.core.machine-gcode-emit"].__setitem__("total_fuel", 1),
        ),
        (
            "deferred WIT lead PP-4 omitted from screen.json",
            lambda pw, sc, ar: sc["deferred_wit_leads"].pop(0),
        ),
        (
            "committed arithmetic.json drift",
            lambda pw, sc, ar: ar["screen_cells"]["benchy-classic-on"].__setitem__(
                "wall_saved_pct", 99.99
            ),
        ),
    ]

    passed = sum(1 for name, fn in controls if mutate_case(name, fn))
    print(f"Self-test result: {passed}/{len(controls)} negative controls rejected")
    return 0 if passed == len(controls) else 1


def main() -> int:
    if "--self-test" in sys.argv:
        return run_self_test()
    probe_wall = arith_mod.load_json(REDUCED_DIR / "probe_wall.json")
    screen = arith_mod.load_json(REDUCED_DIR / "screen.json")
    committed_arith = arith_mod.load_json(REDUCED_DIR / "arithmetic.json")
    t182_ref = arith_mod.load_json(T182_REF)
    t185_route = arith_mod.load_json(T185_ROUTE)
    t188_arith = arith_mod.load_json(T188_ARITH)
    errs = verify_bundle(probe_wall, screen, committed_arith, t182_ref, t185_route, t188_arith)
    if errs:
        for e in errs:
            print(f"ERROR: {e}")
        return 1
    print("[OK] t189 bundle verified cleanly (checkout-only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

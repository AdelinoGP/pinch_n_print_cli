#!/usr/bin/env python3
"""Checkout-only verifier and --self-test harness for t187 evidence.

Reads ONLY committed files under docs/specs/perf-vs-orca/evidence/.
Never touches target/, builds binaries, or reads STL fixtures.
"""

from __future__ import annotations

import copy
import json
import statistics
import sys
from pathlib import Path

EVIDENCE_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(EVIDENCE_DIR))

from arithmetic import (  # noqa: E402
    GATE_FLOOR_PCT,
    SUPPORTS_ON_CELLS,
    T182_REF,
    T185_ROUTE,
    compute_arithmetic,
)

REDUCED_DIR = EVIDENCE_DIR / "reduced"
REQUIRED_FILES = [
    "probe.patch",
    "run_probe.py",
    "run_screen.py",
    "reduce.py",
    "arithmetic.py",
    "verify.py",
    "FINDINGS.md",
    "REPRODUCE.md",
    "reduced/probe_wall.json",
    "reduced/probe_fuel.json",
    "reduced/screen.json",
    "reduced/arithmetic.json",
]


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def verify_bundle(
    probe_wall: dict,
    probe_fuel: dict,
    screen: dict,
    arithmetic_committed: dict,
    t182_ref: dict,
    t185_route: dict,
    check_files: bool = True,
) -> list[str]:
    """Return a list of violation strings (empty means PASS)."""
    errors: list[str] = []
    ref_active = t182_ref["active_set"]

    if check_files:
        for rel in REQUIRED_FILES:
            p = EVIDENCE_DIR / rel
            if not p.exists() or p.stat().st_size == 0:
                errors.append(f"Missing or empty required evidence file: {rel}")

    # 1. Re-derive arithmetic.json from committed inputs and assert exact equality
    try:
        recomputed = compute_arithmetic(
            probe_wall, probe_fuel, screen, t182_ref, t185_route
        )
        if recomputed != arithmetic_committed:
            errors.append("Committed reduced/arithmetic.json disagrees with arithmetic.py recomputation")
    except Exception as exc:  # pragma: no cover
        errors.append(f"compute_arithmetic raised: {exc}")
        recomputed = None

    # 2. Probe wall + qual checks
    if probe_wall.get("total_measured_runs") != 16:
        errors.append(f"Expected 16 probe measured runs, got {probe_wall.get('total_measured_runs')}")
    if not probe_wall.get("all_reference_match"):
        errors.append("probe_wall.all_reference_match is not True")

    for cell in SUPPORTS_ON_CELLS:
        expected_sha = ref_active[cell]
        c_wall = probe_wall["cells"][cell]
        for p in c_wall["passes"]:
            if p["exit_code"] != 0:
                errors.append(f"probe_wall {cell} pass {p['pass']} non-zero exit_code")
            if p["gcode_sha256"] != expected_sha or not p["reference_match"]:
                errors.append(f"probe_wall {cell} pass {p['pass']} SHA mismatch vs t182/reference.json")

        q = probe_wall["qual"][cell]
        if q["exit_code"] != 0 or q["gcode_sha256"] != expected_sha or not q["reference_match"]:
            errors.append(f"probe_wall qual {cell} hash/exit mismatch")
        if q["degraded"] is not False:
            errors.append(f"probe_wall qual {cell} degraded != False")
        if q["fatal_error_count"] != 0 or q["non_fatal_error_count"] != 0:
            errors.append(f"probe_wall qual {cell} non-zero fatal/non_fatal error count")
        if q["code_1200_count"] != 0:
            errors.append(f"probe_wall qual {cell} code_1200_count != 0")

    # 3. Classic == Arachne invariance in probe_wall and probe_fuel
    for model in ("benchy", "base"):
        c_cell = f"{model}-classic-on"
        a_cell = f"{model}-arachne-on"
        if probe_wall["cells"][c_cell]["median"]["counters"] != probe_wall["cells"][a_cell]["median"]["counters"]:
            errors.append(f"Classic != Arachne probe counters on {model}")
        if probe_wall["cells"][c_cell]["median"]["phase5_audit"] != probe_wall["cells"][a_cell]["median"]["phase5_audit"]:
            errors.append(f"Classic != Arachne probe phase5_audit on {model}")
        if probe_fuel["cells"][c_cell]["run_support_geometry_total_fuel"] != probe_fuel["cells"][a_cell]["run_support_geometry_total_fuel"]:
            errors.append(f"Classic != Arachne probe fuel on {model}")

    for cell in SUPPORTS_ON_CELLS:
        pf = probe_fuel["cells"][cell]
        if pf["exit_code"] != 0 or pf["gcode_sha256"] != ref_active[cell] or not pf["reference_match"]:
            errors.append(f"probe_fuel {cell} hash/exit mismatch")

    # 4. Screen checks (42 measured runs, A/B medians, CPU corroboration, inertness, P5_tile falsification)
    if screen.get("measured_runs_count") != 42:
        errors.append(f"Expected 42 screen measured runs, got {screen.get('measured_runs_count')}")
    if not screen.get("all_measured_runs_reference_match"):
        errors.append("screen.all_measured_runs_reference_match is not True")

    for cell in SUPPORTS_ON_CELLS:
        expected_sha = ref_active[cell]
        sc = screen["supports_on"][cell]
        if sc["reference_sha256"] != expected_sha or not sc["all_10_runs_reference_match"]:
            errors.append(f"screen {cell} reference hash/flag mismatch")

        pairs = sc["ab_pairs"]
        if len(pairs) != 3:
            errors.append(f"screen {cell} expected 3 A/B pairs, got {len(pairs)}")
        for pr in pairs:
            if (
                pr["baseline_sha256"] != expected_sha
                or pr["candidate_sha256"] != expected_sha
                or not pr["reference_match"]
            ):
                errors.append(f"screen {cell} round {pr['round']} hash mismatch")

        ab = sc["ab_summary"]
        b_med = round(statistics.median([p["baseline_wall_s"] for p in pairs]), 4)
        c_med = round(statistics.median([p["candidate_wall_s"] for p in pairs]), 4)
        b_cpu = round(statistics.median([p["baseline_cpu_s"] for p in pairs]), 4)
        c_cpu = round(statistics.median([p["candidate_cpu_s"] for p in pairs]), 4)
        if ab["baseline_wall_med_s"] != b_med or ab["candidate_wall_med_s"] != c_med:
            errors.append(f"screen {cell} ab_summary wall median disagrees with ab_pairs")
        if ab["baseline_cpu_med_s"] != b_cpu or ab["candidate_cpu_med_s"] != c_cpu:
            errors.append(f"screen {cell} ab_summary cpu median disagrees with ab_pairs")
        exp_pct = round((b_med - c_med) / b_med * 100.0, 2)
        if ab["wall_saved_pct"] != exp_pct:
            errors.append(f"screen {cell} wall_saved_pct mismatch ({ab['wall_saved_pct']} != {exp_pct})")
        if ab["wall_saved_pct"] < GATE_FLOOR_PCT:
            errors.append(f"screen {cell} wall_saved_pct {ab['wall_saved_pct']} < {GATE_FLOOR_PCT}% floor")
        if ab["cpu_saved_pct"] <= 0.0:
            errors.append(f"screen {cell} cpu_saved_pct {ab['cpu_saved_pct']} <= 0 (CPU does not corroborate)")
        if ab["wall_pair_wins"] != 3:
            errors.append(f"screen {cell} wall_pair_wins {ab['wall_pair_wins']} != 3")

        # Instrumented & profile sanity
        for arm in ("baseline", "candidate"):
            inst = sc["instrumented"][arm]
            if inst["degraded"] is not False or inst["fatal_error_count"] != 0 or inst["code_1200_count"] != 0:
                errors.append(f"screen {cell} instrumented {arm} degraded/error")
        if sc["instrumented"]["candidate"]["tree_support_planner_ms"] >= sc["instrumented"]["baseline"]["tree_support_planner_ms"]:
            errors.append(f"screen {cell} tree_support_planner_ms did not drop")
        if sc["profile"]["candidate"]["tree_support_planner"]["total_fuel"] >= sc["profile"]["baseline"]["tree_support_planner"]["total_fuel"]:
            errors.append(f"screen {cell} deterministic planner fuel did not drop")

    # Classic == Arachne exact host:native call delta check
    for model, expected_drop in (("benchy", 10333), ("base", 34817)):
        for gen in ("classic", "arachne"):
            cell = f"{model}-{gen}-on"
            pb = screen["supports_on"][cell]["profile"]["baseline"]["host_native"]["calls"]
            pc = screen["supports_on"][cell]["profile"]["candidate"]["host_native"]["calls"]
            if pb - pc != expected_drop:
                errors.append(
                    f"screen {cell} host:native call drop {pb - pc} != expected invariant {expected_drop}"
                )

    # Supports-off inertness
    inert = screen.get("supports_off_inertness", {})
    for cell in ("benchy-classic-off", "base-classic-off"):
        if cell not in inert:
            errors.append(f"Missing supports_off_inertness cell {cell}")
        else:
            row = inert[cell]
            if row["exit_code"] != 0 or row["gcode_sha256"] != ref_active[cell] or not row["reference_match"]:
                errors.append(f"supports_off_inertness {cell} hash/exit mismatch")

    # P5_tile falsification must be faithfully recorded (benchy matched, base diverged)
    p5 = screen["falsified_candidates"]["P5_tile_phase5_collision_tile_windowing"]
    if p5["verdict"] != "REJECTED_NOT_ZERO_DRIFT":
        errors.append("P5_tile verdict is not REJECTED_NOT_ZERO_DRIFT")
    if p5["cells"]["benchy-classic-on"]["reference_match"] is not True:
        errors.append("P5_tile benchy-classic-on expected reference_match == True")
    if p5["cells"]["base-classic-on"]["reference_match"] is not False:
        errors.append("P5_tile base-classic-on expected reference_match == False (falsified)")
    if p5["cells"]["base-classic-on"]["gcode_sha256"] == ref_active["base-classic-on"]:
        errors.append("P5_tile base-classic-on hash unexpectedly equals t182 reference")

    return errors


def run_self_test() -> int:
    probe_wall = load_json(REDUCED_DIR / "probe_wall.json")
    probe_fuel = load_json(REDUCED_DIR / "probe_fuel.json")
    screen = load_json(REDUCED_DIR / "screen.json")
    arith = load_json(REDUCED_DIR / "arithmetic.json")
    t182_ref = load_json(T182_REF)
    t185_route = load_json(T185_ROUTE)

    # Positive check on committed bundle
    errs = verify_bundle(probe_wall, probe_fuel, screen, arith, t182_ref, t185_route, check_files=True)
    if errs:
        print("FAIL: committed evidence failed verification:")
        for e in errs:
            print(f"  - {e}")
        return 1
    print("PASS: committed t187 evidence verified cleanly")

    # Negative controls (each must be rejected)
    controls: list[tuple[str, callable]] = []

    def nc_probe_hash(pw, pf, sc, ar):
        pw["cells"]["benchy-classic-on"]["passes"][0]["gcode_sha256"] = "0" * 64

    def nc_qual_degraded(pw, pf, sc, ar):
        pw["qual"]["base-classic-on"]["degraded"] = True

    def nc_qual_1200(pw, pf, sc, ar):
        pw["qual"]["benchy-arachne-on"]["code_1200_count"] = 1

    def nc_counter_invariance(pw, pf, sc, ar):
        pw["cells"]["base-arachne-on"]["median"]["counters"]["line_cut_calls"] += 1

    def nc_fuel_invariance(pw, pf, sc, ar):
        pf["cells"]["benchy-arachne-on"]["run_support_geometry_total_fuel"] += 1

    def nc_screen_hash(pw, pf, sc, ar):
        sc["supports_on"]["base-arachne-on"]["ab_pairs"][1]["candidate_sha256"] = "f" * 64

    def nc_screen_median_lie(pw, pf, sc, ar):
        sc["supports_on"]["benchy-classic-on"]["ab_summary"]["candidate_wall_med_s"] -= 0.1

    def nc_floor_fail(pw, pf, sc, ar):
        for p in sc["supports_on"]["base-classic-on"]["ab_pairs"]:
            p["candidate_wall_s"] = p["baseline_wall_s"]
        sc["supports_on"]["base-classic-on"]["ab_summary"]["candidate_wall_med_s"] = (
            sc["supports_on"]["base-classic-on"]["ab_summary"]["baseline_wall_med_s"]
        )
        sc["supports_on"]["base-classic-on"]["ab_summary"]["wall_saved_pct"] = 0.0

    def nc_cpu_no_corrob(pw, pf, sc, ar):
        sc["supports_on"]["benchy-arachne-on"]["ab_summary"]["cpu_saved_pct"] = -0.5

    def nc_arith_drift(pw, pf, sc, ar):
        ar["route_projection_estimate_only"]["est_cells_below_1_00x_ordinary"] = 8

    def nc_p5_coverup(pw, pf, sc, ar):
        sc["falsified_candidates"]["P5_tile_phase5_collision_tile_windowing"]["cells"][
            "base-classic-on"
        ]["reference_match"] = True

    def nc_inert_hash(pw, pf, sc, ar):
        sc["supports_off_inertness"]["benchy-classic-off"]["gcode_sha256"] = "1" * 64

    def nc_host_call_invariance(pw, pf, sc, ar):
        sc["supports_on"]["benchy-arachne-on"]["profile"]["candidate"]["host_native"]["calls"] += 1

    controls = [
        ("mutated probe_wall hash", nc_probe_hash),
        ("qual degraded=True", nc_qual_degraded),
        ("qual code_1200_count=1", nc_qual_1200),
        ("broken classic==arachne probe counters", nc_counter_invariance),
        ("broken classic==arachne probe fuel", nc_fuel_invariance),
        ("mutated screen A/B hash", nc_screen_hash),
        ("mutated screen median vs raw pairs", nc_screen_median_lie),
        ("screen cell failing 0.80% floor", nc_floor_fail),
        ("screen cell failing CPU corroboration", nc_cpu_no_corrob),
        ("committed arithmetic.json drift", nc_arith_drift),
        ("P5_tile falsification cover-up", nc_p5_coverup),
        ("supports-off inertness hash mismatch", nc_inert_hash),
        ("broken classic==arachne host:native call delta", nc_host_call_invariance),
    ]

    failed_controls = 0
    for name, mutator in controls:
        pw = copy.deepcopy(probe_wall)
        pf = copy.deepcopy(probe_fuel)
        sc = copy.deepcopy(screen)
        ar = copy.deepcopy(arith)
        mutator(pw, pf, sc, ar)
        nc_errs = verify_bundle(pw, pf, sc, ar, t182_ref, t185_route, check_files=False)
        if not nc_errs:
            print(f"FAIL: negative control '{name}' was NOT rejected!")
            failed_controls += 1
        else:
            print(f"  [OK] rejected negative control: {name} ({len(nc_errs)} error(s))")

    if failed_controls:
        print(f"FAIL: {failed_controls}/{len(controls)} negative controls escaped detection")
        return 1
    print(f"PASS: --self-test passed (committed bundle + {len(controls)}/{len(controls)} negative controls rejected)")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return run_self_test()
    probe_wall = load_json(REDUCED_DIR / "probe_wall.json")
    probe_fuel = load_json(REDUCED_DIR / "probe_fuel.json")
    screen = load_json(REDUCED_DIR / "screen.json")
    arith = load_json(REDUCED_DIR / "arithmetic.json")
    t182_ref = load_json(T182_REF)
    t185_route = load_json(T185_ROUTE)
    errs = verify_bundle(probe_wall, probe_fuel, screen, arith, t182_ref, t185_route, check_files=True)
    if errs:
        for e in errs:
            print(f"FAIL: {e}")
        return 1
    print("PASS: t187 evidence verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

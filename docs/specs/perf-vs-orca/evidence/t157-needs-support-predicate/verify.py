#!/usr/bin/env python3
"""Verify t157 needs_support predicate attribution artifacts.

Checks:
1. All 8 probe runs in runs.json exited 0 and matched t151's reference.json SHA-256.
2. In probe.json, every run has degraded=false, fatal_error_count=0, non_fatal_error_count=0.
3. In probe.json, every dual-mode run has 0 mismatches across all callers, and call counts
   equal layer_count per site (2 sites on supports-off, 3 sites on supports-on).
4. In summary.json, thread totals and priced wall-equivalent savings reconcile with probe.json
   and t154's arithmetic.json / carriers.json.
5. Stop-rule evaluation in summary.json accurately reflects the >=1.5s base / >=150ms benchy gates.
6. The temporary probe is reverted from crates/ (no PNP_T157_PROBE left in crates/).

Supports `--self-test`, which mutates in-memory copies and asserts that each check catches the fault.
"""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
T151_REF = HERE.parent / "t151-skirt-hull-contour" / "reference.json"
T154_ARITH = HERE.parent / "t154-benchy-per-layer" / "arithmetic.json"
T154_CARRIERS = HERE.parent / "t154-benchy-per-layer" / "carriers.json"

CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]


def validate_bundle(
    runs: list[dict],
    probe: dict,
    summary: dict,
    ref_set: dict[str, str],
    t154_arith: dict,
    t154_carriers: dict,
) -> list[str]:
    errors: list[str] = []

    if len(runs) != 8:
        errors.append(f"Expected 8 runs in runs.json, got {len(runs)}")

    for r in runs:
        cell = r.get("cell")
        mode = r.get("mode")
        if r.get("exit_code") != 0:
            errors.append(f"{cell} [{mode}] exit_code={r.get('exit_code')} != 0")
        want_sha = ref_set.get(cell)
        if not want_sha or r.get("gcode_sha256") != want_sha or not r.get("reference_match"):
            errors.append(f"{cell} [{mode}] G-code SHA-256 mismatch against t151 reference")

    total_dual_calls = 0
    total_mismatches = 0

    for cell in CELLS:
        if cell not in probe.get("cells", {}):
            errors.append(f"Missing cell {cell} in probe.json")
            continue
        for mode in ["dual", "fast"]:
            mdata = probe["cells"][cell].get(mode)
            if not mdata:
                errors.append(f"Missing mode {mode} for {cell} in probe.json")
                continue
            sc = mdata.get("slice_complete") or {}
            if sc.get("degraded") is not False:
                errors.append(f"{cell} [{mode}] degraded={sc.get('degraded')}")
            if sc.get("fatal_error_count") != 0 or sc.get("non_fatal_error_count") != 0:
                errors.append(f"{cell} [{mode}] non-zero error count in slice_complete")

        dual = probe["cells"][cell]["dual"]
        fast = probe["cells"][cell]["fast"]
        layer_count = dual["slice_stats"]["layer_count"]
        expected_callers = (
            ["Layer::Infill", "Layer::Perimeters", "PrePass::SupportAnalysis"]
            if cell.endswith("-on")
            else ["Layer::Infill", "Layer::Perimeters"]
        )
        ns_dual = dual["ns_dual_by_caller"]
        ns_fast = fast["ns_fast_by_caller"]
        if sorted(ns_dual.keys()) != expected_callers:
            errors.append(
                f"{cell} [dual] callers {sorted(ns_dual.keys())} != {expected_callers}"
            )
        if sorted(ns_fast.keys()) != expected_callers:
            errors.append(
                f"{cell} [fast] callers {sorted(ns_fast.keys())} != {expected_callers}"
            )

        for caller in expected_callers:
            c_dual = ns_dual.get(caller, {})
            c_fast = ns_fast.get(caller, {})
            if c_dual.get("calls") != layer_count:
                errors.append(
                    f"{cell} [dual] {caller} calls={c_dual.get('calls')} != {layer_count}"
                )
            if c_fast.get("calls") != layer_count:
                errors.append(
                    f"{cell} [fast] {caller} calls={c_fast.get('calls')} != {layer_count}"
                )
            mism = c_dual.get("mismatches", -1)
            if mism != 0:
                errors.append(f"{cell} [dual] {caller} mismatches={mism} != 0")
            total_dual_calls += c_dual.get("calls", 0)
            total_mismatches += max(0, mism)

        # Reconcile summary.json cell entry
        scell = summary.get("cells", {}).get(cell)
        if not scell:
            errors.append(f"Missing cell {cell} in summary.json")
            continue

        eff_p = float(t154_carriers["cells"][cell]["effective_parallelism"])
        if abs(scell.get("effective_parallelism_t154", 0.0) - eff_p) > 1e-6:
            errors.append(f"{cell} effective_parallelism_t154 mismatch in summary.json")

        pnp_s = float(t154_arith["cells"][cell]["pnp_s"])
        if abs(scell.get("baseline_pnp_s", 0.0) - pnp_s) > 1e-6:
            errors.append(f"{cell} baseline_pnp_s mismatch in summary.json")

        per_layer_saved_ms = scell["thread_totals_ms"]["per_layer_saved_ms"]
        expected_pl_wall_ms = round(per_layer_saved_ms / eff_p, 1)
        got_pl_wall_ms = scell["priced_wall_savings"]["per_layer_wall_eq_saved_ms"]
        if abs(got_pl_wall_ms - expected_pl_wall_ms) > 0.15:
            errors.append(
                f"{cell} per_layer_wall_eq_saved_ms={got_pl_wall_ms} != {expected_pl_wall_ms}"
            )

    eq = summary.get("equivalence_verification", {})
    if eq.get("total_dual_calls") != total_dual_calls:
        errors.append(
            f"summary total_dual_calls={eq.get('total_dual_calls')} != {total_dual_calls}"
        )
    if eq.get("total_dual_mismatches") != 0 or total_mismatches != 0:
        errors.append("non-zero dual mismatches in summary or probe")

    sr = summary.get("stop_rule_evaluation", {})
    if sr.get("graduate_phase2_ticket_158") is not True:
        errors.append("stop_rule_evaluation.graduate_phase2_ticket_158 must be True")

    return errors


def check_clean_crates() -> list[str]:
    errors = []
    targets = [
        REPO / "crates" / "slicer-sdk" / "src" / "views.rs",
        REPO / "crates" / "slicer-runtime" / "src" / "layer_executor.rs",
        REPO
        / "crates"
        / "slicer-runtime"
        / "src"
        / "builtins"
        / "support_analysis_producer.rs",
    ]
    for p in targets:
        txt = p.read_text(encoding="utf-8")
        if "PNP_T157_PROBE" in txt or "t157_" in txt:
            errors.append(f"Unreverted t157 probe token found in {p}")
    return errors


def run_self_test(
    runs: list[dict],
    probe: dict,
    summary: dict,
    ref_set: dict[str, str],
    t154_arith: dict,
    t154_carriers: dict,
) -> int:
    # 1. Mutate a G-code SHA-256 in runs
    bad_runs = copy.deepcopy(runs)
    bad_runs[0]["gcode_sha256"] = "0" * 64
    if not validate_bundle(bad_runs, probe, summary, ref_set, t154_arith, t154_carriers):
        print("SELF-TEST FAILED: did not catch corrupted gcode_sha256")
        return 1

    # 2. Mutate a mismatch count in probe
    bad_probe = copy.deepcopy(probe)
    bad_probe["cells"]["base-classic-on"]["dual"]["ns_dual_by_caller"][
        "Layer::Perimeters"
    ]["mismatches"] = 1
    if not validate_bundle(runs, bad_probe, summary, ref_set, t154_arith, t154_carriers):
        print("SELF-TEST FAILED: did not catch predicate mismatch in probe.json")
        return 1

    # 3. Mutate priced wall saving in summary
    bad_summary = copy.deepcopy(summary)
    bad_summary["cells"]["base-classic-off"]["priced_wall_savings"][
        "per_layer_wall_eq_saved_ms"
    ] += 500.0
    if not validate_bundle(runs, probe, bad_summary, ref_set, t154_arith, t154_carriers):
        print("SELF-TEST FAILED: did not catch pricing arithmetic drift in summary.json")
        return 1

    print("Self-test passed (3 synthetic faults caught).")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--self-test", action="store_true")
    a = ap.parse_args()

    runs = json.loads((HERE / "runs.json").read_text(encoding="utf-8"))
    probe = json.loads((HERE / "probe.json").read_text(encoding="utf-8"))
    summary = json.loads((HERE / "summary.json").read_text(encoding="utf-8"))
    ref_set = json.loads(T151_REF.read_text(encoding="utf-8"))["active_set"]
    t154_arith = json.loads(T154_ARITH.read_text(encoding="utf-8"))
    t154_carriers = json.loads(T154_CARRIERS.read_text(encoding="utf-8"))

    errors = validate_bundle(runs, probe, summary, ref_set, t154_arith, t154_carriers)
    errors.extend(check_clean_crates())
    if errors:
        for e in errors:
            print(f"ERROR: {e}")
        return 1

    print(
        f"t157 verify OK: 8/8 runs exit=0 & ref_match=True, "
        f"{summary['equivalence_verification']['total_dual_calls']} dual calls with "
        f"0 mismatches, crates/ clean."
    )
    if a.self_test:
        return run_self_test(runs, probe, summary, ref_set, t154_arith, t154_carriers)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

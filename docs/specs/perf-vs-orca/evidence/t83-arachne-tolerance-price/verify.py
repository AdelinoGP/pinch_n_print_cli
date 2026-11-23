#!/usr/bin/env python3
"""Checkout-only verification for the t83 pricing evidence.

Re-derives the headline numbers and the gate verdict from the committed reduced
captures, cross-checks them against summary.json, and runs negative controls
that must be rejected.

    python verify.py             # verify committed evidence
    python verify.py --self-test # verify + negative controls
"""

import argparse
import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import reduce  # noqa: E402

HERE = reduce.HERE


def load_captures():
    return [reduce.load(path) for path in sorted((HERE / "captures").glob("*.json"))]


def build(captures):
    runs = [reduce.summarize(c) for c in captures]
    cells = {}
    for run in runs:
        cells.setdefault(f"{run['fixture']}-{run['cell']}", []).append(run)
    verdicts = {
        cell: reduce.cell_verdict(samples) for cell, samples in sorted(cells.items())
    }
    return {
        "gate": {
            "wall_floor_pct_of_cell_wall": reduce.WALL_FLOOR_PCT,
            "material_in_any_cell": any(v["material"] for v in verdicts.values()),
            "cells": [c for c, v in verdicts.items() if v["material"]],
        },
        "runs": {
            f"{r['fixture']}-{r['cell']}-{r['arm']}-r{r['repeat']}": r for r in runs
        },
        "cells": verdicts,
    }


def expect(condition, message):
    if not condition:
        raise AssertionError(message)


def verify(captures):
    committed = reduce.load(HERE / "summary.json")
    derived = build(captures)
    expect(derived == committed, "summary.json does not match a fresh derivation")

    expect(len(derived["cells"]) == 4, f"expected 4 cells, got {len(derived['cells'])}")
    for cell, verdict in derived["cells"].items():
        for arm in ("default", "canonical"):
            expect(verdict[arm]["runs"] == 2, f"{cell}/{arm} does not have 2 runs")
        expect(
            verdict["default"]["arachne_fuel"] > 0,
            f"{cell}: missing arachne fuel on default arm",
        )
        expect(
            verdict["canonical"]["arachne_fuel"] > 0,
            f"{cell}: missing arachne fuel on canonical arm",
        )
        d = verdict["delta"]
        expect(
            d["occupancy_normalized_ceiling_pct_of_wall"] is not None,
            f"{cell}: missing occupancy-normalized ceiling",
        )
        # Vacuous-evidence guard: both arms must produce different geometry, or
        # the two config keys did nothing and the pricing question is moot.
        base = {
            r["output"]["sha256"]
            for r in derived["runs"].values()
            if f"{r['fixture']}-{r['cell']}" == cell and r["arm"] == "default"
        }
        canon = {
            r["output"]["sha256"]
            for r in derived["runs"].values()
            if f"{r['fixture']}-{r['cell']}" == cell and r["arm"] == "canonical"
        }
        expect(base and canon, f"{cell}: missing output shas")
        expect(base != canon, f"{cell}: arms produced identical G-code (vacuous)")

    gate = derived["gate"]
    material = gate["material_in_any_cell"]
    print(
        "gate:",
        "MATERIAL" if material else "DROP (below floor)",
        f"(floor {gate['wall_floor_pct_of_cell_wall']}% of cell wall)",
    )
    for cell, verdict in derived["cells"].items():
        d = verdict["delta"]
        print(
            f"  {cell}: fuel reduction {d['arachne_fuel_reduction_pct']:+.3f}% "
            f"(spread {d['fuel_repeat_spread_pct']:.3f}%), "
            f"normalized ceiling {d['occupancy_normalized_ceiling_pct_of_wall']:.3f}% wall, "
            f"idealized {d['idealized_serial_ceiling_pct_of_wall']:.3f}% wall, "
            f"material={verdict['material']}"
        )
    return derived


def self_test(captures):
    controls = []

    # C1: swapping the arm labels must flip every reduction's sign (the
    # derivation is direction-sensitive, not a constant).
    swapped = copy.deepcopy(captures)
    for capture in swapped:
        arm = capture["metadata"]["arm"]
        capture["metadata"]["arm"] = "canonical" if arm == "default" else "default"
    swapped_result = build(swapped)
    original_result = build(captures)
    flipped = all(
        swapped_result["cells"][cell]["delta"]["arachne_fuel_reduction_pct"]
        * original_result["cells"][cell]["delta"]["arachne_fuel_reduction_pct"]
        < 0
        for cell in original_result["cells"]
    )
    controls.append(("arm labels swapped flips every fuel reduction sign", flipped))

    # C2: zero the fuel delta -> material must become False everywhere.
    zeroed = copy.deepcopy(captures)
    default_fuel = {}
    for capture in zeroed:
        if capture["metadata"]["arm"] == "default":
            key = (capture["metadata"]["fixture"], capture["metadata"]["cell"])
            profile = capture["profile"]
            default_fuel[key] = next(
                m["total_fuel"]
                for m in profile["modules"]
                if m["module_id"] == reduce.ARACHNE
            )
    for capture in zeroed:
        if capture["metadata"]["arm"] == "canonical":
            key = (capture["metadata"]["fixture"], capture["metadata"]["cell"])
            for module in capture["profile"]["modules"]:
                if module["module_id"] == reduce.ARACHNE:
                    module["total_fuel"] = default_fuel[key]
    zeroed_result = build(zeroed)
    controls.append(
        (
            "zero fuel delta rejects material",
            not zeroed_result["gate"]["material_in_any_cell"],
        )
    )

    # C3: raise the floor above every measured ceiling -> DROP.
    original_floor = reduce.WALL_FLOOR_PCT
    try:
        reduce.WALL_FLOOR_PCT = 100.0
        floor_result = build(captures)
    finally:
        reduce.WALL_FLOOR_PCT = original_floor
    controls.append(
        (
            "floor above every ceiling rejects material",
            not floor_result["gate"]["material_in_any_cell"],
        )
    )

    # C4: dropping one cell's canonical runs must make the derivation fail.
    dropped = [
        c
        for c in copy.deepcopy(captures)
        if not (
            c["metadata"]["fixture"] == "base"
            and c["metadata"]["cell"] == "arachne-supports-on"
            and c["metadata"]["arm"] == "canonical"
        )
    ]
    try:
        build(dropped)
        rejected = False
    except (SystemExit, AssertionError):
        rejected = True
    controls.append(("missing canonical arm is rejected", rejected))

    # C5: tampering one run's arachne fuel must change the derived summary.
    tampered = copy.deepcopy(captures)
    for capture in tampered:
        if capture["metadata"]["arm"] == "canonical":
            for module in capture["profile"]["modules"]:
                if module["module_id"] == reduce.ARACHNE:
                    module["total_fuel"] = int(module["total_fuel"] * 0.5)
            break
    controls.append(
        (
            "tampered fuel changes the derivation",
            build(tampered) != build(captures),
        )
    )

    ok = True
    for name, rejected in controls:
        print(f"  self-test {name}: {'OK' if rejected else 'FAILED'}")
        ok = ok and rejected
    if not ok:
        raise SystemExit("negative controls were not rejected")
    print(f"self-test: {len(controls)}/{len(controls)} negative controls rejected")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    captures = load_captures()
    expect(captures, "no captures found")
    verify(captures)
    if args.self_test:
        self_test(captures)
    print("verify: OK")


if __name__ == "__main__":
    main()

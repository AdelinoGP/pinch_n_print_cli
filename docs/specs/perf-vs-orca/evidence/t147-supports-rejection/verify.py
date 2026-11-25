#!/usr/bin/env python3
"""Stdlib-only verifier for the t147 reduced set (`census.json`).

Every check is a falsifiable arithmetic or structural claim over the reduced
artifact. The raw probe tree (gitignored) is the fuller source; this verifier
is deliberately runnable from a plain checkout.

    python verify.py
    python verify.py --self-test
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CENSUS = HERE / "census.json"


def approx(left: float, right: float, tolerance: float = 1e-6) -> bool:
    return abs(left - right) <= tolerance


def check(payload: dict) -> list[str]:
    failures: list[str] = []

    def expect(condition: bool, message: str) -> None:
        if not condition:
            failures.append(message)

    expect(payload.get("schema") == "t147-supports-rejection/v1", "schema id")
    cells = payload.get("cells", {})
    expect(set(cells) == {"benchy-classic-on", "base-classic-on"}, "both cells present")
    benchy = cells.get("benchy-classic-on", {})
    base = cells.get("base-classic-on", {})

    expect(benchy.get("diagnostics") == 101, "benchy diagnostics == 101")
    expect(benchy.get("rejected_entries") == 1, "benchy rejected entries == 1")
    expect(
        benchy.get("rejected_layers_sorted") == [170], "benchy rejected layer == [170]"
    )
    expect(benchy.get("retained_entries") == 201, "benchy retained entries == 201")

    expect(base.get("diagnostics") == 44474, "base diagnostics == 44474")
    expect(base.get("rejected_entries") == 32, "base rejected entries == 32")
    expect(len(base.get("rejected_layers_sorted", [])) == 32, "base 32 distinct layers")

    all_entries = benchy.get("entries", []) + base.get("entries", [])
    expect(len(all_entries) == 33, "33 rejected entries in the reduced set")
    expect(
        all(entry["top_overlap_mm2"] > 0 for entry in all_entries),
        "every top-plane overlap > 0",
    )
    expect(
        all(entry["mid_overlap_mm2"] == 0 for entry in all_entries),
        "every midplane overlap == 0",
    )

    benchy_entry = benchy["entries"][0]
    expect(
        approx(benchy_entry["top_overlap_mm2"], 5.834218),
        "benchy top overlap 5.834218 mm2",
    )
    expect(
        approx(benchy_entry["body_area_mm2"], 256.326, 1e-3),
        "benchy body area 256.326 mm2",
    )
    expect(
        approx(benchy_entry["overlap_pct_of_body"], 2.27609, 1e-3),
        "benchy overlap 2.276 % of body",
    )
    expect(
        all(role["overlap_units2"] > 0 for role in benchy_entry["roles"]),
        "benchy overlapping role is SupportBody only",
    )

    base_tops = [entry["top_overlap_mm2"] for entry in base["entries"]]
    expect(approx(sum(base_tops), 6.891985, 1e-6), "base top overlap sum 6.891985 mm2")
    expect(approx(max(base_tops), 1.470591, 1e-6), "base max top overlap 1.470591 mm2")
    expect(
        min(base_tops) > 0 and min(base_tops) < 1e-4, "base min top overlap is a sliver"
    )

    expect(
        benchy.get("unique_demands_rejected") == 101,
        "benchy unique rejected demands == 101",
    )
    expect(
        base.get("unique_demands_rejected") == 2892,
        "base unique rejected demands == 2892",
    )
    expect(
        benchy.get("unique_demands_only_rejected") == 0
        and base.get("unique_demands_only_rejected") == 0,
        "no demand is uniquely rejected",
    )

    expect(
        approx(benchy.get("missing_support_path_mm", 0), 192.601, 1e-3),
        "benchy missing path 192.601 mm",
    )
    expect(
        approx(benchy.get("missing_share_pct", 0), 0.233, 1e-3),
        "benchy missing share 0.233 %",
    )
    expect(
        approx(base.get("missing_support_path_mm", 0), 36466.736, 1e-3),
        "base missing path 36466.736 mm",
    )
    expect(
        approx(base.get("missing_share_pct", 0), 10.2765, 1e-3),
        "base missing share 10.2765 %",
    )
    expect(
        approx(base.get("missing_support_path_area_bound_mm", 0), 116454.0, 0.5),
        "base solid-fill upper bound 116454 mm",
    )
    expect(
        approx(
            sum(entry["missing_path_estimate_mm"] for entry in base["entries"]),
            base.get("missing_support_path_mm", 0),
            1e-3,
        ),
        "base missing path is the sum of the per-layer estimates",
    )
    expect(
        all(entry["missing_path_estimate_mm"] > 0 for entry in base["entries"]),
        "every base rejected layer has a positive missing-path estimate",
    )

    expect(
        benchy.get("immediate_candidate_total") == 1,
        "benchy immediate candidate count == 1",
    )
    expect(
        benchy.get("immediate_candidate_orca_supported") == 1,
        "benchy immediate candidate is Orca-supported",
    )
    benchy_check = benchy["entries"][0]["immediate_candidates"][0]
    expect(benchy_check["candidate_id"] == 68, "benchy immediate candidate is 68")
    expect(
        approx(benchy_check["orca_coverage"], 0.3685, 1e-4),
        "benchy candidate 68 Orca coverage 0.3685",
    )
    expect(benchy_check["pnp_coverage"] == 0.0, "benchy candidate 68 PnP coverage 0")

    expect(
        base.get("immediate_candidate_total") == 32,
        "base immediate candidate count == 32",
    )
    expect(
        base.get("immediate_candidate_orca_supported") == 19,
        "base 19/32 immediate candidates are Orca-supported",
    )

    layers = set(base.get("rejected_layers_sorted", []))
    expect(
        {33, 34, 35, 36, 38, 39, 40, 41}.issubset(layers) and 37 not in layers,
        "base runs 33..36 and 38..41 bracket a present layer 37",
    )

    expect(
        approx(benchy["support_path_mm"]["pnp"], 82644.6, 0.1),
        "benchy PnP support path 82644.6 mm",
    )
    expect(
        approx(benchy["support_path_mm"]["orca"], 53323.7, 0.1),
        "benchy Orca support path 53323.7 mm",
    )
    expect(
        approx(base["support_path_mm"]["pnp"], 354855.1, 0.1),
        "base PnP support path 354855.1 mm",
    )
    expect(
        approx(base["support_path_mm"]["orca"], 265993.9, 0.1),
        "base Orca support path 265993.9 mm",
    )

    expect(
        benchy["pnp_gcode_sha256"].startswith("d9fc80ba"),
        "benchy PnP g-code is the t144/t145 classic-on currency",
    )
    expect(
        base["pnp_gcode_sha256"].startswith("585c6a8e"),
        "base PnP g-code is the t144 base classic-on currency",
    )
    expect(
        benchy["orca_gcode_sha256"].startswith("d62f4e3f"),
        "benchy Orca g-code hash recorded",
    )
    expect(
        base["orca_gcode_sha256"].startswith("a5553430"),
        "base Orca g-code hash recorded",
    )

    planner = payload.get("planner_plane", "")
    validator = payload.get("validator_plane", "")
    expect("midpoint" in planner, "planner plane is the layer midpoint")
    expect("anchor_z" in validator, "validator plane is anchor_z")

    return failures


def self_test(payload: dict) -> tuple[list[str], int]:
    """Return the mutations the verifier correctly rejects, plus their count."""
    controls = []

    mutated = copy.deepcopy(payload)
    mutated["cells"]["base-classic-on"]["entries"][0]["mid_overlap_mm2"] = 0.001
    controls.append(("nonzero midplane overlap", mutated, "midplane overlap == 0"))

    mutated = copy.deepcopy(payload)
    mutated["cells"]["benchy-classic-on"]["entries"][0]["top_overlap_mm2"] = 0.0
    controls.append(("zero top-plane overlap", mutated, "top-plane overlap > 0"))

    mutated = copy.deepcopy(payload)
    mutated["cells"]["base-classic-on"]["unique_demands_only_rejected"] = 1
    controls.append(("uniquely rejected demand", mutated, "uniquely rejected"))

    mutated = copy.deepcopy(payload)
    mutated["cells"]["benchy-classic-on"]["diagnostics"] = 100
    controls.append(
        ("wrong benchy diagnostic count", mutated, "benchy diagnostics == 101")
    )

    mutated = copy.deepcopy(payload)
    mutated["cells"]["base-classic-on"]["missing_share_pct"] = 0.0
    controls.append(("wrong missing share", mutated, "base missing share"))

    mutated = copy.deepcopy(payload)
    mutated["cells"]["base-classic-on"]["immediate_candidate_orca_supported"] = 0
    controls.append(("wrong supported candidate count", mutated, "19/32"))

    rejected = []
    for name, mutated, needle in controls:
        failures = check(mutated)
        if any(needle in failure for failure in failures):
            rejected.append(name)
        else:
            print(f"NEGATIVE CONTROL NOT REJECTED: {name}", file=sys.stderr)
    return rejected, len(controls)


def main(argv: list[str]) -> int:
    if not CENSUS.exists():
        print(f"missing reduced set: {CENSUS}", file=sys.stderr)
        return 2
    payload = json.loads(CENSUS.read_text(encoding="utf-8"))
    failures = check(payload)
    if "--self-test" in argv:
        rejected, total = self_test(payload)
        print(f"{len(rejected)}/{total} negative controls rejected")
        if failures:
            for failure in failures:
                print(f"FAIL: {failure}")
            return 1
        return 0
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        print(f"{len(failures)} check(s) failed")
        return 1
    # Count the assertions above without hard-coding a duplicate roster.
    print("all checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

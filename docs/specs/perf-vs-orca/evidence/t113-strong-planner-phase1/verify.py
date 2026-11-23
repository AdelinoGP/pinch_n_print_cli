#!/usr/bin/env python3
"""t113 Strong-planner Phase-1 verifier.

Checkout-only: reads `reduced/summary.json` and re-derives the shares, floor
multiples and the structural stop from the committed numbers. No raw capture
tree is required.

`--self-test` mutates a copy of the summary and asserts each mutation is
rejected.
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "reduced" / "summary.json"
FLOOR = 0.0080
SUPPORTS_ON = [
    "benchy-classic-on",
    "benchy-arachne-on",
    "base-classic-on",
    "base-arachne-on",
]
SUPPORTS_OFF = ["benchy-classic-off", "base-classic-off"]
TOL = 1e-6


def close(a: float, b: float, tol: float = TOL) -> bool:
    return abs(a - b) <= tol * max(1.0, abs(a), abs(b))


def verify_summary(summary: dict) -> list[str]:
    errors: list[str] = []
    if not close(summary.get("floor_fraction", -1.0), FLOOR):
        errors.append("floor_fraction is not 0.0080")
    cells = summary.get("cells", {})
    for name in SUPPORTS_ON:
        row = cells.get(name)
        if row is None:
            errors.append(f"missing supports-on cell {name}")
            continue
        if row.get("clean", {}).get("status") != "ok":
            errors.append(f"{name}: not status=ok")
        if row.get("clean", {}).get("degraded") is not False:
            errors.append(f"{name}: degraded is not false")
        if row.get("clean", {}).get("fatal_error_count") != 0:
            errors.append(f"{name}: fatal_error_count is not 0")
        if row.get("clean", {}).get("non_fatal_error_count") != 0:
            errors.append(f"{name}: non_fatal_error_count is not 0")
        wall = row.get("wall_ms", 0.0)
        planner = row.get("planner_module_ms", 0.0)
        floor = wall * FLOOR
        if not close(row.get("floor_ms", -1.0), floor):
            errors.append(f"{name}: floor_ms does not match wall * floor")
        if not close(row.get("planner_share_of_cell", -1.0), planner / wall):
            errors.append(f"{name}: planner_share_of_cell does not recompute")
        if not close(row.get("planner_multiple_of_floor", -1.0), planner / floor):
            errors.append(f"{name}: planner_multiple_of_floor does not recompute")
        probe_total = row.get("probe_total_ms", 0.0)
        if probe_total <= 0:
            errors.append(f"{name}: missing probe_total_ms")
            continue
        terms = row.get("subterms_ms", {})
        for term, value in terms.items():
            share = row.get("subterm_shares_of_probe_total", {}).get(term)
            if share is None or not close(share, value / probe_total):
                errors.append(f"{name}: {term} share does not recompute")
        inflate = terms.get("occupancy_inflate", 0.0)
        if inflate <= 0:
            errors.append(f"{name}: missing occupancy_inflate")
            continue
        if not close(row.get("occupancy_inflate_share_of_cell", -1.0), inflate / wall):
            errors.append(f"{name}: occupancy share of cell does not recompute")
        if not close(
            row.get("occupancy_inflate_multiple_of_floor", -1.0), inflate / floor
        ):
            errors.append(f"{name}: occupancy multiple of floor does not recompute")
        if not close(
            row.get("occupancy_inflate_ceiling_12x_ms", -1.0),
            inflate * (1.0 - 1.0 / 12.0),
        ):
            errors.append(f"{name}: 12x ceiling does not recompute")
        if not close(
            row.get("occupancy_inflate_ceiling_2_6x_ms", -1.0),
            inflate * (1.0 - 1.0 / 2.6),
        ):
            errors.append(f"{name}: 2.6x ceiling does not recompute")
        if row.get("occupancy_inflate_ceiling_2_6x_multiple_of_floor", 0.0) <= 1.0:
            errors.append(f"{name}: measured-shape 2.6x ceiling is below the floor")
    for name in SUPPORTS_OFF:
        row = cells.get(name)
        if row is None:
            errors.append(f"missing supports-off cell {name}")
            continue
        if row.get("planner_multiple_of_floor", 9.9) >= 1.0:
            errors.append(
                f"{name}: planner is not below the floor "
                f"({row.get('planner_multiple_of_floor')}x)"
            )
        if row.get("planner_share_of_cell", 9.9) >= FLOOR:
            errors.append(f"{name}: planner share is not below 0.80%")
    return errors


def mutate(summary: dict, which: int) -> dict:
    out = copy.deepcopy(summary)
    cells = out["cells"]
    if which == 0:
        cells["base-classic-on"]["subterms_ms"]["occupancy_inflate"] *= 1.10
    elif which == 1:
        out["floor_fraction"] = 0.0100
    elif which == 2:
        cells["base-classic-off"]["planner_module_ms"] = 10_000.0
        cells["base-classic-off"]["planner_multiple_of_floor"] = 9.26
    elif which == 3:
        cells["benchy-classic-on"][
            "occupancy_inflate_ceiling_2_6x_multiple_of_floor"
        ] = 0.5
    elif which == 4:
        del cells["base-arachne-on"]
    elif which == 5:
        cells["base-classic-on"]["clean"]["degraded"] = True
    else:
        raise AssertionError(which)
    return out


def main() -> int:
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    errors = verify_summary(summary)
    if errors:
        print("FAIL")
        for error in errors:
            print(f"  - {error}")
        return 1
    print("PASS")
    print(
        "  supports-on: "
        + ", ".join(
            f"{name} {summary['cells'][name]['occupancy_inflate_ceiling_2_6x_multiple_of_floor']:.2f}x"
            for name in SUPPORTS_ON
        )
        + " floor at the measured 2.6x clamp"
    )
    print(
        "  supports-off: "
        + ", ".join(
            f"{name} {summary['cells'][name]['planner_multiple_of_floor']:.3f}x"
            for name in SUPPORTS_OFF
        )
        + " floor (the planner module alone)"
    )
    if "--self-test" in sys.argv[1:]:
        rejected = 0
        for which in range(6):
            mutated = mutate(summary, which)
            if verify_summary(mutated):
                rejected += 1
        if rejected != 6:
            print(f"FAIL self-test: only {rejected}/6 mutations rejected")
            return 1
        print("self-test: 6/6 mutations rejected")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

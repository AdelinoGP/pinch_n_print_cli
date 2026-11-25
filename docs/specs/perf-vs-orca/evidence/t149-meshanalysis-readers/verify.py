#!/usr/bin/env python3
"""Re-derive every t149 number from census.json.

The ticket's claim is arithmetic over one probe run per cell: the sub-phases of
`PrePass::MeshAnalysis` and the bridge-clustering half's share of the stage,
the cell and the cell's required cut. Nothing here is a measured A/B; the
ratios of record are citations into the census.

Run:            python verify.py
Negative test:  python verify.py --self-test   (4/4 controls must be rejected)
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

CENSUS = Path(__file__).with_name("census.json")

# The numbers published in FINDINGS.md. verify.py recomputes each from the
# census and compares; a claim that drifts fails the check.
CLAIMS = {
    "base-classic-off": {
        "phase_sum_ms": 5279.5,
        "unaccounted_ms": 46.3,
        "unaccounted_pct": 0.87,
        "remaining_stage_ms": 762.6,
        "bridge_share_of_stage_pct": 85.68,
        "stage_share_of_cell_pct": 15.11,
        "bridge_share_of_cell_pct": 12.95,
        "floor_ms": 281.95,
        "floor_multiple": 16.19,
        "required_cut_pct": 51.88,
        "required_cut_ms": 18283.46,
        "bridge_share_of_cut_pct": 24.96,
    },
    "base-classic-on": {
        "phase_sum_ms": 4797.1,
        "unaccounted_ms": 35.8,
        "unaccounted_pct": 0.74,
        "remaining_stage_ms": 682.7,
        "bridge_share_of_stage_pct": 85.87,
        "stage_share_of_cell_pct": 5.65,
        "bridge_share_of_cell_pct": 4.85,
        "floor_ms": 684.20,
        "floor_multiple": 6.07,
        "required_cut_pct": 70.20,
        "required_cut_ms": 60040.8,
        "bridge_share_of_cut_pct": 6.91,
    },
    "benchy-classic-off": {
        "phase_sum_ms": 192.6,
        "unaccounted_ms": 2.5,
        "unaccounted_pct": 1.28,
        "remaining_stage_ms": 100.0,
        "bridge_share_of_stage_pct": 48.74,
        "stage_share_of_cell_pct": 5.27,
        "bridge_share_of_cell_pct": 2.57,
        "floor_ms": 29.62,
        "floor_multiple": 3.21,
        "required_cut_pct": 51.05,
        "required_cut_ms": 1889.96,
        "bridge_share_of_cut_pct": 5.03,
    },
}

PROFILE_CLAIMS = {
    "phase_sum_ms": 4859.3,
    "unaccounted_ms": 54.8,
    "stage_minus_probe_ms": 725.38,
    "stage_over_probe_ratio": 1.1476,
}

# Composition on base classic-off: the whole bridge half disappears and nothing
# else moves. Indicative — the ratio of record is from another window.
COMPOSITION_CLAIMS = {
    "after_ms": 30680.8,
    "ratio_after": 2.3871,
    "cut_after_pct": 58.11,
}

TOL = 0.02


def derive(cell: dict, floor_fraction: float) -> dict:
    phase_sum = (
        cell["classify_ms"]
        + cell["footprint_ms"]
        + cell["needs_support_ms"]
        + cell["bridge_ms"]
    )
    total = cell["probe_total_ms"]
    elapsed = cell["elapsed_ms"]
    cut_fraction = 1.0 - 1.0 / cell["ratio_of_record"]
    cut_ms = elapsed * cut_fraction
    return {
        "phase_sum_ms": phase_sum,
        "unaccounted_ms": total - phase_sum,
        "unaccounted_pct": 100.0 * (total - phase_sum) / total,
        "remaining_stage_ms": total - cell["bridge_ms"],
        "bridge_share_of_stage_pct": 100.0 * cell["bridge_ms"] / total,
        "stage_share_of_cell_pct": 100.0 * total / elapsed,
        "bridge_share_of_cell_pct": 100.0 * cell["bridge_ms"] / elapsed,
        "floor_ms": floor_fraction * elapsed,
        "floor_multiple": cell["bridge_ms"] / (floor_fraction * elapsed),
        "required_cut_pct": 100.0 * cut_fraction,
        "required_cut_ms": cut_ms,
        "bridge_share_of_cut_pct": 100.0 * cell["bridge_ms"] / cut_ms,
    }


def check(census: dict) -> list[str]:
    failures: list[str] = []
    floor = census["floor_fraction"]
    for cell in census["cells"]:
        name = cell["cell"]
        got = derive(cell, floor)
        want = CLAIMS[name]
        for key, expected in want.items():
            actual = got[key]
            if abs(actual - expected) > TOL:
                failures.append(
                    f"{name}.{key}: published {expected}, derived {actual:.4f}"
                )

        # Physical invariants the arithmetic must satisfy.
        if got["unaccounted_ms"] < -1e-9:
            failures.append(f"{name}: sub-phases exceed the stage total")
        if got["unaccounted_ms"] > 0.02 * cell["probe_total_ms"]:
            failures.append(f"{name}: >2% of the stage is unaccounted")

    prof = census["profiled_run"]
    prof_sum = (
        prof["classify_ms"]
        + prof["footprint_ms"]
        + prof["needs_support_ms"]
        + prof["bridge_ms"]
    )
    prof_got = {
        "phase_sum_ms": prof_sum,
        "unaccounted_ms": prof["probe_total_ms"] - prof_sum,
        "stage_minus_probe_ms": prof["stage_wall_ms"] - prof["probe_total_ms"],
        "stage_over_probe_ratio": prof["stage_wall_ms"] / prof["probe_total_ms"],
    }
    for key, expected in PROFILE_CLAIMS.items():
        if abs(prof_got[key] - expected) > TOL:
            failures.append(
                f"profiled.{key}: published {expected}, derived {prof_got[key]:.4f}"
            )
    if prof["stage_wall_ms"] < prof["probe_total_ms"]:
        failures.append("profiled: stage wall is below the probe total")

    base_off = next(c for c in census["cells"] if c["cell"] == "base-classic-off")
    after_ms = base_off["elapsed_ms"] - base_off["bridge_ms"]
    ratio_after = base_off["ratio_of_record"] * base_off["elapsed_ms"] / after_ms
    comp_got = {
        "after_ms": after_ms,
        "ratio_after": ratio_after,
        "cut_after_pct": 100.0 * (1.0 - 1.0 / ratio_after),
    }
    for key, expected in COMPOSITION_CLAIMS.items():
        if abs(comp_got[key] - expected) > TOL:
            failures.append(
                f"composition.{key}: published {expected}, derived {comp_got[key]:.4f}"
            )
    return failures


def negative_controls(census: dict) -> int:
    controls = {
        "halve the bridge half on base-off": lambda c: c["cells"][0].__setitem__(
            "bridge_ms", c["cells"][0]["bridge_ms"] / 2.0
        ),
        "make a sub-phase exceed the stage": lambda c: c["cells"][1].__setitem__(
            "needs_support_ms", 5000.0
        ),
        "swap the two base cell walls": lambda c: (
            c["cells"][0].__setitem__("elapsed_ms", 85525),
            c["cells"][1].__setitem__("elapsed_ms", 35244),
        ),
        "put the profiled stage below the probe": lambda c: c[
            "profiled_run"
        ].__setitem__("stage_wall_ms", 4000.0),
    }
    rejected = 0
    for label, mutate in controls.items():
        broken = copy.deepcopy(census)
        mutate(broken)
        if check(broken):
            rejected += 1
            print(f"  control rejected: {label}")
        else:
            print(f"  CONTROL PASSED (bad): {label}")
    return rejected


def main() -> int:
    census = json.loads(CENSUS.read_text(encoding="utf-8"))
    failures = check(census)

    floor = census["floor_fraction"]
    for cell in census["cells"]:
        got = derive(cell, floor)
        print(
            f"{cell['cell']}  cell={cell['elapsed_ms']} ms  stage={cell['probe_total_ms']} ms"
        )
        for key in CLAIMS[cell["cell"]]:
            print(f"    {key:32s} {got[key]:12.2f}")

    prof = census["profiled_run"]
    print(
        f"profiled {prof['cell']}  probe={prof['probe_total_ms']} ms  "
        f"stage={prof['stage_wall_ms']:.1f} ms  ratio={prof['stage_wall_ms'] / prof['probe_total_ms']:.4f}"
    )

    if failures:
        print("\nFAIL")
        for line in failures:
            print(f"  {line}")
        return 1

    print(
        f"\nPASS  {sum(len(v) for v in CLAIMS.values()) + len(PROFILE_CLAIMS) + len(COMPOSITION_CLAIMS)} claims re-derived"
    )

    if "--self-test" in sys.argv:
        rejected = negative_controls(census)
        if rejected != 4:
            print(f"SELF-TEST FAIL: {rejected}/4 controls rejected")
            return 1
        print("SELF-TEST PASS: 4/4 negative controls rejected")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

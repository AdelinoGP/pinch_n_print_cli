#!/usr/bin/env python3
"""Checkout-only verifier for the ticket-75 ShellClassification split.

Reads `split.json` (reduced from the gitignored raw captures by `reduce.py`)
and checks that the committed attribution and candidate numbers are internally
consistent and reproduce the ticket's claims. No raw capture tree is required.

Exit 0 = verified; exit 1 = a check or a negative control failed.
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPLIT = HERE / "split.json"

PHASE_B_TERMS = (
    "depth_ms",
    "gather_ms",
    "lower_ms",
    "remove_ms",
    "clip_ms",
    "surface_ms",
    "erosion_ms",
    "retain_ms",
    "presort_ms",
    "expansion_ms",
    "angles_ms",
)

# Base cells measure a 78-80% two-Miter-offset share of the serial commit;
# benchy's smaller stack measures ~65%. Floors below those, high enough that
# deleting one offset fails the check.
BASE_OFFSET_SHARE_FLOOR = 0.70
BENCHY_OFFSET_SHARE_FLOOR = 0.55
BASE_STAGE_REDUCTION_FLOOR_MS = 8000.0
BENCHY_STAGE_REDUCTION_FLOOR_MS = 2000.0
STAGE_RATIO_CEILING = 0.75
CPU_RATIO_CEILING = 1.03
TERM_TOLERANCE = 0.05


def check(payload: dict) -> list[str]:
    problems: list[str] = []
    cells = payload.get("cells", {})
    expected = {
        "base-classic-off",
        "base-classic-on",
        "base-arachne-on",
        "benchy-classic-on",
    }
    if set(cells) != expected:
        return [f"cell set is {sorted(cells)}"]
    for name, cell in cells.items():
        baseline = cell["baseline"]
        candidate = cell["candidate"]
        probe = baseline["probe"]
        for key in PHASE_B_TERMS:
            if key not in probe:
                problems.append(f"{name}: probe missing {key}")
        if problems:
            continue
        term_sum = sum(float(probe[key]) for key in PHASE_B_TERMS)
        phase_b = float(probe["phase_b_ms"])
        if abs(term_sum - phase_b) > max(50.0, phase_b * TERM_TOLERANCE):
            problems.append(
                f"{name}: phase-B terms {term_sum:.1f} ms do not explain {phase_b:.1f} ms"
            )
        offsets = float(probe["clip_ms"]) + float(probe["erosion_ms"])
        share = offsets / phase_b if phase_b else 0.0
        floor = (
            BASE_OFFSET_SHARE_FLOOR
            if cell["fixture"] == "base"
            else BENCHY_OFFSET_SHARE_FLOOR
        )
        if share < floor:
            problems.append(f"{name}: two-offset share {share:.3f} below floor {floor}")

        if int(probe["preps"]) != int(probe["clip_calls"]):
            problems.append(
                f"{name}: preps {probe['preps']} != clip calls {probe['clip_calls']}"
            )
        if int(probe["angles_calls"]) > int(probe["preps"]):
            problems.append(f"{name}: more angle calls than preps")
        if int(probe["retain_calls"]) < int(probe["angles_calls"]):
            problems.append(f"{name}: retain calls below angle calls")
        if int(probe["lower_layers"]) <= 0 or int(probe["lower_polys"]) <= 0:
            problems.append(f"{name}: lower-candidate fallback was not exercised")
        if int(probe["deep_after_remove_verts"]) > int(probe["deep_before_verts"]):
            problems.append(f"{name}: lower-candidate subtraction grew the area")
        if int(probe["deep_before_polys"]) <= 0 or int(probe["clip_polys"]) <= 0:
            problems.append(f"{name}: empty deep/clip polygon census")

        baseline_stage = float(baseline["stage_ms"])
        candidate_stage = float(candidate["stage_ms"])
        reduction = baseline_stage - candidate_stage
        floor_ms = (
            BASE_STAGE_REDUCTION_FLOOR_MS
            if cell["fixture"] == "base"
            else BENCHY_STAGE_REDUCTION_FLOOR_MS
        )
        if reduction < floor_ms:
            problems.append(
                f"{name}: stage reduction {reduction:.0f} ms below floor {floor_ms:.0f} ms"
            )
        if candidate_stage > baseline_stage * STAGE_RATIO_CEILING:
            problems.append(
                f"{name}: candidate stage {candidate_stage:.0f} ms above ratio ceiling"
            )
        if (
            float(candidate["cpu_seconds"])
            > float(baseline["cpu_seconds"]) * CPU_RATIO_CEILING
        ):
            problems.append(
                f"{name}: process CPU grew more than {CPU_RATIO_CEILING:.2f}x"
            )

        reference = cell["reference"]
        for arm in ("baseline", "candidate"):
            if cell[arm]["sha256"] != reference["sha256"]:
                problems.append(
                    f"{name}/{arm}: output {cell[arm]['sha256'][:8]} != reference {reference['sha256'][:8]}"
                )
            if int(cell[arm]["bytes"]) != int(reference["bytes"]):
                problems.append(f"{name}/{arm}: output bytes != reference")
    return problems


def negative_controls(payload: dict) -> list[str]:
    failures: list[str] = []

    tampered = copy.deepcopy(payload)
    tampered["cells"]["base-classic-on"]["candidate"]["stage_ms"] = tampered["cells"][
        "base-classic-on"
    ]["baseline"]["stage_ms"]
    if not check(tampered):
        failures.append("negative control: no-reduction candidate was accepted")

    tampered = copy.deepcopy(payload)
    tampered["cells"]["base-classic-off"]["baseline"]["probe"]["clip_ms"] = 0.0
    tampered["cells"]["base-classic-off"]["baseline"]["probe"]["erosion_ms"] = 0.0
    if not check(tampered):
        failures.append("negative control: deleted offset terms were accepted")

    tampered = copy.deepcopy(payload)
    tampered["cells"]["benchy-classic-on"]["candidate"]["sha256"] = "0" * 64
    if not check(tampered):
        failures.append("negative control: output drift was accepted")

    tampered = copy.deepcopy(payload)
    tampered["cells"]["base-arachne-on"]["baseline"]["probe"]["lower_layers"] = 0
    if not check(tampered):
        failures.append("negative control: unexercised fallback was accepted")

    return failures


def main() -> int:
    payload = json.loads(SPLIT.read_text(encoding="utf-8"))
    problems = check(payload)
    problems += negative_controls(payload)
    if problems:
        for problem in problems:
            print("FAIL:", problem)
        return 1
    for name, cell in payload["cells"].items():
        probe = cell["baseline"]["probe"]
        offsets = float(probe["clip_ms"]) + float(probe["erosion_ms"])
        print(
            f"{name}: phase-B {probe['phase_b_ms']:.0f} ms, offsets {offsets:.0f} ms "
            f"({offsets / float(probe['phase_b_ms']) * 100:.1f}%), stage "
            f"{cell['baseline']['stage_ms']} -> {cell['candidate']['stage_ms']} ms, "
            f"sha {cell['candidate']['sha256'][:8]}"
        )
    print(
        "PASS: four attribution cells consistent; candidate stage reductions and output identity hold"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

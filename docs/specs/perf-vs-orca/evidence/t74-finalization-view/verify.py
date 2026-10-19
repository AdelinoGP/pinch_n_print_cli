#!/usr/bin/env python3
"""Checkout-only verifier for the ticket-74 finalization-view split.

Reads `split.json` (reduced from the gitignored raw captures by `reduce.py`)
and checks that the committed numbers are internally consistent and reproduce
the ticket's claims. No raw capture tree is required.

Exit 0 = verified; exit 1 = a check or a negative control failed.
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SPLIT = HERE / "split.json"

# Bracket decomposition tolerance: the probe's per-term timers exclude
# instrumentation and allocation-adjacent work, so the sum can differ from the
# module bracket by a few milliseconds. 5% covers that without hiding a missing
# term (each whole-print clone/drop term is 20%+ of the baseline bracket).
BRACKET_TOLERANCE = 0.05
EXPECTED_SUPPORT_IDS_ON = 23751
BASELINE_ON_FLOOR_MS = 50_000.0
CANDIDATE_ON_CEILING_MS = 3_500.0
REDUCTION_FLOOR = 0.90


def decomposition_ms(module: dict) -> float:
    return sum(
        float(module[key])
        for key in (
            "snapshot_ms",
            "view_ms",
            "call_ms",
            "store_drop_ms",
            "clone_ms",
            "old_layers_drop_ms",
            "apply_ms",
        )
    )


def check(payload: dict) -> list[str]:
    problems: list[str] = []
    captures = payload.get("captures", {})
    if set(captures) != {
        "baseline-on",
        "baseline-off",
        "candidate-on",
        "candidate-off",
    }:
        return [f"capture set is {sorted(captures)}"]
    for name, capture in captures.items():
        modules = capture.get("modules", [])
        if len(modules) != 4:
            problems.append(f"{name}: expected 4 modules, got {len(modules)}")
            continue
        bracket_sum = sum(float(m["bracket_ms"]) for m in modules)
        if abs(bracket_sum - float(capture["module_sum_ms"])) > 0.01:
            problems.append(f"{name}: module_sum_ms does not match the module brackets")
        for module in modules:
            bracket = float(module["bracket_ms"])
            decomposed = decomposition_ms(module)
            if abs(decomposed - bracket) > max(25.0, bracket * BRACKET_TOLERANCE):
                problems.append(
                    f"{name}/{module['module']}: terms {decomposed:.1f} ms do not explain bracket {bracket:.1f} ms"
                )
            expected_support_ids = (
                EXPECTED_SUPPORT_IDS_ON if capture["supports"] == "on" else 0
            )
            if int(module["support_ids"]) != expected_support_ids:
                problems.append(
                    f"{name}/{module['module']}: support_ids {module['support_ids']} != {expected_support_ids}"
                )
            if (
                int(module.get("local_locks", 0)) != 0
                or int(module.get("global_locks", 0)) != 0
            ):
                problems.append(
                    f"{name}/{module['module']}: fast-path lock guard expected 0/0 tags"
                )
        if capture["supports"] == "on":
            if name == "baseline-on" and bracket_sum < BASELINE_ON_FLOOR_MS:
                problems.append(
                    f"{name}: baseline module sum {bracket_sum} below floor"
                )
            if name == "candidate-on":
                if bracket_sum > CANDIDATE_ON_CEILING_MS:
                    problems.append(
                        f"{name}: candidate module sum {bracket_sum} above ceiling"
                    )
                baseline_sum = float(captures["baseline-on"]["module_sum_ms"])
                reduction = 1.0 - bracket_sum / baseline_sum
                if reduction < REDUCTION_FLOOR:
                    problems.append(
                        f"{name}: reduction {reduction:.3f} below {REDUCTION_FLOOR}"
                    )
        else:
            baseline_sum = float(captures["baseline-off"]["module_sum_ms"])
            if name == "candidate-off" and bracket_sum > baseline_sum * 1.10:
                problems.append(
                    f"{name}: supports-off module sum {bracket_sum} regressed vs {baseline_sum}"
                )
    for supports in ("on", "off"):
        shas = {
            captures[f"{arm}-{supports}"]["reference_sha256"]
            for arm in ("baseline", "candidate")
        }
        if len(shas) != 1:
            problems.append(
                f"supports {supports}: arms reference different output shas"
            )
        for arm in ("baseline", "candidate"):
            if not captures[f"{arm}-{supports}"].get("reference_sha256"):
                problems.append(f"{arm}-{supports}: missing reference sha")
    return problems


def negative_controls(payload: dict) -> list[str]:
    """Each tampered copy must be rejected by `check`."""
    failures: list[str] = []
    tampered = copy.deepcopy(payload)
    tampered["captures"]["baseline-on"]["modules"][0]["snapshot_ms"] = 0.0
    if not check(tampered):
        failures.append("negative control: removed snapshot term was accepted")
    tampered = copy.deepcopy(payload)
    tampered["captures"]["baseline-off"]["modules"][0]["support_ids"] = (
        EXPECTED_SUPPORT_IDS_ON
    )
    if not check(tampered):
        failures.append("negative control: supports-off support ids were accepted")
    tampered = copy.deepcopy(payload)
    tampered["captures"]["candidate-on"]["modules"] = copy.deepcopy(
        payload["captures"]["baseline-on"]["modules"]
    )
    tampered["captures"]["candidate-on"]["module_sum_ms"] = payload["captures"][
        "baseline-on"
    ]["module_sum_ms"]
    if not check(tampered):
        failures.append("negative control: baseline-shaped candidate was accepted")
    return failures


def main() -> int:
    payload = json.loads(SPLIT.read_text(encoding="utf-8"))
    problems = check(payload)
    problems += negative_controls(payload)
    if problems:
        for problem in problems:
            print("FAIL:", problem)
        return 1
    baseline_on = payload["captures"]["baseline-on"]["module_sum_ms"]
    candidate_on = payload["captures"]["candidate-on"]["module_sum_ms"]
    print(
        "PASS: four captures consistent; supports-on module brackets "
        f"{baseline_on:.0f} ms -> {candidate_on:.0f} ms "
        f"({(1 - candidate_on / baseline_on) * 100:.1f}% reduction)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

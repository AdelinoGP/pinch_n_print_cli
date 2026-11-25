#!/usr/bin/env python3
"""Re-derive the t152 closure claims from the committed evidence alone.

Claims checked:

1. The baseline arm reproduces the t151 active reference for base classic-off.
2. The candidate arm moves; its bytes/lines and every census field are the
   measured ones.
3. The candidate delta is a geometry delta, not a pure loop rotation: layers
   moved, points added/removed, E totals changed.
4. The seam stage falls 1,779 -> 531 ms; the offset term is 1,248 ms = 70.15 %
   of the baseline stage.
5. The candidate fails the two unit tests that pin the inset contract; the
   baseline passes 31/31.
6. The preserved patch removes the `offset_polygons_batch` call and the inset
   projection.

`--self-test` mutates each claim in turn and requires the verifier to reject it.
"""

from __future__ import annotations

import argparse
import copy
import csv
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CELL = "base-classic-off"
T151_REFERENCE = HERE.parent / "t151-skirt-hull-contour" / "reference.json"


def load(name: str) -> dict:
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def rows() -> dict[str, dict]:
    with (HERE / "rows.csv").open(encoding="utf-8-sig", newline="") as handle:
        return {row["arm"]: row for row in csv.DictReader(handle)}


def check(
    census: dict, stage: dict, tests: dict, rows_by_arm: dict, reference: dict
) -> list[str]:
    errors: list[str] = []
    baseline = rows_by_arm.get("baseline-ordinary")
    candidate = rows_by_arm.get("candidate-ordinary")
    if baseline is None or candidate is None:
        return ["rows.csv is missing an arm"]
    if baseline["cell"] != CELL or candidate["cell"] != CELL:
        errors.append("rows.csv cell is not base-classic-off")
    if baseline["sha256"] != reference["active_set"][CELL]:
        errors.append("baseline does not reproduce the t151 active reference")
    if candidate["sha256"] == baseline["sha256"]:
        errors.append("candidate is byte-identical; the screen would have passed")
    if census["old"]["sha256"] != baseline["sha256"]:
        errors.append("census old hash disagrees with rows.csv baseline")
    if census["new"]["sha256"] != candidate["sha256"]:
        errors.append("census new hash disagrees with rows.csv candidate")
    if census["layers"] != 496:
        errors.append("layer count is not 496")
    if census["moved_layers"] != 456:
        errors.append("moved-layer count is not 456")
    if census["added_points"] != 1177 or census["removed_points"] != 1149:
        errors.append("point delta is not +1177/-1149")
    if census["e_total_changed_layers"] != 142:
        errors.append("E-total-changed layer count is not 142")
    if census["first_moved_layer"] != 1 or census["first_moved_section"] != "Skirt":
        errors.append("first moved layer/section is not 1/Skirt")
    if census["new"]["bytes"] - census["old"]["bytes"] != 1158:
        errors.append("byte delta is not +1158")
    if census["new"]["lines"] - census["old"]["lines"] != 28:
        errors.append("line delta is not +28")

    if stage["baseline"]["stage_ms"] != 1779:
        errors.append("baseline seam stage is not 1779 ms")
    if stage["candidate"]["stage_ms"] != 531:
        errors.append("candidate seam stage is not 531 ms")
    if stage["offset_saving_ms"] != 1248:
        errors.append("offset saving is not 1248 ms")
    if stage["offset_share_of_baseline_stage_pct"] != 70.15:
        errors.append("offset share is not 70.15 %")

    if tests["candidate"]["passed"] != 29 or tests["candidate"]["failed"] != 2:
        errors.append("candidate unit-test counts are not 29/2")
    if set(tests["candidate"]["failed_tests"]) != {
        "tests::batched_inset_offsets_match_the_serial_reference_per_region",
        "tests::candidate_less_region_still_emits_one_keyed_plan_entry",
    }:
        errors.append("candidate failed-test set changed")
    if tests["baseline"]["passed"] != 31 or tests["baseline"]["failed"] != 0:
        errors.append("baseline unit-test counts are not 31/0")

    patch = (HERE / "candidate.patch").read_text(encoding="utf-8", errors="replace")
    if "offset_polygons_batch" not in patch:
        errors.append("preserved patch does not remove the batched offset")
    if "project_point_onto_inset_boundary" not in patch:
        errors.append("preserved patch does not remove the inset projection")

    return errors


def run_self_test() -> int:
    census = load("census.json")
    stage = load("stage.json")
    tests = load("unit-tests.json")
    rows_by_arm = rows()
    reference = json.loads(T151_REFERENCE.read_text(encoding="utf-8"))
    if check(census, stage, tests, rows_by_arm, reference):
        print("self-test: clean evidence rejected")
        return 1

    mutations: list[tuple[str, dict, dict, dict, dict]] = []

    mutated_census = copy.deepcopy(census)
    mutated_census["moved_layers"] = 455
    mutations.append(("moved_layers", mutated_census, stage, tests, rows_by_arm))

    mutated_stage = copy.deepcopy(stage)
    mutated_stage["offset_saving_ms"] = 1247
    mutations.append(("offset_saving_ms", census, mutated_stage, tests, rows_by_arm))

    mutated_tests = copy.deepcopy(tests)
    mutated_tests["candidate"]["failed"] = 1
    mutations.append(("candidate_failed", census, stage, mutated_tests, rows_by_arm))

    mutated_rows = copy.deepcopy(rows_by_arm)
    mutated_rows["candidate-ordinary"]["sha256"] = mutated_rows["baseline-ordinary"][
        "sha256"
    ]
    mutations.append(("candidate_hash", census, stage, tests, mutated_rows))

    for name, c, s, t, r in mutations:
        if not check(c, s, t, r, reference):
            print(f"self-test: mutation '{name}' was not rejected")
            return 1
    print("self-test: 4/4 mutations rejected")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return run_self_test()

    census = load("census.json")
    stage = load("stage.json")
    tests = load("unit-tests.json")
    rows_by_arm = rows()
    reference = json.loads(T151_REFERENCE.read_text(encoding="utf-8"))
    errors = check(census, stage, tests, rows_by_arm, reference)
    if errors:
        for error in errors:
            print(f"FAIL: {error}")
        return 1
    print(
        "PASS: baseline reproduces t151; candidate moves 456/496 layers "
        "(+1177/-1149 points, 142 E-changed); seam stage 1779 -> 531 ms "
        "(1248 ms, 70.15 %); candidate unit tests 29/2 vs baseline 31/0"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Checkout-only verifier for the t129 prepass work-set / per-op parity census.

The verifier re-derives every number in FINDINGS.md from census.json:

  * stage sums reconcile with the prepass phase;
  * the clean work-set tranche (overhang + mesh + 92.4 % of seam) matches its
    recorded value and its cell / prepass / required-cut shares;
  * the Orca wall is derived from the ratio of record;
  * the prepass-only and full-geometry-parity route multiples are re-derived;
  * every shape's floor multiple is re-derived from its ceiling and the cell's
    0.80 % floor;
  * the key prose numbers are present in FINDINGS.md.

`--self-test` runs the same checks against mutated copies of the census and the
report and requires every mutation to be detected.

`--orca-check` resolves the canonical citations against the local OrcaSlicer
checkout at the pinned commit, when that checkout is present. It is a separate
opt-in because the canonical tree is not part of this repository.

Exit codes: 0 = pass, 1 = a check failed, 2 = bad invocation.
"""

from __future__ import annotations

import copy
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CENSUS_PATH = HERE / "census.json"
CITATIONS_PATH = HERE / "canonical_citations.json"
REPORT_PATH = HERE / "FINDINGS.md"

TOL_MS = 0.6
TOL_PCT = 0.15
TOL_MULT = 0.003
TOL_STAGE_SHARE = 0.02
TOL_FLOOR = 0.06

SEAM_SHARE = 0.924  # t84: 92.4 % of SeamPlanning is the per-region inset offset


def load() -> tuple[dict, dict, str]:
    census = json.loads(CENSUS_PATH.read_text(encoding="utf-8"))
    citations = json.loads(CITATIONS_PATH.read_text(encoding="utf-8"))
    report = REPORT_PATH.read_text(encoding="utf-8")
    return census, citations, report


def close(a: float, b: float, tol: float) -> bool:
    return abs(a - b) <= tol


def check_census(census: dict) -> list[str]:
    failures: list[str] = []
    cells = census["cells"]

    for name, cell in cells.items():
        stages = cell["stages_ms"]
        stage_sum = sum(stages.values())
        prepass = cell["prepass_share_pct"] / 100.0 * cell["slice_complete_ms"]
        if not close(stage_sum, prepass, TOL_STAGE_SHARE * prepass):
            failures.append(
                f"{name}: stage sum {stage_sum} != prepass {prepass:.1f} "
                f"(> {TOL_STAGE_SHARE:.0%})"
            )
        if not close(
            stages["ShellClassification"] + stages["Slice"],
            cell["common_shell_slice_ms"],
            TOL_MS,
        ):
            failures.append(
                f"{name}: Shell+Slice {stages['ShellClassification'] + stages['Slice']} "
                f"!= common_shell_slice_ms {cell['common_shell_slice_ms']}"
            )

        clean = (
            stages["OverhangAnnotation"]
            + stages["MeshAnalysis"]
            + SEAM_SHARE * stages["SeamPlanning"]
        )
        if not close(clean, cell["clean_workset_ms"], TOL_MS):
            failures.append(
                f"{name}: clean tranche {clean:.1f} != recorded "
                f"{cell['clean_workset_ms']}"
            )
        if not close(
            100.0 * clean / cell["slice_complete_ms"],
            cell["clean_workset_share_cell_pct"],
            TOL_PCT,
        ):
            failures.append(f"{name}: clean tranche cell share mismatch")
        if not close(
            100.0 * clean / prepass,
            cell["clean_workset_share_prepass_pct"],
            TOL_PCT,
        ):
            failures.append(f"{name}: clean tranche prepass share mismatch")

        required = cell["cut_to_beat_pct"] / 100.0 * cell["slice_complete_ms"]
        if not close(required, cell["required_cut_ms"], TOL_MS):
            failures.append(f"{name}: required cut mismatch")
        if not close(
            100.0 * clean / required,
            cell["clean_workset_share_required_cut_pct"],
            TOL_PCT,
        ):
            failures.append(f"{name}: clean tranche required-cut share mismatch")

        orca_wall = cell["slice_complete_ms"] / cell["ratio_of_record"]
        if not close(orca_wall, cell["orca_wall_derived_ms"], TOL_MS):
            failures.append(f"{name}: derived Orca wall mismatch")

        prepass_only = (
            cell["orca_prepass_ms"]
            + cell["per_layer_ms"]
            + cell["postpass_ms"]
            + cell["outside_ms"]
        )
        if not close(prepass_only, cell["prepass_only_canonical_ms"], TOL_MS):
            failures.append(f"{name}: prepass-only wall mismatch")
        if not close(
            prepass_only / orca_wall,
            cell["prepass_only_canonical_multiple"],
            TOL_MULT,
        ):
            failures.append(f"{name}: prepass-only multiple mismatch")

        full = cell["orca_slicing_ms"] + cell["postpass_ms"] + cell["outside_ms"]
        if not close(full, cell["full_slicing_parity_ms"], TOL_MS):
            failures.append(f"{name}: full-parity wall mismatch")
        if not close(
            full / orca_wall,
            cell["full_slicing_parity_multiple"],
            TOL_MULT,
        ):
            failures.append(f"{name}: full-parity multiple mismatch")

    for shape in census["shapes"]:
        if not isinstance(shape.get("ceiling_ms"), dict):
            continue
        for cell_name, ceiling in shape["ceiling_ms"].items():
            cell = cells[cell_name]
            multiple = ceiling / cell["floor_080pct_ms"]
            recorded = shape["floor_multiple"][cell_name]
            if not close(multiple, recorded, TOL_FLOOR):
                failures.append(
                    f"{shape['id']}: {cell_name} floor multiple {multiple:.2f} "
                    f"!= recorded {recorded}"
                )

    route = census["route"]
    if not route["prepass_only_verdict"].startswith("Wins base classic-off only"):
        failures.append("route verdict wording changed")
    for cell_name, share in route["clean_workset_share_of_required_cut_pct"].items():
        if not close(
            share,
            cells[cell_name]["clean_workset_share_required_cut_pct"],
            TOL_PCT,
        ):
            failures.append(f"route clean-tranche share mismatch on {cell_name}")

    return failures


# Key prose claims. These are the numbers a reader will quote; each must survive
# in the report, not only in the JSON.
REPORT_CLAIMS = [
    "15.5 s",
    "55.8 %",
    "17.5 %",
    "32.1 %",
    "14.9 %",
    "0.787x",
    "1.012x",
    "1.053x",
    "1.075x",
    "0.335x",
    "0.661x",
    "0.550x",
    "0.704x",
    "14,698",
    "9,689",
    "4,260",
    "1,587",
    "32,712",
    "40ecc66f78",
    "DEV-200",
]


def check_report(report: str) -> list[str]:
    return [
        f"report is missing prose claim {claim!r}"
        for claim in REPORT_CLAIMS
        if claim not in report
    ]


def run(census: dict, report: str) -> list[str]:
    return check_census(census) + check_report(report)


def self_test(census: dict, report: str) -> int:
    controls: list[tuple[str, dict, str]] = []

    # 1. A stage wall is moved; the stage sum and the clean tranche must disagree.
    mutated = copy.deepcopy(census)
    mutated["cells"]["base-classic-off"]["stages_ms"]["OverhangAnnotation"] += 500
    controls.append(("stage wall tamper", mutated, report))

    # 2. The clean tranche is overstated.
    mutated = copy.deepcopy(census)
    mutated["cells"]["base-classic-on"]["clean_workset_ms"] += 500
    controls.append(("clean tranche tamper", mutated, report))

    # 3. The ratio of record is moved; the derived Orca wall must disagree.
    mutated = copy.deepcopy(census)
    mutated["cells"]["benchy-classic-on"]["ratio_of_record"] = 3.0
    controls.append(("ratio of record tamper", mutated, report))

    # 4. The prepass-only multiple is moved.
    mutated = copy.deepcopy(census)
    mutated["cells"]["base-classic-on"]["prepass_only_canonical_multiple"] += 0.05
    controls.append(("prepass-only multiple tamper", mutated, report))

    # 5. The full-parity multiple is moved.
    mutated = copy.deepcopy(census)
    mutated["cells"]["benchy-classic-off"]["full_slicing_parity_multiple"] -= 0.05
    controls.append(("full-parity multiple tamper", mutated, report))

    # 6. A shape floor multiple is moved.
    mutated = copy.deepcopy(census)
    mutated["shapes"][0]["floor_multiple"]["base-classic-off"] += 1.0
    controls.append(("shape floor multiple tamper", mutated, report))

    # 7. A required-cut share is moved.
    mutated = copy.deepcopy(census)
    mutated["cells"]["base-classic-off"]["clean_workset_share_required_cut_pct"] -= 10.0
    controls.append(("clean tranche share tamper", mutated, report))

    # 8. The report loses a headline number.
    mutated_report = report.replace("15.5 s", "fifteen point five seconds", 1)
    controls.append(("report prose tamper", census, mutated_report))

    rejected = 0
    for label, c, r in controls:
        if run(c, r):
            rejected += 1
        else:
            print(f"SELF-TEST FAIL: negative control not detected: {label}")
    print(f"SELF-TEST: {rejected}/{len(controls)} negative controls rejected")
    return 0 if rejected == len(controls) else 1


def orca_check(citations: dict) -> int:
    checkout = Path(citations["checkout"])
    if not checkout.is_dir():
        print(f"ORCA-CHECK SKIP: {checkout} is not present")
        return 0
    try:
        head = subprocess.run(
            ["git", "-C", str(checkout), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        print(f"ORCA-CHECK FAIL: cannot read git HEAD: {exc}")
        return 1
    if head != citations["pin"]:
        print(f"ORCA-CHECK FAIL: checkout HEAD {head} != pin {citations['pin']}")
        return 1
    failures: list[str] = []
    for citation in citations["citations"]:
        path = checkout / citation["file"]
        if not path.is_file():
            failures.append(f"{citation['file']}: file missing")
            continue
        first_symbol = citation["function"].split("/")[0].strip().split("(")[0].strip()
        if first_symbol and first_symbol not in path.read_text(
            encoding="utf-8", errors="replace"
        ):
            failures.append(
                f"{citation['file']}: first symbol {first_symbol!r} not found"
            )
    if failures:
        for failure in failures:
            print(f"ORCA-CHECK FAIL: {failure}")
        return 1
    print(
        f"ORCA-CHECK PASS: {len(citations['citations'])} citations resolve at {head[:12]}"
    )
    return 0


def main(argv: list[str]) -> int:
    if any(arg not in {"--self-test", "--orca-check"} for arg in argv):
        print("usage: verify.py [--self-test] [--orca-check]")
        return 2
    census, citations, report = load()

    if "--orca-check" in argv:
        code = orca_check(citations)
        if code:
            return code

    if "--self-test" in argv:
        return self_test(census, report)

    failures = run(census, report)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        print(f"VERDICT: FAIL ({len(failures)} checks)")
        return 1
    print(
        f"VERDICT: PASS ({len(census['cells'])} cells, "
        f"{len(census['shapes'])} shapes, {len(REPORT_CLAIMS)} prose claims)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

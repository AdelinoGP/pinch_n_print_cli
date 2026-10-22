#!/usr/bin/env python3
"""Checkout-only verifier for the ticket-20 consume-only-when-read take.

Reads the committed reduced evidence only:

- `qualification.json` - the 16-cell output-identity gate, which embeds each
  cell's sha256/bytes and re-states the preserved t58 reference;
- `../../evidence/t58-finalization-copies/qualification.json` - the preserved
  reference matrix the gate is checked against;
- `ab-reduction.json` - the paired ordinary + accelerated A/B reduction.

No raw capture tree, model fixture, snapshot tree or `.local-artifacts/` path is
required. Exit 0 = verified; exit 1 = a check or a negative control failed.
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
QUALIFICATION = HERE / "qualification.json"
AB = HERE / "ab-reduction.json"
REFERENCE = HERE.parent / "t58-finalization-copies" / "qualification.json"

GATE_CELLS = (
    "ordinary-benchy-classic-off",
    "ordinary-base-classic-off",
    "accelerated-benchy-classic-off",
    "accelerated-base-classic-off",
)
CPU_CEILING_PCT = 1.0
EXPECTED_CELLS = {
    f"{mode}-{fixture}-{gen}-{sup}"
    for mode in ("ordinary", "accelerated")
    for fixture in ("benchy", "base")
    for gen in ("classic", "arachne")
    for sup in ("off", "on")
}


def check(qualification: dict, ab: dict, reference: dict) -> list[str]:
    problems: list[str] = []

    refs = {cell["cell"]: cell for cell in reference.get("cells", [])}
    cells = {cell["cell"]: cell for cell in qualification.get("cells", [])}

    if set(cells) != EXPECTED_CELLS:
        problems.append(
            f"qualification cell set is {sorted(set(cells) ^ EXPECTED_CELLS)} away from the 16-cell matrix"
        )
    if qualification.get("pass") is not True:
        problems.append("qualification did not pass")

    for name in sorted(EXPECTED_CELLS & set(cells)):
        cell = cells[name]
        ref = refs.get(name)
        if ref is None:
            problems.append(f"{name}: no preserved t58 reference")
            continue
        if cell["sha256"] != ref["sha256"]:
            problems.append(
                f"{name}: sha {cell['sha256'][:8]} != reference {ref['sha256'][:8]}"
            )
        if int(cell["bytes"]) != int(ref["bytes"]):
            problems.append(f"{name}: bytes {cell['bytes']} != reference {ref['bytes']}")
        if cell["status"] != "ok" or cell["degraded"] not in ("False", False):
            problems.append(
                f"{name}: unclean status={cell['status']} degraded={cell['degraded']}"
            )
        types = cell.get("type_counts", {})
        for role in ("Outer wall", "Sparse infill", "Inner wall"):
            if role not in types:
                problems.append(f"{name}: vacuous output (missing {role})")

    if ab.get("verdict", "").split(" ")[0] != "PASS":
        problems.append(f"A/B verdict is {ab.get('verdict')!r}")
    ab_cells = ab.get("cells", {})
    for name in GATE_CELLS:
        cell = ab_cells.get(name)
        if cell is None or cell.get("status") != "reduced":
            problems.append(f"{name}: gate cell not reduced")
            continue
        if int(cell.get("retained_pairs", 0)) != 6:
            problems.append(f"{name}: retained pairs {cell.get('retained_pairs')} != 6")
        if float(cell.get("wall_delta_pct_paired_median", 999.0)) >= 0.0:
            problems.append(f"{name}: wall clause did not pass")
        if float(cell.get("cpu_delta_pct_paired_median", 999.0)) > CPU_CEILING_PCT:
            problems.append(f"{name}: CPU delta above {CPU_CEILING_PCT}%")
        for pair in cell.get("pairs", []):
            if not pair.get("sha_equal"):
                problems.append(f"{name}: pair {pair.get('pair')} outputs differ")
    return problems


def main() -> int:
    for path in (QUALIFICATION, AB, REFERENCE):
        if not path.exists():
            print(f"MISSING: {path}")
            return 1
    qualification = json.loads(QUALIFICATION.read_text(encoding="utf-8"))
    ab = json.loads(AB.read_text(encoding="utf-8"))
    reference = json.loads(REFERENCE.read_text(encoding="utf-8"))

    problems = check(qualification, ab, reference)
    if problems:
        print(f"FAIL ({len(problems)})")
        for problem in problems:
            print(" -", problem)
        return 1

    # Negative controls: each must be rejected by the same checker.
    controls: list[tuple[str, dict, dict]] = []
    mutated = copy.deepcopy(qualification)
    mutated["cells"][0]["sha256"] = "0" * 64
    controls.append(("flipped output sha", mutated, copy.deepcopy(ab)))

    mutated = copy.deepcopy(qualification)
    mutated["pass"] = False
    controls.append(("failed qualification flag", mutated, copy.deepcopy(ab)))

    mutated = copy.deepcopy(ab)
    mutated["cells"][GATE_CELLS[0]]["wall_delta_pct_paired_median"] = 0.1
    controls.append(("wall regression in a gate cell", copy.deepcopy(qualification), mutated))

    mutated = copy.deepcopy(ab)
    mutated["cells"][GATE_CELLS[0]]["cpu_delta_pct_paired_median"] = 1.5
    controls.append(("CPU above ceiling", copy.deepcopy(qualification), mutated))

    for label, q, a in controls:
        if not check(q, a, reference):
            print(f"NEGATIVE CONTROL NOT REJECTED: {label}")
            return 1

    gate = [ab["cells"][name] for name in GATE_CELLS]
    print(
        "verified: 16/16 cells byte-identical to the preserved references; "
        f"{len(gate)} gate cells with 6/6 retained pairs; "
        "wall "
        + " / ".join(f"{c['wall_delta_pct_paired_median']:.2f}%" for c in gate)
        + "; CPU "
        + " / ".join(f"{c['cpu_delta_pct_paired_median']:.2f}%" for c in gate)
        + f"; {len(controls)} negative controls rejected"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

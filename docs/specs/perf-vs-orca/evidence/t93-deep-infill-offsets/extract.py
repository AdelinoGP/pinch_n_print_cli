#!/usr/bin/env python3
"""Reduce this machine's t93 captures into the committed evidence rows.

Reads `.local-artifacts/perimeter-reference-preparation/t93-deep-infill-offsets/`
and writes, next to this script:

  proof-rows.json     16 cells x 2 arms (the qualification rows)
  measure-rows.json    4 cells x (2 warmups + 12 measured) rows
  provenance.json      per-arm host binary SHA and module-tree equality

`reduce.py` derives `qualification.json` / `ab-reduction.json` / `summary.json`
from those committed rows, so `verify.py` can reproduce every headline claim in
a plain checkout.
"""

from __future__ import annotations

import json
import sys

import reduce

HERE = reduce.HERE
ROOT = reduce.ROOT


def build_provenance() -> dict:
    """Snapshot provenance: per-arm binary SHA and module-tree equality.

    The candidate is host-only, so both arms must ship byte-identical guest
    modules; only `pnp_cli.exe` differs. This is what pins the isolation claim
    in the findings without shipping 24 wasm files.
    """
    prov_dir = ROOT / "provenance"
    entries = {}
    for path in sorted(prov_dir.glob("*.json")) if prov_dir.is_dir() else []:
        data = json.loads(path.read_text(encoding="utf-8"))
        # PowerShell's ConvertTo-Json collapses an empty `@()` to null and a
        # one-element array to a scalar; normalise to a list of strings.
        dirty = data.get("candidate_sources_dirty")
        if dirty is None:
            dirty = []
        elif isinstance(dirty, str):
            dirty = [dirty]
        data["candidate_sources_dirty"] = dirty
        entries[f"{data['mode']}-{data['arm']}"] = data
    module_trees_equal = {}
    for mode in ("ordinary", "accelerated"):
        base = entries.get(f"{mode}-baseline")
        cand = entries.get(f"{mode}-candidate")
        if base is None or cand is None:
            continue
        module_trees_equal[mode] = base["modules"] == cand["modules"]
    return {
        "note": (
            "Built by build-snapshots.ps1. `modules` is the sorted per-file "
            "SHA256 list of the snapshot's external module tree; "
            "`candidate_sources_dirty` records which candidate sources were "
            "stashed for the baseline arm, so the arm diff is auditable."
        ),
        "snapshots": entries,
        "module_trees_equal": module_trees_equal,
    }


def main() -> int:
    proof_rows = reduce.load_raw_proof()
    measure_rows = reduce.load_raw_measure()
    missing_proof = [cell for cell, rows in proof_rows.items() if len(rows) != 2]
    incomplete_measure = [
        cell
        for cell, rows in measure_rows.items()
        if len(rows) != 2 + 2 * reduce.PAIRS_REQUIRED
    ]
    if missing_proof:
        print("proof cells incomplete:", missing_proof)
    if incomplete_measure:
        print("measure cells incomplete:", incomplete_measure)

    (HERE / "proof-rows.json").write_text(
        json.dumps(proof_rows, indent=1), encoding="utf-8"
    )
    (HERE / "measure-rows.json").write_text(
        json.dumps(measure_rows, indent=1), encoding="utf-8"
    )
    (HERE / "provenance.json").write_text(
        json.dumps(build_provenance(), indent=1), encoding="utf-8"
    )
    print(
        f"proof cells {sum(1 for rows in proof_rows.values() if len(rows) == 2)}/16, "
        f"measure cells "
        f"{sum(1 for rows in measure_rows.values() if len(rows) == 2 + 2 * reduce.PAIRS_REQUIRED)}/4"
    )
    return 0 if not missing_proof and not incomplete_measure else 1


if __name__ == "__main__":
    raise SystemExit(main())

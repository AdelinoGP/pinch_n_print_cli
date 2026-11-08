#!/usr/bin/env python3
"""t67 preserved-reference verifier.

Cross-checks the t67 16-cell proof hashes (qualification.json, both arms)
against the preserved t58 reference set
(evidence/t58-finalization-copies/qualification.json) for every matched
cell in both modes, and re-asserts the ticket's identity claim independently
of the per-cell baseline-vs-candidate comparison. Exit code mirrors the
result.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]


def main() -> int:
    q66 = json.loads((HERE / "qualification.json").read_text(encoding="utf-8"))
    q58 = json.loads(
        (
            REPO
            / "docs/specs/perf-vs-orca/evidence/t58-finalization-copies/qualification.json"
        ).read_text(encoding="utf-8")
    )
    refs = {c["cell"]: c["sha256"] for c in q58["cells"]}
    problems = []
    checked = 0
    for c in q66["cells"]:
        cell = c["cell"]
        if cell not in refs:
            problems.append(f"{cell}: no preserved reference")
            continue
        if c["sha256"] != refs[cell]:
            problems.append(
                f"{cell}: {c['sha256'][:12]} != preserved {refs[cell][:12]}"
            )
        checked += 1
    passed = not problems and checked == 16
    print(f"reference-checked cells: {checked}/16; pass={passed}")
    for p in problems:
        print("PROBLEM:", p)
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())

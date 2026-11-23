#!/usr/bin/env python3
"""t79 snapshot provenance: confirm the two arms isolate the planner guest.

Reads the four snapshot trees under `.local-artifacts/.../t79-emit-batch-sweep/
snapshots/` and prints, per mode, the files that differ between the baseline
and candidate arms. The expected shape is: the `pnp_cli.exe` binaries are
byte-identical, every module file is byte-identical except
`tree-support-planner`, and the two arms' hashes are recorded for the evidence.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def repo_root() -> Path:
    for ancestor in [HERE, *HERE.parents]:
        if (ancestor / ".local-artifacts").is_dir():
            return ancestor
    raise SystemExit("could not locate repo root (.local-artifacts)")


ROOT = (
    repo_root()
    / ".local-artifacts"
    / "perimeter-reference-preparation"
    / "t79-emit-batch-sweep"
    / "snapshots"
)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree(root: Path) -> dict[str, str]:
    return {
        str(path.relative_to(root)).replace("\\", "/"): digest(path)
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def main() -> int:
    result = {}
    problems = []
    for mode in ("ordinary", "accelerated"):
        base = tree(ROOT / f"{mode}-baseline")
        cand = tree(ROOT / f"{mode}-candidate")
        if set(base) != set(cand):
            problems.append(f"{mode}: snapshot file sets differ")
        differing = sorted(name for name in base if base.get(name) != cand.get(name))
        executable_equal = base.get("pnp_cli.exe") == cand.get("pnp_cli.exe")
        module_diff = [name for name in differing if name.startswith("modules/")]
        result[mode] = {
            "files": len(base),
            "differing": differing,
            "module_differing": module_diff,
            "executable_equal": executable_equal,
            "baseline_planner_sha256": base.get(
                "modules/tree-support-planner/tree-support-planner.wasm"
            ),
            "candidate_planner_sha256": cand.get(
                "modules/tree-support-planner/tree-support-planner.wasm"
            ),
        }
        if not executable_equal:
            problems.append(f"{mode}: pnp_cli.exe differs between arms")
        if module_diff != ["modules/tree-support-planner/tree-support-planner.wasm"]:
            problems.append(f"{mode}: module diff is {module_diff}")
    out = {"pass": not problems, "modes": result, "problems": problems}
    (HERE / "snapshots.json").write_text(json.dumps(out, indent=1), encoding="utf-8")
    for mode, record in result.items():
        print(f"{mode}: files={record['files']} differing={record['differing']}")
    for problem in problems:
        print("PROBLEM:", problem)
    return 0 if not problems else 1


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""t109 harness-defect take -- verifier.

Re-derives the two claims in FINDINGS.md:

  A1  `xtask test` injects `slicer-core/host-algos` only when cargo's selection
      includes slicer-core, so `-p <foreign-package>` no longer fails with
      "does not contain this feature".
  A2  the AGENTS.md narrow-run example names the scheduler's real test target
      `scheduler_contract` and matches 9 tests, not the runtime's contract
      bucket with 0 matches.

  python verify.py              # static + narrow cargo checks
  python verify.py --no-cargo   # static checks only (no build)
  python verify.py --full       # also run the end-to-end `cargo xtask test -p xtask`
  python verify.py --self-test  # negative controls over the committed summary
"""

from __future__ import annotations

import copy
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
AGENTS = REPO / "AGENTS.md"
HARNESS = REPO / "xtask" / "src" / "test.rs"
SCHEDULER_MANIFEST = REPO / "crates" / "slicer-scheduler" / "Cargo.toml"
SCHEDULER_CONTRACT_MAIN = (
    REPO / "crates" / "slicer-scheduler" / "tests" / "contract" / "main.rs"
)

Check = tuple[str, bool, str]


def load() -> dict:
    return json.loads((HERE / "summary.json").read_text(encoding="utf-8"))


def check_static(data: dict) -> list[Check]:
    out: list[Check] = []
    a1 = data["a1"]
    a2 = data["a2"]

    out.append(("ticket id", data["ticket"] == 109, f"ticket={data['ticket']}"))

    agents_line = next(
        (
            line
            for line in AGENTS.read_text(encoding="utf-8").splitlines()
            if a2["module"] in line
        ),
        None,
    )
    agents_ok = agents_line is not None
    if agents_line is not None:
        for token in a2["after_command"].split():
            if token in ("cargo", "test"):
                continue
            agents_ok = agents_ok and token in agents_line
    out.append(
        (
            "A2 AGENTS.md example",
            agents_ok,
            (agents_line or "no line naming the module").strip(),
        )
    )

    manifest = SCHEDULER_MANIFEST.read_text(encoding="utf-8")
    target_ok = f'name = "{a2["target"]}"' in manifest and (
        'path = "tests/contract/main.rs"' in manifest
    )
    out.append(
        (
            "A2 scheduler target",
            target_ok,
            f"target={a2['target']}",
        )
    )

    contract_main = SCHEDULER_CONTRACT_MAIN.read_text(encoding="utf-8")
    module_ok = f"mod {a2['module']};" in contract_main
    out.append(("A2 module declaration", module_ok, a2["module"]))

    harness = HARNESS.read_text(encoding="utf-8")
    symbols = [
        "fn build_test_arguments",
        "fn selection_includes_slicer_core",
        "fn names_slicer_core",
        "fn manifest_declares_package",
    ]
    missing = [symbol for symbol in symbols if symbol not in harness]
    out.append(("A1 harness symbols", not missing, ", ".join(missing) or "all present"))
    feature_ok = f'"{a1["slicer_core_features_arg"]}"' in harness
    out.append(("A1 feature literal", feature_ok, a1["slicer_core_features_arg"]))

    out.append(
        (
            "A1 before failed",
            a1["before_exit"] != 0,
            f"exit={a1['before_exit']}",
        )
    )
    out.append(("A1 after passed", a1["after_exit"] == 0, f"exit={a1['after_exit']}"))
    out.append(
        (
            "A1 after test count",
            a1["after_passed"] == data["unit_tests"]["total"]
            and a1["after_failed"] == 0,
            f"{a1['after_passed']} passed, {a1['after_failed']} failed",
        )
    )

    out.append(
        (
            "A2 before ran nothing",
            a2["before_passed"] == 0 and a2["before_filtered_out"] > 0,
            f"{a2['before_passed']} passed, {a2['before_filtered_out']} filtered",
        )
    )
    out.append(
        (
            "A2 after ran the module",
            a2["after_passed"] == 9
            and a2["after_failed"] == 0
            and a2["after_filtered_out"] > 0,
            f"{a2['after_passed']} passed, {a2['after_failed']} failed, "
            f"{a2['after_filtered_out']} filtered",
        )
    )

    census = data["adjacent"]["slicer_config_census"]
    out.append(
        (
            "adjacent census red is real",
            census["actual"] > census["expected"] and census["pre_existing"],
            f"actual={census['actual']} expected={census['expected']}",
        )
    )

    for name in data["unit_tests"]["names"]:
        out.append((f"unit test named {name}", name in harness, name))

    return out


def run_cargo(args: list[str]) -> tuple[int, str]:
    result = subprocess.run(
        ["cargo", *args],
        cwd=REPO,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return result.returncode, (result.stdout or "") + (result.stderr or "")


def check_cargo(data: dict, full: bool) -> list[Check]:
    out: list[Check] = []
    a1 = data["a1"]
    a2 = data["a2"]

    code, output = run_cargo(
        ["test", "-p", "xtask", "host_algos_selection_follows_the_package_selector"]
    )
    out.append(
        (
            "cargo: selector unit test",
            code == 0 and "1 passed" in output and "0 failed" in output,
            f"exit={code}",
        )
    )

    code, output = run_cargo(
        [
            "test",
            "-p",
            "slicer-scheduler",
            "--test",
            a2["target"],
            a2["module"],
            "--",
            "--list",
        ]
    )
    matched = sum(1 for line in output.splitlines() if f"{a2['module']}::" in line)
    out.append(
        (
            "cargo: documented narrow command matches 9 tests",
            code == 0
            and matched == a2["after_passed"]
            and f"{a2['after_passed']} tests, 0 benchmarks" in output,
            f"exit={code} matched={matched}",
        )
    )

    if full:
        code, output = run_cargo(["xtask", "test", "-p", "xtask", "--summary"])
        out.append(
            (
                "cargo: end-to-end xtask test -p xtask",
                code == 0
                and "VERDICT: PASS" in output
                and f"{a1['after_passed']} passed; {a1['after_failed']} failed"
                in output,
                f"exit={code}",
            )
        )

    return out


def report(results: list[Check]) -> bool:
    ok = True
    for name, passed, detail in results:
        ok = ok and passed
        print(f"[{'PASS' if passed else 'FAIL'}] {name}: {detail}")
    return ok


def self_test(data: dict) -> bool:
    mutations = [
        ("after exit flipped", lambda d: d["a1"].__setitem__("after_exit", 101)),
        (
            "after passed count flipped",
            lambda d: d["a2"].__setitem__("after_passed", 8),
        ),
        ("target renamed", lambda d: d["a2"].__setitem__("target", "contract")),
        ("before matched a test", lambda d: d["a2"].__setitem__("before_passed", 1)),
        (
            "census declared pre-existing false",
            lambda d: d["adjacent"]["slicer_config_census"].__setitem__(
                "pre_existing", False
            ),
        ),
    ]
    ok = True
    for name, mutate in mutations:
        mutated = copy.deepcopy(data)
        mutate(mutated)
        rejected = not all(passed for _, passed, _ in check_static(mutated))
        ok = ok and rejected
        print(f"[{'PASS' if rejected else 'FAIL'}] negative control rejected: {name}")
    return ok


def main(argv: list[str]) -> int:
    data = load()
    if "--self-test" in argv:
        return 0 if self_test(data) else 1

    results = check_static(data)
    if "--no-cargo" not in argv:
        results += check_cargo(data, full="--full" in argv)

    ok = report(results)
    print()
    print("PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

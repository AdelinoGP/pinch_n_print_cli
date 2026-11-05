#!/usr/bin/env python3
"""Verify the t119 lightning-corpus consolidation.

The claim is threefold and each part is re-derived, never trusted from prose:

* the module's active invocation budget (41 before, 32 after) comes from the
  tracked pipeline-entry call sites in the before and after test sources,
  multiplied by the committed baseline's case count and by the number of
  `consumer_failure` calls in the failure test;
* the after run's execution trace (one `lightning-invocation:` line per real
  pipeline start) must agree with that inventory entry by entry;
* the before/after libtest summaries must agree with the walls recorded in
  `FINDINGS.md`, with the same 9-passed / 1-ignored roster on both arms.

The before source is read from the recorded `base-commit` with `git show`, so
the 41-side is re-derived from history rather than from a hand-kept table.
`--self-test` mutates a copy of these inputs and requires every control to be
rejected.
"""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass, replace
import json
from pathlib import Path
import re
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
SOURCE_REL = Path("crates/slicer-runtime/tests/e2e/lightning_migration_tdd.rs")
BASELINE_REL = Path(
    "crates/slicer-runtime/tests/fixtures/lightning_baseline/baseline_classic.json"
)
FINDINGS_REL = HERE / "FINDINGS.md"
BEFORE_LOG_REL = HERE / "before.log"
AFTER_LOG_REL = HERE / "after.log"

TRACKED = (
    "run_visual",
    "run_visual_native",
    "run_slice_preparation_capture",
    "run_slice_with_runtime_extensions",
)
BEFORE_BUDGET = 41
AFTER_BUDGET = 32
CONSUMER_FAILURES = 3

# (caller, callee) -> (textual call sites, multiplier)
# multiplier: "cases" reads the committed baseline case count; "consumer" is
# the number of `consumer_failure` calls in the failure test; 1 is literal.
BEFORE_TABLE = {
    ("module_plan_matches_recorded_baseline", "run_visual"): (1, "cases"),
    ("module_plan_matches_recorded_baseline", "run_visual_native"): (1, "cases"),
    ("fresh_consumers_emit_baseline_segments", "run_slice_preparation_capture"): (
        1,
        "cases",
    ),
    ("fresh_consumers_emit_baseline_segments", "run_visual"): (1, "cases"),
    ("fresh_consumers_emit_baseline_segments", "run_slice_with_runtime_extensions"): (
        1,
        "cases",
    ),
    ("typed_projection_is_atomic_and_neutral", "run_visual"): (4, 1),
    ("typed_projection_is_atomic_and_neutral", "run_slice_with_runtime_extensions"): (
        2,
        1,
    ),
    ("legacy_contract_is_retired", "run_visual"): (1, 1),
    ("equivalence_rejects_corrupted_baseline", "run_visual"): (1, 1),
    ("failed_or_malformed_plan_never_falls_back", "run_slice_preparation_capture"): (
        1,
        1,
    ),
    ("consumer_failure", "run_visual"): (1, "consumer"),
    ("selection_override_empty_and_foreign_owner", "run_visual"): (2, 1),
    (
        "selection_override_empty_and_foreign_owner",
        "run_slice_with_runtime_extensions",
    ): (1, 1),
    ("retirement_leaves_no_host_fallback", "run_visual"): (1, 1),
}

AFTER_TABLE = {
    ("observe_wasm", "run_visual"): (1, "cases"),
    ("observe_native", "run_visual_native"): (1, "cases"),
    ("fresh_consumers_emit_baseline_segments", "run_slice_preparation_capture"): (
        1,
        "cases",
    ),
    ("fresh_consumers_emit_baseline_segments", "run_slice_with_runtime_extensions"): (
        1,
        "cases",
    ),
    ("typed_projection_is_atomic_and_neutral", "run_visual"): (4, 1),
    ("typed_projection_is_atomic_and_neutral", "run_slice_with_runtime_extensions"): (
        2,
        1,
    ),
    ("failed_or_malformed_plan_never_falls_back", "run_slice_preparation_capture"): (
        1,
        1,
    ),
    ("consumer_failure", "run_visual"): (1, "consumer"),
    ("selection_override_empty_and_foreign_owner", "run_visual"): (1, 1),
    (
        "selection_override_empty_and_foreign_owner",
        "run_slice_with_runtime_extensions",
    ): (1, 1),
}

FUNC_START = re.compile(r"^fn ([A-Za-z0-9_]+)\s*\(")
TEST_LINE = re.compile(
    r"^test lightning_migration_tdd::(\w+) \.\.\. (ok|ignored)(?:,.*)?$", re.M
)
SUMMARY_LINE = re.compile(
    r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
    r"\d+ measured; \d+ filtered out; finished in ([\d.]+)s$",
    re.M,
)
INVOCATION_LINE = re.compile(r"^lightning-invocation: (\w+)$", re.M)
MEASUREMENT_BLOCK = re.compile(r"<!-- t119-measurement\n(.*?)-->", re.S)
MEASUREMENT_KEY = re.compile(r"^([a-z-]+): (.+)$", re.M)


class VerificationError(Exception):
    pass


@dataclass(frozen=True)
class Function:
    body: str
    ignored: bool


def locate_functions(source: str) -> dict[str, tuple[int, int]]:
    """Top-level `fn` ranges. Rust top-level bodies close on a column-0 `}`."""
    lines = source.splitlines()
    ranges: dict[str, tuple[int, int]] = {}
    current: tuple[str, int] | None = None
    for index, line in enumerate(lines):
        match = FUNC_START.match(line)
        if match and current is None:
            current = (match.group(1), index)
        elif current is not None and line == "}":
            ranges[current[0]] = (current[1], index)
            current = None
    if current is not None:
        raise VerificationError(f"unterminated top-level function {current[0]}")
    return ranges


def parse_functions(source: str) -> dict[str, Function]:
    lines = source.splitlines()
    functions: dict[str, Function] = {}
    for name, (start, end) in locate_functions(source).items():
        attributes: list[str] = []
        cursor = start - 1
        while cursor >= 0 and (
            lines[cursor].strip().startswith("#[") or not lines[cursor].strip()
        ):
            attributes.append(lines[cursor].strip())
            cursor -= 1
        functions[name] = Function(
            body="\n".join(lines[start + 1 : end]),
            ignored=any(attribute.startswith("#[ignore") for attribute in attributes),
        )
    return functions


def count_calls(body: str, callee: str) -> int:
    return len(re.findall(r"(?<![A-Za-z0-9_])" + re.escape(callee) + r"\s*\(", body))


def inventory(
    source: str, table: dict[tuple[str, str], tuple[int, object]], label: str
) -> tuple[int, Counter[str]]:
    functions = parse_functions(source)
    actual: dict[str, dict[str, int]] = {}
    for name, function in functions.items():
        if function.ignored:
            continue
        counts = {callee: count_calls(function.body, callee) for callee in TRACKED}
        counts = {callee: count for callee, count in counts.items() if count}
        if counts:
            actual[name] = counts

    expected: dict[str, dict[str, int]] = {}
    for (caller, callee), (sites, _) in table.items():
        expected.setdefault(caller, {})[callee] = sites

    if actual != expected:
        raise VerificationError(
            f"{label}: tracked call-site inventory differs from the committed table\n"
            f"  expected {expected}\n"
            f"  actual   {actual}"
        )

    failure_test = functions.get("failed_or_malformed_plan_never_falls_back")
    if failure_test is None:
        raise VerificationError(f"{label}: failure test is absent")
    consumer_calls = count_calls(failure_test.body, "consumer_failure")
    if consumer_calls != CONSUMER_FAILURES:
        raise VerificationError(
            f"{label}: expected {CONSUMER_FAILURES} consumer_failure calls, "
            f"found {consumer_calls}"
        )

    total = 0
    per_entry: Counter[str] = Counter()
    for (caller, callee), (sites, multiplier) in table.items():
        if multiplier == "cases":
            factor = case_count
        elif multiplier == "consumer":
            factor = consumer_calls
        elif multiplier == 1:
            factor = 1
        else:
            raise VerificationError(f"{label}: unknown multiplier {multiplier!r}")
        total += sites * factor
        per_entry[callee] += sites * factor
        del caller
    return total, per_entry


def parse_log(text: str, label: str) -> tuple[dict[str, str], float, Counter[str]]:
    summaries = SUMMARY_LINE.findall(text)
    if len(summaries) != 1:
        raise VerificationError(f"{label}: expected exactly one libtest summary")
    state, passed, failed, ignored, seconds = summaries[0]
    if state != "ok":
        raise VerificationError(f"{label}: libtest summary is {state}")
    if (int(passed), int(failed), int(ignored)) != (9, 0, 1):
        raise VerificationError(
            f"{label}: expected 9 passed / 0 failed / 1 ignored, got "
            f"{passed}/{failed}/{ignored}"
        )
    roster = dict(TEST_LINE.findall(text))
    if len(roster) != 10:
        raise VerificationError(f"{label}: expected 10 test lines, got {len(roster)}")
    invocations = Counter(INVOCATION_LINE.findall(text))
    return roster, float(seconds), invocations


def parse_measurement(text: str) -> dict[str, str]:
    match = MEASUREMENT_BLOCK.search(text)
    if not match:
        raise VerificationError("FINDINGS.md has no t119-measurement block")
    pairs = dict(MEASUREMENT_KEY.findall(match.group(1)))
    required = {
        "base-commit",
        "before-wall-s",
        "after-wall-s",
        "active-invocations",
    }
    missing = required - set(pairs)
    if missing:
        raise VerificationError(
            f"FINDINGS.md measurement block misses {sorted(missing)}"
        )
    return pairs


def check(
    case_count: int,
    before_source: str,
    after_source: str,
    before_log: str,
    after_log: str,
    findings: str,
) -> list[str]:
    problems: list[str] = []

    def record(label: str, action) -> None:
        try:
            action()
        except VerificationError as error:
            problems.append(f"{label}: {error}")

    before_total = before_per_entry = None
    after_total = after_per_entry = None

    def before_inventory() -> None:
        nonlocal before_total, before_per_entry
        before_total, before_per_entry = inventory(
            before_source, BEFORE_TABLE, "before"
        )
        if before_total != BEFORE_BUDGET:
            raise VerificationError(
                f"before budget is {before_total}, expected {BEFORE_BUDGET}"
            )

    def after_inventory() -> None:
        nonlocal after_total, after_per_entry
        after_total, after_per_entry = inventory(after_source, AFTER_TABLE, "after")
        if after_total != AFTER_BUDGET:
            raise VerificationError(
                f"after budget is {after_total}, expected {AFTER_BUDGET}"
            )

    record("before inventory", before_inventory)
    record("after inventory", after_inventory)

    before_parsed = after_parsed = None

    def before_log_check() -> None:
        nonlocal before_parsed
        before_parsed = parse_log(before_log, "before log")
        if before_parsed[2]:
            raise VerificationError(
                "before log carries invocation lines; the pre-change arm had no trace"
            )

    def after_log_check() -> None:
        nonlocal after_parsed
        after_parsed = parse_log(after_log, "after log")
        if sum(after_parsed[2].values()) != AFTER_BUDGET:
            raise VerificationError(
                f"after log carries {sum(after_parsed[2].values())} invocation lines, "
                f"expected {AFTER_BUDGET}"
            )
        if after_per_entry is not None and after_parsed[2] != after_per_entry:
            raise VerificationError(
                f"after log trace disagrees with the source inventory\n"
                f"  source {dict(after_per_entry)}\n"
                f"  trace  {dict(after_parsed[2])}"
            )

    record("before log", before_log_check)
    record("after log", after_log_check)

    def rosters() -> None:
        if before_parsed is None or after_parsed is None:
            raise VerificationError("one log did not parse")
        if before_parsed[0] != after_parsed[0]:
            raise VerificationError("before and after test rosters differ")
        if Counter(before_parsed[0].values()) != Counter({"ok": 9, "ignored": 1}):
            raise VerificationError("roster is not 9 ok + 1 ignored")

    record("roster", rosters)

    def measurement() -> None:
        if before_parsed is None or after_parsed is None:
            raise VerificationError("one log did not parse")
        data = parse_measurement(findings)
        if not re.fullmatch(r"[0-9a-f]{40}", data["base-commit"]):
            raise VerificationError("base-commit is not a full SHA-1")
        if abs(float(data["before-wall-s"]) - before_parsed[1]) > 0.005:
            raise VerificationError(
                f"recorded before wall {data['before-wall-s']} != log {before_parsed[1]}"
            )
        if abs(float(data["after-wall-s"]) - after_parsed[1]) > 0.005:
            raise VerificationError(
                f"recorded after wall {data['after-wall-s']} != log {after_parsed[1]}"
            )
        if int(data["active-invocations"]) != AFTER_BUDGET:
            raise VerificationError(
                f"recorded active-invocations {data['active-invocations']} != "
                f"{AFTER_BUDGET}"
            )
        if after_parsed[1] >= before_parsed[1]:
            raise VerificationError(
                f"after wall {after_parsed[1]} is not below before wall "
                f"{before_parsed[1]}"
            )

    record("measurement", measurement)
    return problems


def read_inputs(root: Path, before_source: str | None) -> dict:
    after_source = (root / SOURCE_REL).read_text(encoding="utf-8")
    baseline = json.loads((root / BASELINE_REL).read_text(encoding="utf-8"))
    findings = (root / FINDINGS_REL).read_text(encoding="utf-8")
    if before_source is None:
        base_commit = parse_measurement(findings)["base-commit"]
        shown = subprocess.run(
            ["git", "-C", str(root), "show", f"{base_commit}:{SOURCE_REL.as_posix()}"],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        if shown.returncode != 0:
            raise VerificationError(
                f"cannot read {SOURCE_REL} at base commit {base_commit}: "
                f"{shown.stderr.strip()}"
            )
        before_source = shown.stdout
    return {
        "case_count": len(baseline["cases"]),
        "before_source": before_source,
        "after_source": after_source,
        "before_log": (root / BEFORE_LOG_REL).read_text(encoding="utf-8"),
        "after_log": (root / AFTER_LOG_REL).read_text(encoding="utf-8"),
        "findings": findings,
    }


def insert_in_function(source: str, function: str, text: str) -> str:
    start, end = locate_functions(source)[function]
    lines = source.splitlines()
    lines.insert(end, text)
    return "\n".join(lines) + "\n"


def replace_in_function(source: str, function: str, old: str, new: str) -> str:
    start, end = locate_functions(source)[function]
    lines = source.splitlines()
    body = "\n".join(lines[start:end])
    if old not in body:
        raise VerificationError(f"{function}: {old!r} not found")
    body = body.replace(old, new, 1)
    return "\n".join(lines[:start] + body.splitlines() + lines[end:])


def self_test(inputs: dict) -> int:
    after_log_lines = inputs["after_log"].splitlines()
    first_invocation = next(
        line for line in after_log_lines if line.startswith("lightning-invocation: ")
    )
    no_invocation_line = "\n".join(
        line for line in after_log_lines if line != first_invocation
    )

    controls = [
        (
            "after source gains an unlisted call site",
            lambda data: {
                **data,
                "after_source": insert_in_function(
                    data["after_source"],
                    "module_plan_matches_recorded_baseline",
                    "    let _unlisted = run_visual(todo!());",
                ),
            },
        ),
        (
            "after source loses a typed-projection call site",
            lambda data: {
                **data,
                "after_source": replace_in_function(
                    data["after_source"],
                    "typed_projection_is_atomic_and_neutral",
                    "run_visual(",
                    "unrecorded_visual(",
                ),
            },
        ),
        (
            "baseline loses a case",
            lambda data: {**data, "case_count": data["case_count"] - 1},
        ),
        (
            "after log loses one invocation line",
            lambda data: {**data, "after_log": no_invocation_line},
        ),
        (
            "findings report a slower after wall than before",
            lambda data: {
                **data,
                "findings": data["findings"].replace(
                    "after-wall-s: 620.44", "after-wall-s: 700.00"
                ),
            },
        ),
        (
            "findings report a different active budget",
            lambda data: {
                **data,
                "findings": data["findings"].replace(
                    "active-invocations: 32", "active-invocations: 33"
                ),
            },
        ),
        (
            "before source loses a module-plan call site",
            lambda data: {
                **data,
                "before_source": replace_in_function(
                    data["before_source"],
                    "module_plan_matches_recorded_baseline",
                    "run_visual(",
                    "unrecorded_visual(",
                ),
            },
        ),
    ]

    for name, mutate in controls:
        try:
            problems = check(**mutate(inputs))
        except VerificationError as error:
            problems = [str(error)]
        if not problems:
            print(f"MISSED: {name}")
            return 1
        print(f"rejected: {name}")
    return 0


def main() -> int:
    root = ROOT
    before_source = None
    args = sys.argv[1:]
    while args:
        token = args.pop(0)
        if token == "--root":
            root = Path(args.pop(0)).resolve()
        elif token == "--before-source":
            before_source = Path(args.pop(0)).read_text(encoding="utf-8")
        else:
            print(f"unknown argument: {token}")
            return 2

    try:
        inputs = read_inputs(root, before_source)
    except (OSError, VerificationError) as error:
        print(f"FAIL: {error}")
        return 1

    problems = check(**inputs)
    if problems:
        for problem in problems:
            print(f"FAIL: {problem}")
        return 1

    after_total = sum(
        sites
        * (
            inputs["case_count"]
            if multiplier == "cases"
            else CONSUMER_FAILURES
            if multiplier == "consumer"
            else 1
        )
        for sites, multiplier in AFTER_TABLE.values()
    )
    before_total = sum(
        sites
        * (
            inputs["case_count"]
            if multiplier == "cases"
            else CONSUMER_FAILURES
            if multiplier == "consumer"
            else 1
        )
        for sites, multiplier in BEFORE_TABLE.values()
    )
    parsed = parse_log(inputs["after_log"], "after log")
    data = parse_measurement(inputs["findings"])
    print(
        "PASS: lightning corpus — "
        f"{before_total} -> {after_total} active invocations; "
        f"trace {dict(parsed[2])}; "
        f"{data['before-wall-s']} -> {data['after-wall-s']} s; "
        "9 passed / 1 ignored on both arms"
    )
    if "--self-test" in sys.argv:
        return self_test(inputs)
    return 0


if __name__ == "__main__":
    sys.exit(main())

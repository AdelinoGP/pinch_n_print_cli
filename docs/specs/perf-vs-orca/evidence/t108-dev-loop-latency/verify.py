#!/usr/bin/env python3
"""t108 dev-loop latency take -- verifier.

Re-derives every headline number in FINDINGS.md from the committed
`summary.json` and asserts the structural claims that the ticket rests on.
Exits 0 only if every claim holds.

  python verify.py              # verify the committed set (checkout-only)
  python verify.py --self-test  # negative controls: mutate the set, demand rejection

The measured arms are wall-clock samples, so the assertions are about
*relationships* (medians, ordering, non-overlap, floor) rather than exact
timings; the absolute values are pinned no tighter than 0.05 s.
"""

from __future__ import annotations

import copy
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
HARNESS = REPO / "xtask" / "src" / "test.rs"

# Pre-declared acceptance floor for this take: a lever must cut at least this
# much wall from a measured `xtask test` invocation. Chosen before the
# measurement, and roughly a third of the serial gate's own cost.
FLOOR_S = 5.0


def load() -> dict:
    return json.loads((HERE / "summary.json").read_text(encoding="utf-8"))


def check(b: dict) -> list[tuple[str, bool, str]]:
    out: list[tuple[str, bool, str]] = []

    def add(name: str, ok: bool, detail: str = "") -> None:
        out.append((name, bool(ok), detail))

    lever = b["lever"]
    before = lever["measured"]["before"]
    after = lever["measured"]["after"]

    add(
        "both arms carry four interleaved samples",
        len(before) == len(after) == lever["rounds"] == 4 and lever["interleaved"],
        f"before={len(before)} after={len(after)} rounds={lever['rounds']}",
    )
    add(
        "the take declares a pre-declared floor",
        abs(lever["floor_s"] - FLOOR_S) < 1e-9,
        f"{lever['floor_s']} vs {FLOOR_S}",
    )

    median_before = statistics.median(before)
    median_after = statistics.median(after)
    delta = median_before - median_after
    pct = 100.0 * delta / median_before

    add(
        "before median is 18.77 s",
        abs(median_before - 18.7705) < 0.05,
        f"{median_before:.4f}",
    )
    add(
        "after median is 10.80 s",
        abs(median_after - 10.8015) < 0.05,
        f"{median_after:.4f}",
    )
    add(
        "median delta is 7.97 s",
        abs(delta - 7.969) < 0.05,
        f"{delta:.4f}",
    )
    add(
        "median cut is 42.4 %",
        abs(pct - 42.45) < 0.2,
        f"{pct:.2f} %",
    )
    add(
        "the delta clears the pre-declared floor",
        delta >= lever["floor_s"],
        f"{delta:.3f} s vs floor {lever['floor_s']} s",
    )
    add(
        "the arms do not overlap: every before sample exceeds every after sample",
        min(before) > max(after),
        f"min(before)={min(before):.3f} max(after)={max(after):.3f}",
    )
    add(
        "every before sample is slower than the after median",
        all(sample > median_after for sample in before),
        f"{sorted(before)}",
    )

    # The gate arithmetic: the serial gate is the sum of three independent
    # scans; the concurrent gate costs the slowest of them. This is why the
    # lever has room, and it is recomputed rather than quoted.
    rounds = b["gate_components"]["rounds"]
    add(
        "two gate-component rounds are recorded",
        len(rounds) == 2,
        f"{len(rounds)}",
    )
    for index, gate in enumerate(rounds, start=1):
        components = [
            gate["check_literals_s"],
            gate["check_test_quality_s"],
            gate["build_guests_check_s"],
        ]
        serial = sum(components)
        add(
            f"gate round {index}: serial sum recomputes",
            abs(serial - gate["serial_sum_s"]) < 1e-6,
            f"{serial:.3f} vs {gate['serial_sum_s']}",
        )
        add(
            f"gate round {index}: the slowest scanner is strictly cheaper than the serial sum",
            max(components) < serial,
            f"max={max(components):.3f} < sum={serial:.3f}",
        )
        add(
            f"gate round {index}: the measured lever is at most the serial gate",
            delta <= serial,
            f"{delta:.3f} <= {serial:.3f}",
        )

    # Inventory arithmetic.
    inv = b["inventory"]
    counts = inv["contract_bucket_counts"]
    add(
        "contract bucket counts reconcile (291 + 23 = 314)",
        counts["passed"] + counts["failed"] == counts["total"] == 314,
        f"{counts['passed']} + {counts['failed']} vs {counts['total']}",
    )
    add(
        "the contract test artifact is a large debug binary and a much larger PDB",
        inv["contract_binary_bytes"] > 80_000_000
        and inv["contract_pdb_bytes"] > 700_000_000
        and inv["contract_pdb_bytes"] > 8 * inv["contract_binary_bytes"],
        f"exe={inv['contract_binary_bytes']} pdb={inv['contract_pdb_bytes']}",
    )
    add(
        "a one-line core edit costs more than a minute of broad check",
        inv["broad_check_after_one_line_edit_slicer_ir_s"] >= 60.0
        and inv["broad_check_after_one_line_edit_slicer_core_s"] >= 60.0,
        f"{inv['broad_check_after_one_line_edit_slicer_ir_s']} / "
        f"{inv['broad_check_after_one_line_edit_slicer_core_s']} s",
    )

    # Falsified premises: the take's credibility rests on these being recorded,
    # so every recorded falsification must still carry a measurement.
    falsified = b["falsified"]
    add(
        "at least four premises were priced and falsified",
        len(falsified) >= 4,
        f"{len(falsified)}",
    )
    check_and_clippy = next(
        (f for f in falsified if "clippy" in f["premise"] and "check" in f["premise"]),
        None,
    )
    add(
        "the check/clippy artifact-split premise is recorded as dead",
        check_and_clippy is not None
        and check_and_clippy["verdict"] == "dead"
        and "0 `Checking` lines" in check_and_clippy["measurement"],
        check_and_clippy["verdict"] if check_and_clippy else "missing",
    )
    add(
        "dropping debug info is recorded as not a win on the incremental path, not as a win",
        any(
            "debug" in f["premise"]
            and f["verdict"].startswith("not")
            and inv["incremental_contract_rebuild_debug_zero_override_s"]
            > inv["incremental_contract_rebuild_default_profile_s"]
            for f in falsified
        ),
        f"{inv['incremental_contract_rebuild_default_profile_s']} vs "
        f"{inv['incremental_contract_rebuild_debug_zero_override_s']} s",
    )
    add(
        "unpriced levers are named rather than silently adopted",
        len(b["not_priced"]) >= 4,
        f"{len(b['not_priced'])}",
    )

    # Adjacent harness defects are part of the take's output.
    add(
        "two adjacent harness defects are recorded",
        len(b["adjacent_findings"]) == 2,
        f"{[f['id'] for f in b['adjacent_findings']]}",
    )
    add(
        "the injected-feature defect names a reproduction",
        any(
            "does not contain this feature" in f["reproduction"]
            for f in b["adjacent_findings"]
        ),
    )

    # Evidence integrity: the candidate's shape is still in the tree. This is a
    # presence check, not a behavioural claim — behaviour is timed above.
    harness = HARNESS.read_text(encoding="utf-8")
    add(
        "the concurrent-preflight seam and its concurrency test are still in the harness",
        "fn run_preflights_with" in harness
        and "std::thread::scope" in harness
        and "fn preflights_overlap_and_preserve_every_outcome" in harness,
    )
    return out


def main() -> int:
    bundle = load()
    rows = check(bundle)
    failed = [r for r in rows if not r[1]]
    for name, ok, detail in rows:
        print(
            f"{'PASS' if ok else 'FAIL'}  {name}" + (f"  [{detail}]" if detail else "")
        )
    print(f"\n{len(rows) - len(failed)}/{len(rows)} claims hold")
    if failed:
        return 1

    if "--self-test" in sys.argv:
        lever = lambda b: b["lever"]["measured"]
        controls = [
            (
                "arms swapped",
                lambda b: lever(b).update(
                    before=lever(b)["after"], after=lever(b)["before"]
                ),
            ),
            ("a before sample dropped", lambda b: lever(b)["before"].pop()),
            (
                "floor raised above the delta",
                lambda b: b["lever"].__setitem__("floor_s", 99.0),
            ),
            (
                "after samples inflated into the before range",
                lambda b: lever(b).update(after=[20.0, 21.0, 22.0, 23.0]),
            ),
            (
                "gate serial sum broken",
                lambda b: b["gate_components"]["rounds"][0].__setitem__(
                    "serial_sum_s", 5.0
                ),
            ),
            (
                "debug=0 recorded as faster than the default",
                lambda b: b["inventory"].__setitem__(
                    "incremental_contract_rebuild_debug_zero_override_s", 1.0
                ),
            ),
            (
                "contract counts broken",
                lambda b: b["inventory"]["contract_bucket_counts"].__setitem__(
                    "passed", 290
                ),
            ),
            (
                "PDB shrunk below the binary",
                lambda b: b["inventory"].__setitem__("contract_pdb_bytes", 1_000_000),
            ),
            ("falsified list emptied", lambda b: b.__setitem__("falsified", [])),
            (
                "adjacent findings emptied",
                lambda b: b.__setitem__("adjacent_findings", []),
            ),
        ]
        missed = 0
        for label, mutate in controls:
            mutated = copy.deepcopy(bundle)
            mutate(mutated)
            if all(ok for _, ok, _ in check(mutated)):
                print(f"SELF-TEST MISSED: {label}")
                missed += 1
        print(
            f"self-test: {len(controls) - missed}/{len(controls)} negative controls detected"
        )
        return 0 if missed == 0 else 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

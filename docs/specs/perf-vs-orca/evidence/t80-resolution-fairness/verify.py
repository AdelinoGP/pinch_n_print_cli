#!/usr/bin/env python3
"""Checkout-only verifier for the t80 resolution-fairness take.

Reads only the committed `rows.csv` and `summary.json` beside this script and
re-derives every figure in the summary from the rows. Exits 0 only if every
check passes and every negative control in `--self-test` is rejected.

Checks:
  1. Every measured row's appendix-verified `resolution_effective` matches its
     arm (0.01 for `default`, 0.001 for `floor`), and every warmup is excluded
     from the medians.
  2. Every (fixture, cell) has two complete arms of equal sample count and at
     least three measured pairs.
  3. `summary.json` re-derives exactly from `rows.csv` (medians, paired deltas,
     median deltas, byte delta).
  4. Every run exited 0 and produced a non-empty G-code.

Raw G-code, Orca logs and the per-run trees are regenerable with
`run_resolution_pair.ps1` and live under `target/t80-resolution/`.
"""

from __future__ import annotations

import copy
import csv
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROWS = HERE / "rows.csv"
SUMMARY = HERE / "summary.json"

EXPECTED_RESOLUTION = {"default": "0.01", "floor": "0.001"}
MIN_PAIRS = 3
EPS = 1e-6


class CheckFailed(Exception):
    pass


def check(condition: bool, message: str) -> None:
    if not condition:
        raise CheckFailed(message)


def median(values: list[float]) -> float | None:
    return statistics.median(values) if values else None


def pct(new: float, old: float) -> float:
    return (new - old) / old * 100.0


def read_rows(path: Path) -> list[dict]:
    with path.open(newline="", encoding="utf-8-sig") as handle:
        return [row for row in csv.DictReader(handle) if row.get("run_id")]


def check_rows(rows: list[dict]) -> None:
    check(bool(rows), "rows.csv is empty")
    for row in rows:
        arm = row["arm"]
        check(arm in EXPECTED_RESOLUTION, f"unknown arm {arm!r} in {row['run_id']}")
        check(
            row["resolution_effective"] == EXPECTED_RESOLUTION[arm],
            f"{row['run_id']}: appendix resolution {row['resolution_effective']!r} "
            f"does not match arm {arm!r} ({EXPECTED_RESOLUTION[arm]})",
        )
        check(row["exit_code"] == "0", f"{row['run_id']}: exit code {row['exit_code']}")
        check(int(row["gcode_bytes"]) > 0, f"{row['run_id']}: empty G-code")
        check(float(row["wall_seconds"]) > 0, f"{row['run_id']}: non-positive wall")


def derive_cells(rows: list[dict]) -> list[dict]:
    """Same derivation as reduce.py, kept independent of the summary file."""
    out: list[dict] = []
    for fixture, cell in sorted({(r["fixture"], r["cell"]) for r in rows}):
        measured = [
            r
            for r in rows
            if r["fixture"] == fixture and r["cell"] == cell and r["warmup"] == "0"
        ]
        arms: dict[str, dict] = {}
        for arm in ("default", "floor"):
            subset = sorted(
                (r for r in measured if r["arm"] == arm),
                key=lambda r: int(r["run_index"]),
            )
            arms[arm] = {
                "n": len(subset),
                "wall_median_s": median([float(r["wall_seconds"]) for r in subset]),
                "cpu_median_s": median([float(r["cpu_seconds"]) for r in subset]),
                "bytes_median": median([float(r["gcode_bytes"]) for r in subset]),
            }
        by_index = {
            arm: {int(r["run_index"]): r for r in measured if r["arm"] == arm}
            for arm in ("default", "floor")
        }
        paired = []
        for index in sorted(set(by_index["default"]) & set(by_index["floor"])):
            d = by_index["default"][index]
            f = by_index["floor"][index]
            paired.append(
                {
                    "run_index": index,
                    "wall_delta_pct": pct(
                        float(f["wall_seconds"]), float(d["wall_seconds"])
                    ),
                    "cpu_delta_pct": pct(
                        float(f["cpu_seconds"]), float(d["cpu_seconds"])
                    ),
                }
            )
        out.append(
            {
                "fixture": fixture,
                "cell": cell,
                "arms": arms,
                "paired": paired,
                "median_wall_delta_pct": median([p["wall_delta_pct"] for p in paired]),
                "median_cpu_delta_pct": median([p["cpu_delta_pct"] for p in paired]),
                "byte_delta_pct": pct(
                    arms["floor"]["bytes_median"], arms["default"]["bytes_median"]
                ),
            }
        )
    return out


def check_structure(rows: list[dict], derived: list[dict]) -> None:
    for cell in derived:
        label = f"{cell['fixture']}/{cell['cell']}"
        n_default = cell["arms"]["default"]["n"]
        n_floor = cell["arms"]["floor"]["n"]
        check(
            n_default == n_floor,
            f"{label}: unequal arm sample counts {n_default} vs {n_floor}",
        )
        check(
            n_default >= MIN_PAIRS, f"{label}: only {n_default} measured runs per arm"
        )
        check(
            len(cell["paired"]) >= MIN_PAIRS,
            f"{label}: only {len(cell['paired'])} measured pairs",
        )
        warmups = [
            r
            for r in rows
            if r["fixture"] == cell["fixture"]
            and r["cell"] == cell["cell"]
            and r["warmup"] == "1"
        ]
        check(
            len(warmups) == 2,
            f"{label}: expected 2 warmup rows (one per arm), found {len(warmups)}",
        )


def check_summary(derived: list[dict], summary: dict) -> None:
    committed = summary.get("cells", [])
    check(len(committed) == len(derived), "summary cell count does not match rows")
    by_key = {(c["fixture"], c["cell"]): c for c in committed}
    for cell in derived:
        key = (cell["fixture"], cell["cell"])
        check(key in by_key, f"summary is missing {key}")
        got = by_key[key]
        label = f"{key[0]}/{key[1]}"
        for arm in ("default", "floor"):
            for field in ("n", "wall_median_s", "cpu_median_s", "bytes_median"):
                expected = cell["arms"][arm][field]
                actual = got["arms"][arm][field]
                if expected is None or actual is None:
                    check(expected == actual, f"{label}/{arm}: {field} null mismatch")
                    continue
                check(
                    abs(float(actual) - float(expected)) < EPS,
                    f"{label}/{arm}: {field} {actual} != re-derived {expected}",
                )
        for field in (
            "median_wall_delta_pct",
            "median_cpu_delta_pct",
            "byte_delta_pct",
        ):
            check(
                abs(float(got[field]) - float(cell[field])) < EPS,
                f"{label}: {field} {got[field]} != re-derived {cell[field]}",
            )
        check(
            len(got["paired"]) == len(cell["paired"]),
            f"{label}: paired count mismatch",
        )
        for actual, expected in zip(got["paired"], cell["paired"]):
            check(
                actual["run_index"] == expected["run_index"],
                f"{label}: pair index mismatch",
            )
            for field in ("wall_delta_pct", "cpu_delta_pct"):
                check(
                    abs(float(actual[field]) - float(expected[field])) < EPS,
                    f"{label}/pair{expected['run_index']}: {field} does not re-derive",
                )
    for arm, spec in summary.get("arms", {}).items():
        check(
            spec["resolution_mm"] == EXPECTED_RESOLUTION[arm],
            f"summary arm {arm} declares resolution {spec['resolution_mm']}",
        )


def verify(rows: list[dict], summary: dict) -> None:
    check_rows(rows)
    derived = derive_cells(rows)
    check_structure(rows, derived)
    check_summary(derived, summary)


def load() -> tuple[list[dict], dict]:
    return read_rows(ROWS), json.loads(SUMMARY.read_text(encoding="utf-8"))


def negative_controls() -> int:
    rows, summary = load()
    controls = 0
    failures: list[str] = []

    def expect_reject(
        name: str, mutated_rows: list[dict], mutated_summary: dict
    ) -> None:
        nonlocal controls
        controls += 1
        try:
            verify(mutated_rows, mutated_summary)
        except CheckFailed:
            return
        failures.append(name)

    # 1. An arm's appendix resolution is swapped: the row-level check must fire.
    bad = copy.deepcopy(rows)
    for row in bad:
        if row["arm"] == "floor" and row["warmup"] == "0":
            row["resolution_effective"] = "0.01"
            break
    expect_reject("appendix resolution does not match arm", bad, copy.deepcopy(summary))

    # 2. A measured pair row is deleted: pair/structure checks must fire.
    bad = copy.deepcopy(rows)
    for i, row in enumerate(bad):
        if row["arm"] == "floor" and row["warmup"] == "0":
            del bad[i]
            break
    expect_reject("missing measured pair row", bad, copy.deepcopy(summary))

    # 3. A warmup is promoted to measured: medians and counts must disagree.
    bad = copy.deepcopy(rows)
    for row in bad:
        if row["warmup"] == "1":
            row["warmup"] = "0"
            break
    expect_reject("warmup promoted to measured", bad, copy.deepcopy(summary))

    # 4. A headline median is tampered with.
    bad_summary = copy.deepcopy(summary)
    bad_summary["cells"][0]["median_wall_delta_pct"] += 1.0
    expect_reject("tampered median delta", copy.deepcopy(rows), bad_summary)

    # 5. A non-zero exit code is smuggled in.
    bad = copy.deepcopy(rows)
    for row in bad:
        if row["warmup"] == "0":
            row["exit_code"] = "1"
            break
    expect_reject("non-zero exit code", bad, copy.deepcopy(summary))

    for name in failures:
        print(f"  NEGATIVE CONTROL NOT REJECTED: {name}")
    print(f"negative controls rejected: {controls - len(failures)}/{controls}")
    return 0 if not failures else 1


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return negative_controls()
    rows, summary = load()
    try:
        verify(rows, summary)
    except CheckFailed as exc:
        print(f"FAIL: {exc}")
        return 1
    print(f"OK: {len(rows)} rows, {len(summary['cells'])} cells re-derived")
    for cell in summary["cells"]:
        print(
            f"  {cell['fixture']:6s} {cell['cell']:11s} "
            f"default {cell['arms']['default']['wall_median_s']:7.2f}s  "
            f"floor {cell['arms']['floor']['wall_median_s']:7.2f}s  "
            f"wall {cell['median_wall_delta_pct']:+6.2f}%  "
            f"cpu {cell['median_cpu_delta_pct']:+6.2f}%  "
            f"bytes {cell['byte_delta_pct']:+6.2f}%"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

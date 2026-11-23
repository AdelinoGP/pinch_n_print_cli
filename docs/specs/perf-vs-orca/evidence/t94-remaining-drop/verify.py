#!/usr/bin/env python3
"""Checkout-only verifier for the t94 Phase-1 reduced evidence set.

Reads `summary.json` and nothing else, recomputes every derived number from the
four run rows, and fails on any inconsistency. `--self-test` also applies eight
in-memory mutations and requires each to be rejected (negative controls).

Usage:
    python verify.py            # validate summary.json, exit 0/1
    python verify.py --self-test
"""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "summary.json"
FLOOR_FRACTION = 0.008
EXPECTED_CELLS = {"ordinary-base-classic-on", "ordinary-base-arachne-on"}
EXPECTED_LAYERS = 495
DROP_MIN_S = 1.0
TAIL_MIN_S = 0.5


def _close(left: float, right: float, *, rel: float = 1e-6, abs_: float = 1e-6) -> bool:
    return abs(left - right) <= max(abs_, rel * max(abs(left), abs(right)))


def check(payload: dict) -> list[str]:
    errors: list[str] = []
    runs = payload.get("runs", [])
    if len(runs) != 4:
        errors.append(f"expected 4 runs, found {len(runs)}")
        return errors
    by_cell: dict[str, dict[str, dict]] = {}
    for row in runs:
        by_cell.setdefault(row["cell"], {})[row["arm"]] = row

    if set(by_cell) != EXPECTED_CELLS:
        errors.append(f"cell set {sorted(by_cell)} != {sorted(EXPECTED_CELLS)}")

    derived = payload.get("derived", {})
    if set(derived) != EXPECTED_CELLS:
        errors.append(f"derived cell set {sorted(derived)} != {sorted(EXPECTED_CELLS)}")

    for cell, arms in by_cell.items():
        if set(arms) != {"serial", "parallel"}:
            errors.append(f"{cell}: arms {sorted(arms)} != ['parallel', 'serial']")
            continue
        serial, parallel = arms["serial"], arms["parallel"]
        if serial["sha256"] != parallel["sha256"]:
            errors.append(f"{cell}: arms disagree on output SHA")
        if serial["bytes"] != parallel["bytes"]:
            errors.append(f"{cell}: arms disagree on output size")
        for arm_name, row in arms.items():
            if row["sha256"] != row["reference_sha256"]:
                errors.append(f"{cell}/{arm_name}: output SHA != reference")
            if row["layers"] != EXPECTED_LAYERS:
                errors.append(
                    f"{cell}/{arm_name}: layers {row['layers']} != {EXPECTED_LAYERS}"
                )
            if row["parallel"] is not row["probe_parallel"]:
                errors.append(f"{cell}/{arm_name}: probe parallel flag != run arm flag")
            if row["exit_code"] != 0:
                errors.append(f"{cell}/{arm_name}: exit_code {row['exit_code']}")
            if row["walk_us"] >= row["drop_us"]:
                errors.append(
                    f"{cell}/{arm_name}: walk >= drop, not the measured shape"
                )
            if row["drop_us"] / 1e6 < DROP_MIN_S:
                errors.append(
                    f"{cell}/{arm_name}: drop {row['drop_us'] / 1e6:.3f}s below {DROP_MIN_S}s"
                )
            bracket = row["emit_bracket_ms"] / 1000.0
            split = (row["rotate_us"] + row["walk_us"] + row["drop_us"]) / 1e6
            if not _close(bracket, split, rel=0.02):
                errors.append(
                    f"{cell}/{arm_name}: bracket {bracket:.3f}s != split {split:.3f}s (>2%)"
                )

        row = derived.get(cell, {})
        wall = row.get("cell_wall_s", 0.0)
        floor_s = row.get("floor_s", 0.0)
        saving_s = (serial["drop_us"] - parallel["drop_us"]) / 1e6
        tail_s = serial["tail_ms"] / 1000.0

        if not _close(floor_s, wall * FLOOR_FRACTION):
            errors.append(f"{cell}: floor_s {floor_s} != cell_wall * floor fraction")
        if not _close(row.get("serial_drop_s", -1), serial["drop_us"] / 1e6):
            errors.append(f"{cell}: serial_drop_s does not match the serial run")
        if not _close(row.get("parallel_drop_s", -1), parallel["drop_us"] / 1e6):
            errors.append(f"{cell}: parallel_drop_s does not match the parallel run")
        if not _close(row.get("parallel_saving_s", -99), saving_s):
            errors.append(f"{cell}: parallel_saving_s does not match the two drops")
        if not _close(
            row.get("parallel_saving_fraction", -99), saving_s / wall, rel=1e-6
        ):
            errors.append(f"{cell}: parallel_saving_fraction does not match")
        if not _close(row.get("tail_serial_s", -1), tail_s):
            errors.append(f"{cell}: tail_serial_s does not match the serial run")
        if not _close(row.get("deferred_ceiling_s", -1), tail_s):
            errors.append(f"{cell}: deferred_ceiling_s != serial tail")
        if not _close(row.get("deferred_ceiling_fraction", -1), tail_s / wall):
            errors.append(f"{cell}: deferred_ceiling_fraction does not match")
        if not _close(row.get("deferred_ceiling_vs_floor", -1), tail_s / floor_s):
            errors.append(f"{cell}: deferred_ceiling_vs_floor does not match")
        if tail_s < TAIL_MIN_S:
            errors.append(f"{cell}: tail {tail_s:.3f}s below {TAIL_MIN_S}s")

        # The screening verdict the ticket turns on: the parallel free is a wash
        # and the deferred ceiling is at the floor. These are the claims the
        # evidence must keep making.
        if abs(saving_s) / wall > 0.005:
            errors.append(
                f"{cell}: parallel free is not a wash ({saving_s / wall:+.4f})"
            )
        if tail_s / floor_s > 1.30:
            errors.append(
                f"{cell}: deferred ceiling is not at the floor ({tail_s / floor_s:.3f}x)"
            )

    if payload.get("floor_fraction") != FLOOR_FRACTION:
        errors.append("top-level floor_fraction changed")

    return errors


MUTATIONS = {
    "parallel saving sign flipped": lambda p: p["derived"][
        "ordinary-base-classic-on"
    ].__setitem__(
        "parallel_saving_s",
        -p["derived"]["ordinary-base-classic-on"]["parallel_saving_s"] + 0.5,
    ),
    "one arm's output SHA changed": lambda p: p["runs"][0].__setitem__(
        "sha256", "0" * 64
    ),
    "layer count changed": lambda p: p["runs"][1].__setitem__("layers", 494),
    "serial tail changed": lambda p: p["derived"][
        "ordinary-base-arachne-on"
    ].__setitem__("tail_serial_s", 1.9),
    "floor fraction rescaled": lambda p: p.__setitem__("floor_fraction", 0.0081),
    "bracket inflated 20%": lambda p: p["runs"][2].__setitem__(
        "emit_bracket_ms", round(p["runs"][2]["emit_bracket_ms"] * 1.2)
    ),
    "probe parallel flag flipped": lambda p: p["runs"][3].__setitem__(
        "probe_parallel", not p["runs"][3]["probe_parallel"]
    ),
    "one run removed": lambda p: p["runs"].pop(),
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    payload = json.loads(SUMMARY.read_text(encoding="utf-8"))
    errors = check(payload)
    if errors:
        for error in errors:
            print(f"FAIL: {error}")
        return 1
    print(f"PASS: {SUMMARY.name} — 4 runs, 2 cells, all derived numbers re-derived")

    if args.self_test:
        rejected = 0
        for name, mutate in MUTATIONS.items():
            mutated = copy.deepcopy(payload)
            mutate(mutated)
            if check(mutated):
                print(f"PASS negative control rejected: {name}")
                rejected += 1
            else:
                print(f"FAIL negative control accepted: {name}")
        print(f"{rejected}/{len(MUTATIONS)} negative controls rejected")
        return 0 if rejected == len(MUTATIONS) else 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

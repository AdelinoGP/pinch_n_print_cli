#!/usr/bin/env python3
"""Checkout-only verifier for the t29 mesh_ops evidence set.

Validates the headline claims against tracked files only (no criterion tree, no
models, no Cargo). Exits 0 when the reduced baselines are internally
consistent and the "new fixtures measure successful operations" claim is
carried by the recorded numbers.

Usage:
    python docs/specs/perf-vs-orca/evidence/t29-mesh-ops-fixtures/verify-t29.py
"""

from __future__ import annotations

import json
import pathlib
import sys

EVIDENCE = pathlib.Path(__file__).resolve().parent
T15_BASELINES = EVIDENCE.parent / "t15-criterion-refresh" / "baselines.json"


def fail(msg: str) -> None:
    print(f"FAIL: {msg}")
    raise SystemExit(1)


def main() -> int:
    baselines = json.loads((EVIDENCE / "baselines.json").read_text(encoding="utf-8"))
    leaves = {row["full_id"]: row for row in baselines["leaves"]}

    expected = {
        "mesh_ops/repair/wedge",
        "mesh_ops/decimate/calicat_default",
        "mesh_ops/import_step/cube",
        "mesh_ops/import_step/assembly",
    }
    if set(leaves) != expected:
        fail(f"leaf set mismatch: {sorted(leaves)} != {sorted(expected)}")

    for name, row in leaves.items():
        if not row["base_identical_to_new"]:
            fail(f"{name}: first-run base == new expected, got base != new")
        if row["samples"] < 100:
            fail(f"{name}: expected 100 samples, got {row['samples']}")
        if row["mean_ns"] <= 0 or row["median_ns"] <= 0:
            fail(f"{name}: nonpositive timing")
        lo, hi = row["mean_ci95_ns"]
        if not (lo <= row["mean_ns"] <= hi):
            fail(f"{name}: mean outside its own CI")

    # The two new leaves must measure successful operations at magnitudes far
    # above the pre-ticket no-op leaves; that is the whole point of the take.
    preticket = baselines["pre_ticket_leaves_superseded"]
    repair_wedge = leaves["mesh_ops/repair/wedge"]["mean_ns"]
    decimate_calicat = leaves["mesh_ops/decimate/calicat_default"]["mean_ns"]
    repair_cube = preticket["mesh_ops/repair/cube"]["mean_ns"]
    decimate_cube = preticket["mesh_ops/decimate/cube_default"]["mean_ns"]
    if repair_wedge <= 5 * repair_cube:
        fail(
            f"repair/wedge mean {repair_wedge} not clearly above pre-ticket "
            f"no-op repair/cube mean {repair_cube}"
        )
    if decimate_calicat <= 5 * decimate_cube:
        fail(
            f"decimate/calicat mean {decimate_calicat} not clearly above "
            f"pre-ticket rejected decimate/cube mean {decimate_cube}"
        )

    # Cross-check the quoted pre-ticket values against t15's own tracked
    # baselines, so the "not comparable" note cites real recorded numbers.
    t15 = json.loads(T15_BASELINES.read_text(encoding="utf-8"))
    t15_by_id = {row["full_id"]: row for row in t15}
    for name, ours in preticket.items():
        theirs = t15_by_id.get(name)
        if theirs is None:
            fail(f"pre-ticket leaf {name} missing from t15 baselines.json")
        for key in ("mean_ns", "median_ns"):
            if abs(ours[key] - theirs[key]) > 0.5:
                fail(f"{name}.{key}: recorded {ours[key]} != t15 {theirs[key]}")

    print("PASS: t29 baselines consistent; new leaves above no-op magnitudes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

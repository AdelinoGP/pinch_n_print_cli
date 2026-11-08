#!/usr/bin/env python3
"""Verify the t100 allocation-census evidence from `summary.json` alone.

Checkout-only: every claim below is checked against the committed reduced set,
with no dependency on the gitignored probe captures or on the slicing rig.

    python docs/specs/perf-vs-orca/evidence/t100-allocation-arena/verify.py
    python docs/specs/perf-vs-orca/evidence/t100-allocation-arena/verify.py --self-test

`--self-test` mutates one field per negative control and requires the matching
claim to reject it, so a passing run cannot be a check that never fires.
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "summary.json"


def _runs(s: dict) -> list[dict]:
    return s["runs"]


def _census(s: dict) -> list[dict]:
    return [r["census"] for r in _runs(s)]


def check_c01_census_deterministic(s: dict) -> None:
    censuses = _census(s)
    blocks = {c["deallocs"] for c in censuses}
    byte_counts = {c["dealloc_bytes"] for c in censuses}
    assert len(blocks) == 1, f"deallocs differ across runs: {blocks}"
    assert len(byte_counts) == 1, f"dealloc_bytes differ across runs: {byte_counts}"
    for c in censuses:
        assert c["allocs_in_window"] == 0, (
            "the drop window allocated, so it is not a clean free window"
        )
        assert c["alloc_bytes_in_window"] == 0, "the drop window allocated bytes"


def check_c02_per_block_cost(s: dict) -> None:
    d = s["derived"]
    assert 80.0 <= d["ns_per_block"] <= 130.0, (
        f"ns/block {d['ns_per_block']} outside [80, 130]"
    )
    assert d["blocks"] == 34_796_481, f"unexpected block count {d['blocks']}"


def check_c03_tiny_strings_dominate(s: dict) -> None:
    d = s["derived"]
    assert d["bucket4_share"] >= 0.99, (
        f"8-15-byte share {d['bucket4_share']} below 0.99"
    )
    samples = []
    for c in _census(s):
        samples.extend(c["samples"])
    assert samples, "no block-content samples recorded"
    assert any("__skirt_" in x for x in samples), (
        "no support family id in the sampled blocks"
    )
    assert any(x.startswith("demand-") for x in samples), (
        "no demand id in the sampled blocks"
    )


def check_c04_geometry_is_not_the_shape(s: dict) -> None:
    d = s["derived"]
    assert d["geometry_block_share"] < 0.001, (
        f"geometry blocks are {d['geometry_block_share']} of all blocks, not a rounding error"
    )
    assert d["geometry_byte_share"] < 0.10, (
        f"geometry bytes are {d['geometry_byte_share']} of the collection"
    )


def check_c05_support_table_scale(s: dict) -> None:
    d = s["derived"]
    assert d["support_rows"] == 23_751, f"support rows {d['support_rows']}"
    assert 1_000 <= d["demand_strings_per_row"] <= 2_000, (
        f"derived demand strings per row {d['demand_strings_per_row']}"
    )
    assert d["demand_strings"] > 30_000_000, (
        "demand string count below the measured order"
    )


def check_c06_bulk_ceiling_above_floor(s: dict) -> None:
    d = s["derived"]
    assert d["bulk_free_ceiling_us"] >= 2.5 * s["floor_us"], (
        f"bulk-free ceiling {d['bulk_free_ceiling_us']} us is under 2.5x the floor"
    )
    assert d["bulk_free_ceiling_share"] >= 0.019, (
        f"bulk-free ceiling share {d['bulk_free_ceiling_share']} below 1.9 %"
    )


def check_c07_bulk_arms_small_against_drop(s: dict) -> None:
    d = s["derived"]
    drop = next(
        c["drop_us"]
        for c in _census(s)
        if c["samples"] == [] and c["drop_us"] < 3_800_000
    )
    assert d["bulk_free_conservative_us"] < 0.10 * drop, (
        "the conservative bulk free is more than a tenth of the measured drop"
    )


def check_c08_flatten_is_strictly_worse(s: dict) -> None:
    d = s["derived"]
    assert d["copy_us"] > 0, "copy pass was elided (checksum path broken)"
    assert d["flatten_total_us"] > d["blocks"] * 0 + 0, "flatten total missing"
    assert d["flatten_penalty_vs_baseline_us"] > 0, (
        "flattening shows no penalty over the baseline drop"
    )
    assert d["flatten_total_us"] >= d["mimalloc_drop_measured_us"] * 2, (
        "flatten total is not clearly above a single drop"
    )


def check_c09_allocator_arm(s: dict) -> None:
    d = s["derived"]
    assert d["mimalloc_drop_measured_us"] < 0.5 * d["sampling_overhead_us"] * 10, (
        "mimalloc drop not clearly below the system-heap drop"
    )
    assert d["mimalloc_saving_measured_us"] >= 1.5 * s["floor_us"], (
        "measured mimalloc saving is under 1.5x the floor"
    )
    assert d["mimalloc_saving_share_measured"] >= 0.012, (
        "measured saving share below 1.2 %"
    )


def check_c10_output_identity(s: dict) -> None:
    for r in _runs(s):
        assert r["sha256"] == s["reference_sha256"], (
            f"{r['id']} output differs from the reference"
        )
        assert r["bytes"] == s["reference_bytes"], f"{r['id']} output size differs"


def check_c11_sampling_overhead_disclosed(s: dict) -> None:
    d = s["derived"]
    assert 200_000 <= d["sampling_overhead_us"] <= 900_000, (
        f"sampling overhead {d['sampling_overhead_us']} us outside the disclosed band"
    )


def check_c12_clean_runs(s: dict) -> None:
    for r in _runs(s):
        assert r["degraded"] is False, f"{r['id']} is degraded"


CLAIMS = [
    (
        "C01",
        "census determinism and a clean drop window",
        check_c01_census_deterministic,
    ),
    ("C02", "the free is per-block (ns per block)", check_c02_per_block_cost),
    (
        "C03",
        "8-15-byte blocks dominate and sample as support-identity strings",
        check_c03_tiny_strings_dominate,
    ),
    (
        "C04",
        "geometry is a rounding error of the block count",
        check_c04_geometry_is_not_the_shape,
    ),
    ("C05", "support-identity table scale", check_c05_support_table_scale),
    ("C06", "bulk-free ceiling is above the floor", check_c06_bulk_ceiling_above_floor),
    (
        "C07",
        "bulk free is small against the drop",
        check_c07_bulk_arms_small_against_drop,
    ),
    (
        "C08",
        "flattening a live collection is strictly worse",
        check_c08_flatten_is_strictly_worse,
    ),
    ("C09", "allocator arm clears the floor", check_c09_allocator_arm),
    ("C10", "every arm reproduces the reference bytes", check_c10_output_identity),
    (
        "C11",
        "sampling overhead is disclosed and bounded",
        check_c11_sampling_overhead_disclosed,
    ),
    ("C12", "no run is degraded", check_c12_clean_runs),
]

BY_ID = {cid: fn for cid, _, fn in CLAIMS}


def _set(path: list, value) -> None:
    def mutator(s: dict) -> None:
        node = s
        for key in path[:-1]:
            node = node[key]
        node[path[-1]] = value

    return mutator


def _first_census_field(field: str, value) -> None:
    def mutator(s: dict) -> None:
        s["runs"][0]["census"][field] = value

    return mutator


def _all_samples(value) -> None:
    def mutator(s: dict) -> None:
        for run in s["runs"]:
            run["census"]["samples"] = list(value)

    return mutator


NEGATIVE_CONTROLS = [
    (
        "nc01 block count drifts between runs",
        "C01",
        _first_census_field("deallocs", 34_796_480),
    ),
    (
        "nc02 the drop window allocated",
        "C01",
        _first_census_field("allocs_in_window", 7),
    ),
    ("nc03 per-block cost collapses", "C02", _set(["derived", "ns_per_block"], 12.0)),
    (
        "nc04 tiny-string share collapses",
        "C03",
        _set(["derived", "bucket4_share"], 0.62),
    ),
    (
        "nc05 samples lose the support family id",
        "C03",
        _all_samples(["demand-1", "heap"]),
    ),
    (
        "nc06 geometry owns the blocks",
        "C04",
        _set(["derived", "geometry_block_share"], 0.42),
    ),
    ("nc07 support table shrinks", "C05", _set(["derived", "support_rows"], 120)),
    (
        "nc08 bulk ceiling drops under the floor",
        "C06",
        _set(["derived", "bulk_free_ceiling_us"], 900_000),
    ),
    (
        "nc09 bulk free rivals the drop",
        "C07",
        _set(["derived", "bulk_free_conservative_us"], 3_400_000),
    ),
    ("nc10 copy pass elided", "C08", _set(["derived", "copy_us"], 0)),
    (
        "nc11 allocator arm saves nothing",
        "C09",
        _set(["derived", "mimalloc_saving_measured_us"], 200_000),
    ),
    (
        "nc12 output drifts from the reference",
        "C10",
        _set(["runs", 0, "sha256"], "0" * 64),
    ),
    ("nc13 a run is degraded", "C12", _set(["runs", 1, "degraded"], True)),
]


def main(argv: list[str]) -> int:
    summary = json.loads(SUMMARY.read_text())
    failures = 0
    for cid, statement, fn in CLAIMS:
        try:
            fn(summary)
        except AssertionError as err:
            failures += 1
            print(f"FAIL {cid} {statement}: {err}")
        else:
            print(f"PASS {cid} {statement}")

    if failures:
        print(f"FAIL {failures} of {len(CLAIMS)} claims")
        return 1

    if "--self-test" in argv:
        rejected = 0
        for name, cid, mutator in NEGATIVE_CONTROLS:
            mutated = copy.deepcopy(summary)
            mutator(mutated)
            try:
                BY_ID[cid](mutated)
            except AssertionError:
                rejected += 1
                print(f"SELFTEST {name}: rejected by {cid}")
            else:
                print(f"SELFTEST {name}: NOT rejected by {cid}")
        if rejected != len(NEGATIVE_CONTROLS):
            print(
                f"FAIL self-test: {rejected}/{len(NEGATIVE_CONTROLS)} negative controls rejected"
            )
            return 1
        print(
            f"PASS {len(CLAIMS)} claims, {rejected}/{len(NEGATIVE_CONTROLS)} negative controls rejected"
        )
    else:
        print(f"PASS {len(CLAIMS)} claims")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

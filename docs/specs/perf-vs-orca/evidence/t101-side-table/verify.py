#!/usr/bin/env python3
"""Verify the t101 side-table evidence from `summary.json` alone.

Checkout-only: every claim below is checked against the committed reduced set,
with no dependency on the gitignored probe captures or on the slicing rig.

    python docs/specs/perf-vs-orca/evidence/t101-side-table/verify.py
    python docs/specs/perf-vs-orca/evidence/t101-side-table/verify.py --self-test

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


def _table(s: dict) -> list[dict]:
    return [r for r in s["runs"] if not r["skip_table"]]


def _skip(s: dict) -> list[dict]:
    return [r for r in s["runs"] if r["skip_table"]]


def check_c01_shape_deterministic(s: dict) -> None:
    shapes = {json.dumps(r["shape"], sort_keys=True) for r in _table(s)}
    assert len(shapes) == 1, (
        f"shape census drifts across table arms: {len(shapes)} variants"
    )
    for r in _table(s):
        assert r["assemble_calls"] == r["shape"]["layers"], (
            f"{r['id']} timed {r['assemble_calls']} assemblies for {r['shape']['layers']} layers"
        )


def check_c02_output_identity(s: dict) -> None:
    for r in s["runs"]:
        assert r["sha256"] == s["reference_sha256"], (
            f"{r['id']} output differs from the post-t81 reference"
        )
        assert r["bytes"] == s["reference_bytes"], f"{r['id']} output size differs"


def check_c03_skip_arm_is_empty(s: dict) -> None:
    for r in _skip(s):
        assert r["shape"]["rows"] == 0, (
            f"{r['id']} still materialised {r['shape']['rows']} rows"
        )
    censused = [r for r in _skip(s) if r["census"]["deallocs"] is not None]
    assert censused, "no censused skip arm, so the empty-table claim has no block count"
    for r in censused:
        assert r["census"]["deallocs"] < 100_000, (
            f"{r['id']} freed {r['census']['deallocs']} blocks with the table skipped"
        )
        assert r["census"]["dealloc_bytes"] < 200_000_000, (
            f"{r['id']} freed {r['census']['dealloc_bytes']} bytes with the table skipped"
        )


def check_c04_table_scale(s: dict) -> None:
    censused = [r for r in _table(s) if r["census"]["deallocs"] is not None]
    assert censused, "no censused table arm"
    for r in censused:
        c = r["census"]
        assert c["deallocs"] == 34_796_481, f"{r['id']} block count {c['deallocs']}"
        assert c["dealloc_bytes"] == 1_358_397_544, (
            f"{r['id']} byte count {c['dealloc_bytes']}"
        )
        assert c["allocs_in_window"] == 0, (
            "the drop window allocated, so it is not a clean free window"
        )


def check_c05_clone_per_path(s: dict) -> None:
    sh = s["shape"]
    assert sh["rows"] == 23_751, f"rows {sh['rows']}"
    assert sh["bodies"] == 431, f"bodies {sh['bodies']}"
    assert s["derived"]["rows_per_body"] >= 50.0, (
        f"only {s['derived']['rows_per_body']} rows per body: the clone is not per path"
    )
    assert sh["mixed_bodies"] == 0, (
        f"{sh['mixed_bodies']} bodies carry rows with differing demand lists, "
        "so sharing per body would not be content-exact"
    )


def check_c06_sharing_would_be_exact_and_cheap(s: dict) -> None:
    sh = s["shape"]
    assert sh["distinct_lists"] == 418, f"distinct lists {sh['distinct_lists']}"
    assert sh["retained_ids"] == 428_726, f"retained ids {sh['retained_ids']}"
    assert s["derived"]["retained_block_share"] <= 0.02, (
        f"sharing retains {s['derived']['retained_block_share']:.4f} of the blocks"
    )


def check_c07_allocator_collapsed_the_free(s: dict) -> None:
    assert s["derived"]["t100_over_t101_drop"] >= 5.0, (
        "t100's system-heap drop is not several times this run's, so mimalloc did "
        "not collapse the free side"
    )
    assert (
        s["derived"]["median_drop_table_us"] <= 0.25 * s["t100_system_heap_drop_us"]
    ), "median table-arm drop is not far below t100's system-heap drop"


def check_c08_allocation_side_is_real(s: dict) -> None:
    d = s["derived"]
    assert d["median_assemble_table_us"] >= 10 * d["median_assemble_skip_us"], (
        "the assembly window does not separate the table arm from the skip arm"
    )
    assert d["median_assemble_table_us"] >= 400_000, (
        f"table-arm assembly is only {d['median_assemble_table_us']} us"
    )


def check_c09_ceiling_below_floor(s: dict) -> None:
    d = s["derived"]
    assert d["ceiling_generous_us"] < s["floor_us"], (
        f"the most generous combined ceiling {d['ceiling_generous_us']} us is "
        f"above the {s['floor_us']} us floor"
    )
    assert d["ceiling_uncontended_us"] < s["floor_us"], (
        f"the uncontended ceiling {d['ceiling_uncontended_us']} us is above the floor"
    )
    assert d["ceiling_generous_multiple"] < 1.0, (
        "generous ceiling multiple is not below 1x"
    )


def check_c10_wall_cannot_resolve_it(s: dict) -> None:
    d = s["derived"]
    delta_us = abs(d["wall_median_table_s"] - d["wall_median_skip_s"]) * 1e6
    walls = [r["wall_s"] for r in s["runs"]]
    spread_us = (max(walls) - min(walls)) * 1e6
    # The disclosure is "no wall claim": the machine's own run-to-run spread has
    # to exceed the whole instrumented ceiling, or the wall would resolve it and
    # the disclosure would be wrong.
    assert spread_us > d["ceiling_generous_us"], (
        f"the run-to-run wall spread {spread_us / 1000:.0f} ms is inside the "
        f"{d['ceiling_generous_us'] / 1000:.0f} ms ceiling, so the wall does resolve the lever"
    )
    assert delta_us < spread_us, (
        f"the median wall gap {delta_us / 1000:.0f} ms exceeds the run spread "
        f"{spread_us / 1000:.0f} ms"
    )


def check_c11_heap_footprint(s: dict) -> None:
    sh = s["shape"]
    assert sh["present_heap_bytes"] >= 1_200_000_000, (
        f"table heap is only {sh['present_heap_bytes']} bytes"
    )
    assert s["derived"]["retained_heap_share"] <= 0.02, (
        f"sharing retains {s['derived']['retained_heap_share']:.4f} of the heap"
    )


def check_c12_single_binary_and_clean(s: dict) -> None:
    exes = {r["exe_sha"] for r in s["runs"]}
    assert len(exes) == 1, f"runs came from {len(exes)} different probe binaries"
    assert len(_table(s)) >= 3, "fewer than three table arms, so the median is weak"
    assert len(_skip(s)) >= 2, "fewer than two skip arms"
    for r in s["runs"]:
        assert r["degraded"] is False, f"{r['id']} is degraded"


CLAIMS = [
    (
        "C01",
        "the shape census is deterministic and covers every layer",
        check_c01_shape_deterministic,
    ),
    (
        "C02",
        "every arm reproduces the post-t81 reference bytes",
        check_c02_output_identity,
    ),
    ("C03", "the skip arm really materialises no row", check_c03_skip_arm_is_empty),
    (
        "C04",
        "the table arm reproduces t100's block and byte counts",
        check_c04_table_scale,
    ),
    (
        "C05",
        "the table clones one demand list per support path",
        check_c05_clone_per_path,
    ),
    (
        "C06",
        "sharing per body would be content-exact and keep ~1 % of the blocks",
        check_c06_sharing_would_be_exact_and_cheap,
    ),
    (
        "C07",
        "mimalloc collapsed the free side t100 priced",
        check_c07_allocator_collapsed_the_free,
    ),
    (
        "C08",
        "the allocation side is real and separated by the assembly window",
        check_c08_allocation_side_is_real,
    ),
    (
        "C09",
        "the combined ceiling is below the pre-declared floor",
        check_c09_ceiling_below_floor,
    ),
    (
        "C10",
        "the paired wall cannot resolve an effect of this size",
        check_c10_wall_cannot_resolve_it,
    ),
    ("C11", "the table is a 1.2 GB heap structure", check_c11_heap_footprint),
    (
        "C12",
        "one probe binary, three-plus table arms, no degraded run",
        check_c12_single_binary_and_clean,
    ),
]

BY_ID = {cid: fn for cid, _, fn in CLAIMS}


def _set(path: list, value):
    def mutator(s: dict) -> None:
        node = s
        for key in path[:-1]:
            node = node[key]
        node[path[-1]] = value

    return mutator


def _run_field(run, field: str, value):
    def mutator(s: dict) -> None:
        s["runs"][run][field] = value

    return mutator


def _census_field(run, field: str, value):
    def mutator(s: dict) -> None:
        s["runs"][run]["census"][field] = value

    return mutator


def _arm_census_field(skip: bool, field: str, value):
    """Mutate a census field on the first run of the requested arm.

    Filter-aware so a control survives a change in run count or ordering.
    """

    def mutator(s: dict) -> None:
        arm = [r for r in s["runs"] if r["skip_table"] is skip]
        arm[0]["census"][field] = value

    return mutator


def _all_wall(value: float):
    def mutator(s: dict) -> None:
        for r in s["runs"]:
            r["wall_s"] = value

    return mutator


def _shape_field(run, field: str, value):
    def mutator(s: dict) -> None:
        s["runs"][run]["shape"][field] = value

    return mutator


NEGATIVE_CONTROLS = [
    ("nc01 shape drifts between table arms", "C01", _shape_field(0, "rows", 23_752)),
    ("nc02 an assembly window is missing", "C01", _run_field(0, "assemble_calls", 494)),
    ("nc03 output drifts from the reference", "C02", _run_field(0, "sha256", "0" * 64)),
    (
        "nc04 the skip arm still frees the table",
        "C03",
        _arm_census_field(True, "deallocs", 34_000_000),
    ),
    (
        "nc05 the table arm's block count drifts",
        "C04",
        _arm_census_field(False, "deallocs", 34_796_480),
    ),
    ("nc06 rows per body collapses", "C05", _set(["shape", "rows"], 900)),
    (
        "nc07 a body carries two different lists",
        "C05",
        _set(["shape", "mixed_bodies"], 3),
    ),
    (
        "nc08 sharing retains everything",
        "C06",
        _set(["derived", "retained_block_share"], 0.98),
    ),
    (
        "nc09 t100 and t101 drops are the same order",
        "C07",
        _set(["derived", "t100_over_t101_drop"], 1.1),
    ),
    (
        "nc10 the assembly window does not separate",
        "C08",
        _set(["derived", "median_assemble_table_us"], 18_000),
    ),
    (
        "nc11 the ceiling clears the floor",
        "C09",
        _set(["derived", "ceiling_generous_us"], 1_400_000),
    ),
    (
        "nc12 the wall resolves the lever",
        "C10",
        _all_wall(151.0),
    ),
    (
        "nc13 the table heap is small",
        "C11",
        _set(["shape", "present_heap_bytes"], 4_000_000),
    ),
    ("nc14 two probe binaries are mixed", "C12", _run_field(1, "exe_sha", "f" * 64)),
    ("nc15 a run is degraded", "C12", _run_field(2, "degraded", True)),
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

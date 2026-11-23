#!/usr/bin/env python3
"""Verify the t102 allocator evidence from the committed artifacts alone.

Checkout-only: no dependency on the gitignored captures or on the slicing rig.

    python docs/specs/perf-vs-orca/evidence/t102-allocator/verify.py
    python docs/specs/perf-vs-orca/evidence/t102-allocator/verify.py --self-test

The claims check the evidence's integrity and internal consistency: the
qualification matrix, the reference match, the per-pair byte identity, and that
every recorded statistic recomputes from the committed rows. The gate verdict is
then checked for consistency with its own clauses, so the file cannot claim a
PASS that its numbers do not support (or a DROP its numbers do not support).

`--self-test` mutates one field per negative control and requires the matching
claim to reject it.
"""

from __future__ import annotations

import copy
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MEASURE_CELLS = [
    f"{mode}-base-{generator}-on"
    for mode in ("ordinary", "accelerated")
    for generator in ("classic", "arachne")
]
CPU_CEILING_PCT = 3.0
PAIRS_REQUIRED = 6


def load(name: str):
    return json.loads((HERE / name).read_text(encoding="utf-8"))


def med(values):
    return statistics.median(values)


def check_c01_qualification(s: dict) -> None:
    q = s["qualification"]
    assert q["pass"] is True, f"qualification failed: {q['problems']}"
    assert len(q["cells"]) == 16, f"qualification cells {len(q['cells'])} != 16"
    assert q["problems"] == [], f"qualification problems: {q['problems']}"


def check_c02_cross_mode(s: dict) -> None:
    cross = s["qualification"]["cross_mode"]
    assert len(cross) == 8, f"cross-mode entries {len(cross)} != 8"
    unequal = [key for key, entry in cross.items() if not entry["equal"]]
    assert not unequal, f"cross-mode bytes differ on {unequal}"


def check_c03_references(s: dict) -> None:
    # The post-t81 reference set is ordinary-only; accelerated cells are gated
    # on cross-mode equality (C02) plus their own baseline/candidate identity.
    cells = [
        c for c in s["qualification"]["cells"] if c["cell"].startswith("ordinary-")
    ]
    assert len(cells) == 8, f"ordinary cells {len(cells)} != 8"
    missing = [c["cell"] for c in cells if c["reference_sha256"] is None]
    assert not missing, f"ordinary cells without a reference: {missing}"
    drift = [
        c["cell"]
        for c in cells
        if c["reference_sha256"] is not None and c["sha256"] != c["reference_sha256"]
    ]
    assert not drift, f"output drifted from the post-t81 references: {drift}"


def check_c04_output_identity(s: dict) -> None:
    for cell, result in s["ab"]["cells"].items():
        assert result["status"] == "reduced", f"{cell} not reduced: {result['status']}"
        shas = {pair["sha256"] for pair in result["pairs"]}
        assert len(shas) == 1, (
            f"{cell} retained pairs differ in output: {len(shas)} SHAs"
        )
    for cell in s["measure_rows"]:
        rows = s["measure_rows"][cell]
        shas = {row["sha256"] for row in rows if row["warmup"] == 0}
        assert len(shas) == 1, f"{cell} measured rows differ in output"


def check_c05_distinct_binaries(s: dict) -> None:
    for cell in s["qualification"]["cells"]:
        exes = set(cell["exe_sha256"].values())
        assert len(exes) == 2, f"{cell['cell']} arms share one host binary"


def check_c06_complete_campaign(s: dict) -> None:
    cells = s["ab"]["cells"]
    assert set(cells) == set(MEASURE_CELLS), f"campaign cells {sorted(cells)}"
    for cell, result in cells.items():
        assert result["retained_pairs"] == PAIRS_REQUIRED, (
            f"{cell} retained {result['retained_pairs']} pairs"
        )
        assert not result["excluded_pairs"], (
            f"{cell} exclusions {result['excluded_pairs']}"
        )


def check_c07_wall_recomputes(s: dict) -> None:
    for cell, result in s["ab"]["cells"].items():
        ratios = [pair["wall_ratio"] for pair in result["pairs"]]
        recomputed = (med(ratios) - 1.0) * 100.0
        assert abs(recomputed - result["wall_delta_pct_paired_median"]) < 1e-9, (
            f"{cell} wall delta {result['wall_delta_pct_paired_median']} != "
            f"recomputed {recomputed}"
        )
        for pair in result["pairs"]:
            own = pair["candidate_wall_s"] / pair["baseline_wall_s"]
            assert abs(own - pair["wall_ratio"]) < 1e-9, (
                f"{cell} pair {pair['pair']} ratio"
            )


def check_c08_cpu_recomputes(s: dict) -> None:
    for cell, result in s["ab"]["cells"].items():
        ratios = [pair["cpu_ratio"] for pair in result["pairs"]]
        recomputed = (med(ratios) - 1.0) * 100.0
        assert abs(recomputed - result["cpu_delta_pct_paired_median"]) < 1e-9, (
            f"{cell} CPU delta {result['cpu_delta_pct_paired_median']} != "
            f"recomputed {recomputed}"
        )


def check_c09_verdict_consistent(s: dict) -> None:
    ab = s["ab"]
    cells = ab["cells"]
    wall_ok = all(c["wall_delta_pct_paired_median"] < 0.0 for c in cells.values())
    cpu_ok = all(
        c["cpu_delta_pct_paired_median"] <= CPU_CEILING_PCT for c in cells.values()
    )
    pairs_ok = all(
        c["retained_pairs"] == PAIRS_REQUIRED and not c["excluded_pairs"]
        for c in cells.values()
    )
    outputs_ok = all(
        len({p["sha256"] for p in c["pairs"]}) == 1 for c in cells.values()
    )
    assert ab["wall_pass"] is wall_ok, "recorded wall_pass disagrees with the cells"
    assert ab["cpu_pass"] is cpu_ok, "recorded cpu_pass disagrees with the cells"
    assert ab["pairs_pass"] is pairs_ok, "recorded pairs_pass disagrees with the cells"
    assert ab["output_pass"] is outputs_ok, (
        "recorded output_pass disagrees with the cells"
    )
    verdict = "PASS" if all((wall_ok, cpu_ok, pairs_ok, outputs_ok)) else "GATE NOT MET"
    assert ab["verdict"] == verdict, f"verdict {ab['verdict']} != {verdict}"


def check_c10_pregate_consistent(s: dict) -> None:
    pre = s["pregate"]
    assert pre["status"] == "reduced", f"pregate status {pre['status']}"
    clauses = pre["clauses"]
    assert clauses["wall_negative"] is (pre["wall_delta_pct_paired_median"] < 0.0)
    assert clauses["cpu_within_ceiling"] is (
        pre["cpu_delta_pct_paired_median"] <= CPU_CEILING_PCT
    )
    proceed = all(clauses.values())
    assert pre["proceed"] is proceed, "pregate proceed disagrees with its clauses"


def check_c11_row_counts(s: dict) -> None:
    for cell in MEASURE_CELLS:
        rows = s["measure_rows"][cell]
        assert len(rows) == 16, f"{cell} has {len(rows)} rows, want 16"
        measured = [row for row in rows if row["warmup"] == 0]
        assert len(measured) == 12, f"{cell} has {len(measured)} measured rows, want 12"
        for arm in ("baseline", "candidate"):
            n = sum(1 for row in measured if row["arm"] == arm)
            assert n == 6, f"{cell} {arm} measured rows {n} != 6"


def check_c12_clean(s: dict) -> None:
    for name in ("proof_rows", "pregate_rows", "measure_rows"):
        for cell, rows in s[name].items():
            for row in rows:
                assert row["status"] == "ok", (
                    f"{cell} {row['run_id']} status {row['status']}"
                )
                assert row["degraded"] is False, f"{cell} {row['run_id']} degraded"
                assert row["fatal"] == 0 and row["non_fatal"] == 0, (
                    f"{cell} {row['run_id']} fatal/non-fatal"
                )


CLAIMS = [
    ("C01", "qualification matrix passes", check_c01_qualification),
    ("C02", "cross-mode bytes equal 8/8", check_c02_cross_mode),
    ("C03", "ordinary cells match the post-t81 references", check_c03_references),
    ("C04", "every retained pair is byte-identical", check_c04_output_identity),
    ("C05", "the arms ship distinct host binaries", check_c05_distinct_binaries),
    (
        "C06",
        "the campaign is complete (4 cells, 6 pairs, no exclusions)",
        check_c06_complete_campaign,
    ),
    ("C07", "wall deltas recompute from the pair rows", check_c07_wall_recomputes),
    ("C08", "CPU deltas recompute from the pair rows", check_c08_cpu_recomputes),
    ("C09", "the gate verdict agrees with its clauses", check_c09_verdict_consistent),
    (
        "C10",
        "the pre-gate verdict agrees with its clauses",
        check_c10_pregate_consistent,
    ),
    (
        "C11",
        "row counts are 16 per cell (4 warmups + 12 measured)",
        check_c11_row_counts,
    ),
    ("C12", "no run is unclean or degraded", check_c12_clean),
]
BY_ID = {cid: fn for cid, _, fn in CLAIMS}


def _set(path: list, value):
    def mutator(s: dict) -> None:
        node = s
        for key in path[:-1]:
            node = node[key]
        node[path[-1]] = value

    return mutator


def _first_pair(field: str, value):
    def mutator(s: dict) -> None:
        s["ab"]["cells"][MEASURE_CELLS[0]]["pairs"][0][field] = value

    return mutator


def _share_one_binary(s: dict) -> None:
    cell = s["qualification"]["cells"][0]
    cell["exe_sha256"]["candidate"] = cell["exe_sha256"]["baseline"]


def _drop_a_row(s: dict) -> None:
    s["measure_rows"][MEASURE_CELLS[0]].pop(0)


NEGATIVE_CONTROLS = [
    ("nc01 qualification marked failed", "C01", _set(["qualification", "pass"], False)),
    (
        "nc02 a cross-mode pair differs",
        "C02",
        _set(["qualification", "cross_mode", "base-classic-on", "equal"], False),
    ),
    (
        "nc03 an ordinary cell drifts from the reference",
        "C03",
        _set(["qualification", "cells", 0, "sha256"], "0" * 64),
    ),
    ("nc04 a retained pair drifts", "C04", _first_pair("sha256", "1" * 64)),
    (
        "nc05 the arms share one binary",
        "C05",
        _share_one_binary,
    ),
    (
        "nc06 a cell loses a pair",
        "C06",
        _set(["ab", "cells", MEASURE_CELLS[0], "retained_pairs"], 5),
    ),
    (
        "nc07 a recorded wall delta is wrong",
        "C07",
        _set(["ab", "cells", MEASURE_CELLS[0], "wall_delta_pct_paired_median"], 1.5),
    ),
    (
        "nc08 a recorded CPU delta is wrong",
        "C08",
        _set(["ab", "cells", MEASURE_CELLS[0], "cpu_delta_pct_paired_median"], -9.0),
    ),
    (
        "nc09 the verdict ignores a failed clause",
        "C09",
        _set(["ab", "wall_pass"], False),
    ),
    (
        "nc10 the pre-gate proceeds against its clauses",
        "C10",
        _set(["pregate", "proceed"], False),
    ),
    (
        "nc11 a cell loses a measured row",
        "C11",
        _drop_a_row,
    ),
    (
        "nc12 a run is degraded",
        "C12",
        _set(["measure_rows", MEASURE_CELLS[0], 0, "degraded"], True),
    ),
]


def main(argv: list[str]) -> int:
    summary = {
        "qualification": load("qualification.json"),
        "pregate": load("pregate.json"),
        "ab": load("ab-reduction.json"),
        "proof_rows": load("proof-rows.json"),
        "pregate_rows": load("pregate-rows.json"),
        "measure_rows": load("measure-rows.json"),
    }
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

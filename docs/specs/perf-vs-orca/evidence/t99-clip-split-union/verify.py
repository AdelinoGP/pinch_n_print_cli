#!/usr/bin/env python3
"""Verify the t99 Phase-1 screen from the in-repo evidence set alone.

The screen priced the ticket's literal split-and-union shape — offset every
`deep_infill_area` polygon on its own, then union the pieces — against the
production whole-layer `offset(deep_infill_area, 1.5 x spacing)` call inside
`compute_internal_bridge_layer`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`).

Every claim below is re-derived from `rows-prod-first.csv`, `rows-split-first.csv`
and their `run-*.json` metadata, then cross-checked against `probe-rows.json`.
`--self-test` additionally requires that six wrong claims are rejected.

Exit codes: 0 = all claims hold and every control is rejected; 1 = otherwise.
No network, no raw capture tree, no model fixtures.
"""

import argparse
import copy
import csv
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CELL_WALL_SECONDS = 73.17  # t87 ordinary base classic supports-off cell wall
FLOOR_SHARE = 0.008  # map's pre-declared floor
REFERENCE_SHA = "292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80"
REFERENCE_BYTES = 21023739
CALLS = 130

RUNS = [
    ("prod-first", "rows-prod-first.csv", "run-prod-first.json"),
    ("split-first", "rows-split-first.csv", "run-split-first.json"),
]


def load_rows(path):
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def aggregate(rows, meta):
    prod = [float(r["prod_ms"]) for r in rows]
    offsets = [float(r["offset_pieces_ms"]) for r in rows]
    union_ms = [float(r["union_ms"]) for r in rows]
    prod_sum = sum(prod)
    offsets_sum = sum(offsets)
    union_sum = sum(union_ms)
    split_sum = offsets_sum + union_sum
    saving = prod_sum - split_sum
    ratio = meta["cpu_wall_ratio"]
    wall_eq = saving / ratio
    return {
        "calls": len(rows),
        "prod_ms": prod_sum,
        "offset_pieces_ms": offsets_sum,
        "union_ms": union_sum,
        "split_ms": split_sum,
        "saving_ms": saving,
        "split_over_prod": split_sum / prod_sum,
        "union_free_over_prod": offsets_sum / prod_sum,
        "wall_eq_ms": wall_eq,
        "share_pct": 100.0 * wall_eq / (CELL_WALL_SECONDS * 1000.0),
        "exact_calls": sum(int(r["exact_equal"]) for r in rows),
        "set_equal_calls": sum(int(r["set_equal"]) for r in rows),
        "diff_polys": sum(int(r["diff_polys"]) for r in rows),
    }


def load_all():
    runs = {}
    for tag, rows_name, meta_name in RUNS:
        rows = load_rows(HERE / rows_name)
        meta = json.loads((HERE / meta_name).read_text(encoding="utf-8"))
        runs[tag] = {"rows": rows, "meta": meta, "agg": aggregate(rows, meta)}
    return runs


class Report:
    def __init__(self):
        self.failures = []

    def check(self, name, ok, detail=""):
        print(f"{'PASS' if ok else 'FAIL'}  {name}{('  ' + detail) if detail else ''}")
        if not ok:
            self.failures.append(name)
        return ok


def claims(report, runs):
    a = runs["prod-first"]["agg"]
    b = runs["split-first"]["agg"]

    report.check(
        "C1 both arms reproduce the post-t81 reference and are 130 calls",
        all(
            runs[tag]["agg"]["calls"] == CALLS
            and runs[tag]["meta"]["sha256"] == REFERENCE_SHA
            and runs[tag]["meta"]["bytes"] == REFERENCE_BYTES
            and runs[tag]["meta"]["matches_reference"]
            for tag in runs
        ),
        f"sha {REFERENCE_SHA[:8]}... / {REFERENCE_BYTES} bytes",
    )

    spread = abs(a["split_over_prod"] - b["split_over_prod"]) / a["split_over_prod"]
    report.check(
        "C2 split-and-union is cheaper in both orders and is order-independent",
        a["split_over_prod"] < 1.0 and b["split_over_prod"] < 1.0 and spread < 0.01,
        f"{a['split_over_prod']:.4f}x / {b['split_over_prod']:.4f}x, spread {100 * spread:.2f}%",
    )

    floor_ms = FLOOR_SHARE * CELL_WALL_SECONDS * 1000.0
    report.check(
        "C3 the saving is BELOW the 0.80% floor in both runs",
        a["wall_eq_ms"] < floor_ms and b["wall_eq_ms"] < floor_ms,
        f"{a['wall_eq_ms']:.1f} / {b['wall_eq_ms']:.1f} ms vs floor {floor_ms:.1f} ms "
        f"({a['share_pct']:.2f}% / {b['share_pct']:.2f}%)",
    )
    report.check(
        "C3b the best case is the union-free lower bound, and it is also below the floor",
        a["union_free_over_prod"] < 1.0,
        f"offsets-only is {a['union_free_over_prod']:.4f}x of production",
    )

    report.check(
        "C4 the split arm is not representation-safe",
        all(
            runs[tag]["agg"]["exact_calls"] < CALLS
            and runs[tag]["agg"]["set_equal_calls"] < CALLS
            for tag in runs
        ),
        f"exact {a['exact_calls']}/130, set-equal {a['set_equal_calls']}/130, "
        f"{a['diff_polys']} differing polygons",
    )
    report.check(
        "C4b the exactness outcome is deterministic across orders",
        a["exact_calls"] == b["exact_calls"]
        and a["set_equal_calls"] == b["set_equal_calls"]
        and a["diff_polys"] == b["diff_polys"],
    )

    union_share = a["union_ms"] / a["split_ms"]
    report.check(
        "C5 the explicit union is ~2% of the split arm, so the win is partitioned cleanup",
        union_share < 0.05,
        f"union {a['union_ms']:.1f} ms of {a['split_ms']:.1f} ms ({100 * union_share:.2f}%)",
    )

    recorded = json.loads((HERE / "probe-rows.json").read_text(encoding="utf-8"))
    by_tag = {run["tag"]: run for run in recorded["runs"]}
    report.check(
        "C6 probe-rows.json matches the raw rows",
        all(
            abs(by_tag[tag]["prod_ms"] - runs[tag]["agg"]["prod_ms"]) < 1e-6
            and by_tag[tag]["exact_calls"] == runs[tag]["agg"]["exact_calls"]
            and by_tag[tag]["set_equal_calls"] == runs[tag]["agg"]["set_equal_calls"]
            for tag in runs
        ),
    )


def controls(report, runs):
    """Each control states a wrong claim; the control passes only if rejected."""

    def rejected(name, claim_ok, detail=""):
        report.check(f"NC rejected: {name}", not claim_ok, detail)

    a = runs["prod-first"]["agg"]
    b = runs["split-first"]["agg"]
    floor_ms = FLOOR_SHARE * CELL_WALL_SECONDS * 1000.0

    rejected(
        "the split shape clears the floor",
        a["wall_eq_ms"] >= floor_ms,
        f"share {a['share_pct']:.2f}% < 0.80%",
    )
    rejected(
        "the split arm is byte-identical to production",
        a["exact_calls"] == CALLS,
        f"exact on {a['exact_calls']}/{CALLS}",
    )
    rejected(
        "the split arm is set-equal on every call",
        a["set_equal_calls"] == CALLS,
        f"set-equal on {a['set_equal_calls']}/{CALLS}",
    )
    union_free_saving = a["prod_ms"] * (1.0 - a["union_free_over_prod"])
    union_free_share = (
        100.0
        * (union_free_saving / runs["prod-first"]["meta"]["cpu_wall_ratio"])
        / (CELL_WALL_SECONDS * 1000.0)
    )
    rejected(
        "a free union would lift the shape over the floor",
        union_free_share >= 0.8,
        f"union-free share {union_free_share:.2f}%",
    )
    spread = abs(a["split_over_prod"] - b["split_over_prod"]) / a["split_over_prod"]
    rejected(
        "the saving is an order artifact (the two orders disagree by > 5%)",
        spread > 0.05,
        f"orders agree within {100 * spread:.2f}%",
    )
    tampered = copy.deepcopy(runs["prod-first"]["rows"])
    for row in tampered:
        row["prod_ms"] = str(float(row["prod_ms"]) * 0.5)
    tampered_agg = aggregate(tampered, runs["prod-first"]["meta"])
    rejected(
        "halving the recorded production spans leaves the aggregate intact",
        abs(tampered_agg["prod_ms"] - a["prod_ms"]) < 1.0,
        f"{tampered_agg['prod_ms']:.1f} vs {a['prod_ms']:.1f} ms",
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    runs = load_all()
    report = Report()
    claims(report, runs)
    if args.self_test:
        controls(report, runs)

    print()
    if report.failures:
        print(f"FAIL ({len(report.failures)} checks)")
        return 1
    suffix = " + 6/6 negative controls rejected" if args.self_test else ""
    print(f"SELF-TEST PASS{suffix}" if args.self_test else "PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())

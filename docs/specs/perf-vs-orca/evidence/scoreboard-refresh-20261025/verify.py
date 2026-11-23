#!/usr/bin/env python3
"""Re-derive the 2026-10-25 matched-pair scoreboard from the committed rows.

Checkout-only: reads the curated row CSVs in this directory (never the raw
target/ capture tree), applies the standing protocol rules, and asserts every
median, gap, output flag and byte-identity claim in SCOREBOARD.md.

Rules mirrored from summarize_scoreboard.ps1:
  - measured rows only (warmup == 0);
  - starvation (PERF-HANDOFF 10.3): within a (fixture, cell, tool) group a
    sample is excluded when cpu/wall < 0.75 * the group's best cpu/wall;
  - median of retained wall/cpu samples (even count averages the middle two).

Exit 0 = every expectation matched. rows-benchy-duplicate-round.csv is part of
the disclosure (an accidentally doubled benchy round) and is deliberately NOT
read as ledger input.
"""

import argparse
import copy
import csv
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

LEDGER_CSVS = [
    "rows-benchy.csv",
    "rows-base-classic-off.csv",
    "rows-base-arachne-off.csv",
    "rows-base-classic-on.csv",
    "rows-base-arachne-on.csv",
]
DISCLOSURE_CSVS = ["rows-benchy-duplicate-round.csv"]

TOL_MEDIAN = 0.011  # docs round medians to 2 dp; CSV carries 4 dp
TOL_RATIO = 0.011

# (fixture, cell) -> {tool: (wall_s, cpu_s)}
EXPECTED = {
    ("benchy", "classic-off"): {
        "orca": (2.79, 9.27),
        "pnp-ordinary": (7.68, 49.70),
        "pnp-accelerated": (7.56, 48.66),
    },
    ("benchy", "arachne-off"): {
        "orca": (2.88, 10.50),
        "pnp-ordinary": (7.90, 50.58),
        "pnp-accelerated": (7.89, 50.59),
    },
    ("benchy", "classic-on"): {
        "orca": (3.53, 14.36),
        "pnp-ordinary": (12.30, 65.41),
        "pnp-accelerated": (12.33, 65.30),
    },
    ("benchy", "arachne-on"): {
        "orca": (3.64, 16.03),
        "pnp-ordinary": (12.58, 66.64),
        "pnp-accelerated": (12.47, 66.44),
    },
    ("base", "classic-off"): {
        "orca": (13.87, 33.20),
        "pnp-ordinary": (73.17, 708.59),
        "pnp-accelerated": (72.92, 704.50),
    },
    ("base", "arachne-off"): {
        "orca": (14.14, 36.56),
        "pnp-ordinary": (73.25, 714.17),
        "pnp-accelerated": (73.23, 711.19),
    },
    ("base", "classic-on"): {
        "orca": (18.37, 58.48),
        "pnp-ordinary": (171.07, 1034.34),
        "pnp-accelerated": (171.71, 1031.16),
    },
    ("base", "arachne-on"): {
        "orca": (19.93, 65.00),
        "pnp-ordinary": (186.63, 1075.62),
        "pnp-accelerated": (183.34, 1071.00),
    },
}

# (fixture, cell) -> (ord/orca, acc/orca)
EXPECTED_GAPS = {
    ("benchy", "classic-off"): (2.75, 2.71),
    ("benchy", "arachne-off"): (2.74, 2.74),
    ("benchy", "classic-on"): (3.48, 3.49),
    ("benchy", "arachne-on"): (3.45, 3.42),
    ("base", "classic-off"): (5.28, 5.26),
    ("base", "arachne-off"): (5.18, 5.18),
    ("base", "classic-on"): (9.31, 9.35),
    ("base", "arachne-on"): (9.36, 9.20),
}

# (fixture, cell) -> (gcode_bytes, sha256) shared by both PnP modes
EXPECTED_PNP_OUTPUT = {
    ("benchy", "classic-off"): (
        3925554,
        "3672dc7e1649cab69a0ffab2ebd293233a9af0504d0f98666746da933d8c641d",
    ),
    ("benchy", "arachne-off"): (
        4064270,
        "5158539849b0ed1b6b1fb89a9f222480cfb30e68027ee4c284e2f60505eda85b",
    ),
    ("benchy", "classic-on"): (
        8252412,
        "eb4817b4be28c0a30e906bed9d960cd59282fa20d95afb509e48c8e39ab4b714",
    ),
    ("benchy", "arachne-on"): (
        8390168,
        "6b8a0074ca7ac4e8142af764dce669495bd411b45ad47f94a29dc57451146e05",
    ),
    ("base", "classic-off"): (
        21023739,
        "292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80",
    ),
    ("base", "arachne-off"): (
        20846160,
        "01a1434c6ab5bbe965e37985e78137b739b47dccf2d364a5d3dbd0156cad7e97",
    ),
    ("base", "classic-on"): (
        42507912,
        "22200671167a97a7efb1b912e9e1661a317b5e81dab169ec154ab5ad4b9f028d",
    ),
    ("base", "arachne-on"): (
        42330265,
        "0b29e1bf928f1c0a160ebdedf08245edfd1d0433b3807ad1118b4578a378981c",
    ),
}


def load_rows(csv_names=None):
    rows = []
    for name in csv_names or LEDGER_CSVS:
        path = HERE / name
        if not path.exists():
            raise SystemExit(f"missing ledger csv: {name}")
        with path.open(newline="", encoding="utf-8-sig") as fh:
            for row in csv.DictReader(fh):
                rows.append(row)
    return rows


def group_rows(rows):
    groups = {}
    for row in rows:
        key = (
            row["fixture"],
            f"{row['generator']}-{row['supports']}",
            row["tool"],
        )
        groups.setdefault(key, []).append(row)
    return groups


def retained(rows):
    measured = [r for r in rows if r["warmup"] == "0"]
    if not measured:
        return [], []
    best = max(float(r["cpu_wall_ratio"]) for r in measured)
    kept = [r for r in measured if float(r["cpu_wall_ratio"]) >= 0.75 * best]
    return kept, [r for r in measured if r not in kept]


def medians(rows):
    w = statistics.median(float(r["wall_seconds"]) for r in rows)
    c = statistics.median(float(r["cpu_seconds"]) for r in rows)
    return w, c


def check(rows, expected, gaps, pnp_output):
    failures = []
    groups = group_rows(rows)

    def near(a, b, tol):
        return abs(a - b) <= tol

    for (fixture, cell), tools in expected.items():
        walls = {}
        for tool, (exp_w, exp_c) in tools.items():
            group = groups.get((fixture, cell, tool), [])
            warmups = [r for r in group if r["warmup"] == "1"]
            measured = [r for r in group if r["warmup"] == "0"]
            if len(warmups) != 1:
                failures.append(
                    f"{fixture}/{cell}/{tool}: warmup count {len(warmups)} != 1"
                )
            if len(measured) != 3:
                failures.append(
                    f"{fixture}/{cell}/{tool}: measured count {len(measured)} != 3"
                )
            kept, starved = retained(group)
            if starved:
                failures.append(
                    f"{fixture}/{cell}/{tool}: {len(starved)} starved sample(s) present"
                )
            if not kept:
                failures.append(f"{fixture}/{cell}/{tool}: no retained samples")
                continue
            w, c = medians(kept)
            walls[tool] = w
            if not near(w, exp_w, TOL_MEDIAN):
                failures.append(f"{fixture}/{cell}/{tool}: wall {w:.3f} != {exp_w}")
            if not near(c, exp_c, TOL_MEDIAN):
                failures.append(f"{fixture}/{cell}/{tool}: cpu {c:.3f} != {exp_c}")
            if tool != "orca":
                for row in kept:
                    if row["degraded"] != "false":
                        failures.append(
                            f"{fixture}/{cell}/{tool}: degraded={row['degraded']} in {row['run_id']}"
                        )
                    if str(row["non_fatal_error_count"]) != "0":
                        failures.append(
                            f"{fixture}/{cell}/{tool}: non_fatal={row['non_fatal_error_count']} in {row['run_id']}"
                        )
        if len(walls) != 3:
            continue
        exp_ord, exp_acc = gaps[(fixture, cell)]
        got_ord = walls["pnp-ordinary"] / walls["orca"]
        got_acc = walls["pnp-accelerated"] / walls["orca"]
        if not near(got_ord, exp_ord, TOL_RATIO):
            failures.append(f"{fixture}/{cell}: ord gap {got_ord:.3f} != {exp_ord}")
        if not near(got_acc, exp_acc, TOL_RATIO):
            failures.append(f"{fixture}/{cell}: acc gap {got_acc:.3f} != {exp_acc}")

        exp_bytes, exp_sha = pnp_output[(fixture, cell)]
        for mode in ("pnp-ordinary", "pnp-accelerated"):
            group = [
                r for r in groups.get((fixture, cell, mode), []) if r["warmup"] == "0"
            ]
            for row in group:
                if int(row["gcode_bytes"]) != exp_bytes:
                    failures.append(
                        f"{fixture}/{cell}/{mode}: bytes {row['gcode_bytes']} != {exp_bytes}"
                    )
                if row["sha256_of_gcode"] != exp_sha:
                    failures.append(
                        f"{fixture}/{cell}/{mode}: sha {row['sha256_of_gcode'][:12]}… != {exp_sha[:12]}…"
                    )
        ord_sha = {
            r["sha256_of_gcode"]
            for r in groups.get((fixture, cell, "pnp-ordinary"), [])
        }
        acc_sha = {
            r["sha256_of_gcode"]
            for r in groups.get((fixture, cell, "pnp-accelerated"), [])
        }
        if not ord_sha or ord_sha != acc_sha:
            failures.append(f"{fixture}/{cell}: ordinary/accelerated SHA sets differ")

    for name in LEDGER_CSVS:
        if not (HERE / name).exists():
            failures.append(f"missing {name}")
    return failures


def self_test():
    """Eight negative controls: each perturbation must be detected."""
    base_rows = load_rows()
    controls = []

    def run(name, rows=None, expected=None, gaps=None, pnp_output=None):
        failures = check(
            rows if rows is not None else copy.deepcopy(base_rows),
            expected if expected is not None else copy.deepcopy(EXPECTED),
            gaps if gaps is not None else copy.deepcopy(EXPECTED_GAPS),
            pnp_output
            if pnp_output is not None
            else copy.deepcopy(EXPECTED_PNP_OUTPUT),
        )
        controls.append((name, bool(failures)))

    exp = copy.deepcopy(EXPECTED)
    exp[("benchy", "classic-off")]["pnp-ordinary"] = (7.68 + 0.5, 49.70)
    run("wall median", expected=exp)

    exp = copy.deepcopy(EXPECTED)
    exp[("base", "classic-on")]["pnp-accelerated"] = (171.71, 1031.16 + 5.0)
    run("cpu median", expected=exp)

    gaps = copy.deepcopy(EXPECTED_GAPS)
    gaps[("base", "arachne-on")] = (9.36 + 0.5, 9.20)
    run("gap ratio", gaps=gaps)

    rows = copy.deepcopy(base_rows)
    victim = next(
        r
        for r in rows
        if r["fixture"] == "benchy"
        and r["generator"] == "arachne"
        and r["supports"] == "on"
        and r["tool"] == "orca"
        and r["warmup"] == "0"
    )
    rows.remove(victim)
    run("sample count", rows=rows)

    rows = copy.deepcopy(base_rows)
    victim = next(
        r
        for r in rows
        if r["fixture"] == "base"
        and r["generator"] == "classic"
        and r["supports"] == "off"
        and r["tool"] == "pnp-ordinary"
        and r["warmup"] == "0"
    )
    victim["cpu_wall_ratio"] = "0.5"
    run("starvation flag", rows=rows)

    rows = copy.deepcopy(base_rows)
    victim = next(
        r
        for r in rows
        if r["fixture"] == "benchy"
        and r["generator"] == "classic"
        and r["supports"] == "on"
        and r["tool"] == "pnp-accelerated"
        and r["warmup"] == "0"
    )
    victim["degraded"] = "true"
    run("degraded flag", rows=rows)

    rows = copy.deepcopy(base_rows)
    victim = next(
        r
        for r in rows
        if r["fixture"] == "base"
        and r["generator"] == "arachne"
        and r["supports"] == "off"
        and r["tool"] == "pnp-accelerated"
        and r["warmup"] == "0"
    )
    victim["gcode_bytes"] = str(int(victim["gcode_bytes"]) + 1)
    run("byte identity", rows=rows)

    output = copy.deepcopy(EXPECTED_PNP_OUTPUT)
    output[("benchy", "arachne-off")] = (
        output[("benchy", "arachne-off")][0],
        "0" * 64,
    )
    run("sha identity", pnp_output=output)

    missing = [name for name, detected in controls if not detected]
    if missing:
        print(f"SELF-TEST FAILED: undetected controls: {', '.join(missing)}")
        return 1
    print(
        f"SELF-TEST OK: {len(controls)}/{len(controls)} negative controls detected "
        f"({', '.join(name for name, _ in controls)})"
    )
    return 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    if not (HERE / DISCLOSURE_CSVS[0]).exists():
        print(f"FAIL\n  - missing disclosure csv: {DISCLOSURE_CSVS[0]}")
        return 1

    if args.self_test:
        return self_test()

    failures = check(
        load_rows(),
        copy.deepcopy(EXPECTED),
        copy.deepcopy(EXPECTED_GAPS),
        copy.deepcopy(EXPECTED_PNP_OUTPUT),
    )
    if args.verbose:
        print(f"ledger csvs: {len(LEDGER_CSVS)}; rows: {len(load_rows())}")
    if failures:
        print("FAIL")
        for failure in failures:
            print("  -", failure)
        return 1
    print(
        "PASS: 24 medians, 16 wall-gap ratios, 24 group shapes (1 warmup + 3 measured), "
        "zero starvation, PnP output flags, and ordinary==accelerated bytes/SHA "
        "match across 8 cells x 3 tools"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Re-derive the 2026-10-17/18 matched-pair scoreboard from the committed rows.

Checkout-only: reads the curated row CSVs in this directory (never the raw
target/ capture tree), applies the standing protocol rules, and asserts every
median and gap in SCOREBOARD.md.

Rules mirrored from summarize_scoreboard.ps1:
  - measured rows only (warmup == 0);
  - starvation (PERF-HANDOFF 10.3): within a (fixture, cell, tool) group a
    sample is excluded when cpu/wall < 0.75 * the group's best cpu/wall;
  - median of retained wall/cpu samples (even count averages the middle two).

Exit 0 = every expectation matched. The .interrupted.csv files are part of the
disclosure (evicted partial batches) and are deliberately NOT read as ledger
input; --include-interrupted shows what happens if they were.
"""

import argparse
import csv
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

# batch csv -> ledger role
LEDGER_CSVS = [
    "rows-benchy.csv",
    "rows-base-classic-off.csv",  # curated: classic-off of batch 2
    "rows-base-arachne-off.csv",  # curated: arachne-off of batch 3
    "rows-base-classic-on-orca.csv",
    "rows-base-arachne-on-orca.csv",
    "rows-base-classic-on-ord.csv",
    "rows-base-classic-on-acc.csv",
    "rows-base-arachne-on-ord.csv",
    "rows-base-arachne-on-acc.csv",
]
INTERRUPTED_CSVS = [
    "rows-refresh-20261017-base.interrupted.csv",
    "rows-refresh-20261017-base-rest.interrupted.csv",
]

TOL_MEDIAN = 0.011  # doc rounds medians to 2 dp; CSV carries 4 dp
TOL_RATIO = 0.011

# (fixture, cell) -> {tool: (wall_s, cpu_s)}
EXPECTED = {
    ("benchy", "classic-off"): {
        "orca": (4.27, 12.06),
        "pnp-ordinary": (21.96, 118.48),
        "pnp-accelerated": (18.52, 97.66),
    },
    ("benchy", "arachne-off"): {
        "orca": (4.45, 11.62),
        "pnp-ordinary": (14.65, 66.97),
        "pnp-accelerated": (14.71, 61.05),
    },
    ("benchy", "classic-on"): {
        "orca": (4.73, 15.84),
        "pnp-ordinary": (27.73, 121.38),
        "pnp-accelerated": (24.06, 102.27),
    },
    ("benchy", "arachne-on"): {
        "orca": (4.38, 16.17),
        "pnp-ordinary": (20.87, 74.94),
        "pnp-accelerated": (20.30, 73.09),
    },
    ("base", "classic-off"): {
        "orca": (18.84, 38.75),
        "pnp-ordinary": (216.28, 1606.39),
        "pnp-accelerated": (173.41, 1182.64),
    },
    ("base", "arachne-off"): {
        "orca": (18.06, 40.66),
        "pnp-ordinary": (133.42, 796.67),
        "pnp-accelerated": (130.63, 762.27),
    },
    ("base", "classic-on"): {
        "orca": (24.38, 64.45),
        "pnp-ordinary": (439.66, 1986.59),
        "pnp-accelerated": (408.35, 1580.12),
    },
    ("base", "arachne-on"): {
        "orca": (23.78, 67.94),
        "pnp-ordinary": (372.16, 1208.95),
        "pnp-accelerated": (368.58, 1174.75),
    },
}

# (fixture, cell) -> (ord/orca, acc/orca)
EXPECTED_GAPS = {
    ("benchy", "classic-off"): (5.14, 4.34),
    ("benchy", "arachne-off"): (3.30, 3.31),
    ("benchy", "classic-on"): (5.87, 5.09),
    ("benchy", "arachne-on"): (4.76, 4.63),
    ("base", "classic-off"): (11.48, 9.20),
    ("base", "arachne-off"): (7.39, 7.24),
    ("base", "classic-on"): (18.03, 16.75),
    ("base", "arachne-on"): (15.65, 15.50),
}


def load_groups(csv_names):
    groups = {}
    for name in csv_names:
        path = HERE / name
        if not path.exists():
            raise SystemExit(f"missing ledger csv: {name}")
        with path.open(newline="", encoding="utf-8-sig") as fh:
            for row in csv.DictReader(fh):
                key = (
                    row["fixture"],
                    f"{row['generator']}-{row['supports']}",
                    row["tool"],
                )
                groups.setdefault(key, []).append(row)
    return groups


def retained(rows, rule=True):
    measured = [r for r in rows if r["warmup"] == "0"]
    if not rule or not measured:
        return measured, []
    best = max(float(r["cpu_wall_ratio"]) for r in measured)
    kept = [r for r in measured if float(r["cpu_wall_ratio"]) >= 0.75 * best]
    return kept, [r for r in measured if r not in kept]


def medians(rows):
    w = statistics.median(float(r["wall_seconds"]) for r in rows)
    c = statistics.median(float(r["cpu_seconds"]) for r in rows)
    return w, c


def verify(rule=True, verbose=False):
    failures = []
    groups = load_groups(LEDGER_CSVS)

    def near(a, b, tol):
        return abs(a - b) <= tol

    if verbose:
        print(f"groups: {len(groups)}")

    for (fixture, cell), tools in EXPECTED.items():
        walls = {}
        for tool, (exp_w, exp_c) in tools.items():
            rows = groups.get((fixture, cell, tool), [])
            kept, starved = retained(rows, rule)
            if len([r for r in rows if r["warmup"] == "1"]) != 1:
                failures.append(f"{fixture}/{cell}/{tool}: warmup count != 1")
            if len([r for r in rows if r["warmup"] == "0"]) != 3:
                failures.append(f"{fixture}/{cell}/{tool}: measured count != 3")
            if starved:
                failures.append(f"{fixture}/{cell}/{tool}: starved samples present")
            w, c = medians(kept)
            walls[tool] = w
            if not near(w, exp_w, TOL_MEDIAN):
                failures.append(f"{fixture}/{cell}/{tool}: wall {w:.3f} != {exp_w}")
            if not near(c, exp_c, TOL_MEDIAN):
                failures.append(f"{fixture}/{cell}/{tool}: cpu {c:.3f} != {exp_c}")
            if tool != "orca":
                for r in kept:
                    if (
                        r["degraded"] != "false"
                        or str(r["non_fatal_error_count"]) != "0"
                    ):
                        failures.append(
                            f"{fixture}/{cell}/{tool}: degraded/non-fatals in {r['run_id']}"
                        )
        exp_ord, exp_acc = EXPECTED_GAPS[(fixture, cell)]
        if not near(walls["pnp-ordinary"] / walls["orca"], exp_ord, TOL_RATIO):
            failures.append(
                f"{fixture}/{cell}: ord gap {walls['pnp-ordinary'] / walls['orca']:.3f} != {exp_ord}"
            )
        if not near(walls["pnp-accelerated"] / walls["orca"], exp_acc, TOL_RATIO):
            failures.append(
                f"{fixture}/{cell}: acc gap {walls['pnp-accelerated'] / walls['orca']:.3f} != {exp_acc}"
            )

    for name in LEDGER_CSVS:
        if not (HERE / name).exists():
            failures.append(f"missing {name}")
    return failures


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--include-interrupted",
        action="store_true",
        help="also load the evicted partial batches (expected to break checks)",
    )
    ap.add_argument(
        "--self-test",
        action="store_true",
        help="perturb one expectation and confirm detection",
    )
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    if args.self_test:
        original = EXPECTED[("base", "classic-on")]["pnp-ordinary"][0]
        EXPECTED[("base", "classic-on")]["pnp-ordinary"] = (
            original + 1.0,
            EXPECTED[("base", "classic-on")]["pnp-ordinary"][1],
        )
        failures = verify(rule=True)
        if not any("wall" in f for f in failures):
            print("SELF-TEST FAILED: perturbed expectation was not detected")
            return 1
        print(f"SELF-TEST OK: perturbation detected ({len(failures)} failure(s))")
        return 0

    global LEDGER_CSVS
    if args.include_interrupted:
        LEDGER_CSVS = LEDGER_CSVS + INTERRUPTED_CSVS

    failures = verify(rule=True, verbose=args.verbose)
    if failures:
        print("FAIL")
        for f in failures:
            print("  -", f)
        return 1
    print(
        "PASS: all 24 medians, 8 wall-gap ratios, sample counts, starvation state and "
        "output-flags match across 8 cells x 3 tools"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

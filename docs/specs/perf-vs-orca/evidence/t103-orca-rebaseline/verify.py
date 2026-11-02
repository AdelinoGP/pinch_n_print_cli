#!/usr/bin/env python3
"""Re-derive the 2026-10-31/11-01 Orca re-baseline scoreboard (ticket 103).

Checkout-only: reads the curated row CSVs in this directory (never the raw
target/ capture tree), applies the standing protocol rules, and asserts every
headline claim in SCOREBOARD.md:

  - 8 cells x 3 tools, 1 warmup + 3 measured rows per group, and only the two
    disclosed base-arachne-on exclusions (every other exclusion fails);
  - every Orca row carries the ruling's three pins in its resolved appendix
    (enable_arc_fitting disabled, elefant_foot_compensation 0, brim_width 0)
    and the resolved vendor filament leg, never the archived hardcoded defaults
    (density 0 / max volumetric speed 2 / first-layer temp 200);
  - the standing adhesion asymmetry: PnP resolves brim_width 8 in all 8 cells,
    pinned-back Orca resolves 0;
  - PnP 16/16 groups degraded=false and non_fatal=0;
  - PnP ordinary == accelerated, byte for byte, in all 8 cells;
  - the re-derived medians, gap ratios and park bound.

Rules mirrored from summarize_scoreboard.ps1:
  - measured rows only (warmup == 0);
  - starvation (PERF-HANDOFF 10.3): within a (fixture, cell, tool) group a
    sample is excluded when cpu/wall < 0.75 * the group's best cpu/wall;
  - median of retained wall/cpu samples (even count averages the middle two).

Exit 0 = every claim holds. `--self-test` mutates copies and requires every
negative control to be detected.
"""

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
# Preserved, not ledger input: two whole-cell base arachne-on runs spoiled by
# load (see SCOREBOARD.md "Disclosures").
DISCLOSURE_CSVS = [
    "rows-base-arachne-on-r2.csv",
    "rows-base-arachne-on-r3.csv",
    "rows-base-arachne-on-r4.csv",
]

TOOLS = ("orca", "pnp-ordinary", "pnp-accelerated")
FLOOR = 0.008
PARK_RATIO = 1.5
TOL_MEDIAN = 0.011
TOL_RATIO = 0.011

# The only starvation exclusions in the ledger batches, both in base arachne-on
# round 3 (external load event; SCOREBOARD.md "Disclosures"). Every other
# exclusion is a failure.
ALLOWED_STARVED = {
    "base-arachne-on-orca-m3",
    "base-arachne-on-pnp-ordinary-m3",
}

# (fixture, cell) -> {tool: (wall_s, cpu_s)}
EXPECTED = {
    ("benchy", "classic-off"): {
        "orca": (2.0315, 6.5625),
        "pnp-ordinary": (7.6778, 48.3750),
        "pnp-accelerated": (7.6709, 48.0781),
    },
    ("benchy", "arachne-off"): {
        "orca": (2.1854, 7.7031),
        "pnp-ordinary": (7.6868, 48.9688),
        "pnp-accelerated": (7.8367, 49.2188),
    },
    ("benchy", "classic-on"): {
        "orca": (2.2861, 8.7656),
        "pnp-ordinary": (12.4034, 64.2500),
        "pnp-accelerated": (12.5655, 64.0781),
    },
    ("benchy", "arachne-on"): {
        "orca": (2.5531, 10.2031),
        "pnp-ordinary": (12.2164, 65.9531),
        "pnp-accelerated": (12.3951, 65.8438),
    },
    ("base", "classic-off"): {
        "orca": (15.2786, 30.7812),
        "pnp-ordinary": (75.3794, 710.3438),
        "pnp-accelerated": (75.7426, 708.7188),
    },
    ("base", "arachne-off"): {
        "orca": (16.7171, 40.1094),
        "pnp-ordinary": (78.8872, 719.6562),
        "pnp-accelerated": (76.5122, 714.1562),
    },
    ("base", "classic-on"): {
        "orca": (19.4880, 49.8438),
        "pnp-ordinary": (160.8570, 1033.3281),
        "pnp-accelerated": (160.0930, 1028.7500),
    },
    ("base", "arachne-on"): {
        "orca": (23.0276, 59.1484),
        "pnp-ordinary": (161.1394, 1035.3593),
        "pnp-accelerated": (173.4689, 1035.1406),
    },
}

# (fixture, cell) -> (ordinary/orca, accelerated/orca)
EXPECTED_GAPS = {
    ("benchy", "classic-off"): (3.7794, 3.7760),
    ("benchy", "arachne-off"): (3.5173, 3.5859),
    ("benchy", "classic-on"): (5.4256, 5.4965),
    ("benchy", "arachne-on"): (4.7849, 4.8549),
    ("base", "classic-off"): (4.9337, 4.9574),
    ("base", "arachne-off"): (4.7190, 4.5769),
    ("base", "classic-on"): (8.2542, 8.2150),
    ("base", "arachne-on"): (6.9977, 7.5331),
}

EXPECTED_PARK = {
    ("benchy", "classic-off"): 0.6031,
    ("benchy", "arachne-off"): 0.5735,
    ("benchy", "classic-on"): 0.7235,
    ("benchy", "arachne-on"): 0.6865,
    ("base", "classic-off"): 0.6960,
    ("base", "arachne-off"): 0.6821,
    ("base", "classic-on"): 0.8183,
    ("base", "arachne-on"): 0.7856,
}


def load_rows(names=None):
    rows = []
    for name in names or LEDGER_CSVS:
        path = HERE / name
        if not path.exists():
            raise SystemExit(f"missing ledger csv: {name}")
        with path.open(newline="", encoding="utf-8-sig") as fh:
            rows.extend(csv.DictReader(fh))
    return rows


def group_rows(rows):
    groups = {}
    for row in rows:
        key = (row["fixture"], f"{row['generator']}-{row['supports']}", row["tool"])
        groups.setdefault(key, []).append(row)
    return groups


def retained(group):
    measured = [r for r in group if r["warmup"] == "0"]
    if not measured:
        return [], []
    best = max(float(r["cpu_wall_ratio"]) for r in measured)
    kept = [r for r in measured if float(r["cpu_wall_ratio"]) >= 0.75 * best]
    return kept, [r for r in measured if r not in kept]


def medians(rows):
    return (
        statistics.median(float(r["wall_seconds"]) for r in rows),
        statistics.median(float(r["cpu_seconds"]) for r in rows),
    )


def check(rows, expected, gaps, park):
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
                # Named, disclosed exceptions (SCOREBOARD.md "Disclosures"):
                # base arachne-on's third round ran during an external load
                # event. Any other exclusion fails the verification.
                unexpected = [
                    r["run_id"]
                    for r in starved
                    if not (
                        (fixture, cell) == ("base", "arachne-on")
                        and r["run_id"] in ALLOWED_STARVED
                    )
                ]
                if unexpected:
                    failures.append(
                        f"{fixture}/{cell}/{tool}: {len(unexpected)} starved sample(s) present: {unexpected}"
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

            for row in kept:
                if tool == "orca":
                    if row["resolved_enable_arc_fitting"] not in ("disabled", "0"):
                        failures.append(
                            f"{fixture}/{cell}/orca: arc pin {row['resolved_enable_arc_fitting']!r}"
                        )
                    if row["resolved_elefant_foot_compensation"] != "0":
                        failures.append(
                            f"{fixture}/{cell}/orca: elefant pin {row['resolved_elefant_foot_compensation']!r}"
                        )
                    if row["resolved_brim_width"] != "0":
                        failures.append(
                            f"{fixture}/{cell}/orca: brim pin {row['resolved_brim_width']!r}"
                        )
                    if row["resolved_filament_density"] in ("", "0"):
                        failures.append(
                            f"{fixture}/{cell}/orca: filament_density {row['resolved_filament_density']!r}"
                        )
                    if row["resolved_filament_max_volumetric_speed"] in ("", "2"):
                        failures.append(
                            f"{fixture}/{cell}/orca: max volumetric speed {row['resolved_filament_max_volumetric_speed']!r}"
                        )
                    if row["resolved_first_layer_temperature"] in ("", "200"):
                        failures.append(
                            f"{fixture}/{cell}/orca: first-layer temperature {row['resolved_first_layer_temperature']!r}"
                        )
                else:
                    if row["degraded"] != "false":
                        failures.append(
                            f"{fixture}/{cell}/{tool}: degraded={row['degraded']} in {row['run_id']}"
                        )
                    if str(row["non_fatal_error_count"]) != "0":
                        failures.append(
                            f"{fixture}/{cell}/{tool}: non_fatal={row['non_fatal_error_count']} in {row['run_id']}"
                        )
                    if row["resolved_brim_width"] != "8":
                        failures.append(
                            f"{fixture}/{cell}/{tool}: PnP brim {row['resolved_brim_width']!r} != 8 (asymmetry)"
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

        exp_park = park[(fixture, cell)]
        got_park = 1.0 - (PARK_RATIO * walls["orca"]) / walls["pnp-ordinary"]
        if not near(got_park, exp_park, 0.005):
            failures.append(
                f"{fixture}/{cell}: park reduction {got_park:.4f} != {exp_park}"
            )
        if got_park <= 0.0:
            failures.append(
                f"{fixture}/{cell}: cell is inside the park, contradicting the table"
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

    if len(expected) != 8:
        failures.append(f"expectation table covers {len(expected)} cells, not 8")
    return failures


def self_test():
    base_rows = load_rows()
    controls = []

    def run(name, rows=None, expected=None, gaps=None, park=None):
        failures = check(
            rows if rows is not None else copy.deepcopy(base_rows),
            expected if expected is not None else copy.deepcopy(EXPECTED),
            gaps if gaps is not None else copy.deepcopy(EXPECTED_GAPS),
            park if park is not None else copy.deepcopy(EXPECTED_PARK),
        )
        controls.append((name, bool(failures)))

    exp = copy.deepcopy(EXPECTED)
    first = sorted(exp)[0]
    tool = "pnp-ordinary"
    exp[first][tool] = (exp[first][tool][0] + 0.5, exp[first][tool][1])
    run("wall median", expected=exp)

    exp = copy.deepcopy(EXPECTED)
    exp[first]["orca"] = (exp[first]["orca"][0], exp[first]["orca"][1] + 5.0)
    run("cpu median", expected=exp)

    gaps = copy.deepcopy(EXPECTED_GAPS)
    gaps[first] = (gaps[first][0] + 0.5, gaps[first][1])
    run("gap ratio", gaps=gaps)

    park = copy.deepcopy(EXPECTED_PARK)
    park[first] = park[first] + 0.05
    run("park bound", park=park)

    rows = copy.deepcopy(base_rows)
    victim = next(r for r in rows if r["tool"] == "orca" and r["warmup"] == "0")
    rows.remove(victim)
    run("sample count", rows=rows)

    rows = copy.deepcopy(base_rows)
    victim = next(
        r
        for r in rows
        if r["tool"] == "pnp-ordinary"
        and r["warmup"] == "0"
        and r["fixture"] == "benchy"
    )
    victim["cpu_wall_ratio"] = "0.5"
    run("starvation flag", rows=rows)

    rows = copy.deepcopy(base_rows)
    victim = next(
        r for r in rows if r["tool"] == "pnp-accelerated" and r["warmup"] == "0"
    )
    victim["degraded"] = "true"
    run("degraded flag", rows=rows)

    rows = copy.deepcopy(base_rows)
    victim = next(r for r in rows if r["tool"] == "pnp-ordinary" and r["warmup"] == "0")
    victim["gcode_bytes"] = str(int(victim["gcode_bytes"]) + 1)
    victim["sha256_of_gcode"] = "0" * 64
    run("byte identity", rows=rows)

    rows = copy.deepcopy(base_rows)
    victim = next(r for r in rows if r["tool"] == "orca" and r["warmup"] == "0")
    victim["resolved_brim_width"] = "5"
    run("orca brim pin", rows=rows)

    rows = copy.deepcopy(base_rows)
    victim = next(r for r in rows if r["tool"] == "orca" and r["warmup"] == "0")
    victim["resolved_elefant_foot_compensation"] = "0.15"
    run("elefant pin", rows=rows)

    rows = copy.deepcopy(base_rows)
    victim = next(r for r in rows if r["tool"] == "orca" and r["warmup"] == "0")
    victim["resolved_enable_arc_fitting"] = "enabled"
    run("arc pin", rows=rows)

    rows = copy.deepcopy(base_rows)
    victim = next(r for r in rows if r["tool"] == "orca" and r["warmup"] == "0")
    victim["resolved_filament_density"] = "0"
    run("filament default", rows=rows)

    rows = copy.deepcopy(base_rows)
    victim = next(r for r in rows if r["tool"] == "pnp-ordinary" and r["warmup"] == "0")
    victim["resolved_brim_width"] = "0"
    run("brim asymmetry", rows=rows)

    missing = [name for name, detected in controls if not detected]
    if missing:
        print(f"SELF-TEST FAILED: undetected controls: {', '.join(missing)}")
        return 1
    print(
        f"SELF-TEST OK: {len(controls)}/{len(controls)} negative controls detected "
        f"({', '.join(name for name, _ in controls)})"
    )
    return 0


def main(argv):
    if not all((HERE / name).exists() for name in LEDGER_CSVS + DISCLOSURE_CSVS):
        print("FAIL: ledger or disclosure CSV missing")
        return 1
    if "--self-test" in argv:
        return self_test()
    failures = check(
        load_rows(),
        copy.deepcopy(EXPECTED),
        copy.deepcopy(EXPECTED_GAPS),
        copy.deepcopy(EXPECTED_PARK),
    )
    if failures:
        print("FAIL")
        for failure in failures:
            print("  -", failure)
        return 1
    print(
        "PASS: 24 medians, 16 wall-gap ratios, 8 park bounds, 24 group shapes "
        "(1 warmup + 3 measured), only the two named base-arachne-on exclusions, "
        "Orca pins + filament leg, PnP brim asymmetry, PnP output flags, "
        "ordinary==accelerated bytes/SHA"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

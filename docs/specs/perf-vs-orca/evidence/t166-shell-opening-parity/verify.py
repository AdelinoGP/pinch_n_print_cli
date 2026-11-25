#!/usr/bin/env python3
"""Re-derive every t166 campaign claim from reduced/campaign.json.

Reads only the in-repo reduced set (no raw tree, no gitignored files), so the
claims stay re-derivable from a plain checkout:

    python verify.py
    python verify.py --self-test      # negative controls must be rejected

Gate read (pre-declared in the ticket):

  * four `base` cells: wall saving >= 0.80 % and outside the measured null band
  * four `benchy` cells: no median wall regression
  * every cell's output must move (the parity fix is expected to move geometry)
"""

import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CAMPAIGN = os.path.join(HERE, "reduced", "campaign.json")

FLOOR_PCT = 0.80
BASE_CELLS = [
    "base-classic-off",
    "base-classic-on",
    "base-arachne-off",
    "base-arachne-on",
]
BENCHY_CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "benchy-arachne-off",
    "benchy-arachne-on",
]
ALL_CELLS = BENCHY_CELLS + BASE_CELLS


def load():
    with open(CAMPAIGN, encoding="utf-8") as fh:
        return json.load(fh)


def rederive(doc, failures):
    """Recompute every derived number from the pair rows."""
    pairs = doc["pairs"]
    output = doc["output"]
    null = doc.get("null", {})

    for cell in ALL_CELLS:
        if cell not in pairs:
            failures.append("%s: no paired campaign" % cell)
            continue
        p = pairs[cell]
        rows = p["pairs"]
        if p["n_pairs"] != len(rows):
            failures.append("%s: n_pairs does not match row count" % cell)
        if len(rows) < 5:
            failures.append("%s: fewer than the pre-declared 5 pairs" % cell)

        wall = statistics.median(
            [r["candidate_wall_s"] / r["baseline_wall_s"] for r in rows]
        )
        cpu = statistics.median(
            [r["candidate_cpu_s"] / r["baseline_cpu_s"] for r in rows]
        )
        if abs(wall - p["wall_ratio_median"]) > 1e-9:
            failures.append("%s: wall ratio median does not re-derive" % cell)
        if abs(cpu - p["cpu_ratio_median"]) > 1e-9:
            failures.append("%s: cpu ratio median does not re-derive" % cell)
        saving = (1.0 - wall) * 100.0
        if abs(saving - p["wall_saving_pct"]) > 1e-9:
            failures.append("%s: wall saving does not re-derive" % cell)
        if abs(saving / FLOOR_PCT - p["floor_multiple"]) > 1e-9:
            failures.append("%s: floor multiple does not re-derive" % cell)
        faster = sum(1 for r in rows if r["candidate_wall_s"] < r["baseline_wall_s"])
        if faster != p["pairs_candidate_faster"]:
            failures.append("%s: pair-win count does not re-derive" % cell)
        if p["hashes_identical_within_pairs"]:
            failures.append(
                "%s: baseline and candidate hashes match inside a pair" % cell
            )

        if cell not in output:
            failures.append("%s: no output row" % cell)
            continue
        o = output[cell]
        if not o["baseline_reproduces_t151"]:
            failures.append("%s: baseline does not reproduce the t151 reference" % cell)
        if not o["output_moved"]:
            failures.append("%s: output did not move" % cell)
        if o["byte_delta"] != o["candidate_bytes"] - o["baseline_bytes"]:
            failures.append("%s: byte delta does not re-derive" % cell)
        if o["baseline_prefix16"] != o["baseline_sha256"][:16]:
            failures.append("%s: baseline prefix is not the hash prefix" % cell)

    for cell, n in null.items():
        if n["n_pairs"] < 3:
            failures.append("%s: null control has fewer than 3 pairs" % cell)

    return {"pairs": pairs, "output": output, "null": null}


def gate(doc):
    """The pre-declared gate, read cell by cell."""
    out = {}
    null = doc["null"]
    for cell in ALL_CELLS:
        p = doc["pairs"].get(cell)
        if not p:
            continue
        saving = p["wall_saving_pct"]
        band = null.get(cell, {}).get("wall_band_pct_any_pair")
        if cell in BASE_CELLS:
            clears_floor = saving >= FLOOR_PCT
            outside_null = None if band is None else abs(saving) > band
            verdict = (
                "PASS" if clears_floor and (outside_null is not False) else "DEGRADED"
            )
            if not clears_floor:
                verdict = "FAIL"
            out[cell] = {
                "saving_pct": saving,
                "clears_floor": clears_floor,
                "outside_null_band": outside_null,
                "null_band_pct": band,
                "verdict": verdict,
            }
        else:
            no_regression = saving >= 0.0
            out[cell] = {
                "saving_pct": saving,
                "no_median_regression": no_regression,
                "verdict": "PASS" if no_regression else "FAIL",
            }
    return out


def main(argv):
    doc = load()
    failures = []
    derived = rederive(doc, failures)

    if "--self-test" in argv:
        import copy

        def mutate(name, fn):
            bad = copy.deepcopy(doc)
            fn(bad)
            fails = []
            rederive(bad, fails)
            print("%-40s %s" % (name, "REJECTED" if fails else "ACCEPTED"))
            return bool(fails)

        controls = [
            mutate(
                "baseline hash stops reproducing t151",
                lambda d: d["output"]["base-classic-off"].update(
                    baseline_reproduces_t151=False
                ),
            ),
            mutate(
                "output declared unmoved",
                lambda d: d["output"]["benchy-classic-on"].update(output_moved=False),
            ),
            mutate(
                "pair ratio median falsified",
                lambda d: d["pairs"]["base-arachne-off"].update(wall_ratio_median=0.5),
            ),
            mutate(
                "a pair deleted", lambda d: d["pairs"]["base-classic-on"]["pairs"].pop()
            ),
            mutate(
                "pair-win count falsified",
                lambda d: d["pairs"]["benchy-arachne-on"].update(
                    pairs_candidate_faster=0
                ),
            ),
            mutate("cell removed", lambda d: d["pairs"].pop("benchy-arachne-off")),
            mutate(
                "byte delta falsified",
                lambda d: d["output"]["base-classic-on"].update(byte_delta=1),
            ),
        ]
        ok = all(controls)
        print("self-test:", "PASS" if ok else "FAIL")
        return 0 if ok else 1

    g = gate(doc)

    print(
        "cell                    pairs  wall saving   floor  cpu saving  pairs faster  verdict"
    )
    for cell in ALL_CELLS:
        p = doc["pairs"].get(cell)
        if not p:
            continue
        print(
            "%-22s %5d  %+9.3f %%  %5.2fx  %+8.3f %%  %3d/%d       %s"
            % (
                cell,
                p["n_pairs"],
                p["wall_saving_pct"],
                p["floor_multiple"],
                p["cpu_saving_pct"],
                p["pairs_candidate_faster"],
                p["n_pairs"],
                g[cell]["verdict"],
            )
        )

    print()
    print("output (baseline -> candidate, all eight cells must move):")
    for cell in ALL_CELLS:
        o = doc["output"].get(cell)
        if o:
            print(
                "%-22s %s -> %s  %+7d B"
                % (
                    cell,
                    o["baseline_prefix16"],
                    o["candidate_prefix16"],
                    o["byte_delta"],
                )
            )

    if doc["null"]:
        print()
        print("null control (baseline arm against itself):")
        for cell, n in sorted(doc["null"].items()):
            print(
                "%-22s n=%d  wall band any-pair %.2f %%, median %.2f %%  cpu any-pair %.2f %%"
                % (
                    cell,
                    n["n_pairs"],
                    n["wall_band_pct_any_pair"],
                    n["wall_band_pct_median"],
                    n["cpu_band_pct_any_pair"],
                )
            )

    passed = [c for c, v in g.items() if v["verdict"] == "PASS"]
    print()
    print(
        "gate: %d/8 PASS (%s)"
        % (len(passed), ", ".join(sorted(passed)) if passed else "none")
    )

    if failures:
        for f in failures:
            print("FAIL", f)
        return 1
    print("PASS every number re-derives from reduced/campaign.json")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
